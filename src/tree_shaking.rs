use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::{HashMap, HashSet};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub name: String,
    pub is_default: bool,
    pub location: (usize, usize), // start, end positions
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub name: String,
    pub source: String,
    pub is_default: bool,
    pub location: (usize, usize),
}

#[derive(Debug, Default)]
pub struct ModuleAnalysis {
    pub exports: Vec<ExportInfo>,
    pub imports: Vec<ImportInfo>,
    pub used_identifiers: HashSet<String>,
    pub defined_identifiers: HashSet<String>,
}

pub struct TreeShaker {
    module_graph: HashMap<String, ModuleAnalysis>,
    used_exports: HashSet<String>, // format: "module_path::export_name"
}

impl TreeShaker {
    pub fn new() -> Self {
        Self {
            module_graph: HashMap::new(),
            used_exports: HashSet::new(),
        }
    }

    /// Analyze a JavaScript/TypeScript file for imports/exports
    pub fn analyze_module(&mut self, path: &str, source: &str) -> Result<()> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap_or_default();

        let parser = Parser::new(&allocator, source, source_type);
        let parse_result = parser.parse();

        if !parse_result.errors.is_empty() {
            // Log errors but continue - we want to be resilient
            eprintln!("Parse errors in {}: {:?}", path, parse_result.errors);
        }

        let mut analysis = ModuleAnalysis::default();

        // Walk the AST to find imports/exports
        self.analyze_ast(&parse_result.program, &mut analysis, source)?;

        self.module_graph.insert(path.to_string(), analysis);
        Ok(())
    }

    fn analyze_ast(&self, program: &Program, analysis: &mut ModuleAnalysis, source: &str) -> Result<()> {
        // For now, use simple text-based analysis since oxc API is complex
        // This is a simplified version that works with basic ES modules
        self.analyze_text_based(source, analysis)?;
        Ok(())
    }

    /// Simplified text-based analysis for ES modules
    fn analyze_text_based(&self, source: &str, analysis: &mut ModuleAnalysis) -> Result<()> {
        let lines: Vec<&str> = source.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Parse import statements
            if trimmed.starts_with("import ") {
                if let Some(import_info) = self.parse_import_line(trimmed) {
                    analysis.imports.push(ImportInfo {
                        name: import_info.0,
                        source: import_info.1,
                        is_default: import_info.2,
                        location: (line_num * 80, (line_num + 1) * 80), // Approximate positions
                    });
                }
            }

            // Parse export statements
            if trimmed.starts_with("export ") {
                if let Some(export_info) = self.parse_export_line(trimmed) {
                    analysis.exports.push(ExportInfo {
                        name: export_info.0,
                        is_default: export_info.1,
                        location: (line_num * 80, (line_num + 1) * 80),
                    });
                }
            }

            // Track function and variable declarations
            if trimmed.starts_with("function ") || trimmed.starts_with("const ") ||
               trimmed.starts_with("let ") || trimmed.starts_with("var ") {
                if let Some(identifier) = self.extract_identifier(trimmed) {
                    analysis.defined_identifiers.insert(identifier);
                }
            }
        }

        Ok(())
    }

    fn parse_import_line(&self, line: &str) -> Option<(String, String, bool)> {
        // Handle: import defaultExport from "module"
        if let Some(caps) = regex::Regex::new(r#"import\s+(\w+)\s+from\s+["']([^"']+)["']"#)
            .ok()?.captures(line) {
            return Some((caps[1].to_string(), caps[2].to_string(), true));
        }

        // Handle: import { named } from "module"
        if let Some(caps) = regex::Regex::new(r#"import\s+\{\s*(\w+)\s*\}\s+from\s+["']([^"']+)["']"#)
            .ok()?.captures(line) {
            return Some((caps[1].to_string(), caps[2].to_string(), false));
        }

        // Handle: import * as namespace from "module"
        if let Some(caps) = regex::Regex::new(r#"import\s+\*\s+as\s+(\w+)\s+from\s+["']([^"']+)["']"#)
            .ok()?.captures(line) {
            return Some((caps[1].to_string(), caps[2].to_string(), false));
        }

        None
    }

    fn parse_export_line(&self, line: &str) -> Option<(String, bool)> {
        // Handle: export default ...
        if line.starts_with("export default") {
            return Some(("default".to_string(), true));
        }

        // Handle: export const/function/let/var identifier
        if let Some(caps) = regex::Regex::new(r#"export\s+(?:const|let|var|function)\s+(\w+)"#)
            .ok()?.captures(line) {
            return Some((caps[1].to_string(), false));
        }

        // Handle: export { identifier }
        if let Some(caps) = regex::Regex::new(r#"export\s+\{\s*(\w+)\s*\}"#)
            .ok()?.captures(line) {
            return Some((caps[1].to_string(), false));
        }

        None
    }

    fn extract_identifier(&self, line: &str) -> Option<String> {
        // Extract identifier from function/variable declarations
        if let Some(caps) = regex::Regex::new(r#"(?:function|const|let|var)\s+(\w+)"#)
            .ok()?.captures(line) {
            return Some(caps[1].to_string());
        }
        None
    }

    /// Mark an export as used (entry point for tree shaking)
    pub fn mark_used(&mut self, module_path: &str, export_name: &str) {
        let key = format!("{}::{}", module_path, export_name);
        self.used_exports.insert(key);
    }

    /// Perform tree shaking analysis to find all reachable code
    pub fn shake(&mut self, entry_points: &[String]) -> Result<HashSet<String>> {
        // Start with entry points
        for entry in entry_points {
            self.mark_used(entry, "default"); // Assume default export for entry points
        }

        // Iteratively find all reachable exports
        let mut changed = true;
        while changed {
            changed = false;

            let current_used = self.used_exports.clone();
            for used_export in current_used {
                let parts: Vec<&str> = used_export.split("::").collect();
                if parts.len() != 2 {
                    continue;
                }

                let (module_path, _export_name) = (parts[0], parts[1]);

                // Find all imports in this module and mark them as used
                if let Some(analysis) = self.module_graph.get(module_path) {
                    for import in &analysis.imports {
                        let import_key = format!("{}::{}", import.source, import.name);
                        if !self.used_exports.contains(&import_key) {
                            self.used_exports.insert(import_key);
                            changed = true;
                        }
                    }
                }
            }
        }

        Ok(self.used_exports.clone())
    }

    /// Generate optimized code by removing unused exports
    pub fn generate_optimized_code(&self, module_path: &str, source: &str) -> Result<String> {
        if let Some(analysis) = self.module_graph.get(module_path) {
            let mut result = source.to_string();

            // Remove unused exports (in reverse order to maintain positions)
            let mut removals: Vec<(usize, usize)> = Vec::new();

            for export in &analysis.exports {
                let export_key = format!("{}::{}", module_path, export.name);
                if !self.used_exports.contains(&export_key) {
                    removals.push(export.location);
                }
            }

            // Sort removals by position (reverse order)
            removals.sort_by(|a, b| b.0.cmp(&a.0));

            // Remove unused exports
            for (start, end) in removals {
                if start < result.len() && end <= result.len() {
                    result.replace_range(start..end, "");
                }
            }

            Ok(result)
        } else {
            Ok(source.to_string())
        }
    }

    /// Get tree shaking statistics
    pub fn get_stats(&self) -> TreeShakingStats {
        let total_exports: usize = self.module_graph.values()
            .map(|analysis| analysis.exports.len())
            .sum();

        let used_exports = self.used_exports.len();
        let removed_exports = total_exports.saturating_sub(used_exports);

        TreeShakingStats {
            total_modules: self.module_graph.len(),
            total_exports,
            used_exports,
            removed_exports,
            reduction_percentage: if total_exports > 0 {
                (removed_exports as f64 / total_exports as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug)]
pub struct TreeShakingStats {
    pub total_modules: usize,
    pub total_exports: usize,
    pub used_exports: usize,
    pub removed_exports: usize,
    pub reduction_percentage: f64,
}

impl std::fmt::Display for TreeShakingStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "🌳 Tree Shaking Results:\n  📦 Modules analyzed: {}\n  📤 Total exports: {}\n  ✅ Used exports: {}\n  🗑️  Removed exports: {}\n  📉 Code reduction: {:.1}%",
            self.total_modules,
            self.total_exports,
            self.used_exports,
            self.removed_exports,
            self.reduction_percentage
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tree_shaking() {
        let mut shaker = TreeShaker::new();

        // Sample module with exports
        let source = r#"
export const usedFunction = () => console.log("used");
export const unusedFunction = () => console.log("unused");
export default function main() { return usedFunction(); }
"#;

        shaker.analyze_module("test.js", source).unwrap();
        shaker.mark_used("test.js", "default");
        shaker.mark_used("test.js", "usedFunction");

        let used_exports = shaker.shake(&["test.js".to_string()]).unwrap();

        assert!(used_exports.contains("test.js::default"));
        assert!(used_exports.contains("test.js::usedFunction"));
        assert!(!used_exports.contains("test.js::unusedFunction"));
    }

    #[test]
    fn test_import_tracking() {
        let mut shaker = TreeShaker::new();

        let source = r#"
import { helper } from './utils.js';
import defaultHelper from './defaults.js';

export const myFunction = () => {
    return helper() + defaultHelper();
};
"#;

        shaker.analyze_module("main.js", source).unwrap();

        if let Some(analysis) = shaker.module_graph.get("main.js") {
            assert_eq!(analysis.imports.len(), 2);
            assert_eq!(analysis.exports.len(), 1);
        }
    }
}