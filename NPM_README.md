# Ultra - The Next-Gen JavaScript Bundler

⚡ **Ultra-fast** • 🌳 **Advanced tree shaking** • 🎯 **Zero config** • 🦀 **Built in Rust**

Ultra is a blazingly fast JavaScript/TypeScript bundler designed for sub-250ms builds with zero configuration.

## 🚀 Quick Start

### Installation

```bash
# npm
npm install -g ultra-bundler

# yarn
yarn global add ultra-bundler

# pnpm
pnpm add -g ultra-bundler

# Or use npx without installing
npx ultra-bundler build
```

### Your First Bundle

```bash
# Create a project
mkdir my-app && cd my-app
echo 'console.log("Hello Ultra!");' > main.js

# Bundle it (zero config!)
ultra build

# Output:
#   ✓ built in 8ms
#   dist/bundle.js (45 B)
```

## ⚡ Features

- **⚡ Ultra-Fast**: Sub-250ms builds for typical projects
- **🌳 Intelligent Tree Shaking**: 50-80% bundle size reduction
- **📦 Full TypeScript/TSX Support**: Complete type stripping and JSX transformation
- **🎯 Zero Config**: Works out of the box, configure when you need it
- **🔥 HMR Ready**: Hot Module Replacement for instant feedback
- **🚀 Smart Auto-Mode**: Automatically optimizes based on project size
- **🦀 Rust Performance**: Native speed with memory safety

## 📖 Usage

### Build for Production

```bash
ultra build
```

### Development Server with HMR

```bash
ultra dev
```

### Watch Mode

```bash
ultra watch
```

### Options

```bash
# Build with source maps
ultra build --source-maps

# Build for development
ultra build --mode development --no-minify

# Force enhanced TypeScript/JSX processing
ultra build --strategy enhanced

# Build with code splitting
ultra build --code-splitting --analyze
```

## 🎯 Processing Strategies

Ultra offers three processing strategies:

- **🚀 Fast Mode**: Minimal transformations, maximum speed (<50ms)
- **📦 Standard Mode**: TypeScript type stripping, basic optimizations (<150ms)
- **⚡ Enhanced Mode**: Full TypeScript + JSX transformations (<250ms)
- **🎯 Ultra Mode**: Auto-enabled for large projects (>100 files)

## 📊 Performance

| Project Size | Files | Build Time | Mode |
|-------------|-------|------------|------|
| **Tiny** | <10 | **<50ms** | Fast |
| **Small** | 10-50 | **<100ms** | Standard |
| **Medium** | 50-100 | **<150ms** | Standard |
| **Large** | 100-500 | **<200ms** | Enhanced |
| **Very Large** | 500-2000+ | **<300ms** | Ultra |

## 🏗️ Project Structure

Ultra works with minimal configuration:

```
my-project/
├── main.js or main.ts     # Entry point (auto-detected)
├── src/                   # Source files
│   ├── components/        # React/TSX components
│   ├── utils/             # Utility modules
│   └── styles/            # CSS files
└── dist/                  # Output directory (generated)
    ├── bundle.js
    └── bundle.css
```

## 🔧 Configuration (Optional)

Create `ultra.config.json` for custom configuration:

```json
{
  "entry": "src/main.ts",
  "outdir": "build",
  "minify": true,
  "sourceMaps": true,
  "treeShaking": true,
  "strategy": "enhanced"
}
```

## 🌐 Platform Support

Ultra provides native binaries for:

- **macOS** (Intel and Apple Silicon)
- **Linux** (x64 and ARM64)
- **Windows** (x64)

The correct binary for your platform is automatically installed.

## 📚 Documentation

- [Full Documentation](https://github.com/bcentdev/ultra#readme)
- [Contributing Guide](https://github.com/bcentdev/ultra/blob/main/CONTRIBUTING.md)
- [Changelog](https://github.com/bcentdev/ultra/blob/main/CHANGELOG.md)

## 🤝 Contributing

Contributions are welcome! See our [Contributing Guide](https://github.com/bcentdev/ultra/blob/main/CONTRIBUTING.md).

## 📄 License

MIT License - see [LICENSE](https://github.com/bcentdev/ultra/blob/main/LICENSE) file for details.

## 🙏 Acknowledgments

Built with:
- 🦀 Rust
- ⚡ oxc (JavaScript/TypeScript parser)
- 🌩️ Lightning CSS
- 🔥 Tokio async runtime

---

**Made with ❤️ and ⚡ by the Ultra Team**

[⭐ Star us on GitHub](https://github.com/bcentdev/ultra) •
[🐛 Report Bug](https://github.com/bcentdev/ultra/issues) •
[💡 Request Feature](https://github.com/bcentdev/ultra/issues)
