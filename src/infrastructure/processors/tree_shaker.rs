use crate::core::{interfaces::TreeShaker, models::*};
use crate::utils::{Result, Logger};
use std::collections::{HashMap, HashSet};

pub struct RegexTreeShaker {
    module_graph: HashMap<String, ModuleAnalysis>,
    used_exports: HashSet<String>,
}

#[derive(Debug, Default)]
struct ModuleAnalysis {
    exports: Vec<ExportInfo>,
    imports: Vec<ImportInfo>,
    defined_identifiers: HashSet<String>,
}

#[derive(Debug, Clone)]
struct ExportInfo {
    #[allow(dead_code)] // Future tree shaking enhancement
    name: String,
}

#[derive(Debug, Clone)]
struct ImportInfo {
    name: String,
    source: String,
}

impl RegexTreeShaker {
    pub fn new() -> Self {
        Self {
            module_graph: HashMap::new(),
            used_exports: HashSet::new(),
        }
    }

    fn analyze_text_based(&self, source: &str, analysis: &mut ModuleAnalysis) -> Result<()> {
        let lines: Vec<&str> = source.lines().collect();

        for (_line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Parse import statements
            if trimmed.starts_with("import ") {
                if let Some((name, source)) = self.parse_import_line(trimmed) {
                    analysis.imports.push(ImportInfo {
                        name,
                        source,
                    });
                }
            }

            // Parse export statements
            if trimmed.starts_with("export ") {
                if let Some(export_name) = self.parse_export_line(trimmed) {
                    analysis.exports.push(ExportInfo {
                        name: export_name,
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

    fn parse_import_line(&self, line: &str) -> Option<(String, String)> {
        // Handle: import defaultExport from "module"
        if let Ok(re) = regex::Regex::new(r#"import\s+(\w+)\s+from\s+["']([^"']+)["']"#) {
            if let Some(caps) = re.captures(line) {
                return Some((caps[1].to_string(), caps[2].to_string()));
            }
        }

        // Handle: import { named } from "module"
        if let Ok(re) = regex::Regex::new(r#"import\s+\{\s*(\w+)\s*\}\s+from\s+["']([^"']+)["']"#) {
            if let Some(caps) = re.captures(line) {
                return Some((caps[1].to_string(), caps[2].to_string()));
            }
        }

        // Handle: import * as namespace from "module"
        if let Ok(re) = regex::Regex::new(r#"import\s+\*\s+as\s+(\w+)\s+from\s+["']([^"']+)["']"#) {
            if let Some(caps) = re.captures(line) {
                return Some((caps[1].to_string(), caps[2].to_string()));
            }
        }

        None
    }

    fn parse_export_line(&self, line: &str) -> Option<String> {
        // Handle: export default ...
        if line.starts_with("export default") {
            return Some("default".to_string());
        }

        // Handle: export const/function/let/var identifier
        if let Ok(re) = regex::Regex::new(r#"export\s+(?:const|let|var|function)\s+(\w+)"#) {
            if let Some(caps) = re.captures(line) {
                return Some(caps[1].to_string());
            }
        }

        // Handle: export { identifier }
        if let Ok(re) = regex::Regex::new(r#"export\s+\{\s*(\w+)\s*\}"#) {
            if let Some(caps) = re.captures(line) {
                return Some(caps[1].to_string());
            }
        }

        None
    }

    fn extract_identifier(&self, line: &str) -> Option<String> {
        if let Ok(re) = regex::Regex::new(r#"(?:function|const|let|var)\s+(\w+)"#) {
            if let Some(caps) = re.captures(line) {
                return Some(caps[1].to_string());
            }
        }
        None
    }

    fn mark_used(&mut self, module_path: &str, export_name: &str) {
        let key = format!("{}::{}", module_path, export_name);
        self.used_exports.insert(key);
    }

    fn shake_internal(&mut self, entry_points: &[String]) -> Result<TreeShakingStats> {
        // Start with entry points
        for entry in entry_points {
            self.mark_used(entry, "default");
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

        // Calculate statistics
        let total_exports: usize = self.module_graph.values()
            .map(|analysis| analysis.exports.len())
            .sum();

        let used_exports = self.used_exports.len();
        let removed_exports = total_exports.saturating_sub(used_exports);

        Ok(TreeShakingStats {
            total_modules: self.module_graph.len(),
            total_exports,
            used_exports,
            removed_exports,
            reduction_percentage: if total_exports > 0 {
                (removed_exports as f64 / total_exports as f64) * 100.0
            } else {
                0.0
            },
        })
    }

}

#[async_trait::async_trait]
impl TreeShaker for RegexTreeShaker {
    async fn analyze_modules(&mut self, modules: &[ModuleInfo]) -> Result<()> {
        Logger::tree_shaking_enabled();

        for module in modules {
            let path = module.path.to_string_lossy();
            let mut analysis = ModuleAnalysis::default();

            Logger::analyzing_module(
                module.path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            );

            self.analyze_text_based(&module.content, &mut analysis)?;
            self.module_graph.insert(path.to_string(), analysis);
        }

        Ok(())
    }

    async fn shake(&mut self, entry_points: &[String]) -> Result<TreeShakingStats> {
        let stats = self.shake_internal(entry_points)?;

        Logger::found_files(stats.total_modules, 0);

        Ok(stats)
    }

}

impl Default for RegexTreeShaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_tree_shaking() {
        let mut shaker = RegexTreeShaker::new();

        let modules = vec![
            ModuleInfo {
                path: PathBuf::from("main.js"),
                content: r#"
import { usedFunction } from './utils.js';
export default function main() {
    return usedFunction();
}
"#.to_string(),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
            ModuleInfo {
                path: PathBuf::from("utils.js"),
                content: r#"
export const usedFunction = () => "used";
export const unusedFunction = () => "unused";
"#.to_string(),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
        ];

        shaker.analyze_modules(&modules).await.unwrap();

        let entry_points = vec!["main.js".to_string()];
        let stats = shaker.shake(&entry_points).await.unwrap();

        assert!(stats.total_exports > 0);
        assert!(stats.removed_exports > 0);
        assert!(stats.reduction_percentage > 0.0);
    }
}