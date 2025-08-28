# NovyWave Actor+Relay Migration Strategy

## Executive Summary

This document outlines the comprehensive migration strategy for NovyWave's transition from 74+ global Mutables to Actor+Relay architecture, emphasizing **event-source relay naming** and **domain-driven design** without enterprise patterns.

## Core Principles

### 1. Event-Source Relay Naming (MANDATORY)

**✅ CORRECT: Describe the event source/trigger**
```rust
// User interactions - what the user DID
button_clicked_relay: Relay,
input_changed_relay: Relay<String>,
checkbox_toggled_relay: Relay<bool>,
file_dropped_relay: Relay<Vec<PathBuf>>,
menu_selected_relay: Relay<MenuOption>,

// System events - what HAPPENED
file_loaded_relay: Relay<PathBuf>,
parse_completed_relay: Relay<ParseResult>,
error_occurred_relay: Relay<String>,
data_received_relay: Relay<ApiResponse>,
timeout_reached_relay: Relay,

// UI events - what the interface DID
dialog_opened_relay: Relay,
panel_resized_relay: Relay<(f32, f32)>,
tab_switched_relay: Relay<TabId>,
scroll_changed_relay: Relay<f32>,
```

**❌ WRONG: Command-like/imperative naming**
```rust
add_file: Relay<PathBuf>,           // Sounds like a command to execute
remove_item: Relay<String>,         // Imperative style
clear_all: Relay,                   // Action-oriented
set_theme: Relay<Theme>,            // Command pattern
update_config: Relay<Config>,       // Imperative verb
```

**Pattern Template:** `{source}_{event}_relay`
- **source**: Who/what generated the event (button, input, file, network, timer)
- **event**: What happened (clicked, changed, loaded, failed, completed)
- **_relay**: Consistent suffix for all event relays

### 2. Domain-Driven Architecture (NO Manager/Service Patterns)

**✅ CORRECT: Direct domain modeling**
```rust
// Models what it IS, not what it manages
struct TrackedFiles {              // Collection of tracked files
    files: ActorVec<TrackedFile>,
    file_dropped_relay: Relay<PathBuf>,
}

struct WaveformTimeline {          // The timeline itself  
    cursor_position: Actor<f64>,
    cursor_moved_relay: Relay<f64>,
}

struct SelectedVariables {         // Currently selected variables
    variables: ActorVec<Variable>,
    variable_clicked_relay: Relay<String>,
}

struct UserConfiguration {        // User's configuration
    theme: Actor<Theme>,
    theme_changed_relay: Relay<Theme>,
}
```

**❌ WRONG: Enterprise abstraction patterns**
```rust
struct FileManager { ... }        // Artificial "manager" layer
struct TimelineService { ... }    // Unnecessary "service" abstraction  
struct DataController { ... }     // Vague "controller" pattern
struct ConfigHandler { ... }      // Generic "handler" pattern
struct VariableProcessor { ... }  // Abstract "processor" concept
```

**Key Rule:** If you can't easily explain what the struct *IS* in domain terms, it's probably an artificial abstraction.

## Migration Scope Analysis

### Current State: 74+ Global Static Mutables

**File Management (13 mutables)**
```rust
// Current problematic patterns
static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
static FILE_PATHS: Lazy<MutableVec<String>> = lazy::default();
static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();
// ... 9 more file-related mutables
```

**Timeline & Canvas (25+ mutables)**
```rust
// Performance-critical state with many race conditions
static TIMELINE_CURSOR_NS: Lazy<Mutable<f64>> = lazy::default();
static CANVAS_WIDTH: Lazy<Mutable<f32>> = lazy::default();
static MOUSE_X_POSITION: Lazy<Mutable<f32>> = lazy::default();
// ... 22+ more timeline/canvas mutables
```

**Variable Selection (8 mutables)**
```rust
// Complex reactive interdependencies
static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = lazy::default();
static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = lazy::default();
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = lazy::default();
// ... 5 more variable-related mutables
```

**UI Layout & State (23 mutables)**
```rust
// Panel dimensions, dialog states, search filters
static FILES_PANEL_WIDTH: Lazy<Mutable<f32>> = lazy::default();
static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();
// ... 20 more UI-related mutables
```

**Configuration & Services (5+ mutables)**
```rust
// Configuration persistence and service coordination
static CONFIG_LOADED: Lazy<Mutable<bool>> = lazy::default();
static ACTIVE_REQUESTS: Lazy<Mutable<HashMap<String, bool>>> = lazy::default();
// ... 3+ more service mutables
```

## Phased Migration Strategy

### Phase 1: Core Domain Structures (Week 1)

#### 1.1 TrackedFiles Domain (13 mutables → 1 domain struct)

**Target Architecture:**
```rust
/// Central collection of waveform files being analyzed
/// Replaces 13 global mutables with clean domain representation
#[derive(Clone)]
struct TrackedFiles {
    /// Core collection of tracked files
    files: ActorVec<TrackedFile>,
    
    /// Event-source based relays (user interactions)
    file_dropped_relay: Relay<Vec<PathBuf>>,        // Files dropped on UI
    file_selected_relay: Relay<PathBuf>,            // User clicked file
    reload_requested_relay: Relay<String>,          // User clicked reload
    remove_requested_relay: Relay<String>,          // User clicked remove
    
    /// Event-source based relays (system events) 
    parse_completed_relay: Relay<(String, ParseResult)>,  // Parser finished
    parse_failed_relay: Relay<(String, String)>,          // Parser failed
    directory_scanned_relay: Relay<Vec<PathBuf>>,          // Directory scan done
}

impl TrackedFiles {
    fn new() -> Self {
        let (file_dropped_relay, file_dropped_stream) = relay();
        let (file_selected_relay, file_selected_stream) = relay();
        let (reload_requested_relay, reload_stream) = relay();
        let (remove_requested_relay, remove_stream) = relay();
        let (parse_completed_relay, parse_completed_stream) = relay();
        let (parse_failed_relay, parse_failed_stream) = relay();
        let (directory_scanned_relay, directory_stream) = relay();
        
        let files = ActorVec::new(vec![], async move |files_vec| {
            loop {
                select! {
                    Some(paths) = file_dropped_stream.next() => {
                        for path in paths {
                            let file_id = generate_file_id(&path);
                            let tracked_file = TrackedFile::new(file_id, path);
                            files_vec.lock_mut().push_cloned(tracked_file);
                        }
                    }
                    Some(file_id) = remove_stream.next() => {
                        files_vec.lock_mut().retain(|f| f.id != file_id);
                    }
                    Some(file_id) = reload_stream.next() => {
                        // Find and reload specific file
                        if let Some(file) = files_vec.lock_ref().iter().find(|f| f.id == file_id) {
                            file.reload_requested_relay.send(());
                        }
                    }
                    Some((file_id, result)) = parse_completed_stream.next() => {
                        if let Some(file) = files_vec.lock_ref().iter().find(|f| f.id == file_id) {
                            file.parse_completed_relay.send(result);
                        }
                    }
                    // ... handle other events
                }
            }
        });
        
        TrackedFiles {
            files,
            file_dropped_relay,
            file_selected_relay, 
            reload_requested_relay,
            remove_requested_relay,
            parse_completed_relay,
            parse_failed_relay,
            directory_scanned_relay,
        }
    }
}

/// Individual tracked file with its own lifecycle
#[derive(Clone)]
struct TrackedFile {
    pub id: String,
    pub path: PathBuf,
    pub state: Actor<FileState>,
    
    /// File-specific events
    pub reload_requested_relay: Relay,
    pub parse_completed_relay: Relay<ParseResult>,
    pub state_changed_relay: Relay<FileState>,
}

#[derive(Clone, Debug)]
enum FileState {
    Loading,
    Parsed { 
        signals: Vec<WaveformSignal>,
        time_range: (f64, f64),
    },
    Error(String),
}
```

**Migration Pattern:**
```rust
// Before: Multiple global mutations across codebase
TRACKED_FILES.lock_mut().push_cloned(file);      // views.rs:333
LOADING_FILES.lock_mut().retain(|f| ...);        // state.rs:214  
FILE_PATHS.lock_mut().set_cloned(i, path);       // config.rs:84

// After: Event emission with clear source
tracked_files.file_dropped_relay.send(vec![path]);   // File dialog result
tracked_files.remove_requested_relay.send(file_id);  // Remove button click
tracked_files.reload_requested_relay.send(file_id);  // Reload button click
```

#### 1.2 SelectedVariables Domain (8 mutables → 1 domain struct)

**Target Architecture:**
```rust
/// Variables currently selected for timeline display  
/// Replaces SELECTED_VARIABLES + related selection mutables
#[derive(Clone)]
struct SelectedVariables {
    /// Core collection of selected variables
    variables: ActorVec<SelectedVariable>,
    
    /// Selection events (user interactions)
    variable_clicked_relay: Relay<String>,          // User clicked variable in tree
    variable_removed_relay: Relay<String>,          // User clicked remove button  
    scope_expanded_relay: Relay<String>,            // User expanded scope
    scope_collapsed_relay: Relay<String>,           // User collapsed scope
    clear_selection_clicked_relay: Relay,           // User clicked clear all
    
    /// Selection events (system events)
    selection_restored_relay: Relay<Vec<String>>,   // Config loaded selection
    filter_applied_relay: Relay<String>,            // Search filter applied
}

impl SelectedVariables {
    fn new() -> Self {
        let (variable_clicked_relay, variable_clicked_stream) = relay();
        let (variable_removed_relay, variable_removed_stream) = relay();
        let (scope_expanded_relay, scope_expanded_stream) = relay();
        let (scope_collapsed_relay, scope_collapsed_stream) = relay();
        let (clear_selection_clicked_relay, clear_stream) = relay();
        let (selection_restored_relay, restoration_stream) = relay();
        let (filter_applied_relay, filter_stream) = relay();
        
        let variables = ActorVec::new(vec![], async move |vars_vec| {
            let mut expanded_scopes = HashSet::new();
            
            loop {
                select! {
                    Some(var_id) = variable_clicked_stream.next() => {
                        // Add variable if not already selected
                        let already_selected = vars_vec.lock_ref().iter()
                            .any(|v| v.unique_id == var_id);
                        if !already_selected {
                            if let Some(variable) = find_variable_by_id(&var_id) {
                                vars_vec.lock_mut().push_cloned(variable);
                            }
                        }
                    }
                    Some(var_id) = variable_removed_stream.next() => {
                        vars_vec.lock_mut().retain(|v| v.unique_id != var_id);
                    }
                    Some(scope_id) = scope_expanded_stream.next() => {
                        expanded_scopes.insert(scope_id);
                    }
                    Some(scope_id) = scope_collapsed_stream.next() => {
                        expanded_scopes.remove(&scope_id);
                    }
                    Some(()) = clear_stream.next() => {
                        vars_vec.lock_mut().clear();
                    }
                    Some(var_ids) = restoration_stream.next() => {
                        // Restore selection from config
                        let variables: Vec<_> = var_ids.into_iter()
                            .filter_map(|id| find_variable_by_id(&id))
                            .collect();
                        vars_vec.lock_mut().replace_cloned(variables);
                    }
                }
            }
        });
        
        SelectedVariables {
            variables,
            variable_clicked_relay,
            variable_removed_relay,
            scope_expanded_relay,
            scope_collapsed_relay, 
            clear_selection_clicked_relay,
            selection_restored_relay,
            filter_applied_relay,
        }
    }
}
```

### Phase 2: Performance-Critical Systems (Week 2)

#### 2.1 WaveformTimeline Domain (25+ mutables → 1 domain struct)

**Target Architecture:**
```rust
/// The waveform timeline with cursor, viewport, and interaction state
/// Replaces 25+ timeline/canvas/animation mutables  
#[derive(Clone)]
struct WaveformTimeline {
    /// Core timeline state
    cursor_position: Actor<f64>,              // Nanoseconds
    visible_range: Actor<(f64, f64)>,        // (start_ns, end_ns)
    zoom_level: Actor<f32>,                  // Zoom factor
    canvas_dimensions: Actor<(f32, f32)>,    // (width, height)
    
    /// User interaction events
    cursor_clicked_relay: Relay<f64>,         // User clicked timeline at position
    mouse_moved_relay: Relay<(f32, f32)>,    // Mouse moved over canvas
    zoom_changed_relay: Relay<f32>,          // Zoom wheel or keyboard
    pan_started_relay: Relay<(f32, f32)>,    // Mouse drag started
    pan_moved_relay: Relay<(f32, f32)>,      // Mouse drag continued
    pan_ended_relay: Relay,                  // Mouse drag ended
    canvas_resized_relay: Relay<(f32, f32)>, // Canvas size changed
    
    /// Keyboard navigation events
    left_key_pressed_relay: Relay,           // Left arrow key
    right_key_pressed_relay: Relay,          // Right arrow key
    zoom_in_pressed_relay: Relay,            // Plus key or Ctrl+Plus
    zoom_out_pressed_relay: Relay,           // Minus key or Ctrl+Minus
    home_pressed_relay: Relay,               // Home key (go to start)
    end_pressed_relay: Relay,                // End key (go to end)
}
```

#### 2.2 SignalData Domain (5 mutables → 1 domain struct)

```rust
/// Signal data for timeline display and cursor values
/// Replaces signal service mutables with clean data representation
#[derive(Clone)]  
struct SignalData {
    /// Core data collections
    viewport_signals: ActorVec<SignalTransition>,    // Visible signal data
    cursor_values: ActorBTreeMap<String, String>,    // Current cursor values
    cache_statistics: Actor<CacheStats>,             // Performance metrics
    
    /// Data request events
    viewport_data_requested_relay: Relay<ViewportRequest>,
    cursor_data_requested_relay: Relay<CursorRequest>,
    cache_invalidated_relay: Relay<String>,          // Signal ID to invalidate
    
    /// Data response events  
    viewport_data_received_relay: Relay<ViewportResponse>,
    cursor_data_received_relay: Relay<CursorResponse>,
    request_failed_relay: Relay<(String, String)>,  // (request_id, error)
}
```

### Phase 3: UI & Configuration Systems (Week 3)

#### 3.1 Local UI State Migration (SimpleState Pattern)

**Convert all local UI mutables to SimpleState:**
```rust
/// Panel dimensions - local to each panel component
struct PanelState {
    width: SimpleState<f32>,
    height: SimpleState<f32>,
    is_collapsed: SimpleState<bool>,
}

/// Dialog state - local to dialog component
struct FileDialogState {
    is_open: SimpleState<bool>,
    filter_text: SimpleState<String>,
    selected_files: SimpleState<Vec<PathBuf>>,
    current_directory: SimpleState<PathBuf>,
}

/// Search state - local to search component
struct SearchState {
    filter_text: SimpleState<String>,
    is_focused: SimpleState<bool>,
    match_count: SimpleState<usize>,
}
```

#### 3.2 UserConfiguration Domain (6 mutables → 1 domain struct)

```rust
/// User's persistent configuration  
/// Replaces CONFIG_* mutables with type-safe persistence
#[derive(Clone)]
struct UserConfiguration {
    /// Configuration state
    workspace_config: Actor<WorkspaceConfig>,
    
    /// Configuration events
    theme_changed_relay: Relay<Theme>,
    dock_mode_changed_relay: Relay<DockMode>,
    panel_resized_relay: Relay<(PanelId, f32, f32)>,
    file_opened_relay: Relay<PathBuf>,           // Add to recent files
    workspace_saved_relay: Relay<PathBuf>,       // Save config to file
    
    /// Configuration system events
    config_loaded_relay: Relay<WorkspaceConfig>, // Loaded from disk
    save_requested_relay: Relay,                 // Auto-save triggered
    config_error_relay: Relay<String>,           // Save/load error
}
```

## Event-Source Naming Reference

### User Interface Events
```rust
// Button interactions
button_clicked_relay: Relay,
button_pressed_relay: Relay,                    // Mouse down
button_released_relay: Relay,                   // Mouse up
button_hovered_relay: Relay<bool>,              // Enter/exit hover

// Input interactions  
input_changed_relay: Relay<String>,             // Text input changed
input_focused_relay: Relay,                     // Input gained focus
input_blurred_relay: Relay,                     // Input lost focus
enter_pressed_relay: Relay,                     // Enter key in input

// Selection interactions
item_selected_relay: Relay<String>,             // User selected item
item_deselected_relay: Relay<String>,           // User deselected item
selection_cleared_relay: Relay,                 // Clear all selection

// Drag and drop
drag_started_relay: Relay<DragData>,            // Drag operation began
drag_moved_relay: Relay<(f32, f32)>,           // Drag position changed
drop_completed_relay: Relay<DropData>,          // Item dropped
drag_cancelled_relay: Relay,                   // Drag cancelled (Escape)
```

### System Events
```rust
// File system events
file_loaded_relay: Relay<PathBuf>,              // File loaded successfully
file_error_relay: Relay<(PathBuf, String)>,    // File load failed
directory_scanned_relay: Relay<Vec<PathBuf>>,   // Directory scan complete

// Network events  
request_sent_relay: Relay<RequestId>,           // HTTP request sent
response_received_relay: Relay<Response>,       // HTTP response received
connection_lost_relay: Relay,                  // Network disconnected
timeout_reached_relay: Relay<RequestId>,       // Request timed out

// Parsing events
parse_started_relay: Relay<PathBuf>,            // Parser began processing
parse_completed_relay: Relay<ParseResult>,     // Parser finished successfully
parse_failed_relay: Relay<(PathBuf, String)>,  // Parser failed with error

// Timer events
timer_elapsed_relay: Relay<TimerId>,            // Timer fired
animation_frame_relay: Relay<f64>,              // Animation frame ready
debounce_expired_relay: Relay<String>,          // Debounce period ended
```

### Keyboard Events  
```rust
// Navigation keys
left_key_pressed_relay: Relay,
right_key_pressed_relay: Relay,
up_key_pressed_relay: Relay,
down_key_pressed_relay: Relay,
home_key_pressed_relay: Relay,
end_key_pressed_relay: Relay,

// Modifier combinations
ctrl_z_pressed_relay: Relay,                   // Undo
ctrl_y_pressed_relay: Relay,                   // Redo  
ctrl_s_pressed_relay: Relay,                   // Save
escape_pressed_relay: Relay,                   // Cancel/close

// Function keys
f5_pressed_relay: Relay,                       // Refresh
f11_pressed_relay: Relay,                      // Fullscreen toggle
```

## Migration Execution Plan

### Week 1: Core Domain Setup
1. **Day 1-2**: Create `TrackedFiles` domain, migrate file-related mutables
2. **Day 3-4**: Create `SelectedVariables` domain, migrate selection mutables  
3. **Day 5**: Integration testing, fix reactive chains

### Week 2: Performance Systems
1. **Day 1-3**: Create `WaveformTimeline` domain, migrate timeline mutables
2. **Day 4-5**: Create `SignalData` domain, migrate service mutables

### Week 3: UI & Configuration
1. **Day 1-2**: Convert all local UI mutables to SimpleState
2. **Day 3-4**: Create `UserConfiguration` domain
3. **Day 5**: Final integration, performance testing

## Success Metrics

### Quantitative Goals
- ✅ Migrate all 74+ global mutables to Actor+Relay
- ✅ Zero recursive lock panics
- ✅ 85% reduction in over-rendering (30+ → <5 renders per operation)
- ✅ 100% event-source relay naming compliance
- ✅ Zero Manager/Service/Controller abstractions
- ✅ 80%+ test coverage for all domain Actors

### Qualitative Goals
- ✅ All state mutations traceable through event logs
- ✅ Clear domain modeling throughout codebase
- ✅ Simplified architecture without artificial boundaries
- ✅ Improved developer experience with cleaner APIs
- ✅ Enhanced debugging with event-source tracing

## Naming Convention Enforcement

### Automated Checks
```rust
// CI check for relay naming compliance
#[cfg(test)]
mod naming_compliance {
    use regex::Regex;
    
    #[test]
    fn verify_relay_naming() {
        let relay_pattern = Regex::new(r"^\w+_\w+_relay$").unwrap();
        
        // Scan codebase for Relay field declarations
        // Ensure all follow {source}_{event}_relay pattern
        // Fail CI if non-compliant naming found
    }
    
    #[test] 
    fn verify_no_manager_service_patterns() {
        let forbidden_patterns = [
            "Manager", "Service", "Controller", 
            "Handler", "Processor", "Helper"
        ];
        
        // Scan struct names for enterprise patterns
        // Fail CI if Manager/Service patterns found
    }
}
```

This migration strategy provides a comprehensive roadmap for transforming NovyWave into a clean, domain-driven Actor+Relay architecture with consistent event-source naming throughout.