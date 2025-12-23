# Configuration Format Reference

The NovyWave configuration is stored as TOML format with the following structure:

```toml
[workspace]
dock_mode = "Bottom" # "Right" or "Bottom"
theme = "Dark" # "Dark" or "Light"

[files]
tracked_files = ["/path/to/file1.vcd", "/path/to/file2.fst"]

[scope]
selected_scope_id = "file1.vcd|TOP|cpu"
expanded_scopes = ["file1.vcd|TOP", "file2.fst|testbench"]

[variables]
selected_variables = [
    {id = "file1.vcd|TOP|cpu|clk", formatter = "Bin"},
    {id = "file1.vcd|TOP|cpu|data", formatter = "Hex"}
]
variable_filter = ""

[panels]
[panels.right_mode]
files_panel_width = 300
variables_panel_width = 400
timeline_panel_height = 600
variables_name_column_width = 150
variables_value_column_width = 100

[panels.bottom_mode]
files_panel_width = 300
files_panel_height = 350
variables_panel_width = 400
variables_name_column_width = 150
variables_value_column_width = 100

[timeline]
cursor_position_ns = 125000000 # nanoseconds
zoom_center_ns = 0
zoom_level = 1.0
visible_range_start_ns = 0
visible_range_end_ns = 250000000

[dialogs]
[dialogs.file_picker]
scroll_position = 0
expanded_directories = ["/home/user", "/home/user/projects"]

[global.workspace_history]
last_selected = "/home/user/repos/NovyWave"
recent_paths = [
  "/home/user/repos/NovyWave",
  "/home/user/projects/example"
]

[global.workspace_history.tree_state."/home/user/repos/NovyWave"]
scroll_top = 128.0
expanded_paths = [
  "/home/user/repos/NovyWave/hardware",
  "/home/user/repos/NovyWave/test_files"
]

[errors]
toast_auto_dismiss_ms = 5000
```

## Auto-Save Behavior

Configuration is automatically saved to disk with proper debouncing to prevent excessive file writes:

- **Panel resizing**: 500ms debounce after drag operation completes
- **Variable selection**: Immediate save on selection change
- **Timeline navigation**: 1000ms debounce for cursor/zoom changes
- **Dialog interactions**: Immediate save for expand/collapse states
- **Theme/dock changes**: Immediate save

## Error Handling

When configuration file cannot be read:
- Display error message: "Configuration file is corrupted: '[specific error]'. Please fix the file or remove '[absolute path]' to reset settings."
- Fall back to default configuration values
- Do not retry loading automatically - user must fix or remove corrupted file
