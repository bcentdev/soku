#!/bin/bash
# Automated npm publishing script for Soku
# Usage: ./scripts/publish-npm.sh [dry-run]

set -e

VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
DRY_RUN=${1:-""}

echo "🚀 Soku (速) npm Publishing Script"
echo "==================================="
echo "Version: $VERSION"
echo ""

# Check if logged in to npm
echo "📝 Checking npm authentication..."
if ! npm whoami &> /dev/null; then
  echo "❌ Not logged in to npm"
  echo "Please run: npm login"
  exit 1
fi

NPM_USER=$(npm whoami)
echo "✓ Logged in as: $NPM_USER"
echo ""

# Verify binaries exist
echo "🔍 Verifying binaries..."
PLATFORMS=("aarch64-apple-darwin" "x86_64-apple-darwin" "x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "x86_64-pc-windows-msvc")
MISSING_BINARIES=0

for target in "${PLATFORMS[@]}"; do
  if [[ "$target" == *"windows"* ]]; then
    binary_path="target/$target/release/soku.exe"
  else
    binary_path="target/$target/release/soku"
  fi

  if [ ! -f "$binary_path" ]; then
    echo "  ⚠️  Missing: $binary_path"
    MISSING_BINARIES=$((MISSING_BINARIES + 1))
  else
    echo "  ✓ Found: $binary_path"
  fi
done

if [ $MISSING_BINARIES -gt 0 ]; then
  echo ""
  echo "❌ Missing $MISSING_BINARIES binaries"
  echo ""
  echo "Build them with:"
  echo "  cargo build --release --target aarch64-apple-darwin"
  echo "  cargo build --release --target x86_64-apple-darwin"
  echo "  cargo build --release --target x86_64-unknown-linux-gnu"
  echo "  cargo build --release --target aarch64-unknown-linux-gnu"
  echo "  cargo build --release --target x86_64-pc-windows-msvc"
  echo ""
  echo "Or use GitHub Actions to build automatically"
  exit 1
fi

echo ""
echo "✓ All binaries found"
echo ""

# Prepare platform packages
echo "📦 Preparing platform packages..."
./scripts/prepare-npm-packages.sh

echo ""
echo "📤 Publishing packages..."
echo ""

# Publish platform packages
cd npm-packages
for platform_dir in */; do
  platform=${platform_dir%/}
  echo "Publishing soku-$platform..."
  cd "$platform"

  if [ "$DRY_RUN" = "dry-run" ]; then
    npm publish --dry-run --access public
  else
    npm publish --access public
  fi

  cd ..
  echo "  ✓ Published soku-$platform"
  echo ""
done

cd ..

# Publish main package
echo "Publishing main package soku..."
if [ "$DRY_RUN" = "dry-run" ]; then
  npm publish --dry-run --access public
else
  npm publish --access public
fi

echo ""
echo "✅ All packages published successfully!"
echo ""
echo "Install with:"
echo "  npm install -g soku"
echo ""
echo "View on npm:"
echo "  https://www.npmjs.com/package/soku"
