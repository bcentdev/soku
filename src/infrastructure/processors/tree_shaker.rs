use crate::core::{interfaces::TreeShaker, models::*};
use crate::utils::{Logger, Result};
use std::collections::{HashMap, HashSet};

pub struct RegexTreeShaker {
    module_graph: HashMap<String, ModuleAnalysis>,
    used_exports: HashSet<String>,
    node_modules_imports: HashMap<String, NodeModuleImport>, // Track node_modules usage
}

/// Represents an import from node_modules
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NodeModuleImport {
    package_name: String,
    #[allow(dead_code)]
    imported_names: Vec<String>, // e.g., ["map", "filter"] from lodash
    #[allow(dead_code)]
    import_type: NodeModuleImportType,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum NodeModuleImportType {
    Default, // import _ from 'lodash'
    #[allow(dead_code)]
    Named(Vec<String>), // import { map, filter } from 'lodash'
    Namespace, // import * as _ from 'lodash'
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
            node_modules_imports: HashMap::new(),
        }
    }

    /// Check if an import path is from node_modules
    fn is_node_modules_import(&self, import_path: &str) -> bool {
        !import_path.starts_with("./")
            && !import_path.starts_with("../")
            && !import_path.starts_with("/")
            && !import_path.ends_with(".js")
            && !import_path.ends_with(".ts")
            && !import_path.ends_with(".css")
    }

    /// Parse node_modules import to extract package name and imports
    fn parse_node_modules_import(&self, line: &str) -> Option<NodeModuleImport> {
        // Handle: import _ from 'lodash'
        if let Ok(re) = regex::Regex::new(r#"import\s+(\w+)\s+from\s+["']([^"']+)["']"#) {
            if let Some(caps) = re.captures(line) {
                let import_name = caps[1].to_string();
                let package_name = caps[2].to_string();

                if self.is_node_modules_import(&package_name) {
                    return Some(NodeModuleImport {
                        package_name,
                        imported_names: vec![import_name],
                        import_type: NodeModuleImportType::Default,
                    });
                }
            }
        }

        // Handle: import { map, filter, reduce } from 'lodash'
        if let Ok(re) = regex::Regex::new(r#"import\s+\{\s*([^}]+)\s*\}\s+from\s+["']([^"']+)["']"#)
        {
            if let Some(caps) = re.captures(line) {
                let imports_str = caps[1].to_string();
                let package_name = caps[2].to_string();

                if self.is_node_modules_import(&package_name) {
                    let imported_names: Vec<String> = imports_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();

                    return Some(NodeModuleImport {
                        package_name,
                        imported_names: imported_names.clone(),
                        import_type: NodeModuleImportType::Named(imported_names),
                    });
                }
            }
        }

        // Handle: import * as _ from 'lodash'
        if let Ok(re) = regex::Regex::new(r#"import\s+\*\s+as\s+(\w+)\s+from\s+["']([^"']+)["']"#) {
            if let Some(caps) = re.captures(line) {
                let namespace_name = caps[1].to_string();
                let package_name = caps[2].to_string();

                if self.is_node_modules_import(&package_name) {
                    return Some(NodeModuleImport {
                        package_name,
                        imported_names: vec![namespace_name],
                        import_type: NodeModuleImportType::Namespace,
                    });
                }
            }
        }

        None
    }

    fn analyze_text_based(
        &mut self,
        source: &str,
        analysis: &mut ModuleAnalysis,
        module_path: &str,
    ) -> Result<()> {
        let lines: Vec<&str> = source.lines().collect();

        for line in lines.iter() {
            let trimmed = line.trim();

            // Parse import statements
            if trimmed.starts_with("import ") {
                // Check if it's a node_modules import first
                if let Some(node_import) = self.parse_node_modules_import(trimmed) {
                    // Track node_modules import for tree shaking
                    let key = format!("{}:{}", module_path, node_import.package_name);
                    self.node_modules_imports.insert(key, node_import);
                }

                // Also handle as regular import for dependency graph
                if let Some((name, source)) = self.parse_import_line(trimmed) {
                    analysis.imports.push(ImportInfo { name, source });
                }
            }

            // Parse export statements
            if trimmed.starts_with("export ") {
                let export_names = self.parse_export_line(trimmed);
                for export_name in export_names {
                    analysis.exports.push(ExportInfo { name: export_name });
                }
            }

            // Track function and variable declarations
            if trimmed.starts_with("function ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("let ")
                || trimmed.starts_with("var ")
            {
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

    fn parse_export_line(&self, line: &str) -> Vec<String> {
        let mut exports = Vec::new();

        // Handle: export default ...
        if line.starts_with("export default") {
            exports.push("default".to_string());
            return exports;
        }

        // Handle: export const/function/let/var identifier
        if let Ok(re) = regex::Regex::new(r#"export\s+(?:const|let|var|function)\s+(\w+)"#) {
            if let Some(caps) = re.captures(line) {
                exports.push(caps[1].to_string());
                return exports;
            }
        }

        // Handle: export { identifier1, identifier2, ... }
        if let Ok(re) = regex::Regex::new(r#"export\s+\{\s*([^}]+)\s*\}"#) {
            if let Some(caps) = re.captures(line) {
                let exports_str = caps[1].to_string();
                // Split by comma and trim each identifier
                for export in exports_str.split(',') {
                    let trimmed = export.trim();
                    // Handle "identifier as alias" -> just take "identifier"
                    let identifier = if let Some(pos) = trimmed.find(" as ") {
                        &trimmed[..pos]
                    } else {
                        trimmed
                    };
                    if !identifier.is_empty() {
                        exports.push(identifier.to_string());
                    }
                }
                return exports;
            }
        }

        exports
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

    /// Get the tracked node_modules imports for optimization
    #[allow(dead_code)] // Future use for node_modules optimization analysis
    pub fn get_node_modules_imports(&self) -> &HashMap<String, NodeModuleImport> {
        &self.node_modules_imports
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
        let total_exports: usize = self
            .module_graph
            .values()
            .map(|analysis| analysis.exports.len())
            .sum();

        let used_exports = self.used_exports.len();
        let removed_exports = total_exports.saturating_sub(used_exports);

        // Count node_modules packages that will be optimized
        let node_modules_count = self.node_modules_imports.len();

        // Build used exports map: module_path -> {export1, export2, ...}
        let mut used_exports_map: HashMap<String, HashSet<String>> = HashMap::new();
        for used_export in &self.used_exports {
            let parts: Vec<&str> = used_export.split("::").collect();
            if parts.len() == 2 {
                let (module_path, export_name) = (parts[0], parts[1]);
                used_exports_map
                    .entry(module_path.to_string())
                    .or_default()
                    .insert(export_name.to_string());
            }
        }

        Logger::debug(&format!(
            "Tree shaking results: {} modules, {} node_modules imports, {} exports removed",
            self.module_graph.len(),
            node_modules_count,
            removed_exports
        ));

        Ok(TreeShakingStats {
            total_modules: self.module_graph.len() + node_modules_count,
            removed_exports,
            reduction_percentage: if total_exports > 0 {
                (removed_exports as f64 / total_exports as f64) * 100.0
            } else {
                0.0
            },
            used_exports: used_exports_map,
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
                module
                    .path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown"),
            );

            self.analyze_text_based(&module.content, &mut analysis, &path)?;
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
"#
                .to_string(),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
            ModuleInfo {
                path: PathBuf::from("utils.js"),
                content: r#"
export const usedFunction = () => "used";
export const unusedFunction = () => "unused";
"#
                .to_string(),
                module_type: ModuleType::JavaScript,
                dependencies: vec![],
                exports: vec![],
            },
        ];

        shaker.analyze_modules(&modules).await.unwrap();

        let entry_points = vec!["main.js".to_string()];
        let stats = shaker.shake(&entry_points).await.unwrap();

        assert!(stats.total_modules > 0);
        assert!(stats.removed_exports > 0);
        assert!(stats.reduction_percentage > 0.0);
    }
}
