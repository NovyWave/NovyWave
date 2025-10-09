use moon::{Lazy, moonlight::CorId, sessions};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use plugin_host::{HostBridge, HostBridgeError, PluginHandle, PluginHost, PluginHostError};
use shared::{DownMsg, PluginsSection};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Default)]
struct BackendPluginBridge {
    opened_files: RwLock<Vec<String>>,
    watchers: Mutex<HashMap<String, PluginWatcher>>,
}

struct PluginWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<RecommendedWatcher>,
    task: JoinHandle<()>,
}

impl BackendPluginBridge {
    fn new() -> Self {
        Self::default()
    }

    fn set_opened_files(&self, files: &[String]) {
        let mut guard = self.opened_files.write().expect("opened_files poisoned");
        *guard = files.to_vec();
    }

    fn normalized_paths(paths: Vec<String>) -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for raw in paths {
            if raw.trim().is_empty() {
                continue;
            }
            let path = PathBuf::from(&raw);
            let canonical = if path.is_absolute() {
                path
            } else {
                match std::fs::canonicalize(&path) {
                    Ok(abs) => abs,
                    Err(_) => path,
                }
            };
            if seen.insert(canonical.clone()) {
                result.push(canonical);
            }
        }
        result
    }
}

impl HostBridge for BackendPluginBridge {
    fn get_opened_files(&self, _plugin_id: &str) -> Vec<String> {
        self.opened_files
            .read()
            .expect("opened_files poisoned")
            .clone()
    }

    fn register_watched_files(
        &self,
        plugin_id: &str,
        paths: Vec<String>,
        debounce_ms: u32,
    ) -> Result<(), HostBridgeError> {
        self.clear_watched_files(plugin_id);

        let normalized = Self::normalized_paths(paths);
        if normalized.is_empty() {
            return Ok(());
        }

        let debounce_ms = debounce_ms.max(50);
        let debounce = Duration::from_millis(debounce_ms as u64);
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<String>>();
        let plugin_label = plugin_id.to_string();
        let plugin_label_for_watcher = plugin_label.clone();

        let mut debouncer =
            new_debouncer(debounce, move |result: DebounceEventResult| match result {
                Ok(events) => {
                    let mut changed = Vec::new();
                    for event in events.into_iter() {
                        let path = event.path;
                        if let Some(text) = path.to_str() {
                            changed.push(text.to_string());
                        }
                    }
                    if !changed.is_empty() {
                        let _ = tx.send(changed);
                    }
                }
                Err(err) => {
                    eprintln!(
                        "🔌 BACKEND: watcher error for plugin '{}': {}",
                        plugin_label_for_watcher, err
                    );
                }
            })
            .map_err(|err| HostBridgeError::Message(format!("failed to create watcher: {err}")))?;

        for path in &normalized {
            if let Err(err) = debouncer.watcher().watch(path, RecursiveMode::NonRecursive) {
                eprintln!(
                    "🔌 BACKEND: watcher failed for plugin '{}' on '{}': {}",
                    plugin_label,
                    path.display(),
                    err
                );
            }
        }

        let broadcast_plugin = plugin_id.to_string();
        let task = tokio::spawn(async move {
            while let Some(paths) = rx.recv().await {
                let unique: HashSet<String> = paths.into_iter().collect();
                if unique.is_empty() {
                    continue;
                }
                let file_paths: Vec<String> = unique.into_iter().collect();
                let msg = DownMsg::ReloadWaveformFiles {
                    file_paths: file_paths.clone(),
                };
                sessions::broadcast_down_msg(&msg, CorId::new()).await;
                println!(
                    "🔌 BACKEND: plugin '{}' requested reload for {} path(s)",
                    broadcast_plugin,
                    file_paths.len()
                );
            }
        });

        let mut watchers = self.watchers.lock().expect("watchers poisoned");
        watchers.insert(
            plugin_id.to_string(),
            PluginWatcher {
                _debouncer: debouncer,
                task,
            },
        );

        Ok(())
    }

    fn clear_watched_files(&self, plugin_id: &str) {
        if let Some(watcher) = self
            .watchers
            .lock()
            .expect("watchers poisoned")
            .remove(plugin_id)
        {
            watcher.task.abort();
        }
    }

    fn log_info(&self, plugin_id: &str, message: &str) {
        println!("🔌 PLUGIN[{}]: {}", plugin_id, message);
    }

    fn log_error(&self, plugin_id: &str, message: &str) {
        eprintln!("🔌 PLUGIN[{}]: {}", plugin_id, message);
    }
}

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
    bridge: Arc<BackendPluginBridge>,
    handles: HashMap<String, PluginHandle>,
    statuses: HashMap<String, PluginStatus>,
    current_config: Option<PluginsSection>,
}

impl PluginManager {
    pub fn new() -> Result<Self, PluginHostError> {
        let bridge = Arc::new(BackendPluginBridge::new());
        Ok(Self {
            host: PluginHost::new(bridge.clone())?,
            bridge,
            handles: HashMap::new(),
            statuses: HashMap::new(),
            current_config: None,
        })
    }

    pub fn reload(&mut self, section: &PluginsSection, opened_files: &[String]) -> bool {
        if self.current_config.as_ref() == Some(section) {
            return false;
        }

        self.shutdown_all();
        self.statuses.clear();
        self.current_config = Some(section.clone());
        self.bridge.set_opened_files(opened_files);

        for entry in &section.entries {
            if !entry.enabled {
                self.bridge.clear_watched_files(&entry.id);
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
                    self.bridge.clear_watched_files(&entry.id);
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
            self.bridge.clear_watched_files(handle.id());
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

pub fn reload_plugins(section: &PluginsSection, opened_files: &[String]) -> PluginReloadOutcome {
    let mut manager = PLUGIN_MANAGER
        .lock()
        .expect("Plugin manager mutex poisoned");
    let reloaded = manager.reload(section, opened_files);
    let statuses = manager.statuses();
    PluginReloadOutcome { statuses, reloaded }
}
