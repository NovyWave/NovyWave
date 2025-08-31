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

## Variables Panel Filtering Performance Issues

### Critical Performance Problem (Resolved)
**Issue**: Filter input changes were triggering expensive file re-processing operations on every keystroke.

### Symptoms
- **Console spam**: 821,470+ tokens of debug logs from single filter change
- **Expensive operations**: `get_variables_from_tracked_files()` called on every keystroke processing 5,371+ variables
- **Verbose logging**: Entire WaveformFile structures dumped to console (970kb+ spam)
- **Poor UX**: Sluggish filtering with noticeable delays

### Root Cause Analysis
```rust
// ❌ WRONG: Filtering inside UI function triggers file processing
map_ref! {
    let variables = variables_loading_signal(),
    let search_filter = search_filter_signal() => {
        // This calls expensive file processing on every filter change!
        virtual_variables_list(variables.clone(), search_filter.clone()).into_element()
    }
}
```

### Solution: Signal-Level Filtering Architecture
```rust
// ✅ CORRECT: Separate expensive loading from cheap filtering
fn variables_loading_signal() -> impl Signal<Item = Vec<VariableWithContext>> {
    // Only depends on scope/files - expensive operations
    map_ref! {
        let selected_scope_id = selected_scope_signal(),
        let _tracked_files = tracked_files_signal() => {
            get_variables_from_tracked_files(&scope_id) // Expensive - scope changes only
        }
    }
}

fn variables_display_signal() -> impl Signal<Item = Vec<VariableWithContext>> {
    // Only depends on loaded variables + filter - cheap operations
    map_ref! {
        let variables = variables_loading_signal(),
        let search_filter = search_filter_signal() => {
            filter_variables_with_context(&variables, &search_filter) // Cheap filtering
        }
    }
}

// UI uses pre-filtered signal
variables_display_signal().map(|filtered_variables| {
    virtual_variables_list_pre_filtered(filtered_variables).into_element()
})
```

### Performance Results
- **Before**: 821,470+ console log tokens, file re-processing on every keystroke
- **After**: Clean console logs, instant filtering of 5,371+ variables
- **Dynamic count**: Variables count shows filtered results (5371 → 608 → 1)
- **Config persistence**: Filter values properly saved and restored

### Key Lessons
1. **Separate concerns**: Expensive data loading vs cheap filtering operations
2. **Signal-level operations**: Move filtering logic to signal level, not UI components
3. **Clean up debug logs**: Remove excessive logging from hot paths
4. **Unified state management**: Single signal source for both count and content

## Critical Performance Antipattern: Accidental Recreation

### The Problem
**Recreating elements or datasets on every signal change** causes severe performance issues and poor UX.

### Common Symptoms
- **Flickering UI**: Elements disappear and reappear on every update
- **Sluggish performance**: UI feels unresponsive during interactions
- **Console spam**: Excessive rendering or processing logs
- **Memory churn**: High allocation/deallocation patterns

### Dangerous Patterns
```rust
// ❌ RECREATING ELEMENTS: New DOM elements on every signal change
signal.map(|data| {
    Column::new()
        .items(data.iter().map(|item| create_element(item))) // NEW elements every time!
})

// ❌ RECREATING DATASETS: Expensive recomputation on every change
signal.map(|raw_data| {
    let processed = expensive_processing(raw_data); // Reprocesses everything!
    render_data(processed)
})

// ❌ RECREATING SIGNAL CHAINS: New reactive chains on every update
signal.map(|state| {
    state.items.signal_vec_cloned().to_signal_cloned() // New signal chain!
        .map(|items| render(items))
})
```

### Solutions
```rust
// ✅ STABLE ELEMENTS: Update content, don't recreate DOM
.items_signal_vec(
    data.signal_vec().map(|item| {
        update_existing_element(item) // Update existing element
    })
)

// ✅ CACHED PROCESSING: Only recompute when input actually changes
let processed_data = expensive_data.signal().dedupe().map(|raw| {
    expensive_processing(raw) // Only when data actually changes
});

// ✅ STABLE SIGNAL CHAINS: Create signals once, update content
let stable_signal = create_signal_once();
stable_signal.map(|content| update_display(content))
```

### Prevention Strategies
1. **Use `.dedupe()`** on signals to prevent unnecessary updates
2. **Cache expensive computations** - only recompute when inputs change
3. **Update existing elements** instead of recreating DOM nodes
4. **Separate data loading from presentation** to avoid coupled recreation
5. **Monitor console logs** for patterns indicating excessive recreation

### Real-World Example
The Variables panel filtering fix demonstrates this principle:
- **Before**: Recreated filter logic on every keystroke → expensive file processing
- **After**: Separated data loading (stable) from filtering (updates only when needed) → instant response