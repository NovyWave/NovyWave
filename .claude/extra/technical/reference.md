# Technical Reference & Solutions

## WASM Development & Build Management

### Critical Build Rules
- **NEVER use `cargo build/check`** - Only mzoon handles WASM properly
- **NEVER restart dev server** without permission - compilation takes minutes
- Monitor: `makers start > dev_server.log 2>&1 &`
- Check: `tail -f dev_server.log` for build status
- Use: `makers kill` and `makers start` commands only

### WASM Logging
```rust
zoon::println!("Debug");  // ✅ Works in WASM
std::println!("Debug");   // ❌ Does nothing
```

### Browser MCP Limitations
- **F12 and F5 keys don't work** with browsermcp - use specialized tools instead
- **For console logs**: Use `mcp__browsermcp__browser_get_console_logs` tool
- **For refresh**: Use `mcp__browsermcp__browser_navigate` to same URL instead of F5
- **For DevTools**: Access logs programmatically rather than manual F12

## Core Component Patterns

### NovyUI Design System
```rust
// Icons: Always use enum tokens, never strings
button().left_icon(IconName::Folder)  // ✅ Never "folder" ❌

// Theme-aware colors
.s(Background::new().color_signal(neutral_3().signal()))
.s(Font::new().color_signal(neutral_11().signal()))

// Layout: Gap spacing, centering, spacers
Row::new().s(Gap::new().x(8)).s(Align::new().center_y())
  .item(title).item(El::new().s(Width::fill())).item(action)

// Height inheritance: Every container needs Height::fill()
El::new().s(Height::screen())  // Root
  .child(Column::new().s(Height::fill())  // ⚠️ Missing breaks chain
    .item(Row::new().s(Height::fill()).item(content)))

// TreeView with external state (Actor+Relay pattern)
TreeView::new()
  .external_expanded_signal(expanded_scopes().map(|scopes| scopes))
  .external_selected_vec_signal(selected_variables().map(|vars| vars.iter().map(|v| v.unique_id.clone()).collect()))
  .single_scope_selection(true)
```

## Configuration System

### Dual-Layer Architecture & Field Addition
```rust
// shared/lib.rs - Backend schema
pub struct WorkspaceSection {
    pub dock_mode: String,
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
    #[serde(default)]  // Always default for new fields
    pub selected_scope_id: Option<String>,
}

// frontend/config.rs - Extended structure
pub struct PanelLayouts {
    pub docked_to_right: Mutable<PanelDimensions>,
    pub docked_to_bottom: Mutable<PanelDimensions>,
}

// New fields require THREE locations: shared types, SerializableConfig, load_from_serializable()
```

### Reactive Persistence & Initialization Order
```rust
// Signal monitoring prevents overwrites with gate flag
pub fn initialize_config() -> impl Future<Output = ()> {
    async move {
        load_config().await;
        CONFIG_LOADED.set_neq(true);  // Gate prevents startup overwrites
        init_config_handlers();       // Start reactive triggers after load
    }
}

// Example handler with gate check
Task::start(current_theme().signal().for_each_sync(|_| {
    if CONFIG_LOADED.get() { save_current_config(); }
}));
```

## Theme System & Graphics

### Theme-Aware Signals & Color Tokens
```rust
// Reactive theme switching
.s(Background::new().color_signal(theme().signal().map(|t| match t {
    Theme::Light => neutral_1(), Theme::Dark => neutral_12(),
})))

// Color usage: neutral_11 (primary text), neutral_8 (dimmed), 
// neutral_1/3 (backgrounds), primary_6 (accents)

// Scrollbar theming with flattened signal chains
.style_signal("scrollbar-color", primary_6().signal().map(|thumb| 
    primary_3().signal().map(move |track| format!("{} {}", thumb, track))
).flatten())
```

### Fast2D Graphics Integration
```rust
// Canvas with shared access + signal-based updates (Actor+Relay pattern)
let canvas_wrapper = Rc::new(RefCell::new(canvas));
let canvas_clone = canvas_wrapper.clone();
Task::start(selected_variables().for_each_sync(move |_| {
    canvas_clone.borrow_mut().clear();
}));

// Theme-aware colors + resize handling
static BACKGROUND_DARK: Color = Color::from_rgba(13, 13, 13, 255);
static BACKGROUND_LIGHT: Color = Color::from_rgba(255, 255, 255, 255);
canvas.set_resize_callback(move |_w, _h| { /* signal updates */ });

// Timeline cursor: Convert page coords to canvas-relative
let relative_x = event.client_x() as f32 - canvas_rect.left();
let time = (relative_x / canvas_width) * total_time;

// Professional timeline: 80px spacing, nice numbers (1-2-5-10 scaling)
fn calculate_timeline_segments(width: f32, range: f64) -> Vec<f64> {
    let intervals = (width / 80.0) as usize;
    let nice_interval = round_to_nice_number(range / intervals as f64);
    (0..segments).map(|i| i as f64 * nice_interval).collect()
}
```

## Virtual Lists & File Management

### Virtual List Optimization
```rust
// Stable element pool - content/position updates only, never recreate DOM
let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(...);

// Velocity-based buffering: 5-15 elements (avoid 50+ causing slow rerendering)
let buffer = if velocity > 1000.0 { 15 } else if velocity > 500.0 { 10 } else { 5 };

// Dynamic height with flex constraints
El::new().s(Height::exact_signal(count.map(|c| (c * 40) as f32)))
  .update_raw_el(|el| el.style("min-height", "0"))  // Allow shrinking
```

### File System Architecture (Legacy)

**NOTE: This section describes legacy patterns. New code should use Actor+Relay TrackedFiles domain from the section above.**

```rust
// ❌ LEGACY: Dual state with manual synchronization
static FILE_PATHS: Lazy<MutableVec<String>> = Lazy::new(MutableVec::new);
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));

fn sync_globals_to_config() {
    let paths: Vec<String> = FILE_PATHS.lock_ref().to_vec();
    CONFIG_STORE.with(|store| {
        store.opened_files.set_neq(paths);
        save_config_to_backend();  // Manual trigger when reactive signals fail
    });
}

// ✅ STILL RELEVANT: Smart labeling algorithm (now used in TrackedFiles actor)
fn create_smart_labels(files: &[TrackedFile]) -> Vec<String> {
    files.iter().map(|file| {
        let filename = file.path.file_name().unwrap_or_default();
        let duplicates = files.iter().filter(|f| f.path.file_name() == Some(filename)).count();
        if duplicates > 1 {
            format!("{}/{}", parent_dir, filename)  // Disambiguate
        } else {
            filename.to_string()
        }
    }).collect()
}
```

**Migration to Actor+Relay:**
```rust
// Replace legacy file management with:
pub fn add_file(path: String) -> Relay<TrackedFile> { ... }
pub fn remove_file(file_id: String) -> Relay<()> { ... }
pub fn expand_scope(scope_id: String) -> Relay<()> { ... }

// Replace signal access with:
pub fn tracked_files() -> impl Signal<Item = Vec<TrackedFile>> { ... }
pub fn expanded_scopes() -> impl Signal<Item = HashSet<String>> { ... }
pub fn smart_labels() -> impl Signal<Item = Vec<String>> { ... }
```

## Signal Patterns & State Management

### Cache-First Signal Architecture

**Critical Pattern**: Always check existing cached data before making backend requests.

```rust
// ✅ CORRECT: Cache-first approach for cursor values
pub fn cursor_value_signal(signal_id: &str) -> impl Signal<Item = String> {
    map_ref! {
        let cursor_pos = TIMELINE_CURSOR_POSITION.signal(),
        let cache_signal = SIGNAL_TRANSITIONS_CACHE.signal_ref(|_| ()) => {
            // 1. Parse signal identifier
            let parts: Vec<&str> = signal_id.split('|').collect();
            
            // 2. Check timeline cache first (already loaded data)
            if let Some(cached_value) = compute_value_from_cached_transitions(
                file_path, scope_path, variable_name, *cursor_pos
            ) {
                match cached_value {
                    SignalValue::Present(data) => data,
                    SignalValue::Missing => "N/A".to_string(),
                }
            } else {
                // 3. Only check backend requests if no cache data
                if has_pending_backend_request(signal_id) {
                    "Loading...".to_string()
                } else {
                    "N/A".to_string()  // Should trigger backend query
                }
            }
        }
    }
}

// ❌ WRONG: Always making backend requests
pub fn cursor_value_signal_wrong(signal_id: &str) -> impl Signal<Item = String> {
    // Immediately makes backend request without checking existing data
    SignalDataService::request_signal_data(requests);  // Inefficient!
}
```

**Benefits of Cache-First**:
- ✅ Immediate response using already-loaded timeline data
- ✅ No backend communication delays or failures
- ✅ Efficient resource usage
- ✅ "Loading..." only when actually needed

### Common Signal Reactivity Issues

#### **Static vs Reactive Signals**
```rust
// ❌ WRONG: Static signal - never updates
.child_signal(map_ref! {
    let text = zoon::always(display_text.clone()),  // Static!
    let width = COLUMN_WIDTH.signal() => {
        format_text(text, width)
    }
})

// ✅ CORRECT: Reactive signal - updates when data changes  
.child_signal(map_ref! {
    let current_value = cursor_value_signal(&unique_id),  // Reactive!
    let format_state = selected_format.signal_cloned() => {
        format_signal_value(current_value, format_state)
    }
})
```

#### **SignalDataService Request Tracking**
```rust
// ❌ CRITICAL BUG: Clearing active requests breaks pending detection
pub fn clear_all_caches() {
    VIEWPORT_SIGNALS.lock_mut().clear();
    CURSOR_VALUES.lock_mut().clear();
    ACTIVE_REQUESTS.set_neq(HashMap::new()); // ❌ BREAKS LOADING STATES!
}

// ✅ CORRECT: Don't clear active requests - they track in-flight operations
pub fn clear_all_caches() {
    VIEWPORT_SIGNALS.lock_mut().clear();
    CURSOR_VALUES.lock_mut().clear();
    // DON'T clear ACTIVE_REQUESTS - those track pending backend requests
    CACHE_STATISTICS.set_neq(None);
}
```

### Timeline Data Reuse Patterns

**Core Principle**: Timeline already loads signal transitions for rendering - reuse this data for cursor values.

```rust
// Existing timeline cache (already populated)
pub static SIGNAL_TRANSITIONS_CACHE: Lazy<Mutable<HashMap<String, Vec<SignalTransition>>>> = ...;

// Reuse timeline data for cursor values
pub fn compute_value_from_cached_transitions(
    file_path: &str, scope_path: &str, variable_name: &str, time_seconds: f64
) -> Option<SignalValue> {
    let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    let cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
    
    if let Some(transitions) = cache.get(&cache_key) {
        // Find most recent transition at or before cursor time
        for transition in transitions.iter().rev() {
            if transition.time_seconds <= time_seconds {
                return Some(SignalValue::Present(transition.value.clone()));
            }
        }
    }
    None  // No cached data - may need backend query
}
```

### Debugging Signal Reactivity

#### **Console Logging for Signal Issues**
```rust
// Use zoon::println! for WASM logging
zoon::println!("🔄 Signal returning: {} for {}", value, signal_id);

// Check if signals are firing vs UI not updating
SIGNAL.signal().for_each_sync(|value| {
    zoon::println!("📡 Signal fired with: {}", value);  // Should see in console
});
```

#### **Common Symptoms & Fixes**
- **Console shows correct values, UI shows wrong values**: Reactive chain break - check for static signals
- **"Loading..." never resolves**: Backend communication issue or request tracking bug  
- **Values don't update on cursor move**: Missing signal dependencies in `map_ref!`
- **"N/A" instead of cached values**: Cache-first logic not implemented

### Signal Composition & Performance
```rust
// Type unification for dynamic switching
Stripe::new()
  .direction_signal(dock_mode.signal().map(|m| if m.is_docked() { Direction::Column } else { Direction::Row }))
  .item_signal(content_signal.map(|c| match c {
    ContentType::A => element_a().into_element(),  // ⚠️ .into_element() required
    ContentType::B => element_b().into_element(),
  }))

// Performance: dedupe + conditional gates
TIMELINE_CURSOR_POSITION.signal().dedupe().for_each_sync(|pos| expensive_update(pos));
if CONFIG_LOADED.get() { perform_config_operation(); }  // Prevent startup races

// ❌ LEGACY: Collection patterns (use Actor+Relay instead)
static ITEMS: Lazy<MutableVec<SelectedVariable>> = Lazy::new(MutableVec::new);
static EXPANDED: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));
static SORTED: Lazy<MutableBTreeMap<String, Data>> = Lazy::new(MutableBTreeMap::new);  // Ordered reactive

// ✅ MODERN: Use Actor+Relay domain signals instead
selected_variables().map(|vars| render_variables(vars))
expanded_scopes().map(|scopes| render_expanded_state(scopes))
```

## Debouncing & Task Management

### True Debouncing with TaskHandle
```rust
// ❌ Task::start().cancel() - NOT guaranteed (tasks may still complete)
// ✅ Task::start_droppable with TaskHandle dropping - guaranteed abortion

let debounce_task: Mutable<Option<TaskHandle<()>>> = Mutable::new(None);
signal.for_each_sync(move |_| {
    debounce_task.set(None); // Drop immediately aborts task
    let handle = Task::start_droppable(async {
        Timer::sleep(1000).await; // Only completes if not dropped
        perform_operation();
    });
    debounce_task.set(Some(handle));
});

// Config save debouncing with multi-level strategy
pub fn save_config_to_backend() {
    if !SAVE_CONFIG_PENDING.get() {
        SAVE_CONFIG_PENDING.set_neq(true);
        Task::start(async {
            Timer::sleep(1000).await; // Global 1s debounce
            save_config_immediately();
            SAVE_CONFIG_PENDING.set_neq(false);
        });
    }
}

// Different urgencies: Immediate (critical UI), Normal (preferences), Deferred (navigation)
```

### Cache-First Timeline Patterns
```rust
// Single unified handler prevents redundant processing
pub fn trigger_unified_query_logic() {
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let start = TIMELINE_VISIBLE_RANGE_START.get() as f64;
    let end = TIMELINE_VISIBLE_RANGE_END.get() as f64;
    
    if cursor_pos >= start && cursor_pos <= end {
        // Fast path - cached transitions with 20% buffer range expansion
        let transitions_cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
        let mut new_values = SIGNAL_VALUES.get_cloned();
        let mut hits = 0;
        
        for var in SELECTED_VARIABLES.lock_ref().iter() {
            if let Some(transitions) = transitions_cache.get(&var.unique_id) {
                for transition in transitions.iter().rev() {
                    if transition.time_seconds <= cursor_pos {
                        new_values.insert(var.unique_id.clone(), SignalValue::from_data(transition.value.clone()));
                        hits += 1;
                        break;
                    }
                }
            }
        }
        if hits > 0 { SIGNAL_VALUES.set(new_values); }
    } else {
        // Slow path - server query outside cached range
        query_signal_values_at_time(cursor_pos);
    }
}

// Range buffering for better cache utilization
pub fn get_current_timeline_range() -> Option<(f32, f32)> {
    let time_range = max_time - min_time;
    let buffer = time_range * 0.2; // 20% buffer each side
    ((min_time - buffer).max(0.0), max_time + buffer)
}
```

## Actor+Relay Architecture Reference

### Core Implementation Patterns

**MANDATORY EVENT-SOURCE RELAY NAMING**: Relay functions MUST be named after the event/action they represent, never after the domain they manage.

```rust
// ✅ CORRECT: Event-source naming
pub fn add_file(path: String) -> Relay<TrackedFile> { ... }
pub fn remove_file(file_id: String) -> Relay<()> { ... }
pub fn expand_scope(scope_id: String) -> Relay<()> { ... }
pub fn select_variables(variables: Vec<String>) -> Relay<()> { ... }

// ❌ WRONG: Manager/Service/Controller naming (enterprise antipattern)
pub fn file_manager() -> Relay<()> { ... }
pub fn variable_service() -> Relay<()> { ... }
pub fn scope_controller() -> Relay<()> { ... }
```

**Actor Creation Pattern:**
```rust
use actor_relay::{Actor, relay, Atom};

#[derive(Default)]
struct TrackedFiles {
    files: Vec<TrackedFile>,
    expanded_scopes: HashSet<String>,
    smart_labels: Vec<String>,
}

impl Actor for TrackedFiles {
    type Event = TrackedFilesEvent;
    
    fn apply(&mut self, event: Self::Event) -> Vec<TrackedFile> {
        match event {
            TrackedFilesEvent::AddFile { file } => {
                self.files.push(file.clone());
                self.update_smart_labels();
                vec![file]
            },
            TrackedFilesEvent::RemoveFile { file_id } => {
                self.files.retain(|f| f.id != file_id);
                self.update_smart_labels();
                vec![]
            },
            TrackedFilesEvent::ExpandScope { scope_id } => {
                self.expanded_scopes.insert(scope_id);
                vec![]
            },
        }
    }
}

// Event enum with domain-specific operations
#[derive(Clone, Debug)]
enum TrackedFilesEvent {
    AddFile { file: TrackedFile },
    RemoveFile { file_id: String },
    ExpandScope { scope_id: String },
    UpdateSmartLabels,
}
```

**Public Relay API Functions:**
```rust
// Event-source relay functions (public API)
pub fn add_file(path: String) -> Relay<TrackedFile> {
    relay(TrackedFilesEvent::AddFile { 
        file: TrackedFile::from_path(path) 
    })
}

pub fn remove_file(file_id: String) -> Relay<()> {
    relay(TrackedFilesEvent::RemoveFile { file_id })
}

pub fn expand_scope(scope_id: String) -> Relay<()> {
    relay(TrackedFilesEvent::ExpandScope { scope_id })
}

// Signal access (reactive reads)
pub fn tracked_files() -> impl Signal<Item = Vec<TrackedFile>> {
    TrackedFiles::signal().map(|state| state.files.clone())
}

pub fn expanded_scopes() -> impl Signal<Item = HashSet<String>> {
    TrackedFiles::signal().map(|state| state.expanded_scopes.clone())
}

pub fn smart_labels() -> impl Signal<Item = Vec<String>> {
    TrackedFiles::signal().map(|state| state.smart_labels.clone())
}
```

### Domain Modeling Patterns

**TrackedFiles Domain:**
```rust
#[derive(Default)]
struct TrackedFiles {
    files: Vec<TrackedFile>,
    expanded_scopes: HashSet<String>,
    smart_labels: Vec<String>,
    loading_states: HashMap<String, LoadingStatus>,
}

impl TrackedFiles {
    fn update_smart_labels(&mut self) {
        self.smart_labels = create_smart_labels(&self.files);
    }
    
    fn mark_file_loading(&mut self, file_id: &str) {
        self.loading_states.insert(file_id.to_string(), LoadingStatus::Loading);
    }
    
    fn mark_file_loaded(&mut self, file_id: &str, success: bool) {
        let status = if success { LoadingStatus::Loaded } else { LoadingStatus::Failed };
        self.loading_states.insert(file_id.to_string(), status);
    }
}

// Event-source relay functions for file operations
pub fn load_waveform_file(path: String) -> Relay<LoadingFile> {
    relay(TrackedFilesEvent::LoadFile { path })
}

pub fn batch_add_files(paths: Vec<String>) -> Relay<Vec<TrackedFile>> {
    relay(TrackedFilesEvent::BatchAdd { paths })
}
```

**SelectedVariables Domain:**
```rust
#[derive(Default)]
struct SelectedVariables {
    variables: Vec<SelectedVariable>,
    selection_order: Vec<String>,
    filters: VariableFilters,
}

impl Actor for SelectedVariables {
    type Event = SelectedVariablesEvent;
    
    fn apply(&mut self, event: Self::Event) -> Vec<SelectedVariable> {
        match event {
            SelectedVariablesEvent::AddVariable { variable } => {
                if !self.variables.iter().any(|v| v.unique_id == variable.unique_id) {
                    self.selection_order.push(variable.unique_id.clone());
                    self.variables.push(variable.clone());
                    vec![variable]
                } else {
                    vec![]
                }
            },
            SelectedVariablesEvent::RemoveVariable { variable_id } => {
                self.variables.retain(|v| v.unique_id != variable_id);
                self.selection_order.retain(|id| id != &variable_id);
                vec![]
            },
            SelectedVariablesEvent::ReorderVariables { new_order } => {
                self.selection_order = new_order;
                // Reorder variables vector to match
                let mut reordered = Vec::new();
                for id in &self.selection_order {
                    if let Some(var) = self.variables.iter().find(|v| &v.unique_id == id) {
                        reordered.push(var.clone());
                    }
                }
                self.variables = reordered;
                self.variables.clone()
            },
        }
    }
}

// Event-source relay functions
pub fn add_selected_variable(variable: SelectedVariable) -> Relay<SelectedVariable> {
    relay(SelectedVariablesEvent::AddVariable { variable })
}

pub fn remove_selected_variable(variable_id: String) -> Relay<()> {
    relay(SelectedVariablesEvent::RemoveVariable { variable_id })
}

pub fn reorder_selected_variables(new_order: Vec<String>) -> Relay<()> {
    relay(SelectedVariablesEvent::ReorderVariables { new_order })
}
```

**WaveformTimeline Domain:**
```rust
#[derive(Default)]
struct WaveformTimeline {
    cursor_position: f64,
    visible_range: (f64, f64),
    zoom_level: f64,
    signal_transitions_cache: HashMap<String, Vec<SignalTransition>>,
    cursor_values: HashMap<String, SignalValue>,
}

impl Actor for WaveformTimeline {
    type Event = TimelineEvent;
    
    fn apply(&mut self, event: Self::Event) -> TimelineUpdate {
        match event {
            TimelineEvent::SetCursorPosition { time_seconds } => {
                self.cursor_position = time_seconds;
                self.update_cursor_values();
                TimelineUpdate::CursorMoved { position: time_seconds }
            },
            TimelineEvent::SetVisibleRange { start, end } => {
                self.visible_range = (start, end);
                TimelineUpdate::RangeChanged { start, end }
            },
            TimelineEvent::CacheSignalTransitions { signal_id, transitions } => {
                self.signal_transitions_cache.insert(signal_id, transitions);
                self.update_cursor_values();
                TimelineUpdate::CacheUpdated
            },
        }
    }
}

// Event-source relay functions
pub fn set_cursor_position(time_seconds: f64) -> Relay<TimelineUpdate> {
    relay(TimelineEvent::SetCursorPosition { time_seconds })
}

pub fn zoom_to_range(start: f64, end: f64) -> Relay<TimelineUpdate> {
    relay(TimelineEvent::SetVisibleRange { start, end })
}

pub fn cache_signal_data(signal_id: String, transitions: Vec<SignalTransition>) -> Relay<()> {
    relay(TimelineEvent::CacheSignalTransitions { signal_id, transitions })
}
```

### Atom Usage

**Local UI State (Replacing Mutables):**
```rust
// ✅ CORRECT: Atom for local UI state
#[derive(Default)]
struct DialogState {
    is_open: bool,
    selected_filter: String,
    search_text: String,
}

impl Atom for DialogState {}

// Usage in components
fn file_dialog() -> impl Element {
    Column::new()
        .item_signal(DialogState::signal().map(|state| {
            if state.is_open {
                dialog_content().into_element()
            } else {
                empty_element().into_element()
            }
        }))
}

// Event-source functions for UI state
pub fn open_file_dialog() -> Relay<()> {
    DialogState::update(|state| state.is_open = true)
}

pub fn close_file_dialog() -> Relay<()> {
    DialogState::update(|state| state.is_open = false)
}

pub fn set_dialog_filter(filter: String) -> Relay<()> {
    DialogState::update(|state| state.selected_filter = filter)
}
```

**Panel Layout State:**
```rust
#[derive(Default)]
struct PanelLayoutState {
    dock_mode: DockMode,
    right_panel_width: f32,
    bottom_panel_height: f32,
    is_files_panel_visible: bool,
    is_variables_panel_visible: bool,
}

impl Atom for PanelLayoutState {}

// Event-source functions for layout
pub fn switch_dock_mode(mode: DockMode) -> Relay<()> {
    PanelLayoutState::update(|state| state.dock_mode = mode)
}

pub fn resize_right_panel(width: f32) -> Relay<()> {
    PanelLayoutState::update(|state| state.right_panel_width = width)
}

pub fn toggle_files_panel() -> Relay<()> {
    PanelLayoutState::update(|state| state.is_files_panel_visible = !state.is_files_panel_visible)
}
```

### Migration Patterns

**From Global Mutables to Domain Actors:**
```rust
// ❌ OLD: Global mutable state
static TRACKED_FILES: Lazy<MutableVec<TrackedFile>> = Lazy::new(MutableVec::new);
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));
static SMART_LABELS: Lazy<Mutable<Vec<String>>> = Lazy::new(|| Mutable::new(Vec::new()));

// ✅ NEW: Domain Actor with cohesive state
#[derive(Default)]
struct TrackedFiles {
    files: Vec<TrackedFile>,
    expanded_scopes: HashSet<String>,
    smart_labels: Vec<String>,
}
```

**Migration Steps:**
1. **Group related state** into domain actors
2. **Convert .get()/.set() calls** to relay events
3. **Replace signal access** with domain signals
4. **Eliminate manual state synchronization** (Actor handles it)
5. **Remove global statics** once migration is complete

**Before/After Example:**
```rust
// ❌ OLD: Manual state management
pub fn add_file(path: String) {
    let file = TrackedFile::from_path(path);
    TRACKED_FILES.lock_mut().push_cloned(file);
    
    // Manual synchronization required
    let files = TRACKED_FILES.lock_ref().to_vec();
    let labels = create_smart_labels(&files);
    SMART_LABELS.set_neq(labels);
}

// ✅ NEW: Actor handles synchronization
pub fn add_file(path: String) -> Relay<TrackedFile> {
    relay(TrackedFilesEvent::AddFile { 
        file: TrackedFile::from_path(path) 
    })
    // Smart labels automatically updated in Actor::apply
}
```

### Testing Patterns

**Signal-Based Testing (No .get() Methods):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_add_file_updates_smart_labels() {
        // Setup
        let initial_files = tracked_files().first().await;
        assert_eq!(initial_files.len(), 0);
        
        // Action
        add_file("test.vcd".to_string()).await;
        
        // Verify through signals
        let files = tracked_files().first().await;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.vcd");
        
        let labels = smart_labels().first().await;
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "test.vcd");
    }
    
    #[tokio::test]
    async fn test_batch_file_loading() {
        let paths = vec!["file1.vcd".to_string(), "file2.vcd".to_string()];
        
        batch_add_files(paths.clone()).await;
        
        let files = tracked_files().first().await;
        assert_eq!(files.len(), 2);
        
        // Test smart labeling with duplicates
        add_file("subdir/file1.vcd".to_string()).await;
        
        let labels = smart_labels().first().await;
        assert_eq!(labels.len(), 3);
        // Should disambiguate: "file1.vcd", "file2.vcd", "subdir/file1.vcd"
        assert!(labels.contains(&"subdir/file1.vcd".to_string()));
    }
    
    #[tokio::test]
    async fn test_timeline_cursor_updates() {
        // Setup timeline data
        let transitions = vec![
            SignalTransition { time_seconds: 1.0, value: "0".to_string() },
            SignalTransition { time_seconds: 2.0, value: "1".to_string() },
        ];
        cache_signal_data("signal1".to_string(), transitions).await;
        
        // Test cursor position
        set_cursor_position(1.5).await;
        
        let cursor_pos = cursor_position().first().await;
        assert_eq!(cursor_pos, 1.5);
        
        let values = cursor_values().first().await;
        assert_eq!(values.get("signal1"), Some(&"0".to_string()));
    }
}
```

**Integration Testing:**
```rust
#[tokio::test]
async fn test_file_loading_workflow() {
    // Test complete workflow from file selection to variable display
    
    // 1. Load files
    let paths = vec!["test1.vcd".to_string(), "test2.vcd".to_string()];
    batch_add_files(paths).await;
    
    // 2. Expand scopes
    expand_scope("scope1".to_string()).await;
    
    // 3. Select variables
    let variable = SelectedVariable {
        unique_id: "test1.vcd|scope1|signal1".to_string(),
        file_path: "test1.vcd".to_string(),
        scope_path: "scope1".to_string(),
        variable_name: "signal1".to_string(),
    };
    add_selected_variable(variable).await;
    
    // 4. Verify integrated state
    let files = tracked_files().first().await;
    let scopes = expanded_scopes().first().await;
    let variables = selected_variables().first().await;
    
    assert_eq!(files.len(), 2);
    assert!(scopes.contains("scope1"));
    assert_eq!(variables.len(), 1);
    assert_eq!(variables[0].unique_id, "test1.vcd|scope1|signal1");
}
```

### Troubleshooting Guide

**Common Issues:**

1. **Event-Source Naming Violations:**
```rust
// ❌ WRONG: Manager naming
pub fn file_manager() -> Relay<()> { ... }

// ✅ CORRECT: Event naming
pub fn add_file(path: String) -> Relay<TrackedFile> { ... }
```

2. **Enterprise Pattern Violations:**
```rust
// ❌ WRONG: Service/Controller patterns
struct FileService;
struct VariableController;

// ✅ CORRECT: Domain actors
struct TrackedFiles;
struct SelectedVariables;
```

3. **Missing Signal Dependencies:**
```rust
// ❌ WRONG: Static data in reactive context
.child_signal(always(some_data).map(|data| render(data)))

// ✅ CORRECT: Reactive signal chain
.child_signal(tracked_files().map(|files| render_files(files)))
```

4. **Improper State Access:**
```rust
// ❌ WRONG: Direct state access (testing anti-pattern)
assert_eq!(TrackedFiles::get().files.len(), 1);  // No .get() method

// ✅ CORRECT: Signal-based access
let files = tracked_files().first().await;
assert_eq!(files.len(), 1);
```

5. **Mixed State Management:**
```rust
// ❌ WRONG: Mixing Mutables with Actors
static OLD_FILES: Lazy<MutableVec<File>> = ...;  // Don't mix patterns

// ✅ CORRECT: Pure Actor approach
// All file state goes through TrackedFiles actor
```

### Performance Considerations

**Event Emission Patterns:**
- Actors automatically batch related updates
- Only emit events when state actually changes
- Derived computations (like smart labels) happen once per event
- No manual synchronization between related state pieces

**Signal Chain Optimization:**
```rust
// ✅ EFFICIENT: Direct actor signal
tracked_files().map(|files| render_file_list(files))

// ❌ INEFFICIENT: Multiple signal sources
map_ref! {
    let files = TRACKED_FILES.signal_vec_cloned().to_signal_cloned(),
    let labels = SMART_LABELS.signal() => {
        combine_files_and_labels(files, labels)  // Manual synchronization
    }
}
```

**Memory Management:**
- Actors own their complete domain state
- No circular references between domain actors
- Atom for ephemeral UI state
- Automatic cleanup when actors go out of scope

## Lock Management & Actor Model (Legacy)

**NOTE: This section describes the old Actor Model pattern. New code should use Actor+Relay architecture above.**

### Recursive Lock Prevention
```rust
// Problem: for_each_sync runs while locks held → recursive lock panic
// Solution: Use async handlers to defer execution until locks drop

// ❌ Synchronous - runs immediately while parent lock held
COLLECTION.signal_vec_cloned().for_each_sync(|data| {
    COLLECTION.lock_mut().update(); // RECURSIVE LOCK PANIC!
});

// ✅ Async - naturally deferred until after locks drop
COLLECTION.signal_vec_cloned().for_each(|data| async move {
    send_message_that_modifies_collection(data); // Safe through actor model
}).await;
```

### Legacy Actor Implementation
```rust
// Core principle: All state mutations through single sequential processor
#[derive(Debug, Clone)]
pub enum StateMessage {
    Add { item: ItemType }, Update { id: String, field: ValueType }, Remove { id: String },
}

// Single message processor with event loop yielding
async fn process_state_message(message: StateMessage) {
    match message {
        StateMessage::Add { item } => SHARED_STATE.lock_mut().push_cloned(item),
        StateMessage::Update { id, field } => { /* find and update */ },
    }
}

Task::start(async {
    loop {
        let messages = take_pending_messages();
        for message in messages {
            Task::next_macro_tick().await; // ⚠️ CRITICAL: Yield between messages
            process_state_message(message).await;
        }
    }
});

// Benefits: Eliminates concurrent access, prevents recursive locks, predictable execution
// Use for: Shared state, complex interdependencies, reactive updates
```

### Legacy Mutable Patterns
```rust
// Mutable types are Arc<RwLock<T>> internally - cheaply cloneable, never wrap in Arc again
// Basic chains: Mutable<T> -> signal() -> map() -> for_each_sync()

// Nested Mutables for lock-free individual updates
static ITEMS: Lazy<MutableVec<Mutable<ItemType>>> = lazy::default();
fn update_item(id: &str, data: ItemType) {
    let items = ITEMS.lock_ref();
    if let Some(item) = items.iter().find(|i| i.lock_ref().id == id) {
        item.set(data);  // No parent lock needed
    }
}

// Derived signals - automatic updates without manual refresh
static COMPUTED: Lazy<Mutable<ComputedType>> = Lazy::new(|| {
    let computed = Mutable::new(ComputedType::default());
    Task::start(PARENT.signal_vec_cloned().for_each_sync(move |items| {
        computed.set(compute(&items));
    }));
    computed
});

// Lock timing: Use explicit scopes {} to drop locks early
let updated_items = { 
    let items = ITEMS.lock_ref(); 
    let mut new_items = items.clone(); 
    new_items[0] = new_item; 
    new_items 
}; // Lock dropped here
ITEMS.lock_mut().replace_cloned(updated_items); // Signals fire safely
```

### Critical Rules (Both Patterns)
1. Never hold locks across await points
2. Drop locks before triggering signals  
3. Use async signal handlers over for_each_sync
4. Prefer domain actors over global mutables (new code)
5. Use event-source relay naming (new code)

## Dock Mode & Performance

### Dock Mode Architecture
```rust
// Per-dock-mode storage with preserved dimensions
#[derive(Clone)]
pub enum DockMode { Right, Bottom, }

pub struct WorkspaceSection {
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
}

// Layout switching with dimension preservation
fn main_layout() -> impl Element {
    El::new().child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|docked| {
        if docked { docked_layout().into_element() } else { undocked_layout().into_element() }
    }))
}

fn switch_dock_mode_preserving_dimensions() {
    let current_dims = get_current_panel_dimensions();
    IS_DOCKED_TO_BOTTOM.set_neq(!IS_DOCKED_TO_BOTTOM.get());
    save_panel_dimensions_for_current_mode(current_dims);
}
```

### Performance Optimizations
```rust
// Signal chain with deduplication
TIMELINE_CURSOR_POSITION.signal().dedupe().for_each_sync(|pos| expensive_update(pos));

// Parallel directory traversal - 4x improvement
use jwalk::WalkDir;
WalkDir::new(path).parallelism(jwalk::Parallelism::RayonNewPool(4))
    .into_iter().filter_map(|e| e.ok()).collect()

// Unicode text filtering for invisible characters
use unicode_width::UnicodeWidthChar;
let clean: String = text.chars()
    .filter(|&c| c == ' ' || UnicodeWidthChar::width(c).unwrap_or(0) > 0)
    .collect();
```

## Memory Management & Troubleshooting

### WASM Integration
```rust
// DOM element access + modern clipboard with fallback
use wasm_bindgen::JsCast;
let canvas_element = event.target().dyn_cast::<web_sys::Element>()
    .expect("Event target is not an element");

async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    if let Some(clipboard) = window.navigator().clipboard() {
        clipboard.write_text(text).await  // Modern API
    } else {
        /* execCommand fallback */  Ok(())
    }
}

// Thread-blocking library integration
let result = tokio::spawn_blocking(move || expensive_blocking_operation(data)).await?;
```

### Common Issues & Fixes

#### Compilation Issues
- **WASM changes not visible**: Check `tail -100 dev_server.log | grep -i "error"` first
- **cargo vs mzoon differences**: Only trust mzoon output for WASM build status
- **IconName errors**: Always use enum tokens: `button().left_icon(IconName::Check)` 
- **Signal type mismatches**: Use `.into_element()` for type unification in match arms

#### Layout Problems
- **Height inheritance breaks**: Every container needs `Height::fill()` in the chain
- **TreeView width issues**: Multi-level constraints - container `min-width: max-content`, item `width: 100%`
- **Scrolling issues**: Add `min-height: 0` to parent containers to allow flex shrinking
- **Dropdown height**: Filter invisible characters with `UnicodeWidthChar::width()`

#### Event & Memory Issues  
- **Event bubbling**: Use `event.pass_to_parent(false)` to prevent propagation
- **Canvas coordinates**: Convert page coords with `event.client_x() - canvas_rect.left()`
- **Modal keyboard**: Use global handlers with state guards: `if DIALOG_IS_OPEN.get()`
- **Config races**: Load config first, then `CONFIG_LOADED.set_neq(true)` gate flag
- **Storage limits**: Use separate log files for data >2KB to avoid session storage issues

#### Performance Fixes
- **Virtual list blanks**: Use stable element pools, update content only, never recreate DOM
- **Directory scanning**: `jwalk::WalkDir` with `.parallelism(RayonNewPool(4))` for 4x improvement
- **Debug spam**: `rg "println!" --type rust | wc -l` to count and remove excessive logging
- **TreeView flickering**: Signal cascades causing 30+ renders - remove intermediate signals, add `.dedupe_cloned()`
- **Duplicate service calls**: Multiple handlers for same signal - use mutually exclusive conditions
- **Config restoration timing**: UI before sync - add immediate sync pattern: `derived.set_neq(current_state)`

#### Persistence Issues
- **Signal chain breaks**: Manual `save_config_to_backend()` trigger when reactive fails
- **Dock mode overwrites**: Separate `panel_dimensions_right/bottom` instead of semantic overloading
- **Scope selection lost**: Add fields to both `shared/lib.rs` and frontend for backend sync

#### Reactive Issues & Debugging
- **Broken signal dependencies**: When UI shows "Loading..." instead of data, check if signal actually updates when data changes
- **Never-triggered signals**: Signals defined but never set break reactive chains silently (e.g. `FILE_LOADING_TRIGGER`)
- **Working pattern for file dependencies**: Use `TRACKED_FILES.signal_vec_cloned().to_signal_cloned()` instead of custom triggers
- **Debug method**: Compare working vs broken panels - identify signal chain differences between them
- **Infinite rendering loops**: Check for circular signal dependencies, excessive console logging
- **Missing UI updates**: Add missing signal dependencies (`_tracked_files` pattern)
- **Integer overflow panics**: Use `saturating_sub()` instead of `-` for counts
- **Checkbox state sync**: Use `label_signal` for dynamic checkbox recreation
- **Initialization timing**: Use one-shot config loading, preserve existing states
- **Signal type errors**: Convert `SignalVec` with `.to_signal_cloned()` for `map_ref!`
- **Loop detection**: Add render counters, look for bidirectional reactive flows
- **State preservation**: Check existing states before replacing during updates
- **FusedFuture compilation errors**: Actor stream processing with `futures::select!` requires `.fuse()` on streams, not `tokio::select!`

#### Actor Stream Processing Patterns

**✅ CORRECT: FusedFuture-compatible stream selection**
```rust
let panel_dimensions_right_actor = Actor::new(PanelDimensions::default(), async move |state| {
    let mut right_stream = panel_dimensions_right_changed_stream.fuse();
    let mut resized_stream = panel_resized_stream.fuse();
    
    loop {
        select! {
            new_dims = right_stream.next() => {
                if let Some(dims) = new_dims {
                    state.set_neq(dims);
                }
            }
            resized_dims = resized_stream.next() => {
                if let Some(dims) = resized_dims {
                    state.set_neq(dims);
                }
            }
        }
    }
});
```

**❌ WRONG: tokio::select! causes compilation errors**
```rust
// ERROR: tokio::select! not available in WASM environment
tokio::select! {
    new_dims = right_stream.next() => { ... }
}

// ERROR: futures::select! requires FusedStream trait
futures::select! {
    new_dims = right_stream.next() => { ... }  // Stream doesn't implement FusedStream
}
```

**Key Requirements:**
- Use `futures::{StreamExt, select}` imports
- Call `.fuse()` on all streams before using in `select!`
- Use plain `select!` macro, not `tokio::select!` or `futures::select!`
- Pattern works in both WASM and native environments

For comprehensive reactive patterns, see: `.claude/extra/technical/reactive-patterns.md`