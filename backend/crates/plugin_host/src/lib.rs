mod bindings {
    wasmtime::component::bindgen!({
        path: "../../../plugins/wit",
        world: "runtime",
    });
}

use anyhow::Error;
use bindings::Runtime;
use shared::PluginConfigEntry;
use std::path::Path;
use std::sync::Arc;
use wasmtime::component::{Component, HasSelf, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2};

/// Bridge trait implemented by the host runtime to service plugin requests.
pub trait HostBridge: Send + Sync + 'static {
    fn get_opened_files(&self, plugin_id: &str) -> Vec<String>;
    fn register_watched_files(
        &self,
        plugin_id: &str,
        paths: Vec<String>,
        debounce_ms: u32,
    ) -> Result<(), HostBridgeError>;
    fn clear_watched_files(&self, plugin_id: &str);
    fn log_info(&self, plugin_id: &str, message: &str);
    fn log_error(&self, plugin_id: &str, message: &str);
}

/// Minimal no-op implementation used by tooling that does not provide a runtime bridge.
#[derive(Clone, Default)]
pub struct NullBridge;

impl HostBridge for NullBridge {
    fn get_opened_files(&self, _plugin_id: &str) -> Vec<String> {
        Vec::new()
    }

    fn register_watched_files(
        &self,
        _plugin_id: &str,
        _paths: Vec<String>,
        _debounce_ms: u32,
    ) -> Result<(), HostBridgeError> {
        Ok(())
    }

    fn clear_watched_files(&self, _plugin_id: &str) {}

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

impl bindings::host::Host for HostState {
    fn get_opened_files(&mut self) -> Vec<String> {
        self.bridge.get_opened_files(&self.plugin_id)
    }

    fn register_watched_files(&mut self, paths: Vec<String>, debounce_ms: u32) -> () {
        if let Err(err) = self
            .bridge
            .register_watched_files(&self.plugin_id, paths, debounce_ms)
        {
            self.bridge.log_error(&self.plugin_id, &err.to_string());
        }
    }

    fn clear_watched_files(&mut self) -> () {
        self.bridge.clear_watched_files(&self.plugin_id);
    }

    fn log_info(&mut self, message: String) -> () {
        self.bridge.log_info(&self.plugin_id, &message);
    }

    fn log_error(&mut self, message: String) -> () {
        self.bridge.log_error(&self.plugin_id, &message);
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

        let mut linker = Linker::new(&self.engine);
        p2::add_to_linker_sync(&mut linker).map_err(|source| PluginHostError::Instantiation {
            plugin_id: entry.id.clone(),
            source,
        })?;
        bindings::host::add_to_linker::<HostState, HasSelf<HostState>>(
            &mut linker,
            host_state_lookup,
        )
        .map_err(|source| PluginHostError::Instantiation {
            plugin_id: entry.id.clone(),
            source,
        })?;

        let host_state = HostState::new(entry.id.clone(), self.bridge.clone());
        let mut store = Store::new(&self.engine, host_state);

        let runtime = Runtime::instantiate(&mut store, &component, &linker).map_err(|source| {
            PluginHostError::Instantiation {
                plugin_id: entry.id.clone(),
                source,
            }
        })?;

        runtime
            .call_init(&mut store)
            .map_err(|source| PluginHostError::GuestCall {
                plugin_id: entry.id.clone(),
                source,
            })?;

        let init_message = "initialized".to_string();

        Ok(PluginHandle {
            id: entry.id.clone(),
            runtime,
            store,
            init_message,
            bridge: self.bridge.clone(),
        })
    }
}

pub struct PluginHandle {
    id: String,
    runtime: Runtime,
    store: Store<HostState>,
    init_message: String,
    bridge: Arc<dyn HostBridge>,
}

impl PluginHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn init_message(&self) -> &str {
        &self.init_message
    }

    pub fn greet(&mut self) -> Result<(), PluginHostError> {
        self.runtime
            .call_greet(&mut self.store)
            .map_err(|source| PluginHostError::GuestCall {
                plugin_id: self.id.clone(),
                source,
            })
    }

    pub fn shutdown(&mut self) -> Result<(), PluginHostError> {
        let result = self
            .runtime
            .call_shutdown(&mut self.store)
            .map_err(|source| PluginHostError::GuestCall {
                plugin_id: self.id.clone(),
                source,
            });
        self.bridge.clear_watched_files(&self.id);
        result
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
