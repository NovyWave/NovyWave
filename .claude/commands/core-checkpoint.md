# /core-checkpoint Command

## Purpose
Rapid development checkpoint during active coding

## CRITICAL: Slash Command = Automation

**NEVER provide consultation when user types `/core-checkpoint`**
**ALWAYS execute the checkpoint workflow immediately**

## Workflow

### 1. Quick Repository State Check
```bash
git status
git diff --name-only
```

### 2. Create Checkpoint Commit
```bash
# Stage all changes
git add .

# Create timestamped checkpoint
git commit -m "CHECKPOINT: $(date '+%Y-%m-%d %H:%M:%S')"

# Verify success
git log --oneline -1
```

## Use Cases
- Preserves work-in-progress state
- Rapid iteration during development sessions
- Before attempting risky refactoring
- End-of-session backup before stopping work

## Safety Rules
- Creates simple timestamped commits
- Never performs destructive operations
- Safe for frequent use during development

## Anti-Consultation Guard
This command MUST execute automation immediately. Never explain the workflow unless explicitly asked after completion.