## Plugin Runtime & Architecture (Draft – 2025-10-09)

### Requirements Snapshot
- Sandbox third-party logic while letting it enumerate waveform files in user-selected folders.
- Enable future plugins to trigger rescans or react to file-system change notifications without blocking the main actor loops.
- Expose explicit loading/empty states in relays instead of inventing defaults; plugins must publish results through existing dataflow patterns.
- Target Rust-first authoring for internal plugins, but keep the door open for other languages via the WASM component model.
- Prefer runtimes with a path to multi-threaded workloads (e.g., offloading file parsing) once wasi-threads support hardens.
- Persist plugin enable/disable state and per-plugin configuration in `.novywave`, keeping serialization forwards-compatible with existing config replay.

### Runtime Options Survey

#### Wasmtime + Component Model (wasi-preview2/preview3)
- First-class Rust support via `wit-bindgen` and the `wasmtime` crate, plus `Component` APIs that broker strongly typed boundaries.
- Component model stable enough for production (v1.0 of the spec approved January 2024), with ongoing improvements in Wasmtime 32–37 (late 2024–2025) covering async, faster module loading, and better resource APIs.
- wasi-preview2 shipping today for capability-oriented I/O; wasi-preview3 work adds incremental features like shared memory, streams, and improved error handling.
- Threads: wasi-threads is still preview-only; we can start single-threaded and gate threaded plugins behind a feature once the runtime marks it stable.

#### Wasmer 6 + WASIX
- Bundles POSIX-like APIs (networking, filesystem, epoll) through WASIX, making rich OS integration straightforward without hand-rolling host shims.
- Introduced cooperative and preemptive scheduler improvements plus zero-cost wasm exceptions in 6.0, and it already exposes wasi-threads-style parallelism when compiled with `--features threads`.
- Mature cross-language toolchain, but component-model support is tracked separately and can lag Wasmtime; typed interfaces require more hand-written glue today.
- WASIX expands surface area; we must review the extra syscalls against our sandboxing story before exposing to community plugins.

#### Extism SDK (Wasmtime/Wasmer shell)
- Provides a stable host-plugin ABI with automatic JSON/msgpack parameter encoding, hot-reload helpers, and simple host function registration.
- Lets us pick Wasmtime or Wasmer under the hood per plugin while keeping the application code agnostic.
- Thread support follows the selected runtime; still limited by wasi-threads maturity.
- Extra host indirection means larger dependency surface, but it accelerates cross-language authoring if we ever publish the API broadly.

#### WasmEdge 0.14+
- Focused on cloud-native and AI workloads; ships with async networking, WASI sockets, and LLVM-based optimizations.
- Component-model work in progress; best suited for WITless modules or WASI snapshot 1/preview2 today.
- Good for embedding if we need built-in networking, but documentation/examples around filesystem delegation and plugin reloads are thinner than Wasmtime/Wasmer.

### Runtime Status Check (2025-10-08)
- **Wasmtime 37.0.1 (2025-09-23)** – ships full exception-handling support (behind a flag), preview WASI 0.3 async APIs, and Linux `PAGEMAP_SCAN` acceleration; we should track `wasmtime` crate >=37 for async experiments while keeping production on the LTS line.
- **Wasmtime 36 LTS (2025-08-20)** – supported through 2027-08-20; plan to embed 36.x for the initial plugin host to soak before adopting 37’s new async surface once stabilized.
- **Wasmer 6.0 (2025-04-25)** – multi-backend binaries, WASIX syscall coverage (including threads, sockets, posix_spawn), and zero-cost exceptions; strong fallback if we need POSIX-heavy plugins.
- **WASI Preview 3 roadmap (2025-08)** – native async enters preview; stabilize by 2025-11 per WASI roadmap, so we should sandbox async APIs in dev builds and avoid exposing them to third-party plugins until the release train lands.
- **Extism 1.7.0 (2025-09-24)** – introduces `PluginBuilder::with_wasmtime_config`, making it easier to tune the embedded Wasmtime engine; reach for Extism if we need a higher-level ABI or multi-language SDK coverage on top of Wasmtime.

### Recommendation (Draft)
- Default to Wasmtime’s component model for first implementation:
  - Gives us typed WIT interfaces that integrate cleanly with our actor/relay boundaries.
  - Bytecode Alliance is driving WASI maturity; we can track wasi-threads GA and upgrade in-place.
- Wrap Wasmtime in a thin “PluginHost” crate that:
  - Manages component loading/unloading and shared engine configuration.
  - Registers host capabilities for directory enumeration, file metadata, and file-change subscription (backed by Rust notify crate or similar).
  - Surfaces plugin results through dedicated relays (e.g., `plugin_discovery_relay`), keeping the rest of the system ignorant of Wasmtime specifics.
- Revisit Extism if we decide to open the plugin API externally; it could sit atop Wasmtime for multi-language support with minimal refactor.
- Keep Wasmer + WASIX as Plan B if we hit a blocker around filesystem/event integration or need mature threaded WASI before Wasmtime catches up.

### Configuration & Toggle Model
- `.novywave` will persist plugin state so replays stay consistent across restarts.
  - Extend the root with a `[plugins]` table containing a `schema_version` integer (start at `1` to allow migrations).
  - Each plugin registers under `[[plugins.entries]]` with fields:
    - `id`: stable string matching the component package name.
    - `enabled`: boolean toggling load execution.
    - `artifact_path`: relative path to the compiled component (e.g., `plugins/hello_world/target/component/hello_world.wasmc`).
    - `config`: inline table for plugin-defined settings (serde-serializable so we can round-trip without losing keys).
    - `watch`: optional inline table (`directories: ["/abs/path"]`, `debounce_ms: 250`) to seed watcher relays.
- Config playback flow:
  - Config saver replays `[plugins]` block after core UI/workspace state so plugin toggles are ready before file relays fire.
  - `PluginManagerActor` listens for config replay messages and emits explicit state relays (`plugin_status_relay`) instead of assuming defaults.
  - Frontend config composer now keeps the last loaded `plugins` section in memory, so manual edits or backend updates are preserved across saves (no more reset-to-empty).
- When config schema evolves, bump `schema_version` and provide migration logic in the config actor (follow existing pattern used for workspace timeline recovery).

#### Configuration Example
```toml
[plugins]
schema_version = 1

[[plugins.entries]]
id = "novywave.hello_world"
enabled = true
artifact_path = "plugins/hello_world/target/component/hello_world.wasmc"
[plugins.entries.config]
greeting = "NovyWave"

[[plugins.entries]]
id = "novywave.waveform_scanner"
enabled = true
artifact_path = "plugins/waveform_scanner/target/component/waveform_scanner.wasmc"
[plugins.entries.config]
extensions = ["fst", "vcd"]
directories = ["/home/user/waves"]
[plugins.entries.watch]
directories = ["/home/user/waves"]
debounce_ms = 250
```

#### Config Loading Flow
1. Config actor parses `.novywave` into a strongly typed `PluginsConfig` struct:
   ```rust
   #[derive(Deserialize, Clone)]
   pub struct PluginsConfig {
       pub schema_version: u32,
       pub entries: Vec<PluginEntry>,
   }

   #[derive(Deserialize, Clone)]
   pub struct PluginEntry {
       pub id: String,
       pub enabled: bool,
       pub artifact_path: PathBuf,
       #[serde(default)]
       pub config: serde_json::Value,
       #[serde(default)]
       pub watch: Option<PluginWatch>,
   }
   ```
2. Actor emits `plugins_config_replay_relay` with the parsed payload.
3. `PluginManagerActor` reacts:
   - Unloads plugins no longer present or now disabled.
   - Loads or reloads enabled entries, passing `config` (as JSON) into the Wasmtime component during instantiation.
4. Plugin status changes feed `plugin_status_relay`, which the UI observes to render toggle states or error banners.
5. When users toggle a plugin in the UI, the actor updates its in-memory state and triggers the config saver to persist the new `.novywave` block.

### Plugin Packaging & Build Flow
- Source lives in `plugins/<plugin_id>/` as a standalone Cargo crate targeting the Wasmtime component model via `wit-bindgen`.
- Shared WIT definitions sit in `plugins/wit/`; individual plugins reference the shared package instead of embedding their own copies.
- Build pipeline today:
  1. `makers build_plugins` discovers every `plugins/**/Cargo.toml` and runs `cargo component build --release --manifest-path <crate>`.
  2. Finished components are copied into `plugins/dist/<plugin_id>_plugin.wasm`; `.novywave` entries point at these dist artifacts.
  3. We rely on Wasmtime’s own caching; there is no plugin manifest or host-side artifact cache yet.
- `plugins/hello_world` acts as the reference implementation:
  - Exports `init`, `greet`, and `shutdown`; `greet()` currently returns `()` and simply proves the component can be invoked.
  - Keeps all logic in Rust with no host imports, serving as the minimal smoke test for PluginHost wiring.

### Host Capability Surface
- Initial host functions (modeled in WIT) must cover:
  - `filesystem.list_dir(directory: string) -> list<DirectoryEntry>` for targeted folder enumeration.
  - `filesystem.read_metadata(path: string) -> Option<FileMetadata>` to query waveform files safely.
  - `watch.subscribe(directories: list<string>, debounce_ms: u32) -> SubscriptionId` feeding change events back through relays.
  - `plugin.log(level: LogLevel, message: string)` bridging into our structured logging without granting stdout/stderr directly.
  - `relay.publish(id: string, payload: PluginEvent)` so plugins push results into actor land; the host validates payloads before forwarding to relays.
- Capabilities are opt-in: the host only wires functions requested by the plugin manifest and records grants in a debug log for auditing.
- File system access is mediated through capability checks that map directories enumerated in `.novywave` to host-side sandbox handles; plugins never receive raw OS paths not explicitly granted.

### Execution Model & Threading
- Host launches plugins on demand inside a shared Wasmtime `Engine` to amortize compilation cost.
- Execution starts single-threaded; `wasi-threads` support is gated behind a runtime feature flag until the Wasmtime crate documents embedders-safe semantics.
- Long-running plugin work dispatches through async tasks; results flow back over relays without blocking actor loops.
- File watcher callbacks debounce and coalesce events before invoking plugin exports to maintain deterministic behavior.
- Metrics hooks collect execution time, memory usage, and crash counts per plugin for future dashboards.

### Security & Maintenance
- Track Wasmtime security advisories and plan monthly digest checks; escalate immediate patches for CVEs that affect our embedder profile.
- Pin the host runtime to `wasmtime` 36.x LTS until 37.0.2 (or newer) lands, because 37.0.0–37.0.1 ship a C-API `externref`/`anyref` leak (CVE-2025-61670); reassess once the fixed release is published.
- Use component-level sandboxing (no direct WASIX extensions) for community plugins; internal plugins may request elevated capabilities behind a feature flag.
- Provide a dry-run mode that loads plugin metadata and validates WIT compatibility without executing any exports, giving user feedback before enabling a plugin.
- Document the process for revoking plugins (set `enabled = false`, host unloads and releases resources) and for clearing cached artifacts.

### Host ↔ Plugin Interface (WIT Draft)
```wit
package novywave:plugins;

world host {
  import log: func(level: log-level, message: string);
  import filesystem-list-dir: func(directory: string) -> list<directory-entry>;
  import filesystem-read-metadata: func(path: string) -> option<file-metadata>;
  import watch-subscribe: func(directories: list<string>, debounce-ms: u32) -> subscription;
  import relay-publish: func(channel: string, event: plugin-event);
  import config-get: func() -> config-data;

  export init: func(config: config-data) -> init-result;
  export handle-event: func(event: host-event) -> result<(), plugin-error>;
  export shutdown: func();
}

enum log-level { trace, debug, info, warn, error }

record directory-entry { path: string, is-dir: bool }

record file-metadata { path: string, size: u64, modified-unix-nanos: u128 }

record subscription { id: u64 }

variant plugin-event {
  discovery: discovery-payload,
  log: string,
}

variant host-event {
  tick,
  filesystem-change: change-notification,
  config-updated: config-data,
}

record change-notification {
  subscription-id: u64,
  paths: list<string>,
}

type config-data = list<config-entry>

record config-entry { key: string, value: string }

variant init-result {
  ok: plugin-metadata,
  err: plugin-error,
}

record plugin-metadata {
  version: string,
  description: string,
  capabilities: list<string>,
}

variant plugin-error {
  invalid-config(string),
  io(string),
  panic(string),
}

record discovery-payload {
  plugin-id: string,
  discovered: list<string>,
}
```
- `config-get` lets plugins lazily fetch structured config if they defer reads.
- `handle-event` receives host-driven events (watch notifications, config updates, periodic ticks).
- `relay-publish` keeps plugin outputs flowing back into relays, with the host validating channel IDs against an allowlist derived from `.novywave`.

### Plugin Lifecycle
- Load order: on startup, the host reads `.novywave`, validates artifacts, instantiates enabled plugins, and emits `plugin_status_relay` updates (`Loading`, `Ready`, `Error`).
- Hot reload: changing `enabled` or `config` triggers a config relay; host unloads the old instance, reloads artifacts if needed, and republishes state.
- No-op replays: the manager keeps a clone of the last `PluginsSection`; identical payloads short-circuit without unloading handles, so duplicate `init/greet` logs disappear.
- Watcher integration: plugins that request file-change subscriptions receive events as `PluginEvent::FileChanged` and can invoke host exports to rescan or publish new data.
- Failure handling: runtime panics bubble to the host; we mark the plugin `Error` and debounce retries (e.g., exponential backoff) to avoid tight crash loops.
- Telemetry: host records plugin duration/memory counters and exposes them via an internal debug relay for observability tooling.

### Planned Plugins & Use Cases
- `novywave.hello_world` (MVP)
  - Validates basic host-to-plugin round-trip.
  - No host imports yet; the plugin just touches a static greeting and returns unit from `greet()`.
  - Gives the host a deterministic place to smoke-test `init → greet → shutdown` without touching relays.
- `novywave.waveform_scanner`
  - Imports directory enumeration and metadata functions to locate waveform files matching configured extensions.
  - Publishes discovery results to a dedicated relay consumed by `TrackedFiles` actors.
  - Uses `watch.subscribe` to receive change events; on notification, re-runs discovery with debounce.
- `novywave.auto_reload`
  - Listens for change events (from watchers or `waveform_scanner` outputs) and triggers host relays that refresh the active timeline without blocking the UI.
  - Configurable thresholds for batching (e.g., wait for N events or T milliseconds before triggering refresh).
- Future ideas: parse-specific preprocessors, report generators, or integration bridges (e.g., remote repository sync) leveraging the same capability surface.

### Immediate Next Steps
- Define a WIT package describing the minimal host capability surface (directory listing, change subscription, log/report). _(Initial `runtime` world landed in `plugins/wit/plugins.wit`; expand with filesystem/watch APIs in next iteration.)_
- Extend the `.novywave` schema with a `plugins` section (enable flags + plugin-specific config blobs) and document how actors replay these settings at startup. _(Schema extended with `PluginsSection` in `shared::AppConfig` and loaded via `plugins::reload_plugins`.)_
- Prototype `plugins/hello_world` as a Wasmtime component built from Rust; host now calls `init` and `greet()` (unit) to confirm instantiation. _(Implemented 2025-10-09; expand once host imports are available.)_
- Spike the PluginHost crate with synchronous execution; document the path to async/multi-thread execution once wasi-threads stabilizes.
- Document capability boundaries and relay wiring here before merging implementation PRs.
- Ensure config replays avoid redundant reloads by caching the last applied `PluginsSection` in the manager. _(Implemented 2025-10-09.)_

### Backend Integration Plan
- **New crate**: `backend/crates/plugin_host` exporting `PluginHost` (engine manager) + `PluginHandle` (per-plugin façade). The crate owns Wasmtime engine configuration, module loading, and translates WIT imports/exports into Rust traits.
- **Actors**:
  - `PluginManagerActor`: orchestrates lifecycle (load, reload, unload), listens to `plugins_config_replay_relay`, and emits `plugin_status_relay`.
  - `PluginExecutionActor`: per-plugin worker spawned by `PluginManagerActor`, executing plugin exports on a dedicated async task to keep the manager non-blocking.
  - `PluginEventRelay`: typed relay struct broadcasting plugin-originated messages (e.g., `PluginEvent::Discovery`).
- **Relays**:
  - `plugins_config_replay_relay`: emits `PluginsConfig`.
  - `plugin_status_relay`: publishes `(plugin_id, PluginRuntimeState, Option<String /*error*/>)`.
  - `plugin_event_relay`: transports domain events (`Discovery`, `Log`, `RefreshTimeline`) back into the dataflow layer.
- **Host capabilities implementation**:
  - File queries delegate to existing `TrackedFilesActor` APIs to avoid divergent logic; plugin triggers a message instead of touching filesystem directly when possible.
  - Watch subscriptions wrap the existing notify-based watcher service so we reuse debouncing/backoff code.
- **Error handling**:
  - Convert Wasmtime traps into `PluginError::RuntimeTrap` and surface them through `plugin_status_relay`.
  - Log structured errors (including backtraces when available) to terminal output with plugin ids for debugging.
- **Hot reload flow**:
  1. Config toggle arrives via UI → config actor persists `.novywave` → `plugins_config_replay_relay`.
  2. Manager compares current handles, unloads disabled ones, spawns new handles for enabled entries.
  3. Execution actor instantiates component, calls `init(config)`; success triggers `plugin_status_relay: Ready`, failure emits `Error`.

### UI & UX Considerations
- Add a Plugins panel that lists entries from `plugin_status_relay`, showing state badges (`Loading`, `Ready`, `Error`, `Disabled`) and latest message.
- Toggling a plugin updates the config actor, which writes `.novywave` and acknowledges via toast once `plugin_status_relay` confirms new state.
- Error states surface actionable copy (e.g., “Failed to load novywave.waveform_scanner: invalid config key `extensions`”) and offer a retry button that replays the config entry.
- For plugins exposing configurable fields, the UI provides a schema-driven editor:
  - Plugins supply a JSON schema snippet in `plugin-metadata.capabilities`; the UI maps it into form controls.
- Provide a per-plugin log view subscribing to `plugin_event_relay` entries of kind `Log`.

### Build & Tooling Tasks
- `makers build_plugins` batches `cargo component build` for every crate under `plugins/` and copies artifacts into `plugins/dist/`; `makers watch_plugins` wraps it in `cargo watch` so wasm artifacts refresh automatically before the dev server reloads them.
- Add a preflight script (`scripts/check_plugins.rs`) that validates:
  - `Plugin.toml` matches `.novywave` entries (ids and artifact paths).
  - Compiled artifacts exist and target the supported WASI preview.
  - WIT definitions compile with `wit-bindgen` and align with host expectations.
- Update CI pipeline to run `cargo fmt`, `cargo clippy`, `cargo test`, then `makers plugin-build --all` in check mode (dry-run) to ensure plugins stay buildable.

### Testing Strategy
- **Host unit tests**: mock Wasmtime engine using `wasmtime::Config::new().cranelift_debug_verifier(true)` to detect miscompiled components; exercise lifecycle transitions.
- **Integration tests**:
  - Spin up `PluginManagerActor` with an in-memory Wasmtime store, load the `hello_world` component, and assert relay emissions.
  - Simulate filesystem change notifications to validate watcher debouncing.
- **Fixture coverage**: extend `test_files/` with mock waveform directories used by plugin tests so scanner plugins operate on deterministic data.
- **Manual QA checklist**:
  - Enable/disable plugins via UI and confirm `.novywave` persists changes.
  - Modify plugin config (e.g., change `greeting`) and verify live reload without app restart.
  - Trigger filesystem change in watched directory and observe auto-reload plugin behavior.

### Documentation Deliverables
- Update `docs/actors_relays/actor_relay_architecture.md` with new actors/relays once implementation lands.
- Author `docs/plugins/authoring_guide.md` covering:
  - Required directory layout (`src/`, `wit/`, `Plugin.toml`).
  - Host capability catalogue and best practices (avoid long-running synchronous loops; rely on async callbacks).
  - Testing guidance (`cargo component test`, `makers plugin-build`).
- Expand `.claude/extra/project/specs/plugins.md` (this file) with final API signatures after prototyping to keep spec vs implementation aligned.

### Implementation Snapshot (2025-10-09)
- `backend/crates/plugin_host` embeds Wasmtime 36.x LTS with component-model + backtrace support enabled. `load()` instantiates components, calls `init()`, and retains a `PluginHandle` that can invoke `greet()`/`shutdown()` on demand.
- Shared WIT at `plugins/wit/plugins.wit` still surfaces the minimal `runtime` world (`init`, `greet`, `shutdown`) without host imports; capability imports are planned but not implemented yet.
- `plugins/hello_world` compiles to a tiny component that touches a static greeting and returns unit from `greet()`. The host uses it as a smoke test during config replay.
- `.novywave` persists a single enabled entry pointing at `plugins/dist/hello_world_plugin.wasm`; the frontend preserves that block during saves.
- `plugins::reload_plugins` caches the last applied config so noop replays skip unload/reload, which prevents duplicate `greet()` logs while still reporting status on real changes.
