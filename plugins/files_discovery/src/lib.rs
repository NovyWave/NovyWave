mod bindings {
    wit_bindgen::generate!({
        path: "./wit",
    });
}

use bindings::{__export_world_plugin_cabi, novywave::files_discovery::host, Guest};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{Match, WalkBuilder};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const DEFAULT_DEBOUNCE_MS: u32 = 250;
const MIN_DEBOUNCE_MS: u32 = 50;

struct DiscoveryPlugin;

static STATE: Lazy<Mutex<Option<DiscoveryState>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Deserialize)]
struct DiscoveryConfig {
    #[serde(default)]
    patterns: Vec<String>,
    #[serde(default = "DiscoveryConfig::default_allow_extensions")]
    allow_extensions: Vec<String>,
    #[serde(default)]
    debounce_ms: Option<u32>,
    #[serde(default)]
    base_dir: Option<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        DiscoveryConfig {
            patterns: Vec::new(),
            allow_extensions: Self::default_allow_extensions(),
            debounce_ms: Some(DEFAULT_DEBOUNCE_MS),
            base_dir: None,
        }
    }
}

impl DiscoveryConfig {
    fn default_allow_extensions() -> Vec<String> {
        vec!["fst".to_string(), "vcd".to_string()]
    }

    fn normalize(mut self) -> Self {
        self.patterns = self
            .patterns
            .into_iter()
            .map(|pattern| pattern.trim().to_string())
            .filter(|pattern| !pattern.is_empty())
            .collect();

        if self.allow_extensions.is_empty() {
            self.allow_extensions = Self::default_allow_extensions();
        } else {
            self.allow_extensions = self
                .allow_extensions
                .into_iter()
                .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
                .filter(|ext| !ext.is_empty())
                .collect();
            if self.allow_extensions.is_empty() {
                self.allow_extensions = Self::default_allow_extensions();
            }
        }

        self
    }

    fn debounce_ms(&self) -> u32 {
        self.debounce_ms
            .unwrap_or(DEFAULT_DEBOUNCE_MS)
            .max(MIN_DEBOUNCE_MS)
    }

    fn allowed_extensions_set(&self) -> HashSet<String> {
        self.allow_extensions
            .iter()
            .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
            .collect()
    }

    fn resolved_base_dir(&self) -> PathBuf {
        let fallback = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match self
            .base_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(path) => {
                let candidate = PathBuf::from(path);
                if candidate.is_absolute() {
                    candidate
                } else {
                    fallback.join(candidate)
                }
            }
            None => fallback,
        }
    }
}

struct DiscoveryState {
    config: DiscoveryConfig,
    base_dir: PathBuf,
    matcher: Gitignore,
    watch_roots: Vec<String>,
    opened: HashSet<String>,
    allow_extensions: HashSet<String>,
}

impl DiscoveryState {
    fn build(config: DiscoveryConfig) -> Result<Self, String> {
        if config.patterns.is_empty() {
            return Err("no patterns configured".to_string());
        }

        let base_dir = normalize_base_dir(config.resolved_base_dir());

        let mut builder = GitignoreBuilder::new(&base_dir);
        for pattern in &config.patterns {
            if let Err(err) = builder.add_line(None, pattern) {
                host::log_error(&format!(
                    "files_discovery: failed to register pattern '{}': {}",
                    pattern, err
                ));
            }
        }

        let matcher = builder
            .build()
            .map_err(|err| format!("failed to build matcher: {err}"))?;

        let allow_extensions = config.allowed_extensions_set();
        let watch_roots = compute_watch_roots(&config.patterns, &base_dir);

        Ok(Self {
            config,
            base_dir,
            matcher,
            watch_roots,
            opened: HashSet::new(),
            allow_extensions,
        })
    }

    fn debounce_ms(&self) -> u32 {
        self.config.debounce_ms()
    }

    fn replace_opened(&mut self, opened: Vec<String>) {
        self.opened.clear();
        for path in opened {
            self.opened.insert(path);
        }
    }

    fn collect_initial_matches(&self) -> Vec<String> {
        let mut discovered = Vec::new();
        let mut seen = HashSet::new();

        for root in &self.watch_roots {
            let root_path = PathBuf::from(root);
            if !root_path.exists() {
                continue;
            }

            let mut walker = WalkBuilder::new(&root_path);
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
                        host::log_error(&format!(
                            "files_discovery: walker error under '{}': {}",
                            root, err
                        ));
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
                if !self.extension_allowed(path) {
                    continue;
                }
                if !self.matches_path(path) {
                    continue;
                }

                let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                let canonical_text = canonical.to_string_lossy().to_string();
                if seen.insert(canonical_text.clone()) {
                    discovered.push(canonical_text);
                }
            }
        }

        discovered
    }

    fn filter_and_mark_new<I>(&mut self, candidates: I) -> Vec<String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut unique = HashSet::new();
        let mut new_paths = Vec::new();

        for candidate in candidates {
            if !unique.insert(candidate.clone()) {
                continue;
            }
            if self.opened.contains(&candidate) {
                continue;
            }
            let path = PathBuf::from(&candidate);
            if !self.extension_allowed(&path) {
                continue;
            }
            if !self.matches_path(&path) {
                continue;
            }
            self.opened.insert(candidate.clone());
            new_paths.push(candidate);
        }

        new_paths
    }

    fn matches_path(&self, path: &Path) -> bool {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        };

        matches!(
            self.matcher
                .matched_path_or_any_parents(&absolute, absolute.is_dir()),
            Match::Whitelist(_)
        )
    }

    fn extension_allowed(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .map(|ext| self.allow_extensions.contains(&ext))
            .unwrap_or(false)
    }
}

impl Guest for DiscoveryPlugin {
    fn init() {
        host::log_info("files_discovery: init");
        let config = load_config();

        if config.patterns.is_empty() {
            host::clear_watched_directories();
            host::log_info("files_discovery: no patterns configured; skipping watcher setup");
            return;
        }

        let state = match DiscoveryState::build(config) {
            Ok(state) => state,
            Err(err) => {
                host::log_error(&format!("files_discovery: init failed: {}", err));
                host::clear_watched_directories();
                return;
            }
        };

        {
            let mut guard = STATE.lock().expect("files_discovery state poisoned");
            *guard = Some(state);
        }

        let (watch_roots, debounce_ms, startup_new_paths) = {
            let mut guard = STATE.lock().expect("files_discovery state poisoned");
            let state = guard.as_mut().expect("state missing after initialization");

            state.replace_opened(host::get_opened_files());
            let watch_roots = state.watch_roots.clone();
            let debounce_ms = state.debounce_ms();
            let startup_new_paths = state.filter_and_mark_new(state.collect_initial_matches());
            (watch_roots, debounce_ms, startup_new_paths)
        };

        if watch_roots.is_empty() {
            host::clear_watched_directories();
            host::log_info("files_discovery: no resolvable directories to watch");
        } else {
            host::register_watched_directories(&watch_roots, debounce_ms);
            host::log_info(&format!(
                "files_discovery: watching {} {}",
                watch_roots.len(),
                if watch_roots.len() == 1 {
                    "directory"
                } else {
                    "directories"
                }
            ));
        }

        if !startup_new_paths.is_empty() {
            host::open_waveform_files(&startup_new_paths);
            host::log_info(&format!(
                "files_discovery: opened {} file(s) on startup",
                startup_new_paths.len()
            ));
        } else {
            host::log_info("files_discovery: no new files discovered on startup");
        }
    }

    fn refresh_opened_files() {
        let opened = host::get_opened_files();
        let mut guard = STATE.lock().expect("files_discovery state poisoned");
        if let Some(state) = guard.as_mut() {
            state.replace_opened(opened);
            host::log_info("files_discovery: opened files snapshot refreshed");
        }
    }

    fn paths_discovered(paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }

        let new_paths = {
            let mut guard = STATE.lock().expect("files_discovery state poisoned");
            match guard.as_mut() {
                Some(state) => state.filter_and_mark_new(paths.into_iter()),
                None => {
                    host::log_error(
                        "files_discovery: discovery event received before initialization",
                    );
                    Vec::new()
                }
            }
        };

        if new_paths.is_empty() {
            return;
        }

        host::open_waveform_files(&new_paths);
        host::log_info(&format!(
            "files_discovery: discovered {} new file(s)",
            new_paths.len()
        ));
    }

    fn shutdown() {
        host::clear_watched_directories();
        let mut guard = STATE.lock().expect("files_discovery state poisoned");
        *guard = None;
        host::log_info("files_discovery: shutdown");
    }
}

__export_world_plugin_cabi!(DiscoveryPlugin with_types_in bindings);

fn load_config() -> DiscoveryConfig {
    let raw = host::get_config_toml();
    if raw.trim().is_empty() {
        return DiscoveryConfig::default();
    }

    match parse_config(&raw) {
        Ok(config) => {
            let normalized = config.normalize();
            host::log_info(&format!(
                "files_discovery: config parsed (patterns={}, allow_extensions={}, debounce_ms={})",
                normalized.patterns.len(),
                normalized.allow_extensions.len(),
                normalized.debounce_ms.unwrap_or(DEFAULT_DEBOUNCE_MS),
            ));
            normalized
        }
        Err(err) => {
            host::log_error(&format!(
                "files_discovery: failed to parse config TOML (using defaults): {}",
                err
            ));
            DiscoveryConfig::default()
        }
    }
}

fn parse_config(raw: &str) -> Result<DiscoveryConfig, toml::de::Error> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(DiscoveryConfig::default());
    }

    match toml::from_str::<DiscoveryConfig>(trimmed) {
        Ok(config) => Ok(config),
        Err(primary_err) => {
            #[derive(Deserialize)]
            struct Wrapper {
                target: DiscoveryConfig,
            }

            let wrapped = format!("target = {}\n", trimmed);
            toml::from_str::<Wrapper>(&wrapped)
                .map(|w| w.target)
                .map_err(|_| primary_err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inline_table_config() {
        let raw = r#"{ patterns = ["test_files/**/*.vcd"], allow_extensions = ["vcd"], debounce_ms = 200 }"#;
        let config = parse_config(raw).expect("inline table should parse");
        let normalized = config.normalize();
        assert_eq!(normalized.patterns, vec!["test_files/**/*.vcd".to_string()]);
        assert_eq!(normalized.allow_extensions, vec!["vcd".to_string()]);
        assert_eq!(normalized.debounce_ms(), 200);
    }

    #[test]
    fn parse_document_table_config() {
        let raw = r#"
patterns = ["test_files/**/*.fst"]
allow_extensions = ["fst", "vcd"]
"#;
        let config = parse_config(raw).expect("document should parse");
        let normalized = config.normalize();
        assert_eq!(normalized.patterns, vec!["test_files/**/*.fst".to_string()]);
        assert_eq!(
            normalized.allow_extensions,
            vec!["fst".to_string(), "vcd".to_string()]
        );
    }
}

fn normalize_base_dir(path: PathBuf) -> PathBuf {
    if path.exists() {
        std::fs::canonicalize(&path).unwrap_or(path)
    } else {
        path
    }
}

fn compute_watch_roots(patterns: &[String], base_dir: &Path) -> Vec<String> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();

    for pattern in patterns {
        let candidate = pattern_root(pattern);
        if let Some(root) = resolve_watch_root(base_dir, candidate) {
            if seen.insert(root.clone()) {
                roots.push(root);
            }
        }
    }

    if roots.is_empty() {
        if let Some(root) = resolve_watch_root(base_dir, PathBuf::new()) {
            if seen.insert(root.clone()) {
                roots.push(root);
            }
        }
    }

    roots
}

fn pattern_root(pattern: &str) -> PathBuf {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return PathBuf::new();
    }

    let mut pat = trimmed;
    if let Some(stripped) = pat.strip_prefix('!') {
        pat = stripped;
    }
    pat = pat.trim_start_matches('/');

    if pat.is_empty() {
        return PathBuf::new();
    }

    let mut literal = String::new();
    for ch in pat.chars() {
        if matches!(ch, '*' | '?' | '[') {
            break;
        }
        literal.push(ch);
    }

    let literal = literal.trim_end_matches('/');
    if literal.is_empty() {
        return PathBuf::new();
    }

    let path = PathBuf::from(literal);

    if path.extension().is_some() {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(PathBuf::new)
    } else {
        path
    }
}

fn resolve_watch_root(base_dir: &Path, candidate: PathBuf) -> Option<String> {
    let mut current = if candidate.as_os_str().is_empty() {
        base_dir.to_path_buf()
    } else if candidate.is_absolute() {
        candidate
    } else {
        base_dir.join(candidate)
    };

    loop {
        if current.exists() {
            let canonical = std::fs::canonicalize(&current).unwrap_or_else(|_| current.clone());
            return Some(canonical.to_string_lossy().to_string());
        }

        if !current.pop() {
            break;
        }
    }

    // Fall back to the base directory if no ancestor exists yet.
    if base_dir.exists() {
        let canonical = std::fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
        Some(canonical.to_string_lossy().to_string())
    } else {
        Some(base_dir.to_string_lossy().to_string())
    }
}
