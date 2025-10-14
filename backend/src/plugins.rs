use moon::{Lazy, moonlight::CorId, sessions};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use plugin_host::{
    HostBridge, HostBridgeError, PluginHandle, PluginHost, PluginHostError, PluginWorld,
};
use shared::{CanonicalPathPayload, DownMsg, PluginsSection};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Default)]
struct BackendPluginBridge {
    opened_files: RwLock<Vec<CanonicalPathPayload>>,
    file_watchers: Mutex<HashMap<String, PluginWatcher>>,
    directory_watchers: Mutex<HashMap<String, PluginWatcher>>,
}

struct PluginWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<RecommendedWatcher>,
    task: JoinHandle<()>,
}

impl BackendPluginBridge {
    fn new() -> Self {
        Self::default()
    }

    fn set_opened_files(&self, files: &[CanonicalPathPayload]) {
        let mut guard = self.opened_files.write().expect("opened_files poisoned");
        *guard = files.to_vec();
    }

    fn normalized_paths(paths: Vec<CanonicalPathPayload>) -> Vec<CanonicalPathPayload> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for payload in paths {
            let canonical_str = if payload.canonical.trim().is_empty() {
                payload.display.trim()
            } else {
                payload.canonical.trim()
            };

            if canonical_str.is_empty() {
                continue;
            }
            let mut canonical_path = PathBuf::from(canonical_str);
            if !canonical_path.is_absolute() {
                if let Ok(abs) = std::fs::canonicalize(&canonical_path) {
                    canonical_path = abs;
                }
            }
            let canonical_text = canonical_path.to_string_lossy().to_string();
            if seen.insert(canonical_text.clone()) {
                result.push(CanonicalPathPayload {
                    canonical: canonical_text,
                    display: if payload.display.trim().is_empty() {
                        canonical_path.to_string_lossy().to_string()
                    } else {
                        payload.display.clone()
                    },
                });
            }
        }
        result
    }

    fn normalized_directories(directories: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for directory in directories {
            let trimmed = directory.trim();
            if trimmed.is_empty() {
                continue;
            }

            let candidate = PathBuf::from(trimmed);
            let absolute = if candidate.is_absolute() {
                candidate
            } else {
                match env::current_dir() {
                    Ok(root) => root.join(candidate),
                    Err(_) => PathBuf::from(trimmed),
                }
            };

            let canonical = std::fs::canonicalize(&absolute).unwrap_or(absolute);
            let canonical_text = canonical.to_string_lossy().to_string();
            if seen.insert(canonical_text.clone()) {
                result.push(canonical_text);
            }
        }

        result
    }
}

impl HostBridge for BackendPluginBridge {
    fn get_opened_files(&self, _plugin_id: &str) -> Vec<CanonicalPathPayload> {
        self.opened_files
            .read()
            .expect("opened_files poisoned")
            .clone()
    }

    fn register_watched_files(
        &self,
        plugin_id: &str,
        paths: Vec<CanonicalPathPayload>,
        debounce_ms: u32,
    ) -> Result<(), HostBridgeError> {
        self.clear_watched_files(plugin_id);

        let normalized = Self::normalized_paths(paths);
        if normalized.is_empty() {
            return Ok(());
        }

        let debounce_ms = debounce_ms.max(50);
        let debounce = Duration::from_millis(debounce_ms as u64);
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<CanonicalPathPayload>>();
        let plugin_label = plugin_id.to_string();
        let plugin_label_for_watcher = plugin_label.clone();
        let display_lookup: HashMap<String, String> = normalized
            .iter()
            .map(|payload| (payload.canonical.clone(), payload.display.clone()))
            .collect();

        let mut debouncer =
            new_debouncer(debounce, move |result: DebounceEventResult| match result {
                Ok(events) => {
                    let mut changed = Vec::new();
                    for event in events.into_iter() {
                        let path = event.path;
                        let canonical_buf = std::fs::canonicalize(&path).unwrap_or(path.clone());
                        if let Some(text) = canonical_buf.to_str() {
                            let canonical = text.to_string();
                            let display = display_lookup
                                .get(&canonical)
                                .cloned()
                                .unwrap_or_else(|| canonical.clone());
                            changed.push(CanonicalPathPayload { canonical, display });
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

        for payload in &normalized {
            let path = PathBuf::from(&payload.canonical);
            if let Err(err) = debouncer
                .watcher()
                .watch(&path, RecursiveMode::NonRecursive)
            {
                eprintln!(
                    "🔌 BACKEND: watcher failed for plugin '{}' on '{}': {}",
                    plugin_label,
                    path.display(),
                    err
                );
            }
        }

        let dispatch_plugin = plugin_id.to_string();
        let task = tokio::spawn(async move {
            while let Some(paths) = rx.recv().await {
                let mut unique = HashMap::new();
                for payload in paths {
                    unique.entry(payload.canonical.clone()).or_insert(payload);
                }
                if unique.is_empty() {
                    continue;
                }
                let file_paths: Vec<CanonicalPathPayload> = unique.into_values().collect();
                dispatch_watched_files_changed(&dispatch_plugin, file_paths);
            }
        });

        let mut watchers = self.file_watchers.lock().expect("file_watchers poisoned");
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
            .file_watchers
            .lock()
            .expect("file_watchers poisoned")
            .remove(plugin_id)
        {
            watcher.task.abort();
        }
    }

    fn register_watched_directories(
        &self,
        plugin_id: &str,
        directories: Vec<String>,
        debounce_ms: u32,
    ) -> Result<(), HostBridgeError> {
        self.clear_watched_directories(plugin_id);

        let normalized = Self::normalized_directories(directories);
        if normalized.is_empty() {
            return Ok(());
        }

        let debounce_ms = debounce_ms.max(50);
        let debounce = Duration::from_millis(debounce_ms as u64);
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<CanonicalPathPayload>>();
        let plugin_label = plugin_id.to_string();
        let plugin_label_for_watcher = plugin_label.clone();

        let mut debouncer =
            new_debouncer(debounce, move |result: DebounceEventResult| match result {
                Ok(events) => {
                    let mut discovered = Vec::new();
                    for event in events.into_iter() {
                        let path = event.path;
                        let canonical_buf =
                            std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                        let is_dir = std::fs::metadata(&canonical_buf)
                            .map(|meta| meta.is_dir())
                            .unwrap_or(false);
                        if is_dir {
                            continue;
                        }

                        let canonical = canonical_buf.to_string_lossy().to_string();
                        let display = path.to_string_lossy().to_string();
                        discovered.push(CanonicalPathPayload { canonical, display });
                    }
                    if !discovered.is_empty() {
                        let _ = tx.send(discovered);
                    }
                }
                Err(err) => {
                    eprintln!(
                        "🔌 BACKEND: directory watcher error for plugin '{}': {}",
                        plugin_label_for_watcher, err
                    );
                }
            })
            .map_err(|err| HostBridgeError::Message(format!("failed to create watcher: {err}")))?;

        for directory in &normalized {
            let path = PathBuf::from(directory);
            if let Err(err) = debouncer.watcher().watch(&path, RecursiveMode::Recursive) {
                eprintln!(
                    "🔌 BACKEND: directory watcher failed for plugin '{}' on '{}': {}",
                    plugin_label,
                    path.display(),
                    err
                );
            }
        }

        let dispatch_plugin = plugin_id.to_string();
        let task = tokio::spawn(async move {
            while let Some(paths) = rx.recv().await {
                let mut unique = HashMap::new();
                for payload in paths {
                    unique.entry(payload.canonical.clone()).or_insert(payload);
                }
                if unique.is_empty() {
                    continue;
                }
                let discovered: Vec<CanonicalPathPayload> = unique.into_values().collect();
                dispatch_paths_discovered(&dispatch_plugin, discovered);
            }
        });

        let mut watchers = self
            .directory_watchers
            .lock()
            .expect("directory_watchers poisoned");
        watchers.insert(
            plugin_id.to_string(),
            PluginWatcher {
                _debouncer: debouncer,
                task,
            },
        );

        Ok(())
    }

    fn clear_watched_directories(&self, plugin_id: &str) {
        if let Some(watcher) = self
            .directory_watchers
            .lock()
            .expect("directory_watchers poisoned")
            .remove(plugin_id)
        {
            watcher.task.abort();
        }
    }

    fn reload_waveform_files(
        &self,
        plugin_id: &str,
        paths: Vec<CanonicalPathPayload>,
    ) -> Result<(), HostBridgeError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut unique = HashMap::new();
        for payload in paths {
            unique.entry(payload.canonical.clone()).or_insert(payload);
        }
        if unique.is_empty() {
            return Ok(());
        }
        let file_paths: Vec<CanonicalPathPayload> = unique.into_values().collect();
        let plugin_label = plugin_id.to_string();

        tokio::spawn(async move {
            let msg = DownMsg::ReloadWaveformFiles {
                file_paths: file_paths.clone(),
            };
            sessions::broadcast_down_msg(&msg, CorId::new()).await;
            println!(
                "🔌 BACKEND: plugin '{}' requested reload for {} path(s): {:?}",
                plugin_label,
                file_paths.len(),
                file_paths
                    .iter()
                    .map(|payload| payload.display.as_str())
                    .collect::<Vec<_>>()
            );
        });

        Ok(())
    }

    fn open_waveform_files(
        &self,
        plugin_id: &str,
        paths: Vec<CanonicalPathPayload>,
    ) -> Result<(), HostBridgeError> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut unique = HashMap::new();
        for payload in paths {
            unique.entry(payload.canonical.clone()).or_insert(payload);
        }

        if unique.is_empty() {
            return Ok(());
        }

        let opened_set: HashSet<String> = self
            .opened_files
            .read()
            .expect("opened_files poisoned")
            .iter()
            .map(|payload| payload.canonical.clone())
            .collect();

        let mut new_files = Vec::new();
        for (canonical, payload) in unique.into_iter() {
            if !opened_set.contains(&canonical) {
                new_files.push(payload);
            }
        }

        if new_files.is_empty() {
            return Ok(());
        }

        let plugin_label = plugin_id.to_string();
        tokio::spawn(async move {
            let msg = DownMsg::OpenWaveformFiles {
                file_paths: new_files.clone(),
            };
            sessions::broadcast_down_msg(&msg, CorId::new()).await;
            println!(
                "🔌 BACKEND: plugin '{}' discovered {} new path(s): {:?}",
                plugin_label,
                new_files.len(),
                new_files
                    .iter()
                    .map(|payload| payload.display.as_str())
                    .collect::<Vec<_>>()
            );
        });

        Ok(())
    }

    fn log_info(&self, plugin_id: &str, message: &str) {
        println!("🔌 PLUGIN[{}]: {}", plugin_id, message);
    }

    fn log_error(&self, plugin_id: &str, message: &str) {
        eprintln!("🔌 PLUGIN[{}]: {}", plugin_id, message);
    }
}

fn dispatch_watched_files_changed(plugin_id: &str, paths: Vec<CanonicalPathPayload>) {
    if paths.is_empty() {
        return;
    }
    match PLUGIN_MANAGER.lock() {
        Ok(mut manager) => {
            manager.handle_watched_files_changed(plugin_id, paths);
        }
        Err(err) => {
            eprintln!(
                "🔌 BACKEND: failed to dispatch watcher event for plugin '{}': {}",
                plugin_id, err
            );
        }
    }
}

fn dispatch_paths_discovered(plugin_id: &str, paths: Vec<CanonicalPathPayload>) {
    if paths.is_empty() {
        return;
    }

    match PLUGIN_MANAGER.lock() {
        Ok(mut manager) => {
            manager.handle_paths_discovered(plugin_id, paths);
        }
        Err(err) => {
            eprintln!(
                "🔌 BACKEND: failed to dispatch discovery event for plugin '{}': {}",
                plugin_id, err
            );
        }
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
    current_opened_files: Vec<CanonicalPathPayload>,
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
            current_opened_files: Vec::new(),
        })
    }

    pub fn reload(
        &mut self,
        section: &PluginsSection,
        opened_files: &[CanonicalPathPayload],
    ) -> bool {
        let config_changed = self.current_config.as_ref() != Some(section);
        let normalized_opened = BackendPluginBridge::normalized_paths(opened_files.to_vec());
        let opened_changed = self.current_opened_files != normalized_opened;

        if !config_changed && !opened_changed {
            return false;
        }

        self.bridge.set_opened_files(&normalized_opened);
        self.current_opened_files = normalized_opened.clone();

        if !config_changed {
            let mut refresh_ok = false;
            for handle in self.handles.values_mut() {
                match handle.refresh_opened_files() {
                    Ok(_) => {
                        if let Some(status) = self.statuses.get_mut(handle.id()) {
                            status.state = PluginRuntimeState::Ready;
                            status.last_message = Some("refresh_opened_files() ok".to_string());
                        }
                        refresh_ok = true;
                    }
                    Err(err) => {
                        if let Some(status) = self.statuses.get_mut(handle.id()) {
                            status.state = PluginRuntimeState::Error;
                            status.last_message = Some(err.to_string());
                        }
                    }
                }
            }
            return refresh_ok || opened_changed;
        }

        self.shutdown_all();
        self.statuses.clear();
        self.current_config = Some(section.clone());

        for entry in &section.entries {
            if !entry.enabled {
                self.bridge.clear_watched_files(&entry.id);
                self.bridge.clear_watched_directories(&entry.id);
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
                    let refresh_result = handle.refresh_opened_files().ok();
                    self.statuses.insert(
                        entry.id.clone(),
                        PluginStatus {
                            id: entry.id.clone(),
                            state: PluginRuntimeState::Ready,
                            init_message: Some(init_message),
                            last_message: refresh_result
                                .map(|_| "refresh_opened_files() ok".to_string()),
                        },
                    );
                    self.handles.insert(entry.id.clone(), handle);
                }
                Err(err) => {
                    self.bridge.clear_watched_files(&entry.id);
                    self.bridge.clear_watched_directories(&entry.id);
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

    fn handle_watched_files_changed(&mut self, plugin_id: &str, paths: Vec<CanonicalPathPayload>) {
        if paths.is_empty() {
            return;
        }

        match self.handles.get_mut(plugin_id) {
            Some(handle) => {
                if !matches!(handle.world(), PluginWorld::ReloadWatcher) {
                    self.bridge.log_info(
                        plugin_id,
                        "watched_files_changed ignored for plugin without reload support",
                    );
                    return;
                }

                match handle.watched_files_changed(paths.clone()) {
                    Ok(_) => {
                        if let Some(status) = self.statuses.get_mut(plugin_id) {
                            status.last_message = Some(format!(
                                "watched_files_changed() ok for {} path(s)",
                                paths.len()
                            ));
                        }
                    }
                    Err(err) => {
                        self.bridge.log_error(
                            plugin_id,
                            &format!("watched_files_changed failed: {}", err),
                        );
                        if let Some(status) = self.statuses.get_mut(plugin_id) {
                            status.state = PluginRuntimeState::Error;
                            status.last_message = Some(err.to_string());
                        }
                    }
                }
            }
            None => {
                self.bridge.log_error(
                    plugin_id,
                    "received watcher event but plugin handle is not loaded",
                );
            }
        }
    }

    fn handle_paths_discovered(&mut self, plugin_id: &str, paths: Vec<CanonicalPathPayload>) {
        if paths.is_empty() {
            return;
        }

        match self.handles.get_mut(plugin_id) {
            Some(handle) => {
                if !matches!(handle.world(), PluginWorld::FilesDiscovery) {
                    self.bridge.log_info(
                        plugin_id,
                        "paths_discovered ignored for plugin without discovery support",
                    );
                    return;
                }

                match handle.paths_discovered(paths.clone()) {
                    Ok(_) => {
                        if let Some(status) = self.statuses.get_mut(plugin_id) {
                            status.last_message =
                                Some(format!("paths_discovered() ok for {} path(s)", paths.len()));
                        }
                    }
                    Err(err) => {
                        self.bridge
                            .log_error(plugin_id, &format!("paths_discovered failed: {}", err));
                        if let Some(status) = self.statuses.get_mut(plugin_id) {
                            status.state = PluginRuntimeState::Error;
                            status.last_message = Some(err.to_string());
                        }
                    }
                }
            }
            None => {
                self.bridge.log_error(
                    plugin_id,
                    "received discovery event but plugin handle is not loaded",
                );
            }
        }
    }

    fn shutdown_all(&mut self) {
        for handle in self.handles.values_mut() {
            let _ = handle.shutdown();
            self.bridge.clear_watched_files(handle.id());
            self.bridge.clear_watched_directories(handle.id());
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

pub fn reload_plugins(
    section: &PluginsSection,
    opened_files: &[CanonicalPathPayload],
) -> PluginReloadOutcome {
    let mut manager = PLUGIN_MANAGER
        .lock()
        .expect("Plugin manager mutex poisoned");
    let reloaded = manager.reload(section, opened_files);
    let statuses = manager.statuses();
    PluginReloadOutcome { statuses, reloaded }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_paths_deduplicates_canonical_entries() {
        let input = vec![
            CanonicalPathPayload {
                canonical: "/tmp/sample.vcd".to_string(),
                display: "/tmp/sample.vcd".to_string(),
            },
            CanonicalPathPayload {
                canonical: "/tmp/sample.vcd".to_string(),
                display: "sample.vcd".to_string(),
            },
        ];

        let normalized = BackendPluginBridge::normalized_paths(input);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].canonical, "/tmp/sample.vcd");
        assert_eq!(normalized[0].display, "/tmp/sample.vcd");
    }
}
