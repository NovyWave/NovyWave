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

3. **Split Analysis (for CHECKPOINT commits):**
   - Analyze accumulated changes for logical boundaries
   - Consider splitting when CHECKPOINT contains multiple distinct concerns
   - Look for natural commit boundaries: different file types, scopes, or purposes
   
   **Split Criteria:**
   - Different conventional commit scopes (ui vs docs vs feat)
   - Unrelated file changes (frontend vs backend vs config vs memory)
   - Different logical purposes (bug fixes vs new features vs documentation)
   - Multiple complete features or solutions
   
   **Split Examples:**
   ```
   ❌ Single commit: "feat: add authentication and fix layout bugs"
   ✅ Split commits: 
      - "feat(auth): implement user authentication system"
      - "fix(ui): resolve layout overflow in sidebar panel"
   
   ❌ Single commit: "docs: update memory patterns and add scrollbar rules"  
   ✅ Split commits:
      - "docs(memory): store session restoration debugging patterns"
      - "docs(layout): add scrollbar hierarchy mastery patterns"
   ```

4. **Smart Amend Detection (if not CHECKPOINT):**
   - Analyze current changes vs last commit scope
   - Check if last commit is unpushed (safe to amend)
   - Determine if amend makes sense or new commit is better

5. **Present Options:**

   **Option A: Split CHECKPOINT (multiple logical changes)**
   ```
   🔄 Found CHECKPOINT commit with multiple distinct changes
   📋 Analyzing accumulated changes...
   
   💡 Recommended: SPLIT into multiple commits
   
   📋 Proposed Split:
   1. fix(ui): document session restoration race condition solution
   2. docs(layout): add scrollbar hierarchy mastery patterns  
   3. docs(memory): create comprehensive layout solutions archive
   
   ✅ Split CHECKPOINT into 3 logical commits?
   1) Implement proposed split automatically
   2) Cancel and let me handle manually
   3) Suggest different split pattern
   ```

   **Option B: Simple CHECKPOINT Conversion**
   ```
   🔄 Found CHECKPOINT commit with accumulated changes
   📋 Analyzing all changes since last real commit...
   
   💭 Suggested commit message:
   "feat(ui): add button component with styling improvements"
   
   ✅ Convert CHECKPOINT to single commit?
   y) Use suggested message
   n) Cancel  
   custom message) Type your own
   ```

   **Option C: Amend makes sense (non-CHECKPOINT)**
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

   **Option D: New commit recommended**
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

6. **Execute:**

   **For CHECKPOINT Split:**
   - Stage any unstaged changes: `git add .`
   - Amend CHECKPOINT with first commit: `git commit --amend -m "first message"`
   - Create additional commits: `git commit --allow-empty -m "second message"`
   - Repeat for each logical split
   - Show: "✅ Split CHECKPOINT into [N] commits"

   **For Simple CHECKPOINT Conversion:**
   - Stage any unstaged changes: `git add .`
   - Amend CHECKPOINT with staged changes: `git commit --amend -m "new message"`
   - Show: "✅ Converted CHECKPOINT to: [message]"

   **For Normal Workflow:**
   - Stage changes with `git add .`
   - For amend: Use `git commit --amend -m "updated message"` (rewrite message based on all accumulated changes)
   - For new commit: Use `git commit -m "message"` (clean conventional format, no Claude boilerplate)
   - Show final `git status` to confirm

## Examples

**Checkpoint splitting workflow:**
```bash
# After accumulating multiple logical changes...
/core-commit
# 🔄 Found CHECKPOINT with multiple distinct changes
# 💡 Recommended: SPLIT into 3 logical commits
# 1) Implement proposed split automatically
```

**Simple checkpoint conversion:**
```bash
# After using /core-checkpoint for single feature...
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

- **Intelligent splitting**: Analyzes CHECKPOINT commits for multiple logical changes and suggests splits
- **Checkpoint conversion**: Automatically converts CHECKPOINT commits to proper conventional commits
- **Smart amend detection**: Analyzes if current changes should amend previous commit
- **Split criteria analysis**: Identifies natural commit boundaries by scope, file type, and purpose
- **Safety checks**: Warns about rewriting published history
- **Scope analysis**: Compares change types between current and last commit
- **Conventional commits**: Suggests proper commit message format with correct scopes
- **Seamless workflow**: Perfect partner with `/core-checkpoint` for rapid iteration
- **Automated execution**: Can implement proposed splits automatically with user approval