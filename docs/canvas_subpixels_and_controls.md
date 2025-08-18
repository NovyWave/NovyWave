# Enhanced Sub-Pixel Transition Visibility & Controls

## Overview
This document outlines the implementation plan for ensuring waveform transitions remain visible at all zoom levels and improving timeline controls for working with files that have vastly different time scales (nanoseconds to seconds).

## Problem Statement
1. Current zoom limit (16x) is insufficient for viewing files with different time units (ns vs seconds)
2. Rapid transitions become invisible when zoomed out (sub-pixel rendering issue)
3. Q/E cursor movement has hardcoded minimum step (0.05s) that doesn't work for nanosecond-scale files
4. No fast zoom option for quickly navigating between vastly different time scales
5. No way to quickly jump to next/previous transition (essential for debugging)
6. Performance concerns when rendering thousands of sub-pixel transitions

## Implementation Plan

### 1. Increase Maximum Zoom Level
**Goal:** Enable viewing from nanoseconds to seconds scale within the same session

**Changes Required:**
- Change MAX_ZOOM from 16.0 to 10000.0 (or higher)
- Update in 3 locations:
  - `frontend/src/state.rs`: TIMELINE_ZOOM_LEVEL initialization comment
  - `frontend/src/config.rs`: Line 1106 zoom validation `.min(16.0)`
  - `frontend/src/waveform_canvas.rs`: Lines 824, 995 zoom limit checks

**Rationale:** 
- Users need to work with files ranging from nanoseconds (stress_test.vcd) to seconds (simple.vcd)
- 10000x zoom allows viewing 1ns to 10μs in the same viewport as 0-100s

### 2. Add Shift+W/S for Faster Zoom
**Goal:** Provide rapid zoom for quickly navigating between time scales

**Implementation:**
```rust
// In main.rs KeyDown handler
// Note: Need to track Shift state with KeyDown/KeyUp for "Shift" key
// as Zoon may not provide shift_key() method on events
let zoom_multiplier = if IS_SHIFT_PRESSED.get() {
    1.10  // Fast zoom with Shift
} else {
    1.02  // Normal smooth zoom
};
```

**Key Changes:**
- Track Shift key state separately with KeyDown/KeyUp events
- Apply different zoom rates:
  - Normal W/S: 1.02x per frame (smooth, precise)
  - Shift+W/S: 1.10x per frame (fast navigation)
- Update both `start_smooth_zoom_in()` and `start_smooth_zoom_out()`

### 3. Fix Q/E Cursor Movement & Add Transition Jumping
**Goal:** Make cursor movement work at all timescales AND add direct transition jumping

**Current Problem:**
```rust
// Line 953 in waveform_canvas.rs
let step_size = (visible_range * 0.005).max(0.05);  // Hardcoded 0.05s minimum
```

**Solution Part A - Fix Smooth Movement:**
```rust
// Calculate appropriate minimum based on time scale
let time_unit_min = detect_time_unit_minimum(visible_range);
let step_size = (visible_range * 0.005).max(time_unit_min);

fn detect_time_unit_minimum(range: f32) -> f32 {
    if range < 0.000001 {      // Nanosecond range
        0.000000001  // 1 picosecond minimum
    } else if range < 0.001 {   // Microsecond range  
        0.000001     // 1 nanosecond minimum
    } else if range < 1.0 {     // Millisecond range
        0.001        // 1 microsecond minimum
    } else {
        0.01         // 10 millisecond minimum for second+ ranges
    }
}
```

**Solution Part B - Add Shift+Q/E for Transition Jumping (NEW):**
```rust
// Much more useful for debugging - jump directly to transitions
pub fn jump_to_previous_transition() {
    let current_pos = TIMELINE_CURSOR_POSITION.get();
    let transitions = get_all_transitions_sorted();
    
    // Find previous transition before current position
    if let Some(prev_transition) = transitions.iter()
        .rev()
        .find(|t| t.time < current_pos - 0.0001) {
        TIMELINE_CURSOR_POSITION.set_neq(prev_transition.time);
    }
}

pub fn jump_to_next_transition() {
    let current_pos = TIMELINE_CURSOR_POSITION.get();
    let transitions = get_all_transitions_sorted();
    
    // Find next transition after current position
    if let Some(next_transition) = transitions.iter()
        .find(|t| t.time > current_pos + 0.0001) {
        TIMELINE_CURSOR_POSITION.set_neq(next_transition.time);
    }
}
```

**Key Bindings:**
- Q/E: Smooth cursor movement (fixed for all timescales)
- Shift+Q/E: Jump to previous/next transition (essential for debugging)

### 4. Sub-Pixel Transition Visibility Indicators with Performance Optimization
**Goal:** Ensure transitions are always visible while maintaining performance with thousands of transitions

#### Transition Density Calculation
```rust
// Calculate density for smart rendering decisions
fn calculate_transition_density(start_time: f32, end_time: f32, canvas_width: f32) -> f32 {
    let transitions_in_view = count_transitions_in_range(start_time, end_time);
    let pixels_available = canvas_width;
    transitions_in_view as f32 / pixels_available
}

// Use density for rendering strategy
let density = calculate_transition_density(visible_start, visible_end, canvas_width);
if density > 10.0 {
    render_activity_bands();  // Heat map or color bands
} else if density > 1.0 {
    render_vertical_lines();  // Vertical line indicators
} else {
    render_normal_rectangles();  // Standard rectangles
}
```

#### Option A: Vertical Transition Lines (Recommended for 1-10 transitions/pixel)
**Implementation:**
```rust
// After calculating rect_width in waveform_canvas.rs
if raw_rect_width < 2.0 {
    // Draw thin vertical line at transition point
    objects.push(
        fast2d::Rectangle::new()
            .position(rect_start_x, y_position)
            .size(1.0, row_height)
            .color(accent_color)  // Bright, contrasting color
            .into()
    );
} else {
    // Draw normal rectangle
    // ... existing code
}
```

#### Option B: Activity Bands (For >10 transitions/pixel)
**Implementation:**
```rust
// For extremely dense transitions, show activity level instead of individual transitions
if density > 10.0 {
    // Use color gradient to show activity level
    let activity_color = match density {
        d if d > 100.0 => (255, 0, 0, 0.8),    // Red: extreme activity
        d if d > 50.0 => (255, 165, 0, 0.8),   // Orange: high activity
        d if d > 10.0 => (255, 255, 0, 0.8),   // Yellow: moderate activity
        _ => (0, 128, 255, 0.8),               // Blue: low activity
    };
    
    // Draw activity band
    objects.push(
        fast2d::Rectangle::new()
            .position(0.0, y_position)
            .size(canvas_width, row_height)
            .color(activity_color)
            .into()
    );
    
    // Add "≈" symbol to indicate approximation
    objects.push(
        fast2d::Text::new()
            .text("≈")
            .position(5.0, y_position + row_height / 2.0)
            .color(text_color)
            .into()
    );
}
```

#### Isolated Spike Detection
**Special Case:** Single transition in otherwise flat signal
```rust
// Check if this is an isolated transition
let is_isolated = (rect_index == 0 || time_value_pairs[rect_index - 1].1 != binary_value) &&
                  (rect_index + 1 >= time_value_pairs.len() || 
                   time_value_pairs[rect_index + 1].1 != binary_value);

if is_isolated && raw_rect_width < 1.0 {
    // Always show isolated transitions with minimum 2px width
    rect_width = 2.0;
}
```

#### Performance Optimization: Transition Culling
```rust
// Don't create Fast2D objects for invisible transitions
if raw_rect_width < 0.1 && !is_isolated {
    // Skip transitions smaller than 0.1 pixels unless isolated
    continue;
}

// Viewport-based rendering with buffer
let buffer = 100.0;  // pixels
if rect_end_x < -buffer || rect_start_x > canvas_width + buffer {
    continue;  // Skip off-screen transitions
}
```

## Testing Strategy

### Test Scenario 1: Multi-Scale Navigation
1. Load stress_test.vcd (0-9μs range, 1ns transitions)
2. Select variables: toggle_fast, byte_pattern, word_counter
3. Load simple.vcd in same session
4. Select variable A (0-200s range, 50s transitions)
5. Test zooming between these scales (9 orders of magnitude difference)

### Test Scenario 2: Zoom Performance
- Test normal zoom (W/S) for precision
- Test Shift+W/S for rapid scale changes
- Verify zoom limits work correctly at extremes

### Test Scenario 3: Cursor Movement
- Test Q/E in nanosecond range (should move by ns, not 0.05s)
- Test Q/E in second range (should move appropriately)
- Verify cursor stays visible and moves smoothly

### Test Scenario 4: Transition Visibility
- Zoom out on stress_test.vcd until transitions merge
- Verify visual indicators appear for dense transitions
- Check isolated spikes remain visible
- Test with both light and dark themes

## Additional Features

### 5. Zoom Level UI Indicator (HIGH PRIORITY)
**Goal:** Users need to see current zoom level for context

**Implementation:**
```rust
// Add to timeline header area
El::new()
    .s(Padding::new().x(8).y(4))
    .s(Background::new().color_signal(neutral_3()))
    .child(Text::with_signal(
        TIMELINE_ZOOM_LEVEL.signal().map(|zoom| {
            if zoom < 1000.0 {
                format!("{:.1}×", zoom)
            } else {
                format!("{:.1}k×", zoom / 1000.0)  // "1.5k×" for 1523x
            }
        })
    ))
```

### 6. Memory Management Considerations
**Goal:** Prevent memory issues with extreme zoom levels

**Strategies:**
- Don't cache all zoom levels - use sliding window cache
- Clear transition cache when switching files
- Implement maximum cache size (e.g., 100MB)
- Use weak references for old cache entries

```rust
// Sliding window cache for recently viewed ranges
struct TransitionCache {
    max_entries: usize,  // e.g., 100
    cache: LruCache<CacheKey, Vec<Transition>>,
}
```

## Future Enhancements (Not for Initial Implementation)

### Zoom Presets (Keys 1-9)
**Goal:** Quick navigation to common time scales

**Concept:**
```rust
// Quick zoom presets - implement later
match key {
    "1" => fit_all_data_to_view(),        // Auto-fit
    "2" => set_view_range_nanoseconds(),   // 0-100ns view
    "3" => set_view_range_microseconds(),  // 0-100μs view
    "4" => set_view_range_milliseconds(),  // 0-100ms view
    "5" => set_view_range_seconds(),       // 0-100s view
    "0" => reset_zoom_to_1x(),            // Reset
    _ => {}
}
```

### Mixed Timescale Handling
**Challenge:** Multiple files with vastly different timescales loaded simultaneously

**Potential Solutions:**
1. **Per-File Zoom:** Each file gets its own zoom factor
2. **Adaptive Timeline:** Show different time units per row
3. **Split Views:** Allow multiple timeline views with different scales
4. **Smart Scaling:** Automatically normalize to common unit when files differ by >1000x

**Example Implementation Concept:**
```rust
// Detect timescale mismatch
let timescales: Vec<f64> = loaded_files.iter()
    .map(|f| f.timescale_factor)
    .collect();

let max_ratio = timescales.iter().max() / timescales.iter().min();
if max_ratio > 1000.0 {
    // Enable adaptive scaling mode
    use_adaptive_timeline_scaling();
}
```

### Edge Cases to Consider
1. **Single Transition Files:** Auto-zoom to show the transition clearly
2. **Empty Signals:** Show appropriate message instead of blank canvas
3. **Extreme Zoom:** Gracefully handle zoom levels beyond float precision

## Expected Outcomes

1. **Seamless Multi-Scale Work:** Users can work with ns-scale and s-scale files in same session
2. **No Hidden Data:** All transitions visible at all zoom levels through visual indicators
3. **Efficient Navigation:** Shift+zoom for rapid scale changes, normal zoom for precision
4. **Accurate Cursor Control:** Q/E works correctly regardless of time scale
5. **Professional Appearance:** Subtle but clear indicators for sub-pixel transitions
6. **Instant Debugging:** Shift+Q/E for jumping between transitions
7. **Performance at Scale:** Smooth rendering even with millions of transitions
8. **Clear Context:** Always know zoom level and transition density

## Implementation Priority

1. **Critical First Implementation:**
   - Increase zoom limit to 10000x (enables multi-scale work)
   - Fix Q/E cursor movement minimum step (critical for ns-scale files)
   - Add Shift+Q/E transition jumping (essential debugging feature)
   - Add zoom level UI indicator (users need context)
   - Basic vertical line indicators for sub-pixel transitions

2. **Essential Second Wave:**
   - Shift+W/S fast zoom (with Shift key state tracking)
   - Performance optimization (transition culling, viewport rendering)
   - Density-based rendering strategy (vertical lines vs activity bands)
   - Isolated spike detection and rendering

3. **Polish Third Wave:**
   - Memory management optimizations
   - Activity band color gradients
   - "≈" approximation indicators
   - Edge case handling (single transition files, empty signals)

4. **Future Enhancements (Documented but not implemented):**
   - Zoom presets (1-9 keys)
   - Mixed timescale handling strategies
   - Per-file zoom factors
   - Auto-fit functionality

## Implementation Notes

- **Shift Key Tracking:** MoonZoon/Zoon may not provide shift_key() on events, need to track state
- **Performance Critical:** With 10000x zoom, must implement culling to avoid rendering thousands of invisible objects
- **Density Threshold:** Use 10 transitions/pixel as cutoff between individual lines and activity bands
- **Cache Strategy:** Don't cache everything - use LRU or sliding window approach
- **Testing Important:** Must test with both ns-scale (stress_test.vcd) and s-scale (simple.vcd) files together
- **Consider Float Precision:** At extreme zoom levels, may hit float32 precision limits