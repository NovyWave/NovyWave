# Development Workflow

## WASM/Frontend Development Process

**CRITICAL WORKFLOW:**
- **ABSOLUTE PROHIBITION: NEVER restart MoonZoon dev server without explicit user permission** - only restart when MoonZoon.toml changes
- **MANDATORY: ALWAYS ask user to use /project-stop or /project-start commands** instead of killing/starting processes directly
- **PATIENCE REQUIREMENT: Backend/shared crate compilation takes DOZENS OF SECONDS TO MINUTES - this is normal, DO NOT restart**
- **WAIT ENFORCEMENT: You MUST wait for compilation to complete, no matter how long it takes**
- **COMPILATION MONITORING ONLY:** Monitor with `tail -f dev_server.log` or read file content periodically - DO NOT manage processes
- This allows Claude to see compilation errors immediately without managing terminal processes
- **Clear dev_server.log when it gets too long** - use `> dev_server.log` to truncate for better readability and token efficiency
- Log file is only for reading recent compilation state, not historical data
- Auto-reload ONLY triggers after successful compilation
- **BROWSER TESTING PROTOCOL:** Always test changes with browser MCP after making changes to verify compilation succeeded
- **READ ERRORS, DON'T RESTART:** Read compilation errors from the running command output, don't restart it repeatedly
- **CARGO PROHIBITION:** NEVER use `cargo build` or `cargo check` - they cannot check WASM compilation properly (IDE has same issue)
- **MZOON OUTPUT ONLY:** Only read compilation errors from mzoon output for accurate WASM build status
- **BROWSER ACCESS RESTRICTION:** NEVER check browser until compilation succeeds - auto-reload only happens after successful compilation
- **COMPILATION TIME REALITY:** Accept that Rust compilation takes significant time, especially for backend/shared crate changes

## Debug Patterns

- Use `zoon::println!()` for console logging, NOT `std::println!()` (which does nothing in WASM)
- All frontend code compiles to WebAssembly and runs in browser environment
- For development verification, use the three built-in examples: Simple Rectangle, Face with Hat, and Sine Wave

## Advanced UI Debugging Techniques

**Auto-Scroll Testing for Width Issues:**
- Create `Task::start + Timer::sleep + viewport_x_signal + i32::MAX` to reveal horizontal layout problems
- Essential for debugging TreeView, table, and scrollable content width constraints
- Allows testing width behavior that's invisible in normal view

**Multi-Subagent Problem Solving:**
- Fire 3+ specialized subagents simultaneously for complex UI issues
- Pattern: (1) Browser DOM/CSS inspection agent (2) Minimal test case creation agent (3) Comprehensive solution research agent
- Each agent provides focused expertise while main session coordinates and implements
- Use TodoWrite for systematic task breakdown and progress tracking

**Width Constraint Debugging:**
- Common issue: TreeView/component backgrounds don't extend to full content width in scrollable containers
- Root cause: Multiple levels of width constraints (container → item → CSS)
- Solution pattern: Container needs `Width::fill() + CSS min-width: max-content` + Items need `Width::fill()` + CSS needs `width: 100%`

## Testing and Quality

- No automated test suite is currently configured
- Manual testing is done through visual examples in the UI
- Always run lint and typecheck commands after completing tasks if available

## Session Documentation

**PATTERN STORAGE:**
Store important discoveries using `/core-remember-important` before ending sessions:
- Bug fixes and compilation solutions
- New UI patterns and component usage examples  
- Architectural decisions and implementation choices
- Framework-specific patterns (Zoon, NovyUI, Fast2D)
- Responsive design solutions and layout fixes
- Feature implementation notes

**DOCUMENTATION DIVISION:**
- **Session Notes:** Temporary discoveries, debugging solutions, implementation examples
- **CLAUDE.md:** General project rules, architecture decisions, permanent guidelines, framework-wide patterns
- **Static Docs:** Technical solutions, project architecture, bug fixes reference, development workflows