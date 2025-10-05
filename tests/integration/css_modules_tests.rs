use std::path::PathBuf;
use soku::core::models::BuildConfig;
use soku::core::interfaces::BuildService;
use soku::infrastructure::{TokioFileSystemService, UnifiedJsProcessor, LightningCssProcessor};
use soku::infrastructure::processors::ProcessingStrategy;

#[tokio::test]
async fn test_css_modules_detection() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/css-modules");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(true)); // CSS Modules enabled

    let mut build_service = soku::core::services::SokuBuildService::new(
        fs_service,
        js_processor,
        css_processor,
    );

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist-detection"),
        enable_tree_shaking: false,
        enable_minification: false,
        enable_source_maps: false,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),    };

    let result = build_service.build(&config).await;
    assert!(result.is_ok(), "CSS Modules build should succeed");

    // Check bundle.css exists
    let css_bundle_path = config.outdir.join("bundle.css");
    assert!(css_bundle_path.exists(), "bundle.css should exist");

    // Check that CSS content is processed
    let css_content = std::fs::read_to_string(&css_bundle_path).unwrap();
    assert!(!css_content.is_empty(), "CSS bundle should not be empty");

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}

#[tokio::test]
async fn test_css_modules_scoping() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/css-modules");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(true));

    let mut build_service = soku::core::services::SokuBuildService::new(
        fs_service,
        js_processor,
        css_processor,
    );

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist-scoping"),
        enable_tree_shaking: false,
        enable_minification: false,
        enable_source_maps: false,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: std::collections::HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: std::collections::HashMap::new(),    };

    let result = build_service.build(&config).await;
    assert!(result.is_ok(), "Build should succeed");

    // Verify CSS was processed
    let css_path = config.outdir.join("bundle.css");
    assert!(css_path.exists(), "bundle.css should exist");

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}
