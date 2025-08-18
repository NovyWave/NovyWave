# Request Deduplication Bug Analysis and Removal Plan

## Problem Overview

The request deduplication system in `waveform_canvas.rs` has a critical bug that causes signal transitions to never be sent to the backend, resulting in empty timeline stripes with no waveform data.

## Root Cause Analysis

### The Critical Bug

**Location**: `frontend/src/waveform_canvas.rs` lines 849-881

**Sequence of Events**:
1. `request_signal_transitions_from_backend()` is called
2. Deduplication code sets flag in `REQUESTS_IN_PROGRESS` (line 858)
3. Code checks if file is loaded (lines 872-883)  
4. **BUG**: If file not loaded, early return at line 881
5. Flag remains set in `REQUESTS_IN_PROGRESS` forever
6. All subsequent requests are blocked with "REQUEST ALREADY IN PROGRESS"
7. No signal transitions are ever sent to backend

### Current Buggy Code

```rust
// Lines 849-881 in waveform_canvas.rs
// Check if request is already in progress to avoid duplicates
{
    let mut requests_in_progress = REQUESTS_IN_PROGRESS.lock_mut();
    if requests_in_progress.contains(&cache_key) {
        zoon::println!("=== REQUEST ALREADY IN PROGRESS - skipping {} ===", cache_key);
        return; // Request already in progress, don't duplicate
    }
    
    // Mark request as in progress
    requests_in_progress.insert(cache_key.clone()); // ❌ FLAG SET HERE
}

// Start timeout cleanup for this request (re-enabled to prevent stuck requests)
clear_stuck_request_after_timeout(cache_key.clone());

// ... file loading check ...
let (file_min, file_max) = {
    let loaded_files = LOADED_FILES.lock_ref();
    if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path || file_path.ends_with(&f.filename)) {
        // ... success path ...
    } else {
        // ❌ CRITICAL BUG: Early return without clearing flag
        zoon::println!("=== FILE NOT LOADED YET - cannot request transitions for {} ===", file_path);
        return; // FLAG NEVER GETS CLEARED!
    }
};
```

### System Complexity

The current deduplication system involves:
- `REQUESTS_IN_PROGRESS`: HashSet tracking active requests
- `REQUEST_TIMEOUTS`: HashMap tracking request timestamps  
- `clear_stuck_request_after_timeout()`: Async cleanup after 5 seconds
- Multiple code paths that must remember to clear flags
- Race conditions between sync flag setting and async message sending

**Total complexity**: ~50 lines of error-prone state management

## Solution A: Complete Removal (Immediate Fix)

### Approach
Remove all deduplication complexity entirely. Let the backend handle duplicate requests naturally.

### Benefits
- ✅ Fixes the critical bug immediately
- ✅ Removes 50+ lines of complex, error-prone code
- ✅ Zero chance of deadlocks or stuck states
- ✅ Simple and reliable
- ✅ Backend already handles duplicates efficiently

### Code Changes Required

#### 1. Remove Static State (lines 23-55)
```rust
// DELETE THESE:
pub static REQUESTS_IN_PROGRESS: Lazy<Mutable<std::collections::HashSet<String>>> = ...;
pub static REQUEST_TIMEOUTS: Lazy<Mutable<std::collections::HashMap<String, f64>>> = ...;

fn clear_stuck_request_after_timeout(cache_key: String) {
    // DELETE ENTIRE FUNCTION
}
```

#### 2. Simplify request_signal_transitions_from_backend (lines 844-862)
```rust
// REPLACE:
pub fn request_signal_transitions_from_backend(file_path: &str, scope_path: &str, variable_name: &str, _time_range: (f32, f32)) {
    
    // Smart request deduplication - prevent duplicate requests for same data
    let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    
    // TEMPORARILY DISABLE deduplication again - infinite recursion still exists
    // { ... complex deduplication logic ... }
    
    // Start timeout cleanup for this request (re-enabled to prevent stuck requests)
    clear_stuck_request_after_timeout(cache_key.clone());

// WITH:
pub fn request_signal_transitions_from_backend(file_path: &str, scope_path: &str, variable_name: &str, _time_range: (f32, f32)) {
    zoon::println!("=== Requesting signal transitions for {}/{} ===", scope_path, variable_name);
```

#### 3. Clean up connection.rs (lines 261-274)
```rust
// SIMPLIFY:
DownMsg::SignalTransitions { file_path, results } => {
    zoon::println!("=== SIGNAL TRANSITIONS RECEIVED: {} results for {} ===", results.len(), file_path);
    
    // Process real signal transitions from backend - UPDATE CACHE
    {
        let mut requests_in_progress = crate::waveform_canvas::REQUESTS_IN_PROGRESS.lock_mut(); // DELETE
        for result in results {
            let cache_key = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
            
            zoon::println!("=== INSERTING TO CACHE: {} with {} transitions ===", cache_key, result.transitions.len());
            
            // Store real backend data in canvas cache
            crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.lock_mut()
                .insert(cache_key.clone(), result.transitions);
            
            // Clear request from in-progress tracking now that data is stored // DELETE
            requests_in_progress.remove(&cache_key); // DELETE
        }
    }
    
    // CRITICAL: Trigger canvas redraw when transition data arrives
    crate::waveform_canvas::trigger_canvas_redraw();
}

// TO:
DownMsg::SignalTransitions { file_path, results } => {
    zoon::println!("=== SIGNAL TRANSITIONS RECEIVED: {} results for {} ===", results.len(), file_path);
    
    // Process signal transitions from backend - UPDATE CACHE
    for result in results {
        let cache_key = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
        
        zoon::println!("=== INSERTING TO CACHE: {} with {} transitions ===", cache_key, result.transitions.len());
        
        // Store backend data in cache
        crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.lock_mut()
            .insert(cache_key, result.transitions);
    }
    
    // Trigger canvas redraw when data arrives
    crate::waveform_canvas::trigger_canvas_redraw();
}
```

## Solution C: Cache-Based Deduplication (Future Enhancement)

### Approach
Use only the existing `SIGNAL_TRANSITIONS_CACHE` for natural deduplication. No complex state tracking.

### Design Principles
- If data exists in cache → use it (natural deduplication)
- If cache miss → send request (allow duplicates during loading)
- When response arrives → update cache
- Cache TTL or invalidation for fresh data when needed

### Benefits
- ✅ Natural deduplication through caching
- ✅ No complex state management
- ✅ No race conditions or deadlocks
- ✅ Simple and maintainable

### Implementation Approach
```rust
pub fn get_signal_transitions_for_variable(...) -> Vec<(f32, String)> {
    let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    
    // Check cache first
    let cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
    if let Some(transitions) = cache.get(&cache_key) {
        // Cache hit - use existing data
        return process_transitions(transitions, time_range);
    }
    drop(cache);
    
    // Cache miss - request from backend (no deduplication)
    request_signal_transitions_from_backend(file_path, scope_path, variable_name, time_range);
    
    // Return empty while waiting for backend
    vec![]
}
```

### Optional Enhancements for Solution C
- **Timestamp-based invalidation**: Add cache timestamps for automatic refresh
- **Request throttling**: Simple time-based throttling (max 1 request per 100ms per signal)
- **Batch requests**: Group multiple signal requests into single backend call

## Performance Analysis

### Solution A Impact
- **Duplicate requests**: Minimal - backend is fast, cache prevents most duplicates naturally
- **Network overhead**: Negligible - requests are small, responses cached
- **Memory usage**: Reduced by removing tracking HashMaps
- **CPU usage**: Reduced by removing timeout management

### Backend Deduplication
The backend already handles duplicate requests efficiently:
- Fast cache lookups for repeated queries
- Optimized file parsing (parse once, query many times)
- Async processing doesn't block on duplicates

## Recommendation

**Implement Solution A immediately** to fix the critical bug, then optionally implement Solution C later if performance analysis shows deduplication is needed.

The current system is broken - **the timeline shows no signal data because of this bug**. Solution A provides immediate relief with zero risk.

## Testing Plan

After implementing Solution A:
1. Load waveform files with signal data
2. Select variables in the scope tree
3. Verify signal transitions appear on timeline immediately
4. Test multiple rapid variable selections (stress test duplicates)
5. Monitor backend logs for duplicate request handling
6. Measure performance impact (should be minimal)

If Solution C becomes necessary:
1. Implement simple cache-based approach
2. Add performance metrics (request counts, cache hit rates)
3. Compare performance with Solution A baseline
4. Iterate on optimizations as needed

## Files Modified

- `frontend/src/waveform_canvas.rs`: Remove deduplication system
- `frontend/src/connection.rs`: Simplify response handling
- `docs/remove_request_deduplication_complexity.md`: This documentation