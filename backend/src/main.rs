mod plugins;

use jwalk::WalkDir;
use moon::*;
use serde::{Deserialize, Serialize};
use shared::{
    self, AppConfig, CanonicalPathPayload, DownMsg, FileError, FileFormat, FileHierarchy,
    FileSystemItem, ScopeData, SignalStatistics, SignalTransition, SignalTransitionQuery,
    SignalTransitionResult, SignalValue, SignalValueQuery, SignalValueResult, UnifiedSignalData,
    UnifiedSignalRequest, UpMsg, WaveformFile,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use tokio::time::{Duration, sleep};

// ===== CENTRALIZED DEBUG FLAGS =====
const DEBUG_BACKEND: bool = true; // Backend request/response debugging
const DEBUG_PARSE: bool = true; // File parsing debugging
const DEBUG_SIGNAL_CACHE: bool = true; // Signal cache hit/miss debugging
const DEBUG_CURSOR: bool = true; // Cursor value computation debugging
const DEBUG_WAVEFORM_STORE: bool = true; // Waveform data storage debugging
const DEBUG_EXTRACT: bool = true; // Signal transition extraction debugging

// Debug macro for easy toggling
macro_rules! debug_log {
    ($flag:expr, $($arg:tt)*) => {
        if $flag {
            println!($($arg)*);
        }
    };
}

async fn frontend() -> Frontend {
    Frontend::new().title("NovyWave ").index_by_robots(false)
}

static PARSING_SESSIONS: Lazy<Arc<Mutex<HashMap<String, Arc<Mutex<f32>>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Storage for parsed waveform data to enable signal value queries
struct WaveformData {
    hierarchy: wellen::Hierarchy,
    signal_source: Arc<Mutex<wellen::SignalSource>>,
    time_table: Vec<wellen::Time>,
    signals: HashMap<String, wellen::SignalRef>, // scope_path|variable_name -> SignalRef
    file_format: wellen::FileFormat,             // Store file format for proper time conversion
    timescale_factor: f64, // Conversion factor from VCD native units to seconds
}

static WAVEFORM_DATA_STORE: Lazy<Arc<Mutex<HashMap<String, WaveformData>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Lightweight metadata for deferred loading - avoids loading GB files into memory
#[derive(Clone)]
struct WaveformMetadata {
    _file_path: String, // Stored for future use
    file_format: wellen::FileFormat,
    timescale_factor: f64,
    _time_bounds: (f64, f64), // Stored for future use
}

static WAVEFORM_METADATA_STORE: Lazy<Arc<Mutex<HashMap<String, WaveformMetadata>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Track VCD body loading in progress to prevent concurrent loading of same file
static VCD_LOADING_IN_PROGRESS: Lazy<Arc<Mutex<std::collections::HashSet<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(std::collections::HashSet::new())));

fn invalidate_waveform_resources(file_path: &str) {
    if let Ok(mut store) = WAVEFORM_DATA_STORE.lock() {
        store.remove(file_path);
    }
    if let Ok(mut metadata) = WAVEFORM_METADATA_STORE.lock() {
        metadata.remove(file_path);
    }
    SIGNAL_CACHE_MANAGER.invalidate_file(file_path);
}

// ===== UNIFIED SIGNAL CACHE MANAGER =====

/// High-performance signal cache manager for desktop applications
/// Uses Arc<RwLock<BTreeMap>> for efficient concurrent access
struct SignalCacheManager {
    /// Complete signal transition data indexed by unique signal ID
    transition_cache: Arc<RwLock<BTreeMap<String, Arc<[SignalTransition]>>>>,
    /// Cache statistics for performance monitoring
    cache_stats: Arc<RwLock<CacheStats>>,
}

#[derive(Default)]
struct CacheStats {
    total_queries: usize,
    cache_hits: usize,
    cache_misses: usize,
}

impl SignalCacheManager {
    fn new() -> Self {
        Self {
            transition_cache: Arc::new(RwLock::new(BTreeMap::new())),
            cache_stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    fn invalidate_file(&self, file_path: &str) {
        let prefix = format!("{}|", file_path);
        let mut cache = self.transition_cache.write().unwrap();
        cache.retain(|key, _| !key.starts_with(&prefix));
    }

    fn reset(&self) {
        if let Ok(mut cache) = self.transition_cache.write() {
            cache.clear();
        }
        if let Ok(mut stats) = self.cache_stats.write() {
            *stats = CacheStats::default();
        }
    }

    /// Process unified signal query with parallel processing
    async fn query_unified_signals(
        &self,
        signal_requests: Vec<UnifiedSignalRequest>,
        cursor_time: Option<u64>,
    ) -> Result<
        (
            Vec<UnifiedSignalData>,
            BTreeMap<String, SignalValue>,
            SignalStatistics,
        ),
        String,
    > {
        use std::collections::HashSet;

        let start_time = std::time::Instant::now();

        // Ensure all referenced waveform bodies are loaded before processing requests
        let mut ensured_files = HashSet::new();
        for request in &signal_requests {
            if ensured_files.insert(request.file_path.clone()) {
                match ensure_waveform_body_loaded(&request.file_path).await {
                    Ok(()) => {}
                    Err(error) => {
                        return Err(error);
                    }
                }
            }
        }

        // Load each requested signal; tolerate missing variables so other signals still resolve
        let mut signal_data = Vec::with_capacity(signal_requests.len());
        let mut missing_signals: Vec<(String, String, String, String, Option<(u64, u64)>)> =
            Vec::new();
        for request in &signal_requests {
            let unique_id = format!(
                "{}|{}|{}",
                request.file_path, request.scope_path, request.variable_name
            );

            match self.get_or_load_signal_data(request) {
                Ok(data) => signal_data.push(data),
                Err(err) => {
                    if err.starts_with("Signal data not found:") {
                        debug_log!(
                            DEBUG_BACKEND,
                            "🔍 BACKEND: signal '{}' missing after reload ({})",
                            unique_id,
                            err
                        );
                        missing_signals.push((
                            unique_id,
                            request.file_path.clone(),
                            request.scope_path.clone(),
                            request.variable_name.clone(),
                            request.time_range_ns,
                        ));
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        for (unique_id, file_path, scope_path, variable_name, requested_range) in missing_signals {
            signal_data.push(UnifiedSignalData {
                file_path,
                scope_path,
                variable_name,
                unique_id,
                transitions: Vec::new(),
                total_transitions: 0,
                actual_time_range_ns: requested_range,
            });
        }

        // Compute cursor values if requested
        let cursor_values = if let Some(time) = cursor_time {
            self.compute_cursor_values(&signal_data, time)
        } else {
            BTreeMap::new()
        };

        // Update cache statistics
        let query_time = start_time.elapsed().as_millis() as u64;
        let (total_queries, cache_hits) = {
            let mut stats = self.cache_stats.write().unwrap();
            stats.total_queries += 1;
            (stats.total_queries, stats.cache_hits)
        };
        let cache_hit_ratio = if total_queries > 0 {
            cache_hits as f64 / total_queries as f64
        } else {
            0.0
        };

        let statistics = SignalStatistics {
            total_signals: signal_data.len(),
            cached_signals: cache_hits,
            query_time_ms: query_time,
            cache_hit_ratio,
        };

        let sample_counts: Vec<_> = signal_data
            .iter()
            .map(|data| (data.unique_id.as_str(), data.transitions.len()))
            .collect();
        debug_log!(
            DEBUG_BACKEND,
            "🔧 BACKEND: timeline query finished in {}ms | cache_hit_ratio {:.2} | signals {:?}",
            query_time,
            cache_hit_ratio,
            sample_counts
        );

        Ok((signal_data, cursor_values, statistics))
    }

    /// Get signal data from cache or load from waveform files
    fn get_or_load_signal_data(
        &self,
        request: &UnifiedSignalRequest,
    ) -> Result<UnifiedSignalData, String> {
        let unique_id = format!(
            "{}|{}|{}",
            request.file_path, request.scope_path, request.variable_name
        );
        debug_log!(
            DEBUG_SIGNAL_CACHE,
            "🔍 SIGNAL_CACHE_MANAGER: Looking for signal: '{}'",
            unique_id
        );

        // Check cache first
        {
            let cache = self.transition_cache.read().unwrap();
            debug_log!(
                DEBUG_SIGNAL_CACHE,
                "🔍 SIGNAL_CACHE_MANAGER: Cache contains {} entries",
                cache.len()
            );
            if let Some(transitions) = cache.get(&unique_id) {
                debug_log!(
                    DEBUG_SIGNAL_CACHE,
                    "🔍 SIGNAL_CACHE_MANAGER: Cache HIT for '{}'",
                    unique_id
                );
                let mut stats = self.cache_stats.write().unwrap();
                stats.cache_hits += 1;

                let range_vec = self.collect_range(transitions, request.time_range_ns);

                // Downsample if requested
                let final_transitions = if let Some(max_transitions) = request.max_transitions {
                    self.downsample_transitions(range_vec, max_transitions)
                } else {
                    range_vec
                };

                let total_transitions = transitions.len();
                return Ok(UnifiedSignalData {
                    file_path: request.file_path.clone(),
                    scope_path: request.scope_path.clone(),
                    variable_name: request.variable_name.clone(),
                    unique_id: unique_id.clone(),
                    transitions: final_transitions,
                    total_transitions,
                    actual_time_range_ns: self.compute_time_range(&transitions[..]),
                });
            }
        }

        // Cache miss - load from waveform data
        debug_log!(
            DEBUG_SIGNAL_CACHE,
            "🔍 SIGNAL_CACHE_MANAGER: Cache MISS for '{}' - loading from waveform",
            unique_id
        );
        self.load_signal_from_waveform(request, &unique_id)
    }

    /// Load signal data from the waveform data store
    fn load_signal_from_waveform(
        &self,
        request: &UnifiedSignalRequest,
        unique_id: &str,
    ) -> Result<UnifiedSignalData, String> {
        let mut stats = self.cache_stats.write().unwrap();
        stats.cache_misses += 1;

        let waveform_store = WAVEFORM_DATA_STORE.lock().unwrap();
        debug_log!(
            DEBUG_WAVEFORM_STORE,
            "🔍 WAVEFORM_STORE: Checking for file '{}' in store with {} files",
            request.file_path,
            waveform_store.len()
        );
        if let Some(waveform_data) = waveform_store.get(&request.file_path) {
            debug_log!(
                DEBUG_WAVEFORM_STORE,
                "🔍 WAVEFORM_STORE: Found file '{}' with {} signals",
                request.file_path,
                waveform_data.signals.len()
            );
            // Load transitions from wellen data
            let signal_key = format!("{}|{}", request.scope_path, request.variable_name);
            debug_log!(
                DEBUG_WAVEFORM_STORE,
                "🔍 WAVEFORM_STORE: Looking for signal key '{}' in {} available signals",
                signal_key,
                waveform_data.signals.len()
            );
            debug_log!(
                DEBUG_WAVEFORM_STORE,
                "🔍 WAVEFORM_STORE: Available signal keys: {:?}",
                waveform_data.signals.keys().collect::<Vec<_>>()
            );
            if let Some(signal_ref) = waveform_data.signals.get(&signal_key) {
                debug_log!(
                    DEBUG_WAVEFORM_STORE,
                    "🔍 WAVEFORM_STORE: Found signal '{}' - extracting transitions",
                    signal_key
                );
                let transitions_vec = self.extract_transitions_from_wellen(
                    waveform_data,
                    signal_ref,
                    &request.format,
                    &signal_key,
                )?;

                let transitions_arc: Arc<[SignalTransition]> = transitions_vec.into();

                // Cache the loaded data
                {
                    let mut cache = self.transition_cache.write().unwrap();
                    cache.insert(unique_id.to_string(), Arc::clone(&transitions_arc));
                }

                // Filter by time range and downsample
                let filtered_transitions =
                    self.collect_range(&transitions_arc, request.time_range_ns);

                let final_transitions = if let Some(max_transitions) = request.max_transitions {
                    self.downsample_transitions(filtered_transitions, max_transitions)
                } else {
                    filtered_transitions
                };

                return Ok(UnifiedSignalData {
                    file_path: request.file_path.clone(),
                    scope_path: request.scope_path.clone(),
                    variable_name: request.variable_name.clone(),
                    unique_id: unique_id.to_string(),
                    transitions: final_transitions,
                    total_transitions: transitions_arc.len(),
                    actual_time_range_ns: self.compute_time_range(&transitions_arc[..]),
                });
            } else {
                debug_log!(
                    DEBUG_WAVEFORM_STORE,
                    "🔍 WAVEFORM_STORE: Signal key '{}' NOT FOUND in waveform data",
                    signal_key
                );
            }
        } else {
            debug_log!(
                DEBUG_WAVEFORM_STORE,
                "🔍 WAVEFORM_STORE: File '{}' NOT FOUND in waveform data store",
                request.file_path
            );
        }

        Err(format!("Signal data not found: {}", unique_id))
    }

    /// Extract transitions from wellen signal data
    fn extract_transitions_from_wellen(
        &self,
        waveform_data: &WaveformData,
        signal_ref: &wellen::SignalRef,
        _requested_format: &shared::VarFormat,
        signal_key: &str,
    ) -> Result<Vec<SignalTransition>, String> {
        debug_log!(
            DEBUG_EXTRACT,
            "🔍 EXTRACT_TRANSITIONS: Loading real waveform data for {}",
            signal_key
        );

        if waveform_data.time_table.is_empty() {
            return Ok(Vec::new());
        }

        // Load the signal once outside of the mutex guard for iteration performance
        let loaded_signals = {
            let mut source = waveform_data
                .signal_source
                .lock()
                .map_err(|_| "Signal source unavailable".to_string())?;
            source.load_signals(&[*signal_ref], &waveform_data.hierarchy, true)
        };

        let Some((_, signal)) = loaded_signals.into_iter().next() else {
            return Err(format!(
                "Failed to load signal '{}' from waveform data",
                signal_key
            ));
        };

        let mut transitions = Vec::new();
        let mut last_value: Option<String> = None;

        for (index, &time_native) in waveform_data.time_table.iter().enumerate() {
            // Convert native units to nanoseconds using the stored timescale factor
            let time_seconds = time_native as f64 * waveform_data.timescale_factor;
            let time_ns = (time_seconds * 1_000_000_000.0).round() as u64;

            if let Some(offset) = signal.get_offset(index as u32) {
                let value = signal.get_value_at(&offset, 0);
                let base_value = format_non_binary_signal_value(&value);

                let stored_value = if let Some(bit_string) = value.to_bit_string() {
                    bit_string
                } else {
                    base_value.clone()
                };

                if last_value.as_ref() != Some(&stored_value) {
                    transitions.push(SignalTransition {
                        time_ns,
                        value: stored_value.clone(),
                    });
                    last_value = Some(stored_value);
                }
            }
        }

        debug_log!(
            DEBUG_EXTRACT,
            "🔍 EXTRACT_TRANSITIONS: Collected {} transitions for {}",
            transitions.len(),
            signal_key
        );

        Ok(transitions)
    }

    /// Compute signal values at cursor time
    fn compute_cursor_values(
        &self,
        signal_data: &[UnifiedSignalData],
        cursor_time: u64,
    ) -> BTreeMap<String, SignalValue> {
        let mut cursor_values = BTreeMap::new();
        let cache_guard = self.transition_cache.read().unwrap();

        debug_log!(
            DEBUG_CURSOR,
            "🔍 CURSOR: Computing cursor values at time {}ns for {} signals",
            cursor_time,
            signal_data.len()
        );

        for signal in signal_data {
            let transitions: &[SignalTransition] =
                if let Some(transitions_arc) = cache_guard.get(&signal.unique_id) {
                    &**transitions_arc
                } else {
                    &signal.transitions
                };
            if transitions.is_empty() {
                cursor_values.insert(signal.unique_id.clone(), SignalValue::Missing);
                continue;
            }

            let value = match transitions.binary_search_by(|t| t.time_ns.cmp(&cursor_time)) {
                Ok(idx) => SignalValue::Present(transitions[idx].value.clone()),
                Err(0) => SignalValue::Missing,
                Err(idx) => {
                    let prev = &transitions[idx - 1];
                    SignalValue::Present(prev.value.clone())
                }
            };

            if DEBUG_CURSOR && signal.unique_id.contains("wave_27.fst|TOP|clk") {
                let idx = transitions
                    .binary_search_by(|t| t.time_ns.cmp(&cursor_time))
                    .unwrap_or_else(|i| i);
                let range_start = idx.saturating_sub(2);
                let range_end = (idx + 2).min(transitions.len());
                let window: Vec<String> = transitions[range_start..range_end]
                    .iter()
                    .map(|t| format!("{}:{}", t.time_ns, t.value))
                    .collect();
                debug_log!(
                    DEBUG_CURSOR,
                    "🔍 CURSOR TRACE clk at {} -> {:?} window=[{}] total={}",
                    cursor_time,
                    value,
                    window.join(", "),
                    transitions.len()
                );
            }

            cursor_values.insert(signal.unique_id.clone(), value);
        }

        cursor_values
    }

    fn collect_range(
        &self,
        transitions: &Arc<[SignalTransition]>,
        range: Option<(u64, u64)>,
    ) -> Vec<SignalTransition> {
        if transitions.is_empty() {
            return Vec::new();
        }

        if let Some((start, end)) = range {
            if start >= end {
                return Vec::new();
            }

            let slice = &transitions[..];

            let start_idx = slice
                .binary_search_by(|transition| transition.time_ns.cmp(&start))
                .unwrap_or_else(|idx| idx);
            let end_idx = slice
                .binary_search_by(|transition| transition.time_ns.cmp(&end))
                .map(|idx| idx + 1)
                .unwrap_or_else(|idx| idx)
                .min(slice.len());

            let mut result = Vec::new();

            if start_idx > 0 {
                let prev = &slice[start_idx - 1];
                let needs_synthetic = slice
                    .get(start_idx)
                    .map(|transition| transition.time_ns != start)
                    .unwrap_or(true);
                if needs_synthetic {
                    result.push(SignalTransition {
                        time_ns: start,
                        value: prev.value.clone(),
                    });
                }
            }

            if start_idx < end_idx {
                result.extend(slice[start_idx..end_idx].iter().cloned());
            }

            if let Some(first) = result.first() {
                if first.time_ns > start {
                    result.insert(
                        0,
                        SignalTransition {
                            time_ns: start,
                            value: first.value.clone(),
                        },
                    );
                }
            }

            result
        } else {
            transitions.to_vec()
        }
    }

    /// Downsample transitions for performance
    fn downsample_transitions(
        &self,
        transitions: Vec<SignalTransition>,
        max_count: usize,
    ) -> Vec<SignalTransition> {
        if transitions.len() <= max_count || max_count == 0 {
            return transitions;
        }

        let mut result = Vec::with_capacity(max_count + 2);
        let mut last_value = transitions[0].value.clone();
        result.push(transitions[0].clone());

        let stride = ((transitions.len() as f64 / max_count as f64).ceil() as usize).max(1);
        let mut steps = 0usize;

        for transition in transitions
            .iter()
            .skip(1)
            .take(transitions.len().saturating_sub(1))
        {
            steps += 1;
            let value_changed = transition.value != last_value;
            if value_changed || steps >= stride {
                if result
                    .last()
                    .map(|prev| prev.time_ns != transition.time_ns)
                    .unwrap_or(true)
                {
                    result.push(transition.clone());
                }
                last_value = transition.value.clone();
                steps = 0;
                if result.len() >= max_count {
                    break;
                }
            }
        }

        if let Some(last) = transitions.last() {
            if result
                .last()
                .map(|prev| prev.time_ns != last.time_ns)
                .unwrap_or(true)
            {
                result.push(last.clone());
            }
        }

        result
    }

    /// Compute time range from transitions
    fn compute_time_range(&self, transitions: &[SignalTransition]) -> Option<(u64, u64)> {
        if transitions.is_empty() {
            None
        } else {
            let min_time = transitions.first().unwrap().time_ns;
            let max_time = transitions.last().unwrap().time_ns;
            Some((min_time, max_time))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_path(relative: &str) -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join(relative)
            .display()
            .to_string()
    }

    #[tokio::test]
    async fn unified_signal_query_returns_transitions_for_simple_vcd() {
        let file_path = project_path("../test_files/simple.vcd");

        ensure_waveform_body_loaded(&file_path)
            .await
            .expect("load waveform body");

        let request = UnifiedSignalRequest {
            file_path: file_path.clone(),
            scope_path: "simple_tb.s".into(),
            variable_name: "A".into(),
            time_range_ns: Some((0, 1_000_000)),
            max_transitions: Some(1024),
            format: shared::VarFormat::Binary,
        };

        let (signal_data, cursor_values, _stats) = SignalCacheManager::new()
            .query_unified_signals(vec![request], Some(0))
            .await
            .expect("query unified signals");

        assert!(
            !signal_data.is_empty(),
            "expected at least one signal payload to be returned"
        );
        assert!(
            !signal_data[0].transitions.is_empty(),
            "expected transitions for simple_tb.s|A"
        );
        assert!(
            cursor_values.contains_key(&format!("{}|{}|{}", file_path, "simple_tb.s", "A")),
            "expected cursor value for the requested signal"
        );
    }

    #[tokio::test]
    async fn unified_signal_query_returns_transitions_for_fst_waveform() {
        let file_path = project_path("../test_files/wave_27.fst");

        ensure_waveform_body_loaded(&file_path)
            .await
            .expect("load fst waveform body");

        let request = UnifiedSignalRequest {
            file_path: file_path.clone(),
            scope_path: "TOP".into(),
            variable_name: "clk".into(),
            time_range_ns: Some((0, 10_000_000)),
            max_transitions: Some(4096),
            format: shared::VarFormat::Binary,
        };

        let (signal_data, _cursor_values, _stats) = SignalCacheManager::new()
            .query_unified_signals(vec![request], Some(0))
            .await
            .expect("query unified signals");

        assert!(
            !signal_data.is_empty(),
            "expected at least one signal payload for FST waveform"
        );
        assert!(
            !signal_data[0].transitions.is_empty(),
            "expected transitions for TOP|clk from FST waveform"
        );
    }
}

/// Global signal cache manager instance
static SIGNAL_CACHE_MANAGER: Lazy<SignalCacheManager> = Lazy::new(|| SignalCacheManager::new());

async fn up_msg_handler(req: UpMsgRequest<UpMsg>) {
    let (session_id, cor_id) = (req.session_id, req.cor_id);

    // Log all incoming requests for debugging - with error handling wrapper
    println!(
        "🔍 BACKEND: Received {:?} (session={}, cor={})",
        req.up_msg, session_id, cor_id
    );

    match &req.up_msg {
        UpMsg::LoadWaveformFile(file_path) => {
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: Processing LoadWaveformFile for '{}'",
                file_path
            );
            load_waveform_file(file_path.clone(), session_id, cor_id).await;
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: Completed LoadWaveformFile for '{}'",
                file_path
            );
        }
        UpMsg::GetParsingProgress(file_id) => {
            send_parsing_progress(file_id.clone(), session_id, cor_id).await;
        }
        UpMsg::LoadConfig => {
            println!("🛰️ BACKEND: LoadConfig received");
            load_config(session_id, cor_id).await;
        }
        UpMsg::SelectWorkspace { root } => {
            println!("🛰️ BACKEND: SelectWorkspace requested root='{}'", root);
            select_workspace(root.clone(), session_id, cor_id).await;
        }
        UpMsg::SaveConfig(config) => {
            save_config(config.clone(), session_id, cor_id).await;
        }
        UpMsg::UpdateWorkspaceHistory(history) => {
            handle_workspace_history_update(history.clone());
        }
        UpMsg::FrontendTrace { target, message } => {
            debug_log!(DEBUG_BACKEND, "🛰️ FRONTEND TRACE [{target}]: {message}");
        }
        UpMsg::BrowseDirectory(dir_path) => {
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: enqueue BrowseDirectory {}", dir_path);
            browse_directory(dir_path.clone(), session_id, cor_id).await;
        }
        UpMsg::BrowseDirectories(dir_paths) => {
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: enqueue BrowseDirectories batch {}", dir_paths.len());
            browse_directories_batch(dir_paths.clone(), session_id, cor_id).await;
        }
        UpMsg::QuerySignalValues { file_path, queries } => {
            // Removed spammy debug logging for signal queries
            query_signal_values(file_path.clone(), queries.clone(), session_id, cor_id).await;
        }
        UpMsg::QuerySignalTransitions {
            file_path,
            signal_queries,
            time_range,
        } => {
            // Add detailed error debugging for QuerySignalTransitions
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: Processing QuerySignalTransitions for {} with {} queries",
                file_path,
                signal_queries.len()
            );
            query_signal_transitions(
                file_path.clone(),
                signal_queries.clone(),
                time_range.clone(),
                session_id,
                cor_id,
            )
            .await;
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: Completed QuerySignalTransitions for {}",
                file_path
            );
        }
        UpMsg::BatchQuerySignalValues {
            batch_id,
            file_queries,
        } => {
            // Handle batch signal value queries - process multiple files in one request
            let mut file_results = Vec::new();

            for file_query in file_queries {
                // Process each file's queries
                let results = match process_signal_value_queries_internal(
                    &file_query.file_path,
                    &file_query.queries,
                )
                .await
                {
                    Ok(results) => results,
                    Err(_) => Vec::new(), // Skip failed file queries in batch
                };
                file_results.push(shared::FileSignalResults {
                    file_path: file_query.file_path.clone(),
                    results,
                });
            }

            // Send batch response
            send_down_msg(
                DownMsg::BatchSignalValues {
                    batch_id: batch_id.clone(),
                    file_results,
                },
                session_id,
                cor_id,
            )
            .await;
        }
        UpMsg::UnifiedSignalQuery {
            signal_requests,
            cursor_time_ns,
            request_id,
        } => {
            debug_log!(
                DEBUG_BACKEND,
                "🛰️ BACKEND: UnifiedSignalQuery len={} cursor={:?} id={}",
                signal_requests.len(),
                cursor_time_ns,
                request_id
            );
            // Handle unified signal query using the new cache manager
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: Processing UnifiedSignalQuery with {} requests, cursor_time: {:?}, request_id: {}",
                signal_requests.len(),
                cursor_time_ns,
                request_id
            );
            handle_unified_signal_query(
                signal_requests.clone(),
                cursor_time_ns.clone(),
                request_id.clone(),
                session_id,
                cor_id,
            )
            .await;
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: Completed UnifiedSignalQuery for request_id: {}",
                request_id
            );
        }
        UpMsg::TriggerTestNotifications => {
            println!("🧪 BACKEND: TriggerTestNotifications received - sending test notifications");

            // Send test error notification
            send_down_msg(
                DownMsg::TestNotification {
                    variant: "error".to_string(),
                    title: "Mock Server Error".to_string(),
                    message: "This is a test error from the backend server.".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;

            // Small delay between notifications
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            // Send test info notification
            send_down_msg(
                DownMsg::TestNotification {
                    variant: "info".to_string(),
                    title: "Mock Server Info".to_string(),
                    message: "This is a test info message from the backend server.".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;

            // Small delay between notifications
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            // Send test success notification
            send_down_msg(
                DownMsg::TestNotification {
                    variant: "success".to_string(),
                    title: "Mock Server Success".to_string(),
                    message: "This is a test success message from the backend server.".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;

            println!("✅ BACKEND: Test notifications sent");
        }
    }
}

async fn load_waveform_file(file_path: String, session_id: SessionId, cor_id: CorId) {
    let path = Path::new(&file_path);
    if !path.exists() {
        let error_msg = format!("File not found: {}", file_path);
        send_down_msg(
            DownMsg::ParsingError {
                file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
                error: error_msg.clone(),
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    invalidate_waveform_resources(&file_path);

    // Let wellen handle all validation - it knows best

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let progress = Arc::new(Mutex::new(0.0));

    {
        match PARSING_SESSIONS.lock() {
            Ok(mut sessions) => {
                sessions.insert(file_path.clone(), progress.clone());
            }
            Err(e) => {
                let error_msg =
                    format!("Internal error: Failed to access parsing sessions - {}", e);
                send_parsing_error(file_path.clone(), filename, error_msg, session_id, cor_id)
                    .await;
                return;
            }
        }
    }

    send_down_msg(
        DownMsg::ParsingStarted {
            file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
            filename: filename.clone(),
        },
        session_id,
        cor_id,
    )
    .await;

    // Use wellen's automatic file format detection instead of extension-based detection
    parse_waveform_file(
        file_path.clone(),
        file_path,
        filename,
        progress,
        session_id,
        cor_id,
    )
    .await;
}

async fn send_parsing_error(
    file_id: String,
    filename: String,
    error: String,
    session_id: SessionId,
    cor_id: CorId,
) {
    println!("Parsing error for {}: {}", filename, error);

    send_down_msg(DownMsg::ParsingError { file_id, error }, session_id, cor_id).await;
}

/// Enhanced error sending with structured FileError - provides better error context
async fn send_structured_parsing_error(
    file_id: String,
    filename: String,
    file_error: FileError,
    session_id: SessionId,
    cor_id: CorId,
) {
    // Log the error with structured context for debugging
    println!(
        "Parsing error for {}: {} - {}",
        filename,
        file_error.category(),
        file_error.user_friendly_message()
    );

    // Send the user-friendly message to maintain compatibility with existing frontend
    send_down_msg(
        DownMsg::ParsingError {
            file_id,
            error: file_error.user_friendly_message(),
        },
        session_id,
        cor_id,
    )
    .await;
}

async fn parse_waveform_file(
    file_path: String,
    file_id: String,
    filename: String,
    progress: Arc<Mutex<f32>>,
    session_id: SessionId,
    cor_id: CorId,
) {
    debug_log!(
        DEBUG_PARSE,
        "🔍 PARSE: Starting to parse file '{}' (id: {})",
        file_path,
        file_id
    );

    let options = wellen::LoadOptions::default();

    // Catch panics from wellen parsing to prevent crashes
    // CRITICAL: Use spawn_blocking to avoid blocking the async runtime
    debug_log!(
        DEBUG_PARSE,
        "🔍 PARSE: Calling wellen::viewers::read_header_from_file for '{}'",
        file_path
    );
    let file_path_clone = file_path.clone();
    let parse_result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wellen::viewers::read_header_from_file(&file_path_clone, &options)
        }))
    }).await;

    let header_result = match parse_result {
        Ok(Ok(Ok(header))) => {
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Header parsing SUCCESS for '{}'",
                file_path
            );
            header
        }
        Ok(Ok(Err(e))) => {
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Header parsing ERROR for '{}': {}",
                file_path,
                e
            );
            let file_error = convert_wellen_error_to_file_error(&e.to_string(), &file_path);
            send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
            return;
        }
        Ok(Err(_panic)) => {
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Header parsing PANIC for '{}'",
                file_path
            );
            let file_error = convert_panic_to_file_error(&file_path);
            send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
            return;
        }
        Err(join_error) => {
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Blocking task failed for '{}': {}",
                file_path,
                join_error
            );
            let file_error = FileError::IoError {
                path: file_path.clone(),
                error: format!("Blocking task failed: {}", join_error),
            };
            send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
            return;
        }
    };

    {
        match progress.lock() {
            Ok(mut p) => *p = 0.3, // Header parsed
            Err(_) => {
                // Progress mutex poisoned - continue without progress updates
                eprintln!("Warning: Progress tracking failed for {}", filename);
            }
        }
    }
    send_progress_update(file_id.clone(), 0.3, session_id, cor_id).await;

    // Handle FST and VCD differently for progressive loading
    match header_result.file_format {
        wellen::FileFormat::Fst => {
            // FST: DEFER body loading to avoid loading GB files into memory
            // For initial file loading, we only need header info (scopes, time bounds)
            // Signal data will be loaded on-demand when actually requested

            // Store just the header info without parsing body
            // This prevents loading gigabytes into memory during file selection
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Deferring FST body parsing for '{}' to avoid memory issues",
                file_path
            );

            // For FST files, we need to get time bounds without loading full body
            // Try a quick scan first, return None if we can't determine actual bounds
            let time_bounds_result = match extract_fst_time_bounds_fast(&file_path) {
                Ok(result) => result,
                Err(e) => {
                    send_down_msg(
                        DownMsg::ParsingError {
                            file_id: file_path.clone(),
                            error: format!("Failed to extract FST time bounds: {}", e),
                        },
                        session_id,
                        cor_id,
                    )
                    .await;
                    return;
                }
            };

            // Store the time bounds result for later processing
            let time_bounds_from_scan = time_bounds_result;

            {
                match progress.lock() {
                    Ok(mut p) => *p = 0.7, // Header processing complete
                    Err(_) => {
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 0.7, session_id, cor_id).await;

            // Calculate timescale factor from header
            let timescale_factor = match header_result.hierarchy.timescale() {
                Some(ts) => {
                    use wellen::TimescaleUnit;
                    let factor = match ts.unit {
                        TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                        TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                        TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                        TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                        TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                        TimescaleUnit::Seconds => ts.factor as f64,
                        TimescaleUnit::Unknown => {
                            // NO FALLBACKS: Cannot determine timescale
                            let error_msg = "Unknown timescale unit in FST file";
                            send_down_msg(
                                DownMsg::ParsingError {
                                    file_id: file_path.clone(),
                                    error: error_msg.to_string(),
                                },
                                session_id,
                                cor_id,
                            )
                            .await;
                            return;
                        }
                    };
                    factor
                }
                None => {
                    // NO FALLBACKS: Cannot proceed without timescale
                    let error_msg = "No timescale information in FST file";
                    send_down_msg(
                        DownMsg::ParsingError {
                            file_id: file_path.clone(),
                            error: error_msg.to_string(),
                        },
                        session_id,
                        cor_id,
                    )
                    .await;
                    return;
                }
            };

            // For FST files, determine final time bounds and corrected timescale
            let (min_seconds, max_seconds, final_timescale_factor) = match time_bounds_from_scan {
                Some(bounds) => (bounds.0, bounds.1, timescale_factor),
                None => {
                    // FST files require actual parsing to get time bounds
                    debug_log!(
                        DEBUG_PARSE,
                        "🔍 PARSE: FST file '{}' requires body parsing for time bounds",
                        file_path
                    );

                    // Parse the FST body to get actual time bounds
                    let body_result =
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            wellen::viewers::read_body(
                                header_result.body,
                                &header_result.hierarchy,
                                None,
                            )
                        })) {
                            Ok(Ok(body)) => body,
                            Ok(Err(e)) => {
                                send_down_msg(
                                    DownMsg::ParsingError {
                                        file_id: file_path.clone(),
                                        error: format!("Failed to parse FST body: {}", e),
                                    },
                                    session_id,
                                    cor_id,
                                )
                                .await;
                                return;
                            }
                            Err(_) => {
                                send_down_msg(
                                    DownMsg::ParsingError {
                                        file_id: file_path.clone(),
                                        error: "Critical error parsing FST body".to_string(),
                                    },
                                    session_id,
                                    cor_id,
                                )
                                .await;
                                return;
                            }
                        };

                    // Get actual time bounds from parsed body
                    if let (Some(&min_time), Some(&max_time)) = (
                        body_result.time_table.first(),
                        body_result.time_table.last(),
                    ) {
                        // FST files often have incorrect embedded timescale, use intelligent inference
                        let inferred_timescale = infer_reasonable_fst_timescale(
                            &body_result,
                            timescale_factor,
                            &file_path,
                        );
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 FST inference: raw_min={} raw_max={} raw_range={} embedded_factor={} inferred_factor={}",
                            min_time,
                            max_time,
                            (max_time as i128 - min_time as i128),
                            timescale_factor,
                            inferred_timescale
                        );
                        let min_seconds = min_time as f64 * inferred_timescale;
                        let max_seconds = max_time as f64 * inferred_timescale;

                        // Store the body data for future use since we already parsed it
                        // Build signal reference map (similar to what we do when loading body on demand)
                        let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
                        build_signal_reference_map(&header_result.hierarchy, &mut signals);

                        // Note: We don't store WaveformData here because header_result.hierarchy
                        // will be moved later when creating the WaveformFile. The body will be
                        // loaded on demand when signal values are requested.
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 PARSE: FST body parsed for time bounds, will reload on demand for signal values",
                        );

                        (min_seconds, max_seconds, inferred_timescale)
                    } else {
                        send_down_msg(
                            DownMsg::ParsingError {
                                file_id: file_path.clone(),
                                error: "FST file has no time data".to_string(),
                            },
                            session_id,
                            cor_id,
                        )
                        .await;
                        return;
                    }
                }
            };

            // Convert time bounds to nanoseconds
            let (min_time_ns, max_time_ns) = (
                Some((min_seconds * 1_000_000_000.0) as u64),
                Some((max_seconds * 1_000_000_000.0) as u64),
            );

            // Extract scopes from hierarchy
            let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_path);

            // Store minimal metadata for lazy loading
            // Do NOT parse body or build signal maps yet - that will happen on-demand
            // This keeps memory usage minimal for large files
            // Use inferred timescale for FST files to fix incorrect display
            let waveform_metadata = WaveformMetadata {
                _file_path: file_path.clone(),
                file_format: header_result.file_format,
                timescale_factor: final_timescale_factor, // Use inferred timescale for FST
                _time_bounds: (min_seconds, max_seconds),
            };

            debug_log!(
                DEBUG_PARSE,
                "🔍 FST metadata stored: {} min_ns={} max_ns={} timescale_factor={}",
                file_path,
                min_time_ns.unwrap_or(0),
                max_time_ns.unwrap_or(0),
                final_timescale_factor
            );

            {
                match WAVEFORM_METADATA_STORE.lock() {
                    Ok(mut store) => {
                        store.insert(file_path.clone(), waveform_metadata);
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 PARSE: Successfully stored metadata for '{}' (total files: {})",
                            file_path,
                            store.len()
                        );
                    }
                    Err(e) => {
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 PARSE: Failed to store metadata for '{}': {}",
                            file_path,
                            e
                        );
                        let error_msg =
                            format!("Internal error: Failed to store waveform metadata - {}", e);
                        send_parsing_error(
                            file_id.clone(),
                            filename,
                            error_msg,
                            session_id,
                            cor_id,
                        )
                        .await;
                        return;
                    }
                }
            }

            let format = FileFormat::FST;

            debug_log!(
                DEBUG_BACKEND,
                "🔧 BACKEND FILE: Creating WaveformFile '{}' with range: {:?}ns to {:?}ns",
                file_id,
                min_time_ns,
                max_time_ns
            );

            let waveform_file = WaveformFile {
                id: file_id.clone(),
                filename: filename.clone(),
                format,
                scopes,
                min_time_ns,
                max_time_ns,
            };

            let file_hierarchy = FileHierarchy {
                files: vec![waveform_file],
            };

            {
                match progress.lock() {
                    Ok(mut p) => *p = 1.0, // Complete
                    Err(_) => {
                        // Progress mutex poisoned - continue without progress updates
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 1.0, session_id, cor_id).await;

            send_down_msg(
                DownMsg::FileLoaded {
                    file_id: file_id.clone(),
                    hierarchy: file_hierarchy,
                },
                session_id,
                cor_id,
            )
            .await;

            cleanup_parsing_session(&file_id);
        }
        wellen::FileFormat::Vcd => {
            // VCD: Use progressive loading with quick time bounds extraction

            // First: Try quick time bounds extraction (much faster)
            let (min_seconds, max_seconds) = match extract_vcd_time_bounds_fast(&file_path) {
                Ok((min_time, max_time)) => {
                    // Apply proper timescale conversion for VCD based on unit
                    let timescale_factor = match header_result.hierarchy.timescale() {
                        Some(ts) => {
                            use wellen::TimescaleUnit;
                            match ts.unit {
                                TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                                TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                                TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                                TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                                TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                                TimescaleUnit::Seconds => ts.factor as f64,
                                TimescaleUnit::Unknown => {
                                    // NO FALLBACKS: Cannot determine timescale
                                    let error_msg = "Unknown timescale unit in VCD file";
                                    send_down_msg(
                                        DownMsg::ParsingError {
                                            file_id: file_path.clone(),
                                            error: error_msg.to_string(),
                                        },
                                        session_id,
                                        cor_id,
                                    )
                                    .await;
                                    return;
                                }
                            }
                        }
                        None => {
                            // NO FALLBACKS: Cannot proceed without timescale
                            let error_msg = "No timescale information in VCD file";
                            send_down_msg(
                                DownMsg::ParsingError {
                                    file_id: file_path.clone(),
                                    error: error_msg.to_string(),
                                },
                                session_id,
                                cor_id,
                            )
                            .await;
                            return;
                        }
                    };

                    let converted_min = min_time * timescale_factor;
                    let converted_max = max_time * timescale_factor;

                    debug_log!(
                        DEBUG_BACKEND,
                        "🔧 BACKEND TIME RANGE: VCD file time range: {:.3}s to {:.3}s (span: {:.3}s)",
                        converted_min,
                        converted_max,
                        converted_max - converted_min
                    );

                    (converted_min, converted_max)
                }
                Err(_) => {
                    // ❌ FALLBACK ELIMINATION: Use minimal 1-second range instead of 100s when quick scan fails
                    println!(
                        "VCD quick scan failed, using minimal 1s range - will be updated after full parsing"
                    );
                    (0.0, 1.0) // Minimal range - will be updated after full parsing
                }
            };

            {
                match progress.lock() {
                    Ok(mut p) => *p = 0.5, // Quick time bounds extracted
                    Err(_) => {
                        // Progress mutex poisoned - continue without progress updates
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 0.5, session_id, cor_id).await;

            // Extract scopes from header (immediate - no body parsing needed)
            let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_path);

            let format = FileFormat::VCD;
            let (min_time_ns, max_time_ns) = (
                Some((min_seconds * 1_000_000_000.0) as u64),
                Some((max_seconds * 1_000_000_000.0) as u64),
            );

            debug_log!(
                DEBUG_BACKEND,
                "🔧 BACKEND FILE: Creating WaveformFile '{}' with range: {:?}ns to {:?}ns",
                file_id,
                min_time_ns,
                max_time_ns
            );

            // Create lightweight file data WITHOUT full signal source
            let waveform_file = WaveformFile {
                id: file_id.clone(),
                filename: filename.clone(),
                format,
                scopes,
                min_time_ns,
                max_time_ns,
            };

            let file_hierarchy = FileHierarchy {
                files: vec![waveform_file],
            };

            {
                match progress.lock() {
                    Ok(mut p) => *p = 1.0, // Complete - fast header-only loading!
                    Err(_) => {
                        // Progress mutex poisoned - continue without progress updates
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 1.0, session_id, cor_id).await;

            // VCD: DEFER body parsing to avoid memory issues with GB files
            // Store only metadata, load signal data on-demand when needed
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Deferring VCD body parsing for '{}' to avoid memory issues",
                file_path
            );

            // Calculate timescale factor from header
            let timescale_factor = match header_result.hierarchy.timescale() {
                Some(ts) => {
                    use wellen::TimescaleUnit;
                    match ts.unit {
                        TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                        TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                        TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                        TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                        TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                        TimescaleUnit::Seconds => ts.factor as f64,
                        TimescaleUnit::Unknown => {
                            // NO FALLBACKS: Cannot determine timescale
                            let error_msg = "Unknown timescale unit in VCD file";
                            send_down_msg(
                                DownMsg::ParsingError {
                                    file_id: file_path.clone(),
                                    error: error_msg.to_string(),
                                },
                                session_id,
                                cor_id,
                            )
                            .await;
                            return;
                        }
                    }
                }
                None => {
                    // NO FALLBACKS: Cannot proceed without timescale
                    let error_msg = "No timescale information in VCD file";
                    send_down_msg(
                        DownMsg::ParsingError {
                            file_id: file_path.clone(),
                            error: error_msg.to_string(),
                        },
                        session_id,
                        cor_id,
                    )
                    .await;
                    return;
                }
            };

            // Store minimal metadata for lazy loading
            let waveform_metadata = WaveformMetadata {
                _file_path: file_path.clone(),
                file_format: header_result.file_format,
                timescale_factor,
                _time_bounds: (min_seconds, max_seconds),
            };

            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Storing minimal VCD metadata for '{}' (deferred body loading)",
                file_path
            );

            {
                match WAVEFORM_METADATA_STORE.lock() {
                    Ok(mut store) => {
                        store.insert(file_path.clone(), waveform_metadata);
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 PARSE: Successfully stored VCD metadata for '{}' (total files: {})",
                            file_path,
                            store.len()
                        );
                    }
                    Err(e) => {
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 PARSE: Failed to store VCD metadata for '{}': {}",
                            file_path,
                            e
                        );
                    }
                }
            }

            send_down_msg(
                DownMsg::FileLoaded {
                    file_id: file_id.clone(),
                    hierarchy: file_hierarchy,
                },
                session_id,
                cor_id,
            )
            .await;

            cleanup_parsing_session(&file_id);
        }
        wellen::FileFormat::Ghw => {
            // GHW: Parse body to get time bounds (GHDL files are typically smaller)
            debug_log!(
                DEBUG_PARSE,
                "🔍 PARSE: Processing GHW file '{}' from GHDL",
                file_path
            );

            // Calculate timescale factor from header (GHW has reliable timescale)
            let timescale_factor = match header_result.hierarchy.timescale() {
                Some(ts) => {
                    use wellen::TimescaleUnit;
                    match ts.unit {
                        TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                        TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                        TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                        TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                        TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                        TimescaleUnit::Seconds => ts.factor as f64,
                        TimescaleUnit::Unknown => {
                            let error_msg = "Unknown timescale unit in GHW file";
                            send_down_msg(
                                DownMsg::ParsingError {
                                    file_id: file_path.clone(),
                                    error: error_msg.to_string(),
                                },
                                session_id,
                                cor_id,
                            )
                            .await;
                            return;
                        }
                    }
                }
                None => {
                    let error_msg = "No timescale information in GHW file";
                    send_down_msg(
                        DownMsg::ParsingError {
                            file_id: file_path.clone(),
                            error: error_msg.to_string(),
                        },
                        session_id,
                        cor_id,
                    )
                    .await;
                    return;
                }
            };

            // Parse GHW body to get time bounds
            let body_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None)
            })) {
                Ok(Ok(body)) => body,
                Ok(Err(e)) => {
                    send_down_msg(
                        DownMsg::ParsingError {
                            file_id: file_path.clone(),
                            error: format!("Failed to parse GHW body: {}", e),
                        },
                        session_id,
                        cor_id,
                    )
                    .await;
                    return;
                }
                Err(_) => {
                    send_down_msg(
                        DownMsg::ParsingError {
                            file_id: file_path.clone(),
                            error: "Critical error parsing GHW body".to_string(),
                        },
                        session_id,
                        cor_id,
                    )
                    .await;
                    return;
                }
            };

            // Get time bounds from parsed body
            let (min_seconds, max_seconds) = if let (Some(&min_time), Some(&max_time)) = (
                body_result.time_table.first(),
                body_result.time_table.last(),
            ) {
                let min_seconds = min_time as f64 * timescale_factor;
                let max_seconds = max_time as f64 * timescale_factor;
                debug_log!(
                    DEBUG_PARSE,
                    "🔍 GHW time bounds: raw_min={} raw_max={} seconds_min={:.6} seconds_max={:.6}",
                    min_time,
                    max_time,
                    min_seconds,
                    max_seconds
                );
                (min_seconds, max_seconds)
            } else {
                send_down_msg(
                    DownMsg::ParsingError {
                        file_id: file_path.clone(),
                        error: "GHW file has no time data".to_string(),
                    },
                    session_id,
                    cor_id,
                )
                .await;
                return;
            };

            // Convert to nanoseconds
            let (min_time_ns, max_time_ns) = (
                Some((min_seconds * 1_000_000_000.0) as u64),
                Some((max_seconds * 1_000_000_000.0) as u64),
            );

            // Extract scopes from hierarchy
            let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_path);

            // Store metadata for on-demand signal loading
            let waveform_metadata = WaveformMetadata {
                _file_path: file_path.clone(),
                file_format: header_result.file_format,
                timescale_factor,
                _time_bounds: (min_seconds, max_seconds),
            };

            {
                match WAVEFORM_METADATA_STORE.lock() {
                    Ok(mut store) => {
                        store.insert(file_path.clone(), waveform_metadata);
                        debug_log!(
                            DEBUG_PARSE,
                            "🔍 PARSE: Successfully stored GHW metadata for '{}' (total files: {})",
                            file_path,
                            store.len()
                        );
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Internal error: Failed to store GHW metadata - {}", e);
                        send_parsing_error(
                            file_id.clone(),
                            filename,
                            error_msg,
                            session_id,
                            cor_id,
                        )
                        .await;
                        return;
                    }
                }
            }

            let format = FileFormat::GHW;

            let waveform_file = WaveformFile {
                id: file_id.clone(),
                filename: filename.clone(),
                format,
                scopes,
                min_time_ns,
                max_time_ns,
            };

            let file_hierarchy = FileHierarchy {
                files: vec![waveform_file],
            };

            {
                match progress.lock() {
                    Ok(mut p) => *p = 1.0,
                    Err(_) => {
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 1.0, session_id, cor_id).await;

            send_down_msg(
                DownMsg::FileLoaded {
                    file_id: file_id.clone(),
                    hierarchy: file_hierarchy,
                },
                session_id,
                cor_id,
            )
            .await;

            cleanup_parsing_session(&file_id);
        }
        wellen::FileFormat::Unknown => {
            let error_msg = "Unknown file format - only VCD, FST, and GHW files are supported";
            send_parsing_error(file_id, filename, error_msg.to_string(), session_id, cor_id).await;
        }
    }
}

/// Convert wellen parsing errors to structured FileError for better error handling
fn convert_wellen_error_to_file_error(wellen_error: &str, file_path: &str) -> FileError {
    let path = file_path.to_string();

    // Pattern match common wellen error messages to appropriate FileError types
    if wellen_error.contains("No such file") || wellen_error.contains("not found") {
        FileError::FileNotFound { path }
    } else if wellen_error.contains("Permission denied") || wellen_error.contains("permission") {
        FileError::PermissionDenied { path }
    } else if wellen_error.contains("corrupted")
        || wellen_error.contains("invalid format")
        || wellen_error.contains("malformed")
    {
        FileError::CorruptedFile {
            path,
            details: wellen_error.to_string(),
        }
    } else if wellen_error.contains("too large") || wellen_error.contains("size") {
        // Extract size information if available, otherwise use defaults
        FileError::FileTooLarge {
            path,
            size: 0,                 // Could parse from error message if needed
            max_size: 1_000_000_000, // 1GB default
        }
    } else if wellen_error.contains("unsupported") || wellen_error.contains("format") {
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        FileError::UnsupportedFormat {
            path,
            extension: extension.to_string(),
            supported_formats: vec!["vcd".to_string(), "fst".to_string(), "ghw".to_string()],
        }
    } else {
        // Generic parsing error for everything else
        FileError::ParseError {
            source: wellen_error.to_string(),
            context: format!("Failed to parse waveform file: {}", file_path),
        }
    }
}

/// Convert panic messages to structured FileError
fn convert_panic_to_file_error(file_path: &str) -> FileError {
    FileError::CorruptedFile {
        path: file_path.to_string(),
        details: "Critical error: Invalid waveform data or corrupted file".to_string(),
    }
}

/// Intelligent FST timescale inference to handle files with incorrect embedded timescale
fn infer_reasonable_fst_timescale(
    body_result: &wellen::viewers::BodyResult,
    embedded_factor: f64,
    _filename: &str,
) -> f64 {
    if body_result.time_table.is_empty() {
        return embedded_factor; // Can't infer from empty time table
    }

    let (raw_min, raw_max) = match (
        body_result.time_table.first(),
        body_result.time_table.last(),
    ) {
        (Some(&min), Some(&max)) => (min as f64, max as f64),
        _ => {
            // If time bounds can't be determined, return the embedded factor
            return embedded_factor;
        }
    };
    let raw_range = raw_max - raw_min;

    // Check if embedded timescale produces unreasonably long duration (>1000 seconds)
    let computed_duration = raw_range * embedded_factor;

    if computed_duration > 1000.0 {
        // FST inference: embedded timescale produces unreasonably long duration

        // Heuristic inference based on value magnitude - optimized for typical FPGA/digital designs
        let inferred_factor = if raw_range > 1e15 {
            1e-15 // femtoseconds
        } else if raw_range > 1e12 {
            1e-12 // picoseconds
        } else if raw_range > 1e6 {
            1e-9 // nanoseconds - most common for FPGA/CPU designs (covers 1ms to 1000s of sim time)
        } else if raw_range > 1e3 {
            1e-6 // microseconds
        } else {
            1e-3 // milliseconds
        };

        return inferred_factor;
    }
    embedded_factor
}

fn _extract_fst_time_range(
    body_result: &wellen::viewers::BodyResult,
    timescale_factor: f64,
) -> (f64, f64) {
    if body_result.time_table.is_empty() {
        // ❌ FALLBACK ELIMINATION: Use minimal 1-second range instead of 100s for empty FST
        return (0.0, 1.0); // Minimal fallback for empty FST
    }

    let raw_min = match body_result.time_table.first() {
        Some(time) => *time as f64,
        None => {
            eprintln!("Warning: Empty time table in FST file");
            // ❌ FALLBACK ELIMINATION: Use minimal 1-second range instead of 100s
            return (0.0, 1.0);
        }
    };

    let raw_max = match body_result.time_table.last() {
        Some(time) => *time as f64,
        None => {
            eprintln!("Warning: Empty time table in FST file");
            // ❌ FALLBACK ELIMINATION: Use minimal 1-second range instead of 100s
            return (0.0, 1.0);
        }
    };

    // Removed spammy FST time extraction debug logging

    // Convert FST time values to seconds using the proper timescale factor
    let min_seconds = raw_min * timescale_factor;
    let max_seconds = raw_max * timescale_factor;

    debug_log!(
        DEBUG_BACKEND,
        "🔧 BACKEND TIME RANGE: FST file time range: {:.3}s to {:.3}s (span: {:.3}s)",
        min_seconds,
        max_seconds,
        max_seconds - min_seconds
    );

    (min_seconds, max_seconds)
}

fn extract_fst_time_bounds_fast(
    _file_path: &str,
) -> Result<Option<(f64, f64)>, Box<dyn std::error::Error>> {
    // FST files need to be parsed to get time bounds
    // However, we can use a callback to stop early once we have min/max
    // For now, we cannot determine bounds without parsing the body
    // This could be improved with FST-specific scanning in the future

    // NO FALLBACKS: Return None if we can't determine actual time bounds
    // The frontend should show "Loading..." or appropriate state
    Ok(None)
}

fn extract_vcd_time_bounds_fast(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs::File;

    let file = File::open(file_path)?;
    let file_size = file.metadata()?.len();

    // For large files, use memory-mapped scanning
    if file_size > 100_000_000 {
        // 100MB threshold
        extract_vcd_time_bounds_mmap(file_path)
    } else {
        extract_vcd_time_bounds_small(file_path)
    }
}

fn extract_vcd_time_bounds_mmap(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs::File;

    let file = File::open(file_path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };

    // Find end of header ($enddefinitions $end)
    let header_end = find_vcd_definitions_end(&mmap)?;
    let body_section = &mmap[header_end..];

    // Scan for first timestamp from beginning of body
    let first_time = find_first_vcd_timestamp(body_section)?;

    // Scan for last timestamp from end (more efficient)
    let last_time = find_last_vcd_timestamp_reverse(body_section)?;

    Ok((first_time, last_time))
}

fn extract_vcd_time_bounds_small(
    file_path: &str,
) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs;

    let content = fs::read_to_string(file_path)?;

    // Find end of header
    let header_end = content
        .find("$enddefinitions $end")
        .ok_or("VCD header end marker not found")?;

    let body_section = &content[header_end..];

    // Find first and last timestamps
    let first_time = find_first_vcd_timestamp_str(body_section)?;
    let last_time = find_last_vcd_timestamp_str(body_section)?;

    Ok((first_time, last_time))
}

fn find_vcd_definitions_end(data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = b"$enddefinitions $end";
    let pos = data
        .windows(pattern.len())
        .position(|window| window == pattern)
        .ok_or("VCD header end marker not found")?;
    Ok(pos + pattern.len())
}

fn find_first_vcd_timestamp(data: &[u8]) -> Result<f64, Box<dyn std::error::Error>> {
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'#' {
            // Found timestamp marker, parse the number
            i += 1;
            let mut timestamp_str = String::new();
            while i < data.len() && data[i].is_ascii_digit() {
                timestamp_str.push(data[i] as char);
                i += 1;
            }
            if !timestamp_str.is_empty() {
                return Ok(timestamp_str.parse::<f64>()?);
            }
        }
        i += 1;
    }
    Err("No VCD timestamp found".into())
}

fn find_last_vcd_timestamp_reverse(data: &[u8]) -> Result<f64, Box<dyn std::error::Error>> {
    let mut i = data.len();
    while i > 0 {
        i -= 1;
        if data[i] == b'#' && i + 1 < data.len() && data[i + 1].is_ascii_digit() {
            // Found timestamp marker, parse the number forward
            let mut j = i + 1;
            let mut timestamp_str = String::new();
            while j < data.len() && data[j].is_ascii_digit() {
                timestamp_str.push(data[j] as char);
                j += 1;
            }
            if !timestamp_str.is_empty() {
                return Ok(timestamp_str.parse::<f64>()?);
            }
        }
    }
    Err("No VCD timestamp found in reverse scan".into())
}

fn find_first_vcd_timestamp_str(body: &str) -> Result<f64, Box<dyn std::error::Error>> {
    for line in body.lines() {
        if line.starts_with('#') {
            let timestamp_str = &line[1..].split_whitespace().next().unwrap_or("");
            if !timestamp_str.is_empty() {
                return Ok(timestamp_str.parse::<f64>()?);
            }
        }
    }
    Err("No VCD timestamp found".into())
}

fn find_last_vcd_timestamp_str(body: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let mut last_timestamp = None;
    for line in body.lines() {
        if line.starts_with('#') {
            let timestamp_str = &line[1..].split_whitespace().next().unwrap_or("");
            if !timestamp_str.is_empty() {
                if let Ok(timestamp) = timestamp_str.parse::<f64>() {
                    last_timestamp = Some(timestamp);
                }
            }
        }
    }
    last_timestamp.ok_or("No VCD timestamp found in body".into())
}

fn extract_scopes_from_hierarchy(hierarchy: &wellen::Hierarchy, file_path: &str) -> Vec<ScopeData> {
    hierarchy
        .scopes()
        .map(|scope_ref| extract_scope_data_with_file_path(hierarchy, scope_ref, file_path))
        .collect()
}

fn extract_scope_data_with_file_path(
    hierarchy: &wellen::Hierarchy,
    scope_ref: wellen::ScopeRef,
    file_path: &str,
) -> ScopeData {
    let scope = &hierarchy[scope_ref];

    let mut variables: Vec<shared::Signal> = scope
        .vars(hierarchy)
        .map(|var_ref| {
            let var = &hierarchy[var_ref];
            shared::Signal {
                id: var.name(hierarchy).to_string(), // Use variable name as ID
                name: var.name(hierarchy).to_string(),
                signal_type: format!("{:?}", var.var_type()),
                width: match var.signal_encoding() {
                    wellen::SignalEncoding::BitVector(width) => width.get(),
                    wellen::SignalEncoding::Real => 1,
                    wellen::SignalEncoding::String => 1,
                },
            }
        })
        .collect();
    variables.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut children: Vec<ScopeData> = scope
        .scopes(hierarchy)
        .map(|child_scope_ref| {
            extract_scope_data_with_file_path(hierarchy, child_scope_ref, file_path)
        })
        .collect();
    children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let scope_type_str = format!("{:?}", scope.scope_type());

    ScopeData {
        id: format!("{}|{}", file_path, scope.full_name(hierarchy)), // Use full file path + | separator + scope path for unique ID
        name: scope.name(hierarchy).to_string(),
        full_name: scope.full_name(hierarchy),
        children,
        variables,
        scope_type: Some(scope_type_str),
    }
}

async fn send_parsing_progress(file_id: String, session_id: SessionId, cor_id: CorId) {
    let sessions = match PARSING_SESSIONS.lock() {
        Ok(sessions) => sessions,
        Err(_) => {
            eprintln!("Warning: Failed to access parsing sessions for progress update");
            return;
        }
    };

    if let Some(progress) = sessions.get(&file_id) {
        let current_progress = match progress.lock() {
            Ok(p) => *p,
            Err(_) => {
                eprintln!("Warning: Failed to read progress for file {}", file_id);
                return;
            }
        };
        send_progress_update(file_id, current_progress, session_id, cor_id).await;
    }
}

async fn send_progress_update(
    file_id: String,
    progress: f32,
    session_id: SessionId,
    cor_id: CorId,
) {
    send_down_msg(
        DownMsg::ParsingProgress { file_id, progress },
        session_id,
        cor_id,
    )
    .await;
}

async fn send_down_msg(msg: DownMsg, session_id: SessionId, cor_id: CorId) {
    if let Some(session) = sessions::by_session_id().wait_for(session_id).await {
        session.send_down_msg(&msg, cor_id).await;
    } else {
    }
}

const CONFIG_FILENAME: &str = ".novywave";
const GLOBAL_CONFIG_FILENAME: &str = ".novywave_global";

static INITIAL_CWD: Lazy<PathBuf> =
    Lazy::new(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

static WORKSPACE_CONTEXT: Lazy<WorkspaceContext> =
    Lazy::new(|| WorkspaceContext::new((*INITIAL_CWD).clone()));

pub(crate) fn workspace_context() -> &'static WorkspaceContext {
    &WORKSPACE_CONTEXT
}

fn is_global_workspace_active() -> bool {
    workspace_context().root() == *INITIAL_CWD
}

struct WorkspaceContext {
    root: RwLock<PathBuf>,
}

impl WorkspaceContext {
    fn new(initial_root: PathBuf) -> Self {
        Self {
            root: RwLock::new(initial_root),
        }
    }

    fn root(&self) -> PathBuf {
        self.root
            .read()
            .expect("workspace root lock poisoned")
            .clone()
    }

    fn set_root(&self, new_root: PathBuf) {
        let mut guard = self.root.write().expect("workspace root lock poisoned");
        *guard = new_root;
    }

    fn config_path(&self) -> PathBuf {
        let root = self.root();
        root.join(CONFIG_FILENAME)
    }

    fn to_absolute(&self, candidate: impl AsRef<Path>) -> PathBuf {
        let candidate = candidate.as_ref();
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            let root = self.root();
            root.join(candidate)
        }
    }

    fn to_relative_if_in_workspace(&self, candidate: impl AsRef<Path>) -> Option<PathBuf> {
        let root = self.root();
        candidate
            .as_ref()
            .strip_prefix(&root)
            .map(|p| p.to_path_buf())
            .ok()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct GlobalConfigFile {
    #[serde(default)]
    global: shared::GlobalSection,
}

fn global_config_path() -> PathBuf {
    // Use platform-specific config directory for global config
    // Linux: ~/.config/novywave/.novywave_global
    // macOS: ~/Library/Application Support/novywave/.novywave_global
    // Windows: %APPDATA%\novywave\.novywave_global
    dirs::config_dir()
        .unwrap_or_else(|| INITIAL_CWD.clone())
        .join("novywave")
        .join(GLOBAL_CONFIG_FILENAME)
}

fn read_global_section() -> shared::GlobalSection {
    let path = global_config_path();
    let mut section = match fs::read_to_string(&path) {
        Ok(content) => toml::from_str::<GlobalConfigFile>(&content)
            .map(|file| file.global)
            .unwrap_or_default(),
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "⚠️ BACKEND: Failed to read global config '{}': {}",
                    path.display(),
                    err
                );
            }
            shared::GlobalSection::default()
        }
    };
    section
        .workspace_history
        .clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
    section
}

fn save_global_section(
    mut global: shared::GlobalSection,
) -> Result<shared::GlobalSection, Box<dyn std::error::Error>> {
    global
        .workspace_history
        .clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);

    // Keep the global file truly global: only persist last_selected, recent_paths,
    // and the picker_tree_state. Drop per-workspace tree_state entries.
    global.workspace_history.tree_state.clear();

    let file = GlobalConfigFile {
        global: global.clone(),
    };
    let toml_content = toml::to_string_pretty(&file)?;
    let content_with_header = format!(
        "# NovyWave Global Configuration\n\
         # Stores workspace history shared across all projects\n\
         \n\
         {}",
        toml_content
    );

    let path = global_config_path();
    debug_log!(
        DEBUG_BACKEND,
        "💾 BACKEND: Writing global config to {} (picker_state={:?})",
        path.display(),
        file.global.workspace_history.picker_tree_state
    );
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content_with_header)?;
    Ok(global)
}

fn update_workspace_history_on_select(root: &Path) -> shared::GlobalSection {
    let mut global = read_global_section();
    let path_str = root.to_string_lossy().to_string();
    global
        .workspace_history
        .touch_path(&path_str, shared::WORKSPACE_HISTORY_MAX_RECENTS);
    match save_global_section(global.clone()) {
        Ok(updated) => updated,
        Err(err) => {
            eprintln!("⚠️ BACKEND: Failed to persist workspace history: {}", err);
            global
        }
    }
}

fn persist_global_section(global: &shared::GlobalSection) -> shared::GlobalSection {
    let existing = read_global_section();
    let incoming_history = &global.workspace_history;
    let has_incoming_updates = has_workspace_history_updates(incoming_history);
    if !has_incoming_updates {
        return existing;
    }

    let mut merged = existing.clone();
    merge_workspace_history(&mut merged.workspace_history, incoming_history);

    if merged.workspace_history == existing.workspace_history {
        return existing;
    }

    match save_global_section(merged.clone()) {
        Ok(updated) => updated,
        Err(err) => {
            eprintln!(
                "⚠️ BACKEND: Failed to save global workspace history: {}",
                err
            );
            updated_on_failure(global)
        }
    }
}

fn updated_on_failure(global: &shared::GlobalSection) -> shared::GlobalSection {
    let mut fallback = global.clone();
    fallback
        .workspace_history
        .clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
    fallback
}

fn handle_workspace_history_update(history: shared::WorkspaceHistory) {
    debug_log!(
        DEBUG_BACKEND,
        "🔧 BACKEND: UpdateWorkspaceHistory recents={:?} picker={:?}",
        history.recent_paths,
        history.picker_tree_state
    );
    let global_section = shared::GlobalSection {
        workspace_history: history,
    };
    let _ = persist_global_section(&global_section);
}

fn has_workspace_history_updates(history: &shared::WorkspaceHistory) -> bool {
    history
        .last_selected
        .as_ref()
        .map(|value| !value.is_empty())
        .unwrap_or(false)
        || !history.recent_paths.is_empty()
        || !history.tree_state.is_empty()
        || history.picker_tree_state.is_some()
}

fn merge_workspace_history(
    target: &mut shared::WorkspaceHistory,
    incoming: &shared::WorkspaceHistory,
) {
    if incoming
        .last_selected
        .as_ref()
        .map(|value| !value.is_empty())
        .unwrap_or(false)
    {
        target.last_selected = incoming.last_selected.clone();
    }

    if !incoming.recent_paths.is_empty() {
        target.recent_paths = incoming.recent_paths.clone();
    }

    if !incoming.tree_state.is_empty() {
        target.tree_state = incoming.tree_state.clone();
    }

    if let Some(incoming_picker) = &incoming.picker_tree_state {
        match target.picker_tree_state.as_mut() {
            Some(existing) => {
                // Always update scroll_top on picker writes
                existing.scroll_top = incoming_picker.scroll_top;
                // Only replace expanded_paths when non-empty arrives to avoid clearing
                if !incoming_picker.expanded_paths.is_empty() {
                    existing.expanded_paths = incoming_picker.expanded_paths.clone();
                }
            }
            None => {
                // First-time write: accept full picker state
                target.picker_tree_state = Some(incoming_picker.clone());
            }
        }
    }
}

fn expand_config_paths(config: &mut AppConfig) {
    let context = workspace_context();

    config.workspace.opened_files = config
        .workspace
        .opened_files
        .iter()
        .map(|payload| {
            let path_buf = PathBuf::from(&payload.canonical);
            let absolute = context.to_absolute(&path_buf);
            CanonicalPathPayload::new(absolute.to_string_lossy().to_string())
        })
        .collect();

    config.workspace.load_files_expanded_directories = config
        .workspace
        .load_files_expanded_directories
        .iter()
        .map(|dir| {
            let path_buf = PathBuf::from(dir);
            let absolute = context.to_absolute(&path_buf);
            absolute.to_string_lossy().to_string()
        })
        .collect();

    config.workspace.expanded_scopes = config
        .workspace
        .expanded_scopes
        .iter()
        .map(|id| expand_scope_identifier(id, context))
        .collect();

    if let Some(selected_scope) = config.workspace.selected_scope_id.clone() {
        config.workspace.selected_scope_id =
            Some(expand_scope_identifier(&selected_scope, context));
    }

    config.workspace.selected_variables = config
        .workspace
        .selected_variables
        .iter()
        .map(|var| {
            let mut cloned = var.clone();
            cloned.unique_id = expand_unique_id(&cloned.unique_id, context);
            cloned
        })
        .collect();
}

fn relativize_config_paths(config: &mut AppConfig) {
    let context = workspace_context();

    config.workspace.opened_files = config
        .workspace
        .opened_files
        .iter()
        .map(|payload| {
            let path_buf = PathBuf::from(&payload.canonical);
            let normalized = context
                .to_relative_if_in_workspace(&path_buf)
                .unwrap_or(path_buf.clone());
            CanonicalPathPayload::new(normalized.to_string_lossy().to_string())
        })
        .collect();

    config.workspace.load_files_expanded_directories = config
        .workspace
        .load_files_expanded_directories
        .iter()
        .map(|dir| {
            let path_buf = PathBuf::from(dir);
            let normalized = context
                .to_relative_if_in_workspace(&path_buf)
                .unwrap_or(path_buf.clone());
            normalized.to_string_lossy().to_string()
        })
        .collect();

    config.workspace.expanded_scopes = config
        .workspace
        .expanded_scopes
        .iter()
        .map(|id| relativize_scope_identifier(id, context))
        .collect();

    if let Some(selected_scope) = config.workspace.selected_scope_id.clone() {
        config.workspace.selected_scope_id =
            Some(relativize_scope_identifier(&selected_scope, context));
    }

    config.workspace.selected_variables = config
        .workspace
        .selected_variables
        .iter()
        .map(|var| {
            let mut cloned = var.clone();
            cloned.unique_id = relativize_unique_id(&cloned.unique_id, context);
            cloned
        })
        .collect();
}

fn expand_unique_id(unique_id: &str, context: &WorkspaceContext) -> String {
    if let Some((path_part, rest)) = unique_id.split_once('|') {
        let path_buf = PathBuf::from(path_part);
        let absolute = context.to_absolute(&path_buf);
        format!("{}|{}", absolute.to_string_lossy(), rest)
    } else {
        let path_buf = PathBuf::from(unique_id);
        let absolute = context.to_absolute(&path_buf);
        absolute.to_string_lossy().to_string()
    }
}

fn relativize_unique_id(unique_id: &str, context: &WorkspaceContext) -> String {
    if let Some((path_part, rest)) = unique_id.split_once('|') {
        let path_buf = PathBuf::from(path_part);
        let normalized = context
            .to_relative_if_in_workspace(&path_buf)
            .unwrap_or(path_buf.clone());
        format!("{}|{}", normalized.to_string_lossy(), rest)
    } else {
        unique_id.to_string()
    }
}

fn expand_scope_identifier(scope_id: &str, context: &WorkspaceContext) -> String {
    if let Some(stripped) = scope_id.strip_prefix("scope_") {
        let (path_part, remainder) = match stripped.split_once('|') {
            Some((path, rest)) => (path, Some(rest)),
            None => (stripped, None),
        };
        let path_buf = PathBuf::from(path_part);
        let absolute = context.to_absolute(&path_buf);
        let mut result = format!("scope_{}", absolute.to_string_lossy());
        if let Some(rest) = remainder {
            result.push('|');
            result.push_str(rest);
        }
        result
    } else {
        scope_id.to_string()
    }
}

fn relativize_scope_identifier(scope_id: &str, context: &WorkspaceContext) -> String {
    if let Some(stripped) = scope_id.strip_prefix("scope_") {
        let (path_part, remainder) = match stripped.split_once('|') {
            Some((path, rest)) => (path, Some(rest)),
            None => (stripped, None),
        };
        let path_buf = PathBuf::from(path_part);
        let normalized = context
            .to_relative_if_in_workspace(&path_buf)
            .unwrap_or(path_buf.clone());
        let mut result = format!("scope_{}", normalized.to_string_lossy());
        if let Some(rest) = remainder {
            result.push('|');
            result.push_str(rest);
        }
        result
    } else {
        scope_id.to_string()
    }
}

fn reset_runtime_state_for_workspace() {
    if let Ok(mut sessions) = PARSING_SESSIONS.lock() {
        sessions.clear();
    }
    if let Ok(mut store) = WAVEFORM_DATA_STORE.lock() {
        store.clear();
    }
    if let Ok(mut metadata) = WAVEFORM_METADATA_STORE.lock() {
        metadata.clear();
    }
    if let Ok(mut loading) = VCD_LOADING_IN_PROGRESS.lock() {
        loading.clear();
    }
    SIGNAL_CACHE_MANAGER.reset();
}

fn read_or_create_config() -> Result<AppConfig, String> {
    let context = workspace_context();
    let config_path = context.config_path();

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                let mut default_config = AppConfig::default();
                let _ = default_config.validate_and_fix();
                expand_config_paths(&mut default_config);
                if let Err(save_err) = save_config_to_file(&default_config) {
                    return Err(format!(
                        "Failed to create default config '{}': {}",
                        config_path.display(),
                        save_err
                    ));
                }
                return Ok(default_config);
            } else {
                return Err(format!(
                    "Failed to read config '{}': {}",
                    config_path.display(),
                    err
                ));
            }
        }
    };

    let mut config: AppConfig = toml::from_str(&content).map_err(|err| {
        format!(
            "Failed to parse config '{}': {}",
            config_path.display(),
            err
        )
    })?;

    let migration_warnings = config.validate_and_fix();
    if !migration_warnings.is_empty() {
        if let Err(save_err) = save_config_to_file(&config) {
            eprintln!(
                "⚠️ BACKEND: Failed to persist migrated config '{}': {}",
                config_path.display(),
                save_err
            );
        }
    }

    expand_config_paths(&mut config);
    Ok(config)
}

fn spawn_directory_preload(expanded_dirs: Vec<String>) {
    if expanded_dirs.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let mut preload_tasks = Vec::new();
        for dir_path in expanded_dirs {
            preload_tasks.push(tokio::spawn(async move {
                let path_obj = Path::new(&dir_path);
                if path_obj.exists() && path_obj.is_dir() {
                    let _ = scan_directory_async(path_obj).await;
                }
            }));
        }

        for task in preload_tasks {
            let _ = task.await;
        }
    });
}

async fn handle_loaded_config(
    config: AppConfig,
    session_id: SessionId,
    cor_id: CorId,
    workspace_event_root: Option<String>,
) {
    let config_for_messages = config;

    spawn_directory_preload(
        config_for_messages
            .workspace
            .load_files_expanded_directories
            .clone(),
    );

    // Guard plugin reload/flush so a plugin panic cannot block config delivery
    match std::panic::catch_unwind(|| {
        plugins::reload_plugins(
            &config_for_messages.plugins,
            &config_for_messages.workspace.opened_files,
        )
    }) {
        Ok(plugin_reload) => {
            if plugin_reload.reloaded {
                for status in &plugin_reload.statuses {
                    let init_msg = status
                        .init_message
                        .as_ref()
                        .map(|msg| msg.as_str())
                        .unwrap_or("");
                    let last_msg = status
                        .last_message
                        .as_ref()
                        .map(|msg| msg.as_str())
                        .unwrap_or("");
                    let detail = match (init_msg.is_empty(), last_msg.is_empty()) {
                        (false, false) => format!(" | init: {} | last: {}", init_msg, last_msg),
                        (false, true) => format!(" | init: {}", init_msg),
                        (true, false) => format!(" | last: {}", last_msg),
                        (true, true) => String::new(),
                    };
                    println!(
                        "🔌 BACKEND: plugin '{}' status {:?}{}",
                        status.id, status.state, detail
                    );
                }
            }
        }
        Err(_) => {
            eprintln!("🔌 BACKEND: plugin reload panicked; continuing without plugins");
        }
    }

    let _ = std::panic::catch_unwind(|| plugins::flush_initial_discoveries());

    if let Some(root) = workspace_event_root {
        let default_root_display = INITIAL_CWD.to_string_lossy().to_string();
        send_down_msg(
            DownMsg::WorkspaceLoaded {
                root,
                default_root: default_root_display,
                config: config_for_messages.clone(),
            },
            session_id,
            cor_id,
        )
        .await;
    }

    println!(
        "🚀 BACKEND: Sending ConfigLoaded to frontend with {} expanded directories",
        config_for_messages
            .workspace
            .load_files_expanded_directories
            .len()
    );

    send_down_msg(
        DownMsg::ConfigLoaded(config_for_messages),
        session_id,
        cor_id,
    )
    .await;
}

async fn load_config(session_id: SessionId, cor_id: CorId) {
    // 1) Read global first and set the workspace root so we load the correct per-workspace config
    let global_section = read_global_section();
    if let Some(last) = global_section.workspace_history.last_selected.clone() {
        if !last.is_empty() {
            let candidate = PathBuf::from(&last);
            let absolute = if candidate.is_absolute() {
                candidate
            } else {
                INITIAL_CWD.join(candidate)
            };
            if let Ok(canon) = fs::canonicalize(&absolute) {
                if canon.is_dir() {
                    workspace_context().set_root(canon);
                }
            }
        }
    }

    // 2) Load the per-workspace config for the selected root
    match read_or_create_config() {
        Ok(mut config) => {
            // Attach the global section we read above
            let mut effective_global = global_section.clone();
            // Ensure recents list limits are applied consistently
            effective_global
                .workspace_history
                .clamp_to_limit(shared::WORKSPACE_HISTORY_MAX_RECENTS);
            config.global = effective_global;

            let root_display = workspace_context().root().to_string_lossy().to_string();
            handle_loaded_config(config, session_id, cor_id, Some(root_display)).await;
        }
        Err(message) => {
            send_down_msg(DownMsg::ConfigError(message), session_id, cor_id).await;
        }
    }
}

async fn save_config(config: AppConfig, session_id: SessionId, cor_id: CorId) {
    let mut runtime_config = config.clone();
    expand_config_paths(&mut runtime_config);

    let plugin_reload = plugins::reload_plugins(
        &runtime_config.plugins,
        &runtime_config.workspace.opened_files,
    );
    if plugin_reload.reloaded {
        for status in &plugin_reload.statuses {
            let init_msg = status
                .init_message
                .as_ref()
                .map(|msg| msg.as_str())
                .unwrap_or("");
            let last_msg = status
                .last_message
                .as_ref()
                .map(|msg| msg.as_str())
                .unwrap_or("");
            let detail = match (init_msg.is_empty(), last_msg.is_empty()) {
                (false, false) => format!(" | init: {} | last: {}", init_msg, last_msg),
                (false, true) => format!(" | init: {}", init_msg),
                (true, false) => format!(" | last: {}", last_msg),
                (true, true) => String::new(),
            };
            println!(
                "🔌 BACKEND: plugin '{}' status {:?}{}",
                status.id, status.state, detail
            );
        }
    }

    plugins::flush_initial_discoveries();

    let persisted_global = persist_global_section(&config.global);
    if is_global_workspace_active() {
        runtime_config.global = persisted_global;
    } else {
        runtime_config.global = shared::GlobalSection::default();
    }

    match save_config_to_file(&runtime_config) {
        Ok(()) => {
            send_down_msg(DownMsg::ConfigSaved, session_id, cor_id).await;
        }
        Err(e) => {
            send_down_msg(
                DownMsg::ConfigError(format!("Failed to save config: {}", e)),
                session_id,
                cor_id,
            )
            .await;
        }
    }
}

async fn select_workspace(root: String, session_id: SessionId, cor_id: CorId) {
    let context = workspace_context();
    let requested = PathBuf::from(&root);
    let absolute_candidate = if requested.is_absolute() {
        requested
    } else {
        context.to_absolute(&requested)
    };

    let canonical_root = match fs::canonicalize(&absolute_candidate) {
        Ok(path) => path,
        Err(err) => {
            send_down_msg(
                DownMsg::ConfigError(format!("Failed to open workspace '{}': {}", root, err)),
                session_id,
                cor_id,
            )
            .await;
            return;
        }
    };

    if !canonical_root.is_dir() {
        send_down_msg(
            DownMsg::ConfigError(format!(
                "Workspace '{}' is not a directory",
                canonical_root.display()
            )),
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    context.set_root(canonical_root.clone());
    reset_runtime_state_for_workspace();

    let global_section = update_workspace_history_on_select(&canonical_root);

    match read_or_create_config() {
        Ok(mut config) => {
            let root_display = canonical_root.to_string_lossy().to_string();
            config.global = global_section.clone();
            handle_loaded_config(config, session_id, cor_id, Some(root_display)).await;
        }
        Err(message) => {
            send_down_msg(DownMsg::ConfigError(message), session_id, cor_id).await;
        }
    }
}

fn save_config_to_file(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut config_to_write = config.clone();
    relativize_config_paths(&mut config_to_write);
    let toml_content = toml::to_string_pretty(&config_to_write)?;

    // Add header comment
    let content_with_header = format!(
        "# NovyWave User Configuration\n\
         # This file stores your application preferences and workspace state\n\
         \n\
         {}",
        toml_content
    );

    let config_path = workspace_context().config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config_path, content_with_header)?;
    Ok(())
}

fn cleanup_parsing_session(file_id: &str) {
    match PARSING_SESSIONS.lock() {
        Ok(mut sessions) => {
            sessions.remove(file_id);
        }
        Err(_) => {
            eprintln!(
                "Warning: Failed to cleanup parsing session for file {}",
                file_id
            );
        }
    }
}

async fn browse_directory(dir_path: String, session_id: SessionId, cor_id: CorId) {
    println!("🗂️ browse_directory request path='{}'", dir_path);
    debug_log!(
        DEBUG_BACKEND,
        "🔍 BACKEND: browse_directory called for path: {}",
        dir_path
    );

    // Handle Windows multi-root scenario - enumerate drives when browsing "/"
    #[cfg(windows)]
    if dir_path == "/" {
        let mut drive_items = Vec::new();

        // Enumerate available drives (A: through Z:)
        for drive_letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", drive_letter as char);
            let drive_root = Path::new(&drive_path);

            // Check if drive exists and is accessible
            if drive_root.exists() {
                drive_items.push(FileSystemItem {
                    name: format!("{}:", drive_letter as char),
                    path: drive_path.clone(),
                    is_directory: true,
                    file_size: None,
                    is_waveform_file: false,
                    file_extension: None,
                    has_expandable_content: true,
                });
            }
        }

        // Sort drives alphabetically
        drive_items.sort_by(|a, b| a.name.cmp(&b.name));

        send_down_msg(
            DownMsg::DirectoryContents {
                path: "/".to_string(),
                items: drive_items,
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    // Handle Linux/Unix root directory - provide useful starting points instead of actual "/"
    #[cfg(unix)]
    if dir_path == "/" {
        debug_log!(
            DEBUG_BACKEND,
            "🔍 BACKEND: Linux root directory requested, providing common paths"
        );
        let mut root_items = Vec::new();

        // Add common useful directories for file selection
        let useful_paths = vec![
            ("/home", "home"),
            ("/tmp", "tmp"),
            ("/opt", "opt"),
            ("/usr", "usr"),
            ("/var", "var"),
        ];

        for (path, name) in useful_paths {
            let path_obj = Path::new(path);
            if path_obj.exists() && path_obj.is_dir() {
                root_items.push(FileSystemItem {
                    name: name.to_string(),
                    path: path.to_string(),
                    is_directory: true,
                    file_size: None,
                    is_waveform_file: false,
                    file_extension: None,
                    has_expandable_content: true,
                });
            }
        }

        eprintln!(
            "🔍 BACKEND: Returning {} Linux root items",
            root_items.len()
        );
        send_down_msg(
            DownMsg::DirectoryContents {
                path: "/".to_string(),
                items: root_items,
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    // Expand ~ to home directory
    let expanded_path = if dir_path == "~" {
        match dirs::home_dir() {
            Some(home) => home.to_string_lossy().to_string(),
            None => {
                let error_msg = "Unable to determine home directory".to_string();
                send_down_msg(
                    DownMsg::DirectoryError {
                        path: dir_path,
                        error: error_msg,
                    },
                    session_id,
                    cor_id,
                )
                .await;
                return;
            }
        }
    } else if dir_path.starts_with("~/") {
        match dirs::home_dir() {
            Some(home) => {
                let relative_path = &dir_path[2..]; // Remove "~/"
                home.join(relative_path).to_string_lossy().to_string()
            }
            None => {
                let error_msg = "Unable to determine home directory".to_string();
                send_down_msg(
                    DownMsg::DirectoryError {
                        path: dir_path,
                        error: error_msg,
                    },
                    session_id,
                    cor_id,
                )
                .await;
                return;
            }
        }
    } else {
        dir_path.clone()
    };

    let path = Path::new(&expanded_path);

    // Check if directory exists and is readable
    if !path.exists() {
        let error_msg = format!("Directory not found: {}", expanded_path);
        send_down_msg(
            DownMsg::DirectoryError {
                path: expanded_path,
                error: error_msg,
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    if !path.is_dir() {
        let error_msg = format!("Path is not a directory: {}", expanded_path);
        send_down_msg(
            DownMsg::DirectoryError {
                path: expanded_path,
                error: error_msg,
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    // Use async parallel directory scanning for maximum performance
    match scan_directory_async(path).await {
        Ok(items) => {
            send_down_msg(
                DownMsg::DirectoryContents {
                    path: expanded_path.clone(),
                    items,
                },
                session_id,
                cor_id,
            )
            .await;
        }
        Err(e) => {
            let error_msg = format!("Failed to scan directory: {}", e);
            send_down_msg(
                DownMsg::DirectoryError {
                    path: expanded_path,
                    error: error_msg,
                },
                session_id,
                cor_id,
            )
            .await;
        }
    }
}

async fn browse_directories_batch(dir_paths: Vec<String>, session_id: SessionId, cor_id: CorId) {
    println!("🗂️ browse_directories batch {:?} (len={})", dir_paths, dir_paths.len());
    // Use jwalk's parallel processing capabilities for batch directory scanning
    let mut results = HashMap::new();

    // Process directories in parallel using jwalk's thread pool
    let parallel_tasks: Vec<_> = dir_paths
        .into_iter()
        .map(|dir_path| {
            tokio::spawn(async move {
                let expanded_path = if dir_path.starts_with("~/") {
                    // Expand home directory path
                    if let Some(home_dir) = dirs::home_dir() {
                        home_dir.join(&dir_path[2..]).to_string_lossy().to_string()
                    } else {
                        dir_path
                    }
                } else if dir_path == "~" {
                    // User home directory
                    dirs::home_dir().map_or(dir_path, |home| home.to_string_lossy().to_string())
                } else {
                    dir_path
                };

                let path = Path::new(&expanded_path);

                // Scan directory with jwalk
                let result = if !path.exists() {
                    Err(format!("Path does not exist: {}", expanded_path))
                } else if !path.is_dir() {
                    Err(format!("Path is not a directory: {}", expanded_path))
                } else {
                    match scan_directory_async(path).await {
                        Ok(items) => Ok(items),
                        Err(e) => Err(format!("Failed to scan directory: {}", e)),
                    }
                };

                (expanded_path, result)
            })
        })
        .collect();

    // Collect all results
    for task in parallel_tasks {
        if let Ok((path, result)) = task.await {
            results.insert(path, result);
        }
    }

    // Send batch results to frontend
    send_down_msg(
        DownMsg::BatchDirectoryContents { results },
        session_id,
        cor_id,
    )
    .await;
}

async fn scan_directory_async(
    path: &Path,
) -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
    let path_buf = path.to_path_buf();

    // Use jwalk for parallel directory traversal, bridged with tokio
    let items = tokio::task::spawn_blocking(
        move || -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
            let mut items = Vec::new();

            // Test directory access before jwalk to catch permission errors early
            match std::fs::read_dir(&path_buf) {
                Ok(_) => {
                    // Directory is readable, proceed with jwalk
                }
                Err(e) => {
                    // Return permission/access error immediately
                    return Err(format!("Permission denied: {}", e).into());
                }
            }

            // jwalk with parallel processing, single directory level
            for entry in WalkDir::new(&path_buf)
                .sort(true) // Enable sorting for consistent results
                .max_depth(1) // Single level only (like TreeView expansion)
                .skip_hidden(false) // We'll filter manually to match existing logic
                .process_read_dir(|_, _, _, dir_entry_results| {
                    // Filter entries in parallel processing callback for better performance
                    dir_entry_results.retain(|entry_result| {
                        if let Ok(entry) = entry_result {
                            let name = entry.file_name().to_string_lossy();
                            !name.starts_with('.') // Skip hidden files
                        } else {
                            true // Keep errors for proper handling
                        }
                    });
                })
            {
                match entry {
                    Ok(dir_entry) => {
                        let entry_path = dir_entry.path();

                        // Skip the root directory itself (jwalk includes it)
                        if entry_path == path_buf {
                            continue;
                        }

                        let name = entry_path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        let is_directory = entry_path.is_dir();
                        let path_str = entry_path.to_string_lossy().to_string();

                        // Only include directories and waveform files for cleaner file dialog
                        let is_waveform = if !is_directory {
                            let name_lower = name.to_lowercase();
                            name_lower.ends_with(".vcd")
                                || name_lower.ends_with(".fst")
                                || name_lower.ends_with(".ghw")
                        } else {
                            false
                        };

                        // Skip non-waveform files to reduce clutter
                        if !is_directory && !is_waveform {
                            continue;
                        }

                        let item = FileSystemItem {
                            name,
                            path: path_str,
                            is_directory,
                            file_size: None, // Skip file size for instant loading
                            is_waveform_file: is_waveform, // Proper waveform detection
                            file_extension: None, // Skip extension parsing for instant loading
                            has_expandable_content: is_directory, // All directories expandable
                        };

                        items.push(item);
                    }
                    Err(_e) => {
                        // Continue processing other entries despite individual errors
                    }
                }
            }

            // Sort items: directories first, then files, both alphabetically
            // jwalk's sort(true) provides basic ordering, but we need custom logic
            items.sort_by(|a, b| match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            Ok(items)
        },
    )
    .await??;

    Ok(items)
}

// Build signal reference map for efficient lookup during value queries
fn build_signal_reference_map(
    hierarchy: &wellen::Hierarchy,
    signals: &mut HashMap<String, wellen::SignalRef>,
) {
    // Recursively process all scopes in the hierarchy
    for scope_ref in hierarchy.scopes() {
        build_signals_for_scope_recursive(hierarchy, scope_ref, signals);
    }
}

// Recursively process a scope and all its child scopes
fn build_signals_for_scope_recursive(
    hierarchy: &wellen::Hierarchy,
    scope_ref: wellen::ScopeRef,
    signals: &mut HashMap<String, wellen::SignalRef>,
) {
    let scope = &hierarchy[scope_ref];
    let scope_path = scope.full_name(hierarchy);

    // Process variables in this scope
    for var_ref in scope.vars(hierarchy) {
        let var = &hierarchy[var_ref];
        let variable_name = var.name(hierarchy);
        let signal_ref = var.signal_ref();

        // Key format: "scope_path|variable_name" to match SelectedVariable format
        let key = format!("{}|{}", scope_path, variable_name);
        signals.insert(key, signal_ref);
    }

    // Recursively process child scopes
    for child_scope_ref in scope.scopes(hierarchy) {
        build_signals_for_scope_recursive(hierarchy, child_scope_ref, signals);
    }
}

// Handle signal value queries
// Load VCD body data on-demand for signal value queries
async fn ensure_waveform_body_loaded(file_path: &str) -> Result<(), String> {
    // Check if already loaded
    {
        let store = match WAVEFORM_DATA_STORE.lock() {
            Ok(store) => store,
            Err(_) => {
                return Err("Internal error: Failed to access waveform data store".to_string());
            }
        };
        if store.contains_key(file_path) {
            return Ok(());
        }
    }

    // Check if loading is already in progress for this file
    {
        let mut loading_in_progress = match VCD_LOADING_IN_PROGRESS.lock() {
            Ok(loading) => loading,
            Err(_) => return Err("Internal error: Failed to access loading tracker".to_string()),
        };

        if loading_in_progress.contains(file_path) {
            drop(loading_in_progress);

            loop {
                // Check if waveform finished loading successfully
                if WAVEFORM_DATA_STORE
                    .lock()
                    .map(|store| store.contains_key(file_path))
                    .unwrap_or(false)
                {
                    return Ok(());
                }

                // If the loader cleared the in-progress flag without populating the store,
                // treat it as a failure and bubble the error up to the caller.
                let still_loading = VCD_LOADING_IN_PROGRESS
                    .lock()
                    .map(|loading| loading.contains(file_path))
                    .unwrap_or(false);

                if !still_loading {
                    break;
                }

                sleep(Duration::from_millis(10)).await;
            }

            if WAVEFORM_DATA_STORE
                .lock()
                .map(|store| store.contains_key(file_path))
                .unwrap_or(false)
            {
                return Ok(());
            } else {
                return Err(format!(
                    "Waveform '{}' failed to load while waiting for existing load",
                    file_path
                ));
            }
        }

        // Mark this file as being loaded
        loading_in_progress.insert(file_path.to_string());
    }

    // Check if we have metadata for this file
    let metadata = {
        let metadata_store = match WAVEFORM_METADATA_STORE.lock() {
            Ok(store) => store,
            Err(_) => {
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err("Internal error: Failed to access metadata store".to_string());
            }
        };
        let meta = metadata_store.get(file_path).cloned();
        debug_log!(
            DEBUG_PARSE,
            "🔍 ensure_waveform_body_loaded metadata lookup for {} -> {}",
            file_path,
            meta.as_ref()
                .map(|m| format!("factor={} format={:?}", m.timescale_factor, m.file_format))
                .unwrap_or_else(|| "missing".to_string())
        );
        meta
    };

    // If we have metadata, we can skip header parsing
    let (hierarchy, file_format, mut timescale_factor) = if let Some(metadata) = metadata.as_ref() {
        debug_log!(
            DEBUG_PARSE,
            "🔍 LAZY: Have metadata for '{}', re-parsing header for hierarchy",
            file_path
        );

        // Re-parse the header to get hierarchy (lightweight operation)
        let options = wellen::LoadOptions::default();
        let header_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wellen::viewers::read_header_from_file(file_path, &options)
        })) {
            Ok(Ok(header)) => header,
            Ok(Err(e)) => {
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err(format!("Failed to re-parse header for body loading: {}", e));
            }
            Err(_panic) => {
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err(format!("Critical error re-parsing header"));
            }
        };

        (
            header_result.hierarchy,
            metadata.file_format.clone(),
            metadata.timescale_factor,
        )
    } else {
        // No metadata, need to parse header (shouldn't normally happen in lazy loading)
        debug_log!(
            DEBUG_PARSE,
            "🔍 LAZY: No metadata for '{}', parsing header",
            file_path
        );
        let options = wellen::LoadOptions::default();

        // Catch panics from header parsing
        let header_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wellen::viewers::read_header_from_file(file_path, &options)
        })) {
            Ok(Ok(header)) => header,
            Ok(Err(e)) => {
                // Remove from loading tracker on failure
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err(format!("Failed to parse header for signal queries: {}", e));
            }
            Err(_panic) => {
                // Remove from loading tracker on panic
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err(format!(
                    "Critical error parsing header: Invalid waveform data"
                ));
            }
        };

        // Extract timescale factor
        let timescale_factor = match header_result.hierarchy.timescale() {
            Some(ts) => {
                use wellen::TimescaleUnit;
                match ts.unit {
                    TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                    TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                    TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                    TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                    TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                    TimescaleUnit::Seconds => ts.factor as f64,
                    TimescaleUnit::Unknown => {
                        // NO FALLBACKS: Cannot determine timescale
                        if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                            loading_in_progress.remove(file_path);
                        }
                        return Err("Unknown timescale unit in waveform file".to_string());
                    }
                }
            }
            None => {
                // NO FALLBACKS: Cannot proceed without timescale
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err("No timescale information in waveform file".to_string());
            }
        };

        (
            header_result.hierarchy,
            header_result.file_format,
            timescale_factor,
        )
    };

    // Now parse the body on-demand
    // NOTE: This still loads the entire body into memory, but only when signals are actually requested
    // This defers the memory load until necessary, not during file selection
    debug_log!(
        DEBUG_PARSE,
        "🔍 LAZY: Loading waveform body for '{}' on-demand",
        file_path
    );

    // Re-parse the file to get the body
    let options = wellen::LoadOptions::default();
    let header_with_body = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_header_from_file(file_path, &options)
    })) {
        Ok(Ok(header)) => header,
        Ok(Err(e)) => {
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Failed to reparse file for body loading: {}", e));
        }
        Err(_panic) => {
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Critical error reparsing file for body loading"));
        }
    };

    // Parse body (this is where the memory is used, but only on-demand)
    let body_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_body(header_with_body.body, &hierarchy, None)
    })) {
        Ok(Ok(body)) => body,
        Ok(Err(e)) => {
            // Remove from loading tracker on failure
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!(
                "Failed to parse VCD body for signal queries: {}",
                e
            ));
        }
        Err(_panic) => {
            // Remove from loading tracker on panic
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!(
                "Critical error parsing VCD body: Invalid signal data"
            ));
        }
    };

    // Build signal reference map
    let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
    build_signal_reference_map(&hierarchy, &mut signals);

    if metadata.is_none() && matches!(file_format, wellen::FileFormat::Fst) {
        let inferred = infer_reasonable_fst_timescale(&body_result, timescale_factor, file_path);
        debug_log!(
            DEBUG_PARSE,
            "🔍 ensure_waveform_body_loaded FST inference fallback: file={} span={} embedded={} inferred={}",
            file_path,
            body_result
                .time_table
                .last()
                .copied()
                .unwrap_or(0)
                .saturating_sub(body_result.time_table.first().copied().unwrap_or(0)),
            timescale_factor,
            inferred
        );
        timescale_factor = inferred;
    }

    // Store waveform data
    let waveform_data = WaveformData {
        hierarchy,
        signal_source: Arc::new(Mutex::new(body_result.source)),
        time_table: body_result.time_table.clone(),
        signals,
        file_format,
        timescale_factor,
    };

    debug_log!(
        DEBUG_PARSE,
        "🔍 ensure_waveform_body_loaded: {} timescale_factor={} time_table_span={}",
        file_path,
        timescale_factor,
        waveform_data
            .time_table
            .last()
            .copied()
            .unwrap_or(0)
            .saturating_sub(waveform_data.time_table.first().copied().unwrap_or(0))
    );

    {
        match WAVEFORM_DATA_STORE.lock() {
            Ok(mut store) => {
                store.insert(file_path.to_string(), waveform_data);
            }
            Err(_) => {
                // Remove from loading tracker on failure
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err("Internal error: Failed to store waveform data".to_string());
            }
        }
    }

    // Remove from loading tracker on success
    if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
        loading_in_progress.remove(file_path);
    }

    Ok(())
}

/// Generate alternative scope path formats to handle format mismatches between frontend and backend
fn generate_scope_path_fallbacks(scope_path: &str, variable_name: &str) -> Vec<String> {
    let mut fallbacks = Vec::new();

    // Try different scope path format alternatives
    // 1. Remove trailing dots: "simple_tb.s." -> "simple_tb.s"
    let scope_no_trailing_dot = scope_path.trim_end_matches('.');
    if scope_no_trailing_dot != scope_path {
        fallbacks.push(format!("{}|{}", scope_no_trailing_dot, variable_name));
    }

    // 2. Convert dots to underscores: "simple_tb.s" -> "simple_tb_s"
    let scope_dots_to_underscores = scope_path.replace('.', "_");
    if scope_dots_to_underscores != scope_path {
        fallbacks.push(format!("{}|{}", scope_dots_to_underscores, variable_name));
    }

    // 3. Remove all dots: "simple_tb.s" -> "simple_tbs"
    let scope_no_dots = scope_path.replace('.', "");
    if scope_no_dots != scope_path {
        fallbacks.push(format!("{}|{}", scope_no_dots, variable_name));
    }

    // 4. Try just the last component: "simple_tb.s" -> "s"
    if let Some(last_component) = scope_path.split('.').last() {
        if last_component != scope_path {
            fallbacks.push(format!("{}|{}", last_component, variable_name));
        }
    }

    // 5. Try without any scope (just variable name): "" + variable_name
    fallbacks.push(format!("|{}", variable_name));
    fallbacks.push(variable_name.to_string());

    debug_log!(
        DEBUG_BACKEND,
        "🔧 Generated {} fallback keys for scope '{}' variable '{}':",
        fallbacks.len(),
        scope_path,
        variable_name
    );
    for fallback in &fallbacks {
        println!("  - '{}'", fallback);
    }

    fallbacks
}

async fn query_signal_values(
    file_path: String,
    queries: Vec<SignalValueQuery>,
    session_id: SessionId,
    cor_id: CorId,
) {
    // For waveform files, ensure body is loaded on-demand (VCD, FST, etc)
    if let Err(e) = ensure_waveform_body_loaded(&file_path).await {
        send_down_msg(
            DownMsg::SignalValuesError {
                file_path,
                error: e,
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    let store = match WAVEFORM_DATA_STORE.lock() {
        Ok(store) => store,
        Err(_) => {
            send_down_msg(
                DownMsg::SignalValuesError {
                    file_path,
                    error: "Internal error: Failed to access waveform data store".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;
            return;
        }
    };

    let waveform_data = match store.get(&file_path) {
        Some(data) => data,
        None => {
            send_down_msg(
                DownMsg::SignalValuesError {
                    file_path,
                    error: "Waveform file not loaded or signal data not available".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;
            return;
        }
    };

    let mut results = Vec::new();

    for query in queries {
        let key = format!("{}|{}", query.scope_path, query.variable_name);

        // Debug: Log the query key and available keys for troubleshooting
        if !waveform_data.signals.contains_key(&key) {
            debug_log!(
                DEBUG_BACKEND,
                "🔍 SIGNAL NOT FOUND: Looking for key '{}' in file '{}'",
                key,
                file_path
            );
            debug_log!(
                DEBUG_BACKEND,
                "🔍 Available signal keys ({} total):",
                waveform_data.signals.len()
            );
            for (available_key, _) in waveform_data.signals.iter().take(10) {
                println!("  - '{}'", available_key);
            }
            if waveform_data.signals.len() > 10 {
                println!("  ... and {} more keys", waveform_data.signals.len() - 10);
            }
        }

        // Try main key first, then fallback alternatives for scope path format mismatches
        let signal_ref = if let Some(&signal_ref) = waveform_data.signals.get(&key) {
            Some(signal_ref)
        } else {
            // Try scope path fallback alternatives - remove dots, try different separators
            let fallback_keys =
                generate_scope_path_fallbacks(&query.scope_path, &query.variable_name);
            fallback_keys
                .iter()
                .filter_map(|fallback_key| waveform_data.signals.get(fallback_key))
                .next()
                .copied()
        };

        match signal_ref {
            Some(signal_ref) => {
                // Convert time to time table index based on file format using proper timescale
                let target_time = match waveform_data.file_format {
                    wellen::FileFormat::Vcd => {
                        // For VCD: Convert from seconds to VCD native units using stored timescale
                        (query.time_seconds / waveform_data.timescale_factor) as u64
                    }
                    _ => {
                        // For other formats, use proper timescale conversion
                        (query.time_seconds / waveform_data.timescale_factor) as u64
                    }
                };

                let time_idx = match waveform_data.time_table.binary_search(&target_time) {
                    Ok(exact_idx) => exact_idx as u32,
                    Err(insert_pos) => {
                        if insert_pos == 0 || query.time_seconds <= 0.0 {
                            // Query is at time 0 or before any data - check if we have initial values
                            if !waveform_data.time_table.is_empty()
                                && waveform_data.time_table[0] == 0
                            {
                                0 // Use time index 0 if it exists
                            } else {
                                // No data at time 0, return N/A
                                results.push(SignalValueResult {
                                    scope_path: query.scope_path,
                                    variable_name: query.variable_name,
                                    time_seconds: query.time_seconds,
                                    raw_value: None,
                                    formatted_value: None,
                                    format: query.format,
                                });
                                continue;
                            }
                        } else {
                            let prev_idx = insert_pos.saturating_sub(1);
                            if prev_idx < waveform_data.time_table.len() {
                                let prev_time = waveform_data.time_table[prev_idx];
                                let time_gap = target_time.saturating_sub(prev_time);

                                // If time gap is too large, return N/A instead of stale value
                                // Calculate adaptive threshold based on actual signal transition spacing
                                if let Some(last_time) = waveform_data.time_table.last() {
                                    // Calculate minimum gap between consecutive transitions
                                    let min_gap = if waveform_data.time_table.len() > 1 {
                                        let mut min = u64::MAX;
                                        for i in 1..waveform_data.time_table.len() {
                                            let gap = waveform_data.time_table[i]
                                                - waveform_data.time_table[i - 1];
                                            if gap > 0 {
                                                min = min.min(gap);
                                            }
                                        }
                                        if min == u64::MAX {
                                            *last_time / 10
                                        } else {
                                            min * 3
                                        } // 3x minimum gap as threshold
                                    } else {
                                        *last_time / 10 // Fallback to 10% for single transition
                                    };

                                    if time_gap > min_gap {
                                        // Gap too large - return N/A
                                        results.push(SignalValueResult {
                                            scope_path: query.scope_path,
                                            variable_name: query.variable_name,
                                            time_seconds: query.time_seconds,
                                            raw_value: None,
                                            formatted_value: None,
                                            format: query.format,
                                        });
                                        continue;
                                    }
                                }
                                prev_idx as u32
                            } else {
                                // No previous data available - return N/A
                                results.push(SignalValueResult {
                                    scope_path: query.scope_path,
                                    variable_name: query.variable_name,
                                    time_seconds: query.time_seconds,
                                    raw_value: None,
                                    formatted_value: None,
                                    format: query.format,
                                });
                                continue;
                            }
                        }
                    }
                };

                // Load signal and get value
                let mut signal_source = match waveform_data.signal_source.lock() {
                    Ok(source) => source,
                    Err(_) => {
                        results.push(SignalValueResult {
                            scope_path: query.scope_path,
                            variable_name: query.variable_name,
                            time_seconds: query.time_seconds,
                            raw_value: Some("Error: Signal source unavailable".to_string()),
                            formatted_value: Some("Error".to_string()),
                            format: query.format,
                        });
                        continue;
                    }
                };
                let loaded_signals =
                    signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);

                match loaded_signals.into_iter().next() {
                    Some((_, signal)) => {
                        if let Some(offset) = signal.get_offset(time_idx) {
                            let value = signal.get_value_at(&offset, 0);

                            // Use the working formatter for all values
                            let formatted = format_non_binary_signal_value(&value);
                            let (raw_value, formatted_value) =
                                (Some(formatted.clone()), Some(formatted));

                            results.push(SignalValueResult {
                                scope_path: query.scope_path,
                                variable_name: query.variable_name,
                                time_seconds: query.time_seconds,
                                raw_value,
                                formatted_value,
                                format: query.format,
                            });
                        } else {
                            results.push(SignalValueResult {
                                scope_path: query.scope_path,
                                variable_name: query.variable_name,
                                time_seconds: query.time_seconds,
                                raw_value: None,
                                formatted_value: None,
                                format: query.format,
                            });
                        }
                    }
                    None => {
                        results.push(SignalValueResult {
                            scope_path: query.scope_path,
                            variable_name: query.variable_name,
                            time_seconds: query.time_seconds,
                            raw_value: None,
                            formatted_value: Some("Signal load failed".to_string()),
                            format: query.format,
                        });
                    }
                }
            }
            None => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    time_seconds: query.time_seconds,
                    raw_value: None,
                    formatted_value: Some("Signal not found".to_string()),
                    format: query.format,
                });
            }
        }
    }

    send_down_msg(
        DownMsg::SignalValues { file_path, results },
        session_id,
        cor_id,
    )
    .await;
}

// Helper function for batch processing - returns results instead of sending
async fn process_signal_value_queries_internal(
    file_path: &str,
    queries: &[SignalValueQuery],
) -> Result<Vec<SignalValueResult>, String> {
    // For waveform files, ensure body is loaded on-demand (VCD, FST, etc)
    if let Err(e) = ensure_waveform_body_loaded(file_path).await {
        return Err(e);
    }

    let store = WAVEFORM_DATA_STORE
        .lock()
        .map_err(|_| "Failed to access waveform data store".to_string())?;

    let waveform_data = store
        .get(file_path)
        .ok_or_else(|| "Waveform file not loaded or signal data not available".to_string())?;

    let mut results = Vec::new();

    for query in queries {
        // Same logic as query_signal_values but collect results instead of sending
        let key = format!("{}|{}", query.scope_path, query.variable_name);
        let signal_ref = match waveform_data.signals.get(&key) {
            Some(&signal_ref) => signal_ref,
            None => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path.clone(),
                    variable_name: query.variable_name.clone(),
                    time_seconds: query.time_seconds,
                    raw_value: Some("Signal not found".to_string()),
                    formatted_value: Some("N/A".to_string()),
                    format: query.format.clone(),
                });
                continue;
            }
        };

        let target_time = match waveform_data.file_format {
            wellen::FileFormat::Vcd => {
                // For VCD: Convert from seconds to VCD native units using stored timescale
                (query.time_seconds / waveform_data.timescale_factor) as u64
            }
            _ => {
                // For other formats, use proper timescale conversion
                (query.time_seconds / waveform_data.timescale_factor) as u64
            }
        };

        let time_idx = match waveform_data.time_table.binary_search(&target_time) {
            Ok(exact_idx) => exact_idx as u32,
            Err(insert_pos) => {
                if insert_pos == 0 || query.time_seconds <= 0.0 {
                    // Query is at time 0 or before any data - check if we have initial values
                    if !waveform_data.time_table.is_empty() && waveform_data.time_table[0] == 0 {
                        0 // Use time index 0 if it exists
                    } else {
                        // No data at time 0, return N/A
                        results.push(SignalValueResult {
                            scope_path: query.scope_path.clone(),
                            variable_name: query.variable_name.clone(),
                            time_seconds: query.time_seconds,
                            raw_value: None,
                            formatted_value: None,
                            format: query.format.clone(),
                        });
                        continue;
                    }
                } else {
                    let prev_idx = insert_pos.saturating_sub(1);
                    if prev_idx < waveform_data.time_table.len() {
                        let prev_time = waveform_data.time_table[prev_idx];
                        let time_gap = target_time.saturating_sub(prev_time);

                        // If time gap is too large, return N/A instead of stale value
                        // Calculate adaptive threshold based on actual signal transition spacing
                        if let Some(last_time) = waveform_data.time_table.last() {
                            // Calculate minimum gap between consecutive transitions
                            let min_gap = if waveform_data.time_table.len() > 1 {
                                let mut min = u64::MAX;
                                for i in 1..waveform_data.time_table.len() {
                                    let gap = waveform_data.time_table[i]
                                        - waveform_data.time_table[i - 1];
                                    if gap > 0 {
                                        min = min.min(gap);
                                    }
                                }
                                if min == u64::MAX {
                                    *last_time / 10
                                } else {
                                    min * 3
                                } // 3x minimum gap as threshold
                            } else {
                                *last_time / 10 // Fallback to 10% for single transition
                            };

                            if time_gap > min_gap {
                                // Gap too large - return N/A
                                results.push(SignalValueResult {
                                    scope_path: query.scope_path.clone(),
                                    variable_name: query.variable_name.clone(),
                                    time_seconds: query.time_seconds,
                                    raw_value: None,
                                    formatted_value: None,
                                    format: query.format.clone(),
                                });
                                continue;
                            }
                        }
                        prev_idx as u32
                    } else {
                        // No previous data available - return N/A
                        results.push(SignalValueResult {
                            scope_path: query.scope_path.clone(),
                            variable_name: query.variable_name.clone(),
                            time_seconds: query.time_seconds,
                            raw_value: None,
                            formatted_value: None,
                            format: query.format.clone(),
                        });
                        continue;
                    }
                }
            }
        };

        let mut signal_source = match waveform_data.signal_source.lock() {
            Ok(source) => source,
            Err(_) => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path.clone(),
                    variable_name: query.variable_name.clone(),
                    time_seconds: query.time_seconds,
                    raw_value: Some("Error: Signal source unavailable".to_string()),
                    formatted_value: Some("Error".to_string()),
                    format: query.format.clone(),
                });
                continue;
            }
        };

        let loaded_signals =
            signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);

        match loaded_signals.into_iter().next() {
            Some((_, signal)) => {
                if let Some(offset) = signal.get_offset(time_idx) {
                    let value = signal.get_value_at(&offset, 0);

                    // Use the working formatter for all values
                    let formatted = format_non_binary_signal_value(&value);
                    let (raw_value, formatted_value) = (Some(formatted.clone()), Some(formatted));

                    results.push(SignalValueResult {
                        scope_path: query.scope_path.clone(),
                        variable_name: query.variable_name.clone(),
                        time_seconds: query.time_seconds,
                        raw_value,
                        formatted_value,
                        format: query.format.clone(),
                    });
                } else {
                    results.push(SignalValueResult {
                        scope_path: query.scope_path.clone(),
                        variable_name: query.variable_name.clone(),
                        time_seconds: query.time_seconds,
                        raw_value: Some("No data".to_string()),
                        formatted_value: Some("X".to_string()),
                        format: query.format.clone(),
                    });
                }
            }
            None => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path.clone(),
                    variable_name: query.variable_name.clone(),
                    time_seconds: query.time_seconds,
                    raw_value: Some("Failed to load signal".to_string()),
                    formatted_value: Some("Error".to_string()),
                    format: query.format.clone(),
                });
            }
        }
    }

    Ok(results)
}

async fn query_signal_transitions(
    file_path: String,
    signal_queries: Vec<SignalTransitionQuery>,
    time_range: (u64, u64),
    session_id: SessionId,
    cor_id: CorId,
) {
    // For waveform files, ensure body is loaded on-demand (VCD, FST, etc)
    if let Err(e) = ensure_waveform_body_loaded(&file_path).await {
        send_down_msg(
            DownMsg::SignalTransitionsError {
                file_path,
                error: e,
            },
            session_id,
            cor_id,
        )
        .await;
        return;
    }

    let store = match WAVEFORM_DATA_STORE.lock() {
        Ok(store) => store,
        Err(_) => {
            send_down_msg(
                DownMsg::SignalTransitionsError {
                    file_path,
                    error: "Internal error: Failed to access waveform data store".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;
            return;
        }
    };
    let waveform_data = match store.get(&file_path) {
        Some(data) => data,
        None => {
            send_down_msg(
                DownMsg::SignalTransitionsError {
                    file_path,
                    error: "Waveform file not loaded or signal data not available".to_string(),
                },
                session_id,
                cor_id,
            )
            .await;
            return;
        }
    };

    let mut results = Vec::new();

    for query in signal_queries {
        let key = format!("{}|{}", query.scope_path, query.variable_name);

        // DEBUG: Log signal lookup to identify key mismatch
        debug_log!(
            DEBUG_BACKEND,
            "🔍 BACKEND: Looking for signal key: '{}'",
            key
        );
        debug_log!(
            DEBUG_BACKEND,
            "🔍 BACKEND: Available keys: {:?}",
            waveform_data.signals.keys().collect::<Vec<_>>()
        );

        // Try multiple key formats to handle scope path variations
        let signal_ref_option = waveform_data
            .signals
            .get(&key)
            .or_else(|| {
                // Try with different scope separators
                let alt_key1 = key.replace(".", "/");
                debug_log!(
                    DEBUG_BACKEND,
                    "🔍 BACKEND: Trying alternative key format: '{}'",
                    alt_key1
                );
                waveform_data.signals.get(&alt_key1)
            })
            .or_else(|| {
                // Try with no scope separator (flat names)
                let parts: Vec<&str> = query.scope_path.split('.').collect();
                if parts.len() > 1 {
                    let flat_key = format!("{}_{}", parts.join("_"), query.variable_name);
                    debug_log!(
                        DEBUG_BACKEND,
                        "🔍 BACKEND: Trying flattened key format: '{}'",
                        flat_key
                    );
                    waveform_data.signals.get(&flat_key)
                } else {
                    None
                }
            })
            .or_else(|| {
                // Try searching for partial matches in case scope path differs
                debug_log!(
                    DEBUG_BACKEND,
                    "🔍 BACKEND: Searching for variable '{}' in any scope",
                    query.variable_name
                );
                waveform_data
                    .signals
                    .iter()
                    .find(|(k, _)| k.ends_with(&format!("|{}", query.variable_name)))
                    .map(|(_, v)| v)
            });

        match signal_ref_option {
            Some(&signal_ref) => {
                let mut transitions = Vec::new();

                // Convert time range from nanoseconds to native file units
                let (mut start_time, mut end_time) = match waveform_data.file_format {
                    wellen::FileFormat::Vcd => {
                        // For VCD: Convert from nanoseconds to VCD native units using stored timescale
                        // time_range is in nanoseconds, convert to seconds then to VCD native units
                        let start_seconds = time_range.0 as f64 / 1_000_000_000.0;
                        let end_seconds = time_range.1 as f64 / 1_000_000_000.0;
                        let start_native = (start_seconds / waveform_data.timescale_factor) as u64;
                        let end_native = (end_seconds / waveform_data.timescale_factor) as u64;
                        (start_native, end_native)
                    }
                    _ => {
                        // For other formats (like FST), use proper timescale conversion
                        let start_seconds = time_range.0 as f64 / 1_000_000_000.0;
                        let end_seconds = time_range.1 as f64 / 1_000_000_000.0;
                        (
                            (start_seconds / waveform_data.timescale_factor) as u64,
                            (end_seconds / waveform_data.timescale_factor) as u64,
                        )
                    }
                };

                // PERFORMANCE OPTIMIZATION: Clamp to actual file bounds to avoid processing non-existent time ranges
                // This prevents scanning millions of non-existent time points when timeline is zoomed out beyond file data
                if !waveform_data.time_table.is_empty() {
                    let file_start = *waveform_data.time_table.first().unwrap();
                    let file_end = *waveform_data.time_table.last().unwrap();
                    start_time = start_time.max(file_start);
                    end_time = end_time.min(file_end);

                    // Clamped query range to file bounds for performance
                }

                // Load signal once for efficiency
                let mut signal_source = match waveform_data.signal_source.lock() {
                    Ok(source) => source,
                    Err(_) => {
                        // Return empty results for this query on mutex error
                        results.push(SignalTransitionResult {
                            scope_path: query.scope_path,
                            variable_name: query.variable_name,
                            transitions: vec![],
                        });
                        continue;
                    }
                };
                let loaded_signals =
                    signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);

                if let Some((_, signal)) = loaded_signals.into_iter().next() {
                    let mut last_value: Option<String> = None;
                    let mut last_transition_time: Option<u64> = None;

                    // Use binary search to find time range bounds - much faster than linear iteration
                    let start_idx = match waveform_data.time_table.binary_search(&start_time) {
                        Ok(exact_idx) => exact_idx,
                        Err(insert_pos) => insert_pos,
                    };

                    let end_idx = match waveform_data.time_table.binary_search(&end_time) {
                        Ok(exact_idx) => exact_idx + 1, // Include exact match
                        Err(insert_pos) => insert_pos,
                    };

                    // PERFORMANCE OPTIMIZATION: Pixel-level decimation for dense signals
                    let transition_count = end_idx - start_idx;
                    let canvas_width = 1200.0; // Approximate canvas width in pixels
                    let max_useful_transitions = (canvas_width * 2.0) as usize; // 2 transitions per pixel maximum

                    let decimation_step = if transition_count > max_useful_transitions {
                        let step = transition_count / max_useful_transitions;
                        // Using decimation for performance optimization
                        step.max(1)
                    } else {
                        1 // No decimation needed
                    };

                    // Only iterate through the relevant time slice, with optional decimation
                    let mut idx = start_idx;
                    while idx < end_idx.min(waveform_data.time_table.len()) {
                        if let Some(&time_val) = waveform_data.time_table.get(idx) {
                            // Convert time to nanoseconds for frontend using proper timescale
                            let time_ns = match waveform_data.file_format {
                                wellen::FileFormat::Vcd => {
                                    // Convert VCD native units to seconds, then to nanoseconds
                                    let time_seconds =
                                        time_val as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                }
                                _ => {
                                    let time_seconds =
                                        time_val as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                }
                            };

                            // Get signal value at this time index
                            if let Some(offset) = signal.get_offset(idx as u32) {
                                let value = signal.get_value_at(&offset, 0);

                                // Convert to string for frontend display using working formatter
                                let value_str = format_non_binary_signal_value(&value);

                                // TRANSITION DETECTION: Only send when value actually changes
                                if last_value.as_ref() != Some(&value_str) {
                                    transitions.push(SignalTransition {
                                        time_ns,
                                        value: value_str.clone(),
                                    });
                                    last_value = Some(value_str);
                                    last_transition_time = Some(time_ns);
                                }
                            }
                        }

                        idx += decimation_step; // Increment by decimation step for performance
                    }

                    // Add filler rectangle: add "0" transition at the actual signal end time
                    // This shows users where signal values end (e.g., A=c and B=5 end at 150s in simple.vcd)
                    if let (Some(last_val), Some(last_time)) = (&last_value, last_transition_time) {
                        if last_val != "0" {
                            // Calculate actual file end time for proper filler timing using proper timescale
                            let file_end_time_ns = match waveform_data.file_format {
                                wellen::FileFormat::Vcd => {
                                    // Convert VCD native units to seconds, then to nanoseconds
                                    let time_seconds =
                                        end_time as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                }
                                _ => {
                                    let time_seconds =
                                        end_time as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                }
                            };

                            // Add "0" filler at actual signal end time (not viewing window end)
                            if last_time < file_end_time_ns {
                                transitions.push(SignalTransition {
                                    time_ns: file_end_time_ns,
                                    value: "0".to_string(),
                                });
                            }
                        }
                    }
                }

                results.push(SignalTransitionResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    transitions,
                });
            }
            None => {
                results.push(SignalTransitionResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    transitions: vec![],
                });
            }
        }
    }

    send_down_msg(
        DownMsg::SignalTransitions { file_path, results },
        session_id,
        cor_id,
    )
    .await;
}

/// Get raw binary string from wellen::SignalValue for frontend formatting
/// Uses wellen's built-in to_bit_string() method which should return binary format
fn format_non_binary_signal_value(value: &wellen::SignalValue) -> String {
    match value {
        wellen::SignalValue::Binary(_bits, width) => {
            if *width == 1 {
                // Handle single bit case with simple 0/1 logic
                match value.to_bit_string() {
                    Some(bit_str) => {
                        if bit_str == "1" || bit_str == "0" {
                            bit_str
                        } else {
                            "X".to_string() // Unknown state
                        }
                    }
                    None => "X".to_string(),
                }
            } else {
                // For multi-bit values, use wellen's to_bit_string() directly
                let bit_string = value.to_bit_string().unwrap_or_else(|| "?".to_string());

                // 🐛 DEBUG: Log what wellen actually returns vs what we expect
                debug_log!(
                    DEBUG_BACKEND,
                    "🔍 WELLEN ACTUAL: width={}, to_bit_string()='{}' (expecting binary like '1100')",
                    width,
                    bit_string
                );

                bit_string
            }
        }
        wellen::SignalValue::FourValue(_bits, width) => {
            if *width == 1 {
                match value.to_bit_string() {
                    Some(bit_str) => {
                        // Should return 0, 1, X, or Z for single bit
                        bit_str
                    }
                    None => "X".to_string(),
                }
            } else {
                // Multi-bit FourValue (can include X and Z states)
                let bit_string = value.to_bit_string().unwrap_or_else(|| "?".to_string());

                // 🐛 DEBUG: Log FourValue output too
                debug_log!(
                    DEBUG_BACKEND,
                    "🔍 WELLEN FOURVALUE: width={}, to_bit_string()='{}' (expecting binary)",
                    width,
                    bit_string
                );

                bit_string
            }
        }
        wellen::SignalValue::NineValue(_bits, _width) => {
            value.to_bit_string().unwrap_or_else(|| "?".to_string())
        }
        wellen::SignalValue::String(s) => s.to_string(),
        wellen::SignalValue::Real(f) => format!("{:.6}", f),
    }
}

/// Handle unified signal query using the new cache manager
async fn handle_unified_signal_query(
    signal_requests: Vec<UnifiedSignalRequest>,
    cursor_time_ns: Option<u64>,
    request_id: String,
    session_id: SessionId,
    cor_id: CorId,
) {
    debug_log!(
        DEBUG_BACKEND,
        "🔍 BACKEND: About to call SIGNAL_CACHE_MANAGER.query_unified_signals for request_id: {}",
        request_id
    );
    match SIGNAL_CACHE_MANAGER
        .query_unified_signals(signal_requests, cursor_time_ns)
        .await
    {
        Ok((signal_data, cursor_values, statistics)) => {
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: SIGNAL_CACHE_MANAGER success - {} signal_data items, {} cursor_values",
                signal_data.len(),
                cursor_values.len()
            );
            let resp_len = signal_data.len();
            let cursor_len = cursor_values.len();
            send_down_msg(
                DownMsg::UnifiedSignalResponse {
                    request_id,
                    signal_data,
                    cursor_values,
                    cached_time_range_ns: None, // Cache time range would be computed from signal data bounds
                    statistics: Some(statistics),
                },
                session_id,
                cor_id,
            )
            .await;
            debug_log!(
                DEBUG_BACKEND,
                "🛰️ BACKEND: UnifiedSignalResponse sent: signal_data={} cursor_values={}",
                resp_len,
                cursor_len
            );
        }
        Err(error) => {
            debug_log!(
                DEBUG_BACKEND,
                "🔍 BACKEND: SIGNAL_CACHE_MANAGER error: {}",
                error
            );
            send_down_msg(
                DownMsg::UnifiedSignalError { request_id, error },
                session_id,
                cor_id,
            )
            .await;
        }
    }
}

#[moon::main]
async fn main() -> std::io::Result<()> {
    // Set panic hook to log panics succinctly and throttle duplicates
    static PANIC_THROTTLE: Lazy<Mutex<HashMap<String, Instant>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));

    if let Ok(dir) = std::env::current_dir() {
        println!("🔍 BACKEND cwd = {}", dir.display());
    }

    std::panic::set_hook(Box::new(|panic_info| {
        let loc_str = if let Some(loc) = panic_info.location() {
            format!("{}:{}", loc.file(), loc.line())
        } else {
            "<unknown>".to_string()
        };
        let now = Instant::now();
        let mut guard = PANIC_THROTTLE.lock().unwrap();
        let should_log = match guard.get(&loc_str) {
            Some(prev) => now.duration_since(*prev) > std::time::Duration::from_secs(30),
            None => true,
        };
        if should_log {
            guard.insert(loc_str.clone(), now);
            eprintln!("🔌 BACKEND: panic at {} (details throttled)", loc_str);
        }
    }));

    start(frontend, up_msg_handler, |_error| {
        // Error logging removed to reduce log spam
    })
    .await
}
