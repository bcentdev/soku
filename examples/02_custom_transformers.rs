// Example: Custom Transformers Usage
//
// This example shows how to use custom transformers to modify
// code during the build process.

use ultra::utils::{CustomTransformer, BuiltInTransformers, TransformerBuilder, Result};
use ultra::core::interfaces::BuildService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create build service with current APIs
    let fs_service = Arc::new(ultra::infrastructure::TokioFileSystemService);
    let js_processor = Arc::new(ultra::infrastructure::UnifiedJsProcessor::new(
        ultra::infrastructure::ProcessingStrategy::Standard
    ));
    let css_processor = Arc::new(ultra::infrastructure::LightningCssProcessor::new(false));

    // Example 1: Remove console.log statements in production
    let remove_logs = BuiltInTransformers::remove_console_logs();

    // Example 2: Remove debugger statements
    let remove_debugger = BuiltInTransformers::remove_debugger();

    // Example 3: Add 'use strict' directive
    let add_strict = BuiltInTransformers::add_use_strict();

    // Example 4: Custom regex transformer - replace API endpoint
    let replace_api = CustomTransformer::regex(
        "replace-api-endpoint",
        "https://api.dev.example.com",
        "https://api.prod.example.com"
    );

    // Example 5: Custom function transformer - add build info
    let add_build_info = CustomTransformer::function(
        "add-build-info",
        |code| {
            let build_time = chrono::Utc::now().to_rfc3339();
            let header = format!(
                "// Built at: {}\n\
                 // Environment: production\n\n",
                build_time
            );
            Ok(format!("{}{}", header, code))
        }
    );

    // Example 6: Conditional transformer - only for test files
    let test_transformer = BuiltInTransformers::test_only(
        ultra::utils::TransformerType::Regex {
            pattern: "describe\\(".to_string(),
            replacement: "test(".to_string(),
        }
    );

    // Example 7: Build a transformer chain
    let service = ultra::core::services::UltraBuildService::new(fs_service, js_processor, css_processor)
        .with_transformer(remove_logs)
        .with_transformer(remove_debugger)
        .with_transformer(add_strict)
        .with_transformer(replace_api)
        .with_transformer(add_build_info);

    // Build configuration
    let config = ultra::core::models::BuildConfig {
        root: std::path::PathBuf::from("./demo-project"),
        outdir: std::path::PathBuf::from("./demo-project/dist"),
        enable_tree_shaking: true,
        enable_minification: true,
        enable_source_maps: true,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "production".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),
    };

    // Run build
    let mut service = service;
    let result = service.build(&config).await?;

    println!("✨ Build complete with transformers applied!");
    println!("   - Removed console.log statements");
    println!("   - Removed debugger statements");
    println!("   - Added 'use strict' directive");
    println!("   - Replaced API endpoints");
    println!("   - Added build timestamp");
    println!("\n📦 Output files:");
    for output in &result.output_files {
        println!("   - {} ({} bytes)",
            output.path.display(),
            output.size
        );
    }

    Ok(())
}
