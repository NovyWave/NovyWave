# Performance Debugging Patterns

## TreeView Over-Rendering

**Symptoms:** 30+ renders in 300ms, flickering, browser lag

**Causes:**
- Signal cascade: `TRACKED_FILES → SMART_LABELS → child_signal(map_ref!) → Full recreation`
- Missing `.dedupe_cloned()` on signal chains
- Debug logging in render paths

**Fixes:**
- Remove intermediate signals, compute directly
- Add `.dedupe_cloned()` to signals feeding UI
- Remove debug logging from render paths

## Duplicate Service Calls

**Symptom:** Same service call logged multiple times with identical params

**Cause:** Multiple signal handlers responding to same trigger

**Fix:** Use mutually exclusive conditions in handlers

## Config Restoration Timing

**Symptom:** TreeView collapsed despite saved expanded_scopes

**Cause:** Signal handler only catches future changes, misses initial load

**Fix:** Immediate sync + future handler:
```rust
derived.set_neq(current_state);  // Immediate
signal.for_each(|s| derived.set_neq(s));  // Future
```

## Variables Panel Filtering

**Problem:** Filter triggering expensive file re-processing on every keystroke

**Fix:** Separate expensive loading from cheap filtering at signal level:
```rust
// Expensive: only on scope/file changes
fn loading_signal() -> Signal { get_variables_from_files(&scope_id) }

// Cheap: only on filter changes
fn display_signal() -> Signal { filter_variables(&variables, &filter) }
```

## Accidental Recreation Antipattern

**Symptoms:** Flickering, sluggish UI, memory churn

**Causes:**
- `signal.map(|data| Column::new().items(data...))` - recreates DOM
- `signal.map(|raw| expensive_processing(raw))` - recomputes everything
- Creating new signal chains on each update

**Fixes:**
- Use `items_signal_vec` for collections
- Add `.dedupe()` before expensive operations
- Create signals once, update content only

## Browser MCP Testing

1. Navigate to localhost:8080
2. Wait 3+ seconds for initialization
3. Check console for spam patterns
4. Test interactions, verify responsiveness
5. Screenshot before/after
