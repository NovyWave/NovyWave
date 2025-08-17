# Enhanced Sub-Pixel Transition Visibility & Controls

## Overview
This document outlines the implementation plan for ensuring waveform transitions remain visible at all zoom levels and improving timeline controls for working with files that have vastly different time scales (nanoseconds to seconds).

## Problem Statement
1. Current zoom limit (16x) is insufficient for viewing files with different time units (ns vs seconds)
2. Rapid transitions become invisible when zoomed out (sub-pixel rendering issue)
3. Q/E cursor movement has hardcoded minimum step (0.05s) that doesn't work for nanosecond-scale files
4. No fast zoom option for quickly navigating between vastly different time scales

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
let zoom_multiplier = if event.shift_key() {
    1.10  // Fast zoom with Shift
} else {
    1.02  // Normal smooth zoom
};
```

**Key Changes:**
- Detect shift key state in KeyDown event
- Apply different zoom rates:
  - Normal W/S: 1.02x per frame (smooth, precise)
  - Shift+W/S: 1.10x per frame (fast navigation)
- Update both `start_smooth_zoom_in()` and `start_smooth_zoom_out()`

### 3. Fix Q/E Cursor Movement for Small Timescales
**Goal:** Make cursor movement proportional to visible time range without artificial limits

**Current Problem:**
```rust
// Line 953 in waveform_canvas.rs
let step_size = (visible_range * 0.005).max(0.05);  // Hardcoded 0.05s minimum
```

**Solution:**
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

### 4. Sub-Pixel Transition Visibility Indicators
**Goal:** Ensure transitions are always visible, even when multiple occur within a single pixel

#### Option A: Vertical Transition Lines (Recommended)
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

#### Option B: Density Indicators
**Implementation:**
```rust
// Track transition density
let transitions_per_pixel = time_value_pairs.len() as f32 / canvas_width;

if transitions_per_pixel > 1.0 {
    // Add hatching or stippling pattern
    for i in (0..canvas_width as i32).step_by(4) {
        objects.push(
            fast2d::Rectangle::new()
                .position(i as f32, y_position + row_height * 0.3)
                .size(1.0, row_height * 0.4)
                .color(density_indicator_color)
                .into()
        );
    }
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

## Expected Outcomes

1. **Seamless Multi-Scale Work:** Users can work with ns-scale and s-scale files in same session
2. **No Hidden Data:** All transitions visible at all zoom levels through visual indicators
3. **Efficient Navigation:** Shift+zoom for rapid scale changes, normal zoom for precision
4. **Accurate Cursor Control:** Q/E works correctly regardless of time scale
5. **Professional Appearance:** Subtle but clear indicators for sub-pixel transitions

## Implementation Priority

1. **High Priority:**
   - Increase zoom limit (enables basic multi-scale work)
   - Fix Q/E cursor movement (critical for usability)
   - Basic vertical line indicators for sub-pixel transitions

2. **Medium Priority:**
   - Shift+W/S fast zoom
   - Isolated spike detection

3. **Low Priority:**
   - Density hatching patterns
   - Advanced visual indicators

## Notes

- Consider making zoom limit configurable in settings
- May need to optimize rendering for extreme zoom levels
- Consider adding zoom level indicator in UI
- Future: Add "fit to view" button to auto-zoom to show all transitions