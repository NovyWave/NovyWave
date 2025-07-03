# Simplified Migration Guide - Single Source of Truth

## Simplified Approach: No Templates

You're absolutely right - templates create confusion and desynchronization. Better approach:

### Commands Structure (Single Source of Truth)
```
.claude/commands/
├── core-focus.md           # Copy to any project
├── core-note.md            # Copy to any project  
├── core-memory-search.md   # Copy to any project
├── core-memory-cleanup.md  # Copy to any project
├── project-start.md        # Customize for each project
└── project-stop.md         # Customize for each project
```

## Migration Process

### For New MoonZoon Project:
```bash
# 1. Copy reusable files
cp -r original/.claude/core new-project/.claude/core
cp original/.claude/commands/core-* new-project/.claude/commands/
cp -r original/.claude/frameworks new-project/.claude/frameworks

# 2. Copy and customize project commands
cp original/.claude/commands/project-start.md new-project/.claude/commands/
cp original/.claude/commands/project-stop.md new-project/.claude/commands/

# 3. Edit the project commands manually:
# - Change "NovyWave" to "NewProject" 
# - Update port if needed (8080 → 3000)
# - Update build command if different

# 4. Create PROJECT.md with project specifics
```

### For Non-MoonZoon Project:
```bash
# 1. Copy only core
cp -r original/.claude/core new-project/.claude/core
cp original/.claude/commands/core-* new-project/.claude/commands/

# 2. Create custom project commands from scratch
# 3. Skip framework layer entirely
```

## Benefits of Single Source

✅ **No desynchronization** - commands are the real thing
✅ **Clear ownership** - core-* vs project-* prefix  
✅ **Simple migration** - copy files, edit project commands
✅ **Easy maintenance** - update commands in place
✅ **No confusion** - templates don't mislead about location

## What Gets Copied vs Customized

**Always Copy (Universal):**
- `.claude/core/` - All files
- `core-*.md` commands

**Copy & Customize (Project-specific):**
- `project-*.md` commands (edit project names, ports, etc.)
- `PROJECT.md` (completely rewrite)
- `.claude/project/` patterns (rewrite for new project)

**Optional (Framework-specific):**
- `.claude/frameworks/moonzoon/` - Copy if same framework