# NovyWave Project Configuration & Framework Patterns

## Project Overview

NovyWave - Professional waveform viewer for digital design verification.

> **🎯 QUICK START:** Reference `domain_map.md` first to identify affected domains.

**Dual-Platform:** Web (Rust/WASM + Zoon) + Tauri desktop

**Structure:** `frontend/` (WASM), `backend/` (MoonZoon), `src-tauri/` (desktop), `shared/` (types), `novyui/` (components)

**Commands:**
- `makers start` - Dev server at localhost:8080
- `makers build` - Production build
- `makers tauri` - Desktop dev mode

## Timeline Visual Elements (CRITICAL)

- **BLUE LINE (Zoom Center)**: Position 0 default, follows mouse hover. Z/R resets to 0. **Zoom centers here.**
- **YELLOW LINE (Cursor)**: Viewport center default. Click jumps, Q/E moves, Shift+Q/E jumps transitions. R resets.

**Key Rule:** Zoom center (blue) ≠ Timeline cursor (yellow) - independent systems.

## Key Dependencies

- MoonZoon: git rev `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`
- Fast2D graphics, NovyUI with IconName tokens

## Shared Crate

Import from `shared/` - never duplicate types:
```rust
use shared::{LoadingFile, WaveformFile, UpMsg, DownMsg, AppConfig};
```

## Actor+Relay Architecture

> **📖 Complete Reference:** See `actor-relay-patterns.md` for all patterns.

**Quick Rules:**
- NO raw Mutables - use Actor+Relay or Atom
- Event-source naming: `button_clicked_relay` not `add_file`
- Domain-driven: `TrackedFiles` not `FileManager`

## Zoon Framework Patterns

### Layout Fundamentals
```rust
El::new().s(Height::screen())           // Root claims viewport
Column::new().s(Height::fill())         // All containers inherit
Row::new().s(Width::fill())             // Responsive width
.s(Gap::new().x(8)).s(Align::center_y()) // Spacing/alignment
```

**Critical:** Missing `Height::fill()` breaks height inheritance chain.

### Signal-Based Layouts
```rust
Stripe::new()
    .direction_signal(mode.map(|m| if m.docked() { Column } else { Row }))
    .item_signal(content.map(|c| match c { A => a().into_element(), B => b() }))
```

Use `.into_element()` for type unification in match arms.

### Global Keyboard Handlers
```rust
.global_event_handler(move |event: KeyDown| {
    if DIALOG_OPEN.get() && event.key() == "Escape" { close_dialog(); }
})
```

## NovyUI Patterns

- ALL icons: `IconName` enum tokens (`button().left_icon(IconName::Folder)`)
- Strings auto-convert: `El::new().child("text")` → `Text::new("text")`
- Header 3-zone: `Row::new().item(title).item(El::s(Width::fill())).item(button)`

### TreeView Width
```rust
// Container-first pattern (recommended)
.update_raw_el(|el| el.style("min-width", "fit-content").style("width", "100%"))
```

## Async Element Functions

```rust
.child_signal(async_element().into_signal_option())

async fn async_element() -> impl Element {
    let value = signal.to_stream().next().await;
    El::new().child(format!("{value}"))
}
```
