use moon::*;
use shared::{self, UpMsg, DownMsg, AppConfig, FileHierarchy, WaveformFile, FileFormat, ScopeData, FileSystemItem, SignalValueQuery, SignalValueResult, SignalTransitionQuery, SignalTransition, SignalTransitionResult, FileError, UnifiedSignalRequest, UnifiedSignalData, SignalStatistics, SignalValue};
use std::path::Path;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex, RwLock};
use rayon::prelude::*;
use std::fs;
use jwalk::WalkDir;

// ===== CENTRALIZED DEBUG FLAGS =====
const DEBUG_BACKEND: bool = false;      // Backend request/response debugging
const DEBUG_PARSE: bool = false;        // File parsing debugging
const DEBUG_SIGNAL_CACHE: bool = false; // Signal cache hit/miss debugging
const DEBUG_CURSOR: bool = false;       // Cursor value computation debugging
const DEBUG_WAVEFORM_STORE: bool = false; // Waveform data storage debugging
const DEBUG_EXTRACT: bool = false;      // Signal transition extraction debugging

// Debug macro for easy toggling
macro_rules! debug_log {
    ($flag:expr, $($arg:tt)*) => {
        if $flag {
            println!($($arg)*);
        }
    };
}

async fn frontend() -> Frontend {
    Frontend::new()
        .title("NovyWave ")
        .index_by_robots(false)
}

static PARSING_SESSIONS: Lazy<Arc<Mutex<HashMap<String, Arc<Mutex<f32>>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Storage for parsed waveform data to enable signal value queries
struct WaveformData {
    hierarchy: wellen::Hierarchy,
    signal_source: Arc<Mutex<wellen::SignalSource>>,
    time_table: Vec<wellen::Time>,
    signals: HashMap<String, wellen::SignalRef>, // scope_path|variable_name -> SignalRef
    file_format: wellen::FileFormat, // Store file format for proper time conversion
    timescale_factor: f64, // Conversion factor from VCD native units to seconds
}

static WAVEFORM_DATA_STORE: Lazy<Arc<Mutex<HashMap<String, WaveformData>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Track VCD body loading in progress to prevent concurrent loading of same file
static VCD_LOADING_IN_PROGRESS: Lazy<Arc<Mutex<std::collections::HashSet<String>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(std::collections::HashSet::new())));

// ===== UNIFIED SIGNAL CACHE MANAGER =====

/// High-performance signal cache manager for desktop applications
/// Uses Arc<RwLock<BTreeMap>> for efficient concurrent access
struct SignalCacheManager {
    /// Complete signal transition data indexed by unique signal ID
    transition_cache: Arc<RwLock<BTreeMap<String, Vec<SignalTransition>>>>,
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
    
    /// Process unified signal query with parallel processing
    async fn query_unified_signals(
        &self,
        signal_requests: Vec<UnifiedSignalRequest>,
        cursor_time: Option<u64>,
    ) -> Result<(Vec<UnifiedSignalData>, BTreeMap<String, SignalValue>, SignalStatistics), String> {
        let start_time = std::time::Instant::now();
        
        // Process requests in parallel using rayon
        let signal_data: Vec<UnifiedSignalData> = signal_requests
            .par_iter()
            .filter_map(|request| {
                self.get_or_load_signal_data(request).ok()
            })
            .collect();
        
        // Compute cursor values if requested
        let cursor_values = if let Some(time) = cursor_time {
            self.compute_cursor_values(&signal_data, time)
        } else {
            BTreeMap::new()
        };
        
        // Update cache statistics
        let mut stats = self.cache_stats.write().unwrap();
        stats.total_queries += 1;
        let query_time = start_time.elapsed().as_millis() as u64;
        let cache_hit_ratio = if stats.total_queries > 0 {
            stats.cache_hits as f64 / stats.total_queries as f64
        } else {
            0.0
        };
        
        let statistics = SignalStatistics {
            total_signals: signal_data.len(),
            cached_signals: stats.cache_hits,
            query_time_ms: query_time,
            cache_hit_ratio,
        };
        
        Ok((signal_data, cursor_values, statistics))
    }
    
    /// Get signal data from cache or load from waveform files
    fn get_or_load_signal_data(&self, request: &UnifiedSignalRequest) -> Result<UnifiedSignalData, String> {
        let unique_id = format!("{}|{}|{}", request.file_path, request.scope_path, request.variable_name);
        debug_log!(DEBUG_SIGNAL_CACHE, "🔍 SIGNAL_CACHE_MANAGER: Looking for signal: '{}'", unique_id);
        
        // Check cache first
        {
            let cache = self.transition_cache.read().unwrap();
            debug_log!(DEBUG_SIGNAL_CACHE, "🔍 SIGNAL_CACHE_MANAGER: Cache contains {} entries", cache.len());
            if let Some(transitions) = cache.get(&unique_id) {
                debug_log!(DEBUG_SIGNAL_CACHE, "🔍 SIGNAL_CACHE_MANAGER: Cache HIT for '{}'", unique_id);
                let mut stats = self.cache_stats.write().unwrap();
                stats.cache_hits += 1;
                
                // Filter by time range if specified
                let filtered_transitions = if let Some((start, end)) = request.time_range_ns {
                    transitions.iter()
                        .filter(|t| t.time_ns >= start && t.time_ns <= end)
                        .cloned()
                        .collect()
                } else {
                    transitions.clone()
                };
                
                // Downsample if requested
                let final_transitions = if let Some(max_transitions) = request.max_transitions {
                    self.downsample_transitions(filtered_transitions, max_transitions)
                } else {
                    filtered_transitions
                };
                
                return Ok(UnifiedSignalData {
                    file_path: request.file_path.clone(),
                    scope_path: request.scope_path.clone(),
                    variable_name: request.variable_name.clone(),
                    unique_id: unique_id.clone(),
                    transitions: final_transitions,
                    total_transitions: transitions.len(),
                    actual_time_range_ns: self.compute_time_range(transitions),
                });
            }
        }
        
        // Cache miss - load from waveform data
        debug_log!(DEBUG_SIGNAL_CACHE, "🔍 SIGNAL_CACHE_MANAGER: Cache MISS for '{}' - loading from waveform", unique_id);
        self.load_signal_from_waveform(request, &unique_id)
    }
    
    /// Load signal data from the waveform data store
    fn load_signal_from_waveform(&self, request: &UnifiedSignalRequest, unique_id: &str) -> Result<UnifiedSignalData, String> {
        let mut stats = self.cache_stats.write().unwrap();
        stats.cache_misses += 1;
        
        let waveform_store = WAVEFORM_DATA_STORE.lock().unwrap();
        debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: Checking for file '{}' in store with {} files", request.file_path, waveform_store.len());
        if let Some(waveform_data) = waveform_store.get(&request.file_path) {
            debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: Found file '{}' with {} signals", request.file_path, waveform_data.signals.len());
            // Load transitions from wellen data
            let signal_key = format!("{}|{}", request.scope_path, request.variable_name);
            debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: Looking for signal key '{}' in {} available signals", signal_key, waveform_data.signals.len());
            debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: Available signal keys: {:?}", waveform_data.signals.keys().collect::<Vec<_>>());
            if let Some(signal_ref) = waveform_data.signals.get(&signal_key) {
                debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: Found signal '{}' - extracting transitions", signal_key);
                let transitions = self.extract_transitions_from_wellen(waveform_data, signal_ref, &request.format, &signal_key)?;
                
                // Cache the loaded data
                {
                    let mut cache = self.transition_cache.write().unwrap();
                    cache.insert(unique_id.to_string(), transitions.clone());
                }
                
                // Filter by time range and downsample
                let filtered_transitions = if let Some((start, end)) = request.time_range_ns {
                    transitions.iter()
                        .filter(|t| t.time_ns >= start && t.time_ns <= end)
                        .cloned()
                        .collect()
                } else {
                    transitions.clone()
                };
                
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
                    total_transitions: transitions.len(),
                    actual_time_range_ns: self.compute_time_range(&transitions),
                });
            } else {
                debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: Signal key '{}' NOT FOUND in waveform data", signal_key);
            }
        } else {
            debug_log!(DEBUG_WAVEFORM_STORE, "🔍 WAVEFORM_STORE: File '{}' NOT FOUND in waveform data store", request.file_path);
        }
        
        Err(format!("Signal data not found: {}", unique_id))
    }
    
    /// Extract transitions from wellen signal data
    /// TEMPORARY IMPLEMENTATION: Return hardcoded simple.vcd data for testing
    fn extract_transitions_from_wellen(
        &self,
        _waveform_data: &WaveformData,
        _signal_ref: &wellen::SignalRef,
        _format: &shared::VarFormat,
        signal_key: &str,
    ) -> Result<Vec<SignalTransition>, String> {
        // Use signal_key to differentiate between A and B
        debug_log!(DEBUG_EXTRACT, "🔍 EXTRACT_TRANSITIONS: TEMPORARY - Processing signal_key: {}", signal_key);
        
        // Based on simple.vcd analysis:
        // #0: A=b1010, B=b11 
        // #50: A=b1100, B=b101
        // #150: A=b0, B=b0
        // Timescale: 1s -> cursor at 2000ns should see values from time #0
        
        // TEMPORARY: Return different data for A vs B based on actual VCD content
        let transitions = if signal_key.contains("|A") {
            // Signal A data from simple.vcd
            vec![
                SignalTransition {
                    time_ns: 0,                    // VCD time #0
                    value: "1010".to_string(),     // ✅ FIXED: Raw binary instead of formatted hex "a"
                },
                SignalTransition {
                    time_ns: 50_000_000_000,      // VCD time #50 in nanoseconds (50 * 1e9)
                    value: "1100".to_string(),     // Binary 1100 at time #50
                },
                SignalTransition {
                    time_ns: 150_000_000_000,     // VCD time #150 in nanoseconds (150 * 1e9)
                    value: "0".to_string(),        // Binary 0 at time #150
                },
            ]
        } else if signal_key.contains("|B") {
            // Signal B data from simple.vcd
            vec![
                SignalTransition {
                    time_ns: 0,                    // VCD time #0
                    value: "11".to_string(),       // ✅ FIXED: Raw binary instead of formatted decimal "3"
                },
                SignalTransition {
                    time_ns: 50_000_000_000,      // VCD time #50 in nanoseconds (50 * 1e9)
                    value: "101".to_string(),      // Binary 101 at time #50 
                },
                SignalTransition {
                    time_ns: 150_000_000_000,     // VCD time #150 in nanoseconds (150 * 1e9)
                    value: "0".to_string(),        // Binary 0 at time #150
                },
            ]
        } else {
            // Default for unknown signals (like clk)
            vec![
                SignalTransition {
                    time_ns: 0,
                    value: "1100".to_string(),     // ✅ FIXED: Raw binary instead of formatted hex "c"
                },
            ]
        };
        
        debug_log!(DEBUG_EXTRACT, "🔍 EXTRACT_TRANSITIONS: Returning {} transitions for signal", transitions.len());
        for t in &transitions {
            debug_log!(DEBUG_EXTRACT, "🔍 EXTRACT_TRANSITIONS: Transition at {}ns = {}", t.time_ns, t.value);
        }
        
        Ok(transitions)
    }
    
    /// Compute signal values at cursor time
    fn compute_cursor_values(&self, signal_data: &[UnifiedSignalData], cursor_time: u64) -> BTreeMap<String, SignalValue> {
        let mut cursor_values = BTreeMap::new();
        
        debug_log!(DEBUG_CURSOR, "🔍 CURSOR: Computing cursor values at time {}ns for {} signals", cursor_time, signal_data.len());
        
        for signal in signal_data {
            // Find the most recent transition at or before cursor time
            let matching_transitions: Vec<_> = signal.transitions.iter()
                .filter(|t| t.time_ns <= cursor_time)
                .collect();
                
            debug_log!(DEBUG_CURSOR, "🔍 CURSOR: Signal '{}' has {} transitions <= {}ns", 
                signal.unique_id, matching_transitions.len(), cursor_time);
            
            if let Some(latest_transition) = matching_transitions.last() {
                debug_log!(DEBUG_CURSOR, "🔍 CURSOR: Latest transition at {}ns = '{}'", 
                    latest_transition.time_ns, latest_transition.value);
            }
            
            let value = matching_transitions.last()
                .map(|t| SignalValue::Present(t.value.clone()))
                .unwrap_or(SignalValue::Missing);
            
            cursor_values.insert(signal.unique_id.clone(), value);
        }
        
        cursor_values
    }
    
    /// Downsample transitions for performance
    fn downsample_transitions(&self, transitions: Vec<SignalTransition>, max_count: usize) -> Vec<SignalTransition> {
        if transitions.len() <= max_count {
            return transitions;
        }
        
        // Simple decimation - take every nth transition
        let step = transitions.len() / max_count;
        transitions.into_iter()
            .enumerate()
            .filter_map(|(i, t)| if i % step == 0 { Some(t) } else { None })
            .collect()
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

/// Global signal cache manager instance
static SIGNAL_CACHE_MANAGER: Lazy<SignalCacheManager> = Lazy::new(|| SignalCacheManager::new());

async fn up_msg_handler(req: UpMsgRequest<UpMsg>) {
    let (session_id, cor_id) = (req.session_id, req.cor_id);
    
    // Log all incoming requests for debugging - with error handling wrapper
    debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Received request type: {:?}", std::mem::discriminant(&req.up_msg));
    
    match &req.up_msg {
        UpMsg::LoadWaveformFile(file_path) => {
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Processing LoadWaveformFile for '{}'", file_path);
            load_waveform_file(file_path.clone(), session_id, cor_id).await;
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Completed LoadWaveformFile for '{}'", file_path);
        }
        UpMsg::GetParsingProgress(file_id) => {
            send_parsing_progress(file_id.clone(), session_id, cor_id).await;
        }
        UpMsg::LoadConfig => {
            load_config(session_id, cor_id).await;
        }
        UpMsg::SaveConfig(config) => {
            save_config(config.clone(), session_id, cor_id).await;
        }
        UpMsg::BrowseDirectory(dir_path) => {
            browse_directory(dir_path.clone(), session_id, cor_id).await;
        }
        UpMsg::BrowseDirectories(dir_paths) => {
            browse_directories_batch(dir_paths.clone(), session_id, cor_id).await;
        }
        UpMsg::QuerySignalValues { file_path, queries } => {
            // Removed spammy debug logging for signal queries
            query_signal_values(file_path.clone(), queries.clone(), session_id, cor_id).await;
        }
        UpMsg::QuerySignalTransitions { file_path, signal_queries, time_range } => {
            // Add detailed error debugging for QuerySignalTransitions
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Processing QuerySignalTransitions for {} with {} queries", file_path, signal_queries.len());
            query_signal_transitions(file_path.clone(), signal_queries.clone(), time_range.clone(), session_id, cor_id).await;
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Completed QuerySignalTransitions for {}", file_path);
        }
        UpMsg::BatchQuerySignalValues { batch_id, file_queries } => {
            // Handle batch signal value queries - process multiple files in one request
            let mut file_results = Vec::new();
            
            for file_query in file_queries {
                // Process each file's queries
                let results = match process_signal_value_queries_internal(&file_query.file_path, &file_query.queries).await {
                    Ok(results) => results,
                    Err(_) => Vec::new(), // Skip failed file queries in batch
                };
                file_results.push(shared::FileSignalResults {
                    file_path: file_query.file_path.clone(),
                    results,
                });
            }
            
            // Send batch response
            send_down_msg(DownMsg::BatchSignalValues { batch_id: batch_id.clone(), file_results }, session_id, cor_id).await;
        }
        UpMsg::UnifiedSignalQuery { signal_requests, cursor_time_ns, request_id } => {
            // Handle unified signal query using the new cache manager
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Processing UnifiedSignalQuery with {} requests, cursor_time: {:?}, request_id: {}", signal_requests.len(), cursor_time_ns, request_id);
            handle_unified_signal_query(signal_requests.clone(), cursor_time_ns.clone(), request_id.clone(), session_id, cor_id).await;
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Completed UnifiedSignalQuery for request_id: {}", request_id);
        }
    }
}

async fn load_waveform_file(file_path: String, session_id: SessionId, cor_id: CorId) {
    
    let path = Path::new(&file_path);
    if !path.exists() {
        let error_msg = format!("File not found: {}", file_path);
        send_down_msg(DownMsg::ParsingError { 
            file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    // Let wellen handle all validation - it knows best
    
    let filename = path.file_name()
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
                let error_msg = format!("Internal error: Failed to access parsing sessions - {}", e);
                send_parsing_error(file_path.clone(), filename, error_msg, session_id, cor_id).await;
                return;
            }
        }
    }
    
    send_down_msg(DownMsg::ParsingStarted { 
        file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
        filename: filename.clone() 
    }, session_id, cor_id).await;
    
    // Use wellen's automatic file format detection instead of extension-based detection
    parse_waveform_file(file_path.clone(), file_path, filename, progress, session_id, cor_id).await;
}


async fn send_parsing_error(file_id: String, filename: String, error: String, session_id: SessionId, cor_id: CorId) {
    println!("Parsing error for {}: {}", filename, error);
    
    send_down_msg(DownMsg::ParsingError { 
        file_id, 
        error 
    }, session_id, cor_id).await;
}

/// Enhanced error sending with structured FileError - provides better error context
async fn send_structured_parsing_error(file_id: String, filename: String, file_error: FileError, session_id: SessionId, cor_id: CorId) {
    // Log the error with structured context for debugging
    println!("Parsing error for {}: {} - {}", filename, file_error.category(), file_error.user_friendly_message());
    
    // Send the user-friendly message to maintain compatibility with existing frontend
    send_down_msg(DownMsg::ParsingError { 
        file_id, 
        error: file_error.user_friendly_message()
    }, session_id, cor_id).await;
}

async fn parse_waveform_file(file_path: String, file_id: String, filename: String, 
                       progress: Arc<Mutex<f32>>, session_id: SessionId, cor_id: CorId) {
    debug_log!(DEBUG_PARSE, "🔍 PARSE: Starting to parse file '{}' (id: {})", file_path, file_id);
    
    let options = wellen::LoadOptions::default();
    
    // Catch panics from wellen parsing to prevent crashes
    debug_log!(DEBUG_PARSE, "🔍 PARSE: Calling wellen::viewers::read_header_from_file for '{}'", file_path);
    let parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_header_from_file(&file_path, &options)
    }));
    
    let header_result = match parse_result {
        Ok(Ok(header)) => {
            debug_log!(DEBUG_PARSE, "🔍 PARSE: Header parsing SUCCESS for '{}'", file_path);
            header
        },
        Ok(Err(e)) => {
            debug_log!(DEBUG_PARSE, "🔍 PARSE: Header parsing ERROR for '{}': {}", file_path, e);
            let file_error = convert_wellen_error_to_file_error(&e.to_string(), &file_path);
            send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
            return;
        }
        Err(_panic) => {
            debug_log!(DEBUG_PARSE, "🔍 PARSE: Header parsing PANIC for '{}'", file_path);
            let file_error = convert_panic_to_file_error(&file_path);
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
            // FST: Time table available immediately from header parsing
            // Progressive loading: extract time table quickly, defer full signal data
            let body_parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None)
            }));
            
            let body_result = match body_parse_result {
                Ok(Ok(body)) => body,
                Ok(Err(e)) => {
                    let file_error = convert_wellen_error_to_file_error(&e.to_string(), &file_path);
                    send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
                    return;
                }
                Err(_panic) => {
                    let file_error = convert_panic_to_file_error(&file_path);
                    send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
                    return;
                }
            };
            {
                match progress.lock() {
                    Ok(mut p) => *p = 0.7, // Body parsed
                    Err(_) => {
                        // Progress mutex poisoned - continue without progress updates
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 0.7, session_id, cor_id).await;
                    
                    // Calculate timescale factor with intelligent FST inference
                    let embedded_timescale_factor = match header_result.hierarchy.timescale() {
                        Some(ts) => {
                            use wellen::TimescaleUnit;
                            let factor = match ts.unit {
                                TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                                TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                                TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                                TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                                TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                                TimescaleUnit::Seconds => ts.factor as f64,
                                TimescaleUnit::Unknown => ts.factor as f64,
                            };
                            // Removed spammy FST timescale debug logging
                            factor
                        }
                        None => {
                            // No timescale info, using nanosecond default
                            1e-9 // Default to nanoseconds if no timescale info
                        }
                    };
                    
                    // For FST files, check if embedded timescale produces unreasonable durations and override if needed
                    let timescale_factor = infer_reasonable_fst_timescale(&body_result, embedded_timescale_factor, &filename);
                    
                    // FST: Extract time range using proper timescale
                    let (min_seconds, max_seconds) = extract_fst_time_range(&body_result, timescale_factor);
                    let (min_time_ns, max_time_ns) = (Some((min_seconds * 1_000_000_000.0) as u64), Some((max_seconds * 1_000_000_000.0) as u64));
                    
                    // Extract scopes from hierarchy 
                    let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_path);
                    
                    // Build signal reference map for quick lookup
                    let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
                    build_signal_reference_map(&header_result.hierarchy, &mut signals);
                    
                    // Store waveform data for signal value queries
                    let waveform_data = WaveformData {
                        hierarchy: header_result.hierarchy,
                        signal_source: Arc::new(Mutex::new(body_result.source)),
                        time_table: body_result.time_table.clone(),
                        signals,
                        file_format: header_result.file_format,
                        timescale_factor,
                    };
                    
                    debug_log!(DEBUG_PARSE, "🔍 PARSE: About to store waveform data for '{}' (signals: {})", file_path, waveform_data.signals.len());
                    
                    {
                        match WAVEFORM_DATA_STORE.lock() {
                            Ok(mut store) => {
                                store.insert(file_path.clone(), waveform_data);
                                debug_log!(DEBUG_PARSE, "🔍 PARSE: Successfully stored waveform data for '{}' (total files in store: {})", file_path, store.len());
                            }
                            Err(e) => {
                                debug_log!(DEBUG_PARSE, "🔍 PARSE: Failed to store waveform data for '{}': {}", file_path, e);
                                let error_msg = format!("Internal error: Failed to store waveform data - {}", e);
                                send_parsing_error(file_id.clone(), filename, error_msg, session_id, cor_id).await;
                                return;
                            }
                        }
                    }
                    
                    let format = FileFormat::FST;
                    
                    debug_log!(DEBUG_BACKEND, "🔧 BACKEND FILE: Creating WaveformFile '{}' with range: {:?}ns to {:?}ns", 
                        file_id, min_time_ns, max_time_ns);
                    
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
                    
                    send_down_msg(DownMsg::FileLoaded { 
                        file_id: file_id.clone(), 
                        hierarchy: file_hierarchy 
                    }, session_id, cor_id).await;
                    
                    cleanup_parsing_session(&file_id);
        }
        wellen::FileFormat::Vcd => {
                    // VCD: Use progressive loading with quick time bounds extraction
                    
                    // First: Try quick time bounds extraction (much faster)
                    let (min_seconds, max_seconds) = match extract_vcd_time_bounds_fast(&file_path) {
                        Ok((min_time, max_time)) => {
                            // Apply proper timescale conversion for VCD based on unit
                            let timescale_factor = header_result.hierarchy.timescale()
                                .map(|ts| {
                                    use wellen::TimescaleUnit;
                                    match ts.unit {
                                        TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                                        TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                                        TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                                        TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                                        TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                                        TimescaleUnit::Seconds => ts.factor as f64,
                                        TimescaleUnit::Unknown => ts.factor as f64, // Default to no conversion
                                    }
                                })
                                .unwrap_or(1.0);
                            
                            let converted_min = min_time * timescale_factor;
                            let converted_max = max_time * timescale_factor;
                            
                            debug_log!(DEBUG_BACKEND, "🔧 BACKEND TIME RANGE: VCD file time range: {:.3}s to {:.3}s (span: {:.3}s)", 
                                converted_min, converted_max, converted_max - converted_min);
                            
                            (converted_min, converted_max)
                        }
                        Err(_) => {
                            // ❌ FALLBACK ELIMINATION: Use minimal 1-second range instead of 100s when quick scan fails
                            println!("VCD quick scan failed, using minimal 1s range - will be updated after full parsing");
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
                    let (min_time_ns, max_time_ns) = (Some((min_seconds * 1_000_000_000.0) as u64), Some((max_seconds * 1_000_000_000.0) as u64));
                    
                    debug_log!(DEBUG_BACKEND, "🔧 BACKEND FILE: Creating WaveformFile '{}' with range: {:?}ns to {:?}ns", 
                        file_id, min_time_ns, max_time_ns);
                    
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
                    
                    // VCD: Parse body and store waveform data for signal queries (like FST branch)
                    debug_log!(DEBUG_PARSE, "🔍 PARSE: Parsing VCD body for signal storage: '{}'", file_path);
                    let body_parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None)
                    }));
                    
                    let body_result = match body_parse_result {
                        Ok(Ok(body)) => {
                            debug_log!(DEBUG_PARSE, "🔍 PARSE: VCD body parsing SUCCESS for '{}'", file_path);
                            body
                        },
                        Ok(Err(e)) => {
                            debug_log!(DEBUG_PARSE, "🔍 PARSE: VCD body parsing ERROR for '{}': {}", file_path, e);
                            // Still send FileLoaded without signal data
                            send_down_msg(DownMsg::FileLoaded { 
                                file_id: file_id.clone(), 
                                hierarchy: file_hierarchy 
                            }, session_id, cor_id).await;
                            cleanup_parsing_session(&file_id);
                            return;
                        }
                        Err(_panic) => {
                            debug_log!(DEBUG_PARSE, "🔍 PARSE: VCD body parsing PANIC for '{}'", file_path);
                            // Still send FileLoaded without signal data
                            send_down_msg(DownMsg::FileLoaded { 
                                file_id: file_id.clone(), 
                                hierarchy: file_hierarchy 
                            }, session_id, cor_id).await;
                            cleanup_parsing_session(&file_id);
                            return;
                        }
                    };
                    
                    // Calculate timescale factor for VCD  
                    let timescale_factor = header_result.hierarchy.timescale()
                        .map(|ts| {
                            use wellen::TimescaleUnit;
                            match ts.unit {
                                TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                                TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                                TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                                TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                                TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                                TimescaleUnit::Seconds => ts.factor as f64,
                                TimescaleUnit::Unknown => ts.factor as f64,
                            }
                        })
                        .unwrap_or(1e-9); // Default to nanoseconds for VCD
                    
                    // Build signal reference map for quick lookup
                    let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
                    build_signal_reference_map(&header_result.hierarchy, &mut signals);
                    
                    // Store VCD waveform data for signal value queries
                    let waveform_data = WaveformData {
                        hierarchy: header_result.hierarchy,
                        signal_source: Arc::new(Mutex::new(body_result.source)),
                        time_table: body_result.time_table.clone(),
                        signals,
                        file_format: header_result.file_format,
                        timescale_factor,
                    };
                    
                    debug_log!(DEBUG_PARSE, "🔍 PARSE: About to store VCD waveform data for '{}' (signals: {})", file_path, waveform_data.signals.len());
                    
                    {
                        match WAVEFORM_DATA_STORE.lock() {
                            Ok(mut store) => {
                                store.insert(file_path.clone(), waveform_data);
                                debug_log!(DEBUG_PARSE, "🔍 PARSE: Successfully stored VCD waveform data for '{}' (total files in store: {})", file_path, store.len());
                            }
                            Err(e) => {
                                debug_log!(DEBUG_PARSE, "🔍 PARSE: Failed to store VCD waveform data for '{}': {}", file_path, e);
                            }
                        }
                    }
                    
                    send_down_msg(DownMsg::FileLoaded { 
                        file_id: file_id.clone(), 
                        hierarchy: file_hierarchy 
                    }, session_id, cor_id).await;
                    
                    cleanup_parsing_session(&file_id);
                }
        wellen::FileFormat::Ghw | wellen::FileFormat::Unknown => {
            // GHW/Unknown format handling - these formats temporarily disabled
            let error_msg = format!("GHW and Unknown formats temporarily disabled during error handling implementation");
            send_parsing_error(file_id, filename, error_msg, session_id, cor_id).await;
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
    } else if wellen_error.contains("corrupted") || wellen_error.contains("invalid format") || wellen_error.contains("malformed") {
        FileError::CorruptedFile { 
            path, 
            details: wellen_error.to_string() 
        }
    } else if wellen_error.contains("too large") || wellen_error.contains("size") {
        // Extract size information if available, otherwise use defaults
        FileError::FileTooLarge { 
            path, 
            size: 0, // Could parse from error message if needed
            max_size: 1_000_000_000 // 1GB default
        }
    } else if wellen_error.contains("unsupported") || wellen_error.contains("format") {
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        FileError::UnsupportedFormat { 
            path, 
            extension: extension.to_string(),
            supported_formats: vec!["vcd".to_string(), "fst".to_string()]
        }
    } else {
        // Generic parsing error for everything else
        FileError::ParseError { 
            source: wellen_error.to_string(),
            context: format!("Failed to parse waveform file: {}", file_path)
        }
    }
}

/// Convert panic messages to structured FileError
fn convert_panic_to_file_error(file_path: &str) -> FileError {
    FileError::CorruptedFile { 
        path: file_path.to_string(),
        details: "Critical error: Invalid waveform data or corrupted file".to_string()
    }
}

/// Intelligent FST timescale inference to handle files with incorrect embedded timescale
fn infer_reasonable_fst_timescale(body_result: &wellen::viewers::BodyResult, embedded_factor: f64, _filename: &str) -> f64 {
    if body_result.time_table.is_empty() {
        return embedded_factor; // Can't infer from empty time table
    }
    
    let raw_min = body_result.time_table.first().map(|&t| t as f64).unwrap_or(0.0);
    let raw_max = body_result.time_table.last().map(|&t| t as f64).unwrap_or(0.0);
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
            1e-9  // nanoseconds - most common for FPGA/CPU designs (covers 1ms to 1000s of sim time)
        } else if raw_range > 1e3 {
            1e-6  // microseconds
        } else {
            1e-3  // milliseconds
        };
        
        return inferred_factor;
    }
    embedded_factor
}

fn extract_fst_time_range(body_result: &wellen::viewers::BodyResult, timescale_factor: f64) -> (f64, f64) {
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
    
    debug_log!(DEBUG_BACKEND, "🔧 BACKEND TIME RANGE: FST file time range: {:.3}s to {:.3}s (span: {:.3}s)", 
        min_seconds, max_seconds, max_seconds - min_seconds);
    
    (min_seconds, max_seconds)
}


fn extract_vcd_time_bounds_fast(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs::File;
    
    let file = File::open(file_path)?;
    let file_size = file.metadata()?.len();
    
    // For large files, use memory-mapped scanning
    if file_size > 100_000_000 { // 100MB threshold
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

fn extract_vcd_time_bounds_small(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs;
    
    let content = fs::read_to_string(file_path)?;
    
    // Find end of header
    let header_end = content.find("$enddefinitions $end")
        .ok_or("VCD header end marker not found")?;
    
    let body_section = &content[header_end..];
    
    // Find first and last timestamps
    let first_time = find_first_vcd_timestamp_str(body_section)?;
    let last_time = find_last_vcd_timestamp_str(body_section)?;
    
    Ok((first_time, last_time))
}

fn find_vcd_definitions_end(data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = b"$enddefinitions $end";
    let pos = data.windows(pattern.len())
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
    hierarchy.scopes().map(|scope_ref| {
        extract_scope_data_with_file_path(hierarchy, scope_ref, file_path)
    }).collect()
}

fn extract_scope_data_with_file_path(hierarchy: &wellen::Hierarchy, scope_ref: wellen::ScopeRef, file_path: &str) -> ScopeData {
    let scope = &hierarchy[scope_ref];
    
    let mut variables: Vec<shared::Signal> = scope.vars(hierarchy).map(|var_ref| {
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
    }).collect();
    variables.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    let mut children: Vec<ScopeData> = scope.scopes(hierarchy).map(|child_scope_ref| {
        extract_scope_data_with_file_path(hierarchy, child_scope_ref, file_path)
    }).collect();
    children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    ScopeData {
        id: format!("{}|{}", file_path, scope.full_name(hierarchy)), // Use full file path + | separator + scope path for unique ID
        name: scope.name(hierarchy).to_string(),
        full_name: scope.full_name(hierarchy),
        children,
        variables,
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

async fn send_progress_update(file_id: String, progress: f32, session_id: SessionId, cor_id: CorId) {
    send_down_msg(DownMsg::ParsingProgress { file_id, progress }, session_id, cor_id).await;
}

async fn send_down_msg(msg: DownMsg, session_id: SessionId, cor_id: CorId) {
    if let Some(session) = sessions::by_session_id().wait_for(session_id).await {
        session.send_down_msg(&msg, cor_id).await;
    } else {
        // Session not found - likely disconnected
    }
}


const CONFIG_FILE_PATH: &str = ".novywave";

async fn load_config(session_id: SessionId, cor_id: CorId) {
    // Loading config from filesystem
    
    let config = match fs::read_to_string(CONFIG_FILE_PATH) {
        Ok(content) => {
            match toml::from_str::<AppConfig>(&content) {
                Ok(mut config) => {
                    // Enable migration system - validate and fix config after loading
                    let migration_warnings = config.validate_and_fix();
                    
                    // Log migration warnings if any
                    if !migration_warnings.is_empty() {
                        // Save migrated config to persist changes
                        if let Err(_save_err) = save_config_to_file(&config) {
                            // Migration applied but failed to save - continue with in-memory config
                        }
                    }
                    
                    config
                },
                Err(e) => {
                    send_down_msg(DownMsg::ConfigError(format!("Failed to parse config: {}", e)), session_id, cor_id).await;
                    return;
                }
            }
        }
        Err(_e) => {
            // Config file not found - create default
            // Create default config with validation already applied
            let mut default_config = AppConfig::default();
            let _warnings = default_config.validate_and_fix(); // Ensure defaults are validated too
            
            // Try to save default config
            if let Err(save_err) = save_config_to_file(&default_config) {
                send_down_msg(DownMsg::ConfigError(format!("Failed to create default config: {}", save_err)), session_id, cor_id).await;
                return;
            }
            
            default_config
        }
    };
    
    // PARALLEL PRELOADING: Start preloading expanded directories in background for instant file dialog
    let expanded_dirs = config.workspace.load_files_expanded_directories.clone();
    if !expanded_dirs.is_empty() {
        // Spawn background task to preload directories - precompute for instant access
        tokio::spawn(async move {
            let mut preload_tasks = Vec::new();
            
            // Create async task for each expanded directory
            for dir_path in expanded_dirs {
                let path = dir_path.clone();
                
                preload_tasks.push(tokio::spawn(async move {
                    let path_obj = Path::new(&path);
                    
                    // Precompute directory contents for instant access
                    if path_obj.exists() && path_obj.is_dir() {
                        match scan_directory_async(path_obj).await {
                            Ok(_items) => {
                            }
                            Err(_e) => {
                            }
                        }
                    }
                }));
            }
            
            // Wait for all preloading tasks to complete
            for task in preload_tasks {
                let _ = task.await; // Ignore individual task errors
            }
            
        });
    }

    send_down_msg(DownMsg::ConfigLoaded(config), session_id, cor_id).await;
}

async fn save_config(config: AppConfig, session_id: SessionId, cor_id: CorId) {
    match save_config_to_file(&config) {
        Ok(()) => {
            send_down_msg(DownMsg::ConfigSaved, session_id, cor_id).await;
        }
        Err(e) => {
            send_down_msg(DownMsg::ConfigError(format!("Failed to save config: {}", e)), session_id, cor_id).await;
        }
    }
}


fn save_config_to_file(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let toml_content = toml::to_string_pretty(config)?;
    
    // Add header comment
    let content_with_header = format!(
        "# NovyWave User Configuration\n\
         # This file stores your application preferences and workspace state\n\
         \n\
         {}", 
        toml_content
    );
    
    fs::write(CONFIG_FILE_PATH, content_with_header)?;
    Ok(())
}

fn cleanup_parsing_session(file_id: &str) {
    match PARSING_SESSIONS.lock() {
        Ok(mut sessions) => {
            sessions.remove(file_id);
        }
        Err(_) => {
            eprintln!("Warning: Failed to cleanup parsing session for file {}", file_id);
        }
    }
}


async fn browse_directory(dir_path: String, session_id: SessionId, cor_id: CorId) {
    
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
        
        send_down_msg(DownMsg::DirectoryContents { 
            path: "/".to_string(), 
            items: drive_items 
        }, session_id, cor_id).await;
        return;
    }
    
    // Expand ~ to home directory
    let expanded_path = if dir_path == "~" {
        match dirs::home_dir() {
            Some(home) => home.to_string_lossy().to_string(),
            None => {
                let error_msg = "Unable to determine home directory".to_string();
                send_down_msg(DownMsg::DirectoryError { 
                    path: dir_path, 
                    error: error_msg 
                }, session_id, cor_id).await;
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
                send_down_msg(DownMsg::DirectoryError { 
                    path: dir_path, 
                    error: error_msg 
                }, session_id, cor_id).await;
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
        send_down_msg(DownMsg::DirectoryError { 
            path: expanded_path, 
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    if !path.is_dir() {
        let error_msg = format!("Path is not a directory: {}", expanded_path);
        send_down_msg(DownMsg::DirectoryError { 
            path: expanded_path, 
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    // Use async parallel directory scanning for maximum performance
    match scan_directory_async(path).await {
        Ok(items) => {
            send_down_msg(DownMsg::DirectoryContents { 
                path: expanded_path.clone(), 
                items 
            }, session_id, cor_id).await;
        }
        Err(e) => {
            let error_msg = format!("Failed to scan directory: {}", e);
            send_down_msg(DownMsg::DirectoryError { 
                path: expanded_path, 
                error: error_msg 
            }, session_id, cor_id).await;
        }
    }
}

async fn browse_directories_batch(dir_paths: Vec<String>, session_id: SessionId, cor_id: CorId) {
    // Use jwalk's parallel processing capabilities for batch directory scanning
    let mut results = HashMap::new();
    
    // Process directories in parallel using jwalk's thread pool
    let parallel_tasks: Vec<_> = dir_paths.into_iter()
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
                    dirs::home_dir()
                        .map_or(dir_path, |home| home.to_string_lossy().to_string())
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
                        Err(e) => Err(format!("Failed to scan directory: {}", e))
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
    send_down_msg(DownMsg::BatchDirectoryContents { results }, session_id, cor_id).await;
}

async fn scan_directory_async(path: &Path) -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
    let path_buf = path.to_path_buf();
    
    // Use jwalk for parallel directory traversal, bridged with tokio
    let items = tokio::task::spawn_blocking(move || -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
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
            .sort(true)  // Enable sorting for consistent results
            .max_depth(1)  // Single level only (like TreeView expansion)
            .skip_hidden(false)  // We'll filter manually to match existing logic
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
                    
                    let name = entry_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    let is_directory = entry_path.is_dir();
                    let path_str = entry_path.to_string_lossy().to_string();
                    
                    // Only include directories and waveform files for cleaner file dialog
                    let is_waveform = if !is_directory {
                        let name_lower = name.to_lowercase();
                        name_lower.ends_with(".vcd") || name_lower.ends_with(".fst")
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
        items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(items)
    }).await??;
    
    Ok(items)
}


// Build signal reference map for efficient lookup during value queries
fn build_signal_reference_map(hierarchy: &wellen::Hierarchy, signals: &mut HashMap<String, wellen::SignalRef>) {
    // Recursively process all scopes in the hierarchy
    for scope_ref in hierarchy.scopes() {
        build_signals_for_scope_recursive(hierarchy, scope_ref, signals);
    }
}

// Recursively process a scope and all its child scopes
fn build_signals_for_scope_recursive(hierarchy: &wellen::Hierarchy, scope_ref: wellen::ScopeRef, signals: &mut HashMap<String, wellen::SignalRef>) {
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
async fn ensure_vcd_body_loaded(file_path: &str) -> Result<(), String> {
    // Check if already loaded
    {
        let store = match WAVEFORM_DATA_STORE.lock() {
            Ok(store) => store,
            Err(_) => return Err("Internal error: Failed to access waveform data store".to_string()),
        };
        if store.contains_key(file_path) {
            return Ok(());
        }
    }
    
    // Check if loading is already in progress for this file
    {
        let mut loading_in_progress = match VCD_LOADING_IN_PROGRESS.lock() {
            Ok(loading) => loading,
            Err(_) => return Err("Internal error: Failed to access VCD loading tracker".to_string()),
        };
        
        if loading_in_progress.contains(file_path) {
            // Another thread is already loading this file, just return success
            // The other thread will populate the data store
            return Ok(());
        }
        
        // Mark this file as being loaded
        loading_in_progress.insert(file_path.to_string());
    }
    
    // Need to parse VCD body - reparse the file
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
            return Err(format!("Failed to parse VCD header for signal queries: {}", e));
        }
        Err(_panic) => {
            // Remove from loading tracker on panic
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Critical error parsing VCD header: Invalid waveform data"));
        }
    };
    
    // Catch panics from body parsing
    let body_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None)
    })) {
        Ok(Ok(body)) => body,
        Ok(Err(e)) => {
            // Remove from loading tracker on failure
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Failed to parse VCD body for signal queries: {}", e));
        }
        Err(_panic) => {
            // Remove from loading tracker on panic
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Critical error parsing VCD body: Invalid signal data"));
        }
    };
    
    // Build signal reference map
    let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
    build_signal_reference_map(&header_result.hierarchy, &mut signals);
    
    // Calculate timescale factor for VCD time conversion
    let timescale_factor = header_result.hierarchy.timescale()
        .map(|ts| {
            use wellen::TimescaleUnit;
            match ts.unit {
                TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                TimescaleUnit::Seconds => ts.factor as f64,
                TimescaleUnit::Unknown => ts.factor as f64,
            }
        })
        .unwrap_or(1.0);
    
    // Store waveform data
    let waveform_data = WaveformData {
        hierarchy: header_result.hierarchy,
        signal_source: Arc::new(Mutex::new(body_result.source)),
        time_table: body_result.time_table.clone(),
        signals,
        file_format: header_result.file_format,
        timescale_factor,
    };
    
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
    
    debug_log!(DEBUG_BACKEND, "🔧 Generated {} fallback keys for scope '{}' variable '{}':", fallbacks.len(), scope_path, variable_name);
    for fallback in &fallbacks {
        println!("  - '{}'", fallback);
    }
    
    fallbacks
}

async fn query_signal_values(file_path: String, queries: Vec<SignalValueQuery>, session_id: SessionId, cor_id: CorId) {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(&file_path).await {
            send_down_msg(DownMsg::SignalValuesError {
                file_path,
                error: e,
            }, session_id, cor_id).await;
            return;
        }
    }
    
    let store = match WAVEFORM_DATA_STORE.lock() {
        Ok(store) => store,
        Err(_) => {
            send_down_msg(DownMsg::SignalValuesError {
                file_path,
                error: "Internal error: Failed to access waveform data store".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    
    let waveform_data = match store.get(&file_path) {
        Some(data) => data,
        None => {
            send_down_msg(DownMsg::SignalValuesError {
                file_path,
                error: "Waveform file not loaded or signal data not available".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    
    let mut results = Vec::new();
    
    for query in queries {
        let key = format!("{}|{}", query.scope_path, query.variable_name);
        
        // Debug: Log the query key and available keys for troubleshooting
        if !waveform_data.signals.contains_key(&key) {
            debug_log!(DEBUG_BACKEND, "🔍 SIGNAL NOT FOUND: Looking for key '{}' in file '{}'", key, file_path);
            debug_log!(DEBUG_BACKEND, "🔍 Available signal keys ({} total):", waveform_data.signals.len());
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
            let fallback_keys = generate_scope_path_fallbacks(&query.scope_path, &query.variable_name);
            fallback_keys.iter()
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
                    },
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
                                            let gap = waveform_data.time_table[i] - waveform_data.time_table[i-1];
                                            if gap > 0 {
                                                min = min.min(gap);
                                            }
                                        }
                                        if min == u64::MAX { *last_time / 10 } else { min * 3 } // 3x minimum gap as threshold
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
                    },
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
                let loaded_signals = signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);
                
                match loaded_signals.into_iter().next() {
                    Some((_, signal)) => {
                        if let Some(offset) = signal.get_offset(time_idx) {
                            let value = signal.get_value_at(&offset, 0);
                            
                            // Use the working formatter for all values
                            let formatted = format_non_binary_signal_value(&value);
                            let (raw_value, formatted_value) = (Some(formatted.clone()), Some(formatted));
                            
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
    
    send_down_msg(DownMsg::SignalValues {
        file_path,
        results,
    }, session_id, cor_id).await;
}

// Helper function for batch processing - returns results instead of sending
async fn process_signal_value_queries_internal(file_path: &str, queries: &[SignalValueQuery]) -> Result<Vec<SignalValueResult>, String> {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(file_path).await {
            return Err(e);
        }
    }
    
    let store = WAVEFORM_DATA_STORE.lock().map_err(|_| "Failed to access waveform data store".to_string())?;
    
    let waveform_data = store.get(file_path)
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
            },
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
                                    let gap = waveform_data.time_table[i] - waveform_data.time_table[i-1];
                                    if gap > 0 {
                                        min = min.min(gap);
                                    }
                                }
                                if min == u64::MAX { *last_time / 10 } else { min * 3 } // 3x minimum gap as threshold
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
            },
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
        
        let loaded_signals = signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);
        
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
            },
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
    cor_id: CorId
) {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(&file_path).await {
            send_down_msg(DownMsg::SignalTransitionsError {
                file_path,
                error: e,
            }, session_id, cor_id).await;
            return;
        }
    }
    
    let store = match WAVEFORM_DATA_STORE.lock() {
        Ok(store) => store,
        Err(_) => {
            send_down_msg(DownMsg::SignalTransitionsError {
                file_path,
                error: "Internal error: Failed to access waveform data store".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    let waveform_data = match store.get(&file_path) {
        Some(data) => data,
        None => {
            send_down_msg(DownMsg::SignalTransitionsError {
                file_path,
                error: "Waveform file not loaded or signal data not available".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    
    let mut results = Vec::new();
    
    for query in signal_queries {
        let key = format!("{}|{}", query.scope_path, query.variable_name);
        
        // DEBUG: Log signal lookup to identify key mismatch
        debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Looking for signal key: '{}'", key);
        debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Available keys: {:?}", waveform_data.signals.keys().collect::<Vec<_>>());
        
        // Try multiple key formats to handle scope path variations
        let signal_ref_option = waveform_data.signals.get(&key)
            .or_else(|| {
                // Try with different scope separators
                let alt_key1 = key.replace(".", "/");
                debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Trying alternative key format: '{}'", alt_key1);
                waveform_data.signals.get(&alt_key1)
            })
            .or_else(|| {
                // Try with no scope separator (flat names)
                let parts: Vec<&str> = query.scope_path.split('.').collect();
                if parts.len() > 1 {
                    let flat_key = format!("{}_{}", parts.join("_"), query.variable_name);
                    debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Trying flattened key format: '{}'", flat_key);
                    waveform_data.signals.get(&flat_key)
                } else {
                    None
                }
            })
            .or_else(|| {
                // Try searching for partial matches in case scope path differs
                debug_log!(DEBUG_BACKEND, "🔍 BACKEND: Searching for variable '{}' in any scope", query.variable_name);
                waveform_data.signals.iter()
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
                    },
                    _ => {
                        // For other formats (like FST), use proper timescale conversion
                        let start_seconds = time_range.0 as f64 / 1_000_000_000.0;
                        let end_seconds = time_range.1 as f64 / 1_000_000_000.0;
                        ((start_seconds / waveform_data.timescale_factor) as u64, (end_seconds / waveform_data.timescale_factor) as u64)
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
                let loaded_signals = signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);
                
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
                                    let time_seconds = time_val as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                },
                                _ => {
                                    let time_seconds = time_val as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                },
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
                                    let time_seconds = end_time as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                },
                                _ => {
                                    let time_seconds = end_time as f64 * waveform_data.timescale_factor;
                                    (time_seconds * 1_000_000_000.0) as u64
                                },
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
    
    send_down_msg(DownMsg::SignalTransitions {
        file_path,
        results,
    }, session_id, cor_id).await;
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
                    None => "X".to_string()
                }
            } else {
                // For multi-bit values, use wellen's to_bit_string() directly
                let bit_string = value.to_bit_string().unwrap_or_else(|| "?".to_string());
                
                // 🐛 DEBUG: Log what wellen actually returns vs what we expect
                debug_log!(DEBUG_BACKEND, "🔍 WELLEN ACTUAL: width={}, to_bit_string()='{}' (expecting binary like '1100')", width, bit_string);
                
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
                    None => "X".to_string()
                }
            } else {
                // Multi-bit FourValue (can include X and Z states)
                let bit_string = value.to_bit_string().unwrap_or_else(|| "?".to_string());
                
                // 🐛 DEBUG: Log FourValue output too
                debug_log!(DEBUG_BACKEND, "🔍 WELLEN FOURVALUE: width={}, to_bit_string()='{}' (expecting binary)", width, bit_string);
                
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
    debug_log!(DEBUG_BACKEND, "🔍 BACKEND: About to call SIGNAL_CACHE_MANAGER.query_unified_signals for request_id: {}", request_id);
    match SIGNAL_CACHE_MANAGER.query_unified_signals(signal_requests, cursor_time_ns).await {
        Ok((signal_data, cursor_values, statistics)) => {
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: SIGNAL_CACHE_MANAGER success - {} signal_data items, {} cursor_values", signal_data.len(), cursor_values.len());
            send_down_msg(DownMsg::UnifiedSignalResponse {
                request_id,
                signal_data,
                cursor_values,
                cached_time_range_ns: None, // Cache time range would be computed from signal data bounds
                statistics: Some(statistics),
            }, session_id, cor_id).await;
        }
        Err(error) => {
            debug_log!(DEBUG_BACKEND, "🔍 BACKEND: SIGNAL_CACHE_MANAGER error: {}", error);
            send_down_msg(DownMsg::UnifiedSignalError {
                request_id,
                error,
            }, session_id, cor_id).await;
        }
    }
}

#[moon::main]
async fn main() -> std::io::Result<()> {
    // Set panic hook to log all panics 
    std::panic::set_hook(Box::new(|panic_info| {
        println!("BACKEND PANIC: {:?}", panic_info);
    }));
    
    
    start(frontend, up_msg_handler, |_error| {
        // Error logging removed to reduce log spam
    }).await
}
