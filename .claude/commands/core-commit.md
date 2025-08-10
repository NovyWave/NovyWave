# /core-commit Command

## Purpose
Create clean conventional commit from accumulated CHECKPOINT changes

## CRITICAL: Slash Command = Automation

**NEVER provide consultation when user types `/core-commit`**
**ALWAYS execute the workflow immediately**

## Workflow

### 1. Git Analysis (Parallel Execution)
Run these commands in parallel using Bash tool:

```bash
# Analyze current repository state
git status
git diff --staged
git diff HEAD
git log --oneline -10
```

### 2. Change Analysis
Identify distinct logical changes by scope:
- `ui` - User interface changes
- `config` - Configuration updates  
- `feat` - New features
- `fix` - Bug fixes
- `refactor` - Code restructuring
- `docs` - Documentation changes
- `chore` - Maintenance tasks

### 3. Commit Creation
Create single commit with multi-line message format:

**Single-Line Format (when one logical change):**
```
fix(ui): resolve panel resize issues
```

**Multi-Line Format (when multiple logical changes):**
```
fix(ui): resolve panel resize issues in docked-to-bottom mode
fix(config): preserve dock mode settings during workspace saves  
refactor(frontend): modularize main.rs into focused modules
```

### 4. Git Operations (Parallel Execution)
```bash
# Stage relevant files
git add .

# Create commit with heredoc format
git commit -m "$(cat <<'EOF'
fix(ui): resolve panel resize issues in docked-to-bottom mode
fix(config): preserve dock mode settings during workspace saves
refactor(frontend): modularize main.rs into focused modules
EOF
)"

# Verify success
git status
```

## Benefits
- **Clear git blame**: Shows all relevant changes when investigating specific files
- **Semantic organization**: Each line follows conventional commits with proper scoping
- **Better debugging**: Complete context visible in file history
- **Clean history**: One commit per development session with clear scope breakdown

## Safety Rules
- **CRITICAL: NEVER perform destructive git operations without explicit user confirmation**
- Never use git commands with `-i` flag (interactive not supported)
- DO NOT push to remote repository unless explicitly asked
- Only create commit - never rebase, reset, or force operations

## Anti-Consultation Guard
This command MUST execute automation immediately. Never explain how it works unless explicitly asked after completion.