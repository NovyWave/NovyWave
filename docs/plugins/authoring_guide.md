# NovyWave Plugin Authoring Guide *(Draft – 2025-10-09)*

This guide captures the current workflow for building and wiring WebAssembly plugins into the NovyWave backend. The runtime surface is still evolving; expect updates as we expand the WIT interfaces and roll out additional host capabilities.

## Runtime Snapshot
- **Host runtime:** Wasmtime 36.x LTS (component model enabled, backtrace details on).
- **Shared WIT:** `plugins/wit/plugins.wit` exports the minimal `runtime` world (`init`, `greet`, `shutdown`) with *no* host imports yet.
- **Host crate:** `backend/crates/plugin_host` exposes `PluginHost` + `PluginHandle`; `backend/src/plugins.rs` keeps a singleton manager that caches the last applied config to avoid redundant reloads.
- **Reference plugin:** `plugins/hello_world` demonstrates a Rust component compiled with `wit-bindgen 0.41` and `cargo component`.

## Prerequisites
1. Install the Rust component toolchain:
   ```bash
   cargo install cargo-component
   ```
2. Ensure `wasm32-wasip1` is the active target (cargo-component adds it automatically):
   ```bash
   rustup target add wasm32-wasip1
   ```

## Creating a Plugin
1. Create a new crate under `plugins/<plugin_id>/` with `crate-type = ["cdylib"]`.
2. Add `wit-bindgen` (0.41) and any runtime dependencies (e.g., `once_cell`).
3. Point `package.metadata.component.target` at `../wit` and select the `runtime` world:
   ```toml
   [package.metadata.component]
   package = "novywave:plugins/<plugin_id>"

   [package.metadata.component.target]
   path = "../wit"
   world = "runtime"
   ```
4. Use `wit_bindgen::generate!` to bind the shared WIT and implement the generated `Guest` trait:
  ```rust
  wit_bindgen::generate!({
      path: "../wit",
      world: "runtime",
  });

  use bindings::Guest;

  struct MyPlugin;

  impl Guest for MyPlugin {
      fn init() {}
      fn greet() {}
      fn shutdown() {}
  }

  export!(MyPlugin);
  ```

## Build & Artifact Locations
Rebuild all plugins with:
```bash
makers build_plugins
```

Artifacts are copied to `plugins/dist/`:
```
plugins/dist/hello_world_plugin.wasm
```

Point `.novywave` entries at the dist artifact (use relative paths when possible):
```toml
[plugins]
schema_version = 1

[[plugins.entries]]
id = "novywave.hello_world"
enabled = true
artifact_path = "plugins/dist/hello_world_plugin.wasm"
[plugins.entries.config]
greeting = "NovyWave"
```

> Note: the sample `greeting` key currently has no effect in the hello_world plugin—it simply shows how plugin-specific tables round-trip through `.novywave`.

## Runtime Behaviour
- The backend reloads plugins on config load/save (`plugins::reload_plugins`). It now skips reloads when the `PluginsSection` has not changed, so you should see a single `init/greet` log per actual update.
- Successful loads log `init: initialized | last: greet() ok`; `greet()` currently returns `()` and simply verifies that the component executed.
- Failed loads disable the plugin until the next config replay; the status message surfaces the error.
- Disabled entries remain tracked but are not instantiated (state stays `Disabled`).

## Next Steps
- Extend the WIT surface with filesystem enumeration and watcher subscriptions.
- Document the relay contract once plugin events flow through `plugin_event_relay`.
- Add automation (`makers plugin-build`) to rebuild all plugins before packaging.

Feedback and edits are welcome—keep this guide in sync with runtime/API changes.
