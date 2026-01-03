# Special Signal States Reference

Digital waveform files contain special signal states that represent various conditions beyond simple 0/1 logic levels. NovyWave handles these states consistently across the interface.

## Supported Special States

### High-Impedance State (Z)
- **Display symbol**: `Z`
- **Usage**: Tri-state buffers, disconnected signals, floating buses
- **Visual**: Bright contrasting color block
- **Tooltip**: "High-impedance state - Signal is disconnected or floating"

### Unknown State (X)
- **Display symbol**: `X`
- **Usage**: Uninitialized signals, timing violations, conflicting drivers
- **Visual**: Bright contrasting color block
- **Tooltip**: "Unknown state - Signal value cannot be determined"

### Uninitialized State (U)
- **Display symbol**: `U`
- **Usage**: Signals before initialization, power-up sequences
- **Visual**: Bright contrasting color block
- **Tooltip**: "Uninitialized state - Signal has not been assigned a value"

### No Data Available (N/A)
- **Display symbol**: `N/A`
- **Usage**: Timeline cursor outside file's time range
- **Visual**: Low-contrasting placeholder text
- **Tooltip**: "No data available - Timeline position is outside this file's range"

## Display Consistency

### Value Column
Special states are displayed as plain text with contrasting colors:
```
Variable A: [Z] [Hex ▼] [📋]
Variable B: [X] [Bin ▼] [📋]
Variable C: [N/A] [Hex ▼] [📋]
```

### Wave Column (Canvas)
Special states are rendered as colored blocks in the timeline:
- **Z state**: Gray/yellow block at mid-level
- **X state**: Red block spanning full signal height
- **U state**: Red block (same as X)
- **N/A**: No block rendered (gap in timeline)

## Formatter Behavior

### Binary Formatter
- Z → `Z`
- X → `X`
- U → `U`
- N/A → `N/A`

### Hexadecimal Formatter
- Z → `Z`
- X → `X`
- U → `?`
- N/A → `N/A`

### Decimal/Integer Formatters
- Z → `-`
- X → `-`
- U → `-`
- N/A → `N/A`

### ASCII Formatter
- Z → `.`
- X → `.`
- U → `.`
- N/A → `N/A`

## Educational Tooltips

When user hovers over special state values in the Value Column, educational tooltips appear:

```html
<!-- Z State -->
<div class="tooltip">
  <strong>High-Impedance (Z)</strong><br>
  Signal is disconnected or floating.<br>
  Common in tri-state buses and disabled outputs.
</div>

<!-- X State -->
<div class="tooltip">
  <strong>Unknown (X)</strong><br>
  Signal value cannot be determined.<br>
  Often caused by timing violations or uninitialized logic.
</div>

<!-- U State -->
<div class="tooltip">
  <strong>Uninitialized (U)</strong><br>
  Signal has not been assigned a value.<br>
  Typically seen during power-up or before reset.
</div>

<!-- N/A State -->
<div class="tooltip">
  <strong>No Data Available</strong><br>
  Timeline cursor is outside this file's simulation range.<br>
  Try moving cursor within the file's timespan.
</div>
```

## Common Occurrence Scenarios

### During Simulation Startup
- Signals show `U` (uninitialized) before reset
- After reset, proper logic levels appear

### Tri-State Bus Operations
- Multiple drivers on bus show `Z` when disabled
- Only active driver shows actual data (0/1)

### Timing Violations
- Setup/hold violations create `X` states
- Metastability results in unknown values

### Multi-File Scenarios
- Short file ends → remaining timeline shows `N/A`
- Different file timespans create `N/A` gaps
