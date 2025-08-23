# Performance Debugging Patterns

Quick reference for diagnosing and fixing common performance issues in NovyWave's reactive UI.

## TreeView Over-Rendering Recognition

### Symptoms
- **Console spam**: 30+ `🔨 [TreeView] RENDERING` messages within 300ms
- **Flickering**: Visible UI instability during file loading or scope expansion
- **Browser lag**: Interactions become sluggish or timeout during TreeView operations

### Common Root Causes
```rust
// ❌ Signal cascade pattern causing over-rendering
TRACKED_FILES → SMART_LABELS → child_signal(map_ref!) → Full TreeView Recreation

// ❌ Missing deduplication
TRACKED_FILES.signal_vec_cloned().to_signal_cloned().map(|files| {...})
//                                                   ^ No .dedupe_cloned()

// ❌ Debug logging in render paths
zoon::println!("🔨 [TreeView] RENDERING tree item: {}", item_id); // Blocks event loop
```

### Quick Fix Patterns
- **Remove intermediate signals**: Compute labels directly instead of derived signals
- **Add deduplication**: `.dedupe_cloned()` on signal chains feeding UI components
- **Remove debug logging**: From render paths, especially in TreeView component
- **Signal timing fixes**: Immediate sync + future changes for config restoration

## Duplicate Service Call Recognition

### Symptoms
- **Identical debug messages**: Same service call logged multiple times with identical parameters
- **Example**: `🔄 TIMELINE: Requested transitions for 7 variables` repeated immediately

### Root Cause Pattern
```rust
// Multiple signal handlers responding to same trigger
// Handler 1 (main.rs):
SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(|vars| {
    timeline_service_call(vars); // First call
});

// Handler 2 (waveform_canvas.rs):  
SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(|vars| {
    timeline_service_call(vars); // Duplicate call
});
```

### Solution Pattern
```rust
// Intelligent handler deduplication with mutually exclusive conditions
if CONFIG_LOADED.get() && !IS_LOADING.get() {
    // Handler 1: Normal operation
    timeline_service_call(vars);
} else {
    // Handler 2: Initialization only
    timeline_service_call(vars);
}
```

## Config Restoration Timing Issues

### Recognition
- **TreeView appears collapsed**: Despite saved `expanded_scopes` in `.novywave` config
- **Signal chain working**: But UI doesn't reflect config state on startup

### Root Cause
```rust
// ❌ Race condition: UI initializes before config sync
Task::start(async move {
    EXPANDED_SCOPES.signal().for_each(|scopes| {
        // Only catches FUTURE changes, misses initial config load
    });
});
```

### Fix Pattern
```rust
// ✅ Immediate sync + future changes
let current_expanded = EXPANDED_SCOPES.get_cloned();
derived.set_neq(current_expanded); // Immediate sync

Task::start(async move {
    EXPANDED_SCOPES.signal().for_each(|scopes| {
        // Also handle future changes
    });
});
```

## Browser MCP Performance Testing

### Verification Workflow
1. **Navigate**: `http://localhost:8080` 
2. **Wait**: 3+ seconds for full initialization
3. **Console logs**: Check for spam patterns
4. **Interaction test**: Click TreeView chevrons, verify smooth response
5. **Screenshot**: Document before/after visual improvements

### Success Criteria
- **Single service calls**: No duplicate debug messages
- **Clean console**: Only essential initialization logs
- **Responsive UI**: TreeView expansion works without lag
- **Config restoration**: Saved expanded state visible on load