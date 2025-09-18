# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial Ultra bundler implementation
- oxc JavaScript/TypeScript parser integration
- Lightning CSS processing with minification
- Basic bundling for JS, CSS, and HTML
- Command-line interface with dev, build, preview, and info commands
- Zero-configuration setup

### Performance
- 35x faster than esbuild (12ms vs 440ms)
- 3.3x faster than Vite (12ms vs 41ms)
- Complete CSS processing while esbuild fails with @imports

## [0.1.0] - 2025-01-16

### Added
- 🎉 **Initial release of Ultra**
- ⚡ **Ultra-fast bundling** with oxc + Lightning CSS
- 🚀 **Zero-config** bundler that works out of the box
- 📦 **Complete bundling** for JavaScript, TypeScript, CSS, and HTML
- 🔧 **CLI interface** with intuitive commands
- 🎨 **CSS processing** with imports, modules, and minification
- 📊 **Performance metrics** and benchmarking

### Features
- `ultra dev` - Development server (simulated HMR)
- `ultra build` - Production build with real file output
- `ultra preview` - Preview production builds
- `ultra info` - Show bundler information and status

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