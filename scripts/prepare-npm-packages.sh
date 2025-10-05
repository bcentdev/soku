#!/bin/bash
# Script to prepare platform-specific npm packages

set -e

VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
REPO="https://github.com/bcentdev/ultra"

echo "📦 Preparing npm packages for version $VERSION"
echo "   Package: soku"
echo "   Registry: npmjs.org"

# Create npm-packages directory
rm -rf npm-packages
mkdir -p npm-packages

# Platform configurations
declare -A PLATFORMS=(
  ["darwin-arm64"]="aarch64-apple-darwin:soku"
  ["darwin-x64"]="x86_64-apple-darwin:soku"
  ["linux-x64"]="x86_64-unknown-linux-gnu:soku"
  ["linux-arm64"]="aarch64-unknown-linux-gnu:soku"
  ["win32-x64"]="x86_64-pc-windows-msvc:soku.exe"
)

# Function to create platform package
create_platform_package() {
  local platform=$1
  local target_info=${PLATFORMS[$platform]}
  local target=$(echo $target_info | cut -d: -f1)
  local binary=$(echo $target_info | cut -d: -f2)

  echo "  Creating package for $platform..."

  # Create package directory structure
  local pkg_dir="npm-packages/$platform"
  mkdir -p "$pkg_dir/bin"

  # Check if binary exists
  local binary_path="target/$target/release/$binary"
  if [ ! -f "$binary_path" ]; then
    echo "    ⚠️  Binary not found: $binary_path"
    echo "    Build it with: cargo build --release --target $target"
    return
  fi

  # Copy binary
  cp "$binary_path" "$pkg_dir/bin/"

  # Create package.json
  cat > "$pkg_dir/package.json" <<EOF
{
  "name": "soku-$platform",
  "version": "$VERSION",
  "description": "Soku (速) bundler native binary for $platform",
  "repository": {
    "type": "git",
    "url": "git+$REPO.git"
  },
  "license": "MIT",
  "files": [
    "bin"
  ],
  "bin": {
    "soku": "./bin/$binary"
  }
}
EOF

  echo "    ✓ Package created at $pkg_dir"
}

# Create packages for all platforms
echo ""
echo "Creating platform packages..."
for platform in "${!PLATFORMS[@]}"; do
  create_platform_package "$platform"
done

echo ""
echo "✅ Done! Platform packages created in npm-packages/"
echo ""
echo "Next steps:"
echo "  1. Test locally: npm install ./npm-packages/darwin-arm64"
echo "  2. Publish each package: cd npm-packages/darwin-arm64 && npm publish --access public"
echo "  3. Publish main package: npm publish --access public"
