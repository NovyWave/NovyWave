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

## Testing and Quality

- No automated test suite is currently configured
- Manual testing is done through visual examples in the UI
- Always run lint and typecheck commands after completing tasks if available

## Memory MCP Workflow

**SESSION START PATTERN:**
Always begin with memory search: `mcp__memory__search_nodes` for relevant context

**IMMEDIATE STORAGE TRIGGERS:**
Store in Memory MCP immediately when you:
- Solve any bug or compilation error (store both problem and solution)
- Create new UI patterns or component usage examples  
- Make architectural decisions or choose between alternatives
- Discover framework-specific patterns (Zoon, NovyUI, Fast2D)
- Fix responsive design issues or layout problems
- Implement new features or modify existing ones

**STORAGE FORMAT:**
Use atomic observations connected to existing entities (NovyWave project, framework entities, developer preferences)

**Memory vs CLAUDE.md Division:**
- **Memory MCP:** Component patterns, debugging solutions, library examples, temporary workarounds, session discoveries
- **CLAUDE.md:** General project rules, architecture decisions, permanent guidelines, framework-wide patterns