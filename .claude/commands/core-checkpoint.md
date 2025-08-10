---
allowed-tools: Bash
description: 'Ultra-fast WIP saves: first call creates CHECKPOINT, subsequent calls amend'
---

# Ultra-Fast Checkpoint

Lightning-fast work-in-progress saves for rapid iteration without thinking about commit messages.

## Usage

```bash
/core-checkpoint             # Ultra-fast WIP save
```

## Your Task

### Ultra-Fast Checkpoint Workflow:

1. **Check Last Commit:**
   - Run `git log -1 --pretty=format:"%s"` to get last commit message

2. **Checkpoint Logic:**
   
   **If last commit ≠ "CHECKPOINT":**
   - Stage all changes: `git add .`
   - Create new checkpoint: `git commit -m "CHECKPOINT"`
   - Show: "✅ Created CHECKPOINT"

   **If last commit = "CHECKPOINT":**
   - Stage all changes: `git add .`  
   - Amend to existing: `git commit --amend --no-edit`
   - Show: "✅ Updated CHECKPOINT"

3. **No Analysis:**
   - No diff analysis
   - No scope checking
   - No conventional commit formatting
   - No user confirmation
   - Just fast, reliable saves

## Examples

**Rapid iteration workflow:**
```bash
# Work on feature...
/core-checkpoint    # "✅ Created CHECKPOINT"

# More changes...  
/core-checkpoint    # "✅ Updated CHECKPOINT"

# Even more changes...
/core-checkpoint    # "✅ Updated CHECKPOINT"

# Ready for final commit...
/core-commit        # Analyzes all accumulated changes
                    # Creates proper conventional commit
```

## Features

- **Lightning fast**: No analysis, no checks, just save
- **Never lose work**: Always have a safety net
- **Seamless workflow**: Perfect partner with `/core-commit`
- **Zero friction**: Type and go, no thinking required