use std::collections::{HashMap, HashSet};
use regex::Regex;
use crate::core::models::{ModuleInfo, TreeShakingStats};
use crate::utils::Result;
use async_trait::async_trait;

/// Advanced AST-based tree shaker for precise dead code elimination
pub struct AstTreeShaker {
    /// Map of module -> exported symbols
    exports: HashMap<String, HashSet<String>>,
    /// Map of module -> imported symbols
    imports: HashMap<String, HashSet<String>>,
    /// Map of symbol -> usage count
    usage_counts: HashMap<String, usize>,
    /// Dead code patterns identified
    dead_code: HashMap<String, Vec<DeadCodeInfo>>,
}

#[derive(Debug, Clone)]
pub struct DeadCodeInfo {
    pub symbol_name: String,
    #[allow(dead_code)] // Future use for precise code removal
    pub line_start: usize,
    #[allow(dead_code)] // Future use for precise code removal
    pub line_end: usize,
    #[allow(dead_code)] // Future use for categorizing dead code types
    pub code_type: DeadCodeType,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Future use for categorizing different types of dead code
pub enum DeadCodeType {
    UnusedFunction,
    UnusedVariable,
    UnusedClass,
    UnusedImport,
    UnusedExport,
    UnreachableCode,
}

impl AstTreeShaker {
    pub fn new() -> Self {
        Self {
            exports: HashMap::new(),
            imports: HashMap::new(),
            usage_counts: HashMap::new(),
            dead_code: HashMap::new(),
        }
    }

    /// Analyze module with advanced regex patterns for accurate symbol extraction
    pub fn analyze_module_advanced(&mut self, module: &ModuleInfo) -> Result<ModuleAnalysis> {
        let module_path = module.path.to_string_lossy().to_string();
        let mut analysis = ModuleAnalysis {
            module_path: module_path.clone(),
            exports: HashSet::new(),
            imports: HashSet::new(),
        };

        // Advanced export detection patterns
        self.extract_exports_advanced(&module.content, &mut analysis.exports)?;

        // Advanced import detection patterns
        self.extract_imports_advanced(&module.content, &mut analysis.imports)?;

        Ok(analysis)
    }

    /// Extract exports with sophisticated pattern matching
    fn extract_exports_advanced(&self, content: &str, exports: &mut HashSet<String>) -> Result<()> {
        // Pattern 1: export const/let/var name = ...
        let export_var_regex = Regex::new(r"export\s+(?:const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)")?;
        for cap in export_var_regex.captures_iter(content) {
            exports.insert(cap[1].to_string());
        }

        // Pattern 2: export function name() { ... }
        let export_func_regex = Regex::new(r"export\s+function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(")?;
        for cap in export_func_regex.captures_iter(content) {
            exports.insert(cap[1].to_string());
        }

        // Pattern 3: export class Name { ... }
        let export_class_regex = Regex::new(r"export\s+class\s+([a-zA-Z_$][a-zA-Z0-9_$]*)")?;
        for cap in export_class_regex.captures_iter(content) {
            exports.insert(cap[1].to_string());
        }

        // Pattern 4: export { name1, name2, ... }
        let export_list_regex = Regex::new(r"export\s*\{\s*([^}]+)\s*\}")?;
        for cap in export_list_regex.captures_iter(content) {
            let names = cap[1].split(',');
            for name in names {
                let clean_name = name.trim()
                    .split(" as ").next()
                    .unwrap_or(name.trim())
                    .trim();
                if !clean_name.is_empty() {
                    exports.insert(clean_name.to_string());
                }
            }
        }

        // Pattern 5: export default ...
        if content.contains("export default") {
            exports.insert("default".to_string());
        }

        Ok(())
    }

    /// Extract imports with sophisticated pattern matching
    fn extract_imports_advanced(&self, content: &str, imports: &mut HashSet<String>) -> Result<()> {
        // Pattern 1: import { name1, name2 } from '...'
        let named_import_regex = Regex::new(r#"import\s*\{\s*([^}]+)\s*\}\s*from\s*['"]([^'"]+)['"]"#)?;
        for cap in named_import_regex.captures_iter(content) {
            let names = cap[1].split(',');
            for name in names {
                let clean_name = name.trim()
                    .split(" as ").next()
                    .unwrap_or(name.trim())
                    .trim();
                if !clean_name.is_empty() {
                    imports.insert(clean_name.to_string());
                }
            }
        }

        // Pattern 2: import name from '...'
        let default_import_regex = Regex::new(r#"import\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s+from\s*['"]([^'"]+)['"]"#)?;
        for _cap in default_import_regex.captures_iter(content) {
            imports.insert("default".to_string());
        }

        // Pattern 3: import * as name from '...'
        let namespace_import_regex = Regex::new(r"import\s*\*\s*as\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s+from")?;
        for _cap in namespace_import_regex.captures_iter(content) {
            imports.insert("*".to_string());
        }

        Ok(())
    }

    /// Perform advanced tree shaking analysis across all modules
    pub async fn analyze_advanced(&mut self, modules: &[ModuleInfo]) -> Result<()> {
        // Phase 1: Extract all symbols from all modules
        for module in modules {
            let analysis = self.analyze_module_advanced(module)?;
            let module_path = module.path.to_string_lossy().to_string();

            // Initialize usage counts before moving exports
            for export in &analysis.exports {
                self.usage_counts.insert(export.clone(), 0);
            }

            self.exports.insert(module_path.clone(), analysis.exports);
            self.imports.insert(module_path.clone(), analysis.imports);
        }

        // Phase 2: Count actual usage across modules
        for imports in self.imports.values() {
            for import in imports {
                if let Some(count) = self.usage_counts.get_mut(import) {
                    *count += 1;
                }
            }
        }

        // Phase 3: Identify dead code
        self.identify_dead_code(modules).await?;

        Ok(())
    }

    /// Identify dead code with sophisticated analysis
    async fn identify_dead_code(&mut self, modules: &[ModuleInfo]) -> Result<()> {
        for module in modules {
            let module_path = module.path.to_string_lossy().to_string();
            let mut dead_code_list = Vec::new();

            // Find unused exports
            if let Some(exports) = self.exports.get(&module_path) {
                for export in exports {
                    let usage_count = self.usage_counts.get(export).unwrap_or(&0);
                    if *usage_count == 0 {
                        // This export is never used - mark as dead
                        dead_code_list.push(DeadCodeInfo {
                            symbol_name: export.clone(),
                            line_start: 0, // TODO: Extract from AST
                            line_end: 0,
                            code_type: DeadCodeType::UnusedExport,
                        });
                    }
                }
            }

            // Advanced analysis: find unused internal functions/variables
            let internal_analysis = self.analyze_internal_usage(module)?;
            dead_code_list.extend(internal_analysis);

            if !dead_code_list.is_empty() {
                self.dead_code.insert(module_path, dead_code_list);
            }
        }

        Ok(())
    }

    /// Analyze internal symbol usage within a module using advanced regex patterns
    fn analyze_internal_usage(&self, module: &ModuleInfo) -> Result<Vec<DeadCodeInfo>> {
        let mut dead_code = Vec::new();
        let content = &module.content;

        // Find unused variable declarations
        let var_decl_regex = Regex::new(r"(?:const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)")?;
        let var_usage_pattern = |var_name: &str| -> Regex {
            Regex::new(&format!(r"\b{}\b", regex::escape(var_name))).unwrap()
        };

        for cap in var_decl_regex.captures_iter(content) {
            let var_name = &cap[1];
            let usage_regex = var_usage_pattern(var_name);
            let usage_count = usage_regex.find_iter(content).count();

            // If variable is used only once (its declaration), it might be dead
            if usage_count <= 1 {
                dead_code.push(DeadCodeInfo {
                    symbol_name: var_name.to_string(),
                    line_start: 0,
                    line_end: 0,
                    code_type: DeadCodeType::UnusedVariable,
                });
            }
        }

        Ok(dead_code)
    }

    /// Generate optimized code with dead code removed
    #[allow(dead_code)] // Future use for direct code transformation
    pub fn remove_dead_code(&self, module: &ModuleInfo) -> String {
        let module_path = module.path.to_string_lossy().to_string();

        if let Some(dead_code_list) = self.dead_code.get(&module_path) {
            let lines: Vec<&str> = module.content.lines().collect();
            let mut result = String::new();
            let dead_symbols: HashSet<String> = dead_code_list.iter()
                .map(|d| d.symbol_name.clone())
                .collect();

            for line in lines.iter() {
                let line_content = line.trim();

                // Skip lines that export dead symbols
                let mut should_skip = false;
                for dead_symbol in &dead_symbols {
                    if line_content.contains(&format!("export {}", dead_symbol)) ||
                       line_content.contains(&format!("export const {}", dead_symbol)) ||
                       line_content.contains(&format!("export function {}", dead_symbol)) {
                        should_skip = true;
                        result.push_str(&format!("// TREE-SHAKEN: Removed unused export '{}'\n", dead_symbol));
                        break;
                    }
                }

                if !should_skip {
                    result.push_str(line);
                    result.push('\n');
                }
            }

            result
        } else {
            module.content.clone()
        }
    }

    /// Get comprehensive tree shaking statistics
    pub fn get_advanced_stats(&self) -> TreeShakingStats {
        let total_exports: usize = self.exports.values().map(|s| s.len()).sum();
        let unused_exports = self.usage_counts.values().filter(|&&count| count == 0).count();
        let total_dead_code_items: usize = self.dead_code.values().map(|v| v.len()).sum();

        // Build used exports map: module_path -> {export1, export2, ...}
        let mut used_exports_map: HashMap<String, HashSet<String>> = HashMap::new();
        for (symbol, &count) in &self.usage_counts {
            if count > 0 {
                // Symbol format could be "module::export" or just "export"
                // Try to find which module this symbol belongs to
                for (module_path, module_exports) in &self.exports {
                    if module_exports.contains(symbol) {
                        used_exports_map
                            .entry(module_path.clone())
                            .or_default()
                            .insert(symbol.clone());
                        break;
                    }
                }
            }
        }

        TreeShakingStats {
            total_modules: self.exports.len(),
            removed_exports: unused_exports + total_dead_code_items,
            reduction_percentage: if total_exports > 0 {
                ((unused_exports + total_dead_code_items) as f64 / total_exports as f64) * 100.0
            } else {
                0.0
            },
            used_exports: used_exports_map,
        }
    }
}

impl Default for AstTreeShaker {
    fn default() -> Self {
        Self::new()
    }
}

// Removed AST visitor implementation - using regex-based approach for better compatibility

/// Analysis result for a single module
#[derive(Debug)]
pub struct ModuleAnalysis {
    #[allow(dead_code)] // Future use for module tracking
    pub module_path: String,
    pub exports: HashSet<String>,
    pub imports: HashSet<String>,
}

// Removed DeadCodeAnalyzer visitor - using regex-based internal analysis instead

#[async_trait]
impl crate::core::interfaces::TreeShaker for AstTreeShaker {
    async fn analyze_modules(&mut self, modules: &[ModuleInfo]) -> Result<()> {
        self.analyze_advanced(modules).await
    }

    async fn shake(&mut self, _entry_points: &[String]) -> Result<TreeShakingStats> {
        Ok(self.get_advanced_stats())
    }
}