# Timeline System Simplification Plan

## Implementation Status (December 2024)

### ✅ Successfully Implemented
- **Integer Time Types**: `TimeNs`, `DurationNs`, `ZoomLevel` in `time_types.rs`
- **State Variables**: Converted to integers (`TIMELINE_CURSOR_NS`, `TIMELINE_VIEWPORT`, `TIMELINE_ZOOM_LEVEL`)
- **Unified Cache**: New `TimelineCache` system in `unified_timeline_service.rs`
- **Backend Protocol**: `UnifiedSignalRequest`/`UnifiedSignalResponse` types
- **Coordinate Functions**: `mouse_to_time_ns()` and `time_to_pixel()` with safe integer arithmetic

### ⚠️ Partially Implemented (Issues)
- **Mixed Time Systems**: Code still converts between integer nanoseconds and floating-point seconds
- **Legacy Caches**: Old `SIGNAL_TRANSITIONS_CACHE` exists alongside new unified cache
- **Service Duplication**: Both `signal_data_service.rs` and `unified_timeline_service.rs` running in parallel
- **Backend Integration**: Still converts to floating-point before sending to backend

### ❌ Not Implemented
- **Animation System**: No smooth integer-pixel animation for panning/zooming
- **Legacy Code Removal**: Old systems still present causing confusion
- **Zoom Algorithm**: Current percentage-based approach has fundamental flaws (see below)

## Critical Zoom Algorithm Problem

### Current Flawed Approach
```rust
pub struct ZoomLevel(u32);  // 100 = 1x, 150 = 1.5x, 1000 = 10x
```
**PROBLEM**: This requires division (150/100 = 1.5) introducing floating-point back into the system!

### Original Problems (Before Implementation)
1. **Floating Point Precision Chaos**: Values like `0.000010852931764380662` everywhere
2. **Panics when cursor outside visible range**: Unsafe arithmetic and bounds checking
3. **All values showing N/A**: Broken signal lookup due to precision mismatches
4. **30+ f32↔f64 conversions**: Precision loss at every boundary
5. **Multiple overlapping cache systems**: SIGNAL_TRANSITIONS_CACHE, VIEWPORT_SIGNALS, CURSOR_VALUES
6. **Complex coordinate calculations**: Canvas pixels → f32 → f64 → time → precision loss

### Precision Loss Chain
```
Mouse Event (i32 pixels)
↓ getBoundingClientRect() → f32 canvas coordinates  
↓ Normalize (x/width) → f64 fractional coordinate
↓ Scale by time range → f64 seconds with extreme precision
↓ Compare with f32 ranges → precision mismatch failures
↓ Convert back to f32 for graphics → precision loss again
```

## Proposed Architecture Fix: Pure Integer Timeline

### New Zoom Algorithm: Nanoseconds Per Pixel (Industry Validated)

**KEY INSIGHT**: Instead of zoom levels/percentages, directly store the timeline resolution as an integer - this approach is validated by industry standards:
- **Oscilloscopes**: Use "time per division" (time per pixel group)
- **Professional DAWs**: Use "samples per pixel" (Ableton, Logic, Pro Tools)
- **Google Maps**: Powers of 2 scaling with direct unit-per-pixel mapping

```rust
pub struct TimeNs(pub u64);        // Nanoseconds since file start
pub struct DurationNs(pub u64);    // Duration in nanoseconds
pub struct NsPerPixel(pub u64);    // How many nanoseconds one pixel represents

// Examples:
// NsPerPixel(1_000)      = 1 microsecond per pixel (very zoomed in)
// NsPerPixel(1_000_000)  = 1 millisecond per pixel (medium zoom)
// NsPerPixel(1_000_000_000) = 1 second per pixel (zoomed out)
```

### Benefits of NsPerPixel Approach (Industry Proven)
1. **Pure Integer Math**: No division needed for viewport calculations
2. **Natural Zoom Limits**: Min = 1 (1ns per pixel), Max = file_duration / canvas_width
3. **Smooth Zooming**: Can use powers of 2 (like Google Maps) or smooth scaling (like DAWs)
4. **Direct Calculations**: `viewport_duration = ns_per_pixel * canvas_width_pixels`
5. **No Precision Loss**: Everything stays in integer domain
6. **Industry Standard**: Same pattern used by oscilloscopes, DAWs, and mapping software

### Zoom Implementation (Fixed)
```rust
// State (pure integers)
static TIMELINE_NS_PER_PIXEL: Lazy<Mutable<NsPerPixel>>;
static TIMELINE_VIEWPORT: Lazy<Mutable<(TimeNs, TimeNs)>>;
static TIMELINE_CURSOR_NS: Lazy<Mutable<TimeNs>>;

// Smooth zoom with NO floating point
fn zoom_timeline(zoom_in: bool, cursor_x_pixels: u32, canvas_width_pixels: u32) {
    let current_ns_per_pixel = TIMELINE_NS_PER_PIXEL.get().0;
    
    // Calculate new resolution (smooth integer scaling)
    let new_ns_per_pixel = if zoom_in {
        // Zoom in: fewer nanoseconds per pixel (min 1)
        (current_ns_per_pixel * 9) / 10  // 90% of previous (10% zoom in)
    } else {
        // Zoom out: more nanoseconds per pixel
        (current_ns_per_pixel * 11) / 10  // 110% of previous (10% zoom out)
    };
    
    // Keep zoom centered on cursor position
    let viewport = TIMELINE_VIEWPORT.get();
    let cursor_time_ns = viewport.0.0 + (cursor_x_pixels as u64 * current_ns_per_pixel);
    
    // Calculate new viewport (pure integer math)
    let ns_before_cursor = cursor_x_pixels as u64 * new_ns_per_pixel;
    let ns_after_cursor = (canvas_width_pixels - cursor_x_pixels) as u64 * new_ns_per_pixel;
    
    let new_viewport = (
        TimeNs(cursor_time_ns.saturating_sub(ns_before_cursor)),
        TimeNs(cursor_time_ns + ns_after_cursor)
    );
    
    TIMELINE_NS_PER_PIXEL.set(NsPerPixel(new_ns_per_pixel));
    TIMELINE_VIEWPORT.set(new_viewport);
}
```

### State Simplification (Fixed)
Replace complex floating-point state:
```rust
// OLD (complex, precision-prone)
static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f64>>;
static TIMELINE_VISIBLE_RANGE_START: Lazy<Mutable<f32>>;
static TIMELINE_VISIBLE_RANGE_END: Lazy<Mutable<f32>>;
static TIMELINE_ZOOM_LEVEL: Lazy<Mutable<f32>>;

// NEW (pure integer, no floating point anywhere)
static TIMELINE_CURSOR_NS: Lazy<Mutable<TimeNs>>;
static TIMELINE_VIEWPORT: Lazy<Mutable<(TimeNs, TimeNs)>>;  // (start, end)
static TIMELINE_NS_PER_PIXEL: Lazy<Mutable<NsPerPixel>>;    // Resolution, not percentage
```

## Smooth Panning/Zooming Strategy

### YES - Smooth UI is preserved!

**Key Insight**: Keep integer time internally, use floating point only for rendering

### Rendering Pipeline
```rust
// Internal: Integer nanoseconds
let cursor_ns: u64 = TIMELINE_CURSOR_NS.get().0;
let viewport = TIMELINE_VIEWPORT.get();

// Rendering: Convert to integer pixels (no sub-pixel needed)
let cursor_pixel = ((cursor_ns - viewport.0.0) / nanoseconds_per_pixel) as i32;
let pan_offset = PAN_ANIMATION.get(); // i32 animation state

// Final render position (integer pixels are smooth enough at 60fps)
let render_x = cursor_pixel + pan_offset;
```

### Animation System
- **Panning**: Animate i32 pixel offset at render time
- **Zooming**: Smooth interpolation between integer zoom levels  
- **Integer pixels only**: Sub-pixel precision unnecessary for timeline rendering
- **60fps smooth**: Integer pixel jumps imperceptible to human eye

## Unified Cache Architecture

### Single Cache System
Replace 4 overlapping caches with one:
```rust
pub struct TimelineCache {
    pub viewport_range: (TimeNs, TimeNs),  // Currently loaded range
    pub signals: HashMap<String, Vec<(TimeNs, SignalValue)>>,  // time → value
}

static TIMELINE_CACHE: Lazy<Mutable<TimelineCache>>;
```

### Benefits
- One source of truth for signal data
- Simple cache invalidation (viewport change)
- Easy bounds checking (is time_ns in loaded range?)
- Direct cursor value lookup without interpolation

## Smart Backend Data Strategy

### Two-Tier Loading System

#### 1. Viewport Data (Timeline Rendering)
```rust
// Load visible range + buffer for timeline rendering
let viewport_start = TIMELINE_VIEWPORT.get().0;
let viewport_end = TIMELINE_VIEWPORT.get().1;
let buffer = (viewport_end.0 - viewport_start.0) / 5;  // 20% buffer

let viewport_request = BackendRequest::ViewportData {
    range: (TimeNs(viewport_start.0.saturating_sub(buffer)), TimeNs(viewport_end.0 + buffer)),
    signals: selected_signals.clone(),
    decimation: DecimationLevel::PerPixel, // One sample per pixel column
};
```

#### 2. Cursor Point Data (Variable Values)
```rust
// Always request cursor value regardless of viewport
let cursor_request = BackendRequest::CursorValue {
    time_ns: TIMELINE_CURSOR_NS.get(),
    signals: selected_signals.clone(),
    precision: CursorPrecision::Exact, // Exact value at cursor time
};
```

### Hybrid Cache Strategy
```rust
pub struct TimelineCache {
    // Dense data for visible timeline (decimated)
    pub viewport_data: HashMap<String, Vec<(TimeNs, SignalValue)>>,
    pub viewport_range: (TimeNs, TimeNs),
    
    // Sparse data for cursor values (anywhere in file)
    pub cursor_values: HashMap<String, (TimeNs, SignalValue)>, 
}
```

### Backend Request Logic
```rust
fn update_data(cursor_ns: TimeNs, viewport: (TimeNs, TimeNs)) {
    // Always request cursor value (even outside viewport)
    request_cursor_values(cursor_ns);
    
    // Request viewport data only if cursor moved viewport or zoom changed
    if !viewport_contains_sufficient_buffer(viewport) {
        request_viewport_data(viewport);
    }
}
```

### Benefits
- **Viewport**: Fast timeline rendering with decimated data
- **Cursor**: Always shows correct values even outside visible range  
- **Efficient**: No redundant requests for same data
- **Scalable**: Works with gigabyte files by loading only what's needed

## Backend Protocol Changes

### Current Protocol Issues
```rust
// Current: Single massive request for entire signal
UpMsg::QuerySignalTransitions {
    file_path: String,
    requests: Vec<SignalTransitionRequest>, // Often entire file range
}

// Problem: Gigabytes of data, slow response, memory issues
```

### New Dual Protocol
```rust
// NEW: Separate requests for different use cases
pub enum UpMsg {
    // For timeline rendering (decimated data)
    QueryViewportData {
        file_path: String,
        time_range_ns: (u64, u64),  // Integer nanoseconds
        signals: Vec<String>,
        pixels_available: u32,      // For server-side decimation
    },
    
    // For cursor values (exact point queries)
    QueryCursorValues {
        file_path: String,
        time_ns: u64,               // Exact cursor position
        signals: Vec<String>,
    },
}

pub enum DownMsg {
    // Decimated data for timeline rendering
    ViewportDataResponse {
        file_path: String,
        time_range_ns: (u64, u64),
        signals: HashMap<String, Vec<(u64, SignalValue)>>, // Decimated samples
    },
    
    // Exact values for cursor position
    CursorValuesResponse {
        file_path: String,
        time_ns: u64,
        values: HashMap<String, SignalValue>, // Exact cursor values
    },
}
```

### Backend Optimization Opportunities
1. **Server-side decimation**: Reduce network traffic by 100x
2. **Cursor interpolation**: Backend can do precise interpolation
3. **Range validation**: Server validates time bounds before processing
4. **Parallel processing**: Timeline and cursor requests can run simultaneously

### Key Insight: Fundamental Architecture Problem

**The current single-request system is fundamentally broken for large files.**

Current approach tries to do everything with one massive data request:
- Timeline rendering needs decimated viewport data
- Cursor values need exact point queries anywhere in file
- Using same cache/protocol for both creates precision chaos and N/A values

### Why Current System Fails

1. **Precision Mismatch**: Timeline uses f32 ranges, cursor needs f64 precision
2. **Data Volume**: Loading entire signals causes gigabyte transfers
3. **Mixed Concerns**: Timeline rendering mixed with cursor value lookup
4. **Cache Complexity**: Multiple overlapping caches trying to serve different needs

### New Architecture Benefits

**Clean Separation of Concerns**:
- Timeline rendering: Fast viewport-based decimation
- Cursor values: Precise point queries anywhere
- No more precision mismatches between different use cases
- Backend can optimize each request type separately

**Network Efficiency**:
- Viewport requests: Server-side decimation reduces traffic 100x
- Cursor requests: Minimal data (just selected variables at one time point)
- Parallel processing: Timeline and cursor requests independent

**Reliability**:
- Integer nanoseconds eliminate precision issues in protocol
- Cursor values always work (even outside viewport)
- Simple cache invalidation (viewport change vs cursor change)
- No more "N/A" from precision mismatches

### Migration Strategy
- Keep old protocol working during transition
- Add new dual protocol alongside
- Backend can do better interpolation server-side
- Frontend becomes much simpler
- Remove old protocol once stable

### Critical Success Factor

This addresses the "broken like hell" assessment by **fixing the fundamental architectural flaw**: trying to serve two completely different data access patterns with one system. The new dual approach makes both timeline rendering and cursor values fast and reliable.

## Implementation Phases (UPDATED)

### Phase 1: Integer Time Foundation ✅ PARTIALLY COMPLETE
1. ✅ Create `TimeNs`, `DurationNs`, `ZoomLevel` types
2. ✅ Replace timeline state variables  
3. ⚠️ Update coordinate conversion functions (still uses float internally)
4. ⚠️ Fix panic-prone arithmetic (bounds checking incomplete)
5. **Status**: Types exist but zoom algorithm is fundamentally broken

### Phase 2: Unified Cache System ✅ PARTIALLY COMPLETE
1. ✅ Design single `TimelineCache` structure
2. ❌ Replace multiple cache systems with unified one (legacy still exists)
3. ⚠️ Update signal value lookup logic (mixed systems)
4. **Status**: New cache exists but old caches not removed

### Phase 3: Viewport Loading ❌ NOT STARTED
1. ❌ Modify backend requests to use viewport ranges
2. ❌ Implement smart buffering strategy
3. ❌ Add cache invalidation on viewport changes
4. **Status**: Still loading entire signal ranges

### Phase 4: Smooth Rendering ❌ NOT STARTED
1. ❌ Implement render-time coordinate conversion
2. ❌ Add animation state for panning/zooming
3. ❌ Sub-pixel precision rendering
4. **Status**: No animation system implemented

## Technical Details (Fixed)

### Coordinate Conversion (Pure Integer)
```rust
// Mouse to time (NO floating point needed)
fn mouse_to_time_ns(mouse_x_pixels: u32, ns_per_pixel: NsPerPixel, viewport_start: TimeNs) -> TimeNs {
    let offset_ns = mouse_x_pixels as u64 * ns_per_pixel.0;
    TimeNs(viewport_start.0 + offset_ns)
}

// Time to pixel (integer division for rendering)
fn time_to_pixel(time_ns: TimeNs, ns_per_pixel: NsPerPixel, viewport_start: TimeNs) -> u32 {
    let offset_ns = time_ns.0.saturating_sub(viewport_start.0);
    (offset_ns / ns_per_pixel.0) as u32  // Integer division is exact
}

// Viewport calculation (multiplication only)
fn calculate_viewport_from_center(center_ns: TimeNs, canvas_width_pixels: u32, ns_per_pixel: NsPerPixel) -> (TimeNs, TimeNs) {
    let half_viewport_ns = (canvas_width_pixels as u64 * ns_per_pixel.0) / 2;
    (
        TimeNs(center_ns.0.saturating_sub(half_viewport_ns)),
        TimeNs(center_ns.0 + half_viewport_ns)
    )
}
```

### Zoom Implementation Options (Industry Patterns)

```rust
// Option 1: Powers of 2 (Google Maps/DAW Pattern - Most Efficient)
const ZOOM_LEVELS_POW2: &[u64] = &[
    1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, // etc
];

// Option 2: Powers of 10 (Human-Friendly Time Units)
const ZOOM_LEVELS_POW10: &[u64] = &[
    1,           // 1 ns/pixel (maximum zoom in)
    10,          // 10 ns/pixel
    100,         // 100 ns/pixel  
    1_000,       // 1 μs/pixel
    10_000,      // 10 μs/pixel
    100_000,     // 100 μs/pixel
    1_000_000,   // 1 ms/pixel
    10_000_000,  // 10 ms/pixel
    100_000_000, // 100 ms/pixel
    1_000_000_000, // 1 s/pixel
];

// Option 3: Smooth Continuous Zoom (DAW-style)
fn smooth_zoom(zoom_in: bool, smooth_factor: f64) {
    let current = TIMELINE_NS_PER_PIXEL.get().0;
    let new_ns_per_pixel = if zoom_in {
        ((current as f64) * (1.0 - smooth_factor)) as u64  // e.g., 0.9x for 10% zoom in
    } else {
        ((current as f64) * (1.0 + smooth_factor)) as u64  // e.g., 1.1x for 10% zoom out
    }.max(1); // Never go below 1 ns/pixel
    TIMELINE_NS_PER_PIXEL.set(NsPerPixel(new_ns_per_pixel));
}

// Recommended: Hybrid approach with snap-to-grid option
fn zoom_timeline(zoom_in: bool, use_snap: bool) {
    if use_snap {
        // Snap to predefined levels for predictable zoom
        let current = TIMELINE_NS_PER_PIXEL.get().0;
        let new_ns_per_pixel = if zoom_in {
            ZOOM_LEVELS_POW2.iter().rev().find(|&&ns| ns < current).copied().unwrap_or(1)
        } else {
            ZOOM_LEVELS_POW2.iter().find(|&&ns| ns > current).copied().unwrap_or(current)
        };
        TIMELINE_NS_PER_PIXEL.set(NsPerPixel(new_ns_per_pixel));
    } else {
        // Smooth zoom for fine control
        smooth_zoom(zoom_in, 0.1); // 10% steps
    }
}
```

## Migration Strategy

### Backward Compatibility
- Keep old APIs working during transition
- Gradual migration file-by-file
- Fallback to old system if needed

### Testing Strategy
- Unit tests for time conversions
- Integration tests for cache behavior
- Performance benchmarks (before/after)
- Visual regression testing for smooth animation

## Expected Benefits

### Reliability
- **No more precision-related panics**
- **No more floating-point edge cases**
- **Predictable behavior at all zoom levels**
- **Simple bounds checking**

### Performance
- **Integer arithmetic 2-3x faster than floating-point**
- **10x-100x less data loaded from backend**
- **Single cache system reduces memory usage**
- **No precision validation overhead**

### Maintainability
- **90% reduction in precision-handling code**
- **Eliminate 30+ f32↔f64 conversion sites**
- **Simpler debugging (integer values are human-readable)**
- **Clear separation: integer logic vs. float rendering**

## Success Criteria

1. ✅ **No panics when cursor outside range**
2. ✅ **All signal values display correctly (no N/A)**
3. ✅ **Smooth 60fps panning/zooming preserved**
4. ✅ **10x faster loading for large files**
5. ✅ **Readable timeline coordinates in debug logs**

## Additional Problematic Algorithms Found

### Similar Issues in Other Timeline Features

After thorough analysis, similar precision/algorithm problems exist throughout the timeline system:

#### 1. **Mouse-to-Time Coordinate Conversion** (HIGH RISK)
**Location:** `waveform_canvas.rs:945-999`
```rust
// PROBLEM: Mixed f32/f64 precision loses accuracy
let mouse_time = min_time + (mouse_x as f32 / canvas_width) as f64 * time_range;
```
**Issue:** Converting f64→f32→f64 loses precision, especially at high zoom levels
**Solution:** Use pure f64 or integer arithmetic throughout

#### 2. **Panning Algorithm** (MEDIUM-HIGH RISK)  
**Location:** `waveform_canvas.rs:1511-1590`
```rust
// PROBLEM: Percentage-based panning accumulates error
let pan_distance = visible_range * pan_multiplier; // 0.02 or 0.10
```
**Issue:** Repeated floating-point multiplication accumulates error over time
**Solution:** Use fixed pixel-distance panning with integer arithmetic

#### 3. **Cursor Movement Calculations** (HIGH RISK)
**Location:** `waveform_canvas.rs:1669-1715`
```rust
// PROBLEM: Division by potentially small values
let current_pixel = ((current_time - visible_start) / visible_range) * canvas_width;
// Emergency fallback indicates primary algorithm fails frequently
if let Some(new_time) = apply_fallback_movement(...)
```
**Issue:** Complex conversions fail, requiring fallback to time-based stepping
**Solution:** Use NsPerPixel for direct pixel-to-time mapping

#### 4. **Zoom Center Calculations** (MEDIUM RISK)
**Location:** `waveform_canvas.rs:1856-1880`
- Mouse position used directly without validation
- Potential precision issues at extreme zoom values
**Solution:** Validate and clamp zoom center, use integer coordinates

#### 5. **Timeline Range Boundaries** (MEDIUM RISK)
**Location:** `waveform_canvas.rs:1242-1356`
```rust
// PROBLEM: Hardcoded minimum range
let min_zoom_range = 1e-9; // Minimum 1 nanosecond
```
**Issue:** May not scale properly for different time units
**Solution:** Use NsPerPixel minimum (1 ns/pixel) instead of range minimum

### Systematic Problems Across Timeline

1. **Mixed Precision Arithmetic:** f32/f64 conversions throughout
2. **Percentage-Based Operations:** Panning uses 0.02/0.10 multipliers
3. **Division by Small Values:** Occurs when visible_range approaches zero
4. **Emergency Fallbacks:** Indicate algorithmic failures
5. **No Coordinate Validation:** Missing bounds checking

### Comprehensive Fix Strategy

All these issues stem from the same root cause as the zoom problem: using floating-point arithmetic for timeline calculations. The solution is to extend the NsPerPixel approach to ALL timeline operations:

```rust
// Universal coordinate system (pure integer)
struct TimelineCoordinates {
    cursor_ns: TimeNs,           // Cursor position in nanoseconds
    viewport_start_ns: TimeNs,   // Viewport start in nanoseconds  
    ns_per_pixel: NsPerPixel,    // Resolution (replaces zoom level)
    canvas_width_pixels: u32,    // Canvas width in pixels
}

// All conversions become pure integer
impl TimelineCoordinates {
    fn mouse_to_time(&self, mouse_x: u32) -> TimeNs {
        TimeNs(self.viewport_start_ns.0 + (mouse_x as u64 * self.ns_per_pixel.0))
    }
    
    fn time_to_pixel(&self, time: TimeNs) -> Option<u32> {
        let offset = time.0.checked_sub(self.viewport_start_ns.0)?;
        Some((offset / self.ns_per_pixel.0) as u32)
    }
    
    fn pan_by_pixels(&mut self, pixels: i32) {
        let delta_ns = (pixels.abs() as u64) * self.ns_per_pixel.0;
        if pixels < 0 {
            self.viewport_start_ns.0 = self.viewport_start_ns.0.saturating_sub(delta_ns);
        } else {
            self.viewport_start_ns.0 = self.viewport_start_ns.0.saturating_add(delta_ns);
        }
    }
}
```

## Implementation TODOs (Priority Order)

### 1. Fix Zoom Algorithm (CRITICAL)
- [ ] Replace `ZoomLevel(u32)` percentage with `NsPerPixel(u64)` resolution
- [ ] Update all zoom calculations to use pure integer math (no division by 100)
- [ ] Implement smooth zoom steps or continuous integer scaling
- [ ] Update `time_to_pixel()` to use `ns_per_pixel` directly
- [ ] Update `mouse_to_time_ns()` to use `ns_per_pixel` directly
- [ ] Test zoom in/out maintains cursor position accurately

### 1b. Fix All Coordinate Conversions (CRITICAL - NEWLY FOUND)
- [ ] Create unified `TimelineCoordinates` struct in waveform_canvas.rs
- [ ] Replace mixed f32/f64 mouse coordinate conversion (line 945-999)
- [ ] Fix percentage-based panning to pixel-based panning (line 1511-1590)
- [ ] Replace complex cursor movement with direct NsPerPixel mapping (line 1669-1715)
- [ ] Add coordinate validation and bounds checking throughout
- [ ] Remove all emergency fallback algorithms (they indicate broken primary logic)

### 2. Remove Legacy Systems (HIGH)
- [ ] Delete `signal_data_service.rs` (keep only unified service)
- [ ] Remove `SIGNAL_TRANSITIONS_CACHE` global
- [ ] Remove `VIEWPORT_SIGNALS` global
- [ ] Remove `CURSOR_VALUES` global
- [ ] Remove all float-based time variables (`TIMELINE_VISIBLE_RANGE_START/END`)
- [ ] Clean up unused imports and dead code

### 3. Complete Backend Integration (HIGH)
- [ ] Stop converting nanoseconds to seconds before backend calls
- [ ] Update `UpMsg`/`DownMsg` to use `u64` nanoseconds directly
- [ ] Ensure backend handles integer nanoseconds throughout
- [ ] Remove all `as f64 / 1e9` conversions
- [ ] Test with real waveform files

### 4. Fix Mixed Time Systems (MEDIUM)
- [ ] Audit all uses of `TimeNs` and ensure no conversions to float
- [ ] Replace any remaining `f32`/`f64` time variables
- [ ] Ensure all time arithmetic uses saturating operations
- [ ] Add debug assertions for time bounds

### 5. Implement Animation System (MEDIUM)
- [ ] Add integer pixel offset state for smooth panning
- [ ] Implement frame-based animation loop
- [ ] Add easing functions for zoom transitions
- [ ] Test 60fps performance with browser MCP

### 6. Optimize Cache Strategy (LOW)
- [ ] Implement viewport data decimation
- [ ] Add cursor-specific point queries
- [ ] Implement cache invalidation on viewport change
- [ ] Add cache statistics for debugging

### 7. Testing & Validation (ONGOING)
- [ ] Unit tests for all coordinate conversion functions
- [ ] Test zoom maintains precision at extreme levels
- [ ] Test with files of different time scales (ns to hours)
- [ ] Performance benchmarks before/after
- [ ] Browser MCP validation of smooth animations

### 8. Code Cleanup (FINAL)
- [ ] Remove all debug `println!` statements
- [ ] Update documentation with new architecture
- [ ] Add comprehensive code comments
- [ ] Ensure no compilation warnings remain

## Critical Path

**Week 1**: Fix zoom algorithm (#1) + Remove legacy systems (#2)
**Week 2**: Complete backend integration (#3) + Fix mixed time (#4)
**Week 3**: Animation system (#5) + Cache optimization (#6)
**Week 4**: Testing (#7) + Final cleanup (#8)

## Key Insights

### Why NsPerPixel Works Better
1. **No Division**: `viewport_duration = ns_per_pixel * canvas_width` (multiplication only)
2. **Natural Units**: Directly represents timeline resolution
3. **Integer Scaling**: Can smoothly adjust by any integer amount
4. **No Precision Loss**: Everything stays in u64 domain
5. **Direct Mapping**: One pixel = exactly N nanoseconds
6. **Industry Standard**: Same pattern used by oscilloscopes (time/div), DAWs (samples/pixel), Google Maps (units/tile)

### Scope of the Problem
The investigation revealed that the zoom algorithm issues are just the tip of the iceberg. The entire timeline coordinate system suffers from:
- Mixed f32/f64 precision throughout
- Percentage-based calculations (zoom 150%, pan 2%, etc.)
- Division by small values causing precision loss
- Emergency fallback algorithms indicating primary logic failures
- No systematic coordinate validation

The fix requires replacing ALL timeline coordinate calculations with the NsPerPixel integer approach, not just zoom.

### Migration Notes
- Start with zoom algorithm fix - it's blocking everything else
- Legacy code removal can happen in parallel
- Backend changes need coordination with backend team
- Animation can be added last without breaking functionality
- Keep old code working until new system fully tested

---

*This plan fixes the fundamental architectural flaws while maintaining pure integer arithmetic throughout the timeline system.*