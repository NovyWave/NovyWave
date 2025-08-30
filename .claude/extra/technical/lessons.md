# Technical Lessons & Discoveries

General technical lessons learned during development that don't fit into specific reference categories.

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
