# Zooming and Panning

Master zoom and pan controls for efficient waveform exploration.

## Zooming

### Basic Zoom

| Key | Action |
|-----|--------|
| `W` | Zoom in |
| `S` | Zoom out |
| `Shift+W` | Zoom in faster (3-5x) |
| `Shift+S` | Zoom out faster (3-5x) |

### Zoom Center

The **blue vertical line** marks the zoom center:

- Zoom operations expand/contract around this point
- Follows mouse cursor when hovering over the canvas
- Default position is time 0 (left edge)
- Press `Z` to reset zoom center to 0

### Zoom Behavior

**Zooming In (`W`):**
- Timeline range contracts around zoom center
- More detail visible in narrower time window
- Tick marks adjust to smaller time units

**Zooming Out (`S`):**
- Timeline range expands around zoom center
- Less detail, larger time overview
- Tick marks adjust to larger time units

### Why Zoom Center Matters

When analyzing multiple files with different time ranges, zoom center at 0 ensures files align properly during zoom operations. This is why the default zoom center is at the timeline start.

**Example:**
```
File A: 0 - 100ns (clk signal)
File B: 0 - 250s (slow_process signal)
```

With zoom center at 0, zooming in shows aligned views of both files' early signals.

## Panning

### Basic Pan

| Key | Action |
|-----|--------|
| `A` | Pan left (earlier in time) |
| `D` | Pan right (later in time) |
| `Shift+A` | Pan left faster |
| `Shift+D` | Pan right faster |

### Pan Behavior

Panning shifts the visible time window without changing zoom level:

```
Before pan: [100ns -------- 200ns]
After pan right: [150ns -------- 250ns]
```

### Panning vs Moving Cursor

- **Panning** (`A`/`D`): Moves the viewport, cursor position in time stays the same
- **Cursor Movement** (`Q`/`E`): Moves the cursor, viewport may follow

## Full Reset

Press `R` to reset everything:
- Zoom level returns to showing full timeline
- Zoom center moves to 0
- Cursor moves to center of timeline

This is useful when you've lost your place or want to start fresh.

## Recommended Workflow

1. **Start zoomed out** - Press `R` for full view
2. **Click to position** - Click on area of interest
3. **Zoom in** - Press `W` to increase detail
4. **Pan to adjust** - Use `A`/`D` for fine positioning
5. **Use cursor for values** - Move cursor with `Q`/`E` to read values

## Zoom Level Display

The current zoom level is shown in the Name Column footer:

```
15ns/px  - Very zoomed in (high detail)
1.5μs/px - Moderately zoomed
250ms/px - Zoomed out (overview)
```

This tells you how much time each pixel represents.
