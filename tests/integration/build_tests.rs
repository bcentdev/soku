use soku::core::interfaces::BuildService;
use soku::core::models::BuildConfig;
use soku::infrastructure::processors::ProcessingStrategy;
use soku::infrastructure::{LightningCssProcessor, TokioFileSystemService, UnifiedJsProcessor};
use std::path::PathBuf;

#[tokio::test]
async fn test_simple_project_build() {
    let fixtures_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple-project");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));

    let mut build_service =
        soku::core::services::SokuBuildService::new(fs_service, js_processor, css_processor);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist-simple"),
        enable_tree_shaking: false,
        enable_minification: false,
        enable_source_maps: false,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),
    };

    let result = build_service.build(&config).await;
    assert!(result.is_ok(), "Build should succeed");

    let build_result = result.unwrap();
    assert!(
        !build_result.output_files.is_empty(),
        "Should have output files"
    );

    // Check bundle.js exists
    let bundle_path = config.outdir.join("bundle.js");
    assert!(bundle_path.exists(), "bundle.js should exist");

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}

#[tokio::test]
async fn test_typescript_project_build() {
    let fixtures_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript-project");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Enhanced));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));

    let mut build_service =
        soku::core::services::SokuBuildService::new(fs_service, js_processor, css_processor);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist-typescript"),
        enable_tree_shaking: false,
        enable_minification: false,
        enable_source_maps: false,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),
    };

    let result = build_service.build(&config).await;
    assert!(result.is_ok(), "TypeScript build should succeed");

    let _build_result = result.unwrap();

    // Check that TypeScript build succeeded
    let bundle_path = config.outdir.join("bundle.js");
    assert!(bundle_path.exists(), "bundle.js should exist");

    let bundle_content = std::fs::read_to_string(&bundle_path).unwrap();
    // Verify bundle contains expected JavaScript code
    assert!(
        bundle_content.contains("Calculator"),
        "Bundle should contain Calculator class"
    );
    assert!(
        bundle_content.contains("function"),
        "Bundle should contain function keyword or similar"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}

#[tokio::test]
async fn test_source_maps_generation() {
    let fixtures_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/simple-project");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));

    let mut build_service =
        soku::core::services::SokuBuildService::new(fs_service, js_processor, css_processor);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist-sourcemaps"),
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

    let result = build_service.build(&config).await;
    assert!(result.is_ok(), "Build with source maps should succeed");

    // Check source map file exists
    let source_map_path = config.outdir.join("bundle.js.map");
    assert!(source_map_path.exists(), "bundle.js.map should exist");

    // Check source map content
    let source_map_content = std::fs::read_to_string(&source_map_path).unwrap();
    assert!(
        source_map_content.contains("\"version\"") && source_map_content.contains("3"),
        "Should be source map v3"
    );
    assert!(
        source_map_content.contains("\"sources\""),
        "Should have sources field"
    );
    assert!(
        source_map_content.contains("\"sourcesContent\""),
        "Should have sourcesContent field"
    );

    // Check bundle has sourceMappingURL
    let bundle_path = config.outdir.join("bundle.js");
    let bundle_content = std::fs::read_to_string(&bundle_path).unwrap();
    assert!(
        bundle_content.contains("sourceMappingURL=bundle.js.map"),
        "Should have sourceMappingURL comment"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}

#[tokio::test]
async fn test_demo_project_build() {
    let fixtures_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/demo-project");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));

    let mut build_service =
        soku::core::services::SokuBuildService::new(fs_service, js_processor, css_processor);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist-demo"),
        enable_tree_shaking: false,
        enable_minification: false,
        enable_source_maps: false,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),
    };

    let result = build_service.build(&config).await;
    assert!(result.is_ok(), "Demo project build should succeed");

    let build_result = result.unwrap();
    assert!(
        !build_result.output_files.is_empty(),
        "Should have output files"
    );

    // Check bundle.js and bundle.css exist
    let js_bundle_path = config.outdir.join("bundle.js");
    let css_bundle_path = config.outdir.join("bundle.css");
    assert!(js_bundle_path.exists(), "bundle.js should exist");
    assert!(css_bundle_path.exists(), "bundle.css should exist");

    // Verify bundle contains expected code
    let bundle_content = std::fs::read_to_string(&js_bundle_path).unwrap();
    assert!(
        bundle_content.contains("main.js") || !bundle_content.is_empty(),
        "Bundle should contain JavaScript code"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}
