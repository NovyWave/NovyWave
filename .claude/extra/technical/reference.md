# Technical Reference & Solutions

## WASM Compilation and Development

### Critical WASM Build Patterns
- **NEVER use `cargo build` or `cargo check`** - Only mzoon handles WASM properly
- Monitor compilation via `makers start > dev_server.log 2>&1 &`
- Auto-reload only triggers after successful compilation
- Check `tail -f dev_server.log` for WASM build status

### WASM Logging
```rust
// Correct WASM logging
zoon::println!("Debug message");

// Wrong - does nothing in browser
std::println!("Debug message");
```

### Development Server Management
- **NEVER restart dev server without permission** - backend/shared crates take minutes to compile
- Backend/shared compilation takes dozens of seconds to minutes - this is normal
- Wait for compilation to complete, don't restart repeatedly
- Use `makers kill` and `makers start` commands instead of manual process management

## Component Patterns and Conventions

### NovyUI Design System
```rust
// Icon usage - always use enum tokens, never strings
button()
    .left_icon(IconName::Folder)  // ✓ Correct
    .left_icon("folder")          // ✗ Never use strings

// Theme-aware colors using design tokens
.s(Background::new().color_signal(neutral_3().signal()))
.s(Font::new().color_signal(neutral_11().signal()))

// Layout patterns
Row::new()
    .s(Gap::new().x(8))           // Normal spacing
    .s(Align::new().center_y())   // Vertical centering
    .item(title_element)
    .item(El::new().s(Width::fill()))  // Spacer
    .item(action_button)
```

### Height Inheritance Pattern
```rust
// Critical height inheritance chain - missing Height::fill() breaks it
El::new().s(Height::screen())     // Root claims viewport
  .child(Column::new().s(Height::fill())    // All containers inherit
    .item(Row::new().s(Height::fill())      // Every container needs fill
      .item(panel_content)))
```

### TreeView Component Architecture
```rust
// TreeView with external state management
TreeView::new()
    .external_expanded_signal(EXPANDED_DIRECTORIES.signal())
    .external_selected_vec_signal(SELECTED_ITEMS.signal_vec_cloned())
    .single_scope_selection(true)  // Radio button behavior
    .item_signal(tree_data.signal_vec_cloned().map(...))
```

## Configuration System (TOML + Reactive Persistence)

### Dual-Layer Config Architecture
```rust
// shared/lib.rs - Backend schema
#[derive(Serialize, Deserialize)]
pub struct WorkspaceSection {
    pub dock_mode: String,
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
    pub selected_scope_id: Option<String>,
    pub expanded_scopes: IndexSet<String>,
}

// frontend/config.rs - Extended frontend structure
#[derive(Clone)]
pub struct PanelLayouts {
    pub docked_to_right: Mutable<PanelDimensions>,
    pub docked_to_bottom: Mutable<PanelDimensions>,
}
```

### Config Field Addition Pattern
```rust
// THREE locations required for new config fields:
// 1. shared/lib.rs types
pub struct WorkspaceSection {
    #[serde(default)]  // Always use default for new fields
    pub new_field: Option<NewType>,
}

// 2. frontend/config.rs SerializableConfig
pub struct SerializableConfig {
    pub new_field: Option<NewType>,
}

// 3. load_from_serializable() method
impl ConfigStore {
    fn load_from_serializable(&self, config: SerializableConfig) {
        if let Some(value) = config.new_field {
            self.new_field.set(value);
        }
    }
}
```

### Reactive Config Persistence
```rust
// Pattern: Signal monitoring + save triggers
fn init_config_handlers() {
    // Theme changes
    Task::start(current_theme().signal().for_each_sync(|_| {
        save_current_config();
    }));
    
    // Panel dimensions
    Task::start(FILES_PANEL_WIDTH.signal().for_each_sync(|_| {
        save_current_config();
    }));
}

// Initialization order prevents overwrites
pub fn initialize_config() -> impl Future<Output = ()> {
    async move {
        load_config().await;
        CONFIG_LOADED.set_neq(true);  // Gate flag
        init_config_handlers();       // Start reactive triggers
    }
}
```

## Theme System Implementation

### Theme-Aware Signal Patterns
```rust
// Reactive theme switching
.s(Background::new().color_signal(
    theme().signal().map(|t| match t {
        Theme::Light => neutral_1(),
        Theme::Dark => neutral_12(),
    })
))

// Scrollbar theming
.style_signal("scrollbar-color", 
    primary_6().signal().map(|thumb| 
        primary_3().signal().map(move |track| 
            format!("{} {}", thumb, track)
        )
    ).flatten()
)
```

### Color Token Usage
```rust
// Text colors
Font::new().color_signal(neutral_11().signal())  // Primary text
Font::new().color_signal(neutral_8().signal())   // Secondary/dimmed

// Background colors
Background::new().color_signal(neutral_1().signal())   // Main background
Background::new().color_signal(neutral_3().signal())   // Panel background
Background::new().color_signal(primary_6().signal())   // Accent elements
```

## Fast2D Graphics Integration

### Canvas Setup with Fast2D
```rust
use fast2d::*;

// Create canvas wrapper with shared access pattern
let canvas_wrapper = Rc::new(RefCell::new(canvas));

// Signal-based canvas updates
let canvas_clone = canvas_wrapper.clone();
Task::start(SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(move |_| {
    canvas_clone.borrow_mut().clear();
    // Redraw logic
}));
```

### Theme-Aware Fast2D Colors
```rust
// Use static RGBA constants matching neutral design tokens
pub static BACKGROUND_DARK: Color = Color::from_rgba(13, 13, 13, 255);
pub static BACKGROUND_LIGHT: Color = Color::from_rgba(255, 255, 255, 255);
```

### Canvas Resize Handling
```rust
// Combine Fast2D resize events with Zoon signal system
canvas.set_resize_callback(move |_width, _height| {
    // Handle resize with signal updates
});
```

### Theme Reactivity in Canvas
```rust
// Signal-based theme switching for canvas
let canvas_clone = canvas_wrapper.clone();
Task::start(theme().signal().for_each_sync(move |theme| {
    let bg_color = match theme {
        Theme::Light => Color::from_rgba(255, 255, 255, 255),
        Theme::Dark => Color::from_rgba(13, 13, 13, 255),
    };
    canvas_clone.borrow_mut().clear_with_color(bg_color);
}));
```

### Timeline Cursor Implementation
```rust
// Interactive timeline cursor with proper coordinate mapping
let canvas_click_handler = {
    let cursor_position = TIMELINE_CURSOR_POSITION.clone();
    move |event: PointerDown| {
        // Click events use page coordinates, need canvas-relative
        let canvas_rect = canvas_element.get_bounding_client_rect();
        let relative_x = event.client_x() as f32 - canvas_rect.left();
        
        // Convert to time with proper scaling
        let time = (relative_x / canvas_width) * total_time;
        cursor_position.set_neq(time);
    }
};
```

### Professional Timeline Algorithm
```rust
fn calculate_timeline_segments(timeline_width: f32, time_range: f64) -> Vec<f64> {
    let target_spacing = 80.0; // pixels
    let rough_intervals = (timeline_width / target_spacing) as usize;
    let raw_interval = time_range / rough_intervals as f64;
    
    // Round to nice numbers (1-2-5-10 scaling)
    let nice_interval = round_to_nice_number(raw_interval);
    
    // Generate segments with 10px edge margins
    (0..segments).map(|i| i as f64 * nice_interval).collect()
}
```

## Virtual List Optimization

### MutableVec Hybrid Stable Pool
```rust
// Optimal virtual list implementation
let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(...);

// Velocity-based dynamic buffering
let velocity_buffer = if current_velocity > 1000.0 { 15 } 
                     else if current_velocity > 500.0 { 10 } 
                     else { 5 };
```

### Critical Virtual List Rules
- **5-15 elements buffer size** with velocity adaptation
- Avoid over-buffering (50+ elements) - causes slower rerendering
- Use stable element pools - DOM elements never recreated, only content/position updates
- Signal simplification reduces performance overhead

### Height Calculation Patterns
```rust
// Dynamic height with proper constraints
El::new()
    .s(Height::exact_signal(item_count_signal.map(|count| (count * 40) as f32)))
    .update_raw_el(|raw_el| {
        raw_el.style("min-height", "0")  // Allow flex shrinking
    })
```

## File Handling and State Management

### File Loading Architecture
```rust
// Dual state system: Legacy globals + ConfigStore
static FILE_PATHS: Lazy<MutableVec<String>> = Lazy::new(MutableVec::new);
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = Lazy::new(|| Mutable::new(HashSet::new()));

// Bidirectional sync pattern
fn sync_globals_to_config() {
    let file_paths: Vec<String> = FILE_PATHS.lock_ref().to_vec();
    CONFIG_STORE.with(|store| {
        store.opened_files.set_neq(file_paths);
        // Manual save trigger needed when reactive signals fail
        save_config_to_backend();
    });
}
```

### Smart File Labeling System
```rust
// VSCode-style filename disambiguation
fn create_smart_labels(files: &[TrackedFile]) -> Vec<String> {
    let mut labels = Vec::new();
    for file in files {
        let filename = file.path.file_name().unwrap_or_default();
        
        // Check for duplicates
        let duplicates: Vec<_> = files.iter()
            .filter(|f| f.path.file_name() == Some(filename))
            .collect();
            
        if duplicates.len() > 1 {
            // Show disambiguating directory prefix
            labels.push(format!("{}/{}", parent_dir, filename));
        } else {
            labels.push(filename.to_string());
        }
    }
    labels
}
```

## Signal-Based Reactive Patterns

### Signal Composition Patterns
```rust
// Unify different signal types
let unified_signal = map_bool_signal(
    condition_signal,
    || first_signal.signal(),
    || second_signal.signal(),
);

// Dynamic element switching with type unification
Stripe::new()
    .direction_signal(dock_mode.signal().map(|mode| {
        if mode.is_docked() { Direction::Column } else { Direction::Row }
    }))
    .item_signal(content_signal.map(|content| {
        match content {
            ContentType::A => element_a().into_element(),  // Type unification
            ContentType::B => element_b().into_element(),
        }
    }))
```

### Performance-Optimized Signals
```rust
// Deduplication for expensive operations
TIMELINE_CURSOR_POSITION.signal()
    .dedupe()  // Prevent redundant triggers
    .for_each_sync(|position| {
        expensive_update(position);
    });

// Conditional signal processing with gates
if CONFIG_LOADED.get() {  // Prevent startup race conditions
    perform_config_operation();
}
```

## Debouncing and Task Management

### True Debouncing with Task::start_droppable
**Problem:** `Task::start().cancel()` doesn't guarantee immediate cancellation - tasks may still complete.

**Solution:** Use `Task::start_droppable` with `TaskHandle` dropping for guaranteed abortion.

```rust
// ❌ Fake debouncing - tasks may still complete after "cancel"
let debounce_task: Mutable<Option<Task<()>>> = Mutable::new(None);
signal.for_each_sync(move |_| {
    if let Some(existing_task) = debounce_task.take() {
        existing_task.cancel(); // NOT guaranteed to prevent execution
    }
    let new_task = Task::start(async { /* work */ });
    debounce_task.set(Some(new_task));
});

// ✅ True debouncing - dropping TaskHandle guarantees abortion
let debounce_task: Mutable<Option<TaskHandle<()>>> = Mutable::new(None);
signal.for_each_sync(move |_| {
    debounce_task.set(None); // Drop immediately aborts the task
    
    let new_handle = Task::start_droppable(async {
        Timer::sleep(1000).await; // Only completes if not dropped
        perform_operation();
    });
    debounce_task.set(Some(new_handle));
});
```

### Config Save Debouncing Pattern
```rust
// Prevent config save spam during rapid UI changes
let timeline_cursor_position_signal = TIMELINE_CURSOR_POSITION.signal();
Task::start(async move {
    let debounce_task: Mutable<Option<TaskHandle<()>>> = Mutable::new(None);
    
    timeline_cursor_position_signal
        .dedupe() // Skip duplicate values
        .for_each_sync(move |_| {
            debounce_task.set(None); // Abort previous save
            
            let new_handle = Task::start_droppable(async {
                Timer::sleep(1000).await; // 1 second of inactivity
                if CONFIG_INITIALIZATION_COMPLETE.get() {
                    save_config_to_backend();
                }
            });
            debounce_task.set(Some(new_handle));
        })
        .await;
});
```

## State Management Patterns

### MutableBTreeMap for Ordered Reactive State
**Use when:** You need ordered iteration and reactive updates
**Benefits:** Maintains sort order + signal reactivity

```rust
// Ordered state with reactive updates
static SORTED_ITEMS: Lazy<MutableBTreeMap<String, ItemData>> = 
    Lazy::new(MutableBTreeMap::new);

// Reactive iteration in sort order
SORTED_ITEMS.entries_cloned().for_each_sync(|entries| {
    // entries are automatically sorted by key
    for (key, value) in entries {
        display_item(key, value);
    }
});
```

### Signal Handler Consolidation
**Problem:** Multiple signal handlers for the same signal cause redundant processing
**Solution:** Unify into single handler with built-in logic

```rust
// ❌ Multiple handlers for same signal - redundant triggers
Task::start(async {
    TIMELINE_CURSOR_POSITION.signal().for_each_sync(|pos| {
        if is_in_visible_range(pos) {
            update_from_cache(pos);
        }
    }).await;
});

Task::start(async {
    TIMELINE_CURSOR_POSITION.signal().for_each_sync(|pos| {
        if !is_in_visible_range(pos) {
            query_server(pos);
        }
    }).await;
});

// ✅ Single unified handler - cleaner and more efficient
Task::start(async {
    TIMELINE_CURSOR_POSITION.signal().for_each_sync(|pos| {
        trigger_unified_query_logic(); // Built-in range checking
    }).await;
});

pub fn trigger_unified_query_logic() {
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let start = TIMELINE_VISIBLE_RANGE_START.get() as f64;
    let end = TIMELINE_VISIBLE_RANGE_END.get() as f64;
    
    if cursor_pos >= start && cursor_pos <= end {
        // Fast path - use cached data
        update_from_cached_transitions();
    } else {
        // Slow path - query server
        query_signal_values_at_time(cursor_pos);
    }
}
```

## Cache-First Signal Patterns

### Timeline Cursor Caching with Range Buffering
**Problem:** Cursor movements trigger server requests even when data is already cached
**Solution:** Expand "visible range" with buffer + check cache first

```rust
pub fn trigger_signal_value_queries() {
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let start = TIMELINE_VISIBLE_RANGE_START.get() as f64;
    let end = TIMELINE_VISIBLE_RANGE_END.get() as f64;
    
    if cursor_pos >= start && cursor_pos <= end {
        // Fast path - use cached transition data
        let selected_vars = SELECTED_VARIABLES.lock_ref();
        let mut new_values = SIGNAL_VALUES.get_cloned();
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        
        let transitions_cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
        
        for selected_var in selected_vars.iter() {
            if let Some(signal_transitions) = transitions_cache.get(&selected_var.unique_id) {
                // Find most recent transition at or before cursor time
                for transition in signal_transitions.iter().rev() {
                    if transition.time_seconds <= cursor_pos {
                        let signal_value = SignalValue::from_data(transition.value.clone());
                        new_values.insert(selected_var.unique_id.clone(), signal_value);
                        cache_hits += 1;
                        break;
                    }
                }
            } else {
                cache_misses += 1;
            }
        }
        
        zoon::println!("CACHE: Fast path results - {} hits, {} misses, UI {}", 
                      cache_hits, cache_misses, if cache_hits > 0 { "updated" } else { "unchanged" });
        
        if cache_hits > 0 {
            SIGNAL_VALUES.set(new_values); // Single UI update
        }
    } else {
        // Slow path - query server for data outside cached range
        query_signal_values_at_time(cursor_pos);
    }
}
```

### Generous Range Buffering for Better Cache Utilization
```rust
pub fn get_current_timeline_range() -> Option<(f32, f32)> {
    // ... existing range calculation ...
    
    let raw_range = if has_valid_files && min_time < max_time {
        // Add 20% buffer on each side to expand "visible range"
        let time_range = max_time - min_time;
        let buffer = time_range * 0.2; // 20% buffer
        let expanded_min = (min_time - buffer).max(0.0);
        let expanded_max = max_time + buffer;
        
        zoon::println!("CACHE: Expanding visible range from [{:.6}, {:.6}] to [{:.6}, {:.6}] (+{:.6}s buffer)", 
                      min_time, max_time, expanded_min, expanded_max, buffer);
        (expanded_min, expanded_max)
    } else {
        (SAFE_FALLBACK_START, SAFE_FALLBACK_END)
    };
    
    validate_and_sanitize_range(raw_range.0, raw_range.1)
}
```

### Cache Monitoring and Debug Patterns
```rust
// Debug cache effectiveness with clear output
zoon::println!("CACHE: Fast path - checking {} variables in cache (cursor at {:.6})", var_count, cursor_pos);
zoon::println!("CACHE: Fast path results - {} hits, {} misses, UI {}", cache_hits, cache_misses, 
               if any_updated { "updated" } else { "unchanged" });
zoon::println!("CACHE: Slow path - cursor outside visible range ({:.6} not in {:.6}-{:.6}), using server requests", 
               cursor_pos, start, end);

// Track cache vs server request ratios
let total_server_queries: usize = backend_queries_by_file.values().map(|v| v.len()).sum();
zoon::println!("CACHE: Slow path results - {} cached, {} server requests", cached_results.len(), total_server_queries);
```

## Configuration Save Optimization

### Multi-Level Debouncing Strategy
**Problem:** Frequent UI changes cause config save spam
**Solution:** Different debounce timers for different types of changes

```rust
// Global debounce at backend level (1 second)
pub fn save_config_to_backend() {
    if !SAVE_CONFIG_PENDING.get() {
        SAVE_CONFIG_PENDING.set_neq(true);
        Task::start(async {
            Timer::sleep(1000).await; // Backend-level debounce
            save_config_immediately();
            SAVE_CONFIG_PENDING.set_neq(false);
        });
    }
}

// Specific debouncing for different UI elements
fn setup_config_triggers() {
    // UI layout changes - immediate save (rare events)
    PANEL_WIDTH.signal().for_each_sync(|_| {
        if CONFIG_INITIALIZATION_COMPLETE.get() {
            save_config_to_backend();
        }
    });
    
    // Cursor position - longer debounce (frequent events)
    let debounce_task: Mutable<Option<TaskHandle<()>>> = Mutable::new(None);
    TIMELINE_CURSOR_POSITION.signal().dedupe().for_each_sync(move |_| {
        debounce_task.set(None); // True debouncing
        let new_handle = Task::start_droppable(async {
            Timer::sleep(1000).await; // Wait for inactivity
            if CONFIG_INITIALIZATION_COMPLETE.get() {
                save_config_to_backend();
            }
        });
        debounce_task.set(Some(new_handle));
    });
}
```

### Initialization Race Condition Prevention
```rust
// Prevent config overwrites during startup
static CONFIG_INITIALIZATION_COMPLETE: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

pub async fn initialize_config() {
    load_config().await;
    CONFIG_INITIALIZATION_COMPLETE.set_neq(true); // Gate flag
    setup_reactive_config_triggers(); // Start after loading
}

// All config save triggers check this flag
if CONFIG_INITIALIZATION_COMPLETE.get() {
    save_config_to_backend();
}
```

### Config Save Prioritization
```rust
// Different urgencies for different config sections
pub enum ConfigSaveUrgency {
    Immediate,    // Critical UI state (window size, etc.)
    Normal,       // User preferences (theme, etc.)
    Deferred,     // Navigation state (cursor position, etc.)
}

// Route saves through appropriate debouncing based on urgency
pub fn save_config_with_urgency(urgency: ConfigSaveUrgency) {
    match urgency {
        ConfigSaveUrgency::Immediate => save_config_immediately(),
        ConfigSaveUrgency::Normal => save_config_to_backend(), // 1s debounce
        ConfigSaveUrgency::Deferred => defer_config_save(),    // Longer debounce
    }
}
```

## Pattern Selection Guide

### When to Use Each Pattern

**Timeline Cursor Position Changes:**
```rust
// ✅ Use Task::start_droppable for true debouncing
// ✅ Expand visible range with buffer for better cache hits  
// ✅ Single unified signal handler with built-in range checking
// ✅ 1-second debounce for config saves during navigation
```

**Frequent UI State Changes:**
```rust  
// ✅ Use cache-first patterns to avoid server requests
// ✅ Debug monitoring to track cache effectiveness
// ✅ Consolidate multiple signal handlers into single logic
// ✅ Different debounce timers based on change frequency
```

**Config Save Optimization:**
```rust
// ✅ Multi-level debouncing (UI-specific + global backend)
// ✅ Initialization gates to prevent startup race conditions  
// ✅ TaskHandle dropping for guaranteed abortion
// ✅ Save urgency classification for different config sections
```

**State Management:**
```rust
// ✅ MutableBTreeMap for ordered reactive collections
// ✅ MutableVec for simple reactive lists
// ✅ Signal deduplication for performance
// ✅ Conditional signal processing with gates
```

### Common Anti-Patterns to Avoid

```rust
// ❌ Multiple signal handlers for same signal
// ❌ Task::start().cancel() for debouncing (not guaranteed)
// ❌ Fixed narrow ranges that cause cache misses
// ❌ Config saves on every UI change without debouncing
// ❌ Missing initialization gates causing startup overwrites
// ❌ Using HashMap when order matters (use BTreeMap)
```

### State Management Patterns
```rust
// MutableVec for reactive collections
static SELECTED_VARIABLES: Lazy<MutableVec<SelectedVariable>> = 
    Lazy::new(MutableVec::new);

// HashSet for expansion state
static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = 
    Lazy::new(|| Mutable::new(HashSet::new()));

// Bridge pattern for compatibility
fn bridge_to_external_selected() -> impl Signal<Item = Vec<TreeId>> {
    SELECTED_ITEMS.signal_vec_cloned()
        .map_ref(|items| items.iter().map(|item| TreeId(item.id.clone())).collect())
}
```

## Dock Mode Architecture

### Dock Mode Configuration
```rust
// Per-dock-mode storage
#[derive(Clone)]
pub enum DockMode {
    Right,
    Bottom,
}

// Separate dimensions per mode
pub struct WorkspaceSection {
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
}

// Layout switching
fn main_layout() -> impl Element {
    El::new()
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|docked| {
            if docked {
                docked_layout().into_element()
            } else {
                undocked_layout().into_element()
            }
        }))
}
```

### Panel Dimension Preservation
```rust
// Switch modes while preserving dimensions
fn switch_dock_mode_preserving_dimensions() {
    let current_dims = get_current_panel_dimensions();
    IS_DOCKED_TO_BOTTOM.set_neq(!IS_DOCKED_TO_BOTTOM.get());
    save_panel_dimensions_for_current_mode(current_dims);
    save_current_config();
}
```

## Performance Optimization Patterns

### Signal Chain Optimization
```rust
// Efficient signal chaining with deduplication
TIMELINE_CURSOR_POSITION.signal()
    .dedupe()  // Prevent duplicate triggers
    .for_each_sync(|position| {
        // Update dependent systems
    });
```

### Parallel Directory Traversal
```rust
// Use jwalk for 4x faster directory scanning
use jwalk::WalkDir;

WalkDir::new(path)
    .parallelism(jwalk::Parallelism::RayonNewPool(4))
    .into_iter()
    .filter_map(|entry| entry.ok())
    .collect()
```

### Unicode Text Filtering
```rust
// Robust invisible character filtering
use unicode_width::UnicodeWidthChar;

let clean_text: String = text.chars()
    .filter(|&c| c == ' ' || UnicodeWidthChar::width(c).unwrap_or(0) > 0)
    .collect();
```

## Memory Management Solutions

### WASM-Bindgen Canvas Integration
```rust
use wasm_bindgen::JsCast;

// Proper DOM element access in WASM
let canvas_element = event.target()
    .dyn_cast::<web_sys::Element>()
    .expect("Event target is not an element");
```

### Clipboard API for WASM
```rust
// Modern clipboard with fallback
async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    
    if let Some(clipboard) = window.navigator().clipboard() {
        // Modern Clipboard API
        clipboard.write_text(text).await
    } else {
        // Fallback to execCommand
        let document = window.document().unwrap();
        // execCommand implementation...
    }
}
```

### Thread-Based Library Integration
```rust
// Bridge async UI with blocking libraries using tokio::spawn_blocking
let result = tokio::spawn_blocking(move || {
    // Thread-blocking operations (file parsing, compression, etc.)
    expensive_blocking_operation(data)
}).await?;
```

## Bug Fixes & Troubleshooting

### Compilation Errors and Fixes

#### WASM Compilation Issues
**Problem**: Frontend changes not visible despite code changes
**Root Cause**: MoonZoon only auto-reloads after successful compilation
**Fix**: Always check compilation first
```bash
tail -100 dev_server.log | grep -i "error"
```

**Problem**: cargo build/check showing no WASM errors but mzoon failing
**Root Cause**: cargo cannot check WASM compilation properly
**Fix**: Only trust mzoon output for WASM build status

#### Icon Compilation Errors
**Problem**: `IconName::Check` causes compilation errors
**Root Cause**: Missing IconName enum variant or incorrect usage
**Fix**: Always use IconName enum tokens, check available variants
```rust
// Correct usage
button().left_icon(IconName::Check)

// Check available icons in novyui/src/icon.rs
pub enum IconName {
    Check, X, Folder, Search, ArrowDownToLine, // etc.
}
```

#### Signal Type Mismatches
**Problem**: `El.item_signal()` API compatibility errors
**Root Cause**: Signal type unification issues
**Fix**: Use `.into_element()` for type unification
```rust
.item_signal(content_signal.map(|content| {
    match content {
        ContentType::A => element_a().into_element(),
        ContentType::B => element_b().into_element(),
    }
}))
```

### Layout and Styling Problems

#### TreeView Width Issues
**Problem**: TreeView item backgrounds don't extend to full content width
**Root Cause**: Multiple levels of width constraints
**Ultimate Fix**: Multi-level constraint solution
```rust
// Container level
El::new()
    .s(Width::fill())
    .update_raw_el(|raw_el| {
        raw_el.style("min-width", "max-content")  // Horizontal expansion
    })

// Button level  
Button::new()
    .s(Width::fill())
    
// CSS level
.update_raw_el(|raw_el| {
    raw_el
        .style("width", "100%")
        .style("box-sizing", "border-box")
})
```

#### Height Inheritance Chain Breaks
**Problem**: Containers not filling available height
**Root Cause**: Missing `Height::fill()` anywhere in the chain
**Fix**: Ensure every container in hierarchy has `Height::fill()`
```rust
El::new().s(Height::screen())              // Root
  .child(Column::new().s(Height::fill())   // Every container needs this
    .item(Row::new().s(Height::fill())     // Missing this breaks everything
      .item(content)))
```

#### Scrollable Container Issues
**Problem**: Content not scrolling properly in flexbox layouts
**Root Cause**: Parent containers don't allow shrinking
**Fix**: Add `min-height: 0` to parent containers
```rust
.update_raw_el(|raw_el| {
    raw_el
        .style("min-height", "0")      // Allow flex shrinking
        .style("overflow-x", "auto")   // Enable horizontal scroll
})
```

#### Dropdown Height Problems
**Problem**: Dropdowns show scrollbars for small content
**Root Cause**: Invisible newline characters causing multi-line rendering
**Fix**: Unicode character filtering
```rust
use unicode_width::UnicodeWidthChar;

let clean_text: String = text.chars()
    .filter(|&c| c == ' ' || UnicodeWidthChar::width(c).unwrap_or(0) > 0)
    .collect();
```

### Event Handling Issues

#### Checkbox Event Bubbling
**Problem**: Checkbox clicks trigger both selection and expansion
**Root Cause**: Event propagation to parent row click handler
**Fix**: Prevent event propagation
```rust
El::new()
    .child(checkbox)
    .on_hovered_change(/* ... */)
    .global_event_handler(move |event: PointerDown| {
        event.pass_to_parent(false);  // Prevent bubbling
    })
```

#### Canvas Click Coordinate Issues
**Problem**: Click events use wrong coordinate system
**Root Cause**: Page coordinates vs canvas-relative coordinates
**Fix**: Convert using getBoundingClientRect
```rust
let relative_x = event.client_x() as f32 - canvas_rect.left();
let relative_y = event.client_y() as f32 - canvas_rect.top();
```

#### Global Keyboard Handlers
**Problem**: Keyboard events not working for modal dialogs
**Root Cause**: Focus management complexity
**Fix**: Use global event handlers with state guards
```rust
.global_event_handler({
    let close_dialog = close_dialog.clone();
    move |event: KeyDown| {
        if DIALOG_IS_OPEN.get() {  // Guard with dialog state
            if event.key() == "Escape" {
                close_dialog();
            }
        }
    }
})
```

### Memory Management Problems

#### Session Storage Issues  
**Problem**: Large session data causing storage problems
**Root Cause**: Excessive data storage in single observations
**Fix**: Use separate log files for large data, 2KB limits for observations
```bash
# Hook implementation with size limits
if [[ ${#data} -gt 2048 ]]; then
    echo "$data" > "$PROJECT_ROOT/.claude/logs/large-data-$(date +%s).log"
    echo "Large data stored in separate log file" # Store small reference instead
fi
```

#### Config Loading Race Conditions
**Problem**: Startup config overwrites user settings
**Root Cause**: Reactive triggers starting before config loads
**Fix**: Use initialization order with gate flags
```rust
// Load config first, then start reactive triggers
async fn initialize_app() {
    load_config().await;
    CONFIG_LOADED.set_neq(true);  // Gate flag
    init_reactive_handlers();     // Start triggers after config loads
}

// Guard reactive operations
if CONFIG_LOADED.get() {
    perform_config_operation();
}
```

### Configuration Persistence Issues

#### MutableVec Signal Chain Breaks
**Problem**: Complex signal chains don't trigger config saves
**Root Cause**: Multiple signal transformations break reactive triggers
**Fix**: Manual save calls in sync functions
```rust
fn sync_globals_to_config() {
    // Update config store
    let items: Vec<_> = GLOBAL_STATE.lock_ref().to_vec();
    CONFIG_STORE.with(|store| store.items.set_neq(items));
    
    // Manual save trigger when reactive signals fail
    save_config_to_backend();
}
```

#### Dock Mode Panel Dimension Issues
**Problem**: Panel heights getting overwritten between dock modes
**Root Cause**: Semantic overloading - same field controls different panels
**Fix**: Separate storage per dock mode
```rust
// Wrong - semantic overloading
pub struct Config {
    pub files_panel_height: f32,  // Means different things in different modes
}

// Correct - explicit per-mode storage
pub struct Config {
    pub panel_dimensions_right: PanelDimensions,
    pub panel_dimensions_bottom: PanelDimensions,
}
```

#### Scope Selection Persistence
**Problem**: Selected scope lost on app restart despite being stored
**Root Cause**: Missing field in shared crate for backend persistence
**Fix**: Add field to both frontend and shared crate
```rust
// shared/lib.rs
pub struct WorkspaceSection {
    pub selected_scope_id: Option<String>,  // Add to backend schema
}

// frontend/config.rs - already exists, just needs backend sync
```

### Performance Issues

#### Virtual List Blank Spaces
**Problem**: Empty spaces during scrolling in virtual lists
**Root Cause**: Element recreation during scroll events
**Fix**: Stable element pools with content-only updates
```rust
// Use stable element pool
let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(
    (0..buffer_size).map(|_| VirtualElementState::default()).collect()
);

// Update content only, never recreate elements
element.text_signal.set_neq(new_content);
element.position_signal.set_neq(new_position);
```

#### Excessive Debug Logging
**Problem**: Development server logs unreadable due to debug spam
**Root Cause**: Excessive println! statements in tight loops
**Fix**: Systematic debug cleanup
```bash
# Find and remove debug statements
rg "println!" --type rust | wc -l    # Count debug statements
# Remove non-essential debug output, keep only error logging
```

#### Directory Scanning Performance
**Problem**: Load Files dialog extremely slow on large directories
**Root Cause**: Synchronous directory traversal with full recursive scans
**Fix**: Parallel traversal with jwalk
```rust
use jwalk::WalkDir;

WalkDir::new(path)
    .parallelism(jwalk::Parallelism::RayonNewPool(4))
    .into_iter()
    .filter_map(|entry| entry.ok())
    .collect()  // 4x performance improvement
```