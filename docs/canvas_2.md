# Waveform Canvas Critical Issues Analysis

**Analysis Date:** August 5, 2025  
**Status:** Critical bugs identified - immediate fixes required  
**Session Context:** Investigation of timeline "0s 0s 0s" bug and missing signal transitions

## Problem Summary

The NovyWave waveform canvas has three critical architectural issues preventing it from functioning as a proper waveform viewer:

1. **Timeline Range Calculation Bug** - Timeline shows "0s 0s 0s" instead of proper time ranges
2. **Hardcoded Mock Data** - Frontend uses static placeholder data instead of real signal transitions
3. **Canvas Redraw Race Conditions** - Click handling causes timeline resets and visual artifacts

## Visual Evidence

### Current Broken Behavior:
- **Before Click:** Timeline shows proper ranges like "50s 100s 150s 200s" with cursor at specific time
- **After Click:** Timeline resets to "0s 0s 0s" showing only single value "f" per signal
- **Expected:** Continuous signal visualization with multiple value transitions over time (like surfer-project.org reference)

### What Should Work:
- Timeline should maintain consistent time ranges across all interactions
- Each signal should show multiple value changes over time (transitions)
- Canvas clicks should only update cursor position, not reset timeline
- Proper waveform visualization with signal progression over time periods

## Critical Discovery: Hardcoded Timeline Data Throughout System

### **MAJOR ISSUE: Files & Scope Panel Shows Fake Timeline Data**

**Location:** `/home/martinkavik/repos/NovyWave/frontend/src/views.rs:1598-1599`

**The Problem:**
The Files & Scope panel displays **completely fake timeline information**:

```rust
} else if file_name == "wave_27.fst" {
    (0, 100, "ns")  // wave_27.fst placeholder (TODO: get actual time range)
```

**What Users See vs Reality:**
- **Display Shows:** "wave_27.fst (0ns-100ns)" 
- **Reality:** This is hardcoded placeholder data with no connection to actual file contents
- **Backend Has Real Data:** The backend correctly extracts actual min_time/max_time from waveform files
- **Problem:** Frontend ignores backend timeline data and uses static placeholder values

**Impact on User Trust:**
- Users see timeline information and assume it's real file metadata
- Waveform canvas calculations are based on these fake ranges
- Signal transitions are generated within fake time ranges
- **Complete disconnect between displayed timeline and actual waveform data**

### **COMPREHENSIVE TIMELINE DATA CORRUPTION**

The fake timeline data problem extends across **four critical areas**:

1. **Files & Scope Panel Display** (`views.rs:1598-1599`)
   - Shows fake "0ns-100ns" for wave_27.fst
   - Users see incorrect file metadata

2. **Waveform Canvas Timeline** (`waveform_canvas.rs:262-283`)
   - Uses same fake data for timeline generation
   - Timeline markers show fake time intervals

3. **Signal Rectangle Generation** (`waveform_canvas.rs:208-274`)
   - Generates signal transitions within fake time ranges
   - Signal values positioned at fake time coordinates

4. **Timeline Cursor Positioning** (`waveform_canvas.rs:172-175`)
   - Click-to-time conversion uses fake timeline range
   - Cursor position shows fake time values

**The Real Data Already Exists:**
- Backend extracts actual `min_time` and `max_time` from VCD/FST files
- `WaveformFile` struct contains real timeline metadata
- `LOADED_FILES` state contains actual file timeline ranges
- **Frontend completely ignores this real data**

## Root Cause Analysis

### Issue 1: Timeline Range Calculation Logic Flaw

**Location:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs:189-211`

**The Problem:**
```rust
fn get_current_timeline_range() -> (f32, f32) {
    let selected_vars = SELECTED_VARIABLES.lock_ref();
    let loaded_files = LOADED_FILES.lock_ref();
    
    // BUG: Loops through selected_vars - when empty, never executes!
    for var in selected_vars.iter() {
        let file_path = var.unique_id.split('|').next().unwrap_or("");
        if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
            // This logic never runs when no variables selected
            if let (Some(min_time), Some(max_time)) = (loaded_file.min_time, loaded_file.max_time) {
                return (min_time as f32, max_time as f32);
            }
        }
    }
    
    // ALWAYS falls back to this when selected_vars is empty
    (0.0, 250.0) // This causes "0s 0s 0s" display
}
```

**When This Breaks:**
- During file loading (before variables are selected)
- After canvas click events (when variable list temporarily empty)
- When user clears selected variables
- During component initialization

**Timeline Display Logic:**
The timeline generation in `generate_timeline()` takes the range `(0.0, 250.0)` and creates intervals. With this range, it calculates intervals that result in displaying "0s 0s 0s" because the range is too small or the interval calculation fails.

### Issue 2: Hardcoded Mock Data vs Real Backend Data

**Location:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs:263-290`

**The Critical Gap:**
The frontend canvas completely bypasses the sophisticated backend signal extraction system and uses hardcoded placeholder data:

```rust
// Current hardcoded implementation
let time_value_pairs = if file_name == "simple.vcd" {
    if variable_name == "A" {
        vec![
            (0.0, "1010"),    // Hardcoded from #0: b1010 comment
            (50.0, "1100"),   // Hardcoded from #50: b1100 comment  
            (150.0, "0"),     // Hardcoded from #150: b0 comment
        ]
    } else if variable_name == "FetchL1Plugin_logic_buffer_push_valid" {
        vec![
            (0.0, "80000080"),
            (100.0, "ac7508324d7f5c178d553fa"),
        ]
    }
    // ... more hardcoded data
} else if file_name == "wave_27.fst" {
    // TODO: Get actual data from wave_27.fst file
    // This TODO was never implemented!
    vec![
        (0.0, "1111"),
        (25.0, "0101"), 
        (75.0, "1010"),
        (100.0, "0000"),
    ]
}
```

**What Actually Exists (Backend):**
The backend has a complete signal data extraction system:

1. **File Parsing:** Uses `wellen` library to parse VCD/FST files correctly
2. **Data Storage:** `WaveformData` struct contains:
   - `hierarchy`: Complete signal hierarchy from file
   - `signal_source`: Access to all signal values at any time
   - `time_table`: All time points where any signal changes value
   - `signals`: HashMap mapping signal paths to signal references
3. **Time Normalization:** Converts file-specific time units to seconds
4. **Query System:** `UpMsg::QuerySignalValues` can get signal values at specific times

**The Disconnect:**
- Backend correctly extracts **all signal transitions from files**
- Frontend canvas **ignores this completely** and uses static placeholder data
- Result: Shows fake "signal transitions" instead of real waveform data

### Issue 3: Canvas Redraw Race Conditions

**Location:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs:164-180`

**The Problem Sequence:**
1. **User clicks canvas** → `handle_canvas_click()` executes
2. **Calculate clicked time** → Converts pixel coordinates to time
3. **Update cursor signal** → `TIMELINE_CURSOR_POSITION.set(clicked_time)`
4. **Cursor signal triggers redraw** → Canvas redraws immediately
5. **Redraw calls `get_current_timeline_range()`** → If variables list is empty/inconsistent, returns `(0.0, 250.0)`
6. **Timeline resets to "0s 0s 0s"** → Visual state corrupted

**Why Variables List Becomes Empty:**
- Signal-based reactive system causes temporary inconsistencies
- Multiple signal updates happening simultaneously
- Canvas redraw happens before variable list is fully synchronized
- Race condition between cursor update and variable list state

## Backend Data Architecture (Already Correct)

### What's Already Working:

**File Parsing Pipeline:**
```rust
// backend/src/main.rs:58-201
async fn load_waveform_file() {
    let vcd_reader = wellen::vcd::read(path)?;       // Parse VCD
    let fst_reader = wellen::fst::read(path)?;       // Parse FST
    
    // Extract complete waveform data:
    let waveform_data = WaveformData {
        hierarchy,      // Complete signal hierarchy
        signal_source,  // Access to signal values
        time_table,     // All time change points
        signals,        // Signal reference map
    };
}
```

**Signal Value Query System:**
```rust
// backend/src/main.rs:672-782
async fn query_signal_values_at_time() {
    // Can query any signal at any time point
    // Properly handles VCD vs FST time units
    // Returns formatted signal values
}
```

**Time Management:**
- Proper time unit conversion (VCD seconds vs FST femtoseconds)
- Min/max time extraction from actual file data
- Complete time table with all signal change points

### What's Missing:

**Signal Transition Queries:**
The backend can query individual time points but lacks **time range queries** for signal transitions:

```rust
// NEEDED: New message types in shared/src/lib.rs
QuerySignalTransitions {
    file_path: String,
    signal_queries: Vec<SignalTransitionQuery>,
    time_range: (f64, f64), // (start_seconds, end_seconds)
}

SignalTransitionQuery {
    scope_path: String,
    variable_name: String,
}

SignalTransitionResult {
    scope_path: String,
    variable_name: String,
    transitions: Vec<SignalTransition>, // All value changes in time range
}
```

## Required Fixes (Implementation Plan)

### Fix 1: Timeline Range Calculation (IMMEDIATE - HIGH PRIORITY)

**File:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs:189-211`

**Replace current logic:**
```rust
fn get_current_timeline_range() -> (f32, f32) {
    let loaded_files = LOADED_FILES.lock_ref();
    
    // Get timeline range from ALL loaded files, not just selected variables
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    let mut has_valid_files = false;
    
    for file in loaded_files.iter() {
        if let (Some(file_min), Some(file_max)) = (file.min_time, file.max_time) {
            min_time = min_time.min(file_min as f32);
            max_time = max_time.max(file_max as f32);
            has_valid_files = true;
        }
    }
    
    if !has_valid_files || min_time == max_time {
        // Reasonable default for empty/invalid files
        (0.0, 100.0)
    } else {
        (min_time, max_time)
    }
}
```

**Why This Fixes Timeline:**
- Uses loaded file data instead of selected variable state
- Timeline range becomes consistent regardless of variable selection
- Eliminates "(0.0, 250.0)" fallback that causes "0s 0s 0s"
- Works during file loading, variable changes, and canvas interactions

### Fix 2: Replace Hardcoded Data with Real Backend Queries (HIGH PRIORITY)

**Phase 2A: Add Signal Transition API**

**File:** `/home/martinkavik/repos/NovyWave/shared/src/lib.rs`

Add to `UpMsg` enum:
```rust
QuerySignalTransitions {
    file_path: String,
    signal_queries: Vec<SignalTransitionQuery>,
    time_range: (f64, f64),
},
```

Add to `DownMsg` enum:
```rust
SignalTransitions {
    results: Vec<SignalTransitionResult>,
},
```

New structs:
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalTransitionQuery {
    pub scope_path: String,
    pub variable_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]  
pub struct SignalTransitionResult {
    pub scope_path: String,
    pub variable_name: String,
    pub transitions: Vec<SignalTransition>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalTransition {
    pub time_seconds: f64,
    pub value: String,
}
```

**Phase 2B: Implement Backend Signal Transition Extraction**

**File:** `/home/martinkavik/repos/NovyWave/backend/src/main.rs`

Add handler:
```rust
async fn query_signal_transitions(
    file_path: String,
    queries: Vec<SignalTransitionQuery>, 
    time_range: (f64, f64),
    session_id: SessionId,
    cor_id: CorId,
) {
    let mut results = Vec::new();
    
    if let Some(waveform_data) = WAVEFORM_DATA.lock().await.get(&file_path) {
        for query in queries {
            let signal_key = format!("{}.{}", query.scope_path, query.variable_name);
            
            if let Some(signal_ref) = waveform_data.signals.get(&signal_key) {
                let mut transitions = Vec::new();
                
                // Iterate through time_table within time_range
                for &time_idx in &waveform_data.time_table {
                    let time_seconds = time_idx as f64 / waveform_data.time_scale;
                    
                    if time_seconds >= time_range.0 && time_seconds <= time_range.1 {
                        // Get signal value at this time
                        if let Some(value) = waveform_data.signal_source.get_value(signal_ref, time_idx) {
                            transitions.push(SignalTransition {
                                time_seconds,
                                value: format_signal_value(&value),
                            });
                        }
                    }
                }
                
                results.push(SignalTransitionResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    transitions,
                });
            }
        }
    }
    
    send_message(DownMsg::SignalTransitions { results }, session_id, cor_id).await;
}
```

**Phase 2C: Frontend Integration**

**File:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs:263-290`

Replace hardcoded section:
```rust
// Remove all hardcoded time_value_pairs logic
// Replace with:
let time_value_pairs = get_signal_transitions_for_variable(var, current_time_range).await;

async fn get_signal_transitions_for_variable(
    variable: &SelectedVariable,
    time_range: (f32, f32),
) -> Vec<(f32, String)> {
    let (file_path, scope_path, variable_name) = variable.parse_unique_id().unwrap();
    
    let query = SignalTransitionQuery {
        scope_path: scope_path.to_string(),
        variable_name: variable_name.to_string(),
    };
    
    // Send query to backend
    let message = UpMsg::QuerySignalTransitions {
        file_path: file_path.to_string(),
        signal_queries: vec![query],
        time_range: (time_range.0 as f64, time_range.1 as f64),
    };
    
    // Wait for response and convert to canvas format
    // Return actual signal transitions from backend
}
```

### Fix 3: Canvas Redraw Race Condition (MEDIUM PRIORITY)

**File:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs:164-180`

Add state management:
```rust
static CANVAS_REDRAW_PENDING: Lazy<Mutable<bool>> = lazy::default();

// In handle_canvas_click:
fn handle_canvas_click(canvas: &HtmlCanvasElement, event: &MouseEvent) {
    if CANVAS_REDRAW_PENDING.get() {
        return; // Prevent cascading redraws
    }
    
    let rect = canvas.get_bounding_client_rect();
    let canvas_x = event.client_x() as f32 - rect.left() as f32;
    
    let current_range = get_current_timeline_range();
    let canvas_width = canvas.width() as f32;
    let clicked_time = (canvas_x / canvas_width) * (current_range.1 - current_range.0) + current_range.0;
    
    // Set redraw lock
    CANVAS_REDRAW_PENDING.set(true);
    
    // Update cursor position
    TIMELINE_CURSOR_POSITION.set(Some(clicked_time));
    
    // Debounce redraw with single frame delay
    Task::start(async move {
        TimeoutFuture::new(16).await; // ~1 frame at 60fps
        CANVAS_REDRAW_PENDING.set(false);
        
        // Trigger coordinated redraw
        redraw_canvas().await;
    });
}
```

### Fix 4: Time Unit Consistency (LOW PRIORITY)

**File:** `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs`

Ensure timeline calculation uses proper time units:
```rust
fn calculate_timeline_with_file_format(file: &LoadingFile, time_range: (f32, f32)) -> (f32, f32) {
    match file.file_path.extension() {
        Some("vcd") => time_range, // Already in seconds
        Some("fst") => {
            // FST files use femtoseconds - convert if needed
            // (Backend should already handle this conversion)
            time_range
        }
        _ => time_range,
    }
}
```

## Implementation Priority

### Phase 1: Critical Fixes (Immediate)
1. **Fix timeline range calculation** - Fixes "0s 0s 0s" immediately
2. **Fix canvas redraw race** - Prevents timeline resets on click

### Phase 2: Real Data Integration (High Priority)  
3. **Add signal transition API** - Backend infrastructure for time-based queries
4. **Replace hardcoded data** - Connect frontend canvas to real backend data

### Phase 3: Enhancement (Medium Priority)
5. **Time unit consistency** - Ensure proper handling of VCD vs FST time scales
6. **Performance optimization** - Efficient signal transition queries for large files

## Testing Strategy

### Manual Testing Steps:
1. **Load waveform file** → Timeline should show proper time range (not "0s 0s 0s")
2. **Click on canvas** → Timeline should remain stable, only cursor moves
3. **Select/deselect variables** → Timeline should not reset
4. **Switch between files** → Timeline should update to new file's time range
5. **Verify signal transitions** → Each signal should show multiple value changes over time

### Browser MCP Verification:
- Take screenshots before/after each fix
- Verify timeline displays correct time ranges
- Confirm signal transitions appear instead of single values
- Test canvas interaction doesn't cause timeline resets

### Compilation Monitoring:
- Watch `dev_server.log` for build errors
- Verify WASM compilation succeeds after each change
- Test browser auto-reload works correctly

## Session Recovery Information

**Critical Files to Monitor:**
- `/home/martinkavik/repos/NovyWave/frontend/src/waveform_canvas.rs` - Main canvas implementation
- `/home/martinkavik/repos/NovyWave/shared/src/lib.rs` - Message types and data structures  
- `/home/martinkavik/repos/NovyWave/backend/src/main.rs` - Backend signal processing

**Key Functions:**
- `get_current_timeline_range()` - Timeline calculation (needs immediate fix)
- `generate_timeline()` - Timeline rendering logic
- `draw_waveform_for_variable()` - Signal visualization (needs real data)
- `handle_canvas_click()` - Canvas interaction (needs race condition fix)

**Development Commands:**
- `makers start` - Start development server
- `tail -f dev_server.log` - Monitor compilation
- Browser MCP - Visual verification of fixes

**Current State:**
- Backend signal extraction working correctly
- Frontend using hardcoded mock data
- Timeline range calculation dependent on selected variables (broken)
- Canvas click interactions cause timeline resets
- Need to transform from cursor-based inspector to time-based waveform viewer

## CRITICAL PRIORITY: Fix Hardcoded Timeline Data Corruption

### **UPDATE (August 5, 2025): Major Discovery of Fake Timeline Data**

**Issue Status:** ✅ PARTIALLY FIXED
- Timeline range calculation bug fixed
- Canvas redraw race conditions fixed
- Signal transition API infrastructure complete
- **REMAINING CRITICAL ISSUE:** Hardcoded fake timeline data throughout system

### **The Hardcoded Timeline Problem**

**Discovery:** The Files & Scope panel shows "wave_27.fst (0ns-100ns)" but this is **completely fake data** with no connection to the actual file contents.

**Evidence:**
```rust
// frontend/src/views.rs:1598-1599
} else if file_name == "wave_27.fst" {
    (0, 100, "ns")  // wave_27.fst placeholder (TODO: get actual time range)
```

**Scope of Corruption:**
1. **Files & Scope Panel** - Users see fake timeline metadata
2. **Waveform Canvas Timeline** - Timeline markers use fake ranges  
3. **Signal Rectangle Positioning** - Signal transitions placed at fake coordinates
4. **Cursor Time Display** - Click-to-time conversion shows fake values

### **Required Fixes for Real Timeline Data Integration**

#### **Fix A: Files & Scope Panel Real Timeline Display (HIGH PRIORITY)**

**Location:** `/home/martinkavik/repos/NovyWave/frontend/src/views.rs:1595-1605`

**Current Broken Code:**
```rust
let (min_time, max_time, unit) = if file_name == "simple.vcd" {
    (0, 250, "s")  // simple.vcd: actually starts at 0s, ends at 250s (verified from file)
} else if file_name == "wave_27.fst" {
    (0, 100, "ns")  // wave_27.fst placeholder (TODO: get actual time range)
} else {
    (0, 100, "ns")
};
```

**Required Replacement:**
```rust
fn get_file_timeline_info(file_path: &str) -> String {
    let loaded_files = LOADED_FILES.lock_ref();
    
    if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
        if let (Some(min_time), Some(max_time)) = (loaded_file.min_time, loaded_file.max_time) {
            // Use actual file timeline data
            let file_name = file_path.split('/').last().unwrap_or("unknown");
            
            // Determine appropriate time unit based on time range magnitude
            let time_range = max_time - min_time;
            let (display_min, display_max, unit) = if time_range >= 1_000_000_000_000.0 {
                // Seconds range
                (min_time / 1_000_000_000_000.0, max_time / 1_000_000_000_000.0, "s")
            } else if time_range >= 1_000_000_000.0 {
                // Milliseconds range  
                (min_time / 1_000_000_000.0, max_time / 1_000_000_000.0, "ms")
            } else if time_range >= 1_000_000.0 {
                // Microseconds range
                (min_time / 1_000_000.0, max_time / 1_000_000.0, "μs")
            } else {
                // Nanoseconds range
                (min_time / 1_000.0, max_time / 1_000.0, "ns")
            };
            
            return format!("{} ({:.1}{}-{:.1}{})", file_name, display_min, unit, display_max, unit);
        }
    }
    
    // Fallback for files not yet loaded
    let file_name = file_path.split('/').last().unwrap_or("unknown");
    format!("{} (loading...)", file_name)
}
```

#### **Fix B: Canvas Timeline Uses Real Backend Data (HIGH PRIORITY)**

**Problem:** Waveform canvas timeline and signal positioning should use actual file timeline ranges instead of hardcoded values.

**Current Fixed:** The timeline range calculation now correctly uses loaded file data after our fixes.

**Verification Needed:** 
- Ensure timeline generation uses real min_time/max_time from backend
- Verify signal rectangle positioning uses actual timeline coordinates
- Confirm cursor positioning reflects real file timeline

#### **Fix C: Signal Transition Integration with Real Timeline (MEDIUM PRIORITY)**

**Current Status:** Signal transition API infrastructure is complete but still uses placeholder data generation.

**Next Step:** Replace placeholder signal transition generation with actual backend queries:

```rust
// Replace get_signal_transitions_for_variable() placeholder logic with:
async fn get_real_signal_transitions_for_variable(
    variable: &SelectedVariable,
    time_range: (f32, f32),
) -> Vec<(f32, String)> {
    // Use actual SignalTransitionQuery to backend
    // Get real signal value changes from loaded waveform data
    // Return actual time-value pairs from file
}
```

### **Implementation Priority for Timeline Data Fixes**

**Phase 1: Critical User Trust (IMMEDIATE)**
1. Fix Files & Scope panel to show real timeline ranges
2. Verify canvas timeline uses actual file data 
3. Test with wave_27.fst to confirm real vs fake data

**Phase 2: Signal Data Integration (HIGH)**
4. Replace signal transition placeholder generation with backend queries
5. Ensure signal rectangles position at actual time coordinates
6. Verify cursor time display shows real file time values

**Phase 3: Full Integration Testing (MEDIUM)**
7. Load different file types (VCD vs FST) and verify time unit handling
8. Test with files having different time ranges (seconds vs nanoseconds)
9. Confirm consistent timeline experience across all UI components

### **Verification Steps**

**To Confirm Real Timeline Data:**
1. Load wave_27.fst and check if Files & Scope panel shows real time range (not "0ns-100ns")
2. Verify waveform canvas timeline shows actual file timeline intervals
3. Click on canvas and confirm cursor position shows real file time values
4. Compare displayed timeline with actual file metadata

**Red Flags Indicating Continued Fake Data:**
- Files & Scope panel still shows "wave_27.fst (0ns-100ns)"
- Timeline always shows round numbers like 0, 50, 100 instead of actual file boundaries
- All FST files show same "ns" time unit regardless of actual file time scale

This analysis provides complete context for continuing the implementation whether in this session or future sessions.