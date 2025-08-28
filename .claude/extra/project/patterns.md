# NovyWave Project Configuration & Framework Patterns

## Project Overview

NovyWave - Professional waveform viewer for digital design verification and analysis.

### Architecture

**Dual-Platform:** Web application + Tauri desktop using shared Rust/WASM frontend

**Framework Stack:**
- **Frontend:** Rust + WASM using Zoon framework 
- **Backend:** Moon framework (browser mode only)
- **Desktop:** Tauri v2 wrapper
- **Graphics:** Fast2D rendering library
- **State Management:** Actor+Relay architecture (mandatory)

**Project Structure:**
```
frontend/     - Rust/WASM frontend (shared)
backend/      - MoonZoon backend (browser only)
src-tauri/    - Tauri desktop wrapper
shared/       - Shared types and utilities between frontend/backend
novyui/       - Custom UI component library
public/       - Static assets
```

### Key Dependencies

- MoonZone pinned to git revision `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`
- Fast2D graphics from NovyWave/Fast2D
- NovyUI component library with IconName tokens

### Development Commands

**Browser Mode (default):**
- `makers start` - Start development server with auto-reload at http://localhost:8080
- `makers build` - Production build for browser deployment

**Desktop Mode (Tauri):**
- `makers tauri` - Start Tauri desktop development mode
- `makers tauri-build` - Build production desktop application

**Utilities:**
- `makers install` - Install all dependencies (MoonZoon CLI, Rust WASM target, etc.)
- `makers clean` - Clean all build artifacts

### NovyWave-Specific Rules

**Component Usage:**
- ALL icons use `IconName` enum tokens, never strings
- Use `Width::fill()` for responsive layouts, never fixed widths
- Apply `Font::new().no_wrap()` to prevent text wrapping

**Domain Focus:**
- Professional waveform visualization
- Digital design verification workflows
- High-performance graphics rendering
- Desktop and web dual deployment

### Shared Crate Usage

The `shared/` crate contains types and utilities that need to be used by both frontend and backend:

**Core Types:**
- `LoadingFile`, `LoadingStatus` - File loading state management
- `WaveformFile`, `ScopeData`, `Signal` - Waveform data structures
- `UpMsg`, `DownMsg` - Communication messages between frontend/backend
- `AppConfig` and related config types - Application configuration

**When to Use:**
- Any type that needs to be serialized/deserialized between frontend and backend
- Data structures representing waveform files and their contents
- Configuration types that are saved/loaded from disk
- Message types for frontend-backend communication

**Import Pattern:**
```rust
use shared::{LoadingFile, LoadingStatus, WaveformFile, Signal};
```

**Do NOT duplicate types:** Always import from `shared` rather than defining duplicate types in frontend or backend.

## Actor+Relay Architecture Patterns

### MANDATORY State Management Rules

**NO RAW MUTABLES:** All state must use Actor+Relay or Atom architecture.

**❌ PROHIBITED:**
```rust
// Global mutables
static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
static DIALOG_OPEN: Lazy<Mutable<bool>> = lazy::default();

// Local mutables in components
let loading_state = Mutable::new(false);
```

**✅ REQUIRED:**
```rust
// Domain-driven Actors
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
}

// Atom for local UI
let dialog_open = Atom::new(false);
```

### Event-Source Relay Naming (MANDATORY)

**Pattern:** `{source}_{event}_relay`

**✅ CORRECT:**
```rust
// User interactions
button_clicked_relay: Relay,
input_changed_relay: Relay<String>,
file_dropped_relay: Relay<Vec<PathBuf>>,
menu_selected_relay: Relay<MenuOption>,

// System events
file_loaded_relay: Relay<PathBuf>,
parse_completed_relay: Relay<ParseResult>,
error_occurred_relay: Relay<String>,
timeout_reached_relay: Relay,

// UI events
dialog_opened_relay: Relay,
panel_resized_relay: Relay<(f32, f32)>,
scroll_changed_relay: Relay<f32>,
```

**❌ PROHIBITED:**
```rust
add_file: Relay<PathBuf>,       // Command-like
remove_item: Relay<String>,     // Imperative  
set_theme: Relay<Theme>,        // Action-oriented
update_config: Relay<Config>,   // Command pattern
```

### Domain-Driven Design Patterns

**✅ REQUIRED: Model domain concepts directly**
```rust
struct TrackedFiles {              // Collection of tracked files
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<Vec<PathBuf>>,
    file_selected_relay: Relay<PathBuf>,
    parse_completed_relay: Relay<(String, ParseResult)>,
}

struct WaveformTimeline {          // The timeline itself
    cursor_position: Actor<f64>,
    visible_range: Actor<(f64, f64)>,
    cursor_moved_relay: Relay<f64>,
    zoom_changed_relay: Relay<f32>,
}

struct SelectedVariables {         // Currently selected variables
    variables: ActorVec<Variable>,
    variable_clicked_relay: Relay<String>,
    selection_cleared_relay: Relay,
    scope_expanded_relay: Relay<String>,
}
```

**❌ PROHIBITED: Enterprise abstractions**
```rust
struct FileManager { ... }        // Artificial "manager" layer
struct TimelineService { ... }    // Unnecessary "service" abstraction
struct DataController { ... }     // Vague "controller" pattern
struct ConfigHandler { ... }      // Generic "handler" pattern
```

### NovyWave Domain Patterns

**File Management Domain:**
```rust
use crate::reactive_actors::{Actor, ActorVec, Relay, relay};

struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    
    // User interactions
    file_dropped_relay: Relay<Vec<PathBuf>>,        // Files dropped on UI
    file_selected_relay: Relay<PathBuf>,            // User clicked file
    reload_requested_relay: Relay<String>,          // User clicked reload
    
    // System events
    parse_completed_relay: Relay<(String, ParseResult)>,  // Parser finished
    error_occurred_relay: Relay<(String, String)>,        // Parse error
}

impl TrackedFiles {
    fn new() -> Self {
        let (file_dropped_relay, file_dropped_stream) = relay();
        let (parse_completed_relay, parse_completed_stream) = relay();
        // ... other relays
        
        let files = ActorVec::new(vec![], async move |files_vec| {
            loop {
                select! {
                    Some(paths) = file_dropped_stream.next() => {
                        for path in paths {
                            let tracked_file = TrackedFile::new(path);
                            files_vec.lock_mut().push_cloned(tracked_file);
                        }
                    }
                    Some((file_id, result)) = parse_completed_stream.next() => {
                        // Update specific file with parse result
                    }
                }
            }
        });
        
        TrackedFiles {
            files,
            file_dropped_relay,
            parse_completed_relay,
            // ... other fields
        }
    }
}
```

**Variable Selection Domain:**
```rust
struct SelectedVariables {
    variables: ActorVec<SelectedVariable>,
    
    // User selection events
    variable_clicked_relay: Relay<String>,          // Variable clicked in tree
    variable_removed_relay: Relay<String>,          // Remove button clicked
    scope_expanded_relay: Relay<String>,            // Scope chevron clicked
    clear_selection_clicked_relay: Relay,           // Clear all clicked
    
    // System events
    selection_restored_relay: Relay<Vec<String>>,   // Config loaded
    filter_applied_relay: Relay<String>,            // Search filter
}
```

**Timeline Domain:**
```rust
struct WaveformTimeline {
    // State
    cursor_position: Actor<f64>,              // Nanoseconds
    visible_range: Actor<(f64, f64)>,        // (start_ns, end_ns)
    zoom_level: Actor<f32>,                  // Zoom factor
    
    // User interactions
    cursor_clicked_relay: Relay<f64>,         // User clicked timeline
    mouse_moved_relay: Relay<(f32, f32)>,    // Mouse over canvas
    zoom_changed_relay: Relay<f32>,          // Zoom wheel
    pan_started_relay: Relay<(f32, f32)>,    // Drag started
    
    // Keyboard events
    left_key_pressed_relay: Relay,           // Arrow navigation
    right_key_pressed_relay: Relay,
    zoom_in_pressed_relay: Relay,            // Keyboard zoom
    zoom_out_pressed_relay: Relay,
}
```

### Atom for Local UI Patterns

**Replace all local Mutables with Atom:**
```rust
// Panel component
struct PanelState {
    width: Atom<f32>,
    height: Atom<f32>,
    is_collapsed: Atom<bool>,
    is_hovered: Atom<bool>,
}

// Dialog component
struct FileDialogState {
    is_open: Atom<bool>,
    filter_text: Atom<String>,
    selected_files: Atom<Vec<PathBuf>>,
    current_directory: Atom<PathBuf>,
    error_message: Atom<Option<String>>,
}

// Search component
struct SearchState {
    filter_text: Atom<String>,
    is_focused: Atom<bool>,
    match_count: Atom<usize>,
    selected_index: Atom<Option<usize>>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            filter_text: Atom::new(String::new()),
            is_focused: Atom::new(false),
            match_count: Atom::new(0),
            selected_index: Atom::new(None),
        }
    }
}
```

### UI Integration Patterns

**Event Emission in UI Components:**
```rust
// File drop area
fn file_drop_zone(tracked_files: &TrackedFiles) -> impl Element {
    El::new()
        .s(Background::new().color(neutral_3()))
        .on_drop({
            let file_dropped_relay = tracked_files.file_dropped_relay.clone();
            move |dropped_files| {
                file_dropped_relay.send(dropped_files);
            }
        })
        .child(Text::new("Drop waveform files here"))
}

// Variable tree item
fn variable_item(
    variable: &Variable, 
    selected_variables: &SelectedVariables
) -> impl Element {
    Row::new()
        .s(Padding::new().x(8).y(4))
        .on_click({
            let variable_clicked_relay = selected_variables.variable_clicked_relay.clone();
            let var_id = variable.unique_id.clone();
            move || variable_clicked_relay.send(var_id.clone())
        })
        .item(Text::new(&variable.name))
        .item_signal(
            selected_variables.variables.signal_vec_cloned()
                .map(move |vars| {
                    if vars.iter().any(|v| v.unique_id == variable.unique_id) {
                        IconName::Check.into()
                    } else {
                        IconName::X.into()
                    }
                })
        )
}
```

### Module Structure

**Frontend Module Organization (to be created during migration):**
```rust
// frontend/src/actors/mod.rs (or similar module structure)
pub mod relay;              // Relay<T> implementation
pub mod actor;              // Actor<T> implementation  
pub mod actor_vec;          // ActorVec<T> implementation
pub mod actor_map;          // ActorMap<K,V> implementation
pub mod atom;               // Atom<T> implementation

// Re-exports for easy importing
pub use relay::Relay;
pub use actor::Actor;
pub use actor_vec::ActorVec;
pub use actor_map::ActorMap;
pub use atom::Atom;

// Core function for creating relays
pub fn relay<T>() -> (Relay<T>, impl Stream<Item = T>) {
    let relay = Relay::new();
    let stream = relay.subscribe();
    (relay, stream)
}
```

**Usage in Components:**
```rust
use crate::actors::{Actor, ActorVec, Relay, Atom, relay};

// Domain struct using Actor+Relay
struct AppState {
    tracked_files: TrackedFiles,
    selected_variables: SelectedVariables,
    waveform_timeline: WaveformTimeline,
    
    // Local UI state
    search_state: SearchState,
    dialog_state: DialogState,
}
```

## MoonZone Framework Configuration

### Framework Overview
MoonZone is a Rust-based full-stack web framework using:
- **Frontend:** Rust + WASM using Zoon UI framework
- **Backend:** Moon server framework (optional)
- **Build Tool:** mzoon CLI

### Critical WASM Rules
**Compilation:**
- Use `zoon::println!()` for logging, NOT `std::println!()`
- NEVER use `cargo build` or `cargo check` - only mzoon handles WASM properly
- Auto-reload only triggers after successful compilation

**Development Workflow:**
- Run `makers start > dev_server.log 2>&1 &` as BACKGROUND PROCESS
- Monitor compilation with `tail -f dev_server.log`
- Read compilation errors from mzoon output for accurate WASM build status
- Browser auto-reloads ONLY after successful compilation

## Zoon Framework Patterns

### Layout Fundamentals

#### Height Inheritance Pattern
```rust
// Root element claims viewport
El::new().s(Height::screen())

// All containers must inherit
Column::new().s(Height::fill())
Row::new().s(Height::fill())
```

**Critical:** Missing `Height::fill()` in any container breaks the height inheritance chain.

#### Responsive Width
```rust
// Good - responsive
Row::new().s(Width::fill())

// Bad - fixed width causes overflow
Row::new().s(Width::exact(800))
```

#### Spacing and Alignment
```rust
// Vertical centering in headers
.s(Align::new().center_y())

// Horizontal spacing
.s(Gap::new().x(8))  // normal spacing
.s(Gap::new().x(1))  // tight spacing

// Spacer element
El::new().s(Width::fill())
```

### Signal-Based Layouts

#### Dynamic Layout Switching
```rust
// Use Stripe for Row/Column switching
Stripe::new()
    .direction_signal(layout_signal.map(|mode| {
        if mode.is_docked() { Direction::Column } else { Direction::Row }
    }))
    .item_signal(content_signal.map(|content| {
        match content {
            ContentType::A => element_a().into_element(),
            ContentType::B => element_b().into_element(),
        }
    }))
```

**Important:** Use `.into_element()` to unify types when using if/else branches in signals.

### Common Layout Patterns

#### Full-Screen Layout
```rust
fn root() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .child(main_layout())
}
```

#### Panel with Header
```rust
fn panel_with_header(title: &str, content: impl Element) -> impl Element {
    Column::new()
        .s(Height::fill())
        .item(header_row(title))
        .item(content)
}
```

### Event Handling Patterns

#### Global Keyboard Event Handler
For dialog keyboard accessibility without focus management:
```rust
.global_event_handler({
    let close_dialog = close_dialog.clone();
    move |event: KeyDown| {
        if DIALOG_IS_OPEN.get() {  // Guard with dialog state
            if event.key() == "Escape" {
                close_dialog();
            } else if event.key() == "Enter" {
                process_selection();
            }
        }
    }
})
```

**Why Global Event Handlers:**
- Work immediately when dialog opens (no focus required)
- Capture events at document level
- Better than autofocus or local element handlers for modal dialogs
- Essential for keyboard accessibility in Zoon applications

**Pattern:** Always guard global handlers with state checks to prevent interference with other UI components.

### Debug Techniques
Use bright background colors on containers to visualize height inheritance:
```rust
.s(Background::new().color(Color::red()))  // Debug only
```

## NovyUI Component Patterns

### Icon Design Tokens
- ALL components use `IconName` enum tokens, never magic strings
- Button: `button().left_icon(IconName::Folder)` 
- Input: `input().left_icon(IconName::Search)`
- Available icons: Check, X, Folder, Search, ArrowDownToLine, ZoomIn, ZoomOut, etc.
- Adding new icons requires: enum entry, to_kebab_case() mapping, SVG file mapping, string parsing
- Icon registry provides compile-time safety and IDE autocompletion

### Component Usage Patterns

#### Button Component
```rust
button()
    .label("Click me")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Medium)
    .left_icon(IconName::Folder)
    .align(Align::center())
    .on_press(|| { /* handler */ })
    .build()
```

#### Input Component
```rust
input()
    .placeholder("Search...")
    .left_icon(IconName::Search)
    .size(InputSize::Small)
    .build()
```

### Layout Patterns

#### Header Layout (3-zone)
```rust
Row::new()
    .s(Gap::new().x(8))
    .s(Align::new().center_y())
    .item(title_text)
    .item(El::new().s(Width::fill()))  // spacer
    .item(right_button)
```

#### Centered Button in Header
```rust
Row::new()
    .item(title)
    .item(El::new().s(Width::fill()).s(Align::center()).child(button))
    .item(right_button)
```

### Responsive Design Rules
- Use `Width::fill()` for responsive elements instead of `Width::exact()`
- Apply `Font::new().no_wrap()` to prevent text wrapping
- Use `Height::screen()` on root + `Height::fill()` on containers for full-screen layouts
- Gap sizing: `.x(1)` for tight spacing, `.x(8)` for normal spacing

### TreeView Component Patterns

#### TreeView Background Width Patterns
**Problem:** TreeView item backgrounds don't extend to full content width in scrollable containers

**Solution A - Content-First (fit-content + min-width):**
```rust
.update_raw_el(|raw_el| {
    raw_el
        .style("width", "fit-content")
        .style("min-width", "100%")
})
```

**Solution B - Container-First (RECOMMENDED):**
```rust
.update_raw_el(|raw_el| {
    raw_el
        .style("min-width", "fit-content")
        .style("width", "100%")
})
```

**Recommendation:** Use Solution B (container-first) for TreeView items:
- Primary behavior: Fill panel width (better UX)
- Exception behavior: Expand for wide content (enables horizontal scroll)
- Semantic clarity: "always fill panel, expand only when content demands it"

#### Scrollable Container Requirements
For panels containing TreeView with horizontal overflow:

```rust
// Panel scrollable container
El::new()
    .s(Scrollbars::both())
    .s(Width::fill())
    .update_raw_el(|raw_el| {
        raw_el.style("min-height", "0")      // Allow flex shrinking
              .style("overflow-x", "auto")   // Enable horizontal scroll
    })
```

**Key Insight:** Zoon `Width::fill()` + CSS `min-width: max-content` allows content to extend beyond container boundaries while maintaining responsive behavior.

## Development & Debugging

### WASM/Frontend Development Process

**CRITICAL WORKFLOW:**
- **ABSOLUTE PROHIBITION: NEVER restart MoonZoon dev server without explicit user permission**
- **MANDATORY: ALWAYS ask user to use `makers kill` or `makers start` commands**
- **PATIENCE REQUIREMENT: Backend/shared crate compilation takes DOZENS OF SECONDS TO MINUTES**
- **WAIT ENFORCEMENT: You MUST wait for compilation to complete, no matter how long it takes**
- **COMPILATION MONITORING ONLY:** Monitor with `tail -f dev_server.log` - DO NOT manage processes
- **Clear dev_server.log when it gets too long** - use `> dev_server.log` to truncate
- **BROWSER TESTING PROTOCOL:** Always test changes with browser MCP after making changes
- **READ ERRORS, DON'T RESTART:** Read compilation errors, don't restart repeatedly
- **CARGO PROHIBITION:** NEVER use `cargo build` or `cargo check` - they cannot check WASM compilation
- **MZOON OUTPUT ONLY:** Only trust mzoon output for accurate WASM build status

### Debug Patterns
- Use `zoon::println!()` for console logging, NOT `std::println!()` (which does nothing in WASM)
- All frontend code compiles to WebAssembly and runs in browser environment
- For development verification, use the three built-in examples: Simple Rectangle, Face with Hat, and Sine Wave

### Advanced UI Debugging Techniques

**Auto-Scroll Testing for Width Issues:**
- Create `Task::start + Timer::sleep + viewport_x_signal + i32::MAX` to reveal horizontal layout problems
- Essential for debugging TreeView, table, and scrollable content width constraints
- Allows testing width behavior that's invisible in normal view

**Width Constraint Debugging:**
- Common issue: TreeView/component backgrounds don't extend to full content width in scrollable containers
- Root cause: Multiple levels of width constraints (container → item → CSS)
- Solution pattern: Container needs `Width::fill() + CSS min-width: max-content` + Items need `Width::fill()` + CSS needs `width: 100%`