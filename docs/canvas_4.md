# Progressive Loading Implementation Plan

## Investigation Results: Major Discovery

### Key Finding
NovyWave can absolutely implement progressive loading! The current implementation forces full body parsing for ALL file formats just to extract time intervals, even though this is unnecessary.

### Root Cause Analysis
**Current Blocker**: Lines 144-146 in `backend/src/main.rs` force `read_body()` for time table access:
```rust
let (min_time, max_time) = if !body_result.time_table.is_empty() {
    let raw_min = *body_result.time_table.first().unwrap() as f64;  // Forces body parsing!
    let raw_max = *body_result.time_table.last().unwrap() as f64;
```

**The Problem**: NovyWave forces body parsing for ALL formats to extract time ranges, even though:
- ✅ **FST files already contain time intervals in headers** via `FstReader::open_and_read_time_table()`
- ✅ **All scope/signal information is available from headers** (no body needed for Files & Scopes panel)

### File Format Analysis

**FST Format**: Fully progressive-ready
- Time ranges available in header via `open_and_read_time_table()`
- Body parsing can be deferred until signal queries
- **Immediate win possible**

**VCD Format**: Requires hybrid approach
- Header has timescale but no time ranges
- Need to scan body for first/last timestamps (lighter than full parsing)
- Could implement "quick scan" for time bounds without full signal loading

**What's Available Header-Only**:
- ✅ File format detection
- ✅ Complete hierarchy (scopes and signals)
- ✅ Timescale information (for VCD)
- ✅ Time table (for FST only)
- ✅ Signal definitions and references

**What Requires Body Parsing**:
- Signal value data (for actual waveform rendering)
- Time table (for VCD only)

## Implementation Plan

### Phase 1: FST Progressive Loading (Immediate Win)
**Effort**: Low | **Impact**: High

**Goal**: Extract time ranges from FST headers during `read_header_from_file()`, skip body parsing entirely until signal queries needed.

**Result**: 3GB FST files load instantly in Files & Scopes panel.

**Implementation**:
```rust
// For FST files - header already contains time table!
match header_result.file_format {
    wellen::FileFormat::Fst => {
        // FST reader already loaded time table during header parsing
        // Extract time ranges directly from header data
        let time_ranges = extract_fst_time_ranges(&header_result);
        
        // Store minimal metadata only - defer body parsing
        let minimal_data = WaveformMetadata {
            hierarchy: header_result.hierarchy,
            file_format: header_result.file_format,
            min_time: time_ranges.0,
            max_time: time_ranges.1,
            body_continuation: Some(header_result.body), // Store for later
        };
    },
    // ... handle VCD differently
}
```

### Phase 2: VCD Quick Time Scan (Medium Effort)
**Goal**: Implement lightweight time bounds scanner for VCD files that parses only timestamps without full signal data loading.

**Result**: Much faster VCD loading with partial body parsing.

### Phase 3: Architecture Redesign (Future)
**Goal**: Split `WaveformData` into two structures:
- `WaveformMetadata` (for Files & Scopes panel) - header-only
- `WaveformData` (for signal queries) - includes body data

**Implementation**: Lazy-load signal data only when actually viewing waveforms.

## Expected Benefits

### Immediate Benefits (Phase 1)
- FST files (3GB+): Load instantly in Files & Scopes panel
- VCD files: Still require time scanning but could be optimized with partial body reading

### Memory Benefits
- Avoid loading 3GB signal data until actually needed
- Files & Scopes panel loads with ~KB memory footprint instead of GB

### Performance Impact
- **Current**: 3GB FST file = minutes of parsing + GB memory
- **Progressive**: 3GB FST file = seconds of header parsing + KB memory

## Technical Details

### Why FST Works Immediately
FST format already supports progressive loading through Wellen's `FstReader::open_and_read_time_table()` which is called during header parsing. NovyWave just needs to use the time table information that Wellen provides during header parsing instead of forcing body parsing.

### VCD Limitations
VCD headers only contain timescale information, not time ranges. Time intervals come from first/last entries in the body's time table, so VCD requires at least partial body parsing to extract time bounds.

### Current Code Structure Issues
The current `load_file` function in `backend/src/main.rs` (lines 97-213) has these dependencies:
1. **Time interval extraction**: Requires body parsing (this is what we'll fix)
2. **Scope extraction**: ✅ Available from `header_result.hierarchy` (no body needed)
3. **Signal reference map**: ✅ Built from `header_result.hierarchy` (no body needed)
4. **File format detection**: ✅ Available from `header_result.file_format`

**NOT needed for Files & Scopes panel**:
- `signal_source` - Only needed for actual signal value queries
- Full `time_table` - Only `min_time`/`max_time` needed for UI display

## Implementation Strategy

### Phase 1 Steps
1. **Research FST header time table access** - Use subagent to find exact Wellen API calls
2. **Create format-specific loading paths** - Split FST vs VCD handling
3. **Extract time ranges from FST headers** - Implement `extract_fst_time_ranges()`
4. **Update data structures** - Modify to support deferred body loading
5. **Test with large FST files** - Verify instant loading

### Context Management
- Use subagents for Wellen API research and code analysis
- Keep main session for architecture decisions and coordination
- Implement incrementally with testing at each step