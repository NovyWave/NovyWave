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
