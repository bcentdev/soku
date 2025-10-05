// Example: Basic Plugin Usage
//
// This example shows how to create and register a custom plugin
// that logs build events and modifies module content.

use ultra::utils::{Plugin, PluginContext, Result};
use ultra::core::models::ModuleInfo;
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

    async fn after_build(&self, context: &PluginContext, result: &ultra::core::models::BuildResult) -> Result<()> {
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
    // Create build service
    let fs_service = Arc::new(ultra::infrastructure::BasicFileSystemService::new());
    let js_processor = Arc::new(ultra::infrastructure::EnhancedJsProcessor::new());
    let css_processor = Arc::new(ultra::infrastructure::LightningCssProcessor::new());

    // Create and register plugin
    let service = ultra::core::UltraBuildService::new(fs_service, js_processor, css_processor)
        .with_plugin(Arc::new(TimingPlugin::new()));

    // Build configuration
    let config = ultra::core::models::BuildConfig {
        root: std::path::PathBuf::from("./demo-project"),
        outdir: std::path::PathBuf::from("./demo-project/dist"),
        entry: std::path::PathBuf::from("./demo-project/main.js"),
        minify: false,
        enable_source_maps: true,
        enable_tree_shaking: false,
        enable_hmr: false,
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
