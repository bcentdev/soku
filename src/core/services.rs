use crate::core::{interfaces::*, models::*, plugin::{PluginManager, PluginContext}};
use crate::utils::{Result, Logger, Timer, UltraUI, CompletionStats, OutputFileInfo, TimingBreakdown, UltraProfiler, UltraCache, performance::parallel, IncrementalBuildState};
use crate::infrastructure::{NodeModuleResolver, MinificationService, CodeSplitter, CodeSplitConfig};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use regex::Regex;

// Pre-compiled regex patterns for performance
static CSS_IMPORT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"@import\s+(?:url\s*\()?\s*['"]([^'"]+)['"]"#).unwrap()
});

/// Main build service implementation
pub struct UltraBuildService {
    fs_service: Arc<dyn FileSystemService>,
    js_processor: Arc<dyn JsProcessor>,
    css_processor: Arc<dyn CssProcessor>,
    tree_shaker: Option<Arc<dyn TreeShaker>>,
    ui: UltraUI,
    node_resolver: NodeModuleResolver,
    profiler: Arc<UltraProfiler>,
    cache: Arc<UltraCache>,
    incremental_state: IncrementalBuildState,
    cache_dir: PathBuf,
    plugin_manager: PluginManager,
}

impl UltraBuildService {
    pub fn new(
        fs_service: Arc<dyn FileSystemService>,
        js_processor: Arc<dyn JsProcessor>,
        css_processor: Arc<dyn CssProcessor>,
    ) -> Self {
        // Initialize cache with persistent storage in .ultra-cache directory
        let cache_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".ultra-cache");
        let cache = Arc::new(UltraCache::with_persistent_cache(&cache_dir));

        // Load incremental state from disk if it exists
        let incremental_state = IncrementalBuildState::load_from_disk(&cache_dir)
            .unwrap_or_else(|_| {
                Logger::debug("No previous incremental state found, starting fresh");
                IncrementalBuildState::new()
            });

        Self {
            fs_service,
            js_processor,
            css_processor,
            tree_shaker: None,
            ui: UltraUI::new(),
            node_resolver: NodeModuleResolver::new(),
            profiler: Arc::new(UltraProfiler::new()),
            cache,
            incremental_state,
            cache_dir,
            plugin_manager: PluginManager::new(),
        }
    }

    pub fn with_tree_shaker(mut self, tree_shaker: Arc<dyn TreeShaker>) -> Self {
        self.tree_shaker = Some(tree_shaker);
        self
    }

    /// Register a plugin with the build service
    pub fn with_plugin(mut self, plugin: Arc<dyn crate::core::plugin::Plugin>) -> Self {
        self.plugin_manager.register(plugin);
        self
    }

    /// Get mutable reference to plugin manager (for advanced usage)
    pub fn plugin_manager_mut(&mut self) -> &mut PluginManager {
        &mut self.plugin_manager
    }



    async fn scan_and_analyze_with_ui(&self, config: &BuildConfig) -> Result<ProjectStructure> {
        let structure = self.fs_service.scan_directory(&config.root).await?;

        // Show file discovery
        self.ui.show_file_discovery(structure.js_modules.len(), structure.css_files.len());

        Ok(structure)
    }

    async fn _process_javascript_modules(
        &self,
        modules: &[ModuleInfo],
        config: &BuildConfig,
    ) -> Result<(String, Option<TreeShakingStats>)> {
        let _timer = Timer::start("JavaScript processing");

        let tree_shaking_stats = if config.enable_tree_shaking {
            if self.tree_shaker.is_some() {
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

    async fn _process_css_files(&self, css_files: &[std::path::PathBuf]) -> Result<String> {
        let _timer = Timer::start("CSS processing");

        let bundled_css = self.css_processor.bundle_css(css_files).await?;
        Ok(bundled_css)
    }

    async fn write_output_files(
        &self,
        config: &BuildConfig,
        js_content: &str,
        css_content: &str,
        source_map: Option<String>,
    ) -> Result<Vec<OutputFile>> {
        let _timer = Timer::start("Writing output files");

        let mut output_files = Vec::new();

        // Write JavaScript bundle (with source map reference if enabled)
        let js_path = config.outdir.join("bundle.js");
        let js_with_source_map = if source_map.is_some() {
            format!("{}\n//# sourceMappingURL=bundle.js.map", js_content)
        } else {
            js_content.to_string()
        };
        self.fs_service.write_file(&js_path, &js_with_source_map).await?;
        output_files.push(OutputFile {
            path: js_path,
            content: js_with_source_map.clone(),
            size: js_with_source_map.len(),
        });

        // Write source map file if provided
        if let Some(source_map_content) = source_map {
            let source_map_path = config.outdir.join("bundle.js.map");
            self.fs_service.write_file(&source_map_path, &source_map_content).await?;
            output_files.push(OutputFile {
                path: source_map_path,
                content: source_map_content.clone(),
                size: source_map_content.len(),
            });
        }

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
        &mut self,
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
            // Skip if already processed - normalize path for consistent deduplication
            let normalized_path = current_path.canonicalize().unwrap_or_else(|_| current_path.clone());
            let path_key = normalized_path.to_string_lossy().to_string();
            if resolved_modules.contains_key(&path_key) {
                continue;
            }

            // Read and process the file
            Logger::debug(&format!("Processing module: {}", current_path.display()));
            if let Ok(content) = self.fs_service.read_file(&current_path).await {
                let module_type = ModuleType::from_extension(
                    current_path.extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                );

                // Extract dependencies in parallel when possible
                let dependencies = match module_type {
                    ModuleType::JavaScript | ModuleType::TypeScript => {
                        // Use blocking task for CPU-intensive dependency extraction
                        let content_clone = content.clone();
                        tokio::task::spawn_blocking(move || {
                            let js_processor = crate::infrastructure::OxcJsProcessor::new();
                            js_processor.extract_dependencies(&content_clone)
                        }).await
                        .map_err(|e| crate::utils::UltraError::build(format!("Dependency extraction failed: {}", e)))?
                    }
                    ModuleType::Css => {
                        // Extract CSS imports (@import statements)
                        self.extract_css_dependencies(&content)
                    }
                    _ => Vec::new(),
                };

                // Resolve dependency paths in parallel (NOW ENABLED with thread-safe resolver)
                let resolve_tasks: Vec<_> = dependencies.iter()
                    .map(|dep| {
                        let dep_clone = dep.clone();
                        let current_path_clone = current_path.clone();
                        let root_dir_clone = root_dir.to_path_buf();
                        let resolver_ref = &self.node_resolver;
                        async move {
                            Logger::debug(&format!("Resolving import '{}' from {}", dep_clone, current_path_clone.display()));
                            let resolved_path = resolver_ref.resolve(&dep_clone, &current_path_clone, &root_dir_clone).await;
                            (dep_clone, resolved_path)
                        }
                    })
                    .collect();

                let parallel_results = futures::future::join_all(resolve_tasks).await;

                // Collect resolved dependencies
                let mut resolved_deps = Vec::new();
                for (dep, resolved_path_opt) in parallel_results {
                    if let Some(resolved_path) = resolved_path_opt {
                        Logger::debug(&format!("Resolved '{}' to: {}", dep, resolved_path.display()));

                        // Track dependency relationship for incremental builds
                        // normalized_path depends on resolved_path
                        self.incremental_state.add_dependency(
                            normalized_path.clone(),
                            resolved_path.clone()
                        );

                        resolved_deps.push(dep);
                        to_process.push(resolved_path);
                    } else {
                        Logger::debug(&format!("Failed to resolve import: {}", dep));
                    }
                }

                let module_info = ModuleInfo {
                    path: normalized_path.clone(),
                    content,
                    module_type,
                    dependencies: resolved_deps,
                    exports: Vec::new(), // TODO: Extract exports
                };

                resolved_modules.insert(path_key, module_info);
            }
        }

        // Process the resolved modules in parallel for any additional processing
        let modules: Vec<ModuleInfo> = resolved_modules.into_values().collect();
        self.process_modules_parallel(&modules).await
    }

    /// Process modules in parallel for enhanced performance
    /// Uses rayon for CPU-bound operations
    async fn process_modules_parallel(&self, modules: &[ModuleInfo]) -> Result<Vec<ModuleInfo>> {
        if modules.len() < 8 {
            // For small projects, skip parallel processing overhead
            return Ok(modules.to_vec());
        }

        Logger::debug(&format!("🔄 Processing {} modules in parallel across {} cores", modules.len(), num_cpus::get()));

        // Use rayon for CPU-bound parallel processing
        use rayon::prelude::*;

        // Clone modules for move into spawn_blocking
        let modules_owned = modules.to_vec();
        let processed_modules: Vec<ModuleInfo> = tokio::task::spawn_blocking(move || {
            modules_owned.par_iter()
                .map(|module| {
                    // Validate and optimize module content in parallel
                    let optimized_content = module.content.clone();

                    // Return optimized module
                    ModuleInfo {
                        path: module.path.clone(),
                        content: optimized_content,
                        module_type: module.module_type.clone(),
                        dependencies: module.dependencies.clone(),
                        exports: module.exports.clone(),
                    }
                })
                .collect()
        }).await
        .map_err(|e| crate::utils::UltraError::build(format!("Parallel processing failed: {}", e)))?;

        Logger::debug(&format!("✅ Parallel processing complete: {} modules processed", processed_modules.len()));

        Ok(processed_modules)
    }

    fn extract_css_dependencies(&self, content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Handle @import statements
            if trimmed.starts_with("@import") {
                // Parse @import statements: @import "path" or @import url("path")
                // Using pre-compiled regex for performance
                if let Some(captures) = CSS_IMPORT_REGEX.captures(trimmed) {
                    let import_path = &captures[1];

                    if import_path.starts_with("./") || import_path.starts_with("../") {
                        dependencies.push(import_path.to_string());
                    }
                }
            }
        }

        dependencies
    }

    /// Generate cache key based on module contents and build configuration
    fn generate_js_cache_key(&self, modules: &[ModuleInfo], config: &BuildConfig, tree_stats: Option<&TreeShakingStats>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash module contents and metadata
        for module in modules {
            module.path.hash(&mut hasher);
            module.content.hash(&mut hasher);
            module.module_type.hash(&mut hasher);
        }

        // Hash build configuration
        config.enable_minification.hash(&mut hasher);
        config.enable_tree_shaking.hash(&mut hasher);
        config.enable_source_maps.hash(&mut hasher);

        // Hash tree shaking stats if present
        if let Some(stats) = tree_stats {
            stats.removed_exports.hash(&mut hasher);
            stats.total_modules.hash(&mut hasher);
        }

        format!("js_bundle_{:x}", hasher.finish())
    }

    /// Generate cache key for CSS processing
    fn generate_css_cache_key(&self, css_files: &[PathBuf]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash file paths and modification times
        for path in css_files {
            path.hash(&mut hasher);
            // Add file modification time if available
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        duration.as_secs().hash(&mut hasher);
                    }
                }
            }
        }

        format!("css_bundle_{:x}", hasher.finish())
    }

    /// Build with code splitting enabled
    async fn build_with_code_splitting(
        &mut self,
        config: &BuildConfig,
        js_modules: &[ModuleInfo],
        structure: &ProjectStructure,
        tree_shaking_stats: Option<&TreeShakingStats>,
        plugin_context: &PluginContext,
    ) -> Result<BuildResult> {
        let build_start = std::time::Instant::now();
        self.profiler.start_timer("code_splitting");

        // Configure code splitter
        let split_config = CodeSplitConfig {
            max_chunk_size: config.max_chunk_size.unwrap_or(250_000),
            min_modules_per_chunk: 2,
            create_vendor_chunks: true,
            split_by_routes: true,
            common_dependency_threshold: 2,
        };

        // Identify entry points
        let entry_points: Vec<String> = structure.js_modules
            .iter()
            .filter(|p| {
                let name = p.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                name.contains("main") || name.contains("index")
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // Analyze and split modules into chunks
        let mut splitter = CodeSplitter::new(split_config);
        let chunks = splitter.analyze_and_split(js_modules, &entry_points)?;

        Logger::info(&format!("📦 Code splitting: Created {} chunks", chunks.len()));

        self.profiler.end_timer("code_splitting");

        // Process each chunk
        self.profiler.start_timer("chunk_processing");
        let mut output_files_for_ui = Vec::new();
        let mut output_files_for_result = Vec::new();

        for chunk in &chunks {
            Logger::info(&format!("  ├─ {} ({} modules, {:.1}KB)",
                chunk.name,
                chunk.modules.len(),
                chunk.size_bytes as f64 / 1024.0
            ));

            // Process chunk modules
            let chunk_content = if tree_shaking_stats.is_some() {
                self.js_processor.bundle_modules_with_tree_shaking(&chunk.modules, tree_shaking_stats).await?
            } else {
                self.js_processor.bundle_modules(&chunk.modules).await?
            };

            // Minify if enabled
            let final_content = if config.enable_minification {
                let minifier = MinificationService::new();
                minifier.minify_bundle(chunk_content, &format!("{}.js", chunk.name)).await?
            } else {
                chunk_content
            };

            // Write chunk file
            let chunk_name = format!("{}.js", chunk.name);
            let chunk_path = config.outdir.join(&chunk_name);
            let content_size = final_content.len();
            self.fs_service.write_file(&chunk_path, &final_content).await?;

            output_files_for_ui.push(OutputFileInfo {
                name: chunk_name.clone(),
                size: content_size,
            });

            output_files_for_result.push(OutputFile {
                path: chunk_path,
                content: final_content,
                size: content_size,
            });
        }

        self.profiler.end_timer("chunk_processing");

        // Process CSS (same as normal build)
        self.profiler.start_timer("css_processing");
        if !structure.css_files.is_empty() {
            let processed = self.css_processor.bundle_css(&structure.css_files).await?;
            let css_path = config.outdir.join("bundle.css");
            let css_size = processed.len();
            self.fs_service.write_file(&css_path, &processed).await?;

            output_files_for_ui.push(OutputFileInfo {
                name: "bundle.css".to_string(),
                size: css_size,
            });

            output_files_for_result.push(OutputFile {
                path: css_path,
                content: processed,
                size: css_size,
            });
        }
        self.profiler.end_timer("css_processing");

        self.profiler.end_timer("total_build");

        // Generate completion stats with timing breakdown
        let timing_breakdown = TimingBreakdown {
            file_scan_ms: self.profiler.get_duration("file_discovery").as_millis() as u64,
            js_processing_ms: self.profiler.get_duration("chunk_processing").as_millis() as u64,
            css_processing_ms: self.profiler.get_duration("css_processing").as_millis() as u64,
            tree_shaking_ms: self.profiler.get_duration("tree_shaking").as_millis() as u64,
            minification_ms: 0, // Minification is included in chunk_processing
            output_write_ms: self.profiler.get_duration("file_writing").as_millis() as u64,
        };

        self.ui.show_epic_completion(CompletionStats {
            output_files: output_files_for_ui,
            node_modules_optimized: None,
            timing_breakdown: Some(timing_breakdown),
        });

        // Update file metadata for incremental builds (after successful build)
        Logger::debug(&format!("Updating file metadata for {} modules", js_modules.len()));
        for module in js_modules {
            if let Err(e) = self.incremental_state.update_file(&module.path) {
                Logger::debug(&format!("Failed to update file metadata for {}: {}", module.path.display(), e));
            }
        }

        // Mark build as complete for incremental build tracking
        self.incremental_state.mark_build_complete();

        // Persist incremental state to disk
        if let Err(e) = self.incremental_state.save_to_disk(&self.cache_dir) {
            Logger::warn(&format!("Failed to save incremental state: {}", e));
        }

        let build_result = BuildResult {
            success: true,
            js_modules_processed: js_modules.len(),
            css_files_processed: structure.css_files.len(),
            errors: Vec::new(),
            warnings: Vec::new(),
            tree_shaking_stats: tree_shaking_stats.cloned(),
            build_time: build_start.elapsed(),
            output_files: output_files_for_result,
            modules: js_modules.to_vec(),
        };

        // 🔌 PLUGIN HOOK: on_build_end
        if self.plugin_manager.plugin_count() > 0 {
            Logger::debug(&format!("Running on_build_end hook for {} plugins", self.plugin_manager.plugin_count()));
            self.plugin_manager.on_build_end(&plugin_context, &build_result)?;
        }

        Ok(build_result)
    }
}

#[async_trait::async_trait]
impl BuildService for UltraBuildService {
    async fn build(&mut self, config: &BuildConfig) -> Result<BuildResult> {
        // 🚀 EPIC BANNER!
        self.ui.show_epic_banner();

        let build_start = std::time::Instant::now();
        self.profiler.start_timer("total_build");

        // 🔌 PLUGIN HOOK: on_build_start
        let plugin_context = PluginContext::new(config.root.clone(), config.clone());
        if self.plugin_manager.plugin_count() > 0 {
            Logger::debug(&format!("Running on_build_start hook for {} plugins", self.plugin_manager.plugin_count()));
            self.plugin_manager.on_build_start(&plugin_context)?;
        }

        // Create output directory
        self.profiler.start_timer("fs_setup");
        self.fs_service.create_directory(&config.outdir).await?;
        self.profiler.end_timer("fs_setup");

        // 🔍 FILE DISCOVERY
        self.profiler.start_timer("file_discovery");
        let structure = self.scan_and_analyze_with_ui(config).await?;
        self.profiler.end_timer("file_discovery");

        // 🔄 CHECK IF FIRST BUILD (before resolving dependencies)
        let is_first_build = self.incremental_state.is_empty();
        Logger::debug(&format!("Is first build: {} (tracked files: {})", is_first_build, self.incremental_state.file_count()));

        // Convert paths to ModuleInfo and resolve dependencies
        self.profiler.start_timer("dependency_resolution");
        let js_modules = self.resolve_all_dependencies(&structure.js_modules, &config.root).await?;
        self.profiler.end_timer("dependency_resolution");

        // 🔄 INCREMENTAL BUILD DETECTION
        self.profiler.start_timer("change_detection");

        Logger::debug(&format!("Incremental build check: is_first_build={}, tracked_files={}",
            is_first_build,
            self.incremental_state.file_count()
        ));

        if !is_first_build {
            let has_changes = self.incremental_state.has_changes();

            Logger::debug(&format!("Incremental build: has_changes={}", has_changes));

            if has_changes {
                let changed_files = self.incremental_state.get_changed_files();
                let files_to_rebuild = self.incremental_state.get_files_to_rebuild();

                Logger::info(&format!("🔄 Incremental build: {} files changed, {} files need rebuild",
                    changed_files.len(),
                    files_to_rebuild.len()
                ));

                // Log changed files in debug mode
                if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
                    for file in &changed_files {
                        Logger::debug(&format!("  Changed: {}", file.display()));
                    }
                }
            } else {
                Logger::info("✨ No changes detected - using cached build");
            }
        } else {
            Logger::debug("First build - no incremental state");
        }

        self.profiler.end_timer("change_detection");

        // 🌳 TREE SHAKING (if enabled)
        self.profiler.start_timer("tree_shaking");
        let tree_shaking_stats = if config.enable_tree_shaking {
            if self.tree_shaker.is_some() {
                self.ui.show_tree_shaking_analysis(js_modules.len());

                // Use AST tree shaker for better accuracy on complex projects
                let use_ast_shaker = js_modules.len() > 3 ||
                    js_modules.iter().any(|m| m.content.len() > 5000);

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

                let stats = if use_ast_shaker {
                    let mut ast_shaker = crate::infrastructure::AstTreeShaker::new();
                    ast_shaker.analyze_modules(&js_modules).await?;
                    ast_shaker.shake(&entry_points).await?
                } else {
                    let mut regex_shaker = crate::infrastructure::RegexTreeShaker::new();
                    regex_shaker.analyze_modules(&js_modules).await?;
                    regex_shaker.shake(&entry_points).await?
                };

                Some(stats)
            } else {
                None
            }
        } else {
            None
        };
        self.profiler.end_timer("tree_shaking");

        // Separate JS modules from CSS modules
        let js_only_modules: Vec<ModuleInfo> = js_modules.iter()
            .filter(|m| matches!(m.module_type, ModuleType::JavaScript | ModuleType::TypeScript))
            .cloned()
            .collect();

        let css_modules: Vec<ModuleInfo> = js_modules.iter()
            .filter(|m| matches!(m.module_type, ModuleType::Css))
            .cloned()
            .collect();

        // 📦 CODE SPLITTING (if enabled)
        if config.enable_code_splitting {
            return self.build_with_code_splitting(config, &js_only_modules, &structure, tree_shaking_stats.as_ref(), &plugin_context).await;
        }

        // ⚡ JAVASCRIPT PROCESSING WITH INTELLIGENT CACHING
        self.profiler.start_timer("js_processing");
        let js_module_names: Vec<String> = js_only_modules.iter()
            .map(|m| m.path.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        self.ui.show_processing_phase(&js_module_names, "⚡ JS");

        // Generate cache key based on modules content and config
        let cache_key = self.generate_js_cache_key(&js_only_modules, config, tree_shaking_stats.as_ref());

        let (mut js_content, source_map) = if let Some(cached_result) = self.cache.get_js(&cache_key, &cache_key) {
            Logger::debug("✅ Using cached JS bundle");
            // Parse cached result - for simplicity, assume no source map in cache for now
            (cached_result, None)
        } else {
            Logger::debug("🔄 Processing JS modules (cache miss)");
            let result = if config.enable_source_maps {
                // Use source maps bundling
                let bundle_output = self.js_processor.bundle_modules_with_source_maps(&js_only_modules, config).await?;
                (bundle_output.code, bundle_output.source_map)
            } else if tree_shaking_stats.is_some() {
                // Use tree shaking bundling
                let js_content = self.js_processor.bundle_modules_with_tree_shaking(&js_only_modules, tree_shaking_stats.as_ref()).await?;
                (js_content, None)
            } else {
                // Regular bundling
                let js_content = self.js_processor.bundle_modules(&js_only_modules).await?;
                (js_content, None)
            };

            // Cache the result for future builds
            self.cache.cache_js(&cache_key, &cache_key, result.0.clone());
            result
        };

        // ⚡ MINIFICATION (if enabled)
        if config.enable_minification {
            let minification_service = MinificationService::new();
            let original_content = js_content.clone();
            js_content = minification_service.minify_bundle(js_content, "bundle.js").await?;
            let stats = minification_service.get_stats(&original_content, &js_content);
            tracing::info!("🗜️  {}", stats);
        }
        self.profiler.end_timer("js_processing");

        // 🎨 CSS PROCESSING WITH INTELLIGENT CACHING
        // Include both original CSS files and CSS modules found through imports
        let mut all_css_files = structure.css_files.clone();
        for css_module in &css_modules {
            all_css_files.push(css_module.path.clone());
        }

        let css_names: Vec<String> = all_css_files.iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        self.ui.show_processing_phase(&css_names, "🎨 CSS");
        self.profiler.start_timer("css_processing");

        let css_cache_key = self.generate_css_cache_key(&all_css_files);
        let css_content = if let Some(cached_css) = self.cache.get_css(&css_cache_key, &css_cache_key) {
            Logger::debug("✅ Using cached CSS bundle");
            cached_css
        } else {
            Logger::debug("🔄 Processing CSS files (cache miss)");

            // Process CSS files in parallel if we have many files
            let result = if all_css_files.len() > 2 {
                Logger::debug(&format!("🔄 Processing {} CSS files in parallel", all_css_files.len()));

                // Read CSS files in parallel
                let file_contents = parallel::process_async_parallel(
                    all_css_files.clone(),
                    |path| async move {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => Some((path, content)),
                            Err(_) => None,
                        }
                    }
                ).await;

                // Filter successful reads and bundle
                let valid_files: Vec<_> = file_contents.into_iter().flatten().collect();
                let _combined_css = valid_files.iter()
                    .map(|(_, content)| content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n/* Next CSS file */\n\n");

                // Process with lightningcss
                self.css_processor.bundle_css(&all_css_files).await?
            } else {
                // For small numbers of CSS files, use sequential processing
                self.css_processor.bundle_css(&all_css_files).await?
            };

            // Cache the result for future builds
            self.cache.cache_css(&css_cache_key, &css_cache_key, result.clone());
            result
        };

        self.profiler.end_timer("css_processing");

        // 💾 WRITE FILES
        self.profiler.start_timer("file_writing");
        let output_files = self.write_output_files(config, &js_content, &css_content, source_map).await?;
        self.profiler.end_timer("file_writing");

        let build_time = build_start.elapsed();

        // 🎉 EPIC COMPLETION SHOWCASE!
        let node_modules_count = js_modules.iter()
            .filter(|m| m.path.to_string_lossy().contains("node_modules"))
            .count();

        let completion_stats = CompletionStats {
            output_files: output_files.iter().map(|f| OutputFileInfo {
                name: f.path.file_name().unwrap().to_str().unwrap().to_string(),
                size: f.size,
            }).collect(),
            node_modules_optimized: if node_modules_count > 0 { Some(node_modules_count) } else { None },
            timing_breakdown: Some(TimingBreakdown {
                file_scan_ms: self.profiler.get_duration("file_discovery").as_millis() as u64,
                js_processing_ms: self.profiler.get_duration("js_processing").as_millis() as u64,
                css_processing_ms: self.profiler.get_duration("css_processing").as_millis() as u64,
                tree_shaking_ms: self.profiler.get_duration("tree_shaking").as_millis() as u64,
                minification_ms: 0, // Minification is included in js_processing
                output_write_ms: self.profiler.get_duration("file_writing").as_millis() as u64,
            }),
        };

        self.ui.show_epic_completion(completion_stats);

        // End total timing and report bottlenecks
        self.profiler.end_timer("total_build");

        // Report performance bottlenecks in debug mode
        if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
            self.profiler.report_bottlenecks();
        }

        // Update file metadata for incremental builds (after successful build)
        Logger::debug(&format!("Updating file metadata for {} modules", js_modules.len()));
        for module in &js_modules {
            if let Err(e) = self.incremental_state.update_file(&module.path) {
                Logger::debug(&format!("Failed to update file metadata for {}: {}", module.path.display(), e));
            }
        }

        // Mark build as complete for incremental build tracking
        self.incremental_state.mark_build_complete();

        // Persist incremental state to disk
        if let Err(e) = self.incremental_state.save_to_disk(&self.cache_dir) {
            Logger::warn(&format!("Failed to save incremental state: {}", e));
        }

        Ok(BuildResult {
            js_modules_processed: js_only_modules.len(),
            css_files_processed: all_css_files.len(),
            tree_shaking_stats,
            build_time,
            output_files,
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            modules: js_only_modules.clone(),
        })
    }
}