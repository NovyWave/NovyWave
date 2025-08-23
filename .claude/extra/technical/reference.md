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

// TreeView with external state
TreeView::new()
  .external_expanded_signal(EXPANDED_DIRS.signal())
  .external_selected_vec_signal(SELECTED_ITEMS.signal_vec_cloned())
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
// Canvas with shared access + signal-based updates
let canvas_wrapper = Rc::new(RefCell::new(canvas));
let canvas_clone = canvas_wrapper.clone();
Task::start(SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(move |_| {
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

### File System Architecture
```rust
// Dual state: Legacy globals + ConfigStore with bidirectional sync
static FILE_PATHS: Lazy<MutableVec<String>> = Lazy::new(MutableVec::new);
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));

fn sync_globals_to_config() {
    let paths: Vec<String> = FILE_PATHS.lock_ref().to_vec();
    CONFIG_STORE.with(|store| {
        store.opened_files.set_neq(paths);
        save_config_to_backend();  // Manual trigger when reactive signals fail
    });
}

// Smart labeling: VSCode-style disambiguation with parent directory for duplicates
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

// Collection patterns
static ITEMS: Lazy<MutableVec<SelectedVariable>> = Lazy::new(MutableVec::new);
static EXPANDED: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));
static SORTED: Lazy<MutableBTreeMap<String, Data>> = Lazy::new(MutableBTreeMap::new);  // Ordered reactive
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

## Lock Management & Actor Model

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

### Actor Model Implementation
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

### Mutable Patterns & Lock Rules
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

### Critical Lock Rules
1. Never hold locks across await points
2. Drop locks before triggering signals  
3. Use async signal handlers over for_each_sync
4. Prefer nested Mutables for frequent updates
5. Use Actor Model for complex state mutations

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

For comprehensive reactive patterns, see: `.claude/extra/technical/reactive-patterns.md`