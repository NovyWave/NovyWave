# Timeline Zoom Implementation Plan (W/S Keys)

## Research Summary

Based on comprehensive subagent analysis, here's the current timeline architecture and implementation strategy:

### **Current Timeline System**
- **No existing zoom functionality** - timeline always shows entire file range  
- **Professional timeline rendering** with 80px spacing algorithm and "nice number" tick intervals
- **Time-to-pixel conversion** uses global `(min_time, max_time)` from all loaded files
- **Interactive timeline** with click-to-set cursor position
- **Empty zoom buttons** exist but have no functionality

### **Configuration Architecture**
- **Dual-layer config** - frontend reactive state + shared crate persistence  
- **Timeline cursor position** already persisted using reactive triggers
- **Established pattern** for adding new config fields with `#[serde(default)]`

### **Keyboard Event System**
- **Global event handlers** used for document-level keyboard shortcuts
- **State guards** prevent interference between different UI modes
- **Main layout location** ideal for W/S key handlers

## Implementation Strategy

### **Core Architecture**
```rust
// New zoom state variables (will be added to state.rs)
TIMELINE_ZOOM_LEVEL: 1.0-16.0 (1.0 = normal, 16.0 = max zoom)  
TIMELINE_VISIBLE_RANGE_START: f32 (visible time window start)
TIMELINE_VISIBLE_RANGE_END: f32 (visible time window end)
```

### **Zoom Logic**
- **Center-focused zooming** - zoom towards timeline cursor position
- **Exponential scaling** - W multiplies by 1.5x, S divides by 1.5x  
- **Bounds checking** - min 1.0x (full range), max 16.0x zoom
- **Smooth integration** - existing timeline algorithm automatically adapts

### **Persistence Integration**
- **Reactive auto-save** - follows exact pattern as cursor position
- **Session restoration** - zoom state restored on app startup
- **Three-location updates** - shared crate + frontend config + loading logic

### **Key Features**
✅ **W key** - Zoom in (1.5x multiplier, max 16x)  
✅ **S key** - Zoom out (1.5x divisor, min 1x)  
✅ **Center-focused** - Zoom around cursor position  
✅ **Persistent** - Zoom level saved/restored between sessions  
✅ **Button integration** - Existing zoom buttons will work identically  
✅ **Professional timeline** - 80px spacing algorithm adapts automatically  

## Implementation Plan

### **Configuration Layer**
1. Add zoom state fields to shared/src/lib.rs WorkspaceSection with serde defaults
2. Add matching zoom fields to frontend config SerializableWorkspaceSection
3. Create zoom state globals in frontend/src/state.rs (zoom level, visible range)
4. Add zoom state restoration logic in load_from_serializable() method
5. Create reactive triggers for zoom state auto-saving following cursor position pattern

### **Timeline Integration**
6. Modify get_current_timeline_range() to respect zoom state instead of full file range
7. Implement zoom in/out functions with bounds checking and center-focused zooming

### **User Interface**
8. Add global W/S keyboard event handler to main layout function
9. Connect existing empty zoom buttons to new zoom functions

### **Testing & Verification**
10. Test zoom in/out with W/S keys and verify timeline scaling works correctly
11. Test zoom state persistence across app restarts
12. Verify compilation succeeds and use browser MCP to test UI behavior

### **Key Technical Details**

**Zoom Function Logic:**
```rust
fn zoom_in() {
    let current_zoom = TIMELINE_ZOOM_LEVEL.get();
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let new_zoom = (current_zoom * 1.5).min(16.0);
    
    // Update visible range centered on cursor
    update_visible_range_centered(cursor_pos, new_zoom);
}
```

**Timeline Range Override:**
```rust
fn get_current_timeline_range() -> (f32, f32) {
    let zoom = TIMELINE_ZOOM_LEVEL.get();
    if zoom > 1.0 {
        (TIMELINE_VISIBLE_RANGE_START.get(), TIMELINE_VISIBLE_RANGE_END.get())
    } else {
        // Existing full-range logic
    }
}
```

## Research Analysis Details

### **Key Files Involved in Timeline Rendering**

1. **`/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs`** - Main timeline rendering and canvas logic
2. **`/home/martinkavik/repos/NovyWave/frontend/src/state.rs`** - Timeline cursor position state management  
3. **`/home/martinkavik/repos/NovyWave/frontend/src/views.rs`** - Contains existing (empty) zoom buttons
4. **`/home/martinkavik/repos/NovyWave/shared/src/lib.rs`** - Configuration persistence including timeline_cursor_position

### **Current Timeline Architecture**

#### **Time-to-Pixel Coordinate Conversion**
Located in `waveform_canvas.rs`, the system uses a global time range approach:

```rust
// Line 326: Core time range calculation
fn get_current_timeline_range() -> (f32, f32) {
    // Gets min/max time from ALL loaded files
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    
    for file in loaded_files.iter() {
        if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
            min_time = min_time.min(file_min as f32);
            max_time = max_time.max(file_max as f32);
        }
    }
    // Returns (min_time, max_time) - the ENTIRE file time range
}

// Line 429: Time-to-pixel conversion
let rect_start_x = ((visible_start_time - min_time) / (max_time - min_time)) * canvas_width;
```

#### **Current Timeline State Management**
```rust
// state.rs line 22: Timeline cursor position
pub static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(10.0));

// state.rs line 25-26: Canvas dimensions
pub static CANVAS_WIDTH: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(800.0));
pub static CANVAS_HEIGHT: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(400.0));

// shared/lib.rs line 792-793: Config persistence
pub timeline_cursor_position: f32,
```

### **No Existing Zoom/Scale State**

**Critical Finding:** The system currently has **NO zoom or viewport management**. The timeline always shows the **entire file time range** from min_time to max_time.

Key evidence:
- `get_current_timeline_range()` always returns the full file range
- All coordinate calculations use the full range: `(max_time - min_time)`
- No zoom level, scale factor, or viewport window state exists
- The zoom buttons in `views.rs` are placeholders with empty handlers: `.on_press(|| {})`

### **Timeline Coordinate System Details**

#### **Professional Timeline Algorithm (Lines 486-529)**
```rust
// Target 80 pixels between timeline ticks
let target_tick_spacing = 80.0;
let max_tick_count = (canvas_width / target_tick_spacing).floor() as i32;

// Round to "nice numbers" (1-2-5-10 scaling)
let raw_time_step = time_range / (tick_count - 1) as f32;
let time_step = round_to_nice_number(raw_time_step);
```

#### **Canvas Click-to-Time Conversion (Lines 172-181)**
```rust
let (min_time, max_time) = get_current_timeline_range();
let time_range = max_time - min_time;
let clicked_time = min_time + (click_x / canvas_width) * time_range;
```

### **Fast2D Graphics Integration**

The system uses Fast2D for high-performance rendering:
- Canvas wrapper: `fast2d::CanvasWrapper::new_with_canvas(dom_canvas)`
- Objects: Rectangles, Text, with theme-aware colors
- Reactive redraws triggered by `SELECTED_VARIABLES`, theme changes, and canvas resize

## Current Keyboard Event Handling Patterns

### **Global Event Handler Pattern**
The codebase uses `global_event_handler` for document-level keyboard events. Found in `/home/martinkavik/repos/NovyWave/frontend/src/views.rs:829-840`:

```rust
.global_event_handler({
    let close_dialog = close_dialog.clone();
    move |event: KeyDown| {
        if SHOW_FILE_DIALOG.get() {  // Guard with state check
            if event.key() == "Escape" {
                close_dialog();
            } else if event.key() == "Enter" {
                process_file_picker_selection();
            }
        }
    }
})
```

**Key Pattern Elements:**
- Uses `global_event_handler` method on elements
- Event type: `KeyDown` from `zoon::events`
- State guards (`if SHOW_FILE_DIALOG.get()`) prevent interference
- String comparison for keys: `event.key() == "Escape"`

### **Best Location for Global W/S Key Handlers**

**Recommended Location: Main Layout Function**
The main layout function in `/home/martinkavik/repos/NovyWave/frontend/src/main.rs:230` is the ideal location because:

1. **Architectural Position**: It's the root container with `Height::screen()` and already handles global mouse events
2. **Existing Global Handlers**: Already has pointer event handlers for divider dragging
3. **Timeline Context**: TIMELINE_CURSOR_POSITION is imported and used throughout the app

## Configuration Persistence Analysis

### **Current Configuration Architecture**

**Frontend Config Store** (`/home/martinkavik/repos/NovyWave/frontend/src/config.rs`):
- Uses `ConfigStore` with reactive `Mutable` fields for live UI state
- Structured into sections: `app`, `ui`, `session`, `workspace`, `dialogs`
- Contains timeline-related configuration in `workspace.timeline_cursor_position`

**Shared/Backend Schema** (`/home/martinkavik/repos/NovyWave/shared/src/lib.rs`):
- `AppConfig` structure with `WorkspaceSection` for persistence
- Already includes `timeline_cursor_position: f32` field with reactive persistence

### **Existing Timeline Configuration Persistence**

**Currently Persisted Timeline Settings:**
```rust
// In shared/src/lib.rs - WorkspaceSection
pub timeline_cursor_position: f32,  // Line 792-793

// In frontend/src/state.rs - Global state
pub static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f32>> = Lazy::new(|| Mutable::new(10.0));

// In frontend/src/config.rs - Reactive persistence trigger  
let timeline_cursor_position_signal = crate::state::TIMELINE_CURSOR_POSITION.signal();
Task::start(async move {
    timeline_cursor_position_signal.for_each_sync(|_| {
        if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
            save_config_to_backend();
        }
    }).await
});
```

**Config File Example** (`.novywave`):
```toml
timeline_cursor_position = 93.2203369140625
```

### **Configuration Field Addition Pattern**

The established pattern for adding new config fields requires **THREE locations**:

#### Location 1: Backend Schema (`shared/src/lib.rs`)
```rust
pub struct WorkspaceSection {
    // Existing fields...
    pub timeline_cursor_position: f32,
    
    // ADD NEW ZOOM FIELDS HERE:
    #[serde(default)]
    pub timeline_zoom_level: Option<f32>,          // 1.0 = normal zoom, 2.0 = 2x zoom
    #[serde(default)]  
    pub timeline_visible_range_start: Option<f32>, // Visible time range start (seconds)
    #[serde(default)]
    pub timeline_visible_range_end: Option<f32>,   // Visible time range end (seconds)
}
```

#### Location 2: Frontend Config Store (`frontend/src/config.rs`)
```rust
pub struct SerializableWorkspaceSection {
    // Existing fields...
    pub timeline_cursor_position: f32,
    
    // ADD MATCHING FIELDS HERE:
    pub timeline_zoom_level: f32,
    pub timeline_visible_range_start: f32,
    pub timeline_visible_range_end: f32,
}
```

#### Location 3: Config Load/Save Implementation
```rust
impl ConfigStore {
    pub fn load_from_serializable(&self, config: SerializableConfig) {
        // Existing restoration...
        self.workspace.lock_mut().timeline_cursor_position.set(config.workspace.timeline_cursor_position);
        
        // ADD ZOOM STATE RESTORATION:
        // Restored through existing timeline state globals (see Integration section)
    }
}
```

This plan leverages NovyWave's existing sophisticated architecture while adding professional zoom functionality with minimal invasive changes.