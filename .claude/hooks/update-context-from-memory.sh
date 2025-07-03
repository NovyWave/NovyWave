#!/bin/bash

# Update context whenever Memory MCP is used
CONTEXT_FILE="./novywave/ai-docs/session-context.md"

# Extract timestamp
TIMESTAMP=$(date)

# Update the context file with fresh timestamp and key patterns
cat > "$CONTEXT_FILE" << EOF
# Auto-Generated Session Context

*Last updated: $TIMESTAMP*

## Essential Patterns (Auto-Updated)

**NovyUI Design System:**
- ALL icons use \`IconName\` enum tokens for compile-time safety
- Button API: \`button().label().variant().left_icon(IconName::X).build()\`
- Responsive: \`Width::fill()\` required, \`Font::new().no_wrap()\` prevents wrapping
- Three-zone headers: title, spacer with centered content, right buttons

**WASM Development Rules:**
- Use \`zoon::println!()\` for logging, NOT \`std::println!()\`
- NEVER use \`cargo build\` or \`cargo check\` - only mzoon handles WASM properly
- Auto-reload only after successful compilation - check mzoon.log first
- Monitor compilation: \`makers start > dev_server.log 2>&1 &\`

**Memory Management:**
- MANDATORY: Store immediately after solving bugs or discovering patterns
- Entity limit: 3-5 observations maximum per entity
- Use \`/project:store-pattern "description"\` for quick storage
- Memory MCP vs CLAUDE.md: persistent patterns vs core rules

**Current Architecture:**
- Dual-platform: Web + Tauri desktop using shared Rust/WASM frontend
- Framework stack: Zoon frontend, Moon backend, Fast2D graphics, NovyUI components
- Target: Professional waveform viewer with 4-panel layout replacing Fast2D examples
- Entry point: frontend/src/main.rs with font loading and app initialization

**Zoon Layout Patterns:**
- Height inheritance: Root uses \`Height::screen()\`, containers use \`Height::fill()\`
- Missing \`Height::fill()\` in any container breaks the height inheritance chain
- Responsive width: Always use \`Width::fill()\` instead of fixed widths
- Debug technique: Use bright background colors to visualize height inheritance

## Recent Solutions

**IconName Compilation Fix:**
- Problem: \`E0382\` (partial move) → \`E0277\` (Copy trait) → \`E0596\` (mutable borrow)
- Solution: Use \`self.align.take()\` with \`mut self\` parameter to extract value without cloning

**Responsive Layout:**
- Always use \`Width::fill()\` instead of fixed widths to prevent horizontal overflow
- Apply \`Font::new().no_wrap()\` to all text elements to prevent line breaks
- Waveform rectangles: Use 12 responsive rectangles, not 20 fixed-width ones

**Button Component Evolution:**
- Fixed inconsistent icon API: Button now uses \`IconName\` tokens like Input component
- \`.left_icon(IconName::Folder)\` and \`.right_icon(IconName::X)\` patterns
- Internal conversion: IconName → string via \`.to_kebab_case()\` method
- Added \`.align()\` method for clean centering without wrapper elements

EOF

echo "🔄 Context synced after Memory MCP usage: $TIMESTAMP" >> /tmp/claude-hooks.log