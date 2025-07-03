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

**Execute this single command:**
```bash
git add . && \
if [ "$(git log -1 --pretty=format:'%s')" = "CHECKPOINT" ]; then \
  git commit --amend --no-edit && echo "✅ Updated CHECKPOINT"; \
else \
  git commit -m "CHECKPOINT" && echo "✅ Created CHECKPOINT"; \
fi
```

**Logic:**
- Always stage all changes first
- If last commit is "CHECKPOINT": amend it
- If last commit is not "CHECKPOINT": create new one
- Fast, atomic operation with no analysis

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