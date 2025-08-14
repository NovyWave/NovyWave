# Technical Solutions Reference

## WASM Compilation and Debugging

### Critical WASM Build Patterns
- **NEVER use `cargo build` or `cargo check`** - Only mzoon handles WASM properly
- Monitor compilation via `makers start > dev_server.log 2>&1 &`
- Auto-reload only triggers after successful compilation
- Check `tail -f dev_server.log` for WASM build status

### WASM Logging
```rust
// Correct WASM logging
zoon::println!("Debug message");

// Wrong - does nothing in browser
std::println!("Debug message");
```

### Development Server Management
- **NEVER restart dev server without permission** - backend/shared crates take minutes to compile
- Backend/shared compilation takes dozens of seconds to minutes - this is normal
- Wait for compilation to complete, don't restart repeatedly
- Use `makers kill` and `makers start` commands instead of manual process management

## Fast2D Graphics Integration

### Canvas Setup with Fast2D
```rust
use fast2d::*;

// Create canvas wrapper with shared access pattern
let canvas_wrapper = Rc::new(RefCell::new(canvas));

// Signal-based canvas updates
let canvas_clone = canvas_wrapper.clone();
Task::start(SELECTED_VARIABLES.signal_vec_cloned().for_each_sync(move |_| {
    canvas_clone.borrow_mut().clear();
    // Redraw logic
}));
```

### Theme-Aware Fast2D Colors
```rust
// Use static RGBA constants matching neutral design tokens
pub static BACKGROUND_DARK: Color = Color::from_rgba(13, 13, 13, 255);
pub static BACKGROUND_LIGHT: Color = Color::from_rgba(255, 255, 255, 255);
```

### Canvas Resize Handling
```rust
// Combine Fast2D resize events with Zoon signal system
canvas.set_resize_callback(move |_width, _height| {
    // Handle resize with signal updates
});
```

## Virtual List Optimization

### MutableVec Hybrid Stable Pool
```rust
// Optimal virtual list implementation
let element_pool: MutableVec<VirtualElementState> = MutableVec::new_with_values(...);

// Velocity-based dynamic buffering
let velocity_buffer = if current_velocity > 1000.0 { 15 } 
                     else if current_velocity > 500.0 { 10 } 
                     else { 5 };
```

### Critical Virtual List Rules
- **5-15 elements buffer size** with velocity adaptation
- Avoid over-buffering (50+ elements) - causes slower rerendering
- Use stable element pools - DOM elements never recreated, only content/position updates
- Signal simplification reduces performance overhead

### Height Calculation Patterns
```rust
// Dynamic height with proper constraints
El::new()
    .s(Height::exact_signal(item_count_signal.map(|count| (count * 40) as f32)))
    .update_raw_el(|raw_el| {
        raw_el.style("min-height", "0")  // Allow flex shrinking
    })
```

## Canvas and Rendering Solutions

### Timeline Cursor Implementation
```rust
// Interactive timeline cursor with proper coordinate mapping
let canvas_click_handler = {
    let cursor_position = TIMELINE_CURSOR_POSITION.clone();
    move |event: PointerDown| {
        // Click events use page coordinates, need canvas-relative
        let canvas_rect = canvas_element.get_bounding_client_rect();
        let relative_x = event.client_x() as f32 - canvas_rect.left();
        
        // Convert to time with proper scaling
        let time = (relative_x / canvas_width) * total_time;
        cursor_position.set_neq(time);
    }
};
```

### Theme Reactivity in Canvas
```rust
// Signal-based theme switching for canvas
let canvas_clone = canvas_wrapper.clone();
Task::start(theme().signal().for_each_sync(move |theme| {
    let bg_color = match theme {
        Theme::Light => Color::from_rgba(255, 255, 255, 255),
        Theme::Dark => Color::from_rgba(13, 13, 13, 255),
    };
    canvas_clone.borrow_mut().clear_with_color(bg_color);
}));
```

### Professional Timeline Algorithm
```rust
fn calculate_timeline_segments(timeline_width: f32, time_range: f64) -> Vec<f64> {
    let target_spacing = 80.0; // pixels
    let rough_intervals = (timeline_width / target_spacing) as usize;
    let raw_interval = time_range / rough_intervals as f64;
    
    // Round to nice numbers (1-2-5-10 scaling)
    let nice_interval = round_to_nice_number(raw_interval);
    
    // Generate segments with 10px edge margins
    (0..segments).map(|i| i as f64 * nice_interval).collect()
}
```

## Performance Optimization Patterns

### Signal Chain Optimization
```rust
// Efficient signal chaining with deduplication
TIMELINE_CURSOR_POSITION.signal()
    .dedupe()  // Prevent duplicate triggers
    .for_each_sync(|position| {
        // Update dependent systems
    });
```

### Parallel Directory Traversal
```rust
// Use jwalk for 4x faster directory scanning
use jwalk::WalkDir;

WalkDir::new(path)
    .parallelism(jwalk::Parallelism::RayonNewPool(4))
    .into_iter()
    .filter_map(|entry| entry.ok())
    .collect()
```

### Unicode Text Filtering
```rust
// Robust invisible character filtering
use unicode_width::UnicodeWidthChar;

let clean_text: String = text.chars()
    .filter(|&c| c == ' ' || UnicodeWidthChar::width(c).unwrap_or(0) > 0)
    .collect();
```

## Memory Management Solutions

### WASM-Bindgen Canvas Integration
```rust
use wasm_bindgen::JsCast;

// Proper DOM element access in WASM
let canvas_element = event.target()
    .dyn_cast::<web_sys::Element>()
    .expect("Event target is not an element");
```

### Clipboard API for WASM
```rust
// Modern clipboard with fallback
async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    
    if let Some(clipboard) = window.navigator().clipboard() {
        // Modern Clipboard API
        clipboard.write_text(text).await
    } else {
        // Fallback to execCommand
        let document = window.document().unwrap();
        // execCommand implementation...
    }
}
```

### Thread-Based Library Integration
```rust
// Bridge async UI with blocking libraries using tokio::spawn_blocking
let result = tokio::spawn_blocking(move || {
    // Thread-blocking operations (file parsing, compression, etc.)
    expensive_blocking_operation(data)
}).await?;
```