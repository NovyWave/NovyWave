# Migration Guide for Modular Claude Configuration

## Command Naming Convention

Since Claude Code doesn't support nested command folders, we use prefixes:

### Core Commands (Universal)
- `core-focus.md` - Display productivity context  
- `core-note.md` - Store discoveries with smart archiving
- `core-memory-search.md` - Search Memory MCP
- `core-memory-cleanup.md` - Monthly maintenance

### Project Commands (Customized)
- `project-start.md` - Start development server
- `project-stop.md` - Stop development server

## Quick Migration for New Projects

### 1. Copy Core Files
```bash
# Copy universal Claude configuration
cp -r original-project/.claude/core new-project/.claude/core
cp original-project/.claude/commands/core-* new-project/.claude/commands/

# Copy framework layer (if applicable)
cp -r original-project/.claude/frameworks new-project/.claude/frameworks
```

### 2. Create Project-Specific Files
```bash
# Create PROJECT.md
echo "# MyProject Configuration" > new-project/PROJECT.md

# Customize commands from templates
sed 's/{{PROJECT_NAME}}/MyProject/g; s/{{DEV_PORT}}/8080/g; s/{{BUILD_COMMAND}}/makers start/g' \
  new-project/.claude/frameworks/moonzoon/templates/start.template.md > \
  new-project/.claude/commands/project-start.md

sed 's/{{PROJECT_NAME}}/MyProject/g; s/{{DEV_PORT}}/8080/g' \
  new-project/.claude/frameworks/moonzoon/templates/stop.template.md > \
  new-project/.claude/commands/project-stop.md
```

### 3. Update CLAUDE.md
```markdown
# CLAUDE.md

<!-- Core System Layer -->
@.claude/core/SYSTEM.md
@.claude/core/memory-management.md
@.claude/core/mcp-tools.md
@.claude/core/development.md

<!-- Framework Layer -->
@.claude/frameworks/moonzoon/FRAMEWORK.md
@.claude/frameworks/moonzoon/patterns.md
@.claude/frameworks/moonzoon/debugging.md

<!-- Project Layer -->
@PROJECT.md
@.claude/project/custom-patterns.md

<!-- Auto-Generated Context -->
@.claude/ai-docs/focus-context.md
```

## Available Slash Commands After Migration

**Universal Commands:**
- `/core-focus` - Show productivity context
- `/core-note` - Store discoveries
- `/core-memory-search` - Search patterns
- `/core-memory-cleanup` - Monthly cleanup

**Project Commands:**
- `/project-start` - Start dev server
- `/project-stop` - Stop dev server

## Benefits

✅ **2-minute setup** for new MoonZoon projects
✅ **1-minute setup** for new any-framework projects  
✅ **Clear ownership** - prefix shows what's reusable vs project-specific
✅ **Easy updates** - pull latest core improvements
✅ **Framework agnostic** - core works everywhere