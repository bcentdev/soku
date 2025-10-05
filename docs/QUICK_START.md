# Soku - Quick Start Guide

Get started with Soku's advanced features in minutes.

## Installation

```bash
cargo add soku
```

## Basic Usage

```rust
use soku::core::services::UltraBuildService;
use soku::core::models::BuildConfig;
use soku::infrastructure::{TokioFileSystemService, LightningCssProcessor};
use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create services with current APIs
    let fs = Arc::new(TokioFileSystemService);
    let js = Arc::new(UnifiedJsProcessor::new(ProcessingStrategy::Enhanced));
    let css = Arc::new(LightningCssProcessor::new(false)); // false = no minify for dev

    // Create build service
    let mut service = UltraBuildService::new(fs, js, css);

    // Configure build with current structure
    let config = BuildConfig {
        root: PathBuf::from("./src"),
        outdir: PathBuf::from("./dist"),
        enable_tree_shaking: true,
        enable_minification: false,  // Development mode
        enable_source_maps: true,
        enable_code_splitting: false,
        max_chunk_size: None,
        mode: "development".to_string(),
        alias: HashMap::new(),
        external: Vec::new(),
        vendor_chunk: false,
        entries: HashMap::new(),  // Auto-detect entry from root
    };

    // Build!
    let result = service.build(&config).await?;
    println!("✨ Build complete in {}ms!", result.build_time);

    Ok(())
}
```

## Processing Strategies

Ultra offers three processing strategies:

```rust
use soku::infrastructure::processors::{UnifiedJsProcessor, ProcessingStrategy};

// Fast mode - minimal transformations, maximum speed
let fast_processor = UnifiedJsProcessor::new(ProcessingStrategy::Fast);

// Standard mode - basic TypeScript stripping
let standard_processor = UnifiedJsProcessor::new(ProcessingStrategy::Standard);

// Enhanced mode - full TypeScript + JSX transformation (recommended)
let enhanced_processor = UnifiedJsProcessor::new(ProcessingStrategy::Enhanced);
```

## Adding Plugins

```rust
use soku::utils::{Plugin, PluginContext};
use async_trait::async_trait;

// Create a plugin
struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str { "my-plugin" }

    async fn before_build(&self, ctx: &PluginContext) -> ultra::utils::Result<()> {
        println!("Building {} modules", ctx.modules.len());
        Ok(())
    }
}

// Register it
let service = service.with_plugin(Arc::new(MyPlugin));
```

## Adding Custom Transformers

```rust
use soku::utils::BuiltInTransformers;

let service = service
    // Remove console.log in production
    .with_transformer(BuiltInTransformers::remove_console_logs())
    // Add 'use strict'
    .with_transformer(BuiltInTransformers::add_use_strict());
```

## Production Build

```rust
// Production configuration
let config = BuildConfig {
    root: PathBuf::from("./src"),
    outdir: PathBuf::from("./dist"),
    enable_tree_shaking: true,      // Remove dead code
    enable_minification: true,       // Minify output
    enable_source_maps: true,        // Generate source maps
    enable_code_splitting: true,     // Split vendor code
    max_chunk_size: Some(500_000),   // 500KB chunks
    mode: "production".to_string(),
    alias: HashMap::new(),
    external: Vec::new(),
    vendor_chunk: true,              // Separate vendor bundle
    entries: HashMap::new(),
};

let css = Arc::new(LightningCssProcessor::new(true)); // true = minify for production
```

## Multiple Entry Points

```rust
use std::collections::HashMap;

let mut entries = HashMap::new();
entries.insert("main".to_string(), PathBuf::from("./src/main.js"));
entries.insert("admin".to_string(), PathBuf::from("./src/admin.js"));

let config = BuildConfig {
    entries,  // Multiple bundles!
    // ... rest of config
    ..Default::default()
};
```

## Hot Module Replacement

```rust
use soku::infrastructure::{UltraHmrService, BuiltInHmrHooks};

let hmr = UltraHmrService::new(PathBuf::from("./src"))
    .with_hook(Arc::new(BuiltInHmrHooks::logger()))
    .await;

// Start HMR server
hmr.start_server(3001).await?;
```

## Tree Shaking

```rust
use soku::infrastructure::processors::TreeShaker;

// Add tree shaker for dead code elimination
let tree_shaker = Arc::new(TreeShaker::new());
let service = service.with_tree_shaker(tree_shaker);

// Tree shaking is also enabled via config
let config = BuildConfig {
    enable_tree_shaking: true,  // 50-80% code reduction
    // ...
    ..Default::default()
};
```

## Next Steps

- [Plugin API Documentation](./PLUGIN_API.md)
- [Development Guide](./DEVELOPMENT.md)
- [Examples Directory](../examples/)

## Features

- ⚡ **Ultra-fast builds** - Sub-250ms typical
- 🌳 **Tree shaking** - 50-80% code reduction
- 📦 **Multiple entry points** - Multi-page apps
- 🗺️ **Advanced source maps** - With inline sources
- 🔌 **Plugin system** - Extensible architecture
- 🔧 **Custom transformers** - Code transformations
- 🔥 **HMR with hooks** - Customizable hot reload
- 🎯 **TypeScript/TSX** - Full support via UnifiedJsProcessor
- 🚀 **Three processing strategies** - Fast, Standard, Enhanced

## Need Help?

- Examples: `examples/` directory
- Documentation: `docs/` directory
- GitHub: https://github.com/bcentdev/ultra
