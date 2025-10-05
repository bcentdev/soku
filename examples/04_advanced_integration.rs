// Example: Advanced Integration - Combining All Features
//
// This example shows how to combine plugins, transformers,
// advanced source maps, and multiple entry points in a real-world scenario.

use soku::utils::{Plugin, PluginContext, CustomTransformer, BuiltInTransformers, Result};
use soku::core::models::{ModuleInfo, BuildConfig};
use soku::core::interfaces::BuildService;
use async_trait::async_trait;
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;

/// Production optimization plugin
struct ProductionPlugin {
    name: String,
}

impl ProductionPlugin {
    pub fn new() -> Self {
        Self {
            name: "production-optimizer".to_string(),
        }
    }
}

#[async_trait]
impl Plugin for ProductionPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_build(&self, context: &PluginContext) -> Result<()> {
        println!("🏭 [Production] Starting optimized build...");
        println!("   - Minification: enabled");
        println!("   - Tree shaking: enabled");
        println!("   - Source maps: enabled (with inline sources)");
        println!("   - Modules: {}", context.modules.len());
        Ok(())
    }

    async fn after_build(&self, _context: &PluginContext, result: &soku::core::models::BuildResult) -> Result<()> {
        // Calculate total output size
        let total_size: usize = result.output_files.iter()
            .map(|f| f.size)
            .sum();

        println!("📊 [Production] Build statistics:");
        println!("   - JS modules processed: {}", result.js_modules_processed);
        println!("   - CSS files processed: {}", result.css_files_processed);
        println!("   - Total output size: {} KB", total_size / 1024);
        println!("   - Build time: {:?}", result.build_time);

        if let Some(stats) = &result.tree_shaking_stats {
            println!("   - Tree shaking: {} exports removed ({:.1}% reduction)",
                stats.removed_exports,
                stats.reduction_percentage
            );
        }

        Ok(())
    }

    async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
        // Add production markers
        if module.path.to_string_lossy().contains("vendor") {
            // Mark vendor code
            Ok(format!("/* VENDOR CODE */\n{}", code))
        } else {
            // Mark app code
            Ok(format!("/* APP CODE */\n{}", code))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Soku Bundler - Advanced Integration Example\n");

    // Create services with current APIs
    let fs_service = Arc::new(soku::infrastructure::TokioFileSystemService);
    let js_processor = Arc::new(soku::infrastructure::UnifiedJsProcessor::new(
        soku::infrastructure::ProcessingStrategy::Enhanced
    ));
    let css_processor = Arc::new(soku::infrastructure::LightningCssProcessor::new(true));
    let tree_shaker = Arc::new(soku::infrastructure::RegexTreeShaker::new());

    // Configure multiple entry points
    let mut entries = HashMap::new();
    entries.insert("main".to_string(), PathBuf::from("./demo-project/main.js"));
    entries.insert("admin".to_string(), PathBuf::from("./demo-project/src/admin.js"));
    entries.insert("worker".to_string(), PathBuf::from("./demo-project/src/worker.js"));

    // Build configuration
    let config = BuildConfig {
        root: PathBuf::from("./demo-project"),
        outdir: PathBuf::from("./demo-project/dist-advanced"),
        enable_tree_shaking: true,
        enable_minification: true,
        enable_source_maps: true,  // Advanced source maps with inline sources
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "production".to_string(),
        alias: HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries,
    };

    // Create build service with all features
    let service = soku::core::services::SokuBuildService::new(fs_service, js_processor, css_processor)
        // Add tree shaking
        .with_tree_shaker(tree_shaker)
        // Add production plugin
        .with_plugin(Arc::new(ProductionPlugin::new()))
        // Add transformers (production optimizations)
        .with_transformer(BuiltInTransformers::remove_console_logs())
        .with_transformer(BuiltInTransformers::remove_debugger())
        .with_transformer(BuiltInTransformers::add_use_strict())
        // Replace development API endpoints with production ones
        .with_transformer(CustomTransformer::regex(
            "replace-api",
            "http://localhost:3000/api",
            "https://api.production.com/v1"
        ))
        // Add build metadata
        .with_transformer(CustomTransformer::function(
            "add-metadata",
            |code| {
                let metadata = format!(
                    "/* Build Info */\n\
                     /* Date: {} */\n\
                     /* Version: 1.0.0 */\n\
                     /* Environment: production */\n\n",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                );
                Ok(format!("{}{}", metadata, code))
            }
        ));

    // Run build
    let mut service = service;
    println!("⚙️  Building with advanced features...\n");
    let result = service.build(&config).await?;

    // Display results
    println!("\n✨ Build completed successfully!\n");
    println!("📦 Output files:");
    for output in &result.output_files {
        let file_type = if output.path.extension().unwrap_or_default() == "map" {
            "Source Map"
        } else if output.path.extension().unwrap_or_default() == "css" {
            "CSS Bundle"
        } else {
            "JS Bundle"
        };
        println!("   - {} ({}, {} KB)",
            output.path.display(),
            file_type,
            output.size / 1024
        );
    }

    println!("\n🎯 Features applied:");
    println!("   ✅ Multiple entry points (3 bundles)");
    println!("   ✅ Advanced source maps with inline sources");
    println!("   ✅ Production plugin with optimization tracking");
    println!("   ✅ Custom transformers (5 transformations)");
    println!("   ✅ Tree shaking enabled");
    println!("   ✅ Minification enabled");

    println!("\n💡 Next steps:");
    println!("   1. Check dist-advanced/ for output files");
    println!("   2. Inspect .map files for advanced source maps");
    println!("   3. Verify transformations were applied");
    println!("   4. Compare bundle sizes with/without optimizations");

    Ok(())
}
