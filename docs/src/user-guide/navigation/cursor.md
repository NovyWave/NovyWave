# Cursor Controls

The timeline cursor is your primary tool for inspecting signal values at specific times.

## The Yellow Cursor Line

The **yellow vertical line** crossing all signal rows is the timeline cursor:

- Shows current inspection time
- Signal values displayed in Value column
- Can be positioned anywhere on timeline

## Moving the Cursor

### Mouse Control

**Click on Timeline:**
- Click anywhere on the waveform canvas
- Cursor jumps to clicked position
- Values update immediately

### Keyboard Control

| Key | Action |
|-----|--------|
| `Q` | Move cursor left continuously |
| `E` | Move cursor right continuously |
| `Shift+Q` | Jump to previous signal transition |
| `Shift+E` | Jump to next signal transition |

### Continuous Movement

Hold `Q` or `E` for smooth cursor scanning:
- Cursor moves at steady pace
- Values update in real-time
- Release key to stop

### Transition Jumping

Use `Shift+Q`/`Shift+E` to jump directly to signal transitions:

```
Signal: [0000]--[1111]--[0000]--[1111]
             ^Shift+E jumps here
```

This is essential for finding specific events without scrolling through constant values.

## Cursor vs Zoom Center

NovyWave has two important vertical lines:

| Line | Color | Purpose |
|------|-------|---------|
| **Cursor** | Yellow | Time position for value inspection |
| **Zoom Center** | Blue | Reference point for zoom operations |

**Important:** These are independent systems:
- Cursor movement (`Q`/`E`) doesn't affect zoom center
- Zoom operations don't move cursor
- Click moves cursor, hover moves zoom center

## Value Inspection

When the cursor is positioned:

1. **Value Column** shows formatted values at cursor time
2. **Copy button** copies value to clipboard
3. **Format dropdown** changes display format

### Reading Values

The Value column shows each signal's value at the cursor time:

```
clk:    1         [Bin ▼]
data:   0xAB      [Hex ▼]
counter: 42       [UInt ▼]
```

### Special States

| Display | Meaning |
|---------|---------|
| `Z` | High-impedance (floating) |
| `X` | Unknown/undefined |
| `U` | Uninitialized |
| `N/A` | No data at this time |

## Cursor Position Display

The current cursor position is shown in the Value Column footer:

```
[Q] 125.5ns [E]
```

This displays the exact time with appropriate units.

## Tips

### Finding Signal Changes

1. Position cursor before area of interest
2. Press `Shift+E` to jump to next change
3. Repeat to scan through transitions

### Inspecting Glitches

1. Zoom in with `W` for detail
2. Use `Q`/`E` for fine positioning
3. Use `Shift+Q`/`Shift+E` to find narrow pulses

### Comparing Values

1. Position cursor at time T1, note values
2. Move cursor to time T2, compare values
3. Both times are shown in footer for reference
