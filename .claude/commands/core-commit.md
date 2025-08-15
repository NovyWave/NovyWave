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

4. **Present Options & WAIT FOR USER INPUT:**

   **Option A: CHECKPOINT Conversion**
   ```
   🔄 Found CHECKPOINT commit with accumulated changes
   📋 Analyzing all changes since last real commit...
   
   💭 Suggested commit message:
   "feat(ui): add button component with styling improvements"
   
   ✅ Convert CHECKPOINT to proper commit?
   **Type your choice:**
   y) Use suggested message
   n) Cancel  
   custom message) Type your own
   
   **WAITING FOR YOUR RESPONSE...**
   ```

   **Option B: Amend makes sense (non-CHECKPOINT)**
   ```
   📝 Last commit: "feat(ui): add button component"
   📋 Current changes: Button styling improvements
   
   💭 Updated commit message:
   "feat(ui): add button component with styling improvements"
   
   ✅ Recommended: AMEND (similar scope, unpushed)
   
   **Type your choice:**
   a) Amend with updated message
   n) Create new commit instead
   
   **WAITING FOR YOUR RESPONSE...**
   ```

   **Option C: New commit recommended**
   ```
   📋 Changed files:
   M  src/auth/login.rs
   A  src/utils/validation.rs
   
   💭 Suggested commit message:
   "feat(auth): add input validation to login flow"
   
   ⚠️  New commit recommended (different scope from last commit)
   
   **Type your choice:**
   y) Use suggested message
   n) Cancel
   custom message) Type your own
   
   **WAITING FOR YOUR RESPONSE...**
   ```

5. **CRITICAL: WAIT FOR USER CONFIRMATION**
   - **NEVER auto-execute commits without user confirmation**
   - **STOP after presenting options and wait for user input**
   - **Only proceed to Execute step after user responds**

6. **Execute (ONLY after user confirmation):**

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

## Anti-Automation Guard

**CRITICAL BEHAVIORAL RULE**: This command MUST be interactive and wait for user confirmation.

**NEVER do these things:**
- ❌ Auto-execute commits without user input
- ❌ Assume "y" when user hasn't responded
- ❌ Skip the "WAITING FOR YOUR RESPONSE..." step
- ❌ Proceed directly to git commands after presenting options

**ALWAYS do these things:**
- ✅ Present analysis and suggested message
- ✅ Show clear options with "Type your choice:"
- ✅ Display "WAITING FOR YOUR RESPONSE..."
- ✅ STOP and wait for user to type y/n/a/custom message
- ✅ Only execute git commands AFTER user confirms

**Red flags indicating failure:**
- 🚨 Executing `git add` or `git commit` immediately after showing options
- 🚨 Not waiting for user input before proceeding
- 🚨 Skipping the interactive confirmation step

**This command is INTERACTIVE by design - user confirmation is mandatory.**