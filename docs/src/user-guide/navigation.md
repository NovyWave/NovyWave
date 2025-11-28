# Timeline Navigation

NovyWave provides powerful navigation tools for exploring waveform data across any time scale, from nanoseconds to seconds.

## The Timeline Interface

The waveform timeline consists of:

1. **Signal Rows** - One row per selected variable showing value transitions
2. **Value Blocks** - Colored blocks showing signal values over time
3. **Timeline Footer** - Time scale with tick marks
4. **Yellow Cursor** - Current time position for value inspection
5. **Blue Zoom Center** - Reference point for zoom operations

## Navigation Controls

### Overview

| Action | Keyboard | Mouse |
|--------|----------|-------|
| Zoom In | `W` | Scroll wheel up |
| Zoom Out | `S` | Scroll wheel down |
| Pan Left | `A` | - |
| Pan Right | `D` | - |
| Move Cursor | `Q`/`E` | Click on timeline |
| Reset View | `R` | - |

See [Zooming and Panning](./navigation/zoom-pan.md) and [Cursor Controls](./navigation/cursor.md) for detailed explanations.

## Understanding Time Scales

NovyWave automatically formats time values based on the current zoom level:

| Zoom Level | Display Format | Example |
|------------|---------------|---------|
| Wide | Seconds | `125s` |
| Medium | Milliseconds | `125.0ms` |
| Close | Microseconds | `125.0μs` |
| Very Close | Nanoseconds | `125ns` |

The timeline footer adapts its tick marks to maintain readability at any zoom level.

## Navigation Workflow

### Inspecting a Specific Event

1. **Overview First**: Press `R` to see the full timeline
2. **Locate Area**: Click near the region of interest
3. **Zoom In**: Press `W` repeatedly or hold for continuous zoom
4. **Fine Position**: Use `Q`/`E` to position cursor precisely
5. **Find Transition**: Use `Shift+Q`/`Shift+E` to snap to transitions

### Comparing Distant Events

1. Press `R` for full view
2. Click on first event location
3. Note the time in the footer
4. Click on second event location
5. Compare times and signal states

### Scanning for Patterns

1. Position cursor at start point
2. Hold `E` to scan forward continuously
3. Watch values update in the Value column
4. Release when pattern is found

## Tips

### Use Shift Modifiers
Hold `Shift` with zoom/pan keys for faster movement. Useful for navigating large time ranges.

### Zoom Center Strategy
The blue zoom center line determines where zoom operations focus. Let it follow your mouse cursor to zoom into areas you're looking at.

### Reset When Lost
If you've zoomed too far or lost your place, press `R` to reset to the full timeline view.
