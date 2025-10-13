mod bindings {
    pub mod hello_world {
        wasmtime::component::bindgen!({
            path: "../../../plugins/hello_world/wit",
            world: "plugin",
        });
    }

    pub mod reload_watcher {
        wasmtime::component::bindgen!({
            path: "../../../plugins/reload_watcher/wit",
            world: "plugin",
        });
    }
}

use anyhow::Error;
use bindings::{
    hello_world::{Plugin as HelloPlugin, novywave::hello_world as hello_wit},
    reload_watcher::{Plugin as ReloadPlugin, novywave::reload_watcher as reload_wit},
};
use shared::{CanonicalPathPayload, PluginConfigEntry};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2};

/// Bridge trait implemented by the host runtime to service plugin requests.
pub trait HostBridge: Send + Sync + 'static {
    fn get_opened_files(&self, plugin_id: &str) -> Vec<CanonicalPathPayload>;
    fn register_watched_files(
        &self,
        plugin_id: &str,
        paths: Vec<CanonicalPathPayload>,
        debounce_ms: u32,
    ) -> Result<(), HostBridgeError>;
    fn clear_watched_files(&self, plugin_id: &str);
    fn reload_waveform_files(
        &self,
        plugin_id: &str,
        paths: Vec<CanonicalPathPayload>,
    ) -> Result<(), HostBridgeError>;
    fn log_info(&self, plugin_id: &str, message: &str);
    fn log_error(&self, plugin_id: &str, message: &str);
}

/// Minimal no-op implementation used by tooling that does not provide a runtime bridge.
#[derive(Clone, Default)]
pub struct NullBridge;

impl HostBridge for NullBridge {
    fn get_opened_files(&self, _plugin_id: &str) -> Vec<CanonicalPathPayload> {
        Vec::new()
    }

    fn register_watched_files(
        &self,
        _plugin_id: &str,
        _paths: Vec<CanonicalPathPayload>,
        _debounce_ms: u32,
    ) -> Result<(), HostBridgeError> {
        Ok(())
    }

    fn clear_watched_files(&self, _plugin_id: &str) {}

    fn reload_waveform_files(
        &self,
        _plugin_id: &str,
        _paths: Vec<CanonicalPathPayload>,
    ) -> Result<(), HostBridgeError> {
        Ok(())
    }

    fn log_info(&self, plugin_id: &str, message: &str) {
        println!("[plugin:{}] {}", plugin_id, message);
    }

    fn log_error(&self, plugin_id: &str, message: &str) {
        eprintln!("[plugin:{}] {}", plugin_id, message);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HostBridgeError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

struct HostState {
    table: ResourceTable,
    wasi: WasiCtx,
    plugin_id: String,
    bridge: Arc<dyn HostBridge>,
}

impl HostState {
    fn new(plugin_id: String, bridge: Arc<dyn HostBridge>) -> Self {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .inherit_args()
            .build();
        Self {
            table: ResourceTable::new(),
            wasi,
            plugin_id,
            bridge,
        }
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn host_state_lookup(state: &mut HostState) -> &mut HostState {
    state
}

impl hello_wit::host::Host for HostState {
    fn log_info(&mut self, message: String) -> () {
        self.bridge.log_info(&self.plugin_id, &message);
    }

    fn log_error(&mut self, message: String) -> () {
        self.bridge.log_error(&self.plugin_id, &message);
    }
}

impl reload_wit::host::Host for HostState {
    fn get_opened_files(&mut self) -> Vec<String> {
        self.bridge
            .get_opened_files(&self.plugin_id)
            .into_iter()
            .map(|payload| payload.canonical)
            .collect()
    }

    fn register_watched_files(&mut self, paths: Vec<String>, debounce_ms: u32) -> () {
        let snapshot = self.bridge.get_opened_files(&self.plugin_id);
        let lookup: HashMap<_, _> = snapshot
            .into_iter()
            .map(|payload| (payload.canonical.clone(), payload.display))
            .collect();

        let payloads: Vec<CanonicalPathPayload> = paths
            .into_iter()
            .map(|path| CanonicalPathPayload {
                display: lookup.get(&path).cloned().unwrap_or_else(|| path.clone()),
                canonical: path,
            })
            .collect();

        if let Err(err) = self
            .bridge
            .register_watched_files(&self.plugin_id, payloads, debounce_ms)
        {
            self.bridge.log_error(&self.plugin_id, &err.to_string());
        }
    }

    fn clear_watched_files(&mut self) -> () {
        self.bridge.clear_watched_files(&self.plugin_id);
    }

    fn reload_waveform_files(&mut self, paths: Vec<String>) -> () {
        let snapshot = self.bridge.get_opened_files(&self.plugin_id);
        let lookup: HashMap<_, _> = snapshot
            .into_iter()
            .map(|payload| (payload.canonical.clone(), payload.display))
            .collect();

        let payloads: Vec<CanonicalPathPayload> = paths
            .into_iter()
            .map(|path| CanonicalPathPayload {
                display: lookup.get(&path).cloned().unwrap_or_else(|| path.clone()),
                canonical: path,
            })
            .collect();
        if let Err(err) = self.bridge.reload_waveform_files(&self.plugin_id, payloads) {
            self.bridge.log_error(&self.plugin_id, &err.to_string());
        }
    }

    fn log_info(&mut self, message: String) -> () {
        self.bridge.log_info(&self.plugin_id, &message);
    }

    fn log_error(&mut self, message: String) -> () {
        self.bridge.log_error(&self.plugin_id, &message);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginWorld {
    HelloWorld,
    ReloadWatcher,
}

fn plugin_world(id: &str) -> PluginWorld {
    match id {
        "novywave.reload_watcher" => PluginWorld::ReloadWatcher,
        _ => PluginWorld::HelloWorld,
    }
}

pub struct PluginHost {
    engine: Engine,
    bridge: Arc<dyn HostBridge>,
}

impl PluginHost {
    pub fn new(bridge: Arc<dyn HostBridge>) -> Result<Self, PluginHostError> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(false);
        config.wasm_multi_memory(true);
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);

        let engine = Engine::new(&config).map_err(PluginHostError::Engine)?;
        Ok(Self { engine, bridge })
    }

    pub fn load(&self, entry: &PluginConfigEntry) -> Result<PluginHandle, PluginHostError> {
        let component_path = Path::new(&entry.artifact_path);
        let component = Component::from_file(&self.engine, component_path).map_err(|source| {
            PluginHostError::ComponentLoad {
                plugin_id: entry.id.clone(),
                source,
            }
        })?;

        let world = plugin_world(&entry.id);

        let host_state = HostState::new(entry.id.clone(), self.bridge.clone());
        let mut store = Store::new(&self.engine, host_state);

        let runtime =
            match world {
                PluginWorld::HelloWorld => {
                    let mut linker = Linker::new(&self.engine);
                    p2::add_to_linker_sync(&mut linker).map_err(|source| {
                        PluginHostError::Instantiation {
                            plugin_id: entry.id.clone(),
                            source,
                        }
                    })?;
                    HelloPlugin::add_to_linker::<HostState, HasSelf<HostState>>(
                        &mut linker,
                        host_state_lookup,
                    )
                    .map_err(|source| PluginHostError::Instantiation {
                        plugin_id: entry.id.clone(),
                        source,
                    })?;

                    let runtime = HelloPlugin::instantiate(&mut store, &component, &linker)
                        .map_err(|source| PluginHostError::Instantiation {
                            plugin_id: entry.id.clone(),
                            source,
                        })?;

                    runtime
                        .call_init(&mut store)
                        .map_err(|source| PluginHostError::GuestCall {
                            plugin_id: entry.id.clone(),
                            source,
                        })?;

                    RuntimeVariant::HelloWorld(runtime)
                }
                PluginWorld::ReloadWatcher => {
                    let mut linker = Linker::new(&self.engine);
                    p2::add_to_linker_sync(&mut linker).map_err(|source| {
                        PluginHostError::Instantiation {
                            plugin_id: entry.id.clone(),
                            source,
                        }
                    })?;
                    ReloadPlugin::add_to_linker::<HostState, HasSelf<HostState>>(
                        &mut linker,
                        host_state_lookup,
                    )
                    .map_err(|source| PluginHostError::Instantiation {
                        plugin_id: entry.id.clone(),
                        source,
                    })?;

                    let runtime = ReloadPlugin::instantiate(&mut store, &component, &linker)
                        .map_err(|source| PluginHostError::Instantiation {
                            plugin_id: entry.id.clone(),
                            source,
                        })?;

                    runtime
                        .call_init(&mut store)
                        .map_err(|source| PluginHostError::GuestCall {
                            plugin_id: entry.id.clone(),
                            source,
                        })?;

                    RuntimeVariant::ReloadWatcher(runtime)
                }
            };

        let init_message = "initialized".to_string();

        Ok(PluginHandle {
            id: entry.id.clone(),
            runtime,
            store,
            init_message,
            bridge: self.bridge.clone(),
            world,
        })
    }
}

enum RuntimeVariant {
    HelloWorld(HelloPlugin),
    ReloadWatcher(ReloadPlugin),
}

pub struct PluginHandle {
    id: String,
    runtime: RuntimeVariant,
    store: Store<HostState>,
    init_message: String,
    bridge: Arc<dyn HostBridge>,
    world: PluginWorld,
}

impl PluginHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn init_message(&self) -> &str {
        &self.init_message
    }

    pub fn refresh_opened_files(&mut self) -> Result<(), PluginHostError> {
        match &mut self.runtime {
            RuntimeVariant::HelloWorld(_) => Ok(()),
            RuntimeVariant::ReloadWatcher(runtime) => runtime
                .call_refresh_opened_files(&mut self.store)
                .map_err(|source| PluginHostError::GuestCall {
                    plugin_id: self.id.clone(),
                    source,
                }),
        }
    }

    pub fn watched_files_changed(
        &mut self,
        paths: Vec<CanonicalPathPayload>,
    ) -> Result<(), PluginHostError> {
        match &mut self.runtime {
            RuntimeVariant::HelloWorld(_) => Ok(()),
            RuntimeVariant::ReloadWatcher(runtime) => {
                let wit_paths: Vec<String> = paths
                    .iter()
                    .map(|payload| payload.canonical.clone())
                    .collect();
                runtime
                    .call_watched_files_changed(&mut self.store, &wit_paths)
                    .map_err(|source| PluginHostError::GuestCall {
                        plugin_id: self.id.clone(),
                        source,
                    })
            }
        }
    }

    pub fn shutdown(&mut self) -> Result<(), PluginHostError> {
        let result =
            match &mut self.runtime {
                RuntimeVariant::HelloWorld(runtime) => runtime
                    .call_shutdown(&mut self.store)
                    .map_err(|source| PluginHostError::GuestCall {
                        plugin_id: self.id.clone(),
                        source,
                    }),
                RuntimeVariant::ReloadWatcher(runtime) => runtime
                    .call_shutdown(&mut self.store)
                    .map_err(|source| PluginHostError::GuestCall {
                        plugin_id: self.id.clone(),
                        source,
                    }),
            };
        self.bridge.clear_watched_files(&self.id);
        result
    }

    pub fn world(&self) -> PluginWorld {
        self.world
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginHostError {
    #[error("failed to initialize Wasmtime engine: {0}")]
    Engine(#[source] wasmtime::Error),
    #[error("failed to load component for plugin '{plugin_id}': {source}")]
    ComponentLoad {
        plugin_id: String,
        #[source]
        source: wasmtime::Error,
    },
    #[error("failed to instantiate plugin '{plugin_id}': {source}")]
    Instantiation {
        plugin_id: String,
        #[source]
        source: Error,
    },
    #[error("guest call failed for plugin '{plugin_id}': {source}")]
    GuestCall {
        plugin_id: String,
        #[source]
        source: wasmtime::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_state_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<HostState>();
    }
}
