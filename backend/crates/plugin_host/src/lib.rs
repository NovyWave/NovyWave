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
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, p2};

struct HostState {
    table: ResourceTable,
    wasi: WasiCtx,
}

impl HostState {
    fn new() -> Self {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .inherit_args()
            .build();
        Self {
            table: ResourceTable::new(),
            wasi,
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

pub struct PluginHost {
    engine: Engine,
}

impl PluginHost {
    pub fn new() -> Result<Self, PluginHostError> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(false);
        config.wasm_multi_memory(true);
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);

        let engine = Engine::new(&config).map_err(PluginHostError::Engine)?;
        Ok(Self { engine })
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

        let mut store = Store::new(&self.engine, HostState::new());

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
        })
    }
}

pub struct PluginHandle {
    id: String,
    runtime: Runtime,
    store: Store<HostState>,
    init_message: String,
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
        self.runtime
            .call_shutdown(&mut self.store)
            .map_err(|source| PluginHostError::GuestCall {
                plugin_id: self.id.clone(),
                source,
            })
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
