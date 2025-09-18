use crate::core::{interfaces::*, models::*};
use crate::utils::{Result, Logger, Timer};
use std::sync::Arc;

/// Main build service implementation
pub struct UltraBuildService {
    fs_service: Arc<dyn FileSystemService>,
    js_processor: Arc<dyn JsProcessor>,
    css_processor: Arc<dyn CssProcessor>,
    tree_shaker: Option<Arc<dyn TreeShaker>>,
    cache_service: Option<Arc<dyn CacheService>>,
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
            cache_service: None,
        }
    }

    pub fn with_tree_shaker(mut self, tree_shaker: Arc<dyn TreeShaker>) -> Self {
        self.tree_shaker = Some(tree_shaker);
        self
    }

    pub fn with_cache(mut self, cache_service: Arc<dyn CacheService>) -> Self {
        self.cache_service = Some(cache_service);
        self
    }

    async fn scan_and_analyze(&self, config: &BuildConfig) -> Result<ProjectStructure> {
        let _timer = Timer::start("File scanning");
        Logger::scanning_files();

        let structure = self.fs_service.scan_directory(&config.root).await?;
        Logger::found_files(structure.js_modules.len(), structure.css_files.len());

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
        let build_start = std::time::Instant::now();

        Logger::build_start(
            &config.root.to_string_lossy(),
            &config.outdir.to_string_lossy(),
        );

        // Create output directory
        self.fs_service.create_directory(&config.outdir).await?;

        // Scan project structure
        let structure = self.scan_and_analyze(config).await?;

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
                dependencies: Vec::new(), // TODO: Extract dependencies
                exports: Vec::new(),      // TODO: Extract exports
            });
        }

        // Process JavaScript modules
        let (js_content, tree_shaking_stats) =
            self.process_javascript_modules(&js_modules, config).await?;

        // Process CSS files
        let css_content = self.process_css_files(&structure.css_files).await?;

        // Write output files
        let output_files = self.write_output_files(config, &js_content, &css_content).await?;

        let build_time = build_start.elapsed();

        // Log completion
        let tree_shaking_str = tree_shaking_stats.as_ref().map(|s| s.to_string());
        Logger::build_complete(
            structure.js_modules.len(),
            structure.css_files.len(),
            tree_shaking_str.as_deref(),
            build_time,
            &config.outdir.to_string_lossy(),
        );

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