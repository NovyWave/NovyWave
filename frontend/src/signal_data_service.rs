//! Unified Signal Data Service - Single Source of Truth for All Signal Data
//! 
//! This service replaces the fragmented signal query system with a unified approach:
//! - Single backend query interface for all signal data needs
//! - Reactive MutableBTreeMap caches for automatic UI updates
//! - Request deduplication and intelligent batching
//! - Desktop-optimized with backend doing heavy lifting

use zoon::*;
use shared::{UpMsg, UnifiedSignalRequest, UnifiedSignalData, SignalValue, SignalStatistics};
use crate::connection::send_up_msg;
use std::collections::{BTreeMap, HashMap, HashSet};

// ===== REACTIVE SIGNAL CACHES =====

/// Primary cache for signal viewport data (transitions for rendering)
/// Using MutableVec since BTreeMap doesn't support direct mutation
static VIEWPORT_SIGNALS: Lazy<MutableVec<(String, SignalViewport)>> = 
    Lazy::new(MutableVec::new);

/// Cursor values cache - reactive values at current timeline position  
/// Using MutableVec for signal values with proper reactivity
static CURSOR_VALUES: Lazy<MutableVec<(String, Mutable<SignalValue>)>> = 
    Lazy::new(MutableVec::new);

/// Request tracking to prevent duplicate backend requests
static ACTIVE_REQUESTS: Lazy<Mutable<HashMap<String, RequestState>>> = 
    Lazy::new(|| Mutable::new(HashMap::new()));

/// Cache statistics for performance monitoring
static CACHE_STATISTICS: Lazy<Mutable<Option<SignalStatistics>>> = 
    Lazy::new(|| Mutable::new(None));

/// Signal-level request tracking to prevent excessive requests for same signal
static SIGNAL_REQUEST_TIMESTAMPS: Lazy<Mutable<HashMap<String, f64>>> = 
    Lazy::new(|| Mutable::new(HashMap::new()));

// ===== DATA STRUCTURES =====

#[derive(Clone, Debug)]
pub struct SignalViewport {
    #[allow(dead_code)]  // Used when storing viewport data, may be used by timeline rendering
    pub transitions: Vec<shared::SignalTransition>,
    #[allow(dead_code)]  // Used for time range validation, may be used by timeline rendering
    pub actual_time_range: Option<(f64, f64)>,
    #[allow(dead_code)]  // Used for performance statistics, may be used by timeline rendering
    pub total_transitions: usize,
    #[allow(dead_code)]  // Used for cache invalidation, may be used by timeline rendering
    pub last_updated_ms: f64,
}

#[derive(Clone, Debug)]
struct RequestState {
    pub request_id: String,
    pub requested_signals: Vec<String>,
    #[allow(dead_code)] // TODO: Use for enhanced duplicate request detection
    pub cursor_time: Option<f64>,
    pub timestamp_ms: f64,
}

impl PartialEq for RequestState {
    fn eq(&self, other: &Self) -> bool {
        self.request_id == other.request_id
    }
}

// ===== PUBLIC API =====

pub struct SignalDataService;

impl SignalDataService {
    /// Initialize the signal data service and start reactive handlers
    pub fn initialize() {
        // Initialize reactive cleanup task
        Self::start_cache_cleanup_task();
    }
    
    /// Single entry point for ALL signal data requests
    /// Replaces all old query functions with unified approach
    pub fn request_signal_data(
        signal_requests: Vec<SignalRequest>,
        cursor_time: Option<f64>,
        _high_priority: bool,
    ) {
        let request_id = generate_request_id();
        
        // Convert to backend request format
        let backend_requests: Vec<UnifiedSignalRequest> = signal_requests
            .into_iter()
            .map(|req| UnifiedSignalRequest {
                file_path: req.file_path,
                scope_path: req.scope_path,
                variable_name: req.variable_name,
                time_range: req.time_range,
                max_transitions: req.max_transitions.or(Some(10000)), // Default downsample
                format: req.format,
            })
            .collect();
        
        // Check for duplicate request
        if Self::is_duplicate_request(&backend_requests, cursor_time) {
            return; // Skip duplicate request
        }
        
        // Track active request
        let signal_ids: Vec<String> = backend_requests.iter()
            .map(|req| format!("{}|{}|{}", req.file_path, req.scope_path, req.variable_name))
            .collect();
        
        let mut active_requests = ACTIVE_REQUESTS.get_cloned();
        active_requests.insert(request_id.clone(), RequestState {
            request_id: request_id.clone(),
            requested_signals: signal_ids.clone(),
            cursor_time,
            timestamp_ms: js_sys::Date::now(),
        });
        ACTIVE_REQUESTS.set_neq(active_requests);
        
        // Update signal-level request timestamps for deduplication
        let mut signal_timestamps = SIGNAL_REQUEST_TIMESTAMPS.get_cloned();
        let now = js_sys::Date::now();
        for signal_id in &signal_ids {
            signal_timestamps.insert(signal_id.clone(), now);
        }
        SIGNAL_REQUEST_TIMESTAMPS.set_neq(signal_timestamps);
        
        // Send unified request to backend
        send_up_msg(UpMsg::UnifiedSignalQuery {
            signal_requests: backend_requests,
            cursor_time,
            request_id,
        });
    }
    
    /// Get reactive signal for cursor value at current timeline position
    /// Uses cache-first approach: check backend responses first, then timeline cache
    pub fn cursor_value_signal(signal_id: &str) -> impl Signal<Item = String> + use<> {
        let signal_id_cloned = signal_id.to_string();
        
        // Create a combined signal that reacts to cursor position AND both cache changes
        map_ref! {
            let cursor_ns = crate::state::TIMELINE_CURSOR_NS.signal(),
            let _cursor_values_signal = CURSOR_VALUES.signal_vec_cloned().to_signal_cloned().map(|_| ()),
            let _timeline_cache_signal = crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.signal_ref(|_| ()) => {
                let cursor_pos = cursor_ns.to_seconds();
                // Parse signal_id: "/path/file.ext|scope|variable"
                let parts: Vec<&str> = signal_id_cloned.split('|').collect();
                if parts.len() != 3 {
                    "N/A".to_string()
                } else {
                    let file_path = parts[0];
                    let scope_path = parts[1];
                    let variable_name = parts[2];
                    
                    // PRIORITY 1: Check CURSOR_VALUES cache (backend responses) FIRST
                    let cursor_values = CURSOR_VALUES.lock_ref();
                    let backend_result = cursor_values.iter().find(|(id, _)| id == &signal_id_cloned)
                        .map(|(_, cached_signal_value)| {
                            match cached_signal_value.get_cloned() {
                                shared::SignalValue::Present(data) => data,
                                shared::SignalValue::Missing => "N/A".to_string(),
                            }
                        });
                    drop(cursor_values); // Release lock
                    
                    if let Some(backend_value) = backend_result {
                        backend_value // Backend response found - use immediately
                    } else {
                        // PRIORITY 2: Check timeline cache (interpolated values) as fallback
                        if let Some(cached_value) = crate::views::compute_value_from_cached_transitions(
                            file_path, scope_path, variable_name, cursor_pos
                        ) {
                            match cached_value {
                                shared::SignalValue::Present(data) => data,
                                shared::SignalValue::Missing => "N/A".to_string(),
                            }
                        } else {
                            // PRIORITY 3: Check for pending backend request
                            let active_requests = ACTIVE_REQUESTS.get_cloned();
                            let has_pending_request = active_requests.values()
                                .any(|request_state| request_state.requested_signals.contains(&signal_id_cloned));
                            
                            if has_pending_request {
                                "Loading...".to_string() // Request in progress
                            } else {
                                // PRIORITY 4: No data available - show N/A (don't trigger more requests)
                                "N/A".to_string()
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Get reactive signal for viewport data (transitions)
    /// Used by Timeline canvas for rendering
    #[allow(dead_code)]  // Part of future timeline rendering API
    pub fn viewport_signal(signal_id: &str) -> impl Signal<Item = Option<SignalViewport>> + use<> {
        let signal_id_cloned = signal_id.to_string();
        
        VIEWPORT_SIGNALS.signal_vec_cloned()
            .to_signal_map(move |viewport_signals| {
                for (id, viewport) in viewport_signals {
                    if id == &signal_id_cloned {
                        return Some(viewport.clone());
                    }
                }
                None
            })
    }
    
    /// Get cache statistics signal for performance monitoring
    #[allow(dead_code)]  // Part of future performance monitoring API
    pub fn statistics_signal() -> impl Signal<Item = Option<SignalStatistics>> + use<> {
        CACHE_STATISTICS.signal()
    }
    
    /// Clear all caches - used during app restart to fix initialization issues
    pub fn clear_all_caches() {
        VIEWPORT_SIGNALS.lock_mut().clear();
        CURSOR_VALUES.lock_mut().clear();
        // DON'T clear ACTIVE_REQUESTS - those are in-flight requests, not cached data
        // ACTIVE_REQUESTS.set_neq(HashMap::new()); // ❌ REMOVED - breaks pending requests
        CACHE_STATISTICS.set_neq(None);
        
    }
    
    /// Clean up data for specific variables that were removed from selection
    /// Called when variables are removed from SELECTED_VARIABLES
    pub fn cleanup_variables(removed_signal_ids: &[String]) {
        if removed_signal_ids.is_empty() {
            return;
        }
        
        
        // Remove viewport data for the removed variables
        let mut viewport_signals = VIEWPORT_SIGNALS.lock_mut();
        let initial_viewport_count = viewport_signals.len();
        viewport_signals.retain(|(id, _)| !removed_signal_ids.contains(id));
        let _removed_viewport_count = initial_viewport_count - viewport_signals.len(); // logging removed
        
        // Remove cursor values for the removed variables
        let mut cursor_values = CURSOR_VALUES.lock_mut();
        let initial_cursor_count = cursor_values.len();
        cursor_values.retain(|(id, _)| !removed_signal_ids.contains(id));
        let _removed_cursor_count = initial_cursor_count - cursor_values.len(); // logging removed
        
    }
    
    /// Check if a variable is currently tracked by the service
    #[allow(dead_code)]  // Used for validation and debugging
    pub fn has_variable(signal_id: &str) -> bool {
        // Check viewport cache
        let viewport_signals = VIEWPORT_SIGNALS.lock_ref();
        if viewport_signals.iter().any(|(id, _)| id == signal_id) {
            return true;
        }
        
        // Check cursor values cache
        let cursor_values = CURSOR_VALUES.lock_ref();
        cursor_values.iter().any(|(id, _)| id == signal_id)
    }
    
    // ===== PRIVATE IMPLEMENTATION =====
    
    /// Check if this request duplicates an existing in-flight request
    fn is_duplicate_request(requests: &[UnifiedSignalRequest], _cursor_time: Option<f64>) -> bool {
        let new_signal_ids: HashSet<String> = requests.iter()
            .map(|req| format!("{}|{}|{}", req.file_path, req.scope_path, req.variable_name))
            .collect();
        
        let now = js_sys::Date::now();
        let active_requests = ACTIVE_REQUESTS.get_cloned();
        let signal_timestamps = SIGNAL_REQUEST_TIMESTAMPS.get_cloned();
        
        // Check signal-level deduplication first (prevents excessive FST requests)
        for signal_id in &new_signal_ids {
            if let Some(&last_request_time) = signal_timestamps.get(signal_id) {
                let elapsed_ms = now - last_request_time;
                
                // FST files need longer deduplication window due to precision issues
                let dedup_window_ms = if signal_id.contains(".fst|") { 
                    1500.0  // 1.5 seconds for FST files
                } else { 
                    500.0   // 0.5 seconds for VCD files
                };
                
                if elapsed_ms < dedup_window_ms {
                    return true; // Recent request for this signal
                }
            }
        }
        
        // Check if any active request covers the same signals
        for active_request in active_requests.values() {
            let active_signal_ids: HashSet<String> = active_request.requested_signals.iter().cloned().collect();
            
            // Check for overlap
            if !new_signal_ids.is_disjoint(&active_signal_ids) {
                // Similar request within last 500ms = duplicate
                let elapsed_ms = now - active_request.timestamp_ms;
                if elapsed_ms < 500.0 {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Handle unified signal response from backend
    pub fn handle_unified_response(
        request_id: String,
        signal_data: Vec<UnifiedSignalData>,
        cursor_values: BTreeMap<String, SignalValue>,
        statistics: Option<SignalStatistics>,
    ) {
        // Update viewport cache
        for signal in signal_data {
            let viewport = SignalViewport {
                transitions: signal.transitions,
                actual_time_range: signal.actual_time_range,
                total_transitions: signal.total_transitions,
                last_updated_ms: js_sys::Date::now(),
            };
            
            // Remove existing entry if present, then add new one
            let mut viewport_signals = VIEWPORT_SIGNALS.lock_mut();
            viewport_signals.retain(|(id, _)| id != &signal.unique_id);
            viewport_signals.push_cloned((signal.unique_id, viewport));
        }
        
        // Update cursor values cache
        for (signal_id, value) in cursor_values {
            let mut cursor_values_vec = CURSOR_VALUES.lock_mut();
            
            // Find existing entry
            if let Some((_, existing_mutable)) = cursor_values_vec.iter().find(|(id, _)| id == &signal_id) {
                existing_mutable.set_neq(value);
            } else {
                // Add new entry
                cursor_values_vec.push_cloned((signal_id, Mutable::new(value)));
            }
        }
        
        // Update statistics
        if let Some(stats) = statistics {
            CACHE_STATISTICS.set_neq(Some(stats));
        }
        
        // Remove from active requests
        let mut active_requests = ACTIVE_REQUESTS.get_cloned();
        active_requests.remove(&request_id);
        ACTIVE_REQUESTS.set_neq(active_requests);
        
    }
    
    /// Handle error response from backend
    pub fn handle_unified_error(request_id: String, _error: String) { // logging removed
        
        // Remove from active requests
        let mut active_requests = ACTIVE_REQUESTS.get_cloned();
        active_requests.remove(&request_id);
        ACTIVE_REQUESTS.set_neq(active_requests);
        
        // TODO: Implement error recovery strategies
        // - Retry with exponential backoff
        // - Mark signals as unavailable
        // - Show error in UI
    }
    
    
    /// Start periodic cache cleanup task
    fn start_cache_cleanup_task() {
        Task::start(async {
            loop {
                Timer::sleep(30000).await; // Every 30 seconds
                Self::cleanup_old_requests();
            }
        });
    }
    
    /// Clean up old/stale requests
    fn cleanup_old_requests() {
        let now = js_sys::Date::now();
        let cutoff_time_ms = now - 10_000.0; // 10 seconds ago
        
        // Clean up active requests
        let mut active_requests = ACTIVE_REQUESTS.get_cloned();
        active_requests.retain(|_, request| request.timestamp_ms > cutoff_time_ms);
        ACTIVE_REQUESTS.set_neq(active_requests);
        
        // Clean up signal-level request timestamps (use longer window for FST)
        let mut signal_timestamps = SIGNAL_REQUEST_TIMESTAMPS.get_cloned();
        signal_timestamps.retain(|signal_id, timestamp| {
            let cleanup_window_ms = if signal_id.contains(".fst|") {
                5_000.0  // 5 seconds for FST files
            } else {
                2_000.0  // 2 seconds for VCD files
            };
            now - *timestamp < cleanup_window_ms
        });
        SIGNAL_REQUEST_TIMESTAMPS.set_neq(signal_timestamps);
    }
}

// ===== HELPER TYPES =====

/// Request format for the unified service
#[derive(Clone, Debug)]
pub struct SignalRequest {
    pub file_path: String,
    pub scope_path: String,
    pub variable_name: String,
    pub time_range: Option<(f64, f64)>,
    pub max_transitions: Option<usize>,
    pub format: shared::VarFormat,
}

/// Generate unique request ID
fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = js_sys::Date::now() as u128;
    
    format!("unified_{}_{}", timestamp, id)
}

// ===== INITIALIZATION HELPER =====

/// Initialize signal data service at app startup
pub fn initialize_signal_data_service() {
    SignalDataService::initialize();
}