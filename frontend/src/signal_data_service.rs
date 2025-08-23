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
            requested_signals: signal_ids,
            cursor_time,
            timestamp_ms: js_sys::Date::now(),
        });
        ACTIVE_REQUESTS.set_neq(active_requests);
        
        // Send unified request to backend
        send_up_msg(UpMsg::UnifiedSignalQuery {
            signal_requests: backend_requests,
            cursor_time,
            request_id,
        });
    }
    
    /// Get reactive signal for cursor value at current timeline position
    /// Uses cache-first approach: check timeline cache, then backend if needed
    pub fn cursor_value_signal(signal_id: &str) -> impl Signal<Item = String> + use<> {
        let signal_id_cloned = signal_id.to_string();
        
        // Create a combined signal that reacts to cursor position AND cache changes
        map_ref! {
            let cursor_pos = crate::state::TIMELINE_CURSOR_POSITION.signal(),
            let cache_signal = crate::waveform_canvas::SIGNAL_TRANSITIONS_CACHE.signal_ref(|_| ()) => {
                // Parse signal_id: "/path/file.ext|scope|variable"
                let parts: Vec<&str> = signal_id_cloned.split('|').collect();
                if parts.len() != 3 {
                    "N/A".to_string()
                } else {
                    let file_path = parts[0];
                    let scope_path = parts[1];
                    let variable_name = parts[2];
                    
                    // First try to get value from timeline cache
                    if let Some(cached_value) = crate::views::compute_value_from_cached_transitions(
                        file_path, scope_path, variable_name, *cursor_pos
                    ) {
                        match cached_value {
                            shared::SignalValue::Present(data) => data,
                            shared::SignalValue::Missing => "N/A".to_string(),
                        }
                    } else {
                        // No cached data - check if we have a pending backend request
                        let active_requests = ACTIVE_REQUESTS.get_cloned();
                        let has_pending_request = active_requests.values()
                            .any(|request_state| request_state.requested_signals.contains(&signal_id_cloned));
                        
                        if has_pending_request {
                            "Loading...".to_string()
                        } else {
                            // No cached data and no pending request - should trigger backend query
                            "N/A".to_string()
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
        
        zoon::println!("🧹 SignalDataService: All caches cleared (except active requests)");
    }
    
    /// Clean up data for specific variables that were removed from selection
    /// Called when variables are removed from SELECTED_VARIABLES
    pub fn cleanup_variables(removed_signal_ids: &[String]) {
        if removed_signal_ids.is_empty() {
            return;
        }
        
        zoon::println!("🧹 SignalDataService: Cleaning up {} removed variables", removed_signal_ids.len());
        
        // Remove viewport data for the removed variables
        let mut viewport_signals = VIEWPORT_SIGNALS.lock_mut();
        let initial_viewport_count = viewport_signals.len();
        viewport_signals.retain(|(id, _)| !removed_signal_ids.contains(id));
        let removed_viewport_count = initial_viewport_count - viewport_signals.len();
        
        // Remove cursor values for the removed variables
        let mut cursor_values = CURSOR_VALUES.lock_mut();
        let initial_cursor_count = cursor_values.len();
        cursor_values.retain(|(id, _)| !removed_signal_ids.contains(id));
        let removed_cursor_count = initial_cursor_count - cursor_values.len();
        
        zoon::println!("🧹 SignalDataService: Cleaned {} viewport entries and {} cursor value entries", 
                     removed_viewport_count, removed_cursor_count);
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
        
        let active_requests = ACTIVE_REQUESTS.get_cloned();
        
        // Check if any active request covers the same signals
        for active_request in active_requests.values() {
            let active_signal_ids: HashSet<String> = active_request.requested_signals.iter().cloned().collect();
            
            // Check for overlap
            if !new_signal_ids.is_disjoint(&active_signal_ids) {
                // Similar request within last 500ms = duplicate
                let elapsed_ms = js_sys::Date::now() - active_request.timestamp_ms;
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
    pub fn handle_unified_error(request_id: String, error: String) {
        zoon::println!("❌ SignalDataService: Request {} failed: {}", request_id, error);
        
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
        let mut active_requests = ACTIVE_REQUESTS.get_cloned();
        
        let cutoff_time_ms = js_sys::Date::now() - 10_000.0; // 10 seconds ago
        active_requests.retain(|_, request| request.timestamp_ms > cutoff_time_ms);
        
        ACTIVE_REQUESTS.set_neq(active_requests);
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