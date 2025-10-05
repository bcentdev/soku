# Plugin API Documentation

Soku provides a powerful plugin system that allows you to customize the build process through lifecycle hooks.

## Table of Contents

- [Overview](#overview)
- [Plugin Trait](#plugin-trait)
- [Lifecycle Hooks](#lifecycle-hooks)
- [Plugin Context](#plugin-context)
- [Creating a Plugin](#creating-a-plugin)
- [Registering Plugins](#registering-plugins)
- [Example Plugins](#example-plugins)
- [Best Practices](#best-practices)

## Overview

The Plugin API allows you to:
- **Hook into build lifecycle** - Execute code before/after build steps
- **Transform module content** - Modify code during processing
- **Resolve imports** - Customize import resolution logic
- **Track build metrics** - Collect performance and statistics data

## Plugin Trait

All plugins must implement the `Plugin` trait:

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin name (required)
    fn name(&self) -> &str;

    /// Plugin version (optional)
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Initialize plugin (optional)
    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Called before build starts (optional)
    async fn before_build(&self, context: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after build completes (optional)
    async fn after_build(&self, context: &PluginContext, result: &BuildResult) -> Result<()> {
        Ok(())
    }

    /// Transform module code (optional)
    async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
        Ok(code)
    }

    /// Resolve import path (optional)
    async fn resolve_import(&self, import_path: &str, from_file: &str) -> Result<Option<String>> {
        Ok(None)
    }
}
```

## Lifecycle Hooks

### `before_build`

Called before the build process starts.

**When to use:**
- Initialize resources
- Validate configuration
- Start timing measurements
- Log build start

**Example:**
```rust
async fn before_build(&self, context: &PluginContext) -> Result<()> {
    println!("Building {} modules", context.modules.len());
    Ok(())
}
```

### `after_build`

Called after the build process completes.

**When to use:**
- Cleanup resources
- Generate reports
- Upload artifacts
- Log build completion

**Example:**
```rust
async fn after_build(&self, context: &PluginContext, result: &BuildResult) -> Result<()> {
    println!("Build completed in {:?}", result.build_time);
    println!("Output files: {}", result.output_files.len());
    Ok(())
}
```

### `transform_code`

Transform module code during processing.

**When to use:**
- Code preprocessing
- Syntax transformations
- Injecting code
- Code optimization

**Example:**
```rust
async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
    // Add header comment
    let header = format!("// Module: {}\n", module.path.display());
    Ok(format!("{}{}", header, code))
}
```

**Note:** Transformers are applied sequentially in registration order.

### `resolve_import`

Customize import resolution logic.

**When to use:**
- Path aliasing
- Virtual modules
- Custom module resolution
- Package redirects

**Example:**
```rust
async fn resolve_import(&self, import_path: &str, from_file: &str) -> Result<Option<String>> {
    if import_path.starts_with("@/") {
        // Resolve @ alias to src/
        let resolved = import_path.replace("@/", "./src/");
        return Ok(Some(resolved));
    }
    Ok(None)
}
```

**Note:** First plugin that returns `Some(path)` wins.

## Plugin Context

The `PluginContext` provides build information:

```rust
pub struct PluginContext {
    pub config: BuildConfig,      // Build configuration
    pub modules: Vec<ModuleInfo>,  // All modules
    pub current_event: PluginEvent, // Current event
}
```

### Accessing Context

```rust
async fn before_build(&self, context: &PluginContext) -> Result<()> {
    // Access configuration
    let root = &context.config.root;
    let minify = context.config.minify;

    // Access modules
    let module_count = context.modules.len();
    let ts_modules = context.modules.iter()
        .filter(|m| m.path.extension().unwrap_or_default() == "ts")
        .count();

    // Check event type
    if context.current_event == PluginEvent::BeforeBuild {
        println!("Build starting...");
    }

    Ok(())
}
```

## Creating a Plugin

### Basic Plugin

```rust
use soku::utils::{Plugin, PluginContext, Result};
use soku::core::models::{ModuleInfo, BuildResult};
use async_trait::async_trait;

struct MyPlugin {
    name: String,
    enabled: bool,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            name: "my-plugin".to_string(),
            enabled: true,
        }
    }
}

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_build(&self, context: &PluginContext) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        println!("[{}] Build starting...", self.name);
        Ok(())
    }

    async fn after_build(&self, _context: &PluginContext, result: &BuildResult) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        println!("[{}] Build completed!", self.name);
        println!("  JS modules: {}", result.js_modules_processed);
        println!("  CSS files: {}", result.css_files_processed);
        Ok(())
    }
}
```

### Stateful Plugin

```rust
use std::sync::Mutex;

struct TimingPlugin {
    name: String,
    start_time: Mutex<Option<std::time::Instant>>,
}

impl TimingPlugin {
    pub fn new() -> Self {
        Self {
            name: "timing".to_string(),
            start_time: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Plugin for TimingPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_build(&self, _context: &PluginContext) -> Result<()> {
        *self.start_time.lock().unwrap() = Some(std::time::Instant::now());
        Ok(())
    }

    async fn after_build(&self, _context: &PluginContext, _result: &BuildResult) -> Result<()> {
        if let Some(start) = *self.start_time.lock().unwrap() {
            let duration = start.elapsed();
            println!("Build took: {:?}", duration);
        }
        Ok(())
    }
}
```

## Registering Plugins

### Single Plugin

```rust
use std::sync::Arc;

let service = UltraBuildService::new(fs, js_proc, css_proc)
    .with_plugin(Arc::new(MyPlugin::new()));
```

### Multiple Plugins

```rust
let service = UltraBuildService::new(fs, js_proc, css_proc)
    .with_plugin(Arc::new(TimingPlugin::new()))
    .with_plugin(Arc::new(LoggerPlugin::new()))
    .with_plugin(Arc::new(AnalyticsPlugin::new()));
```

### Dynamic Registration

```rust
let mut service = UltraBuildService::new(fs, js_proc, css_proc);

// Register plugins conditionally
if config.enable_analytics {
    service = service.with_plugin(Arc::new(AnalyticsPlugin::new()));
}

if config.enable_reporting {
    service = service.with_plugin(Arc::new(ReportPlugin::new()));
}
```

## Example Plugins

### Logger Plugin

```rust
struct LoggerPlugin {
    name: String,
    verbose: bool,
}

#[async_trait]
impl Plugin for LoggerPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_build(&self, context: &PluginContext) -> Result<()> {
        println!("🔌 [{}] Build starting", self.name);
        if self.verbose {
            println!("  Root: {}", context.config.root.display());
            println!("  Modules: {}", context.modules.len());
        }
        Ok(())
    }

    async fn after_build(&self, _ctx: &PluginContext, result: &BuildResult) -> Result<()> {
        println!("🔌 [{}] Build complete", self.name);
        println!("  Output: {} files", result.output_files.len());
        Ok(())
    }
}
```

### Analytics Plugin

```rust
struct AnalyticsPlugin {
    name: String,
    endpoint: String,
}

#[async_trait]
impl Plugin for AnalyticsPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn after_build(&self, _ctx: &PluginContext, result: &BuildResult) -> Result<()> {
        // Send build metrics to analytics endpoint
        let metrics = serde_json::json!({
            "js_modules": result.js_modules_processed,
            "css_files": result.css_files_processed,
            "build_time": result.build_time.as_secs_f64(),
            "success": result.success,
        });

        // Send to analytics (example)
        // reqwest::Client::new()
        //     .post(&self.endpoint)
        //     .json(&metrics)
        //     .send()
        //     .await?;

        Ok(())
    }
}
```

### Bundle Analyzer Plugin

```rust
struct BundleAnalyzerPlugin {
    name: String,
    output_path: PathBuf,
}

#[async_trait]
impl Plugin for BundleAnalyzerPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn after_build(&self, ctx: &PluginContext, result: &BuildResult) -> Result<()> {
        let mut analysis = String::from("# Bundle Analysis\n\n");

        // Analyze output files
        for output in &result.output_files {
            analysis.push_str(&format!(
                "## {}\n- Size: {} bytes\n- Type: {}\n\n",
                output.path.display(),
                output.size,
                output.path.extension().unwrap_or_default().to_string_lossy()
            ));
        }

        // Write analysis
        tokio::fs::write(&self.output_path, analysis).await?;
        println!("📊 Bundle analysis written to {}", self.output_path.display());

        Ok(())
    }
}
```

## Best Practices

### 1. Make Plugins Configurable

```rust
struct MyPlugin {
    name: String,
    enabled: bool,
    config: PluginConfig,
}

impl MyPlugin {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            name: "my-plugin".to_string(),
            enabled: true,
            config,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
```

### 2. Handle Errors Gracefully

```rust
async fn transform_code(&self, module: &ModuleInfo, code: String) -> Result<String> {
    match self.process_code(&code) {
        Ok(transformed) => Ok(transformed),
        Err(e) => {
            eprintln!("Warning: Transform failed for {}: {}",
                module.path.display(), e);
            // Return original code if transformation fails
            Ok(code)
        }
    }
}
```

### 3. Use Descriptive Names

```rust
fn name(&self) -> &str {
    "bundle-size-analyzer"  // Good: descriptive
    // Not: "plugin1"        // Bad: unclear purpose
}
```

### 4. Log Important Events

```rust
async fn before_build(&self, context: &PluginContext) -> Result<()> {
    println!("[{}] Processing {} modules", self.name, context.modules.len());
    Ok(())
}
```

### 5. Keep Plugins Focused

Each plugin should do one thing well. Split complex functionality into multiple plugins.

```rust
// Good: Separate plugins
let service = service
    .with_plugin(Arc::new(MinificationPlugin::new()))
    .with_plugin(Arc::new(CompressionPlugin::new()))
    .with_plugin(Arc::new(AnalyticsPlugin::new()));

// Not ideal: One plugin doing everything
let service = service
    .with_plugin(Arc::new(MegaPlugin::new())); // Too many responsibilities
```

### 6. Document Your Plugins

```rust
/// Analyzes bundle size and generates reports
///
/// # Features
/// - Calculates total bundle size
/// - Identifies large modules
/// - Generates visual size charts
///
/// # Example
/// ```
/// let plugin = BundleAnalyzerPlugin::new("./analysis.md");
/// service.with_plugin(Arc::new(plugin));
/// ```
struct BundleAnalyzerPlugin {
    // ...
}
```

## See Also

- [Custom Transformers](./TRANSFORMERS.md) - Code transformation guide
- [HMR Hooks](./HMR_HOOKS.md) - Hot reload customization
- [Examples](../examples/) - Working code examples
