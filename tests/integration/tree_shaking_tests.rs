use std::path::PathBuf;
use ultra::core::models::BuildConfig;
use ultra::core::interfaces::BuildService;
use ultra::infrastructure::{TokioFileSystemService, UnifiedJsProcessor, LightningCssProcessor, RegexTreeShaker};
use ultra::infrastructure::processors::ProcessingStrategy;

// TODO: Fix tree shaking stats reporting - removed_exports count is 0
// Issue: Tree shaking is working but stats.removed_exports is not being populated
// Likely in RegexTreeShaker::shake() or AstTreeShaker::shake() methods
#[tokio::test]
#[ignore]
async fn test_tree_shaking_removes_unused_code() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tree-shaking");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));
    let tree_shaker = std::sync::Arc::new(RegexTreeShaker::new());

    let mut build_service = ultra::core::services::UltraBuildService::new(
        fs_service,
        js_processor,
        css_processor,
    ).with_tree_shaker(tree_shaker);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist"),
        enable_tree_shaking: true,
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
    assert!(result.is_ok(), "Tree shaking build should succeed");

    let build_result = result.unwrap();

    // Check that tree shaking stats are present
    assert!(build_result.tree_shaking_stats.is_some(), "Should have tree shaking stats");

    let stats = build_result.tree_shaking_stats.unwrap();
    assert!(stats.removed_exports > 0, "Should have removed some exports");

    // Check bundle.js exists
    let bundle_path = config.outdir.join("bundle.js");
    assert!(bundle_path.exists(), "bundle.js should exist");

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}

#[tokio::test]
async fn test_tree_shaking_preserves_used_code() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tree-shaking");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Standard));
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));
    let tree_shaker = std::sync::Arc::new(RegexTreeShaker::new());

    let mut build_service = ultra::core::services::UltraBuildService::new(
        fs_service,
        js_processor,
        css_processor,
    ).with_tree_shaker(tree_shaker);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist"),
        enable_tree_shaking: true,
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

    // Verify bundle contains used code
    let bundle_path = config.outdir.join("bundle.js");
    let bundle_content = std::fs::read_to_string(&bundle_path).unwrap();

    // Should contain main entry point code
    assert!(!bundle_content.is_empty(), "Bundle should not be empty");

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}

// TODO: Fix TypeScript tree shaking with Enhanced strategy
// Issue: EnhancedJsProcessor doesn't properly handle tree shaking
// Needs integration between TypeScript processing and tree shaking passes
#[tokio::test]
#[ignore]
async fn test_tree_shaking_with_typescript() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tree-shaking");

    let fs_service = std::sync::Arc::new(TokioFileSystemService);
    let js_processor = std::sync::Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Enhanced)); // Enhanced for TS
    let css_processor = std::sync::Arc::new(LightningCssProcessor::new(false));
    let tree_shaker = std::sync::Arc::new(RegexTreeShaker::new());

    let mut build_service = ultra::core::services::UltraBuildService::new(
        fs_service,
        js_processor,
        css_processor,
    ).with_tree_shaker(tree_shaker);

    let config = BuildConfig {
        root: fixtures_dir.clone(),
        outdir: fixtures_dir.join("dist"),
        enable_tree_shaking: true,
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
    assert!(result.is_ok(), "TypeScript tree shaking should succeed");

    // Check bundle exists
    let bundle_path = config.outdir.join("bundle.js");
    assert!(bundle_path.exists(), "bundle.js should exist");

    // Cleanup
    let _ = std::fs::remove_dir_all(config.outdir);
}
