# Complete NovyWave Migration Examples: 74+ Mutables → Actor+Relay

This document provides complete migration examples for transforming NovyWave's 74+ global static Mutables into domain-driven Actor+Relay architecture with proper event-source naming.

## Migration Summary

**Current State**: 74+ global static Mutables scattered across the codebase  
**Target State**: 5 domain-driven Actor+Relay structs + Atom for local UI

### Domain Consolidation Strategy

| Category | Current Mutables | Target Actor+Relay Domain |
|----------|------------------|---------------------------|
| File Management (13) | `TRACKED_FILES`, `LOADING_FILES`, `FILE_PATHS`, etc. | `TrackedFiles` struct |
| Variable Selection (8) | `SELECTED_VARIABLES`, `SELECTED_SCOPE_ID`, etc. | `SelectedVariables` struct |  
| Timeline & Canvas (25+) | `TIMELINE_CURSOR_NS`, `CANVAS_WIDTH`, etc. | `WaveformTimeline` struct |
| Signal Data Service (5) | `VIEWPORT_SIGNALS`, `CURSOR_VALUES`, etc. | `SignalDataCache` struct |
| Configuration (6+) | `CONFIG_LOADED`, `SAVE_CONFIG_PENDING`, etc. | `UserConfiguration` struct |
| UI Layout & Local (23+) | Panel dimensions, dialog states, etc. | Atom in components |

## Phase 1: File Management Domain (13 → 1)

### Current Global Mutables
```rust
// 13 global file-related mutables
static FILE_UPDATE_QUEUE: Lazy<MutableVec<FileUpdateMessage>> = lazy::default();
static QUEUE_PROCESSOR_RUNNING: Lazy<Mutable<bool>> = lazy::default();
static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = lazy::default();
static TRACKED_FILE_IDS: Lazy<MutableVec<String>> = lazy::default();
static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
static LOADED_FILES: Lazy<MutableVec<LoadedFile>> = lazy::default();
static FILE_PATHS: Lazy<MutableVec<String>> = lazy::default();
static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();
static FILE_TREE_CACHE: Lazy<Mutable<HashMap<String, Vec<PathBuf>>>> = lazy::default();
static CURRENT_DIRECTORY: Lazy<Mutable<PathBuf>> = lazy::default();
static FILE_PICKER_EXPANDED: Lazy<Mutable<HashSet<String>>> = lazy::default();
static FILE_PICKER_SELECTED: Lazy<MutableVec<PathBuf>> = lazy::default();
static FILE_PICKER_ERROR: Lazy<Mutable<Option<String>>> = lazy::default();
```

### Target Domain Structure
```rust
use crate::reactive_actors::{Actor, ActorVec, Relay, relay, select};

/// Central waveform file management domain
/// Replaces 13 global file-related mutables with clean domain representation
#[derive(Clone)]
pub struct TrackedFiles {
    /// Core collections
    files: ActorVec<TrackedFile>,
    loading_files: ActorVec<LoadingFile>,
    file_tree_cache: Actor<HashMap<String, Vec<PathBuf>>>,
    
    /// User interaction events - what the user DID
    files_dropped_relay: Relay<Vec<PathBuf>>,           // User dropped files on UI
    file_selected_relay: Relay<PathBuf>,                // User clicked file in list
    file_double_clicked_relay: Relay<PathBuf>,          // User double-clicked file
    reload_button_clicked_relay: Relay<String>,         // User clicked reload for file
    remove_button_clicked_relay: Relay<String>,         // User clicked remove button
    clear_all_clicked_relay: Relay,                     // User clicked clear all
    directory_expanded_relay: Relay<String>,            // User expanded directory
    directory_collapsed_relay: Relay<String>,           // User collapsed directory
    
    /// System events - what HAPPENED in the system
    file_load_started_relay: Relay<PathBuf>,            // File loading began
    parse_completed_relay: Relay<(String, ParseResult)>, // Parser finished
    parse_failed_relay: Relay<(String, String)>,        // Parse error occurred
    directory_scanned_relay: Relay<(String, Vec<PathBuf>)>, // Directory scan completed
    file_watcher_changed_relay: Relay<PathBuf>,         // File changed on disk
    cache_invalidated_relay: Relay<String>,             // Cache entry invalidated
}

impl TrackedFiles {
    pub fn new() -> Self {
        // Create all event streams
        let (files_dropped_relay, files_dropped_stream) = relay();
        let (file_selected_relay, file_selected_stream) = relay();  
        let (reload_button_clicked_relay, reload_stream) = relay();
        let (remove_button_clicked_relay, remove_stream) = relay();
        let (clear_all_clicked_relay, clear_stream) = relay();
        let (parse_completed_relay, parse_completed_stream) = relay();
        let (parse_failed_relay, parse_failed_stream) = relay();
        let (directory_scanned_relay, directory_scanned_stream) = relay();
        let (file_load_started_relay, file_load_stream) = relay();
        
        // Create main files collection with event processing
        let files = ActorVec::new(vec![], async move |files_vec| {
            loop {
                select! {
                    Some(paths) = files_dropped_stream.next() => {
                        // Add multiple files from drop operation
                        for path in paths {
                            let file_id = generate_file_id(&path);
                            let tracked_file = TrackedFile::new(file_id.clone(), path.clone());
                            files_vec.lock_mut().push_cloned(tracked_file);
                            
                            // Start loading process
                            file_load_started_relay.send(path);
                        }
                    }
                    Some(file_id) = remove_stream.next() => {
                        files_vec.lock_mut().retain(|f| f.id != file_id);
                    }
                    Some(()) = clear_stream.next() => {
                        files_vec.lock_mut().clear();
                    }
                    Some(file_id) = reload_stream.next() => {
                        // Find and reload specific file
                        if let Some(file) = files_vec.lock_ref().iter().find(|f| f.id == file_id) {
                            file_load_started_relay.send(file.path.clone());
                        }
                    }
                    Some((file_id, result)) = parse_completed_stream.next() => {
                        // Update file with successful parse result
                        if let Some(file) = files_vec.lock_ref().iter().find(|f| f.id == file_id) {
                            file.parse_completed_relay.send(result);
                        }
                    }
                    Some((file_id, error)) = parse_failed_stream.next() => {
                        // Update file with parse error
                        if let Some(file) = files_vec.lock_ref().iter().find(|f| f.id == file_id) {
                            file.parse_failed_relay.send(error);
                        }
                    }
                }
            }
        });
        
        // Create loading files queue
        let loading_files = ActorVec::new(vec![], async move |loading_vec| {
            while let Some(path) = file_load_stream.next().await {
                let loading_file = LoadingFile::new(path.clone());
                loading_vec.lock_mut().push_cloned(loading_file);
                
                // Start background parsing
                let loading_vec_clone = loading_vec.clone();
                let parse_completed_relay_clone = parse_completed_relay.clone();
                let parse_failed_relay_clone = parse_failed_relay.clone();
                
                Task::start(async move {
                    match parse_waveform_file(&path).await {
                        Ok(result) => {
                            let file_id = generate_file_id(&path);
                            parse_completed_relay_clone.send((file_id.clone(), result));
                            
                            // Remove from loading queue
                            loading_vec_clone.lock_mut().retain(|f| f.path != path);
                        }
                        Err(error) => {
                            let file_id = generate_file_id(&path);
                            parse_failed_relay_clone.send((file_id, error.to_string()));
                            
                            // Remove from loading queue  
                            loading_vec_clone.lock_mut().retain(|f| f.path != path);
                        }
                    }
                });
            }
        });
        
        // Create directory cache
        let file_tree_cache = Actor::new(HashMap::new(), async move |cache| {
            while let Some((dir_path, entries)) = directory_scanned_stream.next().await {
                cache.update(|mut map| {
                    map.insert(dir_path, entries);
                    map
                });
            }
        });
        
        TrackedFiles {
            files,
            loading_files, 
            file_tree_cache,
            files_dropped_relay,
            file_selected_relay,
            file_double_clicked_relay: Relay::new(),
            reload_button_clicked_relay,
            remove_button_clicked_relay,
            clear_all_clicked_relay,
            directory_expanded_relay: Relay::new(),
            directory_collapsed_relay: Relay::new(), 
            file_load_started_relay,
            parse_completed_relay,
            parse_failed_relay,
            directory_scanned_relay,
            file_watcher_changed_relay: Relay::new(),
            cache_invalidated_relay: Relay::new(),
        }
    }
    
    /// Get reactive signal for all tracked files
    pub fn files_signal(&self) -> impl SignalVec<Item = TrackedFile> {
        self.files.signal_vec_cloned()
    }
    
    /// Get reactive signal for loading status
    pub fn is_loading_signal(&self) -> impl Signal<Item = bool> {
        self.loading_files.len_signal().map(|count| count > 0)
    }
    
    /// Get reactive signal for file count  
    pub fn file_count_signal(&self) -> impl Signal<Item = usize> {
        self.files.len_signal()
    }
}

/// Individual tracked file with its own state and events
#[derive(Clone)]
pub struct TrackedFile {
    pub id: String,
    pub path: PathBuf,
    pub state: Actor<FileState>,
    
    /// File-specific events
    pub parse_completed_relay: Relay<ParseResult>,
    pub parse_failed_relay: Relay<String>,
    pub state_changed_relay: Relay<FileState>,
}

#[derive(Clone, Debug)]
pub enum FileState {
    Loading,
    Parsed { 
        signals: Vec<WaveformSignal>,
        time_range: (f64, f64),
        scope_count: usize,
    },
    Error(String),
}

impl TrackedFile {
    pub fn new(id: String, path: PathBuf) -> Self {
        let (parse_completed_relay, parse_completed_stream) = relay();
        let (parse_failed_relay, parse_failed_stream) = relay();
        let (state_changed_relay, _) = relay();
        
        let state = Actor::new(FileState::Loading, async move |state_actor| {
            loop {
                select! {
                    Some(result) = parse_completed_stream.next() => {
                        match result {
                            ParseResult::Success { signals, time_range } => {
                                let scope_count = count_unique_scopes(&signals);
                                state_actor.set_neq(FileState::Parsed { 
                                    signals, 
                                    time_range,
                                    scope_count,
                                });
                            }
                        }
                    }
                    Some(error) = parse_failed_stream.next() => {
                        state_actor.set_neq(FileState::Error(error));
                    }
                }
            }
        });
        
        TrackedFile {
            id,
            path,
            state,
            parse_completed_relay,
            parse_failed_relay,
            state_changed_relay,
        }
    }
}
```

### Migration Benefits
- **13 mutables → 1 domain struct**: Massive reduction in global state
- **Event traceability**: Every file operation logged with source
- **No recursive locks**: Sequential event processing eliminates race conditions  
- **Clean separation**: File operations, loading queue, and cache cleanly separated
- **Testable**: Individual file states and operations can be tested in isolation

## Phase 2: Variable Selection Domain (8 → 1)

### Current Global Mutables
```rust
// 8 global variable selection mutables  
static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = lazy::default();
static TREE_SELECTED_ITEMS: Lazy<MutableVec<String>> = lazy::default();
static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default();
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = lazy::default();
static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = lazy::default();
static SELECTED_VARIABLES_INDEX: Lazy<Mutable<HashMap<String, usize>>> = lazy::default();
static SIGNAL_VALUES: Lazy<Mutable<HashMap<String, String>>> = lazy::default();
static SELECTED_VARIABLE_FORMATS: Lazy<Mutable<HashMap<String, SignalFormat>>> = lazy::default();
```

### Target Domain Structure  
```rust
/// Variable selection and display management domain
/// Replaces 8 global variable selection mutables
#[derive(Clone)]
pub struct SelectedVariables {
    /// Core collections
    variables: ActorVec<SelectedVariable>,
    expanded_scopes: Actor<HashSet<String>>,
    variable_formats: Actor<HashMap<String, SignalFormat>>,
    current_scope: Actor<Option<String>>,
    
    /// User selection events - what the user DID
    variable_clicked_relay: Relay<String>,              // User clicked variable in tree
    variable_double_clicked_relay: Relay<String>,       // User double-clicked variable
    variable_removed_relay: Relay<String>,              // User clicked remove button
    scope_expanded_relay: Relay<String>,                // User expanded scope chevron
    scope_collapsed_relay: Relay<String>,               // User collapsed scope chevron
    scope_selected_relay: Relay<String>,                // User selected different scope
    clear_selection_clicked_relay: Relay,               // User clicked clear all button
    select_all_in_scope_clicked_relay: Relay<String>,   // User clicked select all in scope
    format_changed_relay: Relay<(String, SignalFormat)>, // User changed display format
    
    /// System events - what HAPPENED in the system  
    selection_restored_relay: Relay<Vec<SelectedVariable>>, // Config loaded selection
    scope_data_loaded_relay: Relay<ScopeData>,          // New scope data available
    variable_data_updated_relay: Relay<String>,         // Variable metadata changed
    filter_applied_relay: Relay<String>,                // Search filter applied
    variables_reordered_relay: Relay<Vec<String>>,      // Variables reordered by user
}

impl SelectedVariables {
    pub fn new() -> Self {
        // Create event streams
        let (variable_clicked_relay, variable_clicked_stream) = relay();
        let (variable_removed_relay, variable_removed_stream) = relay();
        let (scope_expanded_relay, scope_expanded_stream) = relay();
        let (scope_collapsed_relay, scope_collapsed_stream) = relay(); 
        let (scope_selected_relay, scope_selected_stream) = relay();
        let (clear_selection_clicked_relay, clear_stream) = relay();
        let (format_changed_relay, format_changed_stream) = relay();
        let (selection_restored_relay, restoration_stream) = relay();
        
        // Create variables collection
        let variables = ActorVec::new(vec![], async move |vars_vec| {
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
                    Some(()) = clear_stream.next() => {
                        vars_vec.lock_mut().clear();
                    }
                    Some(variables) = restoration_stream.next() => {
                        // Restore selection from config
                        vars_vec.lock_mut().replace_cloned(variables);
                    }
                }
            }
        });
        
        // Create expanded scopes set
        let expanded_scopes = Actor::new(HashSet::new(), async move |scopes| {
            loop {
                select! {
                    Some(scope_id) = scope_expanded_stream.next() => {
                        scopes.update_mut(|set| { set.insert(scope_id); });
                    }
                    Some(scope_id) = scope_collapsed_stream.next() => {
                        scopes.update_mut(|set| { set.remove(&scope_id); });
                    }
                }
            }
        });
        
        // Create current scope tracker
        let current_scope = Actor::new(None, async move |scope| {
            while let Some(scope_id) = scope_selected_stream.next().await {
                scope.set_neq(Some(scope_id));
            }
        });
        
        // Create format preferences
        let variable_formats = Actor::new(HashMap::new(), async move |formats| {
            while let Some((var_id, format)) = format_changed_stream.next().await {
                formats.update_mut(|map| { map.insert(var_id, format); });
            }
        });
        
        SelectedVariables {
            variables,
            expanded_scopes,
            variable_formats,
            current_scope,
            variable_clicked_relay,
            variable_double_clicked_relay: Relay::new(),
            variable_removed_relay,
            scope_expanded_relay,
            scope_collapsed_relay,
            scope_selected_relay,
            clear_selection_clicked_relay,
            select_all_in_scope_clicked_relay: Relay::new(),
            format_changed_relay,
            selection_restored_relay,
            scope_data_loaded_relay: Relay::new(),
            variable_data_updated_relay: Relay::new(),
            filter_applied_relay: Relay::new(),
            variables_reordered_relay: Relay::new(),
        }
    }
    
    /// Get reactive signal for selected variables
    pub fn variables_signal(&self) -> impl SignalVec<Item = SelectedVariable> {
        self.variables.signal_vec_cloned()
    }
    
    /// Get reactive signal for expanded scopes
    pub fn expanded_scopes_signal(&self) -> impl Signal<Item = HashSet<String>> {
        self.expanded_scopes.signal()
    }
    
    /// Get reactive signal for variable count
    pub fn selection_count_signal(&self) -> impl Signal<Item = usize> {
        self.variables.len_signal()
    }
    
    /// Check if scope is expanded reactively
    pub fn is_scope_expanded_signal(&self, scope_id: String) -> impl Signal<Item = bool> {
        self.expanded_scopes.signal_ref(move |scopes| scopes.contains(&scope_id))
    }
}
```

## Phase 3: Timeline & Canvas Domain (25+ → 1) 

### Current Global Mutables
```rust
// 25+ timeline, canvas, and animation mutables
static TIMELINE_CURSOR_NS: Lazy<Mutable<f64>> = lazy::default();
static TIMELINE_VIEWPORT: Lazy<Mutable<(f64, f64)>> = lazy::default();
static TIMELINE_ZOOM_LEVEL: Lazy<Mutable<f32>> = lazy::default();
static CANVAS_WIDTH: Lazy<Mutable<f32>> = lazy::default();
static CANVAS_HEIGHT: Lazy<Mutable<f32>> = lazy::default();
static MOUSE_X_POSITION: Lazy<Mutable<f32>> = lazy::default();
static MOUSE_TIME_NS: Lazy<Mutable<f64>> = lazy::default();
static ZOOM_CENTER_NS: Lazy<Mutable<f64>> = lazy::default();

// Animation state (8 mutables)
static DIRECT_CURSOR_ANIMATION: Lazy<Mutable<bool>> = lazy::default();
static LAST_CANVAS_UPDATE: Lazy<Mutable<f64>> = lazy::default();
static PENDING_CANVAS_UPDATE: Lazy<Mutable<bool>> = lazy::default();
static LAST_ANIMATION_REQUEST: Lazy<Mutable<f64>> = lazy::default();
static LAST_CURSOR_REQUEST: Lazy<Mutable<f64>> = lazy::default();
static CURSOR_MOVEMENT_START: Lazy<Mutable<f64>> = lazy::default();
static FORCE_REDRAW: Lazy<Mutable<bool>> = lazy::default();
static LAST_REDRAW_TIME: Lazy<Mutable<f64>> = lazy::default();

// Request management (5 mutables)  
static HAS_PENDING_REQUEST: Lazy<Mutable<bool>> = lazy::default();
static ACTIVE_REQUEST_ID: Lazy<Mutable<Option<String>>> = lazy::default();
static REQUEST_ID_COUNTER: Lazy<Mutable<u64>> = lazy::default();
static REQUEST_COUNT: Lazy<Mutable<u32>> = lazy::default();
static REQUEST_RATE_WINDOW_START: Lazy<Mutable<f64>> = lazy::default();

// Caches and UI state (6+ mutables)
static SIGNAL_TRANSITIONS_CACHE: Lazy<Mutable<HashMap<String, Vec<SignalTransition>>>> = lazy::default();
static HOVER_INFO: Lazy<Mutable<Option<HoverInfo>>> = lazy::default();
static STARTUP_CURSOR_POSITION_SET: Lazy<Mutable<bool>> = lazy::default();
// ... more timeline-related mutables
```

### Target Domain Structure
```rust
/// Waveform timeline with cursor, viewport, canvas, and interaction state  
/// Replaces 25+ timeline/canvas/animation mutables
#[derive(Clone)]
pub struct WaveformTimeline {
    /// Core timeline state
    cursor_position: Actor<f64>,                    // Nanoseconds
    visible_range: Actor<(f64, f64)>,              // (start_ns, end_ns)
    zoom_level: Actor<f32>,                        // Zoom factor
    canvas_dimensions: Actor<(f32, f32)>,          // (width, height)
    mouse_position: Actor<(f32, f32)>,             // Current mouse coordinates
    hover_info: Actor<Option<HoverInfo>>,          // Mouse hover details
    
    /// Animation and rendering state
    animation_state: Actor<AnimationState>,
    render_requests: Actor<RenderRequestState>,
    signal_cache: Actor<HashMap<String, Vec<SignalTransition>>>,
    
    /// User interaction events - what the user DID
    cursor_clicked_relay: Relay<f64>,              // User clicked timeline at position
    cursor_dragged_relay: Relay<f64>,              // User dragged cursor
    mouse_moved_relay: Relay<(f32, f32)>,          // Mouse moved over canvas
    mouse_entered_relay: Relay,                    // Mouse entered canvas
    mouse_exited_relay: Relay,                     // Mouse left canvas
    zoom_wheel_scrolled_relay: Relay<f32>,         // User scrolled zoom wheel
    pan_started_relay: Relay<(f32, f32)>,          // User started pan drag  
    pan_moved_relay: Relay<(f32, f32)>,            // User continued pan drag
    pan_ended_relay: Relay,                        // User ended pan drag
    canvas_resized_relay: Relay<(f32, f32)>,       // Canvas size changed
    
    /// Keyboard navigation events - what keys were pressed
    left_arrow_pressed_relay: Relay,               // Left arrow navigation
    right_arrow_pressed_relay: Relay,              // Right arrow navigation
    home_key_pressed_relay: Relay,                 // Home key (go to start)
    end_key_pressed_relay: Relay,                  // End key (go to end)
    page_up_pressed_relay: Relay,                  // Page up navigation
    page_down_pressed_relay: Relay,                // Page down navigation
    plus_key_pressed_relay: Relay,                 // Plus key (zoom in)
    minus_key_pressed_relay: Relay,                // Minus key (zoom out)
    
    /// System events - what HAPPENED in the system
    data_loaded_relay: Relay<TimelineData>,        // Timeline data loaded
    viewport_changed_relay: Relay<(f64, f64)>,     // Visible range updated
    render_completed_relay: Relay<RenderResult>,   // Timeline render finished
    cache_updated_relay: Relay<(String, Vec<SignalTransition>)>, // Signal cache updated
    animation_frame_relay: Relay<f64>,             // Animation frame ready
    resize_detected_relay: Relay<(f32, f32)>,      // Canvas size detection
}

impl WaveformTimeline {
    pub fn new() -> Self {
        // Create all event streams
        let (cursor_clicked_relay, cursor_clicked_stream) = relay();
        let (cursor_dragged_relay, cursor_dragged_stream) = relay();
        let (mouse_moved_relay, mouse_moved_stream) = relay();
        let (zoom_wheel_scrolled_relay, zoom_stream) = relay();
        let (pan_started_relay, pan_started_stream) = relay();
        let (pan_moved_relay, pan_moved_stream) = relay();
        let (pan_ended_relay, pan_ended_stream) = relay();
        let (canvas_resized_relay, resize_stream) = relay();
        let (left_arrow_pressed_relay, left_arrow_stream) = relay();
        let (right_arrow_pressed_relay, right_arrow_stream) = relay();
        let (plus_key_pressed_relay, zoom_in_stream) = relay();
        let (minus_key_pressed_relay, zoom_out_stream) = relay();
        let (viewport_changed_relay, viewport_stream) = relay();
        
        // Create cursor position actor
        let cursor_position = Actor::new(0.0, async move |cursor| {
            loop {
                select! {
                    Some(new_pos) = cursor_clicked_stream.next() => {
                        cursor.set_neq(new_pos);
                        // Trigger cursor value updates
                        update_cursor_values_at_position(new_pos);
                    }
                    Some(new_pos) = cursor_dragged_stream.next() => {
                        cursor.set_neq(new_pos);
                        update_cursor_values_at_position(new_pos);
                    }
                    Some(()) = left_arrow_stream.next() => {
                        cursor.update(|pos| pos - CURSOR_STEP_NS);
                    }
                    Some(()) = right_arrow_stream.next() => {
                        cursor.update(|pos| pos + CURSOR_STEP_NS);
                    }
                }
            }
        });
        
        // Create visible range actor
        let visible_range = Actor::new((0.0, 1_000_000.0), async move |range| {
            loop {
                select! {
                    Some(zoom_delta) = zoom_stream.next() => {
                        range.update(|(start, end)| {
                            let center = (start + end) / 2.0;
                            let current_width = end - start;
                            let new_width = current_width * (1.0 + zoom_delta * 0.1);
                            let new_start = center - new_width / 2.0;
                            let new_end = center + new_width / 2.0;
                            (new_start.max(0.0), new_end)
                        });
                    }
                    Some((start_pos, end_pos)) = pan_moved_stream.next() => {
                        // Handle pan drag to update visible range
                        range.update(|(start, end)| {
                            let width = end - start;
                            let delta = (end_pos.0 - start_pos.0) as f64 * TIME_PER_PIXEL;
                            (start - delta, end - delta)
                        });
                    }
                    Some((new_start, new_end)) = viewport_stream.next() => {
                        range.set_neq((new_start, new_end));
                    }
                }
            }
        });
        
        // Create zoom level actor
        let zoom_level = Actor::new(1.0, async move |zoom| {
            loop {
                select! {
                    Some(()) = zoom_in_stream.next() => {
                        zoom.update(|level| (level * 1.2).min(100.0));
                    }
                    Some(()) = zoom_out_stream.next() => {
                        zoom.update(|level| (level * 0.8).max(0.01));
                    }
                }
            }
        });
        
        // Create canvas dimensions actor
        let canvas_dimensions = Actor::new((800.0, 400.0), async move |dims| {
            while let Some((width, height)) = resize_stream.next().await {
                dims.set_neq((width, height));
                // Trigger canvas redraw
                request_canvas_redraw();
            }
        });
        
        // Create mouse position tracker
        let mouse_position = Actor::new((0.0, 0.0), async move |pos| {
            while let Some((x, y)) = mouse_moved_stream.next().await {
                pos.set_neq((x, y));
                // Update hover info based on mouse position
                update_hover_info_at_position((x, y));
            }
        });
        
        // Create hover info actor
        let hover_info = Actor::new(None, async move |hover| {
            // Update hover info based on mouse position and timeline data
            // This would be connected to mouse movements and data updates
            loop {
                // Hover info updates would be processed here
                Task::next_macro_tick().await;
            }
        });
        
        // Create animation state actor
        let animation_state = Actor::new(AnimationState::default(), async move |anim| {
            // Handle animation frame updates
            loop {
                Task::next_macro_tick().await; 
            }
        });
        
        // Create render request coordinator
        let render_requests = Actor::new(RenderRequestState::default(), async move |requests| {
            // Coordinate canvas render requests
            loop {
                Task::next_macro_tick().await;
            }
        });
        
        // Create signal cache
        let signal_cache = Actor::new(HashMap::new(), async move |cache| {
            // Manage signal transition caching
            loop {
                Task::next_macro_tick().await;
            }
        });
        
        WaveformTimeline {
            cursor_position,
            visible_range,
            zoom_level,
            canvas_dimensions,
            mouse_position,
            hover_info,
            animation_state,
            render_requests,
            signal_cache,
            cursor_clicked_relay,
            cursor_dragged_relay,
            mouse_moved_relay,
            mouse_entered_relay: Relay::new(),
            mouse_exited_relay: Relay::new(),
            zoom_wheel_scrolled_relay,
            pan_started_relay,
            pan_moved_relay,
            pan_ended_relay,
            canvas_resized_relay,
            left_arrow_pressed_relay,
            right_arrow_pressed_relay,
            home_key_pressed_relay: Relay::new(),
            end_key_pressed_relay: Relay::new(),
            page_up_pressed_relay: Relay::new(),
            page_down_pressed_relay: Relay::new(),
            plus_key_pressed_relay,
            minus_key_pressed_relay,
            data_loaded_relay: Relay::new(),
            viewport_changed_relay,
            render_completed_relay: Relay::new(),
            cache_updated_relay: Relay::new(),
            animation_frame_relay: Relay::new(),
            resize_detected_relay: Relay::new(),
        }
    }
    
    /// Get reactive signal for cursor position
    pub fn cursor_position_signal(&self) -> impl Signal<Item = f64> {
        self.cursor_position.signal()
    }
    
    /// Get reactive signal for visible time range  
    pub fn visible_range_signal(&self) -> impl Signal<Item = (f64, f64)> {
        self.visible_range.signal()
    }
    
    /// Get reactive signal for current zoom level
    pub fn zoom_level_signal(&self) -> impl Signal<Item = f32> {
        self.zoom_level.signal()
    }
    
    /// Get reactive signal for canvas dimensions
    pub fn canvas_dimensions_signal(&self) -> impl Signal<Item = (f32, f32)> {
        self.canvas_dimensions.signal()
    }
}

#[derive(Clone, Debug, Default)]
struct AnimationState {
    is_animating: bool,
    last_frame_time: f64,
    frame_count: u64,
}

#[derive(Clone, Debug, Default)]
struct RenderRequestState {
    pending_requests: u32,
    last_render_time: f64,
    force_redraw: bool,
}
```

## Phase 4: Signal Data & Services Domain (5 → 1)

### Current Global Mutables
```rust
// 5 signal data service mutables
static VIEWPORT_SIGNALS: Lazy<MutableVec<Signal>> = lazy::default();
static CURSOR_VALUES: Lazy<MutableVec<CursorValue>> = lazy::default();
static ACTIVE_REQUESTS: Lazy<Mutable<HashMap<String, bool>>> = lazy::default();
static CACHE_STATISTICS: Lazy<Mutable<Option<CacheStats>>> = lazy::default();
static SIGNAL_REQUEST_TIMESTAMPS: Lazy<Mutable<HashMap<String, f64>>> = lazy::default();
```

### Target Domain Structure
```rust
/// Signal data management for timeline display and cursor values
/// Replaces signal service mutables with clean data representation
#[derive(Clone)]
pub struct SignalDataCache {
    /// Core data collections
    viewport_signals: ActorVec<SignalTransition>,      // Visible signal data
    cursor_values: Actor<HashMap<String, String>>,     // Current cursor values
    cache_statistics: Actor<CacheStats>,               // Performance metrics
    active_requests: Actor<HashMap<String, RequestInfo>>, // Request tracking
    
    /// Data request events - what was REQUESTED
    viewport_data_requested_relay: Relay<ViewportRequest>,    // Viewport data needed
    cursor_data_requested_relay: Relay<CursorRequest>,        // Cursor values needed
    cache_invalidated_relay: Relay<String>,                   // Signal ID to invalidate
    statistics_reset_relay: Relay,                            // Reset performance stats
    
    /// Data response events - what was RECEIVED
    viewport_data_received_relay: Relay<ViewportResponse>,    // Server sent viewport data
    cursor_data_received_relay: Relay<CursorResponse>,        // Server sent cursor data
    request_completed_relay: Relay<String>,                   // Request finished
    request_failed_relay: Relay<(String, String)>,           // Request failed with error
    timeout_reached_relay: Relay<String>,                     // Request timed out
}

impl SignalDataCache {
    pub fn new() -> Self {
        let (viewport_data_requested_relay, viewport_request_stream) = relay();
        let (cursor_data_requested_relay, cursor_request_stream) = relay();
        let (viewport_data_received_relay, viewport_response_stream) = relay();
        let (cursor_data_received_relay, cursor_response_stream) = relay();
        let (cache_invalidated_relay, cache_invalidated_stream) = relay();
        let (request_completed_relay, request_completed_stream) = relay();
        let (request_failed_relay, request_failed_stream) = relay();
        
        // Create viewport signals collection
        let viewport_signals = ActorVec::new(vec![], async move |signals_vec| {
            loop {
                select! {
                    Some(request) = viewport_request_stream.next() => {
                        // Handle viewport data request
                        send_viewport_request_to_backend(request);
                    }
                    Some(response) = viewport_response_stream.next() => {
                        // Update viewport signals with received data
                        match response {
                            ViewportResponse::Success { signals, .. } => {
                                signals_vec.lock_mut().replace_cloned(signals);
                            }
                            ViewportResponse::Error { error, .. } => {
                                zoon::println!("Viewport request failed: {error}");
                            }
                        }
                    }
                    Some(signal_id) = cache_invalidated_stream.next() => {
                        // Remove cached data for specific signal
                        signals_vec.lock_mut().retain(|s| s.signal_id != signal_id);
                    }
                }
            }
        });
        
        // Create cursor values map
        let cursor_values = Actor::new(HashMap::new(), async move |values| {
            loop {
                select! {
                    Some(request) = cursor_request_stream.next() => {
                        // Handle cursor data request
                        send_cursor_request_to_backend(request);
                    }
                    Some(response) = cursor_response_stream.next() => {
                        // Update cursor values with received data
                        match response {
                            CursorResponse::Success { values: new_values, .. } => {
                                values.update_mut(|map| {
                                    for (var_id, value) in new_values {
                                        map.insert(var_id, value);
                                    }
                                });
                            }
                            CursorResponse::Error { error, .. } => {
                                zoon::println!("Cursor request failed: {error}");
                            }
                        }
                    }
                }
            }
        });
        
        // Create cache statistics tracker
        let cache_statistics = Actor::new(CacheStats::default(), async move |stats| {
            loop {
                select! {
                    Some(request_id) = request_completed_stream.next() => {
                        stats.update_mut(|s| {
                            s.requests_completed += 1;
                            s.last_request_time = current_time_ms();
                        });
                    }
                    Some((request_id, error)) = request_failed_stream.next() => {
                        stats.update_mut(|s| {
                            s.requests_failed += 1;
                            s.last_error = Some(error);
                        });
                    }
                }
            }
        });
        
        // Create active requests tracker
        let active_requests = Actor::new(HashMap::new(), async move |requests| {
            // Track active requests and timeouts
            loop {
                Task::next_macro_tick().await;
                // Request tracking logic would go here
            }
        });
        
        SignalDataCache {
            viewport_signals,
            cursor_values,
            cache_statistics,
            active_requests,
            viewport_data_requested_relay,
            cursor_data_requested_relay,
            cache_invalidated_relay,
            statistics_reset_relay: Relay::new(),
            viewport_data_received_relay,
            cursor_data_received_relay,
            request_completed_relay,
            request_failed_relay,
            timeout_reached_relay: Relay::new(),
        }
    }
    
    /// Get reactive signal for viewport data
    pub fn viewport_signals_signal(&self) -> impl SignalVec<Item = SignalTransition> {
        self.viewport_signals.signal_vec_cloned()
    }
    
    /// Get reactive signal for cursor values
    pub fn cursor_values_signal(&self) -> impl Signal<Item = HashMap<String, String>> {
        self.cursor_values.signal()
    }
    
    /// Get reactive signal for specific cursor value
    pub fn cursor_value_signal(&self, var_id: String) -> impl Signal<Item = Option<String>> {
        self.cursor_values.signal_ref(move |values| values.get(&var_id).cloned())
    }
    
    /// Get reactive signal for cache statistics
    pub fn statistics_signal(&self) -> impl Signal<Item = CacheStats> {
        self.cache_statistics.signal()
    }
}
```

## Phase 5: Configuration Domain (6+ → 1)

### Current Global Mutables
```rust
// 6+ configuration mutables
static CONFIG_LOADED: Lazy<Mutable<bool>> = lazy::default();
static CONFIG_INITIALIZATION_COMPLETE: Lazy<Mutable<bool>> = lazy::default(); 
static SAVE_CONFIG_PENDING: Lazy<Mutable<bool>> = lazy::default();
static OPENED_FILES_FOR_CONFIG: Lazy<MutableVec<String>> = lazy::default();
static EXPANDED_SCOPES_FOR_CONFIG: Lazy<Mutable<HashSet<String>>> = lazy::default();
static SELECTED_VARIABLES_FOR_CONFIG: Lazy<MutableVec<SelectedVariable>> = lazy::default();
```

### Target Domain Structure
```rust
/// User's persistent configuration and workspace settings
/// Replaces CONFIG_* mutables with type-safe persistence
#[derive(Clone)]
pub struct UserConfiguration {
    /// Configuration state
    workspace_config: Actor<WorkspaceConfig>,
    load_state: Actor<ConfigLoadState>,
    save_requests: Actor<SaveRequestQueue>,
    
    /// Configuration change events - what CHANGED
    theme_changed_relay: Relay<Theme>,                      // Theme was switched
    dock_mode_changed_relay: Relay<DockMode>,               // Dock mode changed
    panel_resized_relay: Relay<(PanelId, f32, f32)>,       // Panel dimensions changed
    file_opened_relay: Relay<PathBuf>,                      // File added to recent files
    workspace_layout_changed_relay: Relay<WorkspaceLayout>, // Layout rearranged
    preferences_updated_relay: Relay<UserPreferences>,      // User preferences modified
    
    /// Configuration system events - what HAPPENED
    config_load_requested_relay: Relay<PathBuf>,            // Load config requested
    config_loaded_relay: Relay<WorkspaceConfig>,            // Config loaded from disk
    config_save_requested_relay: Relay,                     // Save triggered
    config_saved_relay: Relay<PathBuf>,                     // Config saved to disk
    config_error_relay: Relay<(ConfigOperation, String)>,  // Config operation failed
    auto_save_triggered_relay: Relay,                       // Auto-save timer fired
}

impl UserConfiguration {
    pub fn new() -> Self {
        let (theme_changed_relay, theme_changed_stream) = relay();
        let (dock_mode_changed_relay, dock_changed_stream) = relay();
        let (panel_resized_relay, panel_resized_stream) = relay();
        let (file_opened_relay, file_opened_stream) = relay();
        let (config_loaded_relay, config_loaded_stream) = relay();
        let (config_save_requested_relay, save_requested_stream) = relay();
        let (config_saved_relay, config_saved_stream) = relay();
        let (config_error_relay, config_error_stream) = relay();
        
        // Create workspace config actor
        let workspace_config = Actor::new(WorkspaceConfig::default(), async move |config| {
            loop {
                select! {
                    Some(new_theme) = theme_changed_stream.next() => {
                        config.update_mut(|cfg| cfg.theme = new_theme);
                        request_config_save();
                    }
                    Some(new_dock_mode) = dock_changed_stream.next() => {
                        config.update_mut(|cfg| cfg.dock_mode = new_dock_mode);
                        request_config_save();
                    }
                    Some((panel_id, width, height)) = panel_resized_stream.next() => {
                        config.update_mut(|cfg| {
                            cfg.panel_dimensions.insert(panel_id, PanelDimensions { width, height });
                        });
                        request_config_save_debounced(); // Debounce resize events
                    }
                    Some(file_path) = file_opened_stream.next() => {
                        config.update_mut(|cfg| {
                            cfg.recent_files.insert(0, file_path);
                            cfg.recent_files.truncate(MAX_RECENT_FILES);
                        });
                        request_config_save();
                    }
                    Some(loaded_config) = config_loaded_stream.next() => {
                        config.set_neq(loaded_config);
                    }
                }
            }
        });
        
        // Create load state tracker
        let load_state = Actor::new(ConfigLoadState::NotLoaded, async move |state| {
            loop {
                select! {
                    Some(loaded_config) = config_loaded_stream.next() => {
                        state.set_neq(ConfigLoadState::Loaded);
                    }
                    Some((operation, error)) = config_error_stream.next() => {
                        match operation {
                            ConfigOperation::Load => {
                                state.set_neq(ConfigLoadState::LoadFailed(error));
                            }
                            ConfigOperation::Save => {
                                // Keep current load state, just log save error
                                zoon::println!("Config save failed: {error}");
                            }
                        }
                    }
                }
            }
        });
        
        // Create save request queue
        let save_requests = Actor::new(SaveRequestQueue::new(), async move |queue| {
            loop {
                select! {
                    Some(()) = save_requested_stream.next() => {
                        queue.update_mut(|q| q.add_request(current_time_ms()));
                    }
                    Some(saved_path) = config_saved_stream.next() => {
                        queue.update_mut(|q| q.mark_completed(current_time_ms()));
                        zoon::println!("Config saved to: {saved_path:?}");
                    }
                }
            }
        });
        
        UserConfiguration {
            workspace_config,
            load_state,
            save_requests,
            theme_changed_relay,
            dock_mode_changed_relay,
            panel_resized_relay,
            file_opened_relay,
            workspace_layout_changed_relay: Relay::new(),
            preferences_updated_relay: Relay::new(),
            config_load_requested_relay: Relay::new(),
            config_loaded_relay,
            config_save_requested_relay,
            config_saved_relay,
            config_error_relay,
            auto_save_triggered_relay: Relay::new(),
        }
    }
    
    /// Get reactive signal for current config
    pub fn config_signal(&self) -> impl Signal<Item = WorkspaceConfig> {
        self.workspace_config.signal()
    }
    
    /// Get reactive signal for config load state
    pub fn load_state_signal(&self) -> impl Signal<Item = ConfigLoadState> {
        self.load_state.signal()
    }
    
    /// Check if config is loaded
    pub fn is_loaded_signal(&self) -> impl Signal<Item = bool> {
        self.load_state_signal().map(|state| matches!(state, ConfigLoadState::Loaded))
    }
}

#[derive(Clone, Debug)]
enum ConfigLoadState {
    NotLoaded,
    Loading,
    Loaded,
    LoadFailed(String),
}

#[derive(Clone, Debug)]
enum ConfigOperation {
    Load,
    Save,
}

#[derive(Clone, Debug, Default)]
struct SaveRequestQueue {
    pending_requests: u32,
    last_request_time: f64,
    last_save_time: f64,
}
```

## Phase 6: Local UI State Migration (Atom)

### Current Local UI Mutables
```rust
// 23+ local UI mutables → Atom in components
static FILES_PANEL_WIDTH: Lazy<Mutable<f32>> = lazy::default();
static FILES_PANEL_HEIGHT: Lazy<Mutable<f32>> = lazy::default();
static VARIABLES_NAME_COLUMN_WIDTH: Lazy<Mutable<f32>> = lazy::default();
static VARIABLES_VALUE_COLUMN_WIDTH: Lazy<Mutable<f32>> = lazy::default();
static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
static FILE_PATHS_INPUT: Lazy<Mutable<String>> = lazy::default();
static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();
static VARIABLES_SEARCH_INPUT_FOCUSED: Lazy<Mutable<bool>> = lazy::default();
// ... 15+ more local UI mutables
```

### Target Atom Usage
```rust
use crate::reactive_actors::Atom;

// Panel component state
struct FilesPanelState {
    width: Atom<f32>,
    height: Atom<f32>,
    is_collapsed: Atom<bool>,
    is_hovered: Atom<bool>,
    resize_dragging: Atom<bool>,
}

impl Default for FilesPanelState {
    fn default() -> Self {
        Self {
            width: Atom::new(300.0),
            height: Atom::new(400.0),
            is_collapsed: Atom::new(false),
            is_hovered: Atom::new(false),
            resize_dragging: Atom::new(false),
        }
    }
}

// Dialog component state
struct FileDialogState {
    is_open: Atom<bool>,
    filter_text: Atom<String>,
    selected_files: Atom<Vec<PathBuf>>,
    current_directory: Atom<PathBuf>,
    error_message: Atom<Option<String>>,
    is_loading: Atom<bool>,
}

// Search component state  
struct SearchState {
    filter_text: Atom<String>,
    is_focused: Atom<bool>,
    match_count: Atom<usize>,
    selected_index: Atom<Option<usize>>,
    search_history: Atom<Vec<String>>,
}

// Variables panel state
struct VariablesPanelState {
    name_column_width: Atom<f32>,
    value_column_width: Atom<f32>,
    format_column_width: Atom<f32>,
    search_state: SearchState,
    selection_mode: Atom<SelectionMode>,
    show_formats: Atom<bool>,
}

// Timeline canvas state
struct TimelineCanvasState {
    is_hovered: Atom<bool>,
    last_click_position: Atom<Option<(f32, f32)>>,
    drag_state: Atom<Option<DragState>>,
    context_menu_open: Atom<bool>,
    context_menu_position: Atom<(f32, f32)>,
}
```

## UI Integration Examples

### Event Emission in UI Components
```rust
// File drop area
fn file_drop_zone(tracked_files: &TrackedFiles) -> impl Element {
    El::new()
        .s(Background::new().color(neutral_3()))
        .s(Padding::new().all(20))
        .on_drop({
            let files_dropped_relay = tracked_files.files_dropped_relay.clone();
            move |dropped_files| {
                files_dropped_relay.send(dropped_files);
            }
        })
        .child(Text::new("Drop waveform files here"))
}

// Variable tree item with selection
fn variable_item(variable: &Variable, selected_variables: &SelectedVariables) -> impl Element {
    Row::new()
        .s(Padding::new().x(8).y(4))
        .s(Cursor::pointer())
        .on_click({
            let variable_clicked_relay = selected_variables.variable_clicked_relay.clone();
            let var_id = variable.unique_id.clone();
            move || variable_clicked_relay.send(var_id.clone())
        })
        .item(Text::new(&variable.name))
        .item(El::new().s(Width::fill())) // Spacer
        .item_signal(
            selected_variables.variables_signal()
                .map(move |vars| {
                    let var_id = variable.unique_id.clone();
                    if vars.iter().any(|v| v.unique_id == var_id) {
                        IconName::Check.into_element()
                    } else {
                        El::new().into_element()
                    }
                })
        )
}

// Timeline cursor interaction
fn timeline_canvas(waveform_timeline: &WaveformTimeline) -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Background::new().color(neutral_1()))
        .on_click({
            let cursor_clicked_relay = waveform_timeline.cursor_clicked_relay.clone();
            move |event: Click| {
                let cursor_time = pixel_to_time(event.x());
                cursor_clicked_relay.send(cursor_time);
            }
        })
        .on_mouse_move({
            let mouse_moved_relay = waveform_timeline.mouse_moved_relay.clone();
            move |event: MouseMove| {
                mouse_moved_relay.send((event.x(), event.y()));
            }
        })
        .child(canvas_content())
}

// Panel resize handle
fn resize_handle(panel_state: &FilesPanelState) -> impl Element {
    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new().color(neutral_6()))
        .s(Cursor::col_resize())
        .update_raw_el(|raw_el| {
            raw_el.event_handler({
                let resize_dragging = panel_state.resize_dragging.clone();
                let width = panel_state.width.clone();
                move |event: DragEvent| {
                    match event {
                        DragEvent::Start => resize_dragging.set(true),
                        DragEvent::Move { delta_x, .. } => {
                            width.update(|current| (current + delta_x).max(200.0).min(600.0));
                        }
                        DragEvent::End => resize_dragging.set(false),
                    }
                }
            });
        })
}
```

## Testing Examples

### Domain Actor Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[async_test]
    async fn test_tracked_files_workflow() {
        let tracked_files = TrackedFiles::new();
        
        // Test file drop event
        let paths = vec![PathBuf::from("test.vcd"), PathBuf::from("data.fst")];
        tracked_files.files_dropped_relay.send(paths);
        
        // Wait for files to be added
        let files = tracked_files.files_signal()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
        
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.path.file_name().unwrap() == "test.vcd"));
        assert!(files.iter().any(|f| f.path.file_name().unwrap() == "data.fst"));
        
        // Test parse completion
        let file_id = files[0].id.clone();
        tracked_files.parse_completed_relay.send((file_id, ParseResult::Success { 
            signals: vec![], 
            time_range: (0.0, 1000.0) 
        }));
        
        // Verify file state updated
        let file_state = files[0].state.signal().to_stream().next().await.unwrap();
        assert!(matches!(file_state, FileState::Parsed { .. }));
    }
    
    #[async_test]
    async fn test_variable_selection_workflow() {
        let selected_variables = SelectedVariables::new();
        
        // Test variable selection
        selected_variables.variable_clicked_relay.send("scope.signal_a".to_string());
        selected_variables.variable_clicked_relay.send("scope.signal_b".to_string());
        
        // Wait for selections
        let variables = selected_variables.variables_signal()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
            
        assert_eq!(variables.len(), 2);
        
        // Test scope expansion  
        selected_variables.scope_expanded_relay.send("scope".to_string());
        
        let expanded = selected_variables.expanded_scopes_signal()
            .to_stream()
            .next().await.unwrap();
            
        assert!(expanded.contains("scope"));
        
        // Test clear selection
        selected_variables.clear_selection_clicked_relay.send(());
        
        let final_variables = selected_variables.variables_signal()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
            
        assert_eq!(final_variables.len(), 0);
    }
}
```

### Atom Testing
```rust
#[cfg(test)]
mod ui_state_tests {
    use super::*;
    
    #[async_test]
    async fn test_panel_state() {
        let panel_state = FilesPanelState::default();
        
        // Test initial state
        let initial_width = panel_state.width.signal().to_stream().next().await.unwrap();
        assert_eq!(initial_width, 300.0);
        
        // Test width change
        panel_state.width.set(400.0);
        let new_width = panel_state.width.signal().to_stream().next().await.unwrap();
        assert_eq!(new_width, 400.0);
        
        // Test hover toggle
        panel_state.is_hovered.toggle();
        let hovered = panel_state.is_hovered.signal().to_stream().next().await.unwrap();
        assert_eq!(hovered, true);
    }
    
    #[async_test]
    async fn test_search_state() {
        let search_state = SearchState::default();
        
        // Test filter text
        search_state.filter_text.set("waveform".to_string());
        let filter = search_state.filter_text.signal().to_stream().next().await.unwrap();
        assert_eq!(filter, "waveform");
        
        // Test focus state
        search_state.is_focused.set(true);
        let focused = search_state.is_focused.signal().to_stream().next().await.unwrap();
        assert_eq!(focused, true);
    }
}
```

## Migration Summary & Benefits

### Quantitative Improvements
- **74+ global mutables → 5 domain structs + local Atom**
- **100% event-source relay naming compliance**
- **Zero Manager/Service/Controller abstractions**
- **Complete mutation traceability through event logs**
- **Elimination of all recursive lock potential**

### Architectural Benefits
1. **Domain-Driven Design**: Clear boundaries between file management, variable selection, timeline, data caching, and configuration
2. **Event-Source Traceability**: Every state change traceable to its source event
3. **Reactive Consistency**: All state access through signals, no .get() methods
4. **Testing Isolation**: Individual domain actors can be tested independently  
5. **Clean Separation**: UI state (Atom) vs shared state (Actor+Relay) clearly distinguished

### Development Experience Improvements
- **Self-Documenting Code**: Event names explain what happened
- **Easier Debugging**: Clear event flow through logs  
- **Better IDE Support**: Type safety and autocompletion for all events
- **Simpler Testing**: Signal-based testing with no timing issues
- **Maintainable Architecture**: Domain boundaries prevent cross-cutting concerns

This comprehensive migration transforms NovyWave from a collection of 74+ scattered global mutables into a clean, maintainable Actor+Relay architecture with proper domain modeling and event-source naming throughout.