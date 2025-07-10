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

### ⚠️ CRITICAL IMPLEMENTATION RULES

**RULE 1 - CHECKPOINT ANALYSIS**: Must ALWAYS run BOTH commands in parallel:
- `git show HEAD` or `git diff HEAD~1` - to see CHECKPOINT accumulated changes  
- `git diff` - to see unstaged changes
- **ANALYZE BOTH** together for complete commit message scope
- **DO NOT** analyze only unstaged changes when CHECKPOINT exists - this ignores the main work!

**RULE 2 - INTERACTIVE WORKFLOW**: NEVER execute git commands without user confirmation:
- **ALWAYS** present analysis and suggested commit message FIRST
- **ALWAYS** wait for user approval: y/n/custom response
- **NEVER** auto-execute git commands even after conversation compaction
- **ONLY** execute after explicit user confirmation (y, custom message, etc.)

### Smart Commit Workflow:

1. **Analyze Current State:**
   - Run `git status` to see untracked/modified files
   - Run `git diff --staged` and `git diff` to see actual changes
   - Run `git log -1 --oneline` to see recent commit style
   - Check if last commit was pushed: `git log origin/main..HEAD --oneline`

2. **Checkpoint Detection (Priority #1):**
   - Check if last commit message = "CHECKPOINT"
   - If CHECKPOINT found: **MANDATORY** run `git show HEAD` or `git diff HEAD~1` to analyze accumulated changes
   - **CRITICAL**: Must include CHECKPOINT changes in analysis, not just unstaged changes
   - If no CHECKPOINT: Continue to normal amend detection

3. **Multi-line Message Analysis (for CHECKPOINT commits):**
   - Analyze accumulated changes for logical groupings
   - Create single commit with structured multi-line message
   - Each line represents a distinct logical change
   - Use conventional commit format for each line
   
   **Multi-line Structure:**
   - First line: Primary change (most significant)
   - Additional lines: Secondary related changes
   - Each line follows conventional commit format
   - Group related changes logically
   
   **Multi-line Examples:**
   ```
   ❌ Old splitting approach: Multiple commits with --allow-empty
   ✅ New multi-line approach: Single commit, multiple lines
   
   feat(auth): implement user authentication system
   fix(ui): resolve layout overflow in sidebar panel
   docs(api): update authentication endpoint documentation
   
   ❌ Old: "docs: update memory patterns and add scrollbar rules"
   ✅ New multi-line:
   docs(memory): store session restoration debugging patterns
   docs(layout): add scrollbar hierarchy mastery patterns
   docs(workflow): update checkpoint conversion process
   ```

4. **Smart Amend Detection (if not CHECKPOINT):**
   - Analyze current changes vs last commit scope
   - Check if last commit is unpushed (safe to amend)
   - Determine if amend makes sense or new commit is better

5. **Present Options:**

   **Option A: Multi-line CHECKPOINT (multiple logical changes)**
   ```
   🔄 Found CHECKPOINT commit with multiple distinct changes
   📋 Analyzing accumulated changes...
   
   💡 Recommended: Single commit with multi-line message
   
   📋 Proposed Multi-line Message:
   fix(ui): document session restoration race condition solution
   docs(layout): add scrollbar hierarchy mastery patterns  
   docs(memory): create comprehensive layout solutions archive
   
   ✅ Create single commit with multi-line message?
   y) Use proposed multi-line message
   n) Cancel and let me handle manually
   custom) Provide different message structure
   ```

   **Option B: Simple CHECKPOINT Conversion**
   ```
   🔄 Found CHECKPOINT commit with accumulated changes
   📋 Analyzing CHECKPOINT contents with `git show HEAD`...
   📋 Analyzing unstaged changes with `git diff`...
   📋 Combining both to understand complete scope...
   
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

6. **Present Options & Wait for User Response:**
   - Show analysis results
   - Present recommended action with clear options
   - **STOP and wait for user input**
   - Do NOT proceed until user responds with y/n/custom/etc.

7. **Execute (ONLY after user confirmation):**

   **For Multi-line CHECKPOINT:**
   - Stage any unstaged changes: `git add .`
   - Create commit message with multi-line format using HEREDOC:
     ```bash
     git commit --amend -m "$(cat <<'EOF'
     fix(ui): document session restoration race condition solution
     docs(layout): add scrollbar hierarchy mastery patterns
     docs(memory): create comprehensive layout solutions archive
     EOF
     )"
     ```
   - Show: "✅ Created single commit with multi-line message"

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

**Checkpoint multi-line workflow:**
```bash
# After accumulating multiple logical changes...
/core-commit
# 🔄 Found CHECKPOINT with multiple distinct changes
# 💡 Recommended: Single commit with multi-line message
# y) Use proposed multi-line message
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

- **Multi-line messaging**: Analyzes CHECKPOINT commits for multiple logical changes and creates single commit with structured multi-line message
- **Checkpoint conversion**: Automatically converts CHECKPOINT commits to proper conventional commits
- **Smart amend detection**: Analyzes if current changes should amend previous commit
- **Logical grouping**: Identifies related changes and groups them with conventional commit format
- **Safety checks**: Warns about rewriting published history
- **Scope analysis**: Compares change types between current and last commit
- **Conventional commits**: Suggests proper commit message format with correct scopes for each line
- **Seamless workflow**: Perfect partner with `/core-checkpoint` for rapid iteration
- **Simplified approach**: Single commit with clear multi-line structure instead of complex empty commit splitting