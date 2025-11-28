# Configuration

NovyWave automatically saves and restores your workspace state.

## Configuration Files

### Location

**Development Mode:**
- `.novywave` in project root

**Production Mode:**
- **Linux:** `~/.config/novywave/.novywave`
- **macOS:** `~/Library/Application Support/com.novywave.app/.novywave`
- **Windows:** `%APPDATA%/com.novywave.app/.novywave`

### Per-Project Configuration

Like Cargo.toml for Rust projects, NovyWave uses per-project configuration:

- If `.novywave` exists in current directory, it's used
- Otherwise, uses global user configuration
- This allows project-specific waveform setups

## What Gets Saved

### Workspace State
- **Dock mode** - Right or Bottom panel layout
- **Theme** - Dark or Light

### File State
- **Loaded files** - Paths to waveform files
- **Expanded scopes** - Which tree nodes are open
- **Selected scope** - Current scope for variables panel

### Variable Selection
- **Selected variables** - Which signals are displayed
- **Formatters** - Per-variable format settings (Hex, Bin, etc.)

### Panel Dimensions
- **Panel sizes** - Width/height of each panel
- **Column widths** - Variable name and value column sizes
- Saved separately for Right and Bottom dock modes

### Timeline State
- **Cursor position** - Current time position
- **Zoom center** - Zoom reference point
- **Zoom level** - Current magnification
- **Visible range** - Displayed time window

### Dialog State
- **File picker scroll** - Last scroll position
- **Expanded directories** - Which folders are open

## Configuration Format

The `.novywave` file uses TOML format:

```toml
[workspace]
dock_mode = "Bottom"
theme = "Dark"

[files]
tracked_files = ["/path/to/design.vcd"]

[scope]
selected_scope_id = "design.vcd|TOP|cpu"
expanded_scopes = ["design.vcd|TOP"]

[variables]
selected_variables = [
    {id = "design.vcd|TOP|cpu|clk", formatter = "Bin"},
    {id = "design.vcd|TOP|cpu|data", formatter = "Hex"}
]

[panels.bottom_mode]
files_panel_width = 300
files_panel_height = 350

[timeline]
cursor_position_ns = 125000000
zoom_level = 1.0
```

## Auto-Save Behavior

Configuration saves automatically with debouncing:

| Action | Delay |
|--------|-------|
| Panel resize | 500ms after drag ends |
| Variable selection | Immediate |
| Timeline navigation | 1000ms debounce |
| Theme/dock toggle | Immediate |

## Resetting Configuration

### Full Reset

Delete the configuration file:

```bash
# Linux
rm ~/.config/novywave/.novywave

# macOS
rm ~/Library/Application\ Support/com.novywave.app/.novywave

# Windows (PowerShell)
Remove-Item $env:APPDATA\com.novywave.app\.novywave
```

### Corrupted Configuration

If NovyWave fails to start due to corrupted config, it will show an error message with the file path. Either fix the TOML syntax or delete the file.

## Environment Variables

Currently, NovyWave does not use environment variables for configuration. All settings are managed through the `.novywave` file.
