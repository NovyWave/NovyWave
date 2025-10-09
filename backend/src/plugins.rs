use moon::Lazy;
use plugin_host::{PluginHandle, PluginHost, PluginHostError};
use shared::PluginsSection;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub enum PluginRuntimeState {
    Disabled,
    Ready,
    Error,
}

#[derive(Clone, Debug)]
pub struct PluginStatus {
    pub id: String,
    pub state: PluginRuntimeState,
    pub init_message: Option<String>,
    pub last_message: Option<String>,
}

pub struct PluginManager {
    host: PluginHost,
    handles: HashMap<String, PluginHandle>,
    statuses: HashMap<String, PluginStatus>,
    current_config: Option<PluginsSection>,
}

impl PluginManager {
    pub fn new() -> Result<Self, PluginHostError> {
        Ok(Self {
            host: PluginHost::new()?,
            handles: HashMap::new(),
            statuses: HashMap::new(),
            current_config: None,
        })
    }

    pub fn reload(&mut self, section: &PluginsSection) -> bool {
        if self.current_config.as_ref() == Some(section) {
            return false;
        }

        self.shutdown_all();
        self.statuses.clear();
        self.current_config = Some(section.clone());

        for entry in &section.entries {
            if !entry.enabled {
                self.statuses.insert(
                    entry.id.clone(),
                    PluginStatus {
                        id: entry.id.clone(),
                        state: PluginRuntimeState::Disabled,
                        init_message: None,
                        last_message: None,
                    },
                );
                continue;
            }

            match self.host.load(entry) {
                Ok(mut handle) => {
                    let init_message = handle.init_message().to_string();
                    let greet_result = handle.greet().ok();
                    self.statuses.insert(
                        entry.id.clone(),
                        PluginStatus {
                            id: entry.id.clone(),
                            state: PluginRuntimeState::Ready,
                            init_message: Some(init_message),
                            last_message: greet_result.map(|_| "greet() ok".to_string()),
                        },
                    );
                    self.handles.insert(entry.id.clone(), handle);
                }
                Err(err) => {
                    self.statuses.insert(
                        entry.id.clone(),
                        PluginStatus {
                            id: entry.id.clone(),
                            state: PluginRuntimeState::Error,
                            init_message: None,
                            last_message: Some(err.to_string()),
                        },
                    );
                }
            }
        }

        true
    }

    pub fn statuses(&self) -> Vec<PluginStatus> {
        self.statuses.values().cloned().collect()
    }

    fn shutdown_all(&mut self) {
        for handle in self.handles.values_mut() {
            let _ = handle.shutdown();
        }
        self.handles.clear();
    }
}

static PLUGIN_MANAGER: Lazy<Mutex<PluginManager>> = Lazy::new(|| {
    let manager = PluginManager::new().expect("Failed to initialize plugin host runtime");
    Mutex::new(manager)
});

pub struct PluginReloadOutcome {
    pub statuses: Vec<PluginStatus>,
    pub reloaded: bool,
}

pub fn reload_plugins(section: &PluginsSection) -> PluginReloadOutcome {
    let mut manager = PLUGIN_MANAGER
        .lock()
        .expect("Plugin manager mutex poisoned");
    let reloaded = manager.reload(section);
    let statuses = manager.statuses();
    PluginReloadOutcome { statuses, reloaded }
}
