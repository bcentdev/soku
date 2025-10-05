# Ultra Bundler - Quick Start Guide

Get started with Ultra Bundler's advanced features in minutes.

## Installation

```bash
cargo add ultra-bundler
```

## Basic Usage

```rust
use ultra::core::UltraBuildService;
use ultra::core::models::BuildConfig;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Create services
    let fs = Arc::new(ultra::infrastructure::BasicFileSystemService::new());
    let js = Arc::new(ultra::infrastructure::EnhancedJsProcessor::new());
    let css = Arc::new(ultra::infrastructure::LightningCssProcessor::new());

    // Create build service
    let mut service = UltraBuildService::new(fs, js, css);

    // Configure build
    let config = BuildConfig {
        root: PathBuf::from("./src"),
        outdir: PathBuf::from("./dist"),
        entry: PathBuf::from("./src/main.js"),
        minify: true,
        enable_source_maps: true,
        enable_tree_shaking: true,
        enable_hmr: false,
        entries: std::collections::HashMap::new(),
    };

    // Build!
    let result = service.build(&config).await?;
    println!("✨ Build complete!");

    Ok(())
}
```

## Adding Plugins

```rust
use ultra::utils::{Plugin, PluginContext};
use async_trait::async_trait;

// Create a plugin
struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str { "my-plugin" }

    async fn before_build(&self, ctx: &PluginContext) -> Result<()> {
        println!("Building {} modules", ctx.modules.len());
        Ok(())
    }
}

// Register it
let service = service.with_plugin(Arc::new(MyPlugin));
```

## Adding Transformers

```rust
use ultra::utils::BuiltInTransformers;

let service = service
    // Remove console.log in production
    .with_transformer(BuiltInTransformers::remove_console_logs())
    // Add 'use strict'
    .with_transformer(BuiltInTransformers::add_use_strict());
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
};
```

## Hot Module Replacement

```rust
use ultra::infrastructure::{UltraHmrService, BuiltInHmrHooks};

let hmr = UltraHmrService::new(PathBuf::from("./src"))
    .with_hook(Arc::new(BuiltInHmrHooks::logger()))
    .await;

// Start HMR server
hmr.start_server(3001).await?;
```

## Next Steps

- [Plugin API Documentation](./PLUGIN_API.md)
- [Examples Directory](../examples/)
- [Advanced Features Guide](./ADVANCED_FEATURES.md)

## Features

- ⚡ **Ultra-fast builds** - Sub-250ms typical
- 🌳 **Tree shaking** - 50-80% code reduction
- 📦 **Multiple entry points** - Multi-page apps
- 🗺️ **Advanced source maps** - With inline sources
- 🔌 **Plugin system** - Extensible architecture
- 🔧 **Custom transformers** - Code transformations
- 🔥 **HMR with hooks** - Customizable hot reload
- 🎯 **TypeScript/TSX** - Full support

## Need Help?

- Examples: `examples/` directory
- Documentation: `docs/` directory
- Issues: GitHub Issues
