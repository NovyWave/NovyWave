# Debug Code for Missing Waveform Transitions Issue

## Root Cause Analysis

Based on code investigation, the issue is likely in one of these areas:

1. **Backend data filtering** - Backend may be filtering out "0" value transitions
2. **Frontend cache processing** - Data gets lost during conversion from backend format to canvas format  
3. **Rectangle rendering logic** - "0" value rectangles are being skipped during drawing
4. **Time range calculations** - "0" transitions occur outside the visible timeline range

## Debug Code to Add

### 1. Debug Backend Data Reception (connection.rs line 219-232)

```rust
DownMsg::SignalTransitions { file_path, results } => {
    // DEBUGGING: Log all received transitions
    zoon::println!("=== RECEIVED SIGNAL TRANSITIONS ===");
    zoon::println!("File path: {}", file_path);
    
    for result in &results {
        zoon::println!("Signal: {}|{}", result.scope_path, result.variable_name);
        zoon::println!("Transition count: {}", result.transitions.len());
        
        for (i, transition) in result.transitions.iter().enumerate() {
            zoon::println!("  [{}] Time: {}s, Value: '{}'", 
                i, transition.time_seconds, transition.value);
        }
    }
    
    // Process real signal transitions from backend - UPDATE CACHE
    for result in results {
        let cache_key = format!("{}|{}|{}", file_path, result.scope_path, result.variable_name);
        
        // Store real backend data in canvas cache
        crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.lock_mut()
            .insert(cache_key, result.transitions);
    }
    
    // Trigger canvas redraw to show real data
    crate::waveform_canvas::trigger_canvas_redraw();
}
```

### 2. Debug Cache Data Retrieval (waveform_canvas.rs line 198-265)

```rust
fn get_signal_transitions_for_variable(var: &SelectedVariable, time_range: (f32, f32)) -> Vec<(f32, String)> {
    // Parse unique_id: "/path/file.ext|scope|variable"
    let parts: Vec<&str> = var.unique_id.split('|').collect();
    if parts.len() < 3 {
        return vec![(time_range.0, "0".to_string())];
    }
    
    let file_path = parts[0];
    let scope_path = parts[1]; 
    let variable_name = parts[2];
    
    // Create cache key for this specific signal
    let cache_key = format!("{}|{}|{}", file_path, scope_path, variable_name);
    
    zoon::println!("=== GETTING SIGNAL TRANSITIONS ===");
    zoon::println!("Variable: {}", var.unique_id);
    zoon::println!("Cache key: {}", cache_key);
    zoon::println!("Time range: {} to {}", time_range.0, time_range.1);
    
    // Check if we have real backend data cached
    let cache = SIGNAL_TRANSITIONS_CACHE.lock_ref();
    if let Some(transitions) = cache.get(&cache_key) {
        zoon::println!("Found cached transitions count: {}", transitions.len());
        
        // Log all cached transitions
        for (i, t) in transitions.iter().enumerate() {
            zoon::println!("  Cached[{}] Time: {}s, Value: '{}'", i, t.time_seconds, t.value);
        }
        
        // Convert real backend data to canvas format with proper waveform logic
        let mut canvas_transitions: Vec<(f32, String)> = transitions.iter()
            .map(|t| (t.time_seconds as f32, t.value.clone()))
            .collect();
            
        // Sort by time to ensure proper ordering
        canvas_transitions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        zoon::println!("Canvas transitions after sorting:");
        for (i, (time, value)) in canvas_transitions.iter().enumerate() {
            zoon::println!("  Canvas[{}] Time: {}s, Value: '{}'", i, time, value);
        }
        
        // ... rest of existing logic
        
        return canvas_transitions;
    }
    drop(cache);
    
    zoon::println!("No cached data found - requesting from backend");
    
    // ... rest of existing logic
}
```

### 3. Debug Rectangle Rendering (waveform_canvas.rs line 384-448)

```rust
// Around line 384, add debug logging in the rectangle creation loop:

let time_value_pairs = get_signal_transitions_for_variable(var, current_time_range);

zoon::println!("=== RENDERING RECTANGLES FOR VARIABLE ===");
zoon::println!("Variable: {}", var.unique_id);
zoon::println!("Time-value pairs count: {}", time_value_pairs.len());

for (rect_index, (start_time, binary_value)) in time_value_pairs.iter().enumerate() {
    // Calculate end time for this rectangle (next transition time or total_time)
    let end_time = if rect_index + 1 < time_value_pairs.len() {
        time_value_pairs[rect_index + 1].0 // Next transition time
    } else {
        max_time // Last rectangle extends to visible end
    };
    
    zoon::println!("  Rect[{}] Start: {}s, End: {}s, Value: '{}' (visible range: {} to {})", 
        rect_index, start_time, end_time, binary_value, min_time, max_time);
    
    // Skip rectangles completely outside visible range
    if end_time <= min_time || *start_time >= max_time {
        zoon::println!("    -> SKIPPED (outside visible range)");
        continue;
    }
    
    // Clip rectangle to visible time range
    let visible_start_time = start_time.max(min_time);
    let visible_end_time = end_time.min(max_time);
    
    // Calculate rectangle position and width for visible portion
    let rect_start_x = ((visible_start_time - min_time) / (max_time - min_time)) * canvas_width;
    let rect_end_x = ((visible_end_time - min_time) / (max_time - min_time)) * canvas_width;
    let rect_width = rect_end_x - rect_start_x;
    
    zoon::println!("    -> RENDERED X: {} to {} (width: {}px)", rect_start_x, rect_end_x, rect_width);
    
    // ... rest of rectangle creation logic
}
```

## Expected Debug Output Pattern

If the issue is in **backend data**, you'll see:
```
=== RECEIVED SIGNAL TRANSITIONS ===
Signal: scope_path|signal_name
Transition count: 1
  [0] Time: 0s, Value: 'C'
  // Missing the transition to '0'
```

If the issue is in **cache processing**, you'll see:
```
=== GETTING SIGNAL TRANSITIONS ===
Found cached transitions count: 2
  Cached[0] Time: 0s, Value: 'C' 
  Cached[1] Time: 50s, Value: '0'
Canvas transitions after sorting:
  Canvas[0] Time: 0s, Value: 'C'
  // Missing Canvas[1] conversion
```

If the issue is in **rectangle rendering**, you'll see:
```
=== RENDERING RECTANGLES FOR VARIABLE ===
Time-value pairs count: 2
  Rect[0] Start: 0s, End: 50s, Value: 'C'
  Rect[1] Start: 50s, End: 100s, Value: '0'
    -> SKIPPED (outside visible range)  // This would be the problem
```

## Most Likely Root Cause

Based on the code analysis, the most likely issue is in **time range calculations**. The "0" value transitions are probably occurring at time positions that get filtered out by the visible time range logic at lines 401-404:

```rust
// Skip rectangles completely outside visible range
if end_time <= min_time || *start_time >= max_time {
    continue;
}
```

The backend might be sending transitions that extend beyond the current visible timeline, causing the "0" rectangles to be skipped during rendering.