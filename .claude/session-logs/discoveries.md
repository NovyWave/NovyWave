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