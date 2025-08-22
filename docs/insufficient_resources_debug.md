# ERR_INSUFFICIENT_RESOURCES Debug Guide

## Problem Overview

The `POST http://localhost:8080/_api/up_msg_handler net::ERR_INSUFFICIENT_RESOURCES` error occurs during rapid zoom/pan operations, indicating browser resource exhaustion. This is a critical performance issue that can render the application unusable during intensive waveform navigation.

## Root Cause Analysis

### 1. PRIMARY CULPRIT: Smooth Animation Request Flooding

**Location:** `frontend/src/waveform_canvas.rs:1385-1530`

**The Problem:**
Multiple concurrent animation loops running at 60fps (16ms intervals):
- `start_smooth_zoom_in()` 
- `start_smooth_zoom_out()` 
- `start_smooth_pan_left()` 
- `start_smooth_pan_right()`
- `start_direct_cursor_animation_loop()`

**Request Amplification Pattern:**
```
User holds Q key (zoom in)
├─ Smooth zoom loop: 60 updates/second
├─ Each update calls update_zoom_with_mouse_center()
├─ Triggers TIMELINE_ZOOM_LEVEL.signal() change
├─ Triggers timeline range recalculation
├─ Calls request_transitions_for_all_variables()
└─ Generates N backend requests (N = selected variables)

Result: 60 × N requests/second during zoom operations
```

**Critical Math:**
- Smooth zoom: 60fps × 10 variables = 600 requests/second
- Browser connection limit: ~6-8 concurrent connections
- Resource exhaustion: <2 seconds during heavy usage

### 2. Signal Chain Cascade Amplification

**Location:** `frontend/src/waveform_canvas.rs:751-770`

**The Problem:**
Combined signal monitoring creates request cascades:

```rust
map_ref! {
    let start = TIMELINE_VISIBLE_RANGE_START.signal(),
    let end = TIMELINE_VISIBLE_RANGE_END.signal(), 
    let zoom = TIMELINE_ZOOM_LEVEL.signal()
    => (*start, *end, *zoom)
}
.dedupe() // Insufficient for rapid changes
.for_each(move |_| {
    request_transitions_for_all_variables(current_range); // O(N) requests
});
```

**Why `.dedupe()` Fails:**
- Only deduplicates identical values
- During smooth animation, each frame has different zoom/range values
- No protection against rapid sequences of different values

### 3. Cursor Position Signal Value Queries

**Location:** `frontend/src/main.rs:153-180`

**Additional Request Generation:**
- Cursor movement triggers value queries for ALL variables
- Comments indicate "125+ queries/sec during Q/E key holds"
- Adaptive debouncing (200-300ms) helps but doesn't prevent floods

## All Possible Contributing Factors

### A. Connection Layer Issues

#### A1. No Request Queuing/Batching
**Location:** `frontend/src/connection.rs:324-343`
- All messages sent immediately via `send_up_msg()`
- No batching of similar requests
- No prioritization system

#### A2. Missing Request Cancellation
- Old requests continue processing when newer ones arrive
- No ability to cancel obsolete timeline requests
- Stale data processing wastes resources

#### A3. Connection Pool Exhaustion
- Web: Single MoonZone connection overloaded
- Tauri: Multiple invoke calls compete for resources
- No connection-level throttling

### B. Memory Management Issues

#### B1. Unbounded Cache Growth
**Location:** `frontend/src/waveform_canvas.rs:31-42`
- `SIGNAL_TRANSITIONS_CACHE` grows indefinitely
- `ACTIVE_REQUESTS` only cleaned by timeout (5 seconds)
- `TRANSITIONS_REQUESTED` no automatic cleanup

#### B2. Event Handler Memory Leaks
- Animation loops may accumulate without proper cleanup
- Signal chain subscriptions growing over time
- Canvas event handlers not being released

#### B3. WebSocket Message Queue Overflow
- Messages accumulating faster than processing
- Browser internal message buffers filling up
- Network layer resource exhaustion

### C. Signal System Issues

#### C1. Signal Feedback Loops
- Timeline changes → Canvas updates → Signal changes → More requests
- Multiple reactive chains triggering simultaneously
- Circular dependencies in signal graph

#### C2. Signal Subscription Proliferation
- Each animation loop creates new signal subscriptions
- Subscriptions not cleaned up when animations stop
- Memory usage growing with each zoom/pan operation

#### C3. Signal Update Frequency Mismatch
- 60fps animation vs network request capacity
- UI updates faster than backend can respond
- Queue buildup leading to resource exhaustion

### D. Backend Integration Issues

#### D1. Message Serialization Overhead
**Location:** `shared/src/lib.rs`
- Large message structures being serialized repeatedly
- JSON serialization/deserialization bottlenecks
- Network payload size multiplying request impact

#### D2. Backend Processing Bottlenecks
- Server unable to process requests at 60fps rate
- Database query performance under load
- File I/O operations blocking request processing

#### D3. WebSocket vs HTTP Protocol Issues
- Mixed protocol usage causing resource conflicts
- HTTP/1.1 connection limits (6-8 per domain)
- WebSocket connection management issues

### E. Browser Resource Limits

#### E1. JavaScript Engine Memory Limits
- WASM memory allocation limits
- JavaScript heap exhaustion
- Garbage collection pressure

#### E2. Browser Network Limits
- Per-domain connection limits exceeded
- Request timeout accumulation
- Browser cache exhaustion

#### E3. Rendering Engine Conflicts
- Canvas updates competing with network operations
- DOM manipulation during heavy network activity
- Browser main thread blocking

## Diagnostic Strategies

### Phase 1: Request Pattern Analysis

#### Monitor Request Frequency
```javascript
// Add to browser console during issue reproduction
let requestCount = 0;
const originalFetch = window.fetch;
window.fetch = function(...args) {
    requestCount++;
    console.log(`Request #${requestCount}: ${args[0]}`);
    return originalFetch.apply(this, args);
};
```

#### Track Connection State
```javascript
// Monitor WebSocket connection state
setInterval(() => {
    console.log('Active connections:', performance.getEntriesByType('navigation'));
    console.log('Memory usage:', performance.memory);
}, 1000);
```

### Phase 2: Signal Chain Monitoring

#### Add Signal Debugging
```rust
// In waveform_canvas.rs
TIMELINE_ZOOM_LEVEL.signal().for_each_sync(|zoom| {
    zoon::println!("ZOOM SIGNAL: {}", zoom);
});

TIMELINE_VISIBLE_RANGE_START.signal().for_each_sync(|start| {
    zoon::println!("RANGE START SIGNAL: {}", start);
});
```

#### Track Request Generation
```rust
// In request_transitions_for_all_variables()
zoon::println!("REQUEST BATCH: {} variables, range: {}-{}", 
    variables.len(), start, end);
```

### Phase 3: Memory Monitoring

#### Cache Size Tracking
```rust
// Monitor cache growth
setInterval(move || {
    let cache_size = SIGNAL_TRANSITIONS_CACHE.lock_ref().len();
    let active_requests = ACTIVE_REQUESTS.lock_ref().len();
    zoon::println!("Cache: {}, Active: {}", cache_size, active_requests);
}, 1000);
```

## Solution Implementation Strategy

### Immediate Fixes (Emergency Deployment)

#### Fix 1: Animation Request Throttling
```rust
// Add animation-specific throttling
static ANIMATION_REQUEST_THROTTLE: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(0.0));

fn request_transitions_during_animation() {
    let now = Date::now();
    if now - ANIMATION_REQUEST_THROTTLE.get() < 100.0 { // 100ms throttle
        return;
    }
    ANIMATION_REQUEST_THROTTLE.set(now);
    request_transitions_for_all_variables();
}
```

#### Fix 2: Reduce Animation Frequency
```rust
// Change from 60fps to 30fps during smooth operations
Timer::sleep(33).await; // 30fps instead of 16ms (60fps)
```

#### Fix 3: Request Cancellation
```rust
// Cancel obsolete requests
static LATEST_REQUEST_ID: AtomicU64 = AtomicU64::new(0);

fn cancel_obsolete_requests() {
    let current_id = LATEST_REQUEST_ID.fetch_add(1, Ordering::SeqCst);
    // Cancel all requests with ID < current_id
}
```

### Medium-term Optimizations

#### Optimization 1: Request Batching
```rust
// Batch timeline requests
#[derive(Debug, Clone)]
pub struct BatchTimelineRequest {
    pub variables: Vec<String>,
    pub time_range: (f64, f64),
    pub request_id: u64,
}
```

#### Optimization 2: Smart Caching
```rust
// Predictive cache pre-loading
fn prefetch_adjacent_ranges(current_range: (f64, f64)) {
    let range_size = current_range.1 - current_range.0;
    let prefetch_ranges = vec![
        (current_range.0 - range_size, current_range.0), // Previous range
        (current_range.1, current_range.1 + range_size), // Next range
    ];
    // Pre-load these ranges at low priority
}
```

#### Optimization 3: Connection Pool Management
```rust
// Request priority queuing
#[derive(Debug, Clone)]
pub enum RequestPriority {
    Critical,   // User-initiated actions
    High,       // UI updates
    Medium,     // Background loading
    Low,        // Prefetching
}
```

### Long-term Architectural Changes

#### Architecture 1: Server-side Request Coalescing
```rust
// Backend batches multiple timeline requests
#[derive(Debug, Clone)]
pub struct CoalescedTimelineRequest {
    pub variables: Vec<String>,
    pub time_ranges: Vec<(f64, f64)>,
    pub coalesce_window_ms: u64,
}
```

#### Architecture 2: WebSocket Connection Optimization
```rust
// Single persistent WebSocket with message multiplexing
pub struct ConnectionManager {
    websocket: Option<WebSocket>,
    message_queue: VecDeque<PriorityMessage>,
    connection_state: ConnectionState,
}
```

#### Architecture 3: Progressive Data Loading
```rust
// Load data progressively instead of all-at-once
pub struct ProgressiveTimelineLoader {
    pub target_range: (f64, f64),
    pub loaded_ranges: Vec<(f64, f64)>,
    pub loading_strategy: LoadingStrategy,
}
```

## ARCHITECTURAL SIMPLIFICATION PLAN

### Core Insight: The Real Problem

The root issue isn't just request flooding - it's **architectural complexity** creating amplification at every level:
1. **N requests for N variables** when backend already supports batching
2. **5 separate animation loops** running concurrently at 60fps
3. **Complex deduplication logic** that doesn't actually prevent the problem
4. **25+ signal handlers** for config updates creating cascading saves

### SIMPLIFICATION STRATEGY

#### Phase 1: Leverage Existing Batch Support (90% Request Reduction)

**Discovery:** `UpMsg::QuerySignalTransitions` ALREADY supports batching!
```rust
pub enum UpMsg {
    QuerySignalTransitions {
        file_path: String,
        signal_queries: Vec<SignalTransitionQuery>, // ← Already supports multiple!
        time_range: (f64, f64),
    }
}
```

**Current Implementation (WASTEFUL):**
```rust
// frontend/src/waveform_canvas.rs
fn request_transitions_for_all_variables() {
    for variable in SELECTED_VARIABLES.lock_ref().iter() {
        // Creates SEPARATE request for EACH variable
        request_signal_transitions_from_backend(
            file_path, 
            vec![SignalTransitionQuery { /* single query */ }], // ← Only one!
            time_range
        );
    }
}
// Result: 10 variables = 10 network requests
```

**Simplified Implementation:**
```rust
fn request_transitions_for_all_variables() {
    // Group all variables by file
    let queries_by_file = group_variables_by_file(&SELECTED_VARIABLES.lock_ref());
    
    for (file_path, queries) in queries_by_file {
        // Send ONE request with ALL queries for this file
        send_up_msg(UpMsg::QuerySignalTransitions {
            file_path,
            signal_queries: queries, // ← All variables in single request!
            time_range,
        });
    }
}
// Result: 10 variables = 1 network request (90% reduction!)
```

**Benefits:**
- Immediate 90% reduction in network requests
- Backend can process in parallel (already implemented)
- Removes need for complex deduplication logic
- Simpler code, better performance

#### Phase 2: Unify Animation Systems (80% CPU Reduction)

**Current State: 5 Concurrent Animation Loops**
1. `start_direct_cursor_animation_loop()` - Cursor animation
2. `start_smooth_zoom_in/out()` - Zoom animations
3. `start_smooth_pan_left/right()` - Pan animations  
4. Canvas update pending loop
5. Error UI progress animations

**Each loop:**
- Creates new `Timer::sleep(16)` tasks
- Maintains separate state
- Triggers signal updates independently
- No coordination between animations

**Simplified: Single Master Animation Loop**
```rust
// Single animation state
struct AnimationState {
    cursor_animating: bool,
    zoom_direction: Option<ZoomDirection>,
    pan_direction: Option<PanDirection>,
    zoom_speed: f64,
    pan_speed: f64,
    last_frame: f64,
}

static ANIMATION_STATE: Lazy<Mutable<AnimationState>> = Lazy::new(Default::default);

// One loop to rule them all
fn start_master_animation_loop() {
    Task::start(async move {
        loop {
            let now = Date::now();
            let state = ANIMATION_STATE.lock_ref();
            
            // Calculate frame delta for smooth animations
            let delta = now - state.last_frame;
            
            // Update all animations in single pass
            if state.cursor_animating {
                update_cursor_position(delta);
            }
            
            if let Some(direction) = state.zoom_direction {
                update_zoom(direction, state.zoom_speed * delta);
            }
            
            if let Some(direction) = state.pan_direction {
                update_pan(direction, state.pan_speed * delta);
            }
            
            // Single canvas update at end of frame
            if needs_canvas_update() {
                update_canvas();
            }
            
            Timer::sleep(16).await; // 60fps
        }
    });
}
```

**Benefits:**
- 80% reduction in Timer tasks
- Coordinated animations (no competing updates)
- Frame-perfect synchronization
- Easier to throttle during high load

#### Phase 3: Simplify Request Deduplication (Remove 200+ Lines)

**Current Complex System:**
```rust
// Multiple tracking structures
static ACTIVE_REQUESTS: Lazy<Mutable<HashMap<String, f64>>> // Timestamp tracking
static TRANSITIONS_REQUESTED: Lazy<Mutable<HashSet<String>>> // Variable tracking
static REQUEST_DEBOUNCE_MS: f64 = 50.0;

fn should_allow_request(key: &str) -> bool {
    let now = Date::now();
    let mut active = ACTIVE_REQUESTS.lock_mut();
    
    // Complex timestamp-based deduplication
    if let Some(&last_request_time) = active.get(key) {
        if now - last_request_time < REQUEST_DEBOUNCE_MS {
            return false;
        }
    }
    
    active.insert(key.to_string(), now);
    
    // Timeout cleanup task
    Task::start(async move {
        Timer::sleep(5000).await;
        ACTIVE_REQUESTS.lock_mut().remove(key);
    });
    
    true
}
```

**Simplified: Request Generation Control**
```rust
// Single tracking structure
static PENDING_REQUEST: Lazy<Mutable<Option<RequestToken>>> = Lazy::new(Default::default);

fn request_timeline_data() {
    // Cancel any pending request
    PENDING_REQUEST.set(None);
    
    // Create new request token
    let token = RequestToken::new();
    PENDING_REQUEST.set(Some(token.clone()));
    
    // Throttle animation requests
    if is_animating() {
        Timer::sleep(100).await; // 100ms throttle during animations
    }
    
    // Send batched request
    send_batched_timeline_request(token);
}
```

**Removed Complexity:**
- No HashMap lookups
- No timeout cleanup tasks
- No HashSet tracking
- No timestamp comparisons
- 200+ lines of code removed

#### Phase 4: Throttle at Animation Source (Prevention vs Mitigation)

**Current: Trying to Deduplicate After Generation**
- Requests generated at 60fps
- Deduplication tries to filter them
- Still overwhelming the system

**Simplified: Don't Generate Excessive Requests**
```rust
// Add request throttling directly in animation loops
fn update_zoom_with_mouse_center(new_zoom: f64) {
    TIMELINE_ZOOM_LEVEL.set_neq(new_zoom);
    
    // Don't request data every frame during smooth animations
    static LAST_DATA_REQUEST: Lazy<Mutable<f64>> = Lazy::new(Default::default);
    let now = Date::now();
    
    if now - LAST_DATA_REQUEST.get() > 100.0 { // Max 10 requests/second
        LAST_DATA_REQUEST.set(now);
        request_timeline_data(); // Batched request
    }
}
```

**Benefits:**
- Prevents problem at source
- Maintains smooth 60fps animations
- Data updates at reasonable 10fps
- No complex deduplication needed

#### Phase 5: Consolidate Config Saves (75% Reduction)

**Current: 25+ Individual Signal Handlers**
```rust
// Each creates separate save task
Task::start(current_theme().signal().for_each_sync(|_| save_config()));
Task::start(FILES_PANEL_WIDTH.signal().for_each_sync(|_| save_config()));
Task::start(VARIABLES_PANEL_WIDTH.signal().for_each_sync(|_| save_config()));
// ... 22 more handlers
```

**Simplified: Grouped Updates**
```rust
// Single config change signal
static CONFIG_CHANGED: Lazy<Mutable<bool>> = Lazy::new(Default::default);

// Mark config dirty on any change
fn init_config_handlers() {
    // Group related changes
    let ui_changes = merge_signals![
        current_theme().signal(),
        FILES_PANEL_WIDTH.signal(),
        VARIABLES_PANEL_WIDTH.signal(),
    ];
    
    Task::start(ui_changes.for_each_sync(|_| {
        CONFIG_CHANGED.set(true);
    }));
    
    // Single debounced save
    Task::start(CONFIG_CHANGED.signal()
        .dedupe()
        .debounce(1000)
        .for_each_sync(|changed| {
            if changed {
                save_all_config();
                CONFIG_CHANGED.set(false);
            }
        })
    );
}
```

### Implementation Priority & Risk Assessment

| Phase | Impact | Risk | Effort | Priority |
|-------|--------|------|--------|----------|
| **Phase 1: Batch Requests** | 90% request reduction | Low - Uses existing API | 2 hours | **CRITICAL - DO FIRST** |
| **Phase 2: Throttle Animations** | Prevents flooding | Low - Local change | 1 hour | **HIGH** |
| **Phase 3: Unify Animations** | 80% CPU reduction | Medium - Refactoring | 4 hours | **MEDIUM** |
| **Phase 4: Simplify Deduplication** | -200 lines, cleaner | Low - Simplification | 2 hours | **MEDIUM** |
| **Phase 5: Consolidate Config** | Cleaner architecture | Low - Backend unchanged | 3 hours | **LOW** |

### Expected Outcomes

**Before Simplification:**
- 600+ requests/second during zoom with 10 variables
- 5 animation loops consuming CPU
- 500+ lines of deduplication logic
- ERR_INSUFFICIENT_RESOURCES within seconds

**After Simplification:**
- 10 requests/second maximum (60x reduction)
- 1 animation loop with coordinated updates
- Simple throttling at source
- Stable performance under heavy use

### Key Design Principles

1. **Batch at the source** - Don't generate N requests when 1 will do
2. **Throttle at generation** - Don't create requests you'll filter later
3. **Unify similar systems** - 1 animation loop > 5 separate loops
4. **Remove complexity** - Simple throttling > complex deduplication
5. **Leverage existing features** - Backend already supports batching!

## DETAILED IMPLEMENTATION GUIDE

### Phase 1: Batch Request Implementation (CRITICAL - DO FIRST)

#### Step 1.1: Analyze Current Request Pattern
**File:** `frontend/src/waveform_canvas.rs`
**Function:** `request_transitions_for_all_variables()`

**Current Implementation Analysis:**
```rust
// Line ~850-900 (approximate)
fn request_transitions_for_all_variables() {
    let variables = SELECTED_VARIABLES.lock_ref();
    for variable in variables.iter() {
        // Problem: Each variable creates separate request
        request_signal_transitions_from_backend(
            variable.file_path.clone(),
            variable.scope_path.clone(), 
            variable.variable_name.clone(),
            time_range
        );
    }
}
```

#### Step 1.2: Implement Batched Request
**New Implementation:**
```rust
fn request_transitions_for_all_variables() {
    let variables = SELECTED_VARIABLES.lock_ref();
    if variables.is_empty() {
        return;
    }
    
    // Group variables by file path
    let mut queries_by_file: HashMap<String, Vec<SignalTransitionQuery>> = HashMap::new();
    
    for variable in variables.iter() {
        let query = SignalTransitionQuery {
            scope_path: variable.scope_path.clone(),
            signal_name: variable.variable_name.clone(),
        };
        
        queries_by_file
            .entry(variable.file_path.clone())
            .or_insert_with(Vec::new)
            .push(query);
    }
    
    // Get current time range
    let start = TIMELINE_VISIBLE_RANGE_START.get();
    let end = TIMELINE_VISIBLE_RANGE_END.get();
    let time_range = (start, end);
    
    // Send ONE request per file with ALL variables
    for (file_path, signal_queries) in queries_by_file {
        // Check if we should throttle during animations
        if should_throttle_request() {
            continue;
        }
        
        send_up_msg(UpMsg::QuerySignalTransitions {
            file_path,
            signal_queries, // All variables for this file in one request!
            time_range,
        });
    }
}
```

#### Step 1.3: Remove Individual Request Function
**Remove or deprecate:** `request_signal_transitions_from_backend()`
- This function is no longer needed with batching
- Mark as deprecated or remove entirely

### Phase 2: Throttle at Animation Source

#### Step 2.1: Add Animation Throttling State
**Add to top of `frontend/src/waveform_canvas.rs`:**
```rust
// Animation request throttling
static LAST_ANIMATION_REQUEST: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(0.0));
static ANIMATION_REQUEST_INTERVAL_MS: f64 = 100.0; // Max 10 requests/second during animations

fn should_throttle_request() -> bool {
    // Check if we're in an animation
    if IS_ZOOMING_IN.get() || IS_ZOOMING_OUT.get() || 
       IS_PANNING_LEFT.get() || IS_PANNING_RIGHT.get() {
        let now = Date::now();
        if now - LAST_ANIMATION_REQUEST.get() < ANIMATION_REQUEST_INTERVAL_MS {
            return true; // Skip this request
        }
        LAST_ANIMATION_REQUEST.set(now);
    }
    false
}
```

#### Step 2.2: Modify Zoom Update Function
**Function:** `update_zoom_with_mouse_center()`
```rust
fn update_zoom_with_mouse_center(new_zoom: f64) {
    // Update zoom level (keep smooth 60fps animation)
    TIMELINE_ZOOM_LEVEL.set_neq(new_zoom);
    
    // Calculate new timeline range based on mouse position
    // ... existing calculation code ...
    
    // Update timeline range
    TIMELINE_VISIBLE_RANGE_START.set_neq(new_start);
    TIMELINE_VISIBLE_RANGE_END.set_neq(new_end);
    
    // DON'T trigger request here - let the signal handler do it with throttling
    // Remove any direct calls to request_transitions_for_all_variables()
}
```

#### Step 2.3: Modify Signal Handler
**Location:** Timeline range signal handler (~line 751-770)
```rust
// Modify the existing signal handler
map_ref! {
    let start = TIMELINE_VISIBLE_RANGE_START.signal(),
    let end = TIMELINE_VISIBLE_RANGE_END.signal(), 
    let zoom = TIMELINE_ZOOM_LEVEL.signal()
    => (*start, *end, *zoom)
}
.dedupe()
.for_each(move |_| {
    // Add throttling check
    if !should_throttle_request() {
        request_transitions_for_all_variables();
    }
});
```

### Phase 3: Remove Complex Deduplication

#### Step 3.1: Identify Structures to Remove
**Remove these from `frontend/src/waveform_canvas.rs`:**
```rust
// DELETE these lines (approximate locations):
// Line ~35-40
static ACTIVE_REQUESTS: Lazy<Mutable<HashMap<String, f64>>> = ...
static TRANSITIONS_REQUESTED: Lazy<Mutable<HashSet<String>>> = ...
static REQUEST_DEBOUNCE_MS: f64 = 50.0;

// Line ~66-87 - DELETE entire function
fn should_allow_request(key: &str) -> bool { ... }

// Line ~89-95 - DELETE cleanup function  
fn cleanup_old_requests() { ... }
```

#### Step 3.2: Simplify Request Tracking
**Replace with simple pending flag:**
```rust
// Add at top with other statics
static HAS_PENDING_REQUEST: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Simple request management
fn send_timeline_request() {
    if HAS_PENDING_REQUEST.get() {
        return; // Skip if request already pending
    }
    
    HAS_PENDING_REQUEST.set(true);
    request_transitions_for_all_variables();
    
    // Clear flag after response timeout
    Task::start(async move {
        Timer::sleep(500).await;
        HAS_PENDING_REQUEST.set(false);
    });
}
```

#### Step 3.3: Remove Deduplication Calls
**Search and remove all calls to:**
- `should_allow_request()`
- `ACTIVE_REQUESTS.lock_mut()`
- `TRANSITIONS_REQUESTED.lock_mut()`
- `cleanup_old_requests()`

### Phase 4: Unify Animation Loops (Optional - Medium Priority)

#### Step 4.1: Create Unified Animation State
**Add new file or section:**
```rust
#[derive(Default, Clone)]
struct AnimationState {
    // Zoom state
    is_zooming_in: bool,
    is_zooming_out: bool,
    zoom_speed: f64,
    
    // Pan state  
    is_panning_left: bool,
    is_panning_right: bool,
    is_panning_up: bool,
    is_panning_down: bool,
    pan_speed: f64,
    
    // Cursor animation
    cursor_animating: bool,
    cursor_target: f64,
    
    // Frame timing
    last_frame_time: f64,
}

static ANIMATION: Lazy<Mutable<AnimationState>> = Lazy::new(|| Mutable::new(AnimationState::default()));
```

#### Step 4.2: Create Master Animation Loop
```rust
pub fn start_master_animation_loop() {
    Task::start(async move {
        loop {
            let now = Date::now();
            let mut state = ANIMATION.lock_mut();
            let delta = now - state.last_frame_time;
            state.last_frame_time = now;
            
            // Process zoom animations
            if state.is_zooming_in {
                let current = TIMELINE_ZOOM_LEVEL.get();
                let new_zoom = current * (1.0 + state.zoom_speed * delta / 1000.0);
                update_zoom_with_mouse_center(new_zoom);
            } else if state.is_zooming_out {
                let current = TIMELINE_ZOOM_LEVEL.get();
                let new_zoom = current / (1.0 + state.zoom_speed * delta / 1000.0);
                update_zoom_with_mouse_center(new_zoom);
            }
            
            // Process pan animations
            if state.is_panning_left || state.is_panning_right {
                let pan_delta = state.pan_speed * delta / 1000.0;
                let current_start = TIMELINE_VISIBLE_RANGE_START.get();
                let current_end = TIMELINE_VISIBLE_RANGE_END.get();
                let range = current_end - current_start;
                
                if state.is_panning_left {
                    TIMELINE_VISIBLE_RANGE_START.set_neq(current_start - range * pan_delta);
                    TIMELINE_VISIBLE_RANGE_END.set_neq(current_end - range * pan_delta);
                } else {
                    TIMELINE_VISIBLE_RANGE_START.set_neq(current_start + range * pan_delta);
                    TIMELINE_VISIBLE_RANGE_END.set_neq(current_end + range * pan_delta);
                }
            }
            
            // Process cursor animation
            if state.cursor_animating {
                // Update cursor position smoothly
            }
            
            // Single canvas update check
            if needs_canvas_redraw() {
                trigger_canvas_update();
            }
            
            Timer::sleep(16).await; // 60fps
        }
    });
}
```

#### Step 4.3: Replace Individual Animation Loops
**Remove these functions:**
- `start_smooth_zoom_in()`
- `start_smooth_zoom_out()`
- `start_smooth_pan_left()`
- `start_smooth_pan_right()`
- `start_direct_cursor_animation_loop()`

**Replace with state updates:**
```rust
// Instead of start_smooth_zoom_in()
fn begin_zoom_in() {
    ANIMATION.update(|state| {
        state.is_zooming_in = true;
        state.zoom_speed = 0.05; // 5% per frame
    });
}

fn stop_zoom_in() {
    ANIMATION.update(|state| {
        state.is_zooming_in = false;
    });
}
```

### Phase 5: Testing and Verification

#### Step 5.1: Add Debug Logging
```rust
// Add temporary debug logging
fn request_transitions_for_all_variables() {
    let request_count = SELECTED_VARIABLES.lock_ref().len();
    zoon::println!("Batched request: {} variables in 1 request", request_count);
    // ... rest of implementation
}
```

#### Step 5.2: Monitor Request Rate
```rust
// Add request rate monitoring
static REQUEST_COUNT: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));
static REQUEST_RATE_WINDOW_START: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(Date::now()));

fn track_request_rate() {
    REQUEST_COUNT.update(|c| *c += 1);
    
    let now = Date::now();
    let window_start = REQUEST_RATE_WINDOW_START.get();
    
    if now - window_start > 1000.0 { // Every second
        let rate = REQUEST_COUNT.get();
        if rate > 30 {
            zoon::println!("WARNING: High request rate: {}/sec", rate);
        }
        REQUEST_COUNT.set(0);
        REQUEST_RATE_WINDOW_START.set(now);
    }
}
```

### Critical Code Locations Reference

| Component | File | Line Range | Purpose |
|-----------|------|------------|---------|
| Request generation | `waveform_canvas.rs` | ~850-900 | `request_transitions_for_all_variables()` |
| Timeline signal handler | `waveform_canvas.rs` | ~751-770 | Triggers requests on range change |
| Smooth zoom functions | `waveform_canvas.rs` | ~1385-1450 | Animation loops to modify |
| Smooth pan functions | `waveform_canvas.rs` | ~1451-1530 | Animation loops to modify |
| Message sending | `connection.rs` | ~324-343 | `send_up_msg()` function |
| Message types | `shared/lib.rs` | Various | `UpMsg::QuerySignalTransitions` |

### Success Metrics

**Before Implementation:**
- Request rate: 600+ requests/second with 10 variables
- Error: ERR_INSUFFICIENT_RESOURCES within 2-5 seconds
- CPU usage: Multiple animation loops at 100% core usage

**After Implementation:**
- Request rate: Max 10 requests/second (60x reduction)
- Error: No ERR_INSUFFICIENT_RESOURCES during stress test
- CPU usage: Single animation loop, <20% core usage

### Rollback Plan

If issues occur after implementation:
1. **Quick revert:** Git revert the batch request changes
2. **Temporary mitigation:** Limit selected variables to 5
3. **Emergency fix:** Reduce animation frequency to 30fps
4. **User workaround:** Disable smooth animations temporarily

## Testing Protocols

### Reproduction Steps
1. Load waveform with 10+ variables
2. Select all variables in viewer
3. Hold Q key (zoom in) for 5+ seconds
4. Simultaneously use WASD for panning
5. Monitor browser dev tools Network tab
6. Watch for ERR_INSUFFICIENT_RESOURCES errors

### Performance Benchmarks
- **Baseline:** < 10 requests/second during normal usage
- **Target:** < 30 requests/second during smooth operations
- **Maximum:** < 100 requests/second during extreme usage

### Success Criteria
- No ERR_INSUFFICIENT_RESOURCES errors during 30-second stress test
- Smooth animation performance maintained
- Memory usage stabilizes below 500MB
- Network request count stays below thresholds

## Monitoring and Alerting

### Production Monitoring
```rust
// Request rate monitoring
static REQUEST_RATE_MONITOR: Lazy<Mutable<f64>> = Lazy::new(|| Mutable::new(0.0));

fn track_request_rate() {
    if REQUEST_RATE_MONITOR.get() > 50.0 { // 50 requests/second threshold
        error_display::add_error_alert("High request rate detected");
    }
}
```

### User Feedback Integration
```rust
// Automatic error reporting when resources exhausted
fn handle_resource_exhaustion() {
    error_display::add_error_alert(
        "Performance issue detected. Try zooming/panning more slowly."
    );
}
```

## Emergency Procedures

### When Issue Occurs
1. **Immediate:** Refresh browser tab to reset connections
2. **Short-term:** Reduce number of selected variables
3. **Workaround:** Use discrete zoom buttons instead of Q/E keys
4. **Recovery:** Clear browser cache and reload application

### Prevention Guidelines
- Limit selected variables to <5 during intensive navigation
- Use discrete zoom/pan instead of smooth animations
- Take breaks during extended analysis sessions
- Monitor browser memory usage during heavy operations

This comprehensive guide covers all identified and potential causes of the ERR_INSUFFICIENT_RESOURCES error, providing both immediate fixes and long-term architectural solutions.