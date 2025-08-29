//! Unified Timeline Service - Replaces Signal Data Service with Integer Time Architecture
//!
//! This service provides a single interface for all timeline data operations:
//! - Viewport data requests (decimated for rendering)
//! - Cursor value requests (point-in-time values)
//! - Raw transition queries (full precision for calculations)
//! - Intelligent caching and request deduplication
//! - Integer nanosecond precision throughout

use zoon::*;
use shared::{UpMsg, SignalValue, SignalTransition};
use crate::connection::send_up_msg;
use crate::time_types::{TimeNs, TimelineCache, CacheRequestType, CacheRequestState};
use crate::state::UNIFIED_TIMELINE_CACHE;
use crate::actors::domain_bridges::{cursor_position_signal, viewport_signal};

// ===== DATA STRUCTURES =====

// Re-export shared::UnifiedSignalRequest as SignalRequest for compatibility
pub use shared::UnifiedSignalRequest as SignalRequest;


// ===== PUBLIC API =====

pub struct UnifiedTimelineService;

impl UnifiedTimelineService {
    /// Initialize the unified timeline service
    pub fn initialize() {
        // Start cache cleanup and maintenance tasks
        Self::start_cache_maintenance();
        
        // Start reactive cache invalidation handlers
        Self::start_cache_invalidation_handlers();
    }
    
    
    /// Request cursor values at specific timeline position
    pub fn request_cursor_values(
        signal_ids: Vec<String>,
        cursor_time: TimeNs,
    ) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        // Check for duplicate requests
        if cache.is_duplicate_request(&signal_ids, CacheRequestType::CursorValues) {
            return;
        }
        
        // Update cache cursor and invalidate if needed
        cache.invalidate_cursor(cursor_time);
        
        // Check cache hits - cursor values or interpolatable from raw transitions
        let mut cache_hits = Vec::new();
        let mut cache_misses = Vec::new();
        
        for signal_id in &signal_ids {
            // First check cursor value cache
            if cache.get_cursor_value(signal_id).is_some() {
                cache_hits.push(signal_id.clone());
            }
            // Then check if we can interpolate from raw transitions
            else if let Some(transitions) = cache.get_raw_transitions(signal_id) {
                if Self::can_interpolate_cursor_value(transitions, cursor_time) {
                    cache_hits.push(signal_id.clone());
                } else {
                    cache_misses.push(signal_id.clone());
                }
            }
            // Otherwise it's a cache miss
            else {
                cache_misses.push(signal_id.clone());
            }
        }
        
        // Request missing data from backend
        if !cache_misses.is_empty() {
            let request_id = Self::generate_request_id();
            
            cache.active_requests.insert(request_id.clone(), CacheRequestState {
                requested_signals: cache_misses.clone(),
                _viewport: None,
                timestamp_ns: TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0),
                request_type: CacheRequestType::CursorValues,
            });
            
            let backend_requests: Vec<shared::UnifiedSignalRequest> = cache_misses
                .into_iter()
                .map(|signal_id| {
                    let parts: Vec<&str> = signal_id.split('|').collect();
                    shared::UnifiedSignalRequest {
                        file_path: parts[0].to_string(),
                        scope_path: parts[1].to_string(),
                        variable_name: parts[2].to_string(),
                        time_range_ns: None, // Point query, not range
                        max_transitions: None,
                        format: shared::VarFormat::Binary,
                    }
                })
                .collect();
            
            drop(cache); // Release lock
            
            send_up_msg(UpMsg::UnifiedSignalQuery {
                signal_requests: backend_requests,
                cursor_time_ns: Some(cursor_time.nanos()),
                request_id,
            });
        }
    }
    
    /// Get reactive signal for cursor value at current timeline position
    pub fn cursor_value_signal(signal_id: &str) -> impl Signal<Item = String> + use<> {
        let signal_id_cloned = signal_id.to_string();
        
        // React to both cursor changes and cache updates
        map_ref! {
            let cursor_pos = cursor_position_signal(),
            let _cache_signal = UNIFIED_TIMELINE_CACHE.signal_ref(|_| ()) => {
                let cache = UNIFIED_TIMELINE_CACHE.lock_ref();
                
                // Check cursor value cache first
                if let Some(cached_value) = cache.get_cursor_value(&signal_id_cloned) {
                    match cached_value {
                        SignalValue::Present(data) => data.clone(),
                        SignalValue::Missing => "N/A".to_string(),
                    }
                }
                // Then check if we can interpolate from raw transitions
                else if let Some(transitions) = cache.get_raw_transitions(&signal_id_cloned) {
                    let cursor_ns = TimeNs::from_external_seconds(*cursor_pos);
                    if let Some(interpolated) = Self::interpolate_cursor_value(transitions, cursor_ns) {
                        match interpolated {
                            SignalValue::Present(data) => data,
                            SignalValue::Missing => "N/A".to_string(),
                        }
                    } else {
                        "N/A".to_string()
                    }
                }
                // Check for pending request
                else if Self::has_pending_request(&cache, &signal_id_cloned, CacheRequestType::CursorValues) {
                    "Loading...".to_string()
                }
                // No data available - trigger request for cursor values
                else {
                    // Trigger async request for cursor values outside viewport
                    let signal_ids = vec![signal_id_cloned.clone()];
                    let cursor_time = TimeNs::from_external_seconds(*cursor_pos);
                    Task::start(async move {
                        Self::request_cursor_values(signal_ids, cursor_time);
                    });
                    "Loading...".to_string()
                }
            }
        }
    }
    
    /// Handle unified response from backend
    pub fn handle_unified_response(
        request_id: String,
        signal_data: Vec<shared::UnifiedSignalData>,
        cursor_values: std::collections::BTreeMap<String, SignalValue>,
        statistics: Option<shared::SignalStatistics>,
    ) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        // Get request info first to avoid borrow conflicts
        let request_info = cache.active_requests.get(&request_id).cloned();
        
        // Update viewport data
        for signal in signal_data {
            if let Some(request) = &request_info {
                // Always update raw transitions first (move signal.transitions here)
                let raw_transitions = signal.transitions;
                cache.raw_transitions.insert(signal.unique_id.clone(), raw_transitions.clone());
                
                if request.request_type == CacheRequestType::ViewportData {
                    let viewport_data = crate::time_types::ViewportSignalData {
                    };
                    cache.viewport_data.insert(signal.unique_id, viewport_data);
                }
            }
        }
        
        // Update cursor values
        for (signal_id, value) in cursor_values {
            cache.cursor_values.insert(signal_id, value);
        }
        
        // Update statistics
        if let Some(stats) = statistics {
            cache.metadata.statistics = stats;
        }
        
        // Remove completed request
        cache.active_requests.remove(&request_id);
    }
    
    /// Handle error response from backend
    pub fn handle_unified_error(request_id: String, _error: String) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        cache.active_requests.remove(&request_id);
    }
    
    
    /// Get raw transitions for a signal (public accessor for compatibility)
    pub fn get_raw_transitions(signal_id: &str) -> Option<Vec<shared::SignalTransition>> {
        let cache = UNIFIED_TIMELINE_CACHE.lock_ref();
        cache.raw_transitions.get(signal_id).cloned()
    }
    
    /// Insert raw transitions (public accessor for backend data)
    pub fn insert_raw_transitions(signal_id: String, transitions: Vec<shared::SignalTransition>) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        cache.raw_transitions.insert(signal_id, transitions);
    }
    
    /// Get signal for cache changes (public accessor for reactivity)
    pub fn cache_signal() -> impl Signal<Item = ()> + use<> {
        UNIFIED_TIMELINE_CACHE.signal_ref(|_| ())
    }
    
    /// Clean up data for specific variables (for compatibility with legacy API)
    pub fn cleanup_variables(removed_signal_ids: &[String]) {
        if removed_signal_ids.is_empty() {
            return;
        }
        
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        // Remove viewport data for removed variables
        for signal_id in removed_signal_ids {
            cache.viewport_data.remove(signal_id);
            cache.cursor_values.remove(signal_id);
            cache.raw_transitions.remove(signal_id);
        }
        
        // Remove any active requests for these variables
        cache.active_requests.retain(|_, request| {
            !request.requested_signals.iter().any(|id| removed_signal_ids.contains(id))
        });
    }
    
    
    // ===== PRIVATE HELPERS =====
    
    fn generate_request_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = js_sys::Date::now() as u128;
        
        format!("unified_{}_{}", timestamp, id)
    }
    
    fn can_interpolate_cursor_value(transitions: &[SignalTransition], cursor_time: TimeNs) -> bool {
        if transitions.is_empty() {
            return false;
        }
        
        let cursor_seconds = cursor_time.display_seconds();
        let first_time = transitions[0].time_ns as f64 / 1_000_000_000.0;
        let last_time = transitions[transitions.len() - 1].time_ns as f64 / 1_000_000_000.0;
        
        cursor_seconds >= first_time && cursor_seconds <= last_time
    }
    
    fn interpolate_cursor_value(transitions: &[SignalTransition], cursor_time: TimeNs) -> Option<SignalValue> {
        if transitions.is_empty() {
            return None;
        }
        
        let cursor_seconds = cursor_time.display_seconds();
        
        // Find the most recent transition at or before cursor time
        for transition in transitions.iter().rev() {
            if transition.time_ns as f64 / 1_000_000_000.0 <= cursor_seconds {
                return Some(SignalValue::Present(transition.value.clone()));
            }
        }
        
        None
    }
    
    fn has_pending_request(cache: &TimelineCache, signal_id: &str, request_type: CacheRequestType) -> bool {
        cache.active_requests.values().any(|request| {
            request.request_type == request_type && 
            request.requested_signals.contains(&signal_id.to_string())
        })
    }
    
    fn start_cache_maintenance() {
        Task::start(async {
            loop {
                Timer::sleep(30000).await; // Every 30 seconds
                Self::cleanup_old_requests();
            }
        });
    }
    
    fn cleanup_old_requests() {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        let cutoff_threshold = crate::time_types::DurationNs::from_external_seconds(10.0); // 10 seconds
        
        cache.active_requests.retain(|_, request| {
            now.duration_since(request.timestamp_ns) < cutoff_threshold
        });
    }
    
    fn start_cache_invalidation_handlers() {
        // React to viewport changes and invalidate cache accordingly
        Task::start(async {
            viewport_signal().for_each(move |new_viewport| {
                async move {
                    let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
                    cache.invalidate_viewport(new_viewport);
                }
            }).await;
        });
        
        // React to cursor changes and invalidate cursor cache accordingly  
        Task::start(async {
            cursor_position_signal().for_each(move |cursor_pos| {
                let new_cursor = TimeNs::from_external_seconds(cursor_pos);
                async move {
                    let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
                    cache.invalidate_cursor(new_cursor);
                }
            }).await;
        });
        
        // React to selected variables changes and clear related cache entries
        Task::start(async {
            crate::state::SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_selected_vars| {
                async move {
                    // Clear cache when selected variables change significantly
                    // This ensures we don't show stale data for newly selected or deselected variables
                    let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
                    
                    // Only clear if there's a significant change in selection
                    // (Implementation could be optimized to only clear removed variables)
                    if !cache.viewport_data.is_empty() || !cache.cursor_values.is_empty() {
                        // For now, clear all cached values to ensure consistency
                        cache.cursor_values.clear();
                        cache.metadata.validity.cursor_valid = false;
                        
                        // Keep viewport data as it's expensive to reload
                        // but mark as potentially stale
                        cache.metadata.validity.viewport_valid = false;
                    }
                }
            }).await;
        });
    }
}

/// Initialize unified timeline service at app startup
pub fn initialize_unified_timeline_service() {
    UnifiedTimelineService::initialize();
}