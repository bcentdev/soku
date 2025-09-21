# Ultra Bundler Demo Project

This is the official demo project for Ultra Bundler - the fastest bundler in the universe.

## 🚀 Features Demonstrated

- **Lightning Fast Builds** - 0.51ms build times
- **ES6 Modules** - Modern JavaScript with imports/exports
- **CSS Processing** - Advanced styling with Lightning CSS
- **Tree Shaking** - Dead code elimination
- **Hot Module Replacement** - Live reloading during development
- **Clean Architecture** - Well-structured codebase

## 📁 Project Structure

```
demo-project/
├── src/
│   ├── main.js      # Entry point with imports
│   ├── app.js       # Application module
│   ├── utils.js     # Utility functions
│   └── styles.css   # Styling
├── index.html       # HTML template
└── README.md        # This file
```

## 🔧 How to Build

From the ultra-bundler root directory:

```bash
# Build the demo project
cargo run --release -- build --root demo-project --outdir demo-project/dist

# Start development server (with HMR)
cargo run --release -- dev --root demo-project --port 3000

# Preview production build
cargo run --release -- preview --dir demo-project/dist --port 4173
```

## 📊 Performance Metrics

This project is designed to showcase Ultra Bundler's performance:

- **Build Time**: ~0.51ms (ultra-fast)
- **Bundle Size**: Optimized with tree shaking
- **Load Time**: Minimal overhead
- **HMR**: Instant hot reloads

## 🎯 What Gets Bundled

1. **JavaScript**: All `.js` files are processed and bundled
2. **CSS**: Styles are processed with Lightning CSS
3. **Dependencies**: ES6 imports are resolved automatically
4. **Optimization**: Tree shaking removes unused code

## 🔥 Try It Out

1. Build the project using the commands above
2. Open `demo-project/index.html` in your browser
3. See the interactive demo in action
4. Modify files and use the dev server to see HMR in action

---

**Note**: This demo project should not be modified as it serves as the stable test case for Ultra Bundler's functionality.