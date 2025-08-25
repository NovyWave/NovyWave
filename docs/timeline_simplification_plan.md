# Timeline System Simplification Plan

## Current Problems (Crisis Mode)

### Critical Issues
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

## Proposed Architecture: Integer Time System

### Core Concept
**Replace all floating-point time with u64 nanoseconds internally**
- No precision loss: 1 nanosecond resolution up to ~584 years
- No f32↔f64 conversions needed
- Natural bounds checking (0 ≤ time_ns ≤ u64::MAX)
- Integer arithmetic is faster and more predictable

### Time Types
```rust
pub struct TimeNs(u64);  // Nanoseconds since file start
pub struct DurationNs(u64);  // Duration in nanoseconds
pub struct ZoomLevel(u32);   // Zoom as integer percentage (100 = 1x, 1000 = 10x)
```

### State Simplification
Replace complex floating-point state:
```rust
// OLD (complex, precision-prone)
static TIMELINE_CURSOR_POSITION: Lazy<Mutable<f64>>;
static TIMELINE_VISIBLE_RANGE_START: Lazy<Mutable<f32>>;
static TIMELINE_VISIBLE_RANGE_END: Lazy<Mutable<f32>>;
static TIMELINE_ZOOM_LEVEL: Lazy<Mutable<f32>>;

// NEW (simple, precise)
static TIMELINE_CURSOR_NS: Lazy<Mutable<TimeNs>>;
static TIMELINE_VIEWPORT: Lazy<Mutable<(TimeNs, TimeNs)>>;  // (start, end)
static TIMELINE_ZOOM_PERCENT: Lazy<Mutable<u32>>;  // 100 = 1x, 1000 = 10x
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

## Implementation Phases

### Phase 1: Integer Time Foundation (Week 1)
1. Create `TimeNs`, `DurationNs`, `ZoomLevel` types
2. Replace timeline state variables
3. Update coordinate conversion functions
4. Fix panic-prone arithmetic (bounds checking)
5. **Milestone**: No more precision-related panics

### Phase 2: Unified Cache System (Week 2) 
1. Design single `TimelineCache` structure
2. Replace multiple cache systems with unified one
3. Update signal value lookup logic
4. **Milestone**: Signal values display correctly, no more N/A

### Phase 3: Viewport Loading (Week 3)
1. Modify backend requests to use viewport ranges
2. Implement smart buffering strategy
3. Add cache invalidation on viewport changes
4. **Milestone**: Fast loading for large files

### Phase 4: Smooth Rendering (Week 4)
1. Implement render-time coordinate conversion
2. Add animation state for panning/zooming
3. Sub-pixel precision rendering
4. **Milestone**: Smooth 60fps panning/zooming restored

## Technical Details

### Coordinate Conversion
```rust
// Mouse to time (safe integer arithmetic)
fn mouse_to_time_ns(mouse_x: f32, canvas_width: f32, viewport: (TimeNs, TimeNs)) -> TimeNs {
    let viewport_duration = viewport.1.0 - viewport.0.0;
    let normalized_x = (mouse_x / canvas_width).clamp(0.0, 1.0);
    let offset_ns = (viewport_duration as f64 * normalized_x as f64) as u64;
    TimeNs(viewport.0.0 + offset_ns)
}

// Time to pixel (for rendering)
fn time_to_pixel(time_ns: TimeNs, canvas_width: f32, viewport: (TimeNs, TimeNs)) -> f32 {
    let viewport_duration = viewport.1.0 - viewport.0.0;
    let time_offset = time_ns.0 - viewport.0.0;
    (time_offset as f64 / viewport_duration as f64 * canvas_width as f64) as f32
}
```

### Zoom Implementation
```rust
// Integer zoom levels with smooth rendering
fn set_zoom_level(new_zoom_percent: u32, center_time_ns: TimeNs) {
    let viewport = TIMELINE_VIEWPORT.get();
    let current_duration = viewport.1.0 - viewport.0.0;
    
    // Calculate new duration (integer arithmetic)
    let new_duration = (current_duration * 100) / new_zoom_percent as u64;
    
    // Center the viewport around cursor
    let half_duration = new_duration / 2;
    let new_viewport = (
        TimeNs(center_time_ns.0.saturating_sub(half_duration)),
        TimeNs(center_time_ns.0 + half_duration)
    );
    
    TIMELINE_VIEWPORT.set(new_viewport);
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

---

*This plan addresses the "broken like hell" floating-point timeline system while preserving smooth user experience through smart architecture.*