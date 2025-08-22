# Development Practices & Workflows

## Following Conventions

When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

- NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
- When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
- When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
- Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.

## Code Style

- IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked

## Refactoring Rules

**ATOMIC CODE MOVEMENT - NEVER BREAK COMPILATION:**
1. Copy complete code blocks to destination files first
2. Verify compilation succeeds after each copy
3. Only then remove code from source files
4. NEVER create placeholder functions or empty stubs
5. NEVER rename types with aliases (e.g., `Signal as DataSignal`) - move code directly
6. Always preserve exact functionality during moves

## Mandatory Clarification Protocol

**CRITICAL: Always ask clarifying questions before starting complex tasks.**

### When to Ask Questions:
- Multi-step UI/UX tasks requiring specific styling or behavior
- Tasks with ambiguous requirements or multiple interpretation paths
- Complex technical implementations where assumptions could lead to rework
- Integration work where exact behavior needs specification

### Question Types to Ask:
- **Specificity**: "Should the entire file extension part '(*.vcd, *.fst)' be less contrasting, or just the asterisks?"
- **Context**: "Do you want the dialog to take full screen height, or screen height with padding?"
- **Scope**: "Should scrollbar styling match Files & Scope panels specifically?"
- **Verification**: "Can you clarify what you mean by 'background doesn't cover content'?"

### Example Good Clarification:
```
I have a couple of clarifying questions before I start implementing:

1. **Window title styling**: For "Select Waveform Files (*.vcd, *.fst)" - should the entire file extension part "(*.vcd, *.fst)" be less contrasting, or just the asterisks and dots?

2. **Full screen height**: When you say "fill parent height (effectively fill screen)" - do you want the dialog to take up the full viewport height with some margin, or literally edge-to-edge?

3. **Background coverage**: Can you clarify what you mean by "Dialog background does not cover entire content"?

Once I understand these details clearly, I'll implement all the improvements efficiently.
```

**Never assume - always clarify ambiguous requirements upfront.**

## Development Server Management

### Server Management Rules
- **ABSOLUTE PROHIBITION: NEVER restart dev server without explicit user permission**
- **MANDATORY: ALWAYS ask user to use `makers kill` or `makers start` commands**
- Backend/shared crate compilation takes DOZENS OF SECONDS TO MINUTES - this is normal
- **WAIT ENFORCEMENT: Must wait for compilation to complete, no matter how long**

### Log Monitoring Patterns
```bash
# Check for compilation errors
tail -100 dev_server.log | grep -i "error"

# Monitor for successful compilation
tail -f dev_server.log | grep -i "compilation complete"

# Debug patterns
rg "println!" --type rust  # Find debug statements to clean up
```

## Testing & Verification Protocols

### CRITICAL VERIFICATION REQUIREMENTS
- **NEVER claim success without actual verification**
- **ALWAYS use browser MCP for visual verification of UI changes**
- **ALWAYS check compilation logs for errors before testing**
- If you CANNOT verify a fix (compilation fails, browser unreachable, etc.) - **TELL THE USER IMMEDIATELY**
- Never claim "it works" or "it's fixed" without actual testing

### UI Testing Protocol
```bash
# 1. Check compilation status first
tail -f dev_server.log

# 2. Verify frontend compilation succeeds
# Look for "Frontend compilation complete" or similar

# 3. Use browser MCP to test changes
mcp__browsermcp__browser_navigate "http://localhost:8080"
mcp__browsermcp__browser_screenshot  # Full page or element-specific screenshots
```

**Screenshot Options:**
- Full page: `mcp__browsermcp__browser_screenshot` (captures entire page)
- Element-specific: Can target specific elements using CSS selectors to save tokens and focus on relevant UI areas

### Example Honest Responses
- "I cannot verify the fix works because compilation is failing"
- "Browser shows the dialog is still not centered - the fix didn't work"
- "I see scrollbar errors in the console - the styling isn't applying"

### Three-Stage Testing Approach
1. **Compilation Verification**: Ensure code builds without errors
2. **Visual Verification**: Use browser MCP to test UI changes
3. **Functional Verification**: Test actual behavior matches requirements

## Task Management

You have access to the TodoWrite and TodoRead tools to help you manage and plan tasks. Use these tools VERY frequently to ensure that you are tracking your tasks and giving the user visibility into your progress.

### MANDATORY TODO USAGE
- Create detailed todos for ALL multi-step tasks (3+ steps)
- Update todo status in real-time as you work
- Use specific, actionable todo descriptions
- Mark todos completed immediately after finishing each task
- Never batch multiple completions

These tools are also EXTREMELY helpful for planning tasks, and for breaking down larger complex tasks into smaller steps. If you do not use this tool when planning, you may forget to do important tasks - and that is unacceptable.

It is critical that you mark todos as completed as soon as you are done with a task. Do not batch up multiple tasks before marking them as completed.

### Systematic Problem-Solving Process
1. **Acknowledge & Analyze**: Never defend poor results, use TodoWrite to break down issues
2. **Systematic Subagent Research**: Use Task tool subagents to analyze each issue separately
3. **Methodical Implementation**: Apply fixes systematically, one issue at a time
4. **Comprehensive Testing**: Use browser MCP to verify changes visually
5. **Results Verification & Honesty**: Test each fix individually

### Example Response Pattern for Poor Results
```
1/5 is not acceptable. Let me use subagents to systematically analyze and fix each issue:

[Creates detailed todos for each problem]
[Uses Task tool subagents to analyze each issue separately]  
[Applies fixes methodically]
[Verifies all fixes work properly]
```

## Git Workflows


### Git Safety Rules
- **CRITICAL: NEVER perform destructive git operations (reset, rebase, force push, branch deletion, stash drop) without explicit user confirmation**
- **User lost hours of work from uncommitted changes - always confirm before any operation that could lose data**
- Never use git commands with `-i` flag (interactive not supported)
- DO NOT push to remote repository unless explicitly asked
- **Only exceptions: `/core-checkpoint` and `/core-commit` commands where destruction is part of expected flow, but still be careful**


## Subagent Delegation Strategy

### Strategic Subagent Usage
**Use Task tool subagents selectively** to preserve main session context while extending effective session length.

### Delegate to Subagents
- File analysis & research (instead of main session reading multiple files)
- Implementation tasks (code changes, testing, debugging)
- Investigation work (finding patterns, analyzing codebases)
- Complex searches across many files

### Implementor Agent Requirements
**CRITICAL: Implementor agents MUST:**
- Check dev_server.log after making changes (MANDATORY verification protocol)
- Report compilation errors AND warnings found
- Never claim "compilation successful" without verification
- Use `tail -100 dev_server.log | grep -E "error\[E|warning:|Failed|panic|Frontend built"` to verify
- Fix ALL errors before returning control to main session
- Report any warnings that remain after fixes
- **NEVER run `makers build`, `makers start`, or any compilation commands** - dev server auto-compiles
- **NEVER use browser MCP tools** - that's exclusively for Validator agents
- **ONLY make code changes and read logs** - no testing, no browser access

### Validator Agent Requirements
**CRITICAL: Validator agents are responsible for:**
- 4-phase validation: Compilation → Visual → Functional → Console
- Checking dev_server.log for compilation status
- Using browser MCP tools for visual verification
- Testing functionality after Implementor changes
- Screenshot documentation of UI states
- Reporting comprehensive validation results
- **ONLY Validator agents can use browser MCP tools**
- **NEVER make code changes** - only validate and test
- **AUTOMATIC activation** after Implementor agents complete

### Implementor-Validator Collaboration Pattern
**MANDATORY WORKFLOW:**
1. **Implementor Agent**: Makes code changes, checks dev_server.log for compilation
2. **Main Session**: MUST run Validator agent immediately after Implementor completes
3. **Validator Agent**: Performs 4-phase validation including browser testing
4. **Main Session**: Decides next action based on Validator results (✅ PASS, ⚠️ WARN, ❌ FAIL)

### Main Session Focus
- High-level coordination & planning
- User interaction & decision making
- Architecture decisions & task delegation
- Synthesis of subagent results
- **MANDATORY: Run Validator agent after each Implementor agent completes**
- **Orchestrate Implementor → Validator workflow for all changes**

### Context Conservation Benefits
- Subagents use their own context space, not main session's
- Main session gets condensed summaries instead of raw file contents
- Can parallelize multiple research/implementation tasks
- Dramatically extends effective session length (2-3x longer)

### Self-Reminder Checklist
Before using Read/Glob/Grep tools, ask: "Could a subagent research this instead?"
- If reading 2+ files → delegate to Task tool
- If searching for patterns → delegate to Task tool
- If analyzing codebase structure → delegate to Task tool
- Exception: Single specific files (configs, CLAUDE.md)

## Quality Assurance & Best Practices

### Debug Cleanup Methodology
1. Use Task tool subagents for parallel analysis
2. Categorize warnings: definitely fixable vs maybe fixable vs keep as-is
3. Remove dead code first
4. Add TODO comments + `#[allow]` for future features
5. Challenge clone variables - test compilation without them
6. Achieve 100% warning cleanup for production-ready codebase

### Error Handling Verification
- Always use `error_display::add_error_alert()` for ALL error handling
- Never duplicate logging
- Test error states with invalid inputs
- Verify graceful degradation

### Important Development Reminders
- Do what has been asked; nothing more, nothing less
- NEVER create files unless they're absolutely necessary
- ALWAYS prefer editing an existing file to creating a new one
- NEVER proactively create documentation files (*.md) or README files unless explicitly requested

### Planning Guidelines
- Use the Task tool when you are in plan mode
- Only use exit_plan_mode tool when planning implementation steps for code writing tasks
- For research tasks (gathering information, searching, reading), do NOT use exit_plan_mode

