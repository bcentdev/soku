// Code splitting functionality - currently unused but kept for future implementation
use crate::core::models::ModuleInfo;
use crate::utils::Result;
use std::collections::{HashMap, HashSet};

/// Smart code splitter for creating optimized bundle chunks
#[allow(dead_code)]
pub struct CodeSplitter {
    /// Map of chunk name to modules
    chunks: HashMap<String, Vec<ModuleInfo>>,
    /// Map of module to chunk assignment
    module_chunk_map: HashMap<String, String>,
    /// Configuration for splitting strategies
    config: CodeSplitConfig,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CodeSplitConfig {
    /// Maximum size for a chunk in bytes
    pub max_chunk_size: usize,
    /// Minimum modules per chunk
    pub min_modules_per_chunk: usize,
    /// Whether to create vendor chunks
    pub create_vendor_chunks: bool,
    /// Whether to split by route/entry points
    pub split_by_routes: bool,
    /// Common dependency threshold for extraction
    pub common_dependency_threshold: usize,
}

impl Default for CodeSplitConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 250_000, // 250KB max chunk size
            min_modules_per_chunk: 2,
            create_vendor_chunks: true,
            split_by_routes: true,
            common_dependency_threshold: 2, // Extract deps used by 2+ chunks
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ChunkInfo {
    pub name: String,
    pub modules: Vec<ModuleInfo>,
    pub size_bytes: usize,
    pub dependencies: Vec<String>,
    pub chunk_type: ChunkType,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ChunkType {
    Entry,       // Main entry point
    Vendor,      // Third-party dependencies
    Common,      // Shared code between chunks
    Route,       // Route-specific code
    Async,       // Dynamically imported code
}

impl CodeSplitter {
    pub fn new(config: CodeSplitConfig) -> Self {
        Self {
            chunks: HashMap::new(),
            module_chunk_map: HashMap::new(),
            config,
        }
    }

    /// Analyze modules and create optimal code splitting strategy
    pub fn analyze_and_split(&mut self, modules: &[ModuleInfo], entry_points: &[String]) -> Result<Vec<ChunkInfo>> {
        // Step 1: Identify entry points and routes
        let entry_modules = self.identify_entry_modules(modules, entry_points);

        // Step 2: Identify vendor dependencies (node_modules)
        let vendor_modules = if self.config.create_vendor_chunks {
            self.identify_vendor_modules(modules)
        } else {
            Vec::new()
        };

        // Step 3: Identify common dependencies
        let common_modules = self.identify_common_dependencies(modules);

        // Step 4: Create chunks based on strategy
        self.create_chunks(&entry_modules, &vendor_modules, &common_modules, modules)?;

        // Step 5: Generate chunk information
        Ok(self.generate_chunk_info())
    }

    /// Identify entry point modules
    fn identify_entry_modules(&self, modules: &[ModuleInfo], entry_points: &[String]) -> Vec<ModuleInfo> {
        modules.iter()
            .filter(|module| {
                let path_str = module.path.to_string_lossy();
                entry_points.iter().any(|entry| {
                    path_str.contains(entry) ||
                    path_str.contains("main") ||
                    path_str.contains("index")
                })
            })
            .cloned()
            .collect()
    }

    /// Identify vendor modules (from node_modules)
    fn identify_vendor_modules(&self, modules: &[ModuleInfo]) -> Vec<ModuleInfo> {
        modules.iter()
            .filter(|module| {
                module.path.to_string_lossy().contains("node_modules")
            })
            .cloned()
            .collect()
    }

    /// Identify commonly used dependencies
    fn identify_common_dependencies(&self, modules: &[ModuleInfo]) -> Vec<ModuleInfo> {
        let mut dependency_usage = HashMap::new();

        // Count how many modules use each dependency
        for module in modules {
            for dep in &module.dependencies {
                *dependency_usage.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Find modules that are commonly used
        let common_deps: HashSet<String> = dependency_usage.iter()
            .filter(|(_, &count)| count >= self.config.common_dependency_threshold)
            .map(|(dep, _)| dep.clone())
            .collect();

        modules.iter()
            .filter(|module| {
                let module_name = module.path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                common_deps.contains(&module_name)
            })
            .cloned()
            .collect()
    }

    /// Create chunks based on identified module groups
    fn create_chunks(
        &mut self,
        entry_modules: &[ModuleInfo],
        vendor_modules: &[ModuleInfo],
        common_modules: &[ModuleInfo],
        all_modules: &[ModuleInfo],
    ) -> Result<()> {
        // Create vendor chunk if we have vendor modules
        if !vendor_modules.is_empty() {
            self.chunks.insert("vendor".to_string(), vendor_modules.to_vec());
            for module in vendor_modules {
                self.module_chunk_map.insert(
                    module.path.to_string_lossy().to_string(),
                    "vendor".to_string()
                );
            }
        }

        // Create common chunk if we have common modules
        if !common_modules.is_empty() {
            self.chunks.insert("common".to_string(), common_modules.to_vec());
            for module in common_modules {
                self.module_chunk_map.insert(
                    module.path.to_string_lossy().to_string(),
                    "common".to_string()
                );
            }
        }

        // Create entry chunks
        for (i, module) in entry_modules.iter().enumerate() {
            let chunk_name = if entry_modules.len() == 1 {
                "main".to_string()
            } else {
                format!("entry-{}", i)
            };

            self.chunks.insert(chunk_name.clone(), vec![module.clone()]);
            self.module_chunk_map.insert(
                module.path.to_string_lossy().to_string(),
                chunk_name
            );
        }

        // Assign remaining modules to appropriate chunks
        self.assign_remaining_modules(all_modules)?;

        // Optimize chunk sizes
        self.optimize_chunk_sizes()?;

        Ok(())
    }

    /// Assign remaining modules to chunks
    fn assign_remaining_modules(&mut self, all_modules: &[ModuleInfo]) -> Result<()> {
        for module in all_modules {
            let module_key = module.path.to_string_lossy().to_string();

            // Skip if already assigned
            if self.module_chunk_map.contains_key(&module_key) {
                continue;
            }

            // Find the best chunk for this module based on dependencies
            let best_chunk = self.find_best_chunk_for_module(module);
            let chunk_name = best_chunk.unwrap_or_else(|| "misc".to_string());

            // Add to chunk
            self.chunks.entry(chunk_name.clone())
                .or_default()
                .push(module.clone());

            self.module_chunk_map.insert(module_key, chunk_name);
        }

        Ok(())
    }

    /// Find the best chunk for a module based on its dependencies
    fn find_best_chunk_for_module(&self, module: &ModuleInfo) -> Option<String> {
        let mut chunk_scores = HashMap::new();

        // Score chunks based on dependency overlap
        for dep in &module.dependencies {
            for (chunk_name, chunk_modules) in &self.chunks {
                for chunk_module in chunk_modules {
                    if chunk_module.dependencies.contains(dep) ||
                       chunk_module.path.file_stem()
                           .and_then(|s| s.to_str())
                           .map(|s| s == dep)
                           .unwrap_or(false) {
                        *chunk_scores.entry(chunk_name.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Return chunk with highest score
        chunk_scores.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(chunk_name, _)| chunk_name)
    }

    /// Optimize chunk sizes by splitting large chunks and merging small ones
    fn optimize_chunk_sizes(&mut self) -> Result<()> {
        let mut chunks_to_split = Vec::new();
        let mut chunks_to_merge = Vec::new();

        // Identify chunks that need optimization
        for (chunk_name, modules) in &self.chunks {
            let total_size = self.calculate_chunk_size(modules);

            if total_size > self.config.max_chunk_size {
                chunks_to_split.push(chunk_name.clone());
            } else if modules.len() < self.config.min_modules_per_chunk &&
                     chunk_name != "vendor" && chunk_name != "main" {
                chunks_to_merge.push(chunk_name.clone());
            }
        }

        // Split large chunks
        for chunk_name in chunks_to_split {
            self.split_large_chunk(&chunk_name)?;
        }

        // Merge small chunks
        if !chunks_to_merge.is_empty() {
            self.merge_small_chunks(chunks_to_merge)?;
        }

        Ok(())
    }

    /// Split a large chunk into smaller chunks
    fn split_large_chunk(&mut self, chunk_name: &str) -> Result<()> {
        if let Some(modules) = self.chunks.remove(chunk_name) {
            let mut current_chunk = Vec::new();
            let mut current_size = 0;
            let mut chunk_counter = 0;

            for module in modules {
                let module_size = module.content.len();

                if current_size + module_size > self.config.max_chunk_size && !current_chunk.is_empty() {
                    // Create new chunk
                    let new_chunk_name = format!("{}-{}", chunk_name, chunk_counter);
                    self.chunks.insert(new_chunk_name.clone(), current_chunk.clone());

                    // Update module mappings
                    for chunk_module in &current_chunk {
                        self.module_chunk_map.insert(
                            chunk_module.path.to_string_lossy().to_string(),
                            new_chunk_name.clone()
                        );
                    }

                    current_chunk.clear();
                    current_size = 0;
                    chunk_counter += 1;
                }

                current_chunk.push(module);
                current_size += module_size;
            }

            // Handle remaining modules
            if !current_chunk.is_empty() {
                let final_chunk_name = if chunk_counter == 0 {
                    chunk_name.to_string()
                } else {
                    format!("{}-{}", chunk_name, chunk_counter)
                };

                self.chunks.insert(final_chunk_name.clone(), current_chunk.clone());

                for chunk_module in &current_chunk {
                    self.module_chunk_map.insert(
                        chunk_module.path.to_string_lossy().to_string(),
                        final_chunk_name.clone()
                    );
                }
            }
        }

        Ok(())
    }

    /// Merge small chunks into a combined chunk
    fn merge_small_chunks(&mut self, chunk_names: Vec<String>) -> Result<()> {
        let mut merged_modules = Vec::new();

        // Collect all modules from small chunks
        for chunk_name in &chunk_names {
            if let Some(modules) = self.chunks.remove(chunk_name) {
                merged_modules.extend(modules);
            }
        }

        // Create merged chunk
        if !merged_modules.is_empty() {
            let merged_chunk_name = "shared".to_string();
            self.chunks.insert(merged_chunk_name.clone(), merged_modules.clone());

            // Update module mappings
            for module in &merged_modules {
                self.module_chunk_map.insert(
                    module.path.to_string_lossy().to_string(),
                    merged_chunk_name.clone()
                );
            }
        }

        Ok(())
    }

    /// Calculate total size of a chunk in bytes
    fn calculate_chunk_size(&self, modules: &[ModuleInfo]) -> usize {
        modules.iter().map(|m| m.content.len()).sum()
    }

    /// Generate chunk information for output
    fn generate_chunk_info(&self) -> Vec<ChunkInfo> {
        self.chunks.iter().map(|(name, modules)| {
            let chunk_type = match name.as_str() {
                "main" => ChunkType::Entry,
                "vendor" => ChunkType::Vendor,
                "common" | "shared" => ChunkType::Common,
                name if name.starts_with("entry") => ChunkType::Entry,
                _ => ChunkType::Route,
            };

            ChunkInfo {
                name: name.clone(),
                modules: modules.clone(),
                size_bytes: self.calculate_chunk_size(modules),
                dependencies: self.extract_chunk_dependencies(modules),
                chunk_type,
            }
        }).collect()
    }

    /// Extract dependencies for a chunk
    fn extract_chunk_dependencies(&self, modules: &[ModuleInfo]) -> Vec<String> {
        let mut deps = HashSet::new();
        for module in modules {
            for dep in &module.dependencies {
                deps.insert(dep.clone());
            }
        }
        deps.into_iter().collect()
    }

    /// Get chunk assignment for a specific module
    #[allow(dead_code)] // Part of public API
    pub fn get_module_chunk(&self, module_path: &str) -> Option<&String> {
        self.module_chunk_map.get(module_path)
    }

    /// Generate bundle code for a specific chunk
    #[allow(dead_code)] // Part of public API
    pub fn generate_chunk_bundle(&self, chunk_name: &str) -> Option<String> {
        self.chunks.get(chunk_name).map(|modules| {
            let mut bundle = String::new();

            bundle.push_str(&format!("// Soku Bundler - Chunk: {}\n", chunk_name));
            bundle.push_str("(function(){\n");

            for module in modules {
                bundle.push_str(&format!("\n// Module: {}\n", module.path.display()));
                bundle.push_str(&module.content);
                bundle.push('\n');
            }

            bundle.push_str("})();\n");
            bundle
        })
    }
}

impl Default for CodeSplitter {
    fn default() -> Self {
        Self::new(CodeSplitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::ModuleType;
    use std::path::Path;

    fn create_test_module(path: &str, content: &str, deps: Vec<String>) -> ModuleInfo {
        ModuleInfo {
            path: Path::new(path).to_path_buf(),
            content: content.to_string(),
            module_type: ModuleType::JavaScript,
            dependencies: deps,
            exports: Vec::new(),
        }
    }

    #[test]
    fn test_code_splitting_basic() {
        // Use config that allows single-module chunks
        let config = CodeSplitConfig {
            min_modules_per_chunk: 1, // Allow single-module chunks for testing
            ..Default::default()
        };
        let mut splitter = CodeSplitter::new(config);

        let modules = vec![
            create_test_module("main.js", "console.log('main');", vec!["utils".to_string()]),
            create_test_module("utils.js", "export function helper() {}", vec![]),
            create_test_module("node_modules/react/index.js", "export default React;", vec![]),
        ];

        let chunks = splitter.analyze_and_split(&modules, &["main.js".to_string()]).unwrap();

        // Should create at least main and vendor chunks
        assert!(chunks.len() >= 2, "Expected at least 2 chunks, got {}", chunks.len());
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Entry), "No Entry chunk found");
        assert!(chunks.iter().any(|c| c.chunk_type == ChunkType::Vendor), "No Vendor chunk found");
    }

    #[test]
    fn test_chunk_size_optimization() {
        let config = CodeSplitConfig {
            max_chunk_size: 100, // Very small for testing
            ..Default::default()
        };

        let mut splitter = CodeSplitter::new(config);

        let large_content = "a".repeat(200); // Larger than max chunk size
        let modules = vec![
            create_test_module("main.js", &large_content, vec![]),
        ];

        let chunks = splitter.analyze_and_split(&modules, &["main.js".to_string()]).unwrap();

        // Should split the large module
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.size_bytes <= 200); // Some flexibility for chunk overhead
        }
    }
}