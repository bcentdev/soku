# Soku - Development Guide

## 🛠️ Setting Up Development Environment

### Prerequisites
```bash
# Install Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install additional tools
cargo install cargo-watch    # Auto-rebuild on changes
cargo install hyperfine      # Benchmarking
cargo install flamegraph     # Performance profiling
```

### Building from Source
```bash
git clone https://github.com/bcentdev/soku
cd ultra

# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

## 🏗️ Architecture Deep Dive

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Soku Core                      │
├─────────────────┬─────────────────┬─────────────────────────┤
│   File Watcher  │  Module Graph   │      Dev Server         │
│   (notify)      │   (rayon)       │    (axum + ws)          │
├─────────────────┼─────────────────┼─────────────────────────┤
│   Transformer   │     Cache       │      Resolver           │
│   (oxc/simple)  │   (blake3)      │    (Node.js algo)       │
├─────────────────┼─────────────────┼─────────────────────────┤
│    Bundler      │    Plugins      │      Config             │
│  (production)   │   (extensible)  │    (json/code)          │
└─────────────────┴─────────────────┴─────────────────────────┘
```

### Module Structure

```rust
// src/main.rs - CLI entry point
mod cache;           // Content-addressed caching
mod config;          // Configuration management
mod resolver;        // Node.js module resolution
mod graph;           // Dependency graph + invalidation
mod server;          // Dev server + HMR
mod bundler;         // Production builds
mod plugins;         // Plugin system
mod transform;       // oxc integration (full)
mod transform_simple; // Simplified transformer (demo)
```

## 🔍 Key Implementation Details

### File Watcher with Coalescing
```rust
// High-frequency events are batched to prevent thrashing
let mut interval = tokio::time::interval(Duration::from_millis(16));

loop {
    interval.tick().await;
    let events = self.collect_events_since_last_tick();

    if !events.is_empty() {
        self.process_batch(events).await?;
    }
}
```

### Incremental Module Graph
```rust
pub fn invalidate_module(&self, path: &Path) -> Result<Vec<String>> {
    // 1. Find all modules that depend on this one
    let affected = self.get_affected_modules(&changed_id)?;

    // 2. Topologically sort for optimal rebuild order
    let rebuild_order = self.topological_sort(&affected)?;

    // 3. Rebuild in parallel where possible
    rebuild_order.par_iter().try_for_each(|id| {
        self.process_module(id)
    })?;

    Ok(affected)
}
```

### Smart Caching Strategy
```rust
// Multi-level cache for maximum hit rate
pub struct Cache {
    memory_cache: HashMap<String, CacheEntry>,  // L1: In-memory
    disk_cache: PathBuf,                        // L2: Local disk
    global_cache: PathBuf,                      // L3: Global deps
}

// Cache key includes all factors that affect output
fn compute_cache_key(&self, path: &Path, conditions: &[String]) -> String {
    format!("{}:{}:{}:{}",
        path.display(),
        file_mtime(path),
        conditions.join(","),
        VERSION
    )
}
```

## 🧪 Testing Strategy

### Unit Tests
```bash
# Run all tests
cargo test

# Run specific module tests
cargo test cache
cargo test resolver
cargo test graph

# Test with output
cargo test -- --nocapture
```

### Integration Tests
```bash
# Test full dev server flow
cargo test test_dev_server_hmr

# Test build pipeline
cargo test test_production_build

# Test file watching
cargo test test_file_watcher_coalescing
```

### Performance Tests
```bash
# Benchmark against reference implementations
cargo run --release --example benchmark

# Profile memory usage
cargo run --release --features profile-memory

# CPU profiling with flamegraph
cargo flamegraph --bin ultra -- build examples/large-app
```

## 🔧 Development Workflow

### Live Development
```bash
# Auto-rebuild on changes
cargo watch -x 'run -- dev examples/basic'

# Run with detailed logging
RUST_LOG=debug cargo run -- dev examples/basic

# Performance tracing
ULTRA_TRACE=1 cargo run --release -- dev examples/basic
```

### Adding New Features

1. **Create module**: Add new `.rs` file in `src/`
2. **Update main.rs**: Add module declaration
3. **Write tests**: Add tests in `tests/` or inline
4. **Document**: Add docs and examples
5. **Benchmark**: Ensure no performance regression

### Example: Adding CSS Modules Support
```rust
// 1. Add to transform_simple.rs
pub fn transform_css_modules(&self, source: &str) -> Result<String> {
    // Implementation here
}

// 2. Add test
#[cfg(test)]
mod tests {
    #[test]
    fn test_css_modules() {
        // Test implementation
    }
}

// 3. Integrate into pipeline
match module_type {
    ModuleType::Css if is_css_modules => {
        self.transform_css_modules(&content)?
    }
    // ...
}
```

## 📊 Profiling & Optimization

### CPU Profiling
```bash
# Generate flamegraph
cargo flamegraph --bin ultra -- build examples/large-app

# Use perf (Linux)
perf record --call-graph=dwarf cargo run --release -- build
perf report
```

### Memory Profiling
```bash
# Valgrind (Linux)
valgrind --tool=massif cargo run --release -- build

# macOS instruments
cargo build --release
instruments -t Allocations target/release/ultra build
```

### Benchmarking
```bash
# Compare against other bundlers
hyperfine --warmup 3 \
  'ultra build examples/basic' \
  'vite build examples/basic' \
  'bun build examples/basic/index.js'

# Measure HMR latency
cargo run --release --example hmr-benchmark
```

## 🚀 Release Process

### Version Bump
```bash
# Update version in Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml

# Update CHANGELOG.md
echo "## [0.2.0] - $(date +%Y-%m-%d)" >> CHANGELOG.md

# Commit changes
git add .
git commit -m "Release v0.2.0"
git tag v0.2.0
```

### Publishing
```bash
# Dry run
cargo publish --dry-run

# Publish to crates.io
cargo publish

# Create GitHub release
gh release create v0.2.0 --notes "Release notes here"
```

## 🐛 Debugging Tips

### Common Issues

**Slow HMR**: Check file watcher configuration
```rust
// Increase coalescing window if too many events
let coalescing_ms = std::env::var("ULTRA_COALESCING")
    .unwrap_or_else(|_| "16".to_string())
    .parse()
    .unwrap_or(16);
```

**Memory Leaks**: Check cache cleanup
```rust
// Add periodic cleanup
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        cache.cleanup_old_entries().await;
    }
});
```

**Build Failures**: Enable verbose logging
```bash
RUST_LOG=ultra_bundler=debug cargo run -- build
```

### Debug Utilities
```rust
// Add to any module for debugging
use tracing::{debug, info, warn, error};

debug!("Processing module: {}", module_id);
info!("Cache hit for: {}", path.display());
warn!("Slow transformation: {}ms", duration);
error!("Failed to resolve: {}", specifier);
```

## 📚 Learning Resources

### Rust Resources
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)

### Bundler Concepts
- [Webpack Concepts](https://webpack.js.org/concepts/)
- [Vite Architecture](https://vitejs.dev/guide/features.html)
- [Module Resolution](https://nodejs.org/api/modules.html#modules_all_together)

### Performance Optimization
- [oxc Performance](https://oxc-project.github.io/docs/learn/performance.html)
- [Systems Performance](http://www.brendangregg.com/systems-performance-2nd-edition-book.html)

## 🤝 Contributing Guidelines

### Code Style
```bash
# Format code
cargo fmt

# Check for common issues
cargo clippy

# Run all checks
cargo fmt && cargo clippy && cargo test
```

### Pull Request Process
1. Fork the repository
2. Create feature branch: `git checkout -b feature/amazing-feature`
3. Write tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Add documentation
6. Submit pull request

### Performance Expectations
- HMR updates must be <100ms p95
- Memory usage should be <150MB for typical projects
- Build times should scale linearly with project size
- All operations should be cancellable

---

**Happy hacking! 🦀⚡**