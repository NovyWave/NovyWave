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

3. **Deep Technical Analysis (for CHECKPOINT commits):**
   - **MANDATORY: Analyze code changes line-by-line for technical implementation details**
   - Identify specific functions, methods, and architectural patterns modified
   - Capture the "why" behind solutions: workarounds, compatibility issues, signal problems
   - Note architectural context: dual-state systems, reactive patterns, manual triggers
   - Distinguish between fixes (bugs), features (new functionality), refactoring, docs, tooling
   
   **Multi-line vs Single-line Decision:**
   - **PREFER MULTI-LINE** for better scannability when changes span multiple areas/scopes
   - **Each line = one logical change** with conventional commit format
   - **Each line should be scannable** and independently meaningful
   - Use single-line only when truly one cohesive change
   
   **Multi-line Structure (PREFERRED):**
   - Each line is a complete conventional commit: `type(scope): description`
   - Lines ordered by importance/impact (most significant first)
   - Group related changes but keep lines atomic and scannable
   - Each line should make sense if read independently in git log
   
   **Technical Analysis Examples:**
   ```
   ❌ Single-line (less scannable):
   "fix: resolve config persistence issues and update memory patterns and improve auth"
   
   ✅ Multi-line (highly scannable):
   fix(config): implement manual save_config_to_backend() workaround for MutableVec signals
   docs(memory): document dual-state sync architecture and reactive signal limitations  
   refactor(config): identify areas for ConfigStore migration to eliminate sync complexity
   
   ❌ Shallow multi-line:
   fix: config issues
   docs: update patterns
   feat: add auth
   
   ✅ Technical depth multi-line:
   fix(config): resolve expanded_scopes persistence with manual save triggers for MutableVec compatibility
   docs(claude): clean up bloated focus-context.md preventing Claude effectiveness issues
   enhance(tools): add mandatory technical analysis requirements to /core-commit command
   
   ❌ Mixed format (avoid):
   "docs(claude): clean up focus-context.md and implement size limits
   
   Removed 925+ repetitive recovery contexts..."
   
   ✅ Pure multi-line conventional (preferred):
   docs(claude): clean up bloated focus-context.md preventing Claude effectiveness issues
   fix(hooks): implement deterministic size limits in PostCompact hook for focus-context.md
   enhance(tools): add mandatory technical analysis requirements to /core-commit command
   ```

4. **Smart Amend Detection (if not CHECKPOINT):**
   - Analyze current changes vs last commit scope
   - Check if last commit is unpushed (safe to amend)
   - Determine if amend makes sense or new commit is better

5. **Present Options:**

   **Option A: Multi-line CHECKPOINT (PREFERRED - multiple logical changes)**
   ```
   🔄 Found CHECKPOINT commit with multiple distinct changes
   📋 Analyzing accumulated changes for logical grouping...
   
   💡 Recommended: Multi-line conventional commits (better scannability)
   
   📋 Proposed Multi-line Message:
   docs(claude): clean up bloated focus-context.md preventing Claude effectiveness issues
   fix(hooks): implement deterministic size limits in PostCompact hook for focus-context.md
   enhance(tools): add mandatory technical analysis requirements to /core-commit command
   
   ✅ Create single commit with multi-line conventional format?
   y) Use proposed multi-line message
   n) Cancel and let me handle manually
   custom) Provide different message structure
   ```

   **Option B: Single-line CHECKPOINT (when truly one cohesive change)**
   ```
   🔄 Found CHECKPOINT commit with single cohesive change
   📋 Analyzing CHECKPOINT contents with `git show HEAD`...
   📋 Analyzing unstaged changes with `git diff`...
   📋 Single logical change detected...
   
   💭 Suggested single-line commit message:
   fix(config): implement manual save_config_to_backend() workaround for MutableVec reactive signal compatibility
   
   ✅ Convert CHECKPOINT to single-line commit?
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
   
   💭 Suggested commit message with technical analysis:
   ```
   feat(auth): implement input validation with custom validation middleware
   
   Added ValidationError enum, validate_login_input() function with email regex 
   and password strength checks. Integrated with login_handler() using Result<T, E> 
   pattern for error propagation to frontend.
   ```
   
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

   **For Multi-line CHECKPOINT (PREFERRED):**
   - Stage any unstaged changes: `git add .`
   - Create commit message with multi-line conventional format using HEREDOC:
     ```bash
     git commit --amend -m "$(cat <<'EOF'
     docs(claude): clean up bloated focus-context.md preventing Claude effectiveness issues
     fix(hooks): implement deterministic size limits in PostCompact hook for focus-context.md
     enhance(tools): add mandatory technical analysis requirements to /core-commit command
     EOF
     )"
     ```
   - Show: "✅ Created single commit with multi-line conventional format"

   **For Single-line CHECKPOINT:**
   - Stage any unstaged changes: `git add .`
   - Amend CHECKPOINT with single conventional commit: `git commit --amend -m "new message"`
   - Show: "✅ Converted CHECKPOINT to single conventional commit: [message]"

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

- **Multi-line conventional commits**: PREFERRED approach for better git log scannability when changes span multiple areas/scopes
- **Scannable format**: Each line is an atomic conventional commit that makes sense independently
- **Logical grouping**: Identifies distinct changes and groups them by importance/impact  
- **Checkpoint conversion**: Automatically converts CHECKPOINT commits to proper conventional commits
- **Smart amend detection**: Analyzes if current changes should amend previous commit
- **Deep technical analysis**: Line-by-line code analysis to capture implementation details, architectural context, and workarounds
- **Technical depth**: Identifies specific functions, patterns, and "why" behind solutions instead of shallow summaries  
- **Pure conventional format**: Avoids mixed formats (conventional + description blocks) that reduce scannability
- **Safety checks**: Warns about rewriting published history
- **Scope analysis**: Compares change types between current and last commit
- **Seamless workflow**: Perfect partner with `/core-checkpoint` for rapid iteration
- **Decision logic**: Clear preference for multi-line when multiple logical changes exist