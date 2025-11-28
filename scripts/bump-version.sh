#!/bin/bash
# NovyWave Version Bump Script
# Updates version in all project files and creates a git tag

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <new_version>"
    echo ""
    echo "Examples:"
    echo "  $0 0.2.0"
    echo "  $0 1.0.0"
    echo ""
    echo "This script will:"
    echo "  1. Update version in Cargo.toml files"
    echo "  2. Update version in tauri.conf.json"
    echo "  3. Create a git commit with the changes"
    echo "  4. Create a git tag (v<version>)"
    exit 1
}

# Check arguments
if [ $# -ne 1 ]; then
    usage
fi

NEW_VERSION="$1"

# Validate version format (basic semver)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo -e "${RED}Error: Invalid version format. Use semantic versioning (e.g., 0.2.0 or 1.0.0-beta)${NC}"
    exit 1
fi

echo -e "${GREEN}Bumping version to ${NEW_VERSION}${NC}"
echo ""

cd "$PROJECT_ROOT"

# Get current version from tauri.conf.json
CURRENT_VERSION=$(grep '"version":' src-tauri/tauri.conf.json | head -1 | sed 's/.*"\([0-9]*\.[0-9]*\.[0-9]*[^"]*\)".*/\1/')
echo -e "Current version: ${YELLOW}${CURRENT_VERSION}${NC}"
echo -e "New version:     ${GREEN}${NEW_VERSION}${NC}"
echo ""

# Update Cargo.toml files
echo "Updating Cargo.toml files..."

# Root Cargo.toml (workspace)
if [ -f "Cargo.toml" ]; then
    sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" Cargo.toml
    echo "  ✓ Cargo.toml"
fi

# Frontend
if [ -f "frontend/Cargo.toml" ]; then
    sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" frontend/Cargo.toml
    echo "  ✓ frontend/Cargo.toml"
fi

# Backend
if [ -f "backend/Cargo.toml" ]; then
    sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" backend/Cargo.toml
    echo "  ✓ backend/Cargo.toml"
fi

# Shared
if [ -f "shared/Cargo.toml" ]; then
    sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" shared/Cargo.toml
    echo "  ✓ shared/Cargo.toml"
fi

# NovyUI
if [ -f "novyui/Cargo.toml" ]; then
    sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" novyui/Cargo.toml
    echo "  ✓ novyui/Cargo.toml"
fi

# Tauri
if [ -f "src-tauri/Cargo.toml" ]; then
    sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" src-tauri/Cargo.toml
    echo "  ✓ src-tauri/Cargo.toml"
fi

# Update tauri.conf.json
echo ""
echo "Updating tauri.conf.json..."
sed -i "s/\"version\": \"${CURRENT_VERSION}\"/\"version\": \"${NEW_VERSION}\"/" src-tauri/tauri.conf.json
echo "  ✓ src-tauri/tauri.conf.json"

# Update CHANGELOG.md if it exists
if [ -f "CHANGELOG.md" ]; then
    echo ""
    echo "Updating CHANGELOG.md..."
    DATE=$(date +%Y-%m-%d)
    sed -i "s/## \[Unreleased\]/## [Unreleased]\n\n## [${NEW_VERSION}] - ${DATE}/" CHANGELOG.md
    echo "  ✓ CHANGELOG.md"
fi

echo ""
echo -e "${GREEN}Version updated successfully!${NC}"
echo ""

# Ask about git operations
read -p "Create git commit and tag? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "Creating git commit..."
    git add -A
    git commit -m "chore: bump version to ${NEW_VERSION}"
    echo "  ✓ Created commit"

    echo ""
    echo "Creating git tag..."
    git tag -a "v${NEW_VERSION}" -m "Release v${NEW_VERSION}"
    echo "  ✓ Created tag v${NEW_VERSION}"

    echo ""
    echo -e "${GREEN}Done!${NC}"
    echo ""
    echo "To push the release:"
    echo "  git push origin main"
    echo "  git push origin v${NEW_VERSION}"
else
    echo ""
    echo "Skipped git operations. To manually commit and tag:"
    echo "  git add -A"
    echo "  git commit -m \"chore: bump version to ${NEW_VERSION}\""
    echo "  git tag -a \"v${NEW_VERSION}\" -m \"Release v${NEW_VERSION}\""
fi
