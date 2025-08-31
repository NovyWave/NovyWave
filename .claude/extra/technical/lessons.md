# Technical Lessons & Discoveries

General technical lessons learned during development that don't fit into specific reference categories.

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

**Migration Note:** Currently uses Mutables, should be rewritten with Atoms or Actor+Relay architecture later.

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

### Proper Solutions
1. **Fix signal initialization order** - Ensure signals emit initial values properly
2. **Fix dependency chains** - Make sure all signal dependencies are correct
3. **Proper Actor+Relay setup** - Connect signals to real state changes
4. **Startup lifecycle management** - Load config before UI initialization

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
