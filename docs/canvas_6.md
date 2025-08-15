# Canvas 6: Smooth Zoom Implementation

## Overview

Implementation plan for smooth zoom improvements in the NovyWave timeline canvas, addressing stuttering zoom behavior and adding zoom percentage display.

## Current Issues

1. **Stuttering Zoom**: W/S keys rely on OS key repeat with large 1.5x steps causing stuttering
2. **Missing Zoom Percentage**: No visual feedback of current zoom level
3. **Suboptimal Mouse Center**: Current mouse-based zoom center can be improved

## Improvement Goals

### 1. Smooth Zoom Animation
- Replace discrete 1.5x zoom steps with smooth 1.02x increments
- Implement timer-based continuous zoom (60fps updates)
- Use KeyDown/KeyUp events instead of OS key repeat
- Add natural easing for professional feel

### 2. Enhanced Mouse-Based Zoom Center
- Real-time mouse position tracking over timeline canvas
- Accurate mouse coordinate to timeline time conversion
- Pixel-perfect zoom center maintenance during animation
- Improved zoom center calculations

### 3. Zoom Percentage Display
- Show zoom percentage next to selected time in footer
- Real-time updates during zoom operations
- Hide at 100% zoom, show when zoomed in
- Consistent styling with existing footer elements

## Technical Implementation

### Current Zoom Architecture

**State Management** (`frontend/src/state.rs`):
```rust
static TIMELINE_ZOOM_LEVEL: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(1.0));
static TIMELINE_VISIBLE_RANGE_START: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(10.0));
static TIMELINE_VISIBLE_RANGE_END: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(20.0));
static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(10.0));
```

**Current Zoom Functions** (`frontend/src/waveform_canvas.rs:384-406`):
```rust
pub fn zoom_in() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let new_zoom = (current_zoom * 1.5).min(16.0); // Large steps cause stuttering
    update_zoom_level_and_visible_range(new_zoom);
}

pub fn zoom_out() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let new_zoom = (current_zoom / 1.5).max(1.0);
    update_zoom_level_and_visible_range(new_zoom);
}
```

**Current Keyboard Handler** (`frontend/src/main.rs:315-327`):
```rust
.update_raw_el(move |raw_el| {
    raw_el.global_event_handler(move |event: zoon::events::KeyDown| {
        match event.key().as_str() {
            "w" | "W" => crate::waveform_canvas::zoom_in(),
            "s" | "S" => crate::waveform_canvas::zoom_out(),
            _ => {}
        }
    })
})
```

### New Implementation Plan

#### 1. Smooth Zoom Animation System

**Add New State Variables** (`frontend/src/state.rs`):
```rust
// Smooth zoom control
static IS_ZOOMING_IN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static IS_ZOOMING_OUT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Mouse position tracking
static MOUSE_X_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(0.0));
static MOUSE_TIME_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(10.0));
```

**Replace Zoom Functions** (`frontend/src/waveform_canvas.rs`):
```rust
pub fn start_smooth_zoom_in() {
    if !IS_ZOOMING_IN.get() {
        IS_ZOOMING_IN.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_IN.get() {
                let current = TIMELINE_ZOOM_LEVEL.get();
                let new_zoom = (current * 1.02).min(16.0); // Smaller increments
                if new_zoom != current {
                    update_zoom_with_mouse_center(new_zoom);
                }
                Timer::sleep(16).await; // 60fps updates
            }
        });
    }
}

pub fn start_smooth_zoom_out() {
    if !IS_ZOOMING_OUT.get() {
        IS_ZOOMING_OUT.set_neq(true);
        Task::start(async move {
            while IS_ZOOMING_OUT.get() {
                let current = TIMELINE_ZOOM_LEVEL.get();
                let new_zoom = (current / 1.02).max(1.0);
                if new_zoom != current {
                    update_zoom_with_mouse_center(new_zoom);
                }
                Timer::sleep(16).await; // 60fps updates
            }
        });
    }
}

pub fn stop_smooth_zoom_in() {
    IS_ZOOMING_IN.set_neq(false);
}

pub fn stop_smooth_zoom_out() {
    IS_ZOOMING_OUT.set_neq(false);
}
```

**Enhanced Keyboard Handler** (`frontend/src/main.rs`):
```rust
.update_raw_el(move |raw_el| {
    raw_el.global_event_handler(move |event: zoon::events::KeyDown| {
        match event.key().as_str() {
            "w" | "W" => crate::waveform_canvas::start_smooth_zoom_in(),
            "s" | "S" => crate::waveform_canvas::start_smooth_zoom_out(),
            _ => {}
        }
    })
    .global_event_handler(move |event: zoon::events::KeyUp| {
        match event.key().as_str() {
            "w" | "W" => crate::waveform_canvas::stop_smooth_zoom_in(),
            "s" | "S" => crate::waveform_canvas::stop_smooth_zoom_out(),
            _ => {}
        }
    })
})
```

#### 2. Enhanced Mouse-Based Zoom Center

**Mouse Position Tracking** (`frontend/src/waveform_canvas.rs`):
```rust
// Add to canvas element
.event_handler(move |event: events::PointerMove| {
    let canvas_element = event.target()
        .dyn_cast::<web_sys::Element>()
        .expect("Event target is not an element");
    
    let canvas_rect = canvas_element.get_bounding_client_rect();
    let relative_x = event.client_x() as f32 - canvas_rect.left();
    
    MOUSE_X_POSITION.set_neq(relative_x);
    
    // Convert mouse X to timeline time
    let canvas_width = CANVAS_WIDTH.get();
    let (min_time, max_time) = get_current_timeline_range();
    let time_range = max_time - min_time;
    let mouse_time = min_time + (relative_x / canvas_width) * time_range;
    
    MOUSE_TIME_POSITION.set_neq(mouse_time);
})
```

**Mouse-Centered Zoom Calculation**:
```rust
fn update_zoom_with_mouse_center(new_zoom: f32) {
    let mouse_time = MOUSE_TIME_POSITION.get();
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    
    // Get current visible range
    let (current_start, current_end) = get_current_timeline_range();
    let current_range = current_end - current_start;
    
    // Calculate new range
    let new_range = current_range * (current_zoom / new_zoom);
    
    // Position mouse time as the center of zoom
    let mouse_ratio = (mouse_time - current_start) / current_range;
    let new_start = mouse_time - (new_range * mouse_ratio);
    let new_end = new_start + new_range;
    
    // Update zoom and visible range with bounds checking
    TIMELINE_ZOOM_LEVEL.set_neq(new_zoom);
    TIMELINE_VISIBLE_RANGE_START.set_neq(new_start.max(0.0));
    TIMELINE_VISIBLE_RANGE_END.set_neq(new_end);
}
```

#### 3. Zoom Percentage Display

**Footer Integration** (`frontend/src/views.rs:1277-1287`):
```rust
// Extend existing footer row
Row::new()
    .s(Gap::new().x(8))
    .s(Align::new().center_y())
    .item(
        // Existing selected time display
        Text::new_signal(
            TIMELINE_CURSOR_POSITION.signal().map(|cursor_pos| {
                format!("Selected: {}s", cursor_pos)
            })
        )
        .s(Font::new().color_signal(neutral_11()))
    )
    .item(
        // NEW: Zoom percentage display
        Text::new_signal(
            TIMELINE_ZOOM_LEVEL.signal().map(|zoom_level| {
                let percentage = (zoom_level * 100.0) as u32;
                format!("Zoom: {}%", percentage)
            })
        )
        .s(Font::new().color_signal(neutral_8().signal()))
        .s(Font::new().size(11))
    )
```

#### 4. Performance Optimizations

**Signal Throttling**:
```rust
// In waveform_canvas.rs initialization
fn init_smooth_zoom_handlers() {
    Task::start(async {
        TIMELINE_ZOOM_LEVEL.signal()
            .dedupe() // Prevent duplicate triggers
            .throttle(Duration::milliseconds(16)) // 60fps throttling
            .for_each_sync(|_zoom_level| {
                // Trigger canvas redraw with new zoom
                trigger_canvas_redraw();
            }).await
    });
}
```

## Implementation Steps

### Phase 1: Core Smooth Zoom
1. Add new state variables for smooth zoom control
2. Implement smooth zoom functions with timer-based animation
3. Replace keyboard event handlers with KeyDown/KeyUp pattern
4. Test smooth zoom behavior with W/S keys

### Phase 2: Enhanced Mouse Center
1. Add mouse position tracking to canvas
2. Implement mouse-to-time coordinate conversion
3. Create mouse-centered zoom calculation function
4. Test zoom center accuracy

### Phase 3: Zoom Display
1. Add zoom percentage to footer layout
2. Style consistently with existing elements
3. Test real-time updates during zoom operations

### Phase 4: Optimization & Polish
1. Add signal throttling for performance
2. Implement zoom bounds checking
3. Add easing functions for natural feel
4. Comprehensive testing of all zoom scenarios

## Testing Scenarios

1. **Smooth Zoom**: Hold W/S keys - should zoom smoothly without stuttering
2. **Mouse Center**: Zoom in/out - mouse position should remain fixed
3. **Zoom Display**: Watch footer - percentage should update in real-time
4. **Performance**: Rapid zoom - should maintain 60fps without lag
5. **Bounds**: Test min (100%) and max (1600%) zoom limits
6. **Edge Cases**: Mouse at canvas edges, very fast zoom operations

## Expected Results

- **Smooth 60fps zoom** with natural easing and small increments
- **Pixel-perfect mouse-centered zooming** maintaining cursor position
- **Real-time zoom percentage display** in footer next to selected time
- **Professional user experience** matching modern design tools
- **Maintained performance** with optimized signal handling

## File Modifications Summary

| File | Changes | Lines |
|------|---------|-------|
| `frontend/src/state.rs` | Add mouse tracking and zoom control states | ~10 |
| `frontend/src/waveform_canvas.rs` | Replace zoom functions, add mouse tracking | ~80 |
| `frontend/src/main.rs` | Update keyboard handlers | ~20 |
| `frontend/src/views.rs` | Add zoom percentage to footer | ~10 |

**Total estimated changes**: ~120 lines across 4 files
**Implementation time**: 2-3 hours
**Testing time**: 1 hour