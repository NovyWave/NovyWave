# Event-Source Relay Naming Guide for NovyWave

This document provides comprehensive guidance on the **mandatory** event-source relay naming convention for NovyWave's Actor+Relay architecture. All relay names must follow the `{source}_{event}_relay` pattern.

## Core Principle: Event-Source Pattern

**Relays describe WHAT HAPPENED, not what to do**

The naming pattern captures the **source** of the event and the **event** that occurred, making the system naturally traceable and self-documenting.

### Pattern Template: `{source}_{event}_relay`

- **source**: Who or what generated the event (user, system, component)
- **event**: What happened (clicked, changed, completed, failed, etc.)
- **_relay**: Consistent suffix for all event relays

## ✅ CORRECT Event-Source Examples

### User Interaction Events

**Pattern: `{ui_element}_{interaction}_relay`**

```rust
// Button interactions
button_clicked_relay: Relay,
button_pressed_relay: Relay,                    // Mouse down
button_released_relay: Relay,                   // Mouse up  
save_button_clicked_relay: Relay,               // Specific button
cancel_button_clicked_relay: Relay,

// Input field interactions  
input_changed_relay: Relay<String>,             // Text content changed
input_focused_relay: Relay,                     // Gained focus
input_blurred_relay: Relay,                     // Lost focus
search_input_changed_relay: Relay<String>,      // Specific input

// Dropdown/Select interactions
dropdown_opened_relay: Relay,                   // Dropdown expanded
dropdown_closed_relay: Relay,                   // Dropdown collapsed  
option_selected_relay: Relay<String>,           // User chose option
menu_item_selected_relay: Relay<MenuId>,        // Menu navigation

// Mouse interactions
mouse_moved_relay: Relay<(f32, f32)>,          // Mouse position changed
mouse_clicked_relay: Relay<(f32, f32)>,        // Mouse clicked at position
mouse_entered_relay: Relay,                     // Mouse entered element
mouse_exited_relay: Relay,                      // Mouse left element

// Drag and drop
drag_started_relay: Relay<DragData>,            // Drag operation began
drag_moved_relay: Relay<(f32, f32)>,           // Drag position changed  
drop_completed_relay: Relay<DropData>,          // Item dropped
files_dropped_relay: Relay<Vec<PathBuf>>,       // Files dropped on area

// Keyboard events
key_pressed_relay: Relay<String>,               // Any key pressed
enter_pressed_relay: Relay,                     // Enter key specifically
escape_pressed_relay: Relay,                    // Escape key
arrow_key_pressed_relay: Relay<Direction>,      // Arrow navigation
```

### System Events

**Pattern: `{system_component}_{event}_relay`**

```rust
// File system events
file_loaded_relay: Relay<PathBuf>,              // File loaded successfully
file_saved_relay: Relay<PathBuf>,               // File saved to disk
file_error_relay: Relay<(PathBuf, String)>,    // File operation failed
directory_scanned_relay: Relay<Vec<PathBuf>>,   // Directory scan complete
file_watcher_changed_relay: Relay<PathBuf>,     // File changed on disk

// Network events
request_sent_relay: Relay<RequestId>,           // HTTP request sent
response_received_relay: Relay<Response>,       // HTTP response received  
connection_lost_relay: Relay,                   // Network disconnected
connection_restored_relay: Relay,               // Network reconnected
timeout_reached_relay: Relay<RequestId>,        // Request timed out

// Parsing events
parser_started_relay: Relay<PathBuf>,           // Parser began processing
parse_completed_relay: Relay<ParseResult>,      // Parser finished successfully
parse_failed_relay: Relay<(PathBuf, String)>,  // Parser failed with error
parse_progress_relay: Relay<f32>,               // Parser progress update

// Timer events
timer_elapsed_relay: Relay<TimerId>,            // Timer fired
debounce_expired_relay: Relay<String>,          // Debounce period ended
animation_frame_relay: Relay<f64>,              // Animation frame ready
```

### Application State Events

**Pattern: `{state_area}_{change}_relay`**

```rust
// Configuration events
config_loaded_relay: Relay<AppConfig>,          // Config loaded from disk
config_changed_relay: Relay<ConfigKey>,         // Config value changed
theme_changed_relay: Relay<Theme>,              // Theme switched
preferences_updated_relay: Relay<UserPrefs>,    // User preferences modified

// Window/UI state events  
window_resized_relay: Relay<(f32, f32)>,       // Window size changed
panel_resized_relay: Relay<PanelDimensions>,   // Panel dimensions changed
dialog_opened_relay: Relay<DialogType>,        // Dialog became visible
dialog_closed_relay: Relay<DialogType>,        // Dialog was dismissed
tab_switched_relay: Relay<TabId>,              // Tab selection changed

// Data events
data_invalidated_relay: Relay<String>,         // Data cache invalidated  
refresh_requested_relay: Relay,                // User requested refresh
sync_completed_relay: Relay<SyncResult>,       // Data synchronization done
backup_created_relay: Relay<PathBuf>,          // Backup file created
```

## ❌ PROHIBITED Command-Style Examples

**These patterns sound like commands or imperatives - never use them:**

```rust
// ❌ Command-like naming (sounds like actions to take)
add_file: Relay<PathBuf>,                       // Use: file_dropped_relay
remove_item: Relay<String>,                     // Use: item_removed_relay  
set_theme: Relay<Theme>,                        // Use: theme_changed_relay
update_config: Relay<Config>,                   // Use: config_updated_relay
clear_selection: Relay,                         // Use: selection_cleared_relay
save_document: Relay,                           // Use: save_requested_relay
load_data: Relay,                               // Use: data_load_requested_relay
refresh_ui: Relay,                              // Use: refresh_requested_relay

// ❌ Imperative verbs (sound like instructions)
create_user: Relay<UserData>,                   // Use: user_created_relay
delete_record: Relay<RecordId>,                 // Use: record_deleted_relay
send_message: Relay<Message>,                   // Use: message_sent_relay
process_queue: Relay,                           // Use: queue_processed_relay

// ❌ Manager/Service style (artificial abstractions)  
file_manager_action: Relay<FileAction>,         // Use specific events
data_service_request: Relay<ServiceRequest>,    // Use specific events
ui_controller_event: Relay<ControllerEvent>,    // Use specific events
```

## NovyWave Domain-Specific Examples

### File Management Domain

```rust
struct TrackedFiles {
    files: ActorVec<TrackedFile>,
    
    // ✅ User interactions with files
    file_dropped_relay: Relay<Vec<PathBuf>>,        // User dropped files on UI
    file_selected_relay: Relay<PathBuf>,            // User clicked file in list
    file_double_clicked_relay: Relay<PathBuf>,      // User double-clicked file
    reload_button_clicked_relay: Relay<String>,     // User clicked reload for file
    remove_button_clicked_relay: Relay<String>,     // User clicked remove button
    clear_all_clicked_relay: Relay,                 // User clicked clear all files
    
    // ✅ System events for file operations
    parse_started_relay: Relay<String>,             // Parser began processing file
    parse_completed_relay: Relay<(String, ParseResult)>, // Parser finished
    parse_failed_relay: Relay<(String, String)>,    // Parse error occurred
    file_watcher_changed_relay: Relay<PathBuf>,     // File changed on disk
    
    // ❌ WRONG: Command-style naming
    // add_file: Relay<PathBuf>,                    // Sounds like command
    // remove_file: Relay<String>,                  // Imperative style
    // parse_file: Relay<PathBuf>,                  // Action-oriented
}
```

### Variable Selection Domain

```rust  
struct SelectedVariables {
    variables: ActorVec<SelectedVariable>,
    
    // ✅ User selection interactions
    variable_clicked_relay: Relay<String>,          // User clicked variable in tree
    variable_double_clicked_relay: Relay<String>,   // User double-clicked variable  
    variable_removed_relay: Relay<String>,          // User removed variable
    scope_expanded_relay: Relay<String>,            // User expanded scope node
    scope_collapsed_relay: Relay<String>,           // User collapsed scope node
    clear_selection_clicked_relay: Relay,           // User clicked clear all
    select_all_clicked_relay: Relay,                // User clicked select all
    
    // ✅ System events for selection
    selection_restored_relay: Relay<Vec<String>>,   // Config loaded selections
    filter_applied_relay: Relay<String>,            // Search filter applied
    scope_loaded_relay: Relay<ScopeData>,           // New scope data loaded
    variable_data_updated_relay: Relay<String>,     // Variable metadata changed
    
    // ❌ WRONG: Command-style naming  
    // add_variable: Relay<String>,                 // Sounds like command
    // remove_selection: Relay<String>,             // Imperative
    // update_filter: Relay<String>,                // Action-oriented
}
```

### Timeline/Waveform Domain

```rust
struct WaveformTimeline {
    // State
    cursor_position: Actor<f64>,
    visible_range: Actor<(f64, f64)>,
    zoom_level: Actor<f32>,
    
    // ✅ User timeline interactions
    cursor_clicked_relay: Relay<f64>,               // User clicked timeline at position
    cursor_dragged_relay: Relay<f64>,               // User dragged cursor  
    mouse_moved_relay: Relay<(f32, f32)>,          // Mouse moved over canvas
    zoom_wheel_scrolled_relay: Relay<f32>,          // User scrolled zoom wheel
    pan_started_relay: Relay<(f32, f32)>,          // User started pan drag
    pan_moved_relay: Relay<(f32, f32)>,            // User continued pan drag
    pan_ended_relay: Relay,                        // User ended pan drag
    canvas_resized_relay: Relay<(f32, f32)>,       // Canvas size changed
    
    // ✅ Keyboard navigation events
    left_arrow_pressed_relay: Relay,               // Left arrow key pressed
    right_arrow_pressed_relay: Relay,              // Right arrow key pressed
    home_key_pressed_relay: Relay,                 // Home key pressed (go to start)
    end_key_pressed_relay: Relay,                  // End key pressed (go to end)
    page_up_pressed_relay: Relay,                  // Page up key pressed
    page_down_pressed_relay: Relay,                // Page down key pressed
    plus_key_pressed_relay: Relay,                 // Plus key (zoom in)
    minus_key_pressed_relay: Relay,                // Minus key (zoom out)
    
    // ✅ System events for timeline
    data_loaded_relay: Relay<TimelineData>,        // Timeline data loaded
    viewport_changed_relay: Relay<ViewportRange>,  // Visible range changed
    cursor_value_updated_relay: Relay<CursorValues>, // Signal values at cursor updated
    render_completed_relay: Relay,                 // Timeline render finished
    
    // ❌ WRONG: Command-style naming
    // set_cursor: Relay<f64>,                      // Sounds like command
    // zoom_in: Relay,                              // Imperative verb
    // pan_timeline: Relay<f32>,                    // Action-oriented
    // update_viewport: Relay<ViewportRange>,       // Command pattern
}
```

### UI Panel/Dialog Domain

```rust
// Panel state - using Atom for local UI
struct FileDialogState {
    is_open: Atom<bool>,
    filter_text: Atom<String>,
    selected_files: Atom<Vec<PathBuf>>,
    current_directory: Atom<PathBuf>,
    
    // ✅ Dialog-specific events (when using Actors)
    open_button_clicked_relay: Relay,              // User clicked Open button
    cancel_button_clicked_relay: Relay,            // User clicked Cancel  
    filter_changed_relay: Relay<String>,           // User typed in filter
    directory_navigated_relay: Relay<PathBuf>,     // User navigated to directory
    file_item_clicked_relay: Relay<PathBuf>,       // User clicked file item
    file_item_double_clicked_relay: Relay<PathBuf>, // User double-clicked file
    
    // ❌ WRONG: Command-style naming
    // open_dialog: Relay,                          // Sounds like command  
    // close_dialog: Relay,                         // Imperative verb
    // set_directory: Relay<PathBuf>,               // Action-oriented
}

struct PanelState {
    width: Atom<f32>,
    height: Atom<f32>,
    is_collapsed: Atom<bool>,
    
    // ✅ Panel interaction events (when using Actors)
    resize_handle_dragged_relay: Relay<(f32, f32)>, // User dragged resize handle
    collapse_button_clicked_relay: Relay,           // User clicked collapse button
    header_double_clicked_relay: Relay,             // User double-clicked header
    panel_moved_relay: Relay<(f32, f32)>,          // Panel was repositioned
    
    // ❌ WRONG: Command-style naming
    // resize_panel: Relay<(f32, f32)>,             // Sounds like command
    // collapse_panel: Relay,                       // Imperative verb
}
```

## Naming Decision Tree

Use this decision tree to choose the correct relay name:

```
What happened?
├── User performed an action?
│   ├── Clicked something? → {element}_clicked_relay
│   ├── Typed/Changed input? → {input}_changed_relay  
│   ├── Dragged something? → {element}_dragged_relay
│   ├── Selected something? → {item}_selected_relay
│   └── Navigated somewhere? → {location}_navigated_relay
│
├── System event occurred?
│   ├── Data loaded/processed? → {data_type}_loaded_relay
│   ├── Operation completed? → {operation}_completed_relay
│   ├── Error occurred? → {operation}_failed_relay  
│   ├── Timer/Schedule fired? → {timer}_elapsed_relay
│   └── External change detected? → {source}_changed_relay
│
└── State change happened?
    ├── Configuration changed? → {config_item}_changed_relay
    ├── UI state toggled? → {ui_element}_toggled_relay
    ├── Data updated? → {data_type}_updated_relay
    └── Connection status changed? → {connection}_changed_relay
```

## Common Mistakes and Corrections

### Mistake 1: Using Imperative Verbs

```rust
// ❌ WRONG: Sounds like a command to execute
add_file_relay: Relay<PathBuf>,
remove_variable_relay: Relay<String>,
update_theme_relay: Relay<Theme>,

// ✅ CORRECT: Describes what happened
file_dropped_relay: Relay<PathBuf>,           // User dropped file
variable_removed_relay: Relay<String>,        // User removed variable  
theme_changed_relay: Relay<Theme>,            // Theme was changed
```

### Mistake 2: Generic Action Names

```rust
// ❌ WRONG: Too generic, doesn't specify source
action_relay: Relay<Action>,
event_relay: Relay<Event>,
update_relay: Relay<UpdateData>,

// ✅ CORRECT: Specific source and event
button_clicked_relay: Relay,                  // Button was clicked
data_loaded_relay: Relay<Data>,               // Data finished loading
config_updated_relay: Relay<Config>,          // Config was updated
```

### Mistake 3: Manager/Service Patterns

```rust
// ❌ WRONG: Abstract management concepts
file_manager_event: Relay<FileEvent>,
data_service_request: Relay<ServiceRequest>,
ui_controller_action: Relay<ControllerAction>,

// ✅ CORRECT: Specific domain events  
file_loaded_relay: Relay<PathBuf>,            // File was loaded
data_requested_relay: Relay<DataRequest>,     // Data was requested
button_clicked_relay: Relay<ButtonId>,        // Button was clicked
```

### Mistake 4: Missing Event Context

```rust
// ❌ WRONG: Doesn't specify what happened
file_relay: Relay<PathBuf>,                   // What about the file?
user_relay: Relay<UserId>,                    // What did the user do?
timer_relay: Relay,                           // What happened with the timer?

// ✅ CORRECT: Clear event context
file_selected_relay: Relay<PathBuf>,          // File was selected
user_logged_in_relay: Relay<UserId>,          // User logged in
timer_elapsed_relay: Relay<TimerId>,          // Timer elapsed/fired
```

## Validation Checklist

Before implementing a relay, verify it passes these checks:

- [ ] **Event-source pattern**: Uses `{source}_{event}_relay` format
- [ ] **Past tense or state**: Describes what happened, not what to do
- [ ] **Specific source**: Clearly identifies where the event came from
- [ ] **Specific event**: Clearly describes what occurred  
- [ ] **No commands**: Doesn't sound like an instruction or action to take
- [ ] **No abstractions**: Avoids Manager/Service/Controller patterns
- [ ] **Domain appropriate**: Fits the specific domain (files, timeline, UI, etc.)
- [ ] **Type safety**: Uses appropriate generic type for event data

## Benefits of Event-Source Naming

### 1. **Natural Traceability**
```rust
// Clear event flow through logs
file_dropped_relay.send(paths);           // "File drop event from UI"  
parse_completed_relay.send(result);       // "Parse completion from parser"
cursor_moved_relay.send(position);        // "Cursor movement from user"
```

### 2. **Self-Documenting Code**
```rust
// Names explain themselves - no comments needed
user_clicked_variable_relay.send(var_id);    // Obvious: user clicked variable
config_loaded_relay.send(config);            // Obvious: config finished loading  
error_occurred_relay.send(error_msg);        // Obvious: error happened
```

### 3. **Debugging Clarity**
```rust
// Debug logs are immediately understandable
zoon::println!("🔄 Processing file_dropped_relay event");
zoon::println!("✅ Emitting parse_completed_relay event"); 
zoon::println!("❌ Sending error_occurred_relay event");
```

### 4. **Testing Clarity**
```rust
#[async_test]
async fn test_file_drop_workflow() {
    let tracked_files = TrackedFiles::new();
    
    // Test names clearly show what's being tested
    tracked_files.file_dropped_relay.send(vec![path]);
    tracked_files.parse_completed_relay.send((file_id, result));
    
    // Assertions are self-explanatory
    assert!(files_updated_correctly);
}
```

This event-source naming convention ensures NovyWave's Actor+Relay architecture remains clean, traceable, and maintainable throughout the migration from 74+ global Mutables.