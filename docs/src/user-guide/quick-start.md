# Quick Start

Get up and running with NovyWave in under 5 minutes.

## Step 1: Launch NovyWave

Open NovyWave from your applications menu or run from terminal:

```bash
novywave
```

## Step 2: Load a Waveform File

1. Click the **Load Files** button in the Files & Scopes panel
2. Navigate to your waveform file (`.vcd`, `.fst`, or `.ghw`)
3. Select one or more files and click **Load**

Alternatively, drag and drop waveform files directly onto the application window.

## Step 3: Select a Scope

After loading, your file appears in the Files & Scopes panel as a tree:

1. Click the **chevron** (▶) next to your file to expand it
2. Navigate through the module hierarchy
3. Click the **checkbox** next to a scope to select it

## Step 4: Choose Signals

With a scope selected, the Variables panel shows available signals:

1. Browse the list of signals
2. Click on signals to add them to the Selected Variables panel
3. Use the search box to filter signals by name

## Step 5: Navigate the Timeline

Your selected signals now appear in the waveform viewer:

- **Zoom**: Use `W` to zoom in, `S` to zoom out
- **Pan**: Use `A` to pan left, `D` to pan right
- **Cursor**: Click on the timeline to move the cursor, or use `Q`/`E`

## Step 6: Inspect Values

- The **yellow cursor line** shows the current time position
- Signal values at the cursor position appear in the Value column
- Click the dropdown to change number format (Hex, Binary, etc.)

## Keyboard Quick Reference

| Key | Action |
|-----|--------|
| `W` / `S` | Zoom in / out |
| `A` / `D` | Pan left / right |
| `Q` / `E` | Move cursor left / right |
| `Shift+Q` / `Shift+E` | Jump to previous / next transition |
| `Z` | Reset zoom center |
| `R` | Reset view (full zoom out) |
| `Ctrl+T` | Toggle theme (dark/light) |
| `Ctrl+D` | Toggle dock mode |

## Next Steps

- [Loading Waveform Files](./loading-files.md) - Learn about multi-file projects
- [Timeline Navigation](./navigation.md) - Master zoom and pan controls
- [Keyboard Shortcuts](./keyboard-shortcuts.md) - Full shortcut reference
- [Configuration](./configuration.md) - Customize NovyWave
