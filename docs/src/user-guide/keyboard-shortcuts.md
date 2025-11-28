# Keyboard Shortcuts

NovyWave is designed for keyboard-driven workflows. Master these shortcuts to navigate waveforms efficiently.

## Global Shortcuts

These shortcuts work anywhere in the application:

| Shortcut | Action |
|----------|--------|
| `Ctrl+T` | Toggle between dark and light theme |
| `Ctrl+D` | Toggle dock mode (Right ↔ Bottom) |
| `Escape` | Close dialog / Cancel operation |

## Timeline Navigation

### Zooming

| Shortcut | Action |
|----------|--------|
| `W` | Zoom in (centered on zoom point) |
| `Shift+W` | Zoom in faster |
| `S` | Zoom out |
| `Shift+S` | Zoom out faster |
| `Z` | Move zoom center to position 0 |
| `R` | Reset to default view (full timeline visible) |

### Panning

| Shortcut | Action |
|----------|--------|
| `A` | Pan left |
| `Shift+A` | Pan left faster |
| `D` | Pan right |
| `Shift+D` | Pan right faster |

### Cursor Movement

| Shortcut | Action |
|----------|--------|
| `Q` | Move cursor left continuously |
| `Shift+Q` | Jump to previous signal transition |
| `E` | Move cursor right continuously |
| `Shift+E` | Jump to next signal transition |

## Dialog Shortcuts

When a dialog is open:

| Shortcut | Action |
|----------|--------|
| `Enter` | Confirm / Load selected files |
| `Escape` | Cancel and close dialog |

## Understanding the Timeline

### Yellow Cursor Line
The **yellow vertical line** represents the timeline cursor:
- Shows current time position
- Signal values are displayed at this time
- Move with `Q`/`E` keys or click on timeline
- Jumps to transitions with `Shift+Q`/`Shift+E`

### Blue Zoom Center
The **blue vertical line** represents the zoom center:
- Default position: 0 (timeline start)
- Follows mouse cursor when hovering over canvas
- Zoom operations center on this point
- Reset with `Z` key

## Tips for Efficient Navigation

### Quick Value Inspection
1. Hold `Q` or `E` to smoothly scan through time
2. Use `Shift+Q`/`Shift+E` to snap to interesting transitions
3. Watch values update in real-time in the Value column

### Finding Specific Events
1. Zoom out with `S` for overview
2. Click to position cursor near area of interest
3. Use `Shift+Q`/`Shift+E` to find exact transitions
4. Zoom in with `W` for detail

### Multi-File Navigation
When working with multiple files:
1. Different files may have different time ranges
2. Zoom center at 0 aligns most files
3. Use `R` to see full combined timeline
