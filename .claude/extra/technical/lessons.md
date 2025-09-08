# Technical Lessons & Discoveries

General technical lessons learned during development that don't fit into specific reference categories.

## Toast UI Architectural Simplification Pattern

### Eliminating Bridge Patterns in Actor State Management

When refactoring complex state management, look for opportunities to eliminate unnecessary bridge patterns:

**❌ BEFORE: Complex bridge pattern**
```rust
// Wrapper struct for simple state
#[derive(Clone, Debug)]
struct ToastState {
    elapsed_time: u64,
    is_paused: bool,
}

// Actor manages struct, Atom bridges to UI
let progress_atom = Atom::new(100.0);
let toast_actor = Actor::new(ToastState::default(), async move |state| {
    // Update progress Atom for UI reactivity
    progress_atom.set(progress);
});

// UI reads from bridge atom
.s(Width::percent_signal(progress_atom.signal().map(|p| p as f32)))
```

**✅ AFTER: Direct Actor state**
```rust
// Progress directly as Actor state
type Progress = f32;

// Internal variables in actor loop instead of struct fields
let toast_actor = Actor::new(100.0 as Progress, async move |state_handle| {
    let mut elapsed_time = 0.0f32;  // Internal variables
    let mut is_paused = false;
    
    // Update Actor state directly
    state_handle.set(progress);
});

// UI reads directly from actor
.s(Width::percent_signal(toast_actor.signal()))
```

### Type Strategy for Timer-based Code

**Pattern: f32 for calculations, cast only at API boundaries**

```rust
// ✅ OPTIMAL: f32 everywhere, single cast at API boundary
let auto_dismiss_ms = alert.auto_dismiss_ms as f32;     // Once at start
let mut elapsed_time = 0.0f32;
let update_interval_ms = 50.0f32;

_ = Timer::sleep(update_interval_ms as u32).fuse() => { // Single cast per cycle
    elapsed_time += update_interval_ms;                  // No casting
    let progress = 100.0 - (elapsed_time / auto_dismiss_ms * 100.0);  // No casting!
}
```

**Why this works:**
- **Optimizes for frequent operations** - progress calculation runs every 50ms
- **Minimizes casting overhead** - only one f32→u32 cast per timer cycle
- **Type consistency** - Progress is f32, so calculations stay in f32

### Import Organization for Actor+Relay Code

```rust
// ✅ Clean imports for Actor/Relay heavy code
use crate::dataflow::*;           // Actor, Relay, etc. used frequently
use futures::{select, stream::StreamExt};  // Move to top level
type Progress = f32;              // Type definitions with imports

// Usage becomes cleaner
let (relay, stream) = relay();    // vs crate::dataflow::relay()
```

### When to Apply This Pattern

- **Wrapper structs with 1-2 simple fields** → Use direct type + internal variables
- **Bridge Atoms between Actor and UI** → Connect UI directly to Actor signals  
- **Complex casting chains** → Choose types that minimize casting in hot paths
- **Frequent dataflow imports** → Use wildcard import `dataflow::*`

**Key insight:** Simpler architecture often emerges by questioning whether intermediate layers add real value or just complexity.

## Actor Cloning vs Signal Extraction Pattern

### Pass Cloned Actors Instead of Extracted Signals

**CRITICAL: Prefer passing cloned actors over extracted signals to prevent complexity from broadcasters and lifetime issues.**

**❌ WRONG: Extract signals before passing to functions**
```rust
// Creates unnecessary lifetime complexity and broadcaster dependencies
fn variables_loading_signal(tracked_files: crate::tracked_files::TrackedFiles) -> impl Signal<Item = Vec<VariableWithContext>> {
    let files_signal = tracked_files.files_signal();  // Extract signal first
    let files_broadcaster = files_signal.broadcast(); // Need broadcaster for lifetime
    map_ref! {
        let selected_scope_id = selected_scope_signal(),
        let _tracked_files = files_broadcaster.signal() => {  // Complex signal routing
            // Use extracted signal...
        }
    }
}
```

**✅ CORRECT: Pass cloned actors and use signals directly in map_ref!**
```rust
// Simple, clean pattern - no broadcasters or complex signal extraction needed
fn variables_loading_signal(tracked_files: crate::tracked_files::TrackedFiles) -> impl Signal<Item = Vec<VariableWithContext>> {
    map_ref! {
        let selected_scope_id = selected_scope_signal(),
        let _tracked_files = tracked_files.files_signal() => {  // Direct signal access
            // Use signal directly...
        }
    }
}
```

**Why actor cloning is better:**
- ✅ **No broadcaster complexity** - Direct signal access within map_ref!
- ✅ **No lifetime issues** - Owned actor values work naturally in closures
- ✅ **Cleaner code** - Single pattern instead of extract-then-broadcast
- ✅ **Better performance** - No intermediate signal extraction/broadcasting step

**When to apply:**
- Function parameters that need signal access
- Signal composition within `map_ref!` blocks
- Any situation where you're tempted to extract signals early to avoid lifetime issues

**Key insight:** Actor+Relay architecture is designed for cheap cloning - use it instead of fighting Rust's lifetime system.

## Signal Dependencies for Data Loading Timing

### Problem Pattern
Reactive signals that depend on data which loads asynchronously (like file parsing) need to include the data loading state as a dependency, not just the selection state.

**Broken Pattern:**
```rust
// ❌ Only depends on selection - doesn't re-fire when data loads
.child_signal(
    SELECTED_SCOPE_ID.signal_cloned().map(|selected_scope_id| {
        if let Some(scope_id) = selected_scope_id {
            let variables = get_variables_from_files(&scope_id); // May return empty if files not loaded yet
            variables.len().to_string()
        } else {
            "0".to_string()
        }
    })
)
```

**Issue:** Signal fires once on startup when scope is restored from config, but files are still Loading and return no variables. When files finish loading, the signal never fires again.

### Solution Pattern
Use `map_ref!` to merge multiple signal dependencies - both selection AND data loading:

```rust
// ✅ CORRECT: Depends on both selection and file loading state
.child_signal(
    map_ref! {
        let selected_scope_id = SELECTED_SCOPE_ID.signal_cloned(),
        let _tracked_files = tracked_files_signal() => {
            if let Some(scope_id) = selected_scope_id {
                let variables = get_variables_from_files(&scope_id);
                variables.len().to_string()
            } else {
                "0".to_string()
            }
        }
    }
)
```

**Why This Works:**
- Signal fires on startup when `SELECTED_SCOPE_ID` is restored (files still Loading → shows "0")
- Signal fires again when `tracked_files_signal()` changes from Loading → Loaded (shows actual count)
- Both dependencies are reactive - changes to either trigger re-evaluation

### When to Use This Pattern
- Any UI that displays data dependent on async loading (file parsing, network requests, etc.)
- Situations where selection state loads before data state
- Cases where "loading" vs "loaded" states need different UI behavior

### Key Principle
**Merge signals using `map_ref!` when UI depends on multiple changing data sources.** Don't rely on single signals when multiple async processes affect the result.

## DOM/State Loading Timing Issues with Scroll Position

### Problem Pattern
Scroll position restoration fails when attempting to restore before DOM content is loaded:

```rust
// ❌ WRONG: Restores scroll before tree data loads
.viewport_y_signal(app_config().file_picker_scroll_position.signal())
```

### Solution Pattern
Wait for data dependencies before applying scroll position:

```rust
// ✅ CORRECT: Wait for tree data before scroll restoration
.viewport_y_signal(
    map_ref! {
        let tree_cache = FILE_TREE_CACHE.signal_cloned(),
        let scroll_position = app_config().file_picker_scroll_position.signal() => {
            if tree_cache.contains_key("/") {
                *scroll_position  // Only restore when data loaded
            } else {
                0  // Default while loading
            }
        }
    }
)
.after_insert({
    let scroll_position = app_config().file_picker_scroll_position.get();
    move |_element| {
        app_config().file_picker_scroll_position.set(scroll_position);  // Force signal
    }
})
```

**Key Principles:**
- Check data dependencies (`tree_cache.contains_key("/")`) before applying scroll
- Use `after_insert()` to force initial signal emission after DOM ready
- Combine reactive signals with insertion callbacks for reliable timing


## CONFIG_LOADED Guard Antipattern

### Antipattern Identified
Using `CONFIG_LOADED` guards to prevent save operations during startup:

```rust
// ❌ ANTIPATTERN: Initialization guard
if CONFIG_LOADED.get() { 
    save_config_to_backend(); 
}
```

### Why It's Wrong
- Creates race conditions between config loading and UI initialization  
- Adds unnecessary complexity and state synchronization
- Breaks reactive architecture with imperative checks
- Becomes obsolete with proper signal-based config loading

### Correct Approach
Use await-based config loading in main, then rely on reactive signals:

```rust
// ✅ CORRECT: Load config with await in main
async fn main() {
    load_config().await;  // Config guaranteed loaded
    start_ui();           // UI can immediately use reactive signals
}

// ✅ CORRECT: Direct reactive save triggers
expanded_directories.signal().for_each_sync(|dirs| {
    session_state_actor.send(UpdateExpandedDirectories(dirs));
});
```

**Migration Steps:**
1. Remove `CONFIG_INITIALIZATION_COMPLETE` static declarations
2. Remove guard checks from config save functions  
3. Use signal-based reactive patterns instead of imperative guards
4. Ensure config loaded with await before UI initialization

**Key Insight:** Proper initialization order eliminates need for runtime guards.

## Signal Timing Issues: Avoid Artificial Workarounds

### The Temptation
When signals don't fire on app startup but work on user interaction, the tempting "quick fix" is to add periodic timers or artificial delays:

```rust
// ❌ WRONG: Periodic refresh workaround
let refresh_trigger = Mutable::new(0);
Task::start(async move {
    loop {
        Timer::sleep(1000).await; // Check every second
        refresh_trigger.set_neq(refresh_trigger.get() + 1);
        if current > 10 { break; } // Stop after 10 seconds
    }
});
```

### Why This Is Wrong
- **Masks the real problem** - Signal initialization/timing issues remain unfixed
- **Performance waste** - Unnecessary periodic processing
- **Fragile solution** - May break with timing changes or different hardware
- **Technical debt** - Makes codebase harder to understand and maintain
- **Not reactive** - Defeats the purpose of reactive architecture

### CRITICAL: No Artificial Timer::sleep Delays

**❌ NEVER use Timer::sleep() for timing coordination:**
```rust
// ❌ PROHIBITED: Artificial timing delays
zoon::Timer::sleep(10).await;  // "Fix" timing issues with delay
Timer::sleep(1000).await;      // Wait for data to be available
```

**Why Timer::sleep() delays are wrong:**
- **Error-prone**: Arbitrary delays don't guarantee data availability
- **Fragile**: Hardware differences change timing requirements
- **Non-deterministic**: Race conditions still exist, just harder to reproduce
- **Maintenance nightmare**: Mysterious timing dependencies throughout codebase

### Proper Solutions

**✅ CORRECT: Use proper async coordination:**
```rust
// ✅ Use Task::next_macro_tick() for event loop yielding
Task::next_macro_tick().await;  // Yield to event loop properly

// ✅ Use signal-based waiting for actual data availability
let data = some_signal.to_stream().next().await.expect("Data should be available");

// ✅ Use Actor model for proper state synchronization
let actor = Actor::new(State::default(), async move |state| {
    // Proper state management with event coordination
});
```

**Core Principles:**
1. **Fix signal initialization order** - Ensure signals emit initial values properly
2. **Fix dependency chains** - Make sure all signal dependencies are correct
3. **Proper Actor+Relay setup** - Connect signals to real state changes
4. **Startup lifecycle management** - Load config before UI initialization
5. **Use Task::next_macro_tick()** - For proper event loop coordination
6. **Signal-based waiting** - Wait for actual conditions, not arbitrary time

```rust
// ✅ CORRECT: Fix the root cause
pub fn selected_scope_signal() -> impl Signal<Item = Option<String>> {
    // Ensure signal emits initial config value
    crate::state::SELECTED_SCOPE_ID.signal_cloned().dedupe_cloned()
}

// ✅ CORRECT: Proper initialization order
async fn main() {
    load_config().await;          // Config loaded first
    initialize_domains().await;   // Domains have config data
    start_ui();                   // UI gets immediate signal values
}
```

### Migration Strategy
When encountering signal timing issues:
1. **Identify the real problem** - Why doesn't the signal fire initially?
2. **Fix initialization order** - Config → Domains → UI
3. **Fix signal dependencies** - Remove static signal antipatterns
4. **Test thoroughly** - Ensure signals emit initial values
5. **Remove workarounds** - Delete any artificial delays or periodic checks

**Remember:** Reactive architecture should "just work" without artificial timing fixes.

## Error Display UX: Console Logging vs. Toast Notifications

### Lesson: Separate Developer Debugging from User Experience

When implementing error handling during architectural migrations, distinguish between **developer needs** and **user experience**:

**Problem Pattern**: Background operations (directory preloading, file parsing) show error toasts immediately on app startup, confusing users who didn't initiate those actions.

**Solution**: `log_error_console_only()` function for background errors:

```rust
/// Log error to browser console only (no toast notification)  
/// Use for background operations or non-user-initiated errors
pub fn log_error_console_only(alert: ErrorAlert) {
    zoon::eprintln!("{}", alert.technical_error);  // Console for developers
    add_domain_alert(alert);  // Domain tracking without toast
}

// Usage for background operations
DownMsg::DirectoryError { path, error } => {
    log_error_console_only(ErrorAlert::new_directory_error(path, error));  // Console only
    store_error_for_ui_display(path, error);  // Still show in dialog where relevant
}
```

**Key Insight**: The simplest UX solution is often elimination, not conditional logic. Instead of "only show toast if dialog visible", better to show error directly where users need it (dialog tree) and log for developers (console).

**Benefits:**
- ✅ Developers get debugging info in browser console
- ✅ Users get clean experience without confusing startup toasts  
- ✅ Errors still visible in context (file dialog tree)
- ✅ No complex conditional visibility logic needed

## Comprehensive Troubleshooting Guide

### Common Actor+Relay Issues

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

### WASM Integration Issues

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
- **Config races**: Use await-based config loading before UI initialization
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
- **FusedFuture compilation**: Actor stream processing with `futures::select!` works directly with our relay streams (no `.fuse()` needed)

#### Actor Stream Processing Patterns

**✅ CORRECT: Direct stream usage (relay streams are already fused)**
```rust
let panel_dimensions_right_actor = Actor::new(PanelDimensions::default(), async move |state| {
    let mut right_stream = panel_dimensions_right_changed_stream; // No .fuse() needed!
    let mut resized_stream = panel_resized_stream; // Already implements FusedStream
    
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
```

**Key Requirements for Our Relay Streams:**
- Use `futures::{StreamExt, select}` imports
- Relay streams already implement `FusedStream` - no manual `.fuse()` needed
- Use plain `select!` macro, not `tokio::select!`
- Pattern works in both WASM and native environments
- `UnboundedReceiver<T>` automatically implements `FusedStream`

### Timer::sleep() Fusing Requirements (Temporary Issue)

**❌ Timer::sleep() requires .fuse() workaround:**
```rust
loop {
    select! {
        // NOTE: .fuse() required due to broken FusedFuture in oneshot::Receiver
        // See: https://github.com/rust-lang/futures-rs/issues/2455
        //      https://github.com/rust-lang/futures-rs/issues/1989
        //      https://github.com/rust-lang/futures-rs/issues/2207
        _ = Timer::sleep(update_interval_ms as u32).fuse() => {
            // Process timer events
        }
    }
}
```

**Root Cause:**
- `Timer::sleep()` uses `futures::channel::oneshot::Receiver` internally
- `oneshot::Receiver` has broken `FusedFuture.is_terminated()` implementation
- Returns `true` when sender is dropped (incorrect behavior for select!)
- Affects all oneshot channels in select! loops

**Status:** This is a known issue in the futures library. Once resolved upstream, the `.fuse()` calls can be removed from Timer::sleep() usage.

## Critical Signal Routing Debugging Pattern

### Always Trace UI Signal Sources Before Updating State

When implementing drag/resize functionality that updates correctly in logs but doesn't show visual changes, the problem is usually **updating the wrong signals**.

**Debugging Steps:**
1. **Trace from UI backwards**: Find what signal the UI actually reads from
2. **Identify all signal systems**: Multiple width/state systems often exist in complex apps
3. **Update the correct signal source**: Not just any related signal

**Real Example - Panel Width Drag:**
```rust
// ❌ WRONG: Drag logic was perfect, but updating wrong signals
if let Some(signals) = PANEL_LAYOUT_SIGNALS.get() {
    signals.files_panel_width_mutable.set_neq(new_width);  // Legacy migration signal
}

// ✅ UI actually reads from config system via files_width_signal()
// files_width_signal() → app_config().panel_dimensions_right_actor.signal()

// ✅ CORRECT: Update the actual config signals UI reads from
config.panel_dimensions_right_changed_relay.send(updated_dimensions);
config.panel_dimensions_bottom_changed_relay.send(updated_dimensions);
```

**Common Signal System Confusion:**
- **Config system**: `app_config().panel_dimensions_*` (what UI uses)
- **Migration signals**: `PANEL_LAYOUT_SIGNALS.*_mutable` (legacy compatibility) 
- **Actor internal state**: Actor's private state values
- **Domain signals**: Business logic state separate from UI

**Key Insight:** Perfect drag calculations + wrong signal updates = logs show changes but UI doesn't move.

**Prevention:** Always `grep` for UI usage patterns like `Width::exact_signal(signal_name())` to trace the actual signal dependency chain before implementing updates.

## Actor State Synchronization: Eliminating Drag Jump Issues

### Problem Pattern
Actor-based dragging systems can experience "jump" behavior when the Actor's internal state doesn't match the actual UI state at drag start.

**Broken Pattern:**
```rust
// ❌ Actor initialized with hardcoded default
let files_panel_width = Actor::new(470, async move |handle| {
    loop {
        select! {
            Some((movement_x, _)) = mouse_moved_stream.next() => {
                if is_dragging {
                    let current_width = handle.get();  // Gets 470, not actual width!
                    let new_width = current_width + movement_x;  // Jumps from 470
                    handle.set(new_width);
                }
            }
        }
    }
});
```

**Issue:** UI shows actual dock-specific width (600px for Right, 626px for Bottom) but Actor starts with hardcoded default (470px). When dragging starts, there's a visual jump from actual position to Actor's default position.

### Solution Pattern
Sync Actor state with current config-driven UI state when dragging starts:

```rust
// ✅ CORRECT: Sync Actor state with current dock-specific width on drag start
let files_panel_width = Actor::new(470, async move |handle| {
    let mut is_dragging = false;
    
    loop {
        select! {
            Some(dragging_state) = dragging_stream.next() => {
                // Sync Actor state with current dock-specific width when dragging starts
                if dragging_state && !is_dragging {
                    let config = crate::config::app_config();
                    let config_clone = config.clone();
                    let handle_clone = handle.clone();
                    Task::start(async move {
                        let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                        let current_width = match current_dock_mode {
                            Some(shared::DockMode::Right) => {
                                if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                    dims.files_panel_width as u32
                                } else { 470 }
                            }
                            Some(shared::DockMode::Bottom) => {
                                if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                    dims.files_panel_width as u32
                                } else { 470 }
                            }
                            None => 470
                        };
                        handle_clone.set(current_width);  // Sync with actual UI state
                        zoon::println!("🎯 Synced Actor width to current dock: {}", current_width);
                    });
                }
                is_dragging = dragging_state;
            }
            Some((movement_x, _)) = mouse_moved_stream.next() => {
                if is_dragging {
                    let current_width = handle.get();  // Now gets synced width!
                    let new_width = current_width + movement_x;  // Smooth from actual position
                    handle.set(new_width);
                }
            }
        }
    }
});
```

**Why This Works:**
- **Detects drag start**: `dragging_state && !is_dragging` catches the transition to dragging
- **Gets current dock-specific state**: Reads the actual width from config that UI is displaying
- **Syncs Actor internal state**: Updates Actor's state to match UI before processing drag movements
- **Eliminates jump**: Dragging starts from actual visual position, not hardcoded default

### When to Use This Pattern
- Actor-based dragging systems where UI state comes from external config
- Dock-mode-specific dimensions that change independently
- Any situation where Actor default state differs from actual UI state
- Multi-mode layouts where same Actor handles different contexts

### Key Principle
**Always sync Actor internal state with external UI state before processing user interactions.** Don't assume Actor defaults match the actual visual state the user sees.

## Dock Mode Dimension Syncing Fix

### Problem
When switching dock modes (Right ↔ Bottom), panel dimensions were being synced between modes instead of maintaining independent values. The issue occurred because dock switching logic used shared Actor state values and saved them to both dock configurations.

### Root Cause
```rust
// ❌ WRONG: Using shared Actor values for both dock modes
files_panel_height: current_files_height as f32,  // Same value saved to both configs
```

The `current_files_height` came from a shared panel_layout Actor, so when switching dock modes, this shared value overwrote the config for both Right and Bottom modes.

### Solution
Preserve existing config values instead of overwriting with shared Actor state:

```rust
// ✅ CORRECT: Keep existing config values during dock switch
let updated_dims = PanelDimensions {
    files_panel_width: current_dims.files_panel_width,     // Keep existing
    files_panel_height: current_dims.files_panel_height,   // Keep existing - don't overwrite
    variables_panel_width: current_dims.variables_panel_width,
    timeline_panel_height: current_dims.timeline_panel_height,
    variables_name_column_width: current_name_width as f32,  // Update from Actor (actively used)
    variables_value_column_width: current_value_width as f32, // Update from Actor (actively used)
};
```

### Key Insight
Only update dimensions that are actively managed by the current dock mode. Preserve dimensions that aren't directly controlled by shared Actors to maintain independent dock mode storage.

### Testing Pattern
1. Set different values in config: Bottom `height=356`, Right `height=510`
2. Switch modes repeatedly: Right→Bottom→Right
3. Verify console logs show preserved values: "height=356 (kept)", "height=510"
4. Confirm config file maintains independent values after switches

## Global Domains Migration Patterns

### Complete Architecture Migration Success Patterns

When migrating from global static patterns to Actor+Relay architecture, these proven patterns eliminate compilation errors systematically:

#### Domain Parameter Propagation Pattern

**Problem**: Functions accessing global domains break when globals are removed.

**Solution**: Thread domain instances through function parameters.

```rust
// ✅ BEFORE: Global access (deprecated)
fn render_variables() -> impl Element {
    let vars = crate::global_domains::selected_variables();
}

// ✅ AFTER: Domain parameter passing
fn render_variables(
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    let vars = selected_variables.variables_vec_signal.signal_cloned();
}

// ✅ Update all call sites to pass domains
variables_panel(tracked_files, selected_variables, waveform_timeline)
```

#### Signal Broadcasting Pattern for Lifetime Issues

**Problem**: Rust 'static lifetime requirements for signals in closures when using borrowed domain parameters.

**Solution**: Use `.broadcast()` method to create cloneable signal broadcasters before closures.

```rust
// ❌ BROKEN: Borrowed data escapes closure
.items_signal_vec(
    selected_variables.variables.signal_vec().map(|var| {
        tracked_files.files_signal().map(|files| ...) // Lifetime error!
    })
)

// ✅ CORRECT: Broadcaster pattern
.items_signal_vec({
    let selected_variables_for_items = selected_variables.clone();
    let tracked_files_broadcaster = tracked_files.files_signal().broadcast();
    selected_variables.variables.signal_vec().map(move |var| {
        tracked_files_broadcaster.signal().map(|files| ...) // Works!
    })
})
```

#### ConnectionAdapter Pattern

**Problem**: Static Connection instances break Actor+Relay architecture.

**Solution**: Wrapper that provides Actor-compatible message streams.

```rust
// ✅ Actor-compatible Connection wrapper
pub struct ConnectionAdapter {
    connection: Connection<UpMsg, DownMsg>,
}

impl ConnectionAdapter {
    pub fn new() -> (Self, impl futures::stream::Stream<Item = DownMsg>) {
        let (message_sender, message_stream) = futures::channel::mpsc::unbounded();
        let connection = Connection::new(move |down_msg, _| {
            let _ = message_sender.unbounded_send(down_msg);
        });
        (ConnectionAdapter { connection }, message_stream)
    }
}

// Usage in domain Actors
let (connection_adapter, mut down_msg_stream) = ConnectionAdapter::new();
let message_handler = Actor::new((), async move |_state| {
    while let Some(down_msg) = down_msg_stream.next().await {
        handle_down_msg(down_msg, &tracked_files, &selected_variables);
    }
});
```

#### Closure Lifetime Management Pattern

**Problem**: Closures capturing borrowed domains fail lifetime checks.

**Solution**: Clone domains before closures, use cloned references inside.

```rust
// ❌ BROKEN: Closure captures borrowed value
.on_press(move || {
    selected_variables.variable_removed_relay.send(id); // Borrow error!
})

// ✅ CORRECT: Clone before closure
.on_press({
    let selected_variables_for_press = selected_variables.clone();
    move || {
        selected_variables_for_press.variable_removed_relay.send(id); // Works!
    }
})
```

#### Actor+Relay Integration Pattern

**Problem**: UI components lose reactive updates when globals are removed.

**Solution**: Connect components directly to domain signals, not intermediate functions.

```rust
// ❌ OLD: Global function calls
crate::selected_variables::variables_signal()

// ✅ NEW: Direct domain signal access
selected_variables.variables_vec_signal.signal_cloned()

// ✅ Function signature updates
pub fn virtual_variables_list_pre_filtered(
    filtered_variables: Vec<VariableWithContext>,
    selected_variables: &crate::selected_variables::SelectedVariables, // Add domain param
) -> Column<column::EmptyFlagNotSet, RawHtmlEl> {
    // Use selected_variables.variables_vec_signal.signal_cloned() directly
}
```

### Migration Success Metrics

**Systematic Error Reduction Pattern:**
- Start: 146 compilation errors (global domains removal)
- Apply domain parameter propagation: ~100 errors
- Add signal broadcasting: ~50 errors  
- Fix closure lifetimes: ~10 errors
- Complete Actor+Relay integration: 0 errors

**Key Success Indicators:**
- Zero compilation errors with only warnings remaining
- All domain parameters passed correctly through call chains
- No global static access remaining in codebase
- Proper Actor+Relay signal patterns throughout
- Broadcaster pattern resolving all lifetime issues

This systematic approach enables complete architectural migrations while maintaining functionality and avoiding temporary bridges or workarounds.

## Getter Antipattern Masking Rust Edition Issues

### Problem: Hidden Lifetime Issues from Unnecessary Method Wrappers

**Discovered Issue**: Getter methods for public struct fields can mask Rust 2024 edition compiler requirements, leading to hard-to-resolve lifetime errors.

**Example Scenario:**
```rust
// ❌ ANTIPATTERN: Unnecessary getter method masking real issue
impl TrackedFiles {
    pub fn files_signal_vec(&self) -> impl SignalVec<Item = TrackedFile> {
        self.files.signal_vec()  // Wrapper around public field
    }
}

// Usage causes mysterious lifetime errors
tracked_files_for_signals.files_signal_vec().map(move |tracked_file| {
    // ERROR: Lifetime issues that seem unresolvable
})
```

**Root Cause**: 
- **Unnecessary wrapper methods** hide the real issue: missing `+ use<T>` syntax in Rust 2024 edition
- **ActorVec method return types** need explicit `+ use<T>` capture syntax for newer compiler
- **Getter methods add indirection** that obscures the actual compilation requirement

**✅ SOLUTION: Direct public field access reveals real issue**
```rust
// ✅ CORRECT: Use public fields directly (no getter wrapper)
pub struct TrackedFiles {
    pub files: ActorVec<TrackedFile>,  // Direct access - no getter needed
}

// Direct usage shows the real error clearly
tracked_files_for_signals.files.signal_vec().map(move |tracked_file| {
    // Now error clearly indicates missing + use<T> in ActorVec return type
})
```

**Fix Required in ActorVec Implementation:**
```rust
// ActorVec method needs + use<T> for Rust 2024 edition
impl<T> ActorVec<T> {
    pub fn signal_vec(&self) -> impl SignalVec<Item = T> + use<T> {  // Add + use<T>
        // Implementation
    }
}
```

**Key Lessons:**
- **Getter methods mask real issues** - Remove unnecessary wrappers for struct fields
- **Direct field access is clearer** - Shows actual compilation requirements
- **Rust 2024 edition changes** require explicit `+ use<T>` capture syntax
- **Follow public field architecture** - Don't create getters for simple field access

**Prevention:**
- Use public struct fields instead of getter methods
- Check ActorVec/Actor method signatures for `+ use<T>` requirements
- When encountering mysterious lifetime errors, simplify to direct field access first

## Systematic Actor+Relay Architecture Migration Pattern

### Bridge Pattern Elimination Strategy

When encountering complex bi-directional sync code, the solution is usually **complete elimination** rather than trying to fix the bridge. Direct signal connections work better than intermediate synchronization layers.

**Successful Pattern:**
```rust
// ❌ BEFORE: Complex bridge pattern (40+ lines)
let bridge_mutable = Mutable::new(vec![]);
Task::start(async move {
    actor_signal.for_each_sync(|data| {
        bridge_mutable.set_neq(data);  // Manual sync
    });
});
TreeView::new().external_expanded(bridge_mutable)

// ✅ AFTER: Direct connection (eliminated entirely)
TreeView::new().external_expanded(app_config.expanded_directories.clone())
```

### Compilation Error Resolution Priority

**Systematic approach that works:**
1. **Comment out unused functions** causing lifetime issues rather than fighting complex fixes
2. **Fix missing variable references** with proper types (`MutableVec<String>` vs `Mutable<Vec<String>>`)
3. **Address type mismatches** at the source rather than adding conversions

### Mutable Usage Rules (CORRECTED)

**✅ ONLY ACCEPTABLE Mutable usage:**
1. **External component API requirements** - When UI components like TreeView explicitly require `Mutable<T>` or `MutableVec<T>`
2. **Interface compliance** - When integrating with external libraries that demand specific types

**❌ NOT ACCEPTABLE (use Atom instead):**
- Dialog visibility → `Atom<bool>`
- Hover states → `Atom<bool>` 
- Temporary selections → `Atom<Vec<String>>` or domain Actor
- Form input values → `Atom<String>`
- All other local UI state → `Atom<T>`

**Key Principle:** Mutables are a necessary evil ONLY for external API compliance. When we have control over the implementation, use Actor+Relay or Atom.

### TreeView Integration Pattern

TreeView components need specific types:
- `external_expanded(Mutable<HashSet<String>>)` for expansion state
- `external_selected_vec(MutableVec<String>)` for selection state (NOT `Mutable<Vec<String>>`)
- Direct signal connections rather than bridge patterns

**Two acceptable patterns:**

```rust
// Pattern 1: Local UI state for dialogs
let selected_files = zoon::MutableVec::<String>::new();
TreeView::new().external_selected_vec(selected_files.clone())

// Pattern 2: Domain integration (preferred for important state)
struct FilePickerDomain {
    pub expanded_directories: Mutable<HashSet<String>>,
    pub selected_files: MutableVec<String>,
}
```

### Systematic Migration Success Framework

1. **Identify all violations** through comprehensive search (Mutable, TODO, OnceLock patterns)
2. **Create detailed todo tracking** for each violation type  
3. **Fix systematically** one category at a time
4. **Verify compilation** after each major change
5. **Re-enable functionality** with proper reactive patterns
6. **Complete verification** that all target issues are resolved

**Critical Success Factors:**
- Remove static signal bypass antipatterns (OnceLock patterns) entirely
- Use Atom for local UI state, never Mutable unless external API requires it
- Eliminate bridge patterns completely rather than trying to fix them
- Comment out problematic unused functions rather than fighting lifetime issues
- Ensure proper types for external component APIs (MutableVec vs Mutable)
