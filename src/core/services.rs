use crate::core::{interfaces::*, models::*};
use crate::utils::{Result, Logger, Timer, UltraUI, CompletionStats, OutputFileInfo};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};

/// Main build service implementation
pub struct UltraBuildService {
    fs_service: Arc<dyn FileSystemService>,
    js_processor: Arc<dyn JsProcessor>,
    css_processor: Arc<dyn CssProcessor>,
    tree_shaker: Option<Arc<dyn TreeShaker>>,
    ui: UltraUI,
}

impl UltraBuildService {
    pub fn new(
        fs_service: Arc<dyn FileSystemService>,
        js_processor: Arc<dyn JsProcessor>,
        css_processor: Arc<dyn CssProcessor>,
    ) -> Self {
        Self {
            fs_service,
            js_processor,
            css_processor,
            tree_shaker: None,
            ui: UltraUI::new(),
        }
    }

    pub fn with_tree_shaker(mut self, tree_shaker: Arc<dyn TreeShaker>) -> Self {
        self.tree_shaker = Some(tree_shaker);
        self
    }


    async fn scan_and_analyze(&self, config: &BuildConfig) -> Result<ProjectStructure> {
        let _timer = Timer::start("File scanning");
        Logger::scanning_files();

        let structure = self.fs_service.scan_directory(&config.root).await?;
        Logger::found_files(structure.js_modules.len(), structure.css_files.len());

        Ok(structure)
    }

    async fn scan_and_analyze_with_ui(&self, config: &BuildConfig) -> Result<ProjectStructure> {
        let structure = self.fs_service.scan_directory(&config.root).await?;

        // Show file discovery
        self.ui.show_file_discovery(structure.js_modules.len(), structure.css_files.len());

        Ok(structure)
    }

    async fn process_javascript_modules(
        &self,
        modules: &[ModuleInfo],
        config: &BuildConfig,
    ) -> Result<(String, Option<TreeShakingStats>)> {
        let _timer = Timer::start("JavaScript processing");

        let tree_shaking_stats = if config.enable_tree_shaking {
            if let Some(_) = self.tree_shaker {
                Logger::tree_shaking_enabled();

                // Create a new tree shaker instance for this build
                let mut shaker = crate::infrastructure::RegexTreeShaker::new();
                shaker.analyze_modules(modules).await?;

                let entry_points: Vec<String> = modules
                    .iter()
                    .filter(|m| {
                        let name = m.path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        name.contains("main") || name.contains("index")
                    })
                    .map(|m| m.path.to_string_lossy().to_string())
                    .collect();

                let stats = shaker.shake(&entry_points).await?;
                Some(stats)
            } else {
                Logger::warn("Tree shaking enabled but no tree shaker provided");
                None
            }
        } else {
            Logger::tree_shaking_disabled();
            None
        };

        // Process modules
        let bundled_js = self.js_processor.bundle_modules(modules).await?;

        Ok((bundled_js, tree_shaking_stats))
    }

    async fn process_css_files(&self, css_files: &[std::path::PathBuf]) -> Result<String> {
        let _timer = Timer::start("CSS processing");

        let bundled_css = self.css_processor.bundle_css(css_files).await?;
        Ok(bundled_css)
    }

    async fn write_output_files(
        &self,
        config: &BuildConfig,
        js_content: &str,
        css_content: &str,
    ) -> Result<Vec<OutputFile>> {
        let _timer = Timer::start("Writing output files");

        let mut output_files = Vec::new();

        // Write JavaScript bundle
        let js_path = config.outdir.join("bundle.js");
        self.fs_service.write_file(&js_path, js_content).await?;
        output_files.push(OutputFile {
            path: js_path,
            content: js_content.to_string(),
            size: js_content.len(),
        });

        // Write CSS bundle
        let css_path = config.outdir.join("bundle.css");
        self.fs_service.write_file(&css_path, css_content).await?;
        output_files.push(OutputFile {
            path: css_path,
            content: css_content.to_string(),
            size: css_content.len(),
        });

        Ok(output_files)
    }

    async fn resolve_all_dependencies(
        &self,
        entry_files: &[PathBuf],
        root_dir: &Path,
    ) -> Result<Vec<ModuleInfo>> {
        let mut resolved_modules = HashMap::new();
        let mut to_process = Vec::new();

        // Start with entry files
        for path in entry_files {
            to_process.push(path.clone());
        }

        while let Some(current_path) = to_process.pop() {
            // Skip if already processed
            let path_key = current_path.to_string_lossy().to_string();
            if resolved_modules.contains_key(&path_key) {
                continue;
            }

            // Read and process the file
            if let Ok(content) = self.fs_service.read_file(&current_path).await {
                let module_type = ModuleType::from_extension(
                    current_path.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                );

                let mut dependencies = Vec::new();

                // Extract dependencies based on file type
                match module_type {
                    ModuleType::JavaScript | ModuleType::TypeScript => {
                        // Create a temporary JS processor to extract dependencies
                        let js_processor = crate::infrastructure::OxcJsProcessor::new();
                        dependencies = js_processor.extract_dependencies(&content);
                    }
                    ModuleType::Css => {
                        // Extract CSS imports (@import statements)
                        dependencies = self.extract_css_dependencies(&content);
                    }
                    _ => {
                        // Other file types don't have dependencies we can process
                    }
                }

                // Resolve dependency paths
                let mut resolved_deps = Vec::new();
                println!("🔍 DEPS: Processing {} dependencies for {}", dependencies.len(), current_path.display());
                for dep in &dependencies {
                    println!("  Processing dependency: '{}'", dep);
                    if let Some(resolved_path) = self.resolve_import_path(&current_path, dep, root_dir).await {
                        println!("    ✅ Resolved to: {}", resolved_path.display());
                        resolved_deps.push(dep.clone());
                        to_process.push(resolved_path);
                    } else {
                        println!("    ❌ Failed to resolve: {}", dep);
                    }
                }

                let module_info = ModuleInfo {
                    path: current_path.clone(),
                    content,
                    module_type,
                    dependencies: resolved_deps,
                    exports: Vec::new(), // TODO: Extract exports
                };

                resolved_modules.insert(path_key, module_info);
            }
        }

        Ok(resolved_modules.into_values().collect())
    }

    async fn resolve_import_path(
        &self,
        current_file: &Path,
        import_path: &str,
        _root_dir: &Path,
    ) -> Option<PathBuf> {
        println!("🔍 RESOLVE: Resolving '{}' from '{}'", import_path, current_file.display());

        // Handle relative imports
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let current_dir = current_file.parent()?;
            let resolved = current_dir.join(import_path);

            println!("  Current dir: {}", current_dir.display());
            println!("  Resolved base: {}", resolved.display());

            // Check if the file exists as-is first (with original extension)
            if resolved.exists() {
                println!("  ✅ Found exact match: {}", resolved.display());
                return Some(resolved);
            }

            // Try different extensions for JS/TS files only if no extension provided
            if !import_path.contains('.') {
                for ext in &[".js", ".ts", ".jsx", ".tsx"] {
                    let full_path = resolved.with_extension(&ext[1..]);
                    println!("  Trying: {} -> exists: {}", full_path.display(), full_path.exists());

                    if full_path.exists() {
                        println!("  ✅ Found: {}", full_path.display());
                        return Some(full_path);
                    }
                }
            }

            println!("  ❌ Not found with any extension");
        } else {
            println!("  ❌ Not a relative import, skipping");
        }

        None
    }

    fn extract_css_dependencies(&self, content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();

        println!("🔍 CSS: Extracting CSS dependencies from content ({} lines)", content.lines().count());

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Handle @import statements
            if trimmed.starts_with("@import") {
                println!("  Line {}: Found CSS import: {}", line_num + 1, trimmed);

                // Parse @import statements: @import "path" or @import url("path")
                let import_regex = regex::Regex::new(r#"@import\s+(?:url\s*\()?\s*['"]([^'"]+)['"]"#).unwrap();
                if let Some(captures) = import_regex.captures(trimmed) {
                    let import_path = &captures[1];
                    println!("    Found CSS import path: '{}'", import_path);

                    if import_path.starts_with("./") || import_path.starts_with("../") {
                        println!("    ✅ Adding CSS dependency: '{}'", import_path);
                        dependencies.push(import_path.to_string());
                    } else {
                        println!("    ❌ Skipping non-relative CSS import: '{}'", import_path);
                    }
                } else {
                    println!("    ❌ Could not parse CSS import statement");
                }
            }
        }

        println!("🔍 CSS: Found {} dependencies: {:?}", dependencies.len(), dependencies);
        dependencies
    }
}

#[async_trait::async_trait]
impl BuildService for UltraBuildService {
    async fn build(&self, config: &BuildConfig) -> Result<BuildResult> {
        // 🚀 EPIC BANNER!
        self.ui.show_epic_banner();

        let build_start = std::time::Instant::now();

        // Create output directory
        self.fs_service.create_directory(&config.outdir).await?;

        // 🔍 FILE DISCOVERY
        let structure = self.scan_and_analyze_with_ui(config).await?;

        // Convert paths to ModuleInfo and resolve dependencies
        println!("🔍 DEBUG: Found {} JS modules before resolution", structure.js_modules.len());
        for (i, path) in structure.js_modules.iter().enumerate() {
            println!("  {}: {}", i, path.display());
        }

        let js_modules = self.resolve_all_dependencies(&structure.js_modules, &config.root).await?;

        println!("🔍 DEBUG: Resolved {} JS modules after dependency resolution", js_modules.len());
        for (i, module) in js_modules.iter().enumerate() {
            println!("  {}: {} (deps: {})", i, module.path.display(), module.dependencies.len());
        }

        // 🌳 TREE SHAKING (if enabled)
        let tree_shaking_stats = if config.enable_tree_shaking {
            if let Some(_) = self.tree_shaker {
                self.ui.show_tree_shaking_analysis(js_modules.len());

                let mut shaker = crate::infrastructure::RegexTreeShaker::new();
                shaker.analyze_modules(&js_modules).await?;

                let entry_points: Vec<String> = js_modules
                    .iter()
                    .filter(|m| {
                        let name = m.path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        name.contains("main") || name.contains("index")
                    })
                    .map(|m| m.path.to_string_lossy().to_string())
                    .collect();

                Some(shaker.shake(&entry_points).await?)
            } else {
                None
            }
        } else {
            None
        };

        // Separate JS modules from CSS modules
        let js_only_modules: Vec<ModuleInfo> = js_modules.iter()
            .filter(|m| matches!(m.module_type, ModuleType::JavaScript | ModuleType::TypeScript))
            .cloned()
            .collect();

        let css_modules: Vec<ModuleInfo> = js_modules.iter()
            .filter(|m| matches!(m.module_type, ModuleType::Css))
            .cloned()
            .collect();

        // ⚡ JAVASCRIPT PROCESSING
        let js_module_names: Vec<String> = js_only_modules.iter()
            .map(|m| m.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        self.ui.show_processing_phase(&js_module_names, "⚡ JS");
        let js_content = self.js_processor.bundle_modules(&js_only_modules).await?;

        // 🎨 CSS PROCESSING
        // Include both original CSS files and CSS modules found through imports
        let mut all_css_files = structure.css_files.clone();
        for css_module in &css_modules {
            all_css_files.push(css_module.path.clone());
        }

        let css_names: Vec<String> = all_css_files.iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        self.ui.show_processing_phase(&css_names, "🎨 CSS");
        let css_content = self.css_processor.bundle_css(&all_css_files).await?;

        // 💾 WRITE FILES
        let output_files = self.write_output_files(config, &js_content, &css_content).await?;

        let build_time = build_start.elapsed();

        // 🎉 EPIC COMPLETION SHOWCASE!
        let completion_stats = CompletionStats {
            js_count: js_only_modules.len(),
            css_count: all_css_files.len(),
            tree_shaking_info: if let Some(ref stats) = tree_shaking_stats {
                format!("{}% reduction, {} exports removed",
                    stats.reduction_percentage as u32,
                    stats.removed_exports
                )
            } else {
                "disabled (fast mode)".to_string()
            },
            output_files: output_files.iter().map(|f| OutputFileInfo {
                name: f.path.file_name().unwrap().to_str().unwrap().to_string(),
                size: f.size,
            }).collect(),
        };

        self.ui.show_epic_completion(completion_stats);

        Ok(BuildResult {
            js_modules_processed: js_only_modules.len(),
            css_files_processed: all_css_files.len(),
            tree_shaking_stats,
            build_time,
            output_files,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }
}