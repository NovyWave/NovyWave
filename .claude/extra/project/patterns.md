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

- MoonZoon pinned to git revision `7c5178d891cf4afbc2bbbe864ca63588b6c10f2a`
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

> **📖 Complete API Reference:** See `docs/actors_relays/moonzoon/api.md` for full API specification with all methods and the critical "Cache Current Values" pattern.

### MANDATORY State Management Rules

**NO RAW MUTABLES:** All state must use Actor+Relay or Atom architecture.

**See CLAUDE.md for complete Actor+Relay vs raw Mutables examples.**

### Event-Source Relay Naming (MANDATORY)

**CRITICAL RULE: Relay names must identify the source of events, not the destination or purpose**

**Pattern:** `{specific_source}_{event}_relay`

**✅ CORRECT - Source identification:**
```rust
// User interactions - identify WHO initiated the event
theme_button_clicked_relay: Relay,           // Theme button clicked
dock_button_clicked_relay: Relay,            // Dock mode button clicked  
file_drop_area_dropped_relay: Relay<Vec<PathBuf>>, // Files dropped on drop area
search_input_changed_relay: Relay<String>,   // Search input text changed
variable_tree_item_clicked_relay: Relay<String>, // Tree item clicked

// System events - identify WHAT system component triggered
file_parser_completed_relay: Relay<ParseResult>, // File parser finished
network_error_occurred_relay: Relay<String>,     // Network component errored
config_loader_finished_relay: Relay<Config>,     // Config loader completed

// UI component events - identify WHICH component
main_dialog_opened_relay: Relay,             // Main dialog opened
timeline_panel_resized_relay: Relay<(f32, f32)>, // Timeline panel resized
```

**❌ PROHIBITED - Vague or command-like naming:**
```rust
// ❌ Generic "request" - doesn't identify source
theme_toggle_requested_relay: Relay,         // WHO requested? Button? Keyboard? API?
file_add_requested_relay: Relay<PathBuf>,    // WHERE did the request come from?

// ❌ Command-like naming
add_file: Relay<PathBuf>,                    // Sounds like imperative command
remove_item: Relay<String>,                  // Action-oriented
set_theme: Relay<Theme>,                     // Imperative verb
update_config: Relay<Config>,                // Generic command
```

**Key Principle: Single Source, Multiple Subscribers**
- **One source location** can send events through a relay (e.g., only theme button sends theme_button_clicked events)
- **Multiple subscribers** can listen to the same relay (UI updates, config saving, logging, etc.)
- **Source identification** in the name makes debugging and code navigation easy

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

### Critical Pattern: Cache Current Values

**⚠️ MANDATORY: This is the ONLY acceptable place to cache values in Actor+Relay architecture**

The "Cache Current Values" pattern is used **EXCLUSIVELY inside Actor processing loops** to maintain current state values for use when responding to events.

```rust
// ✅ CORRECT: Cache values ONLY inside Actor loops for event response
let actor = ActorVec::new(vec![], async move |state| {
    // Cache current values as they flow through streams
    let mut current_filter = String::new();
    let mut selected_items = Vec::new();
    
    loop {
        select! {
            // Update cached values when they change
            Some(filter) = filter_stream.next() => {
                current_filter = filter;
            }
            Some(items) = selection_stream.next() => {
                selected_items = items;
            }
            // Use cached values when responding to events
            Some(()) = action_button_stream.next() => {
                process_selection(&current_filter, &selected_items);
            }
        }
    }
});
```

**Key Rules:**
- **ONLY cache inside Actor loops** - never in UI components or globally
- **Use caching for event response** - when you need multiple values at once
- **Otherwise use signals** - for all other state access
- **Never use raw Mutables for caching** - defeats the architecture

See [docs/actors_relays/moonzoon/api.md#critical-pattern-cache-current-values](../../docs/actors_relays/moonzoon/api.md#critical-pattern-cache-current-values) for detailed examples.

### CRITICAL ANTIPATTERN: State Access Outside Actor Loops

**❌ NEVER DO: get() + set() pattern**
```rust
// ANTIPATTERN 1: Race condition prone - value can change between get() and set()
let current_theme = state.get();  // Read
let new_theme = toggle_theme(current_theme);  // Compute
state.set(new_theme);  // Write - but state may have changed!

// ANTIPATTERN 2: Unnecessary cloning for every access
fn toggle_theme() {
    let theme = THEME_STATE.get();  // Always clones entire value
    let new_theme = match theme {
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::Light,
    };
    THEME_STATE.set(new_theme);  // Another clone + signal emission
}
```

**❌ EVEN WORSE: Internal caching to "optimize" get/set**
```rust
// ANTIPATTERN 3: Shadowing actor state with manual cache
let mut cached_theme = Theme::Light;  // Manual cache outside Actor
Task::start(THEME_STATE.signal().for_each_sync(move |theme| {
    cached_theme = theme;  // Trying to "optimize" by caching
}));

// Now you have TWO sources of truth - recipe for bugs
```

**❌ ESCAPE HATCH ANTIPATTERNS: Breaking Actor+Relay Architecture**
```rust
// ANTIPATTERN 4: UI caching current values (violates Actor+Relay principles)
pub fn current_theme_now() -> SharedTheme {
    app_config().theme_actor.get()  // ❌ UI should never cache state
}

pub fn toggle_theme() {
    let current = current_theme_now();  // ❌ Business logic in UI layer
    let new_theme = match current {     // ❌ Race condition risk
        SharedTheme::Light => SharedTheme::Dark,
        SharedTheme::Dark => SharedTheme::Light,
    };
    app_config().theme_changed_relay.send(new_theme);
}

// ANTIPATTERN 5: "Convenience" functions that bypass architecture
pub fn get_cursor_position() -> TimeNs {
    TIMELINE_STATE.get()  // ❌ Direct state access outside Actor
}

pub fn current_variables() -> Vec<SelectedVariable> {
    SELECTED_VARIABLES.get()  // ❌ Breaks reactive flow
}
```

**CRITICAL RULE: NO ESCAPE HATCHES**
- **NEVER create `current_X_now()` functions** - These break Actor+Relay architecture
- **NEVER cache state outside Actor loops** - Only Actors can safely cache current values  
- **NEVER use `.get()` methods in UI code** - Use signals exclusively
- **NO convenience getters** - Every state access must be through proper signal chains

**✅ CORRECT: Direct lock_mut() manipulation**
```rust
// ✅ ATOMIC: Single lock covers read+modify+write operation
{
    let mut theme = state.lock_mut();
    let old_theme = *theme;  // Read current value
    *theme = match *theme {  // Atomic modify
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::Light,
    };
    // Lock automatically dropped, signals fire atomically
}
```

**Why lock_mut() is correct:**
- **Atomic operations** - No race conditions between read and write
- **No cloning** - Direct mutation of the actual state
- **Single source of truth** - No shadow caches to get out of sync  
- **Automatic signaling** - Changes immediately trigger reactive updates
- **Lock scope control** - Explicit lifetime management with block scope

**When this pattern applies:**
- Toggle operations (theme, dock mode, checkboxes)
- Increment/decrement counters  
- State machines with transitions
- Any read-modify-write sequence that should be atomic

### UI Toggle Event Pattern (Successful Implementation)

**CRITICAL: Proper separation of UI events from business logic**

Based on successful Theme and Dock mode button implementation, this pattern prevents the UI Business Logic antipattern:

#### ✅ Correct Implementation

**1. Add Toggle Request Relays:**
```rust
pub struct AppConfig {
    // Direct value changes (from config loading)
    pub theme_changed_relay: Relay<SharedTheme>,
    pub dock_mode_changed_relay: Relay<DockMode>,
    
    // Toggle requests (from UI buttons)
    pub theme_toggle_requested_relay: Relay,
    pub dock_mode_toggle_requested_relay: Relay,
}
```

**2. UI Functions Only Emit Events:**
```rust
/// Request theme toggle - UI just emits event, Actor handles business logic
pub fn toggle_theme_requested() {
    app_config().theme_toggle_requested_relay.send(());
}

/// Request dock mode toggle - UI just emits event, Actor handles business logic  
pub fn toggle_dock_mode_requested() {
    app_config().dock_mode_toggle_requested_relay.send(());
}
```

**3. Actors Handle Both Direct Changes and Toggle Logic:**
```rust
let theme_actor = Actor::new(SharedTheme::Light, async move |state| {
    let mut theme_stream = theme_changed_stream; // No .fuse() needed - already implements FusedStream
    let mut theme_toggle_stream = theme_toggle_requested_stream;
    
    // ✅ Cache current values as they flow through streams
    let mut current_theme = SharedTheme::Light;
    
    loop {
        select! {
            new_theme = theme_stream.next() => {
                if let Some(new_theme) = new_theme {
                    current_theme = new_theme.clone();  // Update cache
                    state.set_neq(new_theme);
                }
            }
            toggle_request = theme_toggle_stream.next() => {
                if let Some(()) = toggle_request {
                    // ✅ Business logic inside Actor with cached state
                    let new_theme = match current_theme {
                        SharedTheme::Light => SharedTheme::Dark,
                        SharedTheme::Dark => SharedTheme::Light,
                    };
                    current_theme = new_theme.clone();  // Update cache
                    state.set_neq(new_theme);  // ✅ No race conditions
                }
            }
        }
    }
});
```

**4. ConfigSaver Integration:**
ConfigSaver automatically detects Actor signal changes and performs debounced saves:
```
UI Event → Actor Toggle Logic → ConfigSaver Detection → Debounced Save
```

#### Key Implementation Details

**ConfigSaver Lifetime Management:**
```rust
// ✅ Proper storage using Arc<TaskHandle> and struct field
#[derive(Debug, Clone)]
struct ConfigSaver {
    _task_handle: Arc<TaskHandle>,  // Arc allows proper Clone
}

pub struct AppConfig {
    // Keep ConfigSaver alive in struct (not _config_saver = dropped immediately)
    config_saver: ConfigSaver,
}
```

**Benefits of This Pattern:**
- ✅ UI only emits events (no business logic or state caching)
- ✅ Race condition-free toggle logic inside Actor
- ✅ Automatic persistence through ConfigSaver signal watching  
- ✅ Clean separation of concerns
- ✅ Proper Arc-based lifetime management (no global statics)

**Common Pitfalls Avoided:**
- ❌ `let _config_saver = ...` (underscore prefix causes immediate drop)
- ❌ UI caching current values outside Actors (`current_theme_now()`)
- ❌ Business logic in UI functions (toggle decisions) 
- ❌ Global statics for lifetime management
- ❌ Manual save triggers (ConfigSaver handles automatically)

### Automatic Config Saving with ConfigSaver Actor

**CRITICAL ARCHITECTURAL PATTERN: Signal-based automatic persistence**

The ConfigSaver pattern eliminates manual save triggers by watching all config signals automatically:

#### Core Architecture Principles

**1. Default Relay Type Parameter:**
```rust
// ✅ Now possible: Clean syntax for common case
pub struct AppConfig {
    pub config_load_requested_relay: Relay,  // Instead of Relay<()>
    pub theme_changed_relay: Relay<SharedTheme>,
}

// Implementation:
#[derive(Clone, Debug)]
pub struct Relay<T = ()>  // T defaults to () for clean syntax
```

**2. Signal vs Event Usage Pattern:**
- **Relays for UI/user events** - Button clicks, file drops, user interactions
- **Signals for internal state watching** - ConfigSaver monitors config changes automatically

#### ConfigSaver Implementation

**Automatic Persistence Actor:**
```rust
/// Watches all config signals and automatically saves with debouncing
struct ConfigSaver {
    _task_handle: TaskHandle,
}

impl ConfigSaver {
    pub fn new(
        theme_actor: Actor<SharedTheme>,
        dock_mode_actor: Actor<DockMode>,
        // ... all other config actors
    ) -> Self {
        let task_handle = Task::start_droppable(async move {
            let mut debounce_task: Option<TaskHandle> = None;
            
            // Listen to ALL config actor signals
            let mut theme_stream = theme_actor.signal().to_stream(); // Signal streams may need .fuse()
            let mut dock_stream = dock_mode_actor.signal().to_stream(); // depending on implementation
            // ... other streams
            
            loop {
                select! {
                    _ = theme_stream.next() => {
                        Self::schedule_debounced_save(&mut debounce_task).await;
                    }
                    _ = dock_stream.next() => {
                        Self::schedule_debounced_save(&mut debounce_task).await;
                    }
                    // ... other config changes
                }
            }
        });
        
        Self { _task_handle: task_handle }
    }
    
    async fn schedule_debounced_save(debounce_task: &mut Option<TaskHandle>) {
        *debounce_task = None;  // Cancel previous save
        
        let handle = Task::start_droppable(async {
            Timer::sleep(1000).await;  // 1-second debounce
            save_config_to_backend().await;
        });
        
        *debounce_task = Some(handle);
    }
}
```

#### Before vs After Architecture

**❌ OLD: Manual coupling everywhere**
```rust
// UI code had to manually trigger saves
theme_button.on_click(|| {
    theme_changed_relay.send(new_theme);
    config_save_requested_relay.send(());  // Manual coupling!
});

dock_button.on_click(|| {
    dock_mode_changed_relay.send(new_mode);
    config_save_requested_relay.send(());  // Repeated everywhere!
});
```

**✅ NEW: Automatic decoupled persistence**
```rust  
// UI code only changes what it needs
theme_button.on_click(|| {
    theme_changed_relay.send(new_theme);  // ConfigSaver automatically saves
});

dock_button.on_click(|| {
    dock_mode_changed_relay.send(new_mode);  // ConfigSaver automatically saves  
});
```

#### Key Benefits

1. **Eliminated Coupling**: No `config_save_requested()` calls scattered throughout UI code
2. **Automatic Persistence**: ANY config change triggers save - no manual triggers needed
3. **Debounced Efficiency**: 1-second debounce prevents excessive backend calls during rapid changes
4. **Signal-Based Architecture**: Follows principle "Relays for events, signals for state watching"
5. **Clean Syntax**: `Relay` instead of `Relay<()>` for common unit type case

This pattern provides automatic, efficient, and decoupled configuration persistence while maintaining clean Actor+Relay architecture principles.

### CRITICAL Bi-directional UI State Antipattern

**❌ ANTIPATTERN: Local State Copies Breaking Bi-directional Updates**

From successful expanded scopes persistence fix:

```rust
// ❌ BROKEN: Creates local copy, breaks bi-directional updates
let expanded_scopes_mutable = zoon::Mutable::new(expanded_scopes.clone());
tree_view().external_expanded(expanded_scopes_mutable)  // Updates local copy only!
```

**✅ CORRECT: Use global state directly for bi-directional updates**
```rust
tree_view().external_expanded(crate::state::EXPANDED_SCOPES.clone())  // Updates global state!
```

**Key Lesson:** When UI components need to both READ and WRITE state (bi-directional), they must reference the actual state, not copies. Creating local `Mutable` copies breaks the write-back mechanism.

### State Persistence & Local UI Patterns

**State Persistence:**
1. **Domain Events** through event-source relays for user interactions
2. **Actor State** managed within domain Actors  
3. **Config Integration** with ConfigSaver monitoring domain signals automatically
4. **UI Integration** connecting components to domain signals, not global state

**Local UI State:**
Use Atom for simple UI state like panel dimensions, dialog visibility, search filters, hover effects, and other UI-only concerns. See verified examples in development.md.

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

## MoonZoon Framework Configuration

### Framework Overview
MoonZoon is a Rust-based full-stack web framework using:
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

#### Automatic String to Text Conversion

**CONVENIENT: Strings automatically convert to Text elements**
```rust
// ✅ CLEAN: Direct string usage
El::new().child("✕")                    // Becomes Text::new("✕") 
El::new().child("Drop files here")       // Becomes Text::new("Drop files here")
Column::new().item("Simple text")        // Becomes Text::new("Simple text")

// ❌ VERBOSE: Manual Text wrapping (unnecessary)
El::new().child(Text::new("✕"))
El::new().child(Text::new("Drop files here"))
Column::new().item(Text::new("Simple text"))
```

**Key Rule:** Any method expecting `impl IntoElement` automatically treats strings as `Text` elements, making code cleaner and more readable.

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