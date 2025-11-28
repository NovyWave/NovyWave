# NovyWave Configuration System

NovyWave uses a hierarchical configuration system that supports both per-project and global settings.

## Configuration Files

### Per-Project Config (`.novywave`)

Located in the workspace/project root directory. Contains workspace-specific settings:

- Opened waveform files
- Selected scope and variables
- Panel dimensions (per dock mode)
- Theme preference
- Timeline cursor position and zoom

**Resolution**: When NovyWave starts, it checks for `.novywave` in the current working directory first.

### Global Workspace History (`.novywave_global`)

Stores workspace history shared across all projects:

- Recent workspace paths
- Last selected workspace
- File picker tree state (scroll position, expanded directories)

**Location**: Platform-specific config directory:
- **Linux**: `~/.config/novywave/.novywave_global`
- **macOS**: `~/Library/Application Support/novywave/.novywave_global`
- **Windows**: `%APPDATA%\novywave\.novywave_global`

### Tauri Desktop Global Config (`config.toml`)

When no per-project `.novywave` exists, Tauri mode uses a global config:

**Location**: Platform-specific config directory:
- **Linux**: `~/.config/novywave/config.toml`
- **macOS**: `~/Library/Application Support/novywave/config.toml`
- **Windows**: `%APPDATA%\novywave\config.toml`

## Config Resolution Order

### Browser Mode (Development)

1. **Per-project**: `{cwd}/.novywave` for workspace settings
2. **Global**: `{platform_config_dir}/novywave/.novywave_global` for workspace history

### Tauri Desktop Mode

1. **Per-project**: `{cwd}/.novywave` (if exists) → loads workspace settings
2. **Global fallback**: `{platform_config_dir}/novywave/config.toml` → creates/loads default settings
3. **Workspace history**: `{platform_config_dir}/novywave/.novywave_global` → same as browser mode

## Platform Config Directories

The `dirs::config_dir()` Rust crate resolves to:

| Platform | Path |
|----------|------|
| Linux | `~/.config/` |
| macOS | `~/Library/Application Support/` |
| Windows | `%APPDATA%\` (typically `C:\Users\{user}\AppData\Roaming\`) |

## File Format

All configuration files use TOML format. See `.novywave_global_example` in the repo root for an example of global workspace history structure.

## Development Notes

- Per-project config enables version control of workspace state (add `.novywave` to `.gitignore` if not wanted)
- Global config ensures portable behavior across installations (AppImage, installers, etc.)
- Config is auto-saved with debouncing to prevent excessive disk writes
