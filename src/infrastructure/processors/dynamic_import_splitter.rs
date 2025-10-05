// Dynamic Import Splitter - Detects and splits import() statements into lazy-loaded chunks
// Currently infrastructure for future integration
use crate::core::models::ModuleInfo;
use crate::utils::{Result, SokuError};
use std::collections::{HashMap, HashSet};
use regex::Regex;

/// Dynamic import analyzer and splitter
#[allow(dead_code)] // Infrastructure for future integration
pub struct DynamicImportSplitter {
    /// Detected dynamic imports: source module -> imported modules
    dynamic_imports: HashMap<String, Vec<String>>,
    /// Import ID counter for chunk naming
    import_counter: usize,
}

#[allow(dead_code)] // Infrastructure methods for future integration
impl DynamicImportSplitter {
    pub fn new() -> Self {
        Self {
            dynamic_imports: HashMap::new(),
            import_counter: 0,
        }
    }

    /// Analyze code for dynamic import() statements
    pub fn analyze_dynamic_imports(&mut self, modules: &[ModuleInfo]) -> Result<()> {
        // Regex to match import('path') or import("path")
        let import_re = Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#)
            .map_err(|e| SokuError::Build {
                message: format!("Failed to create import regex: {}", e),
                context: None,
            })?;

        for module in modules {
            let module_path = module.path.to_string_lossy().to_string();
            let mut imports = Vec::new();

            // Find all dynamic imports in this module
            for cap in import_re.captures_iter(&module.content) {
                if let Some(import_path) = cap.get(1) {
                    imports.push(import_path.as_str().to_string());
                }
            }

            if !imports.is_empty() {
                self.dynamic_imports.insert(module_path, imports);
            }
        }

        Ok(())
    }

    /// Get all dynamically imported module paths
    pub fn get_dynamic_imports(&self) -> HashSet<String> {
        let mut all_imports = HashSet::new();
        for imports in self.dynamic_imports.values() {
            for import in imports {
                all_imports.insert(import.clone());
            }
        }
        all_imports
    }

    /// Check if a module is dynamically imported
    pub fn is_dynamically_imported(&self, module_path: &str) -> bool {
        self.dynamic_imports.values()
            .any(|imports| imports.iter().any(|imp| {
                // Normalize paths for comparison
                let imp_normalized = imp.trim_start_matches("./").trim_start_matches("../");
                let module_normalized = module_path.trim_start_matches("./").trim_start_matches("../");

                // Check if paths match (either exact or contains)
                module_normalized.contains(imp_normalized) || imp_normalized.contains(module_normalized)
            }))
    }

    /// Replace import() statements with chunk loader calls
    pub fn replace_dynamic_imports(&mut self, code: &str, chunk_manifest: &HashMap<String, String>) -> String {
        let import_re = Regex::new(r#"import\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();

        let mut result = code.to_string();
        let mut replacements = Vec::new();

        // Collect all replacements first to avoid borrow issues
        for cap in import_re.captures_iter(code) {
            if let (Some(full_match), Some(import_path)) = (cap.get(0), cap.get(1)) {
                let import_str = import_path.as_str();

                // Find matching chunk in manifest
                let chunk_file = chunk_manifest.iter()
                    .find(|(path, _)| path.contains(import_str))
                    .map(|(_, file)| file.clone())
                    .unwrap_or_else(|| {
                        self.import_counter += 1;
                        format!("chunk-{}.js", self.import_counter)
                    });

                // Replace import() with __ultra_load_chunk()
                replacements.push((
                    full_match.as_str().to_string(),
                    format!("__ultra_load_chunk('{}')", chunk_file)
                ));
            }
        }

        // Apply replacements
        for (old, new) in replacements {
            result = result.replace(&old, &new);
        }

        result
    }

    /// Generate runtime chunk loader
    pub fn generate_chunk_loader() -> String {
        r#"
// Ultra Dynamic Import Loader
(function() {
  window.__ultra_loaded_chunks = window.__ultra_loaded_chunks || {};
  window.__ultra_loading_chunks = window.__ultra_loading_chunks || {};

  window.__ultra_load_chunk = function(chunkPath) {
    // Return cached chunk if already loaded
    if (window.__ultra_loaded_chunks[chunkPath]) {
      return Promise.resolve(window.__ultra_loaded_chunks[chunkPath]);
    }

    // Return in-progress load if already loading
    if (window.__ultra_loading_chunks[chunkPath]) {
      return window.__ultra_loading_chunks[chunkPath];
    }

    // Load chunk dynamically
    const loadPromise = new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = chunkPath;
      script.async = true;

      script.onload = () => {
        // Extract module exports from global __ultra_chunk_exports
        const exports = window.__ultra_chunk_exports || {};
        window.__ultra_loaded_chunks[chunkPath] = exports;
        delete window.__ultra_loading_chunks[chunkPath];
        resolve(exports);
      };

      script.onerror = () => {
        delete window.__ultra_loading_chunks[chunkPath];
        reject(new Error(`Failed to load chunk: ${chunkPath}`));
      };

      document.head.appendChild(script);
    });

    window.__ultra_loading_chunks[chunkPath] = loadPromise;
    return loadPromise;
  };
})();
"#.to_string()
    }

    /// Create chunk manifest mapping module paths to chunk files
    pub fn create_chunk_manifest(&self, modules: &[ModuleInfo]) -> HashMap<String, String> {
        let mut manifest = HashMap::new();

        let mut chunk_counter = 0;
        for module in modules {
            let module_path = module.path.to_string_lossy().to_string();

            // Check if this module is dynamically imported using the same logic
            if self.is_dynamically_imported(&module_path) {
                chunk_counter += 1;
                let chunk_name = format!("chunk-{}.js", chunk_counter);
                manifest.insert(module_path, chunk_name);
            }
        }

        manifest
    }

    /// Get dynamic import statistics
    pub fn get_stats(&self) -> DynamicImportStats {
        let total_dynamic_imports: usize = self.dynamic_imports.values()
            .map(|v| v.len())
            .sum();

        DynamicImportStats {
            modules_with_imports: self.dynamic_imports.len(),
            total_dynamic_imports,
            unique_imports: self.get_dynamic_imports().len(),
        }
    }
}

impl Default for DynamicImportSplitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about dynamic imports
#[derive(Debug, Clone)]
#[allow(dead_code)] // Infrastructure for future integration
pub struct DynamicImportStats {
    pub modules_with_imports: usize,
    pub total_dynamic_imports: usize,
    pub unique_imports: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::ModuleType;
    use std::path::PathBuf;

    fn create_test_module(path: &str, content: &str) -> ModuleInfo {
        ModuleInfo {
            path: PathBuf::from(path),
            content: content.to_string(),
            module_type: ModuleType::JavaScript,
            dependencies: Vec::new(),
            exports: Vec::new(),
        }
    }

    #[test]
    fn test_detect_dynamic_imports() {
        let mut splitter = DynamicImportSplitter::new();

        let modules = vec![
            create_test_module("main.js", r#"
                import('./utils.js');
                import("./components.js");
            "#),
        ];

        splitter.analyze_dynamic_imports(&modules).unwrap();

        let imports = splitter.get_dynamic_imports();
        assert_eq!(imports.len(), 2);
        assert!(imports.contains("./utils.js"));
        assert!(imports.contains("./components.js"));
    }

    #[test]
    fn test_is_dynamically_imported() {
        let mut splitter = DynamicImportSplitter::new();

        let modules = vec![
            create_test_module("main.js", "import('./lazy.js');"),
        ];

        splitter.analyze_dynamic_imports(&modules).unwrap();

        assert!(splitter.is_dynamically_imported("./lazy.js"));
        assert!(splitter.is_dynamically_imported("/path/to/lazy.js"));
        assert!(!splitter.is_dynamically_imported("./static.js"));
    }

    #[test]
    fn test_replace_dynamic_imports() {
        let mut splitter = DynamicImportSplitter::new();

        let code = r#"
            async function loadModule() {
                const module = await import('./feature.js');
                return module;
            }
        "#;

        let mut manifest = HashMap::new();
        manifest.insert("./feature.js".to_string(), "chunk-1.js".to_string());

        let result = splitter.replace_dynamic_imports(code, &manifest);

        assert!(result.contains("__ultra_load_chunk('chunk-1.js')"));
        assert!(!result.contains("import('./feature.js')"));
    }

    #[test]
    fn test_chunk_loader_generation() {
        let loader = DynamicImportSplitter::generate_chunk_loader();

        assert!(loader.contains("__ultra_load_chunk"));
        assert!(loader.contains("__ultra_loaded_chunks"));
        assert!(loader.contains("document.createElement('script')"));
    }

    #[test]
    fn test_create_chunk_manifest() {
        let mut splitter = DynamicImportSplitter::new();

        let modules = vec![
            create_test_module("main.js", "import('./lazy.js');"),
            create_test_module("lazy.js", "export const feature = 'test';"),
        ];

        splitter.analyze_dynamic_imports(&modules).unwrap();
        let manifest = splitter.create_chunk_manifest(&modules);

        assert!(manifest.values().any(|v| v.starts_with("chunk-")));
    }

    #[test]
    fn test_get_stats() {
        let mut splitter = DynamicImportSplitter::new();

        let modules = vec![
            create_test_module("main.js", r#"
                import('./a.js');
                import('./b.js');
            "#),
            create_test_module("other.js", "import('./c.js');"),
        ];

        splitter.analyze_dynamic_imports(&modules).unwrap();
        let stats = splitter.get_stats();

        assert_eq!(stats.modules_with_imports, 2);
        assert_eq!(stats.total_dynamic_imports, 3);
        assert_eq!(stats.unique_imports, 3);
    }
}
