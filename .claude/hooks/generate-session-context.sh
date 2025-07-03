#!/bin/bash

# Auto-generate session context from Memory MCP
CONTEXT_FILE="./novywave/ai-docs/session-context.md"

cat > "$CONTEXT_FILE" << 'EOF'
# Auto-Generated Session Context

*This file is automatically updated by Claude Code hooks.*

## Recent Key Patterns

**NovyUI Component Usage:**
- ALL icons use `IconName` enum tokens for compile-time safety
- Button API: `button().label().variant().left_icon(IconName::X).build()`
- Responsive: `Width::fill()` required, `Font::new().no_wrap()` prevents wrapping

**WASM Development:**
- Use `zoon::println!()` for logging, NOT `std::println!()`
- NEVER use `cargo build` or `cargo check` - only mzoon handles WASM properly
- Auto-reload only after successful compilation - check mzoon.log first

**Memory Storage Triggers:**
- Store immediately: solved bugs, new patterns, architectural decisions
- Entity limit: 3-5 observations maximum per entity
- Use `/project:store-pattern "description"` after discoveries

**Current Architecture:**
- Dual-platform: Web + Tauri desktop using shared Rust/WASM frontend
- Framework stack: Zoon frontend, Moon backend, Fast2D graphics, NovyUI components
- Target: Professional waveform viewer with 4-panel layout

**Height Layout Pattern:**
- Root: `Height::screen()` to claim viewport
- Containers: `Height::fill()` to inherit properly
- Missing `Height::fill()` breaks the inheritance chain

## Recent Solutions

**IconName Compilation Fix:**
- Fixed by adding `mut self` to `build()` method
- Use `self.align.take()` to extract value without cloning

**Responsive Layout:**
- Use `Width::fill()` instead of fixed widths
- Apply `Font::new().no_wrap()` to prevent text wrapping
- Three-zone headers: title, spacer with centered content, right buttons

EOF

echo "✅ Session context updated: $(date)"