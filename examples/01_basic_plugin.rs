// Example: Basic Plugin Usage
//
// This example shows how to create and register a custom plugin
// that logs build events and modifies module content.

use soku::utils::{Plugin, PluginContext, Result};
use soku::core::models::ModuleInfo;
use soku::core::interfaces::BuildService;
use async_trait::async_trait;
use std::sync::Arc;

/// Custom plugin that logs build events
struct TimingPlugin {
    name: String,
    start_time: std::sync::Mutex<Option<std::time::Instant>>,
}

impl TimingPlugin {
    pub fn new() -> Self {
        Self {
            name: "timing-plugin".to_string(),
            start_time: std::sync::Mutex::new(None),
        }
    }
}

#[async_trait]
impl Plugin for TimingPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_build(&self, context: &PluginContext) -> Result<()> {
        *self.start_time.lock().unwrap() = Some(std::time::Instant::now());
        println!("🔌 [{}] Build starting with {} modules",
            self.name,
            context.modules.len()
        );
        Ok(())
    }

    async fn after_build(&self, _context: &PluginContext, result: &soku::core::models::BuildResult) -> Result<()> {
        if let Some(start) = *self.start_time.lock().unwrap() {
            let duration = start.elapsed();
            println!("🔌 [{}] Build completed in {:?}", self.name, duration);
            println!("   - JS modules: {}", result.js_modules_processed);
            println!("   - CSS files: {}", result.css_files_processed);
            println!("   - Output files: {}", result.output_files.len());
        }
        Ok(())
    }

    async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
        // Add a comment header to each module
        let header = format!(
            "// ================================================\n\
             // Module: {}\n\
             // Processed by: {}\n\
             // ================================================\n\n",
            module.path.display(),
            self.name
        );
        Ok(format!("{}{}", header, code))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create build service with current APIs
    let fs_service = Arc::new(soku::infrastructure::TokioFileSystemService);
    let js_processor = Arc::new(soku::infrastructure::UnifiedJsProcessor::new(
        soku::infrastructure::ProcessingStrategy::Standard
    ));
    let css_processor = Arc::new(soku::infrastructure::LightningCssProcessor::new(false));

    // Create and register plugin
    let service = soku::core::services::SokuBuildService::new(fs_service, js_processor, css_processor)
        .with_plugin(Arc::new(TimingPlugin::new()));

    // Build configuration
    let config = soku::core::models::BuildConfig {
        root: std::path::PathBuf::from("./demo-project"),
        outdir: std::path::PathBuf::from("./demo-project/dist"),
        enable_tree_shaking: false,
        enable_minification: false,
        enable_source_maps: true,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),
    };

    // Run build
    let mut service = service;
    let result = service.build(&config).await?;

    println!("\n✨ Build successful! Output files:");
    for output in &result.output_files {
        println!("   - {} ({} bytes)",
            output.path.display(),
            output.size
        );
    }

    Ok(())
}
