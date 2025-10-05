# Ultra Bundler - Examples

This directory contains practical examples demonstrating how to use Ultra Bundler's advanced features.

## Examples

### 01. Basic Plugin (`01_basic_plugin.rs`)

Shows how to create and register a custom plugin that:
- Tracks build timing
- Logs build events
- Transforms module code

**Run:**
```bash
cargo run --example 01_basic_plugin
```

**Key Concepts:**
- Implementing the `Plugin` trait
- Using `before_build` and `after_build` hooks
- Transforming code with `transform_code`
- Registering plugins with `with_plugin()`

### 02. Custom Transformers (`02_custom_transformers.rs`)

Demonstrates various code transformations:
- Remove console.log statements
- Remove debugger statements
- Add 'use strict' directive
- Replace API endpoints
- Add build metadata
- Conditional transformations

**Run:**
```bash
cargo run --example 02_custom_transformers
```

**Key Concepts:**
- Built-in transformers (`BuiltInTransformers`)
- Regex-based transformations
- Function-based transformations
- Conditional transformations (file pattern matching)
- Chaining multiple transformers

### 03. HMR Hooks (`03_hmr_hooks.rs`)

Shows how to customize Hot Module Replacement behavior:
- Detailed logging of HMR events
- Full reload for specific file patterns
- Throttling updates
- Desktop notifications
- Custom client lifecycle tracking

**Run:**
```bash
cargo run --example 03_hmr_hooks
```

**Key Concepts:**
- Implementing the `HmrHook` trait
- Built-in HMR hooks
- Client connect/disconnect events
- Update lifecycle hooks
- Content transformation during HMR

### 04. Advanced Integration (`04_advanced_integration.rs`)

Combines all features in a production-ready setup:
- Multiple entry points
- Advanced source maps
- Production plugin
- Multiple transformers
- Tree shaking
- Minification

**Run:**
```bash
cargo run --example 04_advanced_integration
```

**Key Concepts:**
- Combining plugins and transformers
- Multiple entry points configuration
- Production optimizations
- Advanced source maps with inline sources
- Complete build pipeline customization

## Prerequisites

Make sure you have a test project in `./demo-project` with:
- `main.js` - Main entry point
- `src/` - Source files
- `dist/` - Output directory (created automatically)

## Example Project Structure

```
demo-project/
├── main.js              # Main entry point
├── src/
│   ├── app.js          # Application code
│   ├── admin.js        # Admin entry point
│   ├── worker.js       # Worker entry point
│   └── utils.js        # Utilities
└── dist/               # Build output
```

## Dependencies

These examples require the following dependencies (already in Cargo.toml):
- `tokio` - Async runtime
- `async-trait` - Async trait support
- `chrono` - Date/time operations (for metadata examples)

## Learning Path

1. **Start with 01**: Learn basic plugin concepts
2. **Move to 02**: Understand code transformations
3. **Try 03**: Explore HMR customization (if using dev server)
4. **Master 04**: See how everything works together

## Tips

- Check the terminal output for detailed logging
- Inspect `dist/` to see generated files
- Compare builds with/without optimizations
- Modify examples to match your use case
- Read inline comments for implementation details

## API Documentation

For complete API documentation, see:
- [PLUGIN_API.md](../docs/PLUGIN_API.md) - Plugin system documentation
- [TRANSFORMERS.md](../docs/TRANSFORMERS.md) - Transformers guide
- [HMR_HOOKS.md](../docs/HMR_HOOKS.md) - HMR hooks reference
- [ADVANCED_FEATURES.md](../docs/ADVANCED_FEATURES.md) - Advanced features guide

## Need Help?

- **Issues**: https://github.com/anthropics/ultra-bundler/issues
- **Discussions**: https://github.com/anthropics/ultra-bundler/discussions
- **Documentation**: See `docs/` directory
