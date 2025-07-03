# Development Workflow

## WASM/Frontend Development Process

**CRITICAL WORKFLOW:**
- Run `makers start > dev_server.log 2>&1 &` as BACKGROUND PROCESS with output logging
- Monitor compilation with `tail -f dev_server.log` or read file content periodically
- This allows Claude to see compilation errors immediately without managing terminal processes
- Auto-reload ONLY triggers after successful compilation
- Always test changes with browser MCP after making changes to verify compilation succeeded
- Read compilation errors from the running command output, don't restart it repeatedly
- NEVER use `cargo build` or `cargo check` - they cannot check WASM compilation properly (IDE has same issue)
- Only read compilation errors from mzoon output for accurate WASM build status
- **NEVER check browser until compilation succeeds** - auto-reload only happens after successful compilation
- **Kill dev server properly:** `pkill -f "makers start" && pkill -f mzoon` or find PIDs with `ps aux | grep -E "(mzoon|makers)" | grep -v grep` then `kill <PIDs>`

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