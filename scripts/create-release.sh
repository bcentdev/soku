#!/bin/bash
# Automated release creation script for Ultra
# Usage: ./scripts/create-release.sh [version]

set -e

# Get version from Cargo.toml or argument
if [ -n "$1" ]; then
  VERSION="$1"
else
  VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
fi

TAG="v$VERSION"

echo "🏷️  Ultra Release Creation Script"
echo "=================================="
echo "Version: $VERSION"
echo "Tag: $TAG"
echo ""

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "❌ Tag $TAG already exists"
  echo ""
  echo "To create a new release:"
  echo "  1. Update version in Cargo.toml"
  echo "  2. Update version in package.json"
  echo "  3. Update CHANGELOG.md"
  echo "  4. Run this script again"
  exit 1
fi

# Check if working directory is clean
if [ -n "$(git status --porcelain)" ]; then
  echo "❌ Working directory is not clean"
  echo ""
  echo "Commit or stash your changes first:"
  echo "  git status"
  exit 1
fi

echo "✓ Working directory is clean"
echo ""

# Verify versions match
CARGO_VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
PACKAGE_VERSION=$(node -p "require('./package.json').version")

if [ "$CARGO_VERSION" != "$PACKAGE_VERSION" ]; then
  echo "❌ Version mismatch!"
  echo "  Cargo.toml: $CARGO_VERSION"
  echo "  package.json: $PACKAGE_VERSION"
  echo ""
  echo "Please update both to the same version"
  exit 1
fi

echo "✓ Versions match: $VERSION"
echo ""

# Extract changelog for this version
echo "📝 Extracting changelog..."
CHANGELOG_SECTION=$(awk "/^## \[$VERSION\]/,/^## \[/" CHANGELOG.md | sed '$d' | tail -n +2)

if [ -z "$CHANGELOG_SECTION" ]; then
  echo "⚠️  No changelog section found for version $VERSION"
  echo ""
  echo "Please add a section to CHANGELOG.md:"
  echo "  ## [$VERSION] - $(date +%Y-%m-%d)"
  echo ""
  read -p "Continue anyway? (y/N) " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
  fi
  CHANGELOG_SECTION="Release $VERSION"
fi

echo "✓ Changelog section found"
echo ""

# Create annotated tag
echo "🏷️  Creating tag $TAG..."
git tag -a "$TAG" -m "Release $VERSION

$CHANGELOG_SECTION"

echo "✓ Tag created"
echo ""

# Push tag
echo "📤 Pushing tag to origin..."
git push origin "$TAG"

echo "✓ Tag pushed"
echo ""

# Create GitHub release using gh CLI if available
if command -v gh &> /dev/null; then
  echo "🐙 Creating GitHub release..."

  # Create temp file with changelog
  TEMP_FILE=$(mktemp)
  echo "$CHANGELOG_SECTION" > "$TEMP_FILE"

  gh release create "$TAG" \
    --title "Ultra $VERSION" \
    --notes-file "$TEMP_FILE"

  rm "$TEMP_FILE"

  echo "✓ GitHub release created"
  echo ""
  echo "View release:"
  echo "  https://github.com/bcentdev/ultra/releases/tag/$TAG"
else
  echo "⚠️  GitHub CLI (gh) not found"
  echo ""
  echo "To create a GitHub release manually:"
  echo "  1. Go to: https://github.com/bcentdev/ultra/releases/new"
  echo "  2. Tag: $TAG"
  echo "  3. Title: Ultra $VERSION"
  echo "  4. Copy changelog from CHANGELOG.md"
fi

echo ""
echo "✅ Release $VERSION created successfully!"
echo ""
echo "Next steps:"
echo "  1. GitHub Actions will build binaries automatically"
echo "  2. Wait for builds to complete"
echo "  3. Run: ./scripts/publish-npm.sh"
echo ""
echo "Or publish manually:"
echo "  npm publish --access public"
