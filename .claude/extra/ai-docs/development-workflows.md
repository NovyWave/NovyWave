# Development Workflows

## Testing and Verification Protocols

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
mcp__browsermcp__browser_screenshot  # Document state before/after
```

### Example Honest Responses
- "I cannot verify the fix works because compilation is failing"
- "Browser shows the dialog is still not centered - the fix didn't work"
- "I see scrollbar errors in the console - the styling isn't applying"

### Three-Stage Testing Approach
1. **Compilation Verification**: Ensure code builds without errors
2. **Visual Verification**: Use browser MCP to test UI changes
3. **Functional Verification**: Test actual behavior matches requirements

## Development Server Patterns

### Server Management Rules
- **ABSOLUTE PROHIBITION: NEVER restart dev server without explicit user permission**
- **MANDATORY: ALWAYS ask user to use `makers kill` or `makers start` commands**
- Backend/shared crate compilation takes DOZENS OF SECONDS TO MINUTES - this is normal
- **WAIT ENFORCEMENT: Must wait for compilation to complete, no matter how long**

### Development Server Commands
```bash
# Start development server (background process)
makers start > dev_server.log 2>&1 &

# Monitor compilation
tail -f dev_server.log

# Clean log when it gets too long (token efficiency)
> dev_server.log

# Development server commands (preferred)
makers kill      # Stop development server
makers start     # Start development server
makers open      # Start and open browser
```

### Log Monitoring Patterns
```bash
# Check for compilation errors
tail -100 dev_server.log | grep -i "error"

# Monitor for successful compilation
tail -f dev_server.log | grep -i "compilation complete"

# Debug patterns
rg "println!" --type rust  # Find debug statements to clean up
```

## Browser Integration Workflows

### Browser MCP Testing Sequence
```rust
// 1. Navigate to application
mcp__browsermcp__browser_navigate("http://localhost:8080")

// 2. Take screenshot for documentation
mcp__browsermcp__browser_screenshot()

// 3. Interact with elements
mcp__browsermcp__browser_click("Load Files button", "button_ref")

// 4. Verify results
mcp__browsermcp__browser_screenshot()  // After state
```

### Multi-Subagent Testing Strategy
Fire 3+ specialized subagents simultaneously for complex issues:
1. **Browser DOM/CSS inspection agent** - Analyze actual DOM state
2. **Minimal test case creation agent** - Create isolated reproduction
3. **Comprehensive solution research agent** - Find proven solutions

### Auto-Scroll Testing for Width Issues
```rust
// Reveal horizontal layout problems invisible in normal view
Task::start(async move {
    for position in [0, 200, 400, 600, i32::MAX] {
        VIEWPORT_X.set_neq(position);
        Timer::sleep(1000).await;
    }
});
```

## Task Management Approaches

### TodoWrite Usage Protocol
- **MANDATORY TODO USAGE** for ALL multi-step tasks (3+ steps)
- Update todo status in real-time as you work
- Mark todos completed immediately after finishing each task
- Never batch multiple completions

### Systematic Problem-Solving Process
1. **Acknowledge & Analyze**: Never defend poor results, use TodoWrite to break down issues
2. **Systematic Subagent Research**: Use Task tool subagents to analyze each issue separately
3. **Methodical Implementation**: Apply fixes systematically, one issue at a time
4. **Comprehensive Testing**: Use browser MCP to verify changes visually
5. **Results Verification & Honesty**: Test each fix individually

### Example Response Pattern for Poor Results
```
You're absolutely right - 1/5 is not acceptable. Let me use subagents to systematically analyze and fix each issue:

[Creates detailed todos for each problem]
[Uses Task tool subagents to analyze each issue separately]  
[Applies fixes methodically]
[Verifies all fixes work properly]
```

## Git Workflows

### Two-Stage Checkpoint Workflow
1. **CHECKPOINT** - Rapid iteration saves during development
2. **COMMIT** - Clean conventional commit messages for history

### Checkpoint Command
```bash
/core-checkpoint  # Creates rapid development checkpoint
```

### Commit Command  
```bash
/core-commit  # Creates clean conventional commit
```

### Multi-Line Commit Format
```
fix(ui): resolve panel resize issues in docked-to-bottom mode
fix(config): preserve dock mode settings during workspace saves
refactor(frontend): modularize main.rs into focused modules
```

### Git Safety Rules
- **CRITICAL: NEVER perform destructive git operations without explicit user confirmation**
- Never use git commands with `-i` flag (interactive not supported)
- DO NOT push to remote repository unless explicitly asked
- Only exceptions: `/core-checkpoint` and `/core-commit` commands

## Session Discovery Storage

### Important Discoveries Storage
Use `/core-remember-important` before ending sessions to store:
```bash
/core-remember-important  # Store session discoveries to .claude/session-notes.md
```

### Storage Triggers
Use `/core-remember-important` when you:
- Solve any bug or compilation error
- Create new UI patterns or component examples
- Make architectural decisions
- Discover framework-specific patterns
- Fix responsive design issues

### Storage Commands
```bash
/core-note "Fixed compilation by adding mut self"
/core-memory-search "IconName"
/core-remember-important  # Store important session discoveries
```

## Subagent Delegation Strategy

### MANDATORY: Use Task tool subagents extensively to preserve main session context

### Delegate to Subagents
- File analysis & research (instead of main session reading multiple files)
- Implementation tasks (code changes, testing, debugging)
- Investigation work (finding patterns, analyzing codebases)
- Complex searches across many files

### Implementor Agent Requirements
**CRITICAL: Implementor agents MUST:**
- Check dev_server.log after making changes
- Report compilation errors AND warnings found
- Never claim "compilation successful" without verification
- Use `tail -50 dev_server.log | grep -E "error|Error|warning|Warning|Failed|Frontend built"` to verify
- Fix ALL errors before returning control to main session
- Report any warnings that remain after fixes

### Main Session Focus
- High-level coordination & planning
- User interaction & decision making
- Architecture decisions & task delegation
- Synthesis of subagent results
- **MANDATORY: Run verifier agent after each implementor agent completes**

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

## Quality Assurance Patterns

### Debug Cleanup Methodology
1. Use Task tool subagents for parallel analysis
2. Categorize warnings: definitely fixable vs maybe fixable vs keep as-is
3. Remove dead code first
4. Add TODO comments + `#[allow]` for future features
5. Challenge clone variables - test compilation without them
6. Achieve 100% warning cleanup for production-ready codebase

### Performance Testing Protocol
```bash
# Check compilation time
time makers build

# Monitor memory usage
ps aux | grep mzoon

# Test with large datasets
# Load test files with 5000+ variables
```

### Error Handling Verification
- Always use `error_display::add_error_alert()` for ALL error handling
- Never duplicate logging
- Test error states with invalid inputs
- Verify graceful degradation

### CSS/Styling Verification
```rust
// Debug technique: Use bright colors to visualize layout
.s(Background::new().color(Color::red()))  // Debug only

// Remove debug colors before committing
```