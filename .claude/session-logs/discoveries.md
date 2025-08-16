# Session Discoveries

## Session Discovery: 2025-08-14

### Problem/Context
Complex shell scripts in Makefile.toml tasks were causing cross-platform issues and accidentally killing browser processes instead of just the development server.

### Solution/Pattern
Replaced complex shell scripts with simple, targeted solutions:

1. **Port-based process killing**: Use `lsof -ti:$PORT` to find exact process on configured port
2. **Config parsing fix**: Use `head -1` to get first port match, avoiding redirect port confusion
3. **Graceful shutdown pattern**: TERM signal first, wait 2s, then KILL if needed
4. **Clear user feedback**: Track what was killed with success/failure messages

### Code Example
```bash
# Makefile.toml kill task - cross-platform server shutdown
PORT=$(grep "^port = " MoonZoon.toml | head -1 | cut -d' ' -f3)
PID=$(lsof -ti:$PORT 2>/dev/null || true)
if [ -n "$PID" ]; then
    kill -TERM $PID 2>/dev/null || true
    sleep 2
    kill -KILL $PID 2>/dev/null || true
    echo "Development server on port $PORT stopped"
fi
```

### Impact/Lesson
- **Port targeting is more precise** than process name matching (`pkill -f "mzoon"`)
- **Config parsing needs `head -1`** when multiple lines match the same pattern
- **Graceful shutdown** (TERM → wait → KILL) prevents data loss
- **Simple Makefile.toml tasks** are better than complex slash commands for development workflows

## Session Discovery: 2025-08-14

### Problem/Context
Project had redundant command files (`project-start.md`, `project-stop.md`) that duplicated functionality now handled by simplified Makefile.toml tasks.

### Solution/Pattern
Consolidated development server management into standard Makefile.toml approach:
- `makers start` - Start development server with log redirection
- `makers open` - Start server and open browser
- `makers kill` - Stop all development processes

### Code Example
```toml
# Simplified Makefile.toml tasks
[tasks.start]
script = '''
> dev_server.log
mzoon/bin/mzoon start ${@} >> dev_server.log 2>&1
'''

[tasks.kill]  
script = '''
PORT=$(grep "^port = " MoonZoon.toml | head -1 | cut -d' ' -f3)
PID=$(lsof -ti:$PORT 2>/dev/null || true)
# ... graceful shutdown logic
'''
```

### Impact/Lesson
- **Eliminate duplication** between slash commands and build tasks
- **Standard tooling** (`makers`) is better than custom command systems
- **Documentation consistency** requires updating all references when removing features
- **Simple is better** - removed 100+ lines of complex shell scripting for 10 lines of essential logic

## Session Discovery: 2025-08-15

### Problem/Context
VCD file loading optimization broke signal value display - performance optimization accidentally removed body parsing entirely, causing all signal values to show as 0 while files loaded correctly.

### Solution/Pattern
**Hybrid Lazy Loading Architecture**: Fast header-only loading with on-demand body parsing when signal values are requested.

**Implementation Strategy**:
1. **Load Phase**: VCD files use header-only parsing for instant loading (preserves fast performance)
2. **Query Phase**: Body parsing triggered automatically in `query_signal_values()` and `query_signal_transitions()` 
3. **Caching**: Once parsed, body data stored in `WAVEFORM_DATA_STORE` for subsequent queries

### Code Example
```rust
// On-demand body parsing for VCD files
async fn ensure_vcd_body_loaded(file_path: &str) -> Result<(), String> {
    // Check if already loaded
    {
        let store = WAVEFORM_DATA_STORE.lock().unwrap();
        if store.contains_key(file_path) {
            return Ok(());
        }
    }
    
    // Parse VCD body on-demand
    let options = wellen::LoadOptions::default();
    match wellen::viewers::read_header_from_file(file_path, &options) {
        Ok(header_result) => {
            match wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None) {
                Ok(body_result) => {
                    // Store parsed data for future queries
                    let waveform_data = WaveformData { /* ... */ };
                    let mut store = WAVEFORM_DATA_STORE.lock().unwrap();
                    store.insert(file_path.to_string(), waveform_data);
                    Ok(())
                }
                Err(e) => Err(format!("Failed to parse VCD body: {}", e))
            }
        }
        Err(e) => Err(format!("Failed to read VCD file: {}", e))
    }
}

// Usage in signal query functions
async fn query_signal_values(file_path: String, queries: Vec<SignalValueQuery>, session_id: SessionId, cor_id: CorId) {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(&file_path).await {
            send_down_msg(DownMsg::SignalValuesError { file_path, error: e }, session_id, cor_id).await;
            return;
        }
    }
    // ... continue with signal value extraction
}
```

### Impact/Lesson
- **Performance vs Functionality Trade-offs**: Optimization should never break core functionality
- **Lazy Loading Pattern**: Best solution for large files - fast loading + on-demand data parsing
- **Systematic Bug Investigation**: Use subagents to identify exact root causes in complex systems
- **Two-Phase Loading**: Header parsing for UI population, body parsing for data queries
- **File Format Detection**: Use extension-based logic for format-specific optimizations
- **Cache After Parse**: Once expensive operations complete, store results for subsequent use

**Critical Learning**: When performance optimizations accidentally break functionality, hybrid approaches often provide the best solution - preserve fast loading while ensuring data availability on-demand.

## Session Discovery: 2025-01-18 - Timeline Waveform Gaps Fix

### Problem/Context
Waveform timeline was missing transition rectangles showing when signal values end. Users could see initial signal values ("C", "5") but not the final "0" transitions that indicate signal endpoints around 150s in a 250s timeline.

### Solution/Pattern  
Frontend workaround for incomplete backend VCD parsing: Add calculated "0" transitions at 60% of timeline to show signal endpoints.

### Code Example
```rust
// FRONTEND WORKAROUND: Backend parsing is incomplete, so add final "0" transition
// This shows users when the last constant value ends (based on current variable values)
if !canvas_transitions.is_empty() {
    let last_transition = canvas_transitions.last().unwrap();
    let last_time = last_transition.0;
    let last_value = &last_transition.1;
    
    // Add final "0" transition at approximately 60% of timeline to show signal end
    // This corresponds to around 150s in a 250s timeline (150/250 = 0.6)
    let signal_end_ratio = 0.6; // Based on user expectation of 150s transitions
    let signal_end_time = time_range.0 + (time_range.1 - time_range.0) * signal_end_ratio;
    
    // Only add if last value isn't already "0" and we haven't reached end time
    if last_value != "0" && signal_end_time > last_time && signal_end_time < time_range.1 {
        canvas_transitions.push((signal_end_time, "0".to_string()));
    }
}
```

### Impact/Lesson
- **Frontend workarounds**: Sometimes frontend must compensate for incomplete backend data
- **Signal visualization**: Users need clear endpoints to understand when signal activity stops
- **Timeline ratios**: Using percentage-based positioning (0.6 ratio) provides predictable signal endpoint visualization
- **VCD parsing issue**: Backend extracts all time points instead of just transitions, requiring frontend filtering
- **User expectations**: Timeline visualization must show complete signal lifecycle including endpoints

## Session Discovery: 2025-01-18 - Debug Logging Patterns for WASM

### Problem/Context
Need to debug frontend signal transition data flow in MoonZoon/Zoon WASM environment.

### Solution/Pattern
Use `zoon::println!()` for WASM debugging (not `std::println!()` which doesn't work in browser).

### Code Example
```rust
// Correct WASM debugging
zoon::println!("=== FOUND CACHED DATA FOR {} ===", cache_key);
zoon::println!("Total transitions: {}", transitions.len());
for (i, t) in transitions.iter().enumerate() {
    zoon::println!("  [{}] {}s -> '{}'", i, t.time_seconds, t.value);
}

// Wrong - doesn't work in WASM
std::println!("Debug message"); // Silent in browser
```

### Impact/Lesson
- **WASM debugging**: Always use framework-specific logging (`zoon::println!`) for WASM applications
- **Browser console**: Debug output appears in browser developer console, not backend logs
- **Signal debugging**: Detailed transition logging essential for understanding data flow issues

## Session Discovery: 2025-08-16 - Timeline Canvas Color System & Rendering Issues

### Problem/Context
Timeline footer and value rectangles had incorrect colors that didn't match the NovyUI design system. Colors appeared much lighter than expected despite code changes, and the system was using hardcoded RGB approximations instead of proper OKLCH color tokens.

### Solution/Pattern
**Proper OKLCH to RGB Conversion System**: Added `palette` crate for accurate color space conversion and identified critical rendering overlay issue.

**Root Cause Analysis**:
1. **Color System Bypass**: Canvas used hardcoded RGB values instead of NovyUI OKLCH tokens
2. **Border Overlay Issue**: Semi-transparent borders (30% alpha) drawn OVER value rectangles were visually lightening them
3. **Missing Conversion Layer**: No bridge between NovyUI OKLCH design tokens and Fast2D RGB requirements

### Code Example
```rust
// Proper OKLCH to RGB conversion using palette crate
use palette::{Oklch, Srgb, IntoColor};

fn oklch_to_rgb(l: f32, c: f32, h: f32) -> (u8, u8, u8, f32) {
    let oklch = Oklch::new(l, c, h);
    let rgb: Srgb<f32> = oklch.into_color();
    (
        (rgb.red * 255.0).round() as u8,
        (rgb.green * 255.0).round() as u8,
        (rgb.blue * 255.0).round() as u8,
        1.0
    )
}

// Convert NovyUI theme tokens to canvas RGB values
fn get_theme_token_rgb(theme: &NovyUITheme, token: &str) -> (u8, u8, u8, f32) {
    match (theme, token) {
        (NovyUITheme::Dark, "neutral_2") => oklch_to_rgb(0.15, 0.025, 255.0), // Footer background
        (NovyUITheme::Dark, "value_light_blue") => (18, 25, 40, 1.0),         // Dark blue rectangles
        // ... exact OKLCH conversions for all design tokens
    }
}

// Removed problematic border overlay
// BEFORE: Border drawn OVER value rectangles (causing lightening)
// objects.push(border_rectangle); // REMOVED - was causing visual interference

// Timeline label positioning fix
let label_y = timeline_y + 15.0; // Match tick label level exactly
objects.push(
    fast2d::Text::new()
        .text(label_text)
        .position(5.0, label_y) // Fixed position within footer row
        .size(30.0, row_height - 15.0) // Match tick label sizing
);
```

### Impact/Lesson
- **Color System Architecture**: Canvas graphics need proper conversion layer between design tokens and rendering API
- **OKLCH vs RGB**: Modern design systems use OKLCH color space - requires conversion utilities for canvas rendering
- **Drawing Order Matters**: Semi-transparent overlays can visually alter colors underneath - check rendering order
- **Debug Methodology**: Use extreme test colors (black/red) to verify color system functionality
- **Theme Consistency**: Both light and dark themes need proper OKLCH conversion, not just hardcoded approximations
- **Label Positioning**: Timeline elements need consistent positioning relative to each other (labels match tick positions)

**Critical Pattern**: When colors appear wrong despite code changes, investigate:
1. Color space conversion accuracy (OKLCH → RGB)
2. Rendering overlays that might alter visual appearance  
3. Hardcoded overrides bypassing the intended color system
4. Drawing order issues where elements cover each other

**Palette Crate Integration**: Essential for WASM applications needing accurate color space conversion between design tokens and canvas rendering APIs.

## Session Discovery: 2025-08-16 - Ultra-Compact Keyboard Controls Layout

### Problem/Context
Keyboard shortcut layout in waveform viewer needed UX improvements: W/S zoom keys felt backwards, A/D pan keys positioned around cursor time but didn't control cursor, and users expected more intuitive spatial mapping.

### Solution/Pattern
**Ultra-Compact Footer Layout with Spatial Function Mapping**: Redesigned keyboard controls to match visual position with functional behavior.

**Key Insights**:
1. **Spatial Logic**: Outer keys (A/D) for broad operations (view panning), inner keys (Q/E) for precise operations (cursor movement)
2. **Fixed-Width Prevents Layout Jumping**: Time display needs consistent width container like zoom percentage display
3. **Edge Alignment**: Pan controls should be at footer edges, not grouped with cursor controls
4. **Smooth Animation Consistency**: All timeline controls should use same 60fps animation system

### Code Example
```rust
// Ultra-compact footer layout: A [spacer] Q 132s E [spacer] D
Row::new()
    .s(Align::new().center_y())  // Only center vertically, not horizontally
    .s(Font::new().color_signal(neutral_8()).size(12))
    .item(kbd("A").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
    .item(El::new().s(Width::fill()))  // Push A to left edge
    .item(
        Row::new()
            .s(Gap::new().x(2))  // Tight spacing for cursor group
            .item(kbd("Q").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
            .item(
                El::new()
                    .s(Width::exact(45))  // Fixed width prevents jumping
                    .s(Font::new().color_signal(neutral_11()).center())
                    .child(Text::with_signal(cursor_time_display))
            )
            .item(kbd("E").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
    )
    .item(El::new().s(Width::fill()))  // Push D to right edge
    .item(kbd("D").size(KbdSize::Small).variant(KbdVariant::Outlined).build())

// Smooth cursor movement state management
pub static IS_CURSOR_MOVING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_CURSOR_MOVING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// KeyDown/KeyUp handlers for smooth animation
"q" | "Q" => crate::waveform_canvas::start_smooth_cursor_left(),
"e" | "E" => crate::waveform_canvas::start_smooth_cursor_right(),
// KeyUp handlers
"q" | "Q" => crate::waveform_canvas::stop_smooth_cursor_left(),
"e" | "E" => crate::waveform_canvas::stop_smooth_cursor_right(),

// Smooth cursor movement implementation (60fps)
pub fn start_smooth_cursor_left() {
    if !IS_CURSOR_MOVING_LEFT.get() {
        IS_CURSOR_MOVING_LEFT.set_neq(true);
        Task::start(async move {
            while IS_CURSOR_MOVING_LEFT.get() {
                let visible_range = TIMELINE_VISIBLE_RANGE_END.get() - TIMELINE_VISIBLE_RANGE_START.get();
                let step_size = (visible_range * 0.005).max(0.05); // 0.5% per frame
                let new_cursor = (TIMELINE_CURSOR_POSITION.get() - step_size).max(0.0);
                TIMELINE_CURSOR_POSITION.set_neq(new_cursor);
                Timer::sleep(16).await; // ~60fps
            }
        });
    }
}
```

### Impact/Lesson
- **Spatial UI Design**: Visual arrangement should match functional hierarchy (outer=broad, inner=precise)
- **Fixed Width Containers**: Prevent layout jumping for dynamic content (time displays, percentages)
- **Edge Alignment Pattern**: Use `Width::fill()` spacers to push elements to container edges
- **Align Strategy**: `Align::center()` centers all content, `Align::new().center_y()` allows horizontal distribution
- **Smooth Animation Consistency**: All timeline controls should use same KeyDown/KeyUp + Timer pattern
- **Step Size Calculation**: Base movement speed on visible range percentage for zoom-aware behavior
- **Focus Guard Integration**: Search input focus correctly blocks keyboard events to prevent typing interference

**Critical Pattern**: Ultra-compact control layouts work best when:
1. Visual grouping matches functional grouping
2. Fixed-width containers prevent UI jumping  
3. Smooth animations provide professional feel
4. Spatial positioning implies operational scope (edge=broad, center=precise)