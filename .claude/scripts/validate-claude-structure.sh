#!/bin/bash

# Claude Structure Validation Script
# Validates CLAUDE.md imports and command structure

echo "🔍 Validating Claude structure..."
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

# Function to check if file exists
check_file() {
    if [[ ! -f "$1" ]]; then
        echo -e "${RED}✗ Missing file: $1${NC}"
        ((ERRORS++))
        return 1
    fi
    return 0
}

# Function to check YAML frontmatter
check_yaml_frontmatter() {
    local file="$1"
    if [[ ! -f "$file" ]]; then
        return 1
    fi
    
    # Check if file starts with ---
    if ! head -n1 "$file" | grep -q "^---$"; then
        echo -e "${YELLOW}⚠ Missing YAML frontmatter: $file${NC}"
        ((WARNINGS++))
        return 1
    fi
    
    # Check if it has description
    if ! grep -q "^description:" "$file"; then
        echo -e "${YELLOW}⚠ Missing description in YAML frontmatter: $file${NC}"
        ((WARNINGS++))
        return 1
    fi
    
    return 0
}

echo "📋 Checking CLAUDE.md imports..."

# Extract all @filename imports from CLAUDE.md
while IFS= read -r line; do
    if [[ "$line" =~ ^@(.+)$ ]]; then
        import_path="${BASH_REMATCH[1]}"
        
        # Handle relative paths
        if [[ "$import_path" == ./* ]]; then
            full_path="$import_path"
        else
            full_path="./$import_path"
        fi
        
        if check_file "$full_path"; then
            echo -e "${GREEN}✓ Found: $import_path${NC}"
        fi
    fi
done < CLAUDE.md

echo
echo "🔧 Checking command structure..."

# Check all command files for proper YAML frontmatter
for cmd_file in .claude/commands/*.md; do
    if [[ -f "$cmd_file" ]]; then
        if check_yaml_frontmatter "$cmd_file"; then
            echo -e "${GREEN}✓ Valid command: $(basename "$cmd_file")${NC}"
        fi
    fi
done

echo
echo "📊 Summary:"
echo "Errors: $ERRORS"
echo "Warnings: $WARNINGS"

if [[ $ERRORS -eq 0 && $WARNINGS -eq 0 ]]; then
    echo -e "${GREEN}✅ All validations passed!${NC}"
    exit 0
elif [[ $ERRORS -eq 0 ]]; then
    echo -e "${YELLOW}⚠ Validation completed with warnings${NC}"
    exit 1
else
    echo -e "${RED}❌ Validation failed with errors${NC}"
    exit 2
fi