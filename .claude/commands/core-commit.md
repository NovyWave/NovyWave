---
allowed-tools: Bash
description: 'Smart git commit with checkpoint conversion and auto-amend detection'
---

# Smart Git Commit

Intelligent git commit workflow with checkpoint conversion, change analysis, and auto-amend detection.

## Usage

```bash
/core-commit              # Interactive commit with suggested message
```

## Your Task

### Smart Commit Workflow:

1. **Analyze Current State:**
   - Run `git status` to see untracked/modified files
   - Run `git diff --staged` and `git diff` to see actual changes
   - Run `git log -1 --oneline` to see recent commit style
   - Check if last commit was pushed: `git log origin/main..HEAD --oneline`

2. **Checkpoint Detection (Priority #1):**
   - Check if last commit message = "CHECKPOINT"
   - If CHECKPOINT found: Skip normal workflow, go to checkpoint conversion
   - If no CHECKPOINT: Continue to normal amend detection

3. **Smart Amend Detection (if not CHECKPOINT):**
   - Analyze current changes vs last commit scope
   - Check if last commit is unpushed (safe to amend)
   - Determine if amend makes sense or new commit is better

4. **Present Options:**

   **Option A: CHECKPOINT Conversion**
   ```
   🔄 Found CHECKPOINT commit with accumulated changes
   📋 Analyzing all changes since last real commit...
   
   💭 Suggested commit message:
   "feat(ui): add button component with styling improvements"
   
   ✅ Convert CHECKPOINT to proper commit?
   y) Use suggested message
   n) Cancel  
   custom message) Type your own
   ```

   **Option B: Amend makes sense (non-CHECKPOINT)**
   ```
   📝 Last commit: "feat(ui): add button component"
   📋 Current changes: Button styling improvements
   
   💭 Updated commit message:
   "feat(ui): add button component with styling improvements"
   
   ✅ Recommended: AMEND (similar scope, unpushed)
   
   Options:
   a) Amend with updated message
   n) Create new commit instead
   ```

   **Option C: New commit recommended**
   ```
   📋 Changed files:
   M  src/auth/login.rs
   A  src/utils/validation.rs
   
   💭 Suggested commit message:
   "feat(auth): add input validation to login flow"
   
   ⚠️  New commit recommended (different scope from last commit)
   
   Options:
   y) Use suggested message
   n) Cancel
   custom message) Type your own
   ```

5. **Execute:**

   **For CHECKPOINT Conversion:**
   - Stage any unstaged changes: `git add .`
   - Amend CHECKPOINT with staged changes: `git commit --amend -m "new message"`
   - Show: "✅ Converted CHECKPOINT to: [message]"

   **For Normal Workflow:**
   - Stage changes with `git add .`
   - For amend: Use `git commit --amend -m "updated message"` (rewrite message based on all accumulated changes)
   - For new commit: Use `git commit -m "message"` (clean conventional format, no Claude boilerplate)
   - Show final `git status` to confirm

## Examples

**Checkpoint conversion workflow:**
```bash
# After using /core-checkpoint for rapid iteration...
/core-commit
# 🔄 Found CHECKPOINT commit with accumulated changes
# Analyzes all changes, suggests proper conventional commit
```

**Normal interactive commit:**
```bash
/core-commit
# Shows analysis, suggests message, waits for approval
```

## Features

- **Checkpoint conversion**: Automatically converts CHECKPOINT commits to proper conventional commits
- **Smart amend detection**: Analyzes if current changes should amend previous commit
- **Safety checks**: Warns about rewriting published history
- **Scope analysis**: Compares change types between current and last commit
- **Conventional commits**: Suggests proper commit message format
- **Seamless workflow**: Perfect partner with `/core-checkpoint` for rapid iteration