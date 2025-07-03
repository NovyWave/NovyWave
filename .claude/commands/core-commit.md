---
allowed-tools: Bash
description: 'Smart git commit: /core-commit (interactive) or /core-commit a (amend)'
---

# Smart Git Commit

Intelligent git commit workflow with change analysis and amend validation.

## Usage

```bash
/core-commit              # Interactive commit with suggested message
/core-commit a            # Smart amend with safety checks
```

## Your Task

### For `/core-commit` (no parameters):

1. **Analyze Current State:**
   - Run `git status` to see untracked/modified files
   - Run `git diff --staged` and `git diff` to see actual changes
   - Run `git log -1 --oneline` to see recent commit style

2. **Suggest Commit Message:**
   - Analyze the nature of changes (new feature, bug fix, refactor, etc.)
   - Look at file patterns and diff content
   - Follow repository's commit message style and conventional commits format
   - Format: "type(scope): description" (e.g., "feat(ui): add button component", "fix(auth): resolve login validation", "refactor(core): simplify memory management")

3. **Present to User:**
   ```
   📋 Changed files:
   M  src/components/Button.rs
   A  src/utils/validation.rs
   
   💭 Suggested commit message:
   "Add input validation and improve button styling"
   
   ✅ Approve this message? (y/n/custom message)
   ```

4. **Execute:**
   - Wait for user confirmation or custom message
   - Stage all changes with `git add .`
   - Commit with proper message formatting
   - Show final `git status` to confirm

### For `/core-commit a` (amend):

1. **Safety Analysis:**
   - Check if last commit was pushed: `git log origin/main..HEAD --oneline`
   - Warn if amending pushed commits (rewrites history)

2. **Relevance Check:**
   - Show last commit message: `git log -1 --pretty=format:"%s"`
   - Analyze current changes vs last commit scope
   - Determine if amend makes sense

3. **Smart Decision:**
   ```
   📝 Last commit: "Fix button styling issues"
   📋 Current changes: More button style fixes in Button.rs
   
   ✅ Safe to amend (similar scope, not pushed)
   or
   ⚠️  Recommend new commit (different scope/already pushed)
   ```

4. **Execute if Appropriate:**
   - Stage changes: `git add .`
   - Amend: `git commit --amend --no-edit`
   - Or suggest creating new commit instead

## Examples

**Interactive commit:**
```bash
/core-commit
# Shows analysis, suggests message, waits for approval
```

**Smart amend:**
```bash
/core-commit a
# Analyzes if amend makes sense, executes or suggests alternative
```

## Safety Features

- **Push detection**: Warn about rewriting published history
- **Scope analysis**: Compare change types between commits
- **Change validation**: Ensure amend actually makes logical sense
- **User confirmation**: Always confirm before executing git operations