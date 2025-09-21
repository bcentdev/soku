use crate::core::{interfaces::*, models::*};
use crate::utils::{Result, Logger, Timer, UltraUI, CompletionStats, OutputFileInfo};
use std::sync::Arc;

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

        // Convert paths to ModuleInfo
        let mut js_modules = Vec::new();
        for path in &structure.js_modules {
            let content = self.fs_service.read_file(path).await?;
            let module_type = ModuleType::from_extension(
                path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
            );

            js_modules.push(ModuleInfo {
                path: path.clone(),
                content,
                module_type,
                dependencies: Vec::new(),
                exports: Vec::new(),
            });
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

        // ⚡ JAVASCRIPT PROCESSING
        let module_names: Vec<String> = js_modules.iter()
            .map(|m| m.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        self.ui.show_processing_phase(&module_names, "⚡ JS");
        let js_content = self.js_processor.bundle_modules(&js_modules).await?;

        // 🎨 CSS PROCESSING
        let css_names: Vec<String> = structure.css_files.iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        self.ui.show_processing_phase(&css_names, "🎨 CSS");
        let css_content = self.css_processor.bundle_css(&structure.css_files).await?;

        // 💾 WRITE FILES
        let output_files = self.write_output_files(config, &js_content, &css_content).await?;

        let build_time = build_start.elapsed();

        // 🎉 EPIC COMPLETION SHOWCASE!
        let completion_stats = CompletionStats {
            js_count: structure.js_modules.len(),
            css_count: structure.css_files.len(),
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
            js_modules_processed: structure.js_modules.len(),
            css_files_processed: structure.css_files.len(),
            tree_shaking_stats,
            build_time,
            output_files,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }
}