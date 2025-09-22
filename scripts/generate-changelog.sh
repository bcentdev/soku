#!/bin/bash

# Ultra Bundler - Automated Changelog Generator
# Generates CHANGELOG.md from git commits when creating tags

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }

# Get the latest tag
LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
NEW_VERSION=""

# Check if we're creating a new tag
if [ "$1" != "" ]; then
    NEW_VERSION="$1"
    print_info "Generating changelog for new version: $NEW_VERSION"
else
    print_error "Usage: $0 <new-version>"
    print_info "Example: $0 v0.3.0"
    exit 1
fi

# Determine commit range
if [ "$LATEST_TAG" = "" ]; then
    COMMIT_RANGE="HEAD"
    print_warning "No previous tags found, generating changelog from all commits"
else
    COMMIT_RANGE="$LATEST_TAG..HEAD"
    print_info "Generating changelog from $LATEST_TAG to HEAD"
fi

# Create changelog header
CHANGELOG_TEMP=$(mktemp)
echo "# Changelog" > "$CHANGELOG_TEMP"
echo "" >> "$CHANGELOG_TEMP"
echo "All notable changes to Ultra Bundler will be documented in this file." >> "$CHANGELOG_TEMP"
echo "" >> "$CHANGELOG_TEMP"
echo "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)," >> "$CHANGELOG_TEMP"
echo "and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)." >> "$CHANGELOG_TEMP"
echo "" >> "$CHANGELOG_TEMP"

# Generate new version section
echo "## [$NEW_VERSION] - $(date +%Y-%m-%d)" >> "$CHANGELOG_TEMP"
echo "" >> "$CHANGELOG_TEMP"

# Get commits and categorize them
FEATURES=$(git log --format="%s" "$COMMIT_RANGE" | grep "^feat:" | sed 's/^feat: /- /' || true)
FIXES=$(git log --format="%s" "$COMMIT_RANGE" | grep "^fix:" | sed 's/^fix: /- /' || true)
REFACTOR=$(git log --format="%s" "$COMMIT_RANGE" | grep "^refactor:" | sed 's/^refactor: /- /' || true)
CHORE=$(git log --format="%s" "$COMMIT_RANGE" | grep "^chore:" | sed 's/^chore: /- /' || true)
PERF=$(git log --format="%s" "$COMMIT_RANGE" | grep "^perf:" | sed 's/^perf: /- /' || true)
DOCS=$(git log --format="%s" "$COMMIT_RANGE" | grep "^docs:" | sed 's/^docs: /- /' || true)

# Add sections with content
if [ ! -z "$FEATURES" ]; then
    echo "### ✨ Added" >> "$CHANGELOG_TEMP"
    echo "$FEATURES" >> "$CHANGELOG_TEMP"
    echo "" >> "$CHANGELOG_TEMP"
fi

if [ ! -z "$FIXES" ]; then
    echo "### 🐛 Fixed" >> "$CHANGELOG_TEMP"
    echo "$FIXES" >> "$CHANGELOG_TEMP"
    echo "" >> "$CHANGELOG_TEMP"
fi

if [ ! -z "$PERF" ]; then
    echo "### ⚡ Performance" >> "$CHANGELOG_TEMP"
    echo "$PERF" >> "$CHANGELOG_TEMP"
    echo "" >> "$CHANGELOG_TEMP"
fi

if [ ! -z "$REFACTOR" ]; then
    echo "### 🔧 Changed" >> "$CHANGELOG_TEMP"
    echo "$REFACTOR" >> "$CHANGELOG_TEMP"
    echo "" >> "$CHANGELOG_TEMP"
fi

if [ ! -z "$CHORE" ]; then
    echo "### 🏠 Maintenance" >> "$CHANGELOG_TEMP"
    echo "$CHORE" >> "$CHANGELOG_TEMP"
    echo "" >> "$CHANGELOG_TEMP"
fi

if [ ! -z "$DOCS" ]; then
    echo "### 📚 Documentation" >> "$CHANGELOG_TEMP"
    echo "$DOCS" >> "$CHANGELOG_TEMP"
    echo "" >> "$CHANGELOG_TEMP"
fi

# Append existing changelog if it exists
if [ -f "CHANGELOG.md" ]; then
    # Skip the header of existing changelog and append the rest
    tail -n +8 "CHANGELOG.md" >> "$CHANGELOG_TEMP" 2>/dev/null || true
fi

# Replace the original changelog
mv "$CHANGELOG_TEMP" "CHANGELOG.md"

print_success "Changelog generated successfully!"
print_info "Review CHANGELOG.md and commit it before creating the tag"

# Show preview
echo ""
print_info "Preview of new changelog section:"
echo "$(head -30 CHANGELOG.md)"

# Offer to create tag
echo ""
read -p "$(echo -e ${YELLOW}Create tag $NEW_VERSION now? [y/N]:${NC} )" -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git add CHANGELOG.md
    git commit -m "chore: update changelog for $NEW_VERSION release"
    git tag -a "$NEW_VERSION" -m "Release $NEW_VERSION"
    print_success "Tag $NEW_VERSION created successfully!"
    print_info "Push with: git push origin main --tags"
else
    print_info "Tag not created. You can create it manually with:"
    print_info "git tag -a $NEW_VERSION -m \"Release $NEW_VERSION\""
fi