---
model: claude-sonnet-4-0
allowed-tools: Bash
description: 'Smart git commit with comprehensive analysis and checkpoint conversion'
---

# Git Commit

**Command:** `/commit`

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
   - If CHECKPOINT found: **COMPREHENSIVE DIFF ANALYSIS REQUIRED**
   - Use `git show --name-only HEAD` to see ALL files changed in CHECKPOINT
   - Use `git diff HEAD~1` to see ALL accumulated changes since last real commit
   - **NEVER focus only on unstaged changes** - analyze the complete CHECKPOINT scope
   - If no CHECKPOINT: Continue to normal amend detection

3. **Smart Amend Detection (if not CHECKPOINT):**
   - Analyze current changes vs last commit scope
   - Check if last commit is unpushed (safe to amend)
   - Determine if amend makes sense or new commit is better

4. **COMPREHENSIVE CHANGE ANALYSIS (for CHECKPOINT):**
   - **File Categorization**: Group changes by type (new features, config, docs, refactor)
   - **Scope Detection**: Identify primary functional area (ui, backend, canvas, config, etc.)
   - **Feature Assessment**: Distinguish between major feature implementations vs minor tweaks
   - **Impact Analysis**: Count new functions, state changes, UI components, API changes
   - **NEVER minimize major implementations** - properly recognize substantial work
   - **Multi-Impact Detection**: When multiple major change areas exist, use impact-based prioritization

5. **Present Options & WAIT FOR USER INPUT:**

   **Option A: CHECKPOINT Conversion (IMPROVED ANALYSIS)**
   ```
   🔄 Found CHECKPOINT commit with accumulated changes
   📋 COMPREHENSIVE ANALYSIS of all changes since last real commit:
   
   📁 Files Modified (8 total):
   - frontend/src/waveform_canvas.rs: +89 lines (NEW: zoom functions, timeline range override)
   - frontend/src/state.rs: +5 lines (NEW: zoom state globals)
   - frontend/src/config.rs: +56 lines (NEW: zoom persistence, reactive triggers)
   - frontend/src/main.rs: +15 lines (NEW: W/S keyboard shortcuts)
   - frontend/src/views.rs: +4 lines (CONNECT: zoom buttons to functions)
   - shared/src/lib.rs: [shared crate changes for persistence]
   - docs/canvas_5.md: +305 lines (NEW: comprehensive technical documentation)
   - .novywave: +3 lines (CONFIG: new zoom state fields)
   
   🚀 FEATURE ASSESSMENT: **MAJOR FEATURE IMPLEMENTATION**
   - Complete timeline zoom system (1x-16x range)
   - Center-focused zoom with bounds checking
   - Keyboard shortcuts (W/S) + button integration
   - Full persistence with reactive auto-save
   - Canvas integration with dynamic range calculation
   
   💭 Suggested commit message:
   "Implement timeline zoom functionality with keyboard shortcuts
   
   - Add timeline zoom state management with 1x-16x zoom range
   - Implement center-focused zoom in/out with bounds checking  
   - Add W/S keyboard shortcuts for zoom control
   - Connect existing zoom buttons to zoom functions
   - Add zoom state persistence with reactive auto-save
   - Update timeline range calculation to respect zoom level
   - Add reactive canvas redraws on zoom changes
   - Include comprehensive technical documentation"
   
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
   "Add button component with styling improvements"
   
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
   "Add input validation to login flow"
   
   ⚠️  New commit recommended (different scope from last commit)
   
   **Type your choice:**
   y) Use suggested message
   n) Cancel
   custom message) Type your own
   
   **WAITING FOR YOUR RESPONSE...**
   ```

6. **CRITICAL: WAIT FOR USER CONFIRMATION**
   - **NEVER auto-execute commits without user confirmation**
   - **STOP after presenting options and wait for user input**
   - **Only proceed to Execute step after user responds**

7. **Execute (ONLY after user confirmation):**

   **For CHECKPOINT Conversion:**
   - Stage any unstaged changes: `git add .`
   - Amend CHECKPOINT with staged changes: `git commit --amend -m "new message"`
   - Show: "✅ Converted CHECKPOINT to: [message]"

   **For Normal Workflow:**
   - Stage changes with `git add .`
   - For amend: Use `git commit --amend -m "updated message"` (rewrite message based on all accumulated changes)
   - For new commit: Use `git commit -m "message"` (descriptive summary format, no prefixes, no Claude boilerplate)
   - Show final `git status` to confirm

## Examples

**Checkpoint conversion workflow:**
```bash
# After using /core-checkpoint for rapid iteration...
/core-commit
# 🔄 Found CHECKPOINT commit with accumulated changes
# Analyzes all changes, suggests descriptive commit with bullet points
```

**Normal interactive commit:**
```bash
/core-commit
# Shows analysis, suggests message, waits for approval
```

## Multi-Impact Commit Strategy

**When CHECKPOINT contains multiple major change types:**

### Impact Classification Priority
1. **Data Correctness** - Fixes that resolve incorrect data/calculations
2. **Performance** - Optimizations that improve user experience 
3. **Functionality** - New features or bug fixes
4. **Development Infrastructure** - Workflow, tooling, architecture

### Natural Commit Message Format
**Title**: Descriptive summary of primary changes (no prefixes)
```
Fix FST timescale calculation, optimize performance, and implement agent architecture
```

**Body**: List changes in impact priority order with bullet points
- Lead with most user-critical fixes
- Follow with performance improvements  
- Include functionality changes
- End with development/infrastructure additions

### Example Multi-Impact Analysis
```
🎯 MULTI-IMPACT COMMIT DETECTED

📊 Impact Classification:
- Data Correctness: FST timescale calculation fix (CRITICAL)
- Performance: Binary search + decimation (HIGH) 
- Infrastructure: Agent architecture system (MEDIUM)

💭 Natural commit message:
"Fix FST timescale calculation, optimize performance, and implement agent architecture

- Fix incorrect timescale calculations in FST file parsing
- Implement binary search optimization for signal lookup  
- Add decimation for smooth rendering performance
- Create agent architecture system for task delegation
- Update build configuration for new agent workflow"
```

### Why This Approach
- **Git blame clarity**: Readers immediately understand all major work done
- **Avoid artificial splitting**: Prevents redundant commits that hurt tooling
- **Impact transparency**: Prioritizes by actual project value, not categories

## Features

- **Comprehensive CHECKPOINT analysis**: Thoroughly analyzes ALL accumulated changes, not just unstaged files
- **Smart feature detection**: Distinguishes major implementations from minor config changes
- **File categorization**: Groups changes by type and impact for accurate commit message generation
- **Multi-impact commit strategy**: Handles complex commits spanning multiple areas with impact-based prioritization
- **Smart amend detection**: Analyzes if current changes should amend previous commit
- **Safety checks**: Warns about rewriting published history
- **Scope analysis**: Compares change types between current and last commit
- **Natural commit format**: Uses descriptive summaries with bullet point details instead of artificial prefixes
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