use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{Match, WalkBuilder};
use moon::{Lazy, moonlight::CorId, sessions};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use plugin_host::{
    HostBridge, HostBridgeError, PluginHandle, PluginHost, PluginHostError, PluginWorld,
};
use shared::{CanonicalPathPayload, DownMsg, PluginConfigEntry, PluginsSection};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

struct BackendPluginBridge {
    opened_files: RwLock<Vec<CanonicalPathPayload>>,
    file_watchers: Mutex<HashMap<String, PluginWatcher>>,
    directory_watchers: Mutex<HashMap<String, PluginWatcher>>,
    discovery_configs: RwLock<HashMap<String, DiscoveryHostConfig>>,
    pending_initial_discoveries: Mutex<HashMap<String, Vec<CanonicalPathPayload>>>,
}

struct PluginWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<RecommendedWatcher>,
    task: JoinHandle<()>,
}

impl Default for BackendPluginBridge {
    fn default() -> Self {
        Self {
            opened_files: RwLock::new(Vec::new()),
            file_watchers: Mutex::new(HashMap::new()),
            directory_watchers: Mutex::new(HashMap::new()),
            discovery_configs: RwLock::new(HashMap::new()),
            pending_initial_discoveries: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Clone, Debug)]
struct DiscoveryHostConfig {
    base_dir: PathBuf,
    patterns: Vec<String>,
    allow_extensions: HashSet<String>,
}

impl DiscoveryHostConfig {
    const DEFAULT_EXTENSIONS: &'static [&'static str] = &["fst", "vcd"];

    fn from_entry(entry: &PluginConfigEntry) -> Option<Self> {
        if entry.id != "novywave.files_discovery" {
            return None;
        }

        let workspace_root = crate::workspace_context().root();
        let base_dir = entry
            .config
            .get("base_dir")
            .and_then(|value| value.as_str())
            .map(|path_str| {
                let candidate = PathBuf::from(path_str.trim());
                if candidate.is_absolute() {
                    candidate
                } else {
                    workspace_root.join(candidate)
                }
            })
            .unwrap_or_else(|| workspace_root.clone());
        let canonical_base_dir =
            std::fs::canonicalize(&base_dir).unwrap_or_else(|_| base_dir.clone());

        let patterns = entry
            .config
            .get("patterns")
            .and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(|pattern| pattern.trim().to_string())
                    .filter(|pattern| !pattern.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let allow_extensions = entry
            .config
            .get("allow_extensions")
            .and_then(|value| value.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|value| value.as_str())
                    .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
                    .filter(|ext| !ext.is_empty())
                    .collect::<HashSet<_>>()
            })
            .filter(|set| !set.is_empty())
            .unwrap_or_else(|| {
                Self::DEFAULT_EXTENSIONS
                    .iter()
                    .map(|ext| ext.to_string())
                    .collect()
            });

        Some(Self {
            base_dir: canonical_base_dir,
            patterns,
            allow_extensions,
        })
    }

    fn build_matcher(&self) -> Result<Gitignore, String> {
        // Guard against panics inside `ignore` on odd inputs/paths during dev.
        let base_dir = self.base_dir.clone();
        let patterns = self.patterns.clone();
        std::panic::catch_unwind(move || {
            let mut builder = GitignoreBuilder::new(&base_dir);
            for pattern in &patterns {
                if let Err(err) = builder.add_line(None, pattern) {
                    return Err(err.to_string());
                }
            }
            builder
                .build()
                .map_err(|err| format!("failed to build matcher: {err}"))
        })
        .map_err(|_| "panic while building discovery matcher".to_string())?
    }

    fn extension_allowed(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| self.allow_extensions.contains(&ext.to_ascii_lowercase()))
            .unwrap_or(false)
    }

    fn matches(&self, matcher: &Gitignore, path: &Path) -> bool {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        // Guard against panics inside the ignore crate on odd inputs/paths.
        match std::panic::catch_unwind({
            let absolute = absolute.clone();
            let matcher = matcher.clone();
            move || {
                let m = matcher.matched_path_or_any_parents(&absolute, absolute.is_dir());
                matches!(m, Match::Whitelist(_) | Match::Ignore(_))
            }
        }) {
            Ok(is_match) => is_match,
            Err(_) => {
                eprintln!(
                    "🔌 BACKEND: ignore::matched panic for path '{}'; skipping",
                    absolute.display()
                );
                false
            }
        }
    }
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
        let workspace_root = crate::workspace_context().root();
        for payload in paths {
            let trimmed = payload.canonical.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut canonical_path = PathBuf::from(trimmed);
            if !canonical_path.is_absolute() {
                canonical_path = workspace_root.join(&canonical_path);
            }
            if let Ok(abs) = std::fs::canonicalize(&canonical_path) {
                canonical_path = abs;
            }
            let canonical_text = canonical_path.to_string_lossy().to_string();
            if seen.insert(canonical_text.clone()) {
                result.push(CanonicalPathPayload::new(canonical_text));
            }
        }
        result
    }

    fn normalized_directories(directories: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        let workspace_root = crate::workspace_context().root();

        for directory in directories {
            let trimmed = directory.trim();
            if trimmed.is_empty() {
                continue;
            }

            let candidate = PathBuf::from(trimmed);
            let absolute = if candidate.is_absolute() {
                candidate
            } else {
                workspace_root.join(&candidate)
            };

            if let Some(normalized) = Self::canonical_watch_directory(&absolute) {
                let text = normalized.to_string_lossy().to_string();
                if seen.insert(text.clone()) {
                    result.push(text);
                }
            }
        }

        result
    }

    fn canonical_watch_directory(path: &Path) -> Option<PathBuf> {
        let mut candidate = path.to_path_buf();
        let mut popped = false;

        loop {
            match std::fs::canonicalize(&candidate) {
                Ok(canonical) => {
                    if canonical.as_path() == Path::new("/") && popped && path != Path::new("/") {
                        eprintln!(
                            "🔌 BACKEND: skipping watch registration for '{}' to avoid monitoring filesystem root",
                            path.display()
                        );
                        return None;
                    }
                    return Some(canonical);
                }
                Err(_) => {
                    if !candidate.pop() {
                        break;
                    }
                    popped = true;
                }
            }
        }

        if path == Path::new("/") {
            Some(PathBuf::from("/"))
        } else {
            eprintln!(
                "🔌 BACKEND: unable to resolve watch directory '{}'; skipping",
                path.display()
            );
            None
        }
    }

    fn queue_initial_discovery(&self, plugin_id: &str, payloads: Vec<CanonicalPathPayload>) {
        if payloads.is_empty() {
            return;
        }

        let mut guard = self
            .pending_initial_discoveries
            .lock()
            .expect("pending_initial_discoveries poisoned");
        let entry = guard.entry(plugin_id.to_string()).or_insert_with(Vec::new);

        let mut seen: HashSet<String> = entry
            .iter()
            .map(|payload| payload.canonical.clone())
            .collect();
        for payload in payloads {
            if seen.insert(payload.canonical.clone()) {
                entry.push(payload);
            }
        }
    }

    fn take_initial_discoveries(&self) -> HashMap<String, Vec<CanonicalPathPayload>> {
        self.pending_initial_discoveries
            .lock()
            .expect("pending_initial_discoveries poisoned")
            .drain()
            .collect()
    }

    fn dispatch_open_waveform_files(&self, plugin_id: &str, paths: Vec<CanonicalPathPayload>) {
        if paths.is_empty() {
            return;
        }

        let mut unique = HashMap::new();
        for payload in paths {
            unique.entry(payload.canonical.clone()).or_insert(payload);
        }
        if unique.is_empty() {
            return;
        }

        let opened_set: HashSet<String> = self
            .opened_files
            .read()
            .expect("opened_files poisoned")
            .iter()
            .map(|payload| payload.canonical.clone())
            .collect();

        let mut new_files = Vec::new();
        for (_, payload) in unique {
            if !opened_set.contains(&payload.canonical) {
                new_files.push(payload);
            }
        }

        if new_files.is_empty() {
            return;
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
                    .map(|payload| payload.display())
                    .collect::<Vec<_>>()
            );
        });
    }

    fn update_discovery_config(&self, plugin_id: &str, config: Option<DiscoveryHostConfig>) {
        let mut guard = self
            .discovery_configs
            .write()
            .expect("discovery_configs poisoned");
        if let Some(config) = config {
            guard.insert(plugin_id.to_string(), config);
        } else {
            guard.remove(plugin_id);
        }
    }

    fn discovery_config(&self, plugin_id: &str) -> Option<DiscoveryHostConfig> {
        self.discovery_configs
            .read()
            .ok()
            .and_then(|guard| guard.get(plugin_id).cloned())
    }

    fn perform_initial_discovery(&self, plugin_id: &str, directories: &[String]) {
        let Some(config) = self.discovery_config(plugin_id) else {
            return;
        };
        if config.patterns.is_empty() {
            return;
        }

        let matcher = match config.build_matcher() {
            Ok(matcher) => matcher,
            Err(err) => {
                eprintln!(
                    "🔌 BACKEND: failed to compile discovery patterns for '{}': {}",
                    plugin_id, err
                );
                return;
            }
        };

        let opened: HashSet<String> = self
            .opened_files
            .read()
            .map(|guard| {
                guard
                    .iter()
                    .map(|payload| payload.canonical.clone())
                    .collect()
            })
            .unwrap_or_default();

        let mut discovered = HashMap::new();

        for directory in directories {
            let dir_path = PathBuf::from(directory);
            if !dir_path.exists() {
                continue;
            }

            let mut walker = WalkBuilder::new(&dir_path);
            walker
                .standard_filters(false)
                .git_ignore(false)
                .git_exclude(false)
                .git_global(false)
                .follow_links(false)
                .threads(1);

            for entry in walker.build() {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        eprintln!(
                            "🔌 BACKEND: discovery walker error under '{}': {}",
                            dir_path.display(),
                            err
                        );
                        continue;
                    }
                };

                if !entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
                {
                    continue;
                }

                let path = entry.path();
                if !config.extension_allowed(path) {
                    continue;
                }
                if !config.matches(&matcher, path) {
                    continue;
                }

                let canonical = match std::fs::canonicalize(path) {
                    Ok(canonical) => canonical,
                    Err(_) => continue,
                };
                let canonical_text = canonical.to_string_lossy().to_string();
                if opened.contains(&canonical_text) {
                    continue;
                }

                let payload = CanonicalPathPayload::new(canonical_text.clone());
                discovered.entry(canonical_text).or_insert(payload);
            }
        }

        if discovered.is_empty() {
            return;
        }

        let payloads: Vec<CanonicalPathPayload> = discovered.into_values().collect();
        self.queue_initial_discovery(plugin_id, payloads);
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

        let mut debouncer =
            new_debouncer(debounce, move |result: DebounceEventResult| match result {
                Ok(events) => {
                    let mut changed = Vec::new();
                    for event in events.into_iter() {
                        let path = event.path;
                        let canonical_buf = std::fs::canonicalize(&path).unwrap_or(path.clone());
                        if let Some(text) = canonical_buf.to_str() {
                            let canonical = text.to_string();
                            changed.push(CanonicalPathPayload::new(canonical));
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

        // Be resilient to poisoned mutex during dev hot-reloads
        let mut watchers = match self.file_watchers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("🔌 BACKEND: file_watchers mutex poisoned; recovering");
                let mut inner = poisoned.into_inner();
                inner.clear();
                inner
            }
        };
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
        if let Some(watcher) = match self.file_watchers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("🔌 BACKEND: file_watchers mutex poisoned on clear; recovering");
                poisoned.into_inner()
            }
        }
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

        eprintln!(
            "🔌 BACKEND: plugin '{}' requested directory watches: {:?}",
            plugin_id, directories
        );

        let normalized = Self::normalized_directories(directories);
        eprintln!(
            "🔌 BACKEND: plugin '{}' normalized directory watches: {:?}",
            plugin_id, normalized
        );
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
                        discovered.push(CanonicalPathPayload::new(canonical));
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

        let mut watchers = match self.directory_watchers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("🔌 BACKEND: directory_watchers mutex poisoned; recovering");
                let mut inner = poisoned.into_inner();
                inner.clear();
                inner
            }
        };
        watchers.insert(
            plugin_id.to_string(),
            PluginWatcher {
                _debouncer: debouncer,
                task,
            },
        );

        self.perform_initial_discovery(plugin_id, &normalized);

        Ok(())
    }

    fn clear_watched_directories(&self, plugin_id: &str) {
        if let Some(watcher) = match self.directory_watchers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("🔌 BACKEND: directory_watchers mutex poisoned on clear; recovering");
                poisoned.into_inner()
            }
        }
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
                    .map(|payload| payload.display())
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
        self.dispatch_open_waveform_files(plugin_id, paths);
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
        self.bridge
            .update_discovery_config("novywave.files_discovery", None);

        for entry in &section.entries {
            if !entry.enabled {
                if entry.id == "novywave.files_discovery" {
                    self.bridge.update_discovery_config(&entry.id, None);
                }
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

            if entry.id == "novywave.files_discovery" {
                let config = DiscoveryHostConfig::from_entry(entry);
                self.bridge.update_discovery_config(&entry.id, config);
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

    fn flush_initial_discoveries(&self) {
        let pending = self.bridge.take_initial_discoveries();
        for (plugin_id, payloads) in pending {
            if payloads.is_empty() {
                continue;
            }
            println!(
                "🔌 BACKEND: emitting initial discovery for plugin '{}' with {} path(s)",
                plugin_id,
                payloads.len()
            );
            self.bridge
                .dispatch_open_waveform_files(&plugin_id, payloads);
        }
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
    // Be resilient to previous panics: recover from poisoned mutex and try to reinitialize.
    let mut guard = match PLUGIN_MANAGER.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("🔌 BACKEND: plugin manager mutex poisoned; attempting recovery");
            let mut inner = poisoned.into_inner();
            // Try to put the manager back into a clean state
            inner.shutdown_all();
            if let Ok(new_manager) = PluginManager::new() {
                *inner = new_manager;
            }
            inner
        }
    };

    let reloaded = guard.reload(section, opened_files);
    let statuses = guard.statuses();
    PluginReloadOutcome { statuses, reloaded }
}

pub fn flush_initial_discoveries() {
    if let Ok(manager) = PLUGIN_MANAGER.lock() {
        manager.flush_initial_discoveries();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalized_paths_deduplicates_canonical_entries() {
        let input = vec![
            CanonicalPathPayload::new("/tmp/sample.vcd".to_string()),
            CanonicalPathPayload::new("/tmp/sample.vcd".to_string()),
        ];

        let normalized = BackendPluginBridge::normalized_paths(input);
        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].canonical, "/tmp/sample.vcd");
    }

    #[test]
    fn discovery_host_config_matches_expected_files() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_root = env::temp_dir().join(format!("novywave_discovery_test_{suffix}"));
        let target_dir = temp_root.join("test_files/to_discover");
        fs::create_dir_all(&target_dir).unwrap();
        let file_path = target_dir.join("sample.vcd");
        fs::write(&file_path, b"dummy").unwrap();

        let mut entry = PluginConfigEntry::default();
        entry.id = "novywave.files_discovery".to_string();
        entry.config.insert(
            "base_dir".to_string(),
            toml::Value::String(temp_root.to_string_lossy().to_string()),
        );
        entry.config.insert(
            "patterns".to_string(),
            toml::Value::Array(vec![toml::Value::String(
                "test_files/to_discover/**/*.vcd".to_string(),
            )]),
        );

        let config = DiscoveryHostConfig::from_entry(&entry).expect("config present");
        let matcher = config.build_matcher().expect("matcher builds");
        assert!(config.extension_allowed(&file_path));
        assert!(config.matches(&matcher, &file_path));

        fs::remove_dir_all(&temp_root).unwrap();
    }
}
