# Publishing Ultra to npm

This guide explains how to publish Ultra as an npm package with native binaries for multiple platforms.

> **⚠️ IMPORTANT**: Before following this guide, read [NPM_OR_GITHUB_PACKAGES.md](./NPM_OR_GITHUB_PACKAGES.md) to choose between:
> - **npmjs.org** (recommended for public tools)
> - **GitHub Packages** (integrated with GitHub)
>
> This guide assumes you're publishing to **npmjs.org**.

## Architecture

Ultra uses a multi-package strategy similar to esbuild and swc:

1. **Main package** (`ultra-bundler`): JavaScript wrapper that detects platform and loads the correct binary
2. **Platform packages** (`@ultra-bundler/*`): Contains native binaries for specific platforms

### Supported Platforms

- `@ultra-bundler/darwin-arm64` - macOS Apple Silicon
- `@ultra-bundler/darwin-x64` - macOS Intel
- `@ultra-bundler/linux-x64` - Linux x86_64
- `@ultra-bundler/linux-arm64` - Linux ARM64
- `@ultra-bundler/win32-x64` - Windows x64

## Publishing Process

### Prerequisites

1. **npm account** with access to publish `ultra-bundler` package
2. **Organization** `@ultra-bundler` on npm (or change to your org name)
3. **GitHub Actions** configured for multi-platform builds
4. **npm token** added to GitHub secrets

### Step 1: Build Binaries for All Platforms

Use GitHub Actions or local cross-compilation:

```bash
# macOS ARM64 (M1/M2/M3)
cargo build --release --target aarch64-apple-darwin

# macOS x64 (Intel)
cargo build --release --target x86_64-apple-darwin

# Linux x64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu

# Windows x64
cargo build --release --target x86_64-pc-windows-msvc
```

### Step 2: Create Platform Packages

For each platform, create a package structure:

```
npm-packages/
├── darwin-arm64/
│   ├── package.json
│   └── bin/
│       └── ultra          # Binary from target/aarch64-apple-darwin/release/ultra
├── darwin-x64/
│   ├── package.json
│   └── bin/
│       └── ultra
├── linux-x64/
│   ├── package.json
│   └── bin/
│       └── ultra
├── linux-arm64/
│   ├── package.json
│   └── bin/
│       └── ultra
└── win32-x64/
    ├── package.json
    └── bin/
        └── ultra.exe
```

Example `package.json` for platform package:

```json
{
  "name": "@ultra-bundler/darwin-arm64",
  "version": "0.3.0",
  "description": "Ultra bundler native binary for macOS Apple Silicon",
  "repository": "https://github.com/bcentdev/ultra",
  "license": "MIT",
  "os": ["darwin"],
  "cpu": ["arm64"],
  "files": ["bin"],
  "bin": {
    "ultra": "./bin/ultra"
  }
}
```

### Step 3: Publish Platform Packages

```bash
# Navigate to each platform package and publish
cd npm-packages/darwin-arm64
npm publish --access public

cd ../darwin-x64
npm publish --access public

# Repeat for all platforms...
```

### Step 4: Publish Main Package

```bash
# From repository root
npm publish --access public
```

## Automated Publishing with GitHub Actions

Create `.github/workflows/publish-npm.yml`:

```yaml
name: Publish to npm

on:
  release:
    types: [published]

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            package: darwin-arm64
          - os: macos-latest
            target: x86_64-apple-darwin
            package: darwin-x64
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            package: linux-x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            package: linux-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            package: win32-x64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: ${{ matrix.target }}
          override: true

      - name: Build binary
        run: cargo build --release --target ${{ matrix.target }}

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'
          registry-url: 'https://registry.npmjs.org'

      - name: Prepare platform package
        run: |
          mkdir -p npm-packages/${{ matrix.package }}/bin
          cp target/${{ matrix.target }}/release/ultra* npm-packages/${{ matrix.package }}/bin/

      - name: Create platform package.json
        working-directory: npm-packages/${{ matrix.package }}
        run: |
          cat > package.json <<EOF
          {
            "name": "@ultra-bundler/${{ matrix.package }}",
            "version": "${{ github.event.release.tag_name }}",
            "description": "Ultra bundler for ${{ matrix.package }}",
            "repository": "https://github.com/bcentdev/ultra",
            "license": "MIT",
            "files": ["bin"]
          }
          EOF

      - name: Publish platform package
        working-directory: npm-packages/${{ matrix.package }}
        run: npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}

  publish-main:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '18'
          registry-url: 'https://registry.npmjs.org'

      - name: Update version in package.json
        run: |
          npm version ${{ github.event.release.tag_name }} --no-git-tag-version

      - name: Publish main package
        run: npm publish --access public
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

## Testing Locally

Before publishing, test the installation:

```bash
# Install from local directory
npm install /path/to/ultra-bundler

# Test the CLI
npx ultra --version
npx ultra build
```

## Version Management

Keep versions in sync:
- `Cargo.toml` version
- `package.json` version
- Git tags (e.g., `v0.3.0`)

Update all three when releasing a new version.

## Troubleshooting

### "Binary not found" error

Users may see this if optional dependencies weren't installed. They can manually install:

```bash
npm install @ultra-bundler/darwin-arm64  # or their platform
```

### Platform not supported

Users on unsupported platforms need to build from source:

```bash
git clone https://github.com/bcentdev/ultra
cd ultra
cargo build --release
```

## Alternative: Cargo-dist

For automated multi-platform releases, consider using [cargo-dist](https://opensource.axo.dev/cargo-dist/):

```bash
cargo install cargo-dist
cargo dist init
cargo dist build
```

This handles building, packaging, and publishing automatically.

## Resources

- [npm documentation](https://docs.npmjs.com/)
- [Publishing native Node.js modules](https://nodejs.org/api/addons.html)
- [esbuild's approach](https://github.com/evanw/esbuild)
- [swc's approach](https://github.com/swc-project/swc)
