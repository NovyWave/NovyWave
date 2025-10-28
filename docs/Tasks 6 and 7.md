# Demo

- TODO: Video

# Task 6: Workspace and Plugin Management

## Milestones

### **6a.** Design UI elements for workspace management backend

![Header Dark](./Task%206%20and%207%20-%20media/header_dark.png)

![Header Light](./Task%206%20and%207%20-%20media/header_light.png)

![Open Workspace Dialog Dark](./Task%206%20and%207%20-%20media/open_workspace_dialog_dark.png)

![Open Workspace Dialog Light](./Task%206%20and%207%20-%20media/open_workspace_dialog_light.png)

### **6b.** Implement functionality to open folders as workspaces and create a ".novywave" workspace folder	

- See the video at the top.
- `.novywave` file is created instead of a special workspace folder in the opened folders. It allows more flexible configuration and it reduces complexity.

### **6c.**  Integrate Wasmtime and create a proof-of-concept plugin	

plugins/hello_world/src/lib.rs:

```rust
mod bindings {
    wit_bindgen::generate!({
        path: "./wit",
    });
}

use bindings::{__export_world_plugin_cabi, novywave::hello_world::host, Guest};

struct HelloWorld;

impl Guest for HelloWorld {
    fn init() {
        host::log_info("Hello World!");
    }

    fn shutdown() {
        host::log_info("hello_world plugin shutting down");
    }
}

__export_world_plugin_cabi!(HelloWorld with_types_in bindings);
```
Server log:
```
🔌 PLUGIN[novywave.hello_world]: Hello World!
```

### **6d.** Enable plugin loading from global and workspace storage

Every plugin is defined by its id and path to Wasm component in a .novywave file placed in a project root or a global folder (it's treated as another - default - workspace).

.novywave:
```toml
# ...

[plugins]
schema_version = 1

#### Plugin hello_world ####
[[plugins.entries]]
id = "novywave.hello_world"
enabled = true
artifact_path = "plugins/dist/hello_world_plugin.wasm"

[plugins.entries.config]

#### Plugin reload_watcher ####
[[plugins.entries]]
id = "novywave.reload_watcher"
enabled = true
artifact_path = "plugins/dist/reload_watcher_plugin.wasm"

[plugins.entries.config]

#### Plugin files_discovery ####
[[plugins.entries]]
id = "novywave.files_discovery"
enabled = true
artifact_path = "plugins/dist/files_discovery_plugin.wasm"

[plugins.entries.config]
debounce_ms = 200
patterns = ["test_files/to_discover/**/*.*"]
```

### **6e.** Remember recently opened workspace

- A new configuration file `.novywave_global` has been introduced with `last_selected` workspace property.

.novywave_global:

```toml
# NovyWave Global Configuration
# Stores workspace history shared across all projects

[global.workspace_history]
last_selected = "/home/martinkavik/repos/NovyWave/test_files/my_workspaces/workspace_a"
recent_paths = [
    "/home/martinkavik/repos/NovyWave/test_files/my_workspaces/workspace_a",
    "/home/martinkavik/repos/NovyWave",
    "/home/martinkavik/repos/NovyWave/test_files/my_workspaces/workspace_b",
]

[global.workspace_history.picker_tree_state]
scroll_top = 1016.0
expanded_paths = [
    "/",
    "/home",
    "/home/martinkavik",
    "/home/martinkavik/repos",
    "/home/martinkavik/repos/NovyWave",
    "/home/martinkavik/repos/NovyWave/test_files",
    "/home/martinkavik/repos/NovyWave/test_files/my_workspaces",
]
```

# Task 7: Basic Workspace Plugins

## Milestones

### **7a.** Build a waveform discovery plugin with NovyWave API support to detect waveform files	

plugins/files_discovery/wit/plugin.wit:

```wit
package novywave:files-discovery;

interface host {
  /// Return the current set of opened waveform files.
  get-opened-files: func() -> list<string>;

  /// Replace the watched directory set for this plugin; supplying an empty list clears watchers.
  register-watched-directories: func(directories: list<string>, debounce-ms: u32);

  /// Remove any registered directory watchers for this plugin.
  clear-watched-directories: func();

  /// Request the host to open the provided waveform files.
  open-waveform-files: func(paths: list<string>);

  /// Return the plugin configuration as a TOML document.
  get-config-toml: func() -> string;

  /// Log an informational message via the host.
  log-info: func(message: string);

  /// Log an error message via the host.
  log-error: func(message: string);
}

world plugin {
  import host;

  /// Called once when the plugin starts; set up watchers and discover existing files.
  export init: func();

  /// Called when the backend reloads the plugin configuration or opened file list.
  export refresh-opened-files: func();

  /// Called by the backend when watched directories emit new filesystem entries.
  export paths-discovered: func(paths: list<string>);

  /// Called before the component is unloaded; clean up watchers here.
  export shutdown: func();
}
```

.novywave:

```toml
[[plugins.entries]]
id = "novywave.files_discovery"
enabled = true
artifact_path = "plugins/dist/files_discovery_plugin.wasm"

[plugins.entries.config]
debounce_ms = 200
patterns = ["test_files/to_discover/**/*.*"]
```

### **7b.** Develop an auto-reload plugin to automatically update displayed data	

- See video at the top.

plugins/reload_watcher/src/lib.rs:

```rust
mod bindings {
    wit_bindgen::generate!({
        path: "./wit",
    });
}

use bindings::{__export_world_plugin_cabi, novywave::reload_watcher::host, Guest};

struct ReloadWatcher;

fn configure_watchers() {
    let opened = host::get_opened_files();
    let debounce_ms = 250u32;
    host::register_watched_files(&opened, debounce_ms);
    host::log_info(&format!(
        "Registered {} waveform path(s) for live reload",
        opened.len()
    ));
}

fn request_reload(paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    host::reload_waveform_files(paths);
    host::log_info(&format!(
        "Requested reload for {} waveform path(s)",
        paths.len()
    ));
}

impl Guest for ReloadWatcher {
    fn init() {
        configure_watchers();
    }

    fn refresh_opened_files() {
        configure_watchers();
    }

    fn watched_files_changed(paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        request_reload(&paths);
    }

    fn shutdown() {
        host::log_info("Reload watcher shutting down");
        host::clear_watched_files();
    }
}

__export_world_plugin_cabi!(ReloadWatcher with_types_in bindings);

```

### **7c.**  Create a state saver plugin to save and restore application configurations

- (Re)storing, configuration, managing workspaces and .novywave(_global) files is a fundamental part of the application. It does not make too much sense to move the logic into a plugin and it would also make the logic more complex and less flexible. It means I decided to implement the saver in the application code.
