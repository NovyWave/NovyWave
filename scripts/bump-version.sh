#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

VERSION=$1
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Bumping version to $VERSION..."

# Update workspace Cargo.toml
if [ -f "$PROJECT_ROOT/Cargo.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PROJECT_ROOT/Cargo.toml"
    echo "Updated Cargo.toml"
fi

# Update src-tauri/Cargo.toml
if [ -f "$PROJECT_ROOT/src-tauri/Cargo.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PROJECT_ROOT/src-tauri/Cargo.toml"
    echo "Updated src-tauri/Cargo.toml"
fi

# Update src-tauri/tauri.conf.json
if [ -f "$PROJECT_ROOT/src-tauri/tauri.conf.json" ]; then
    # Use jq if available, otherwise sed
    if command -v jq &> /dev/null; then
        tmp=$(mktemp)
        jq ".version = \"$VERSION\"" "$PROJECT_ROOT/src-tauri/tauri.conf.json" > "$tmp"
        mv "$tmp" "$PROJECT_ROOT/src-tauri/tauri.conf.json"
    else
        sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" "$PROJECT_ROOT/src-tauri/tauri.conf.json"
    fi
    echo "Updated src-tauri/tauri.conf.json"
fi

# Update shared/Cargo.toml
if [ -f "$PROJECT_ROOT/shared/Cargo.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PROJECT_ROOT/shared/Cargo.toml"
    echo "Updated shared/Cargo.toml"
fi

# Update frontend/Cargo.toml
if [ -f "$PROJECT_ROOT/frontend/Cargo.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PROJECT_ROOT/frontend/Cargo.toml"
    echo "Updated frontend/Cargo.toml"
fi

# Update backend/Cargo.toml
if [ -f "$PROJECT_ROOT/backend/Cargo.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PROJECT_ROOT/backend/Cargo.toml"
    echo "Updated backend/Cargo.toml"
fi

echo ""
echo "Version bumped to $VERSION"
echo ""
echo "Next steps:"
echo "  1. Update CHANGELOG.md with release notes"
echo "  2. Commit: git commit -am 'Release v$VERSION'"
echo "  3. Tag: git tag v$VERSION"
echo "  4. Push: git push && git push --tags"
