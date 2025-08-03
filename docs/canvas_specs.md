# Canvas Specification

## Overview
The waveform canvas displays selected variables and timeline using Fast2D rendering with dynamic row-based layout.

### Integration Location
- **Target**: Selected Variables panel's right column (replacing placeholder text)
- **Container**: Embedded Fast2D canvas within Zoon UI layout system
- **Sizing**: Canvas dimensions from panel container (responsive to panel resizing)

## Layout Structure

### Row Count & Height
- **Total rows**: `selected_variables_count + 1` (variables + timeline)
- **Row height**: `canvas_height / (selected_variables_count + 1)` (equal height for all rows)
- **Last row**: Always timeline
- **Other rows**: One per selected variable, in selection order

### Timeline Row (Last Row)
- **Time range**: 0 to maximum time from files referenced by selected variables
- **Units**: Use actual VCD timescale units (e.g., "s", "ns", "ps")
- **Intervals**: Dynamic segment count (prepare for zoom support)
  - Divide time range into reasonable number of segments (6-8 segments recommended)
  - Display time labels at segment boundaries
- **Styling**: Plain background, simple labels, visually distinct from variable rows
- **Font**: Standard UI font for timeline labels

### Variable Rows
Each row displays one selected variable's values over time.

#### Value Rectangles
- **Rectangle span**: From one transition time to next transition time
- **Rectangle content**: Formatted variable value (using existing formatter from Variable Value column)
  - Use selected format (Oct/Hex/Dec) but display only the value, not the type prefix
  - Example: Show "14" not "Oct 14", show "C" not "0xC"
- **Font**: FiraCode (monospaced) for consistent value alignment
- **Text overflow**: Handle with ellipsis (to be resolved later)

#### Color Scheme
- **Row backgrounds**: Alternating `neutral_2()` and `neutral_3()` design tokens (theme-aware)
- **Value rectangles**: Per variable, per consecutive rectangle alternation
  - **Primary**: `neutral_4()` and `neutral_5()` for odd/even rectangles
  - **Purpose**: Distinguish when one value starts/ends within same variable
  - **Scope**: Color alternation resets for each variable row
- **Text colors**: `neutral_12()` for high contrast readability

## Data Integration

### Time Range Calculation
**For testing phase**: simple.vcd with simple_tb.s.A and simple_tb.s.B variables
1. Access loaded simple.vcd file data
2. Find maximum timestamp from simple.vcd only
3. Use 0 as minimum time
4. Timeline spans from 0 to calculated maximum

**For production**: Identify files referenced by currently selected variables and find maximum across all referenced files

### Value Formatting
**Data Access Pattern**:
- Access selected variables data through existing signals/state system
- Use existing Variable Value column formatter (locate and reuse formatting logic)
- Apply user's selected format (Oct/Hex/Dec dropdown from UI state)
- Display formatted value without type prefix
- **Example**: Variable Value shows "Oct 14" → Canvas shows "14"
- Maintain consistency with Variable Value column formatting

**Testing Data**: simple_tb.s.A and simple_tb.s.B from simple.vcd

## Technical Implementation Notes

### Fast2D Canvas Integration
```rust
// Embed Fast2D canvas in Zoon UI component
Canvas::new()
    .width_signal(panel_width_signal())
    .height_signal(panel_height_signal()) 
    .renderer(waveform_renderer)
    .draw_on_change(selected_variables_signal())
```

### Row Height Calculation
```rust
let total_rows = selected_variables_count + 1;
let row_height = canvas_height / total_rows as f32;
let timeline_y = canvas_height - row_height;
let variable_row_y = |index: usize| -> f32 { index as f32 * row_height };
```

### Timeline Segmentation
```rust
let max_time = calculate_max_time_from_selected_variable_files();
let segment_count = 6; // Or calculate dynamically
let time_step = max_time / segment_count as f32;
```

### Font Loading & Usage
- **Timeline**: Standard UI font (Inter) - available in Fast2D font system
- **Variable values**: FiraCode monospaced font
  - **Implementation**: Load FiraCode through Fast2D font loading system
  - **Fallback**: Use monospace system font if FiraCode unavailable

### Data Access Implementation
```rust
// Access selected variables data
let selected_vars = selected_variables_signal();
let variable_data = selected_vars.map(|vars| {
    vars.iter().map(|var| {
        // Get variable's time-value pairs
        // Access from loaded file data (simple.vcd)
    }).collect()
});
```

### Canvas Lifecycle & Redraw Triggers
- **Initial render**: When canvas component mounts
- **Data changes**: When selected variables change
- **Panel resize**: When container dimensions change
- **Theme switch**: When color tokens change
- **Format change**: When Oct/Hex/Dec dropdown changes

### Error Handling Patterns
- **No selected variables**: Show empty canvas with timeline only
- **Missing data**: Show variable name with "No Data" placeholder
- **File not loaded**: Show "Load files to view waveforms" message

## Implementation TODO List

### Phase 1: Basic Canvas Setup
1. **Integrate empty Fast2D canvas** at place of "Unified Waveform Canvas" placeholder
2. **Split into rows with alternating background colors** (non-contrasting alternation for visual separation)
3. **⏸️ MANUAL TESTING CHECKPOINT** - Developer visually verifies row layout looks as expected

### Phase 2: Text Rendering Test
4. **Add hardcoded text placeholders** on each row using FiraCode font (test text rendering)
5. **⏸️ MANUAL TESTING CHECKPOINT** - Developer verifies text displays correctly in browser

### Phase 3: Value Rectangle Visualization
6. **Create hardcoded 3-5 rectangles** per row representing changing values (follow specs but hardcoded for visual testing)
7. **⏸️ MANUAL TESTING CHECKPOINT** - Developer verifies rectangle layout and alternating colors work

### Phase 4: Timeline Implementation
8. **Create hardcoded timeline** according to specs (0-250s range with proper segmentation)
9. **⏸️ MANUAL TESTING CHECKPOINT** - Developer verifies timeline displays correctly at bottom

### Phase 5: Live Data Integration
10. **Update rectangle count/sizes** according to live data from selected variables
11. **⏸️ MANUAL TESTING CHECKPOINT** - Developer verifies dynamic rectangles match actual data transitions
12. **Update rectangle labels** with formatted values according to specs (use existing formatter, strip prefixes)
13. **⏸️ MANUAL TESTING CHECKPOINT** - Developer verifies value formatting displays correctly

### Phase 6: Timeline Data Integration
14. **Load timeline data** from live info (using simple.vcd as test case)
15. **⏸️ MANUAL TESTING CHECKPOINT** - Developer verifies timeline reflects actual file time range

### Phase 7: Multi-File Support
16. **Adapt for multi-file sourced variables** (expand beyond simple.vcd + two variables testing)

### Phase 8: Theme Integration ✅ COMPLETED
17. **Adapt canvas to theme changes** (replace hardcoded RGBA colors with reactive theme-aware design tokens)
    - **IMPLEMENTATION**: Canvas now uses theme-aware color constants based on neutral design tokens
    - **COLOR MAPPING**: All hardcoded colors replaced with theme_colors module constants
    - **CURRENT LIMITATION**: Static theme colors (not reactive to theme switches yet)
    - **FUTURE ENHANCEMENT**: Make colors fully reactive to theme changes

### Phase 9: Timeline Refinements ✅ COMPLETED
18. **Fix timeline edge label visibility** - Timeline start (0s) and end (250s) labels are partially cut off at canvas edges
    - **SOLUTION**: Implemented 10px margin system - labels only show when they won't be cut off
19. **Improve timeline tick spacing** - Replace fixed segment count with pixel-based spacing for professional appearance
    - **SOLUTION**: Implemented round_to_nice_number() algorithm for professional tick spacing
    - **RESULT**: Timeline now shows round numbers (50s, 100s, 150s, 200s) instead of awkward values
    - **ADAPTIVE**: 80px target spacing automatically adjusts tick count based on canvas width
    - **PROFESSIONAL**: Uses 1-2-5-10 scaling pattern for optimal readability

### Phase 10: Dynamic Format Updates
20. **Implement reactive canvas updates on format changes** - Canvas must redraw when user changes variable format (Oct/Hex/Dec dropdown)
    - **Current limitation**: Canvas only updates on initial load, not when user changes format dropdown
    - **Required**: Add signal listener for format changes in selected variables
    - **Implementation**: Canvas should react to SELECTED_VARIABLES signal changes and re-render affected rows
    - **User experience**: When user clicks Hex→Bins dropdown, corresponding row should immediately update values

### Phase 11: File Timeline Information ✅ COMPLETED
21. **Add maximum timeline value and unit to file names in Files & Scope panel** - Display time range info next to file names
    - **IMPLEMENTATION**: Added get_file_timeline_info() helper function in views.rs
    - **DISPLAY FORMAT**: "simple.vcd (0s-250s)" and "wave_27.fst (0ns-100ns)" 
    - **UI INTEGRATION**: Enhanced convert_tracked_files_to_tree_data() to include timeline info in file labels
    - **CURRENT APPROACH**: Uses same hardcoded values as waveform canvas for consistency
    - **FUTURE ENHANCEMENT**: Extract actual timeline data from waveform file metadata
    - **USER BENEFIT**: Users can now see file time ranges at a glance before selecting variables

### Implementation Notes
- **Error Prevention**: Check compilation logs after each step to avoid blank page with hidden errors
- **Incremental Verification**: Each phase includes MANUAL TESTING CHECKPOINTS where developer visually inspects results in browser
- **Claude waits**: Claude implements, then stops and waits for developer to manually test and give feedback
- **Clear Communication**: Ask for clarification if any requirement is unclear
- **Testing Protocol**: Developer loads browser, navigates to Selected Variables panel, visually inspects canvas changes

### Potential Implementation Blockers
1. **Fast2D Canvas Integration**: May need to research how to embed Fast2D canvas in Zoon UI layout
2. **Selected Variables Data Access**: Need to locate existing signals/state for selected variables
3. **Variable Value Formatter Access**: Need to find and reuse existing formatting logic
4. **FiraCode Font Loading**: Verify font availability in Fast2D system
5. **simple.vcd Data Structure**: Understand how loaded VCD data is stored and accessed
6. **Panel Sizing Signals**: Locate panel width/height signals for responsive canvas sizing

### Pre-Implementation Research Tasks (USE SUBAGENTS)
**SAFE FOR SUBAGENTS - These are research/analysis tasks that don't modify code:**
- [ ] Find Selected Variables panel component location (use Task tool subagent)
- [ ] Identify selected variables signal/state system (use Task tool subagent)
- [ ] Locate Variable Value column formatter code (use Task tool subagent)
- [ ] Verify Fast2D canvas embedding patterns in codebase (use Task tool subagent)
- [ ] Check FiraCode font availability and loading mechanism (use Task tool subagent)
- [ ] Research Fast2D Canvas API and integration patterns (use Task tool subagent)
- [ ] Find panel width/height signals for responsive sizing (use Task tool subagent)

**CONTEXT CONSERVATION STRATEGY:**
- Use Task tool subagents extensively for all research and codebase analysis
- Main session focuses only on coordination and actual code implementation
- Subagents return condensed findings to preserve main session context
- This approach can extend session length 2-3x for complex implementations

## Session Transfer Notes

### Key Implementation Discoveries (for next session)
- **Canvas specs are complete and implementation-ready** with incremental phases
- **Test data confirmed**: simple.vcd with simple_tb.s.A and simple_tb.s.B variables always selected
- **Integration target**: Selected Variables panel right column (replacing placeholder text)
- **Color scheme**: Theme-aware using neutral design tokens (neutral_2 through neutral_12)
- **Manual testing protocol**: Developer visually inspects each phase in browser before proceeding

### Critical Success Factors
- **Start with subagent research phase** to locate all required components/signals before coding
- **Check compilation logs** after every step to prevent hidden errors
- **Use browser MCP** for visual verification of canvas rendering
- **Wait for manual testing checkpoints** - Claude implements, then stops for developer feedback

### Context Conservation Strategy
- **Research phase**: Use Task tool subagents for all codebase analysis (extends session 2-3x)
- **Implementation phase**: Main session does actual code changes after research complete
- **Testing phase**: Use browser MCP for automated verification + developer manual testing

## Future Considerations
- Zoom functionality will change visible time range and segment intervals
- Text ellipsis handling for long values in narrow rectangles
- Performance optimization for large numbers of variables/time segments