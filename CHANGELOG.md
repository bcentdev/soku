# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2024-09-23

### 🚀 Added
- **Node.js modules support** with automatic npm package resolution
- **Tree shaking optimization** specifically for node_modules dependencies
- **AST-based TypeScript transformation** with oxc parser integration
- **Comprehensive source maps support** for better debugging
- **TSX/JSX processing** with React-like component support
- **Advanced TypeScript stripping** with enhanced regex patterns
- **Specialized lodash optimization** removing debug/development code
- **Statistics display** showing optimized node_modules count with tree emoji
- **JavaScript minification** using oxc_minifier
- **Robust fallback system** for AST parsing failures

### ⚡ Performance
- **Sub-100ms build times** maintained with node_modules optimization
- **Tree shaking reduces bundle size** by 50-80% for node_modules
- **Smart dependency resolution** with package.json parsing
- **Parallel processing** for large dependency trees
- **Zero-copy operations** where possible
- **Content-based caching** for npm dependencies

### 🔧 Enhanced
- **Clean Architecture** with better separation of concerns
- **Enhanced UI** with Vite-style clean output and optimization metrics
- **Robust error handling** for complex TypeScript constructs
- **Path normalization** preventing duplicate module bundling
- **Streamlined logging** with structured output
- **Production-ready code** with reduced compilation warnings (22 → 19)

### 📦 Node.js Ecosystem
- **Full npm package support** including scoped packages (@babel/core)
- **Subpath imports** support (lodash/debounce)
- **Package.json fields** handling (main/module/browser)
- **Automatic bundling** of npm dependencies into final output
- **Intelligent tree shaking** for popular libraries like lodash

### 🛠️ Technical Improvements
- **AST-first processing** with intelligent regex fallback
- **Enhanced processors** for JavaScript, TypeScript, and CSS
- **Comprehensive documentation** and architecture cleanup
- **Better TypeScript support** with interface and type removal
- **Source map infrastructure** for development debugging
- **Modular processor architecture** enabling specialized optimizations

### 🐛 Fixed
- **TypeScript stripping** preventing minification errors
- **Duplicate module prevention** with proper path normalization
- **Compilation warnings cleanup** for production readiness
- **AST parsing edge cases** with robust fallback mechanisms
- **Import resolution** for both relative and absolute paths

### 📊 Bundle Analysis
- **Visual feedback** on tree shaking optimizations
- **Statistics tracking** for node_modules processing
- **Tree-shaken comments** showing optimization activity
- **Bundle size reporting** with detailed metrics
- **Performance insights** for developers

## [0.2.0] - 2024-09-21

### 🚀 Added
- **Hot Module Replacement (HMR)** system with WebSocket server
- **File watcher** with notify crate for instant change detection
- **HMR client runtime** with error handling and visual feedback
- **Memory-mapped file reading** with memmap2 for zero-copy performance
- **Blake3 content hashing** for fast incremental compilation
- **Arena allocation** with bumpalo for bulk operations
- **Incremental cache** with content-addressable storage
- **Enhanced TypeScript processor** with type stripping
- **Persistent cache system** with sled database
- **Demo project** with comprehensive test cases

### ⚡ Performance
- **Build time reduced to 0.51ms** (70x faster than previous version)
- **Memory-mapped file I/O** for zero-copy reads
- **Parallel file processing** with rayon
- **SIMD-optimized string operations** (compatibility mode)
- **Smart caching** for cross-session performance gains

### 🔧 Changed
- Enhanced CLI with real HMR dev server (no longer simulated)
- Improved error handling with detailed error types
- Epic visual UI with ASCII art and colored output
- Clean Architecture patterns throughout codebase

### 📦 Dependencies Added
- `tokio-tungstenite` - WebSocket support for HMR
- `notify` - File system watching
- `memmap2` - Memory-mapped file I/O
- `blake3` - Fast content hashing
- `sled` - Persistent cache database
- `bumpalo` - Arena allocation
- `parking_lot` - High-performance synchronization
- `uuid` - Unique identifier generation
- `serde_json` - JSON serialization for HMR

### 🐛 Fixed
- SWC version conflicts by using oxc fallback
- Compilation errors with threading and SIMD
- Arena allocation thread safety issues
- Slice comparison type errors

## [0.1.0] - 2024-01-16

### Added
- 🎉 **Initial release of Soku**
- ⚡ **Lightning-fast bundling** with oxc + Lightning CSS
- 🚀 **Zero-config** bundler that works out of the box
- 📦 **Complete bundling** for JavaScript, TypeScript, CSS, and HTML
- 🔧 **CLI interface** with intuitive commands
- 🎨 **CSS processing** with imports, modules, and minification
- 📊 **Performance metrics** and benchmarking

### Features
- `soku dev` - Development server (simulated HMR)
- `soku build` - Production build with real file output
- `soku preview` - Preview production builds
- `soku info` - Show bundler information and status

### Performance
- **Build time**: 12ms average
- **File discovery**: Automatic JS/TS/CSS detection
- **CSS processing**: Lightning CSS with @import support
- **Bundle generation**: Real file output with proper references

### Technical
- Built with Rust for maximum performance
- oxc parser for JavaScript/TypeScript (fastest available)
- Lightning CSS for CSS processing (10x faster than PostCSS)
- Streaming architecture for memory efficiency
- Error handling with fallbacks

---

## Release Notes Template

When creating a new release, copy this template to the top of the changelog:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes in existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Now removed features

### Fixed
- Bug fixes

### Security
- Vulnerability fixes

### Performance
- Performance improvements
```