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
use crate::time_types::{TimeNs, Viewport, TimelineCache, CacheRequestType, CacheRequestState};
use crate::state::UNIFIED_TIMELINE_CACHE;

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
    
    /// Request viewport data for rendering (decimated transitions)
    pub fn request_viewport_data(
        signal_ids: Vec<String>,
        viewport: Viewport,
    ) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        // Check for duplicate requests
        if cache.is_duplicate_request(&signal_ids, CacheRequestType::ViewportData) {
            return;
        }
        
        // Update cache viewport and invalidate if needed
        cache.invalidate_viewport(viewport);
        
        // Check cache hits first
        let mut cache_hits = Vec::new();
        let mut cache_misses = Vec::new();
        
        for signal_id in &signal_ids {
            if let Some(viewport_data) = cache.get_viewport_data(signal_id) {
                // Check if cached data covers current viewport
                if viewport_data.viewport.start <= viewport.start && 
                   viewport_data.viewport.end >= viewport.end {
                    cache_hits.push(signal_id.clone());
                    cache.metadata.statistics.cached_signals += 1;
                } else {
                    cache_misses.push(signal_id.clone());
                }
            } else {
                cache_misses.push(signal_id.clone());
            }
        }
        
        // Update cache hit ratio
        let total_requests = signal_ids.len();
        cache.metadata.statistics.total_signals += total_requests;
        if total_requests > 0 {
            cache.metadata.statistics.cache_hit_ratio = 
                (cache.metadata.statistics.cached_signals as f64) / 
                (cache.metadata.statistics.total_signals as f64);
        }
        
        // Request missing data from backend
        if !cache_misses.is_empty() {
            let request_id = Self::generate_request_id();
            
            // Track active request
            cache.active_requests.insert(request_id.clone(), CacheRequestState {
                request_id: request_id.clone(),
                requested_signals: cache_misses.clone(),
                cursor_time: None,
                viewport: Some(viewport),
                timestamp_ns: TimeNs::from_seconds(js_sys::Date::now() / 1000.0),
                request_type: CacheRequestType::ViewportData,
            });
            
            // Convert to backend request format
            let backend_requests: Vec<shared::UnifiedSignalRequest> = cache_misses
                .into_iter()
                .map(|signal_id| {
                    let parts: Vec<&str> = signal_id.split('|').collect();
                    shared::UnifiedSignalRequest {
                        file_path: parts[0].to_string(),
                        scope_path: parts[1].to_string(),
                        variable_name: parts[2].to_string(),
                        time_range: Some((viewport.start.to_seconds(), viewport.end.to_seconds())),
                        max_transitions: Some(10000), // Decimation limit
                        format: shared::VarFormat::Binary, // Default format
                    }
                })
                .collect();
            
            drop(cache); // Release lock before sending request
            
            send_up_msg(UpMsg::UnifiedSignalQuery {
                signal_requests: backend_requests,
                cursor_time: None,
                request_id,
            });
        }
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
                request_id: request_id.clone(),
                requested_signals: cache_misses.clone(),
                cursor_time: Some(cursor_time),
                viewport: None,
                timestamp_ns: TimeNs::from_seconds(js_sys::Date::now() / 1000.0),
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
                        time_range: None, // Point query, not range
                        max_transitions: None,
                        format: shared::VarFormat::Binary,
                    }
                })
                .collect();
            
            drop(cache); // Release lock
            
            send_up_msg(UpMsg::UnifiedSignalQuery {
                signal_requests: backend_requests,
                cursor_time: Some(cursor_time.to_seconds()),
                request_id,
            });
        }
    }
    
    /// Get reactive signal for cursor value at current timeline position
    pub fn cursor_value_signal(signal_id: &str) -> impl Signal<Item = String> + use<> {
        let signal_id_cloned = signal_id.to_string();
        
        // React to both cursor changes and cache updates
        map_ref! {
            let cursor_ns = crate::state::TIMELINE_CURSOR_NS.signal(),
            let _cache_signal = UNIFIED_TIMELINE_CACHE.signal_ref(|_| ()) => {
                let cache = UNIFIED_TIMELINE_CACHE.lock_ref();
                
                // Check cursor value cache first
                if let Some(cached_value) = cache.get_cursor_value(&signal_id_cloned) {
                    zoon::println!("🎯 CURSOR: Cache hit for {}", signal_id_cloned);
                    match cached_value {
                        SignalValue::Present(data) => data.clone(),
                        SignalValue::Missing => "N/A".to_string(),
                    }
                }
                // Then check if we can interpolate from raw transitions
                else if let Some(transitions) = cache.get_raw_transitions(&signal_id_cloned) {
                    zoon::println!("🔄 CURSOR: Interpolating from {} transitions for {}", transitions.len(), signal_id_cloned);
                    if let Some(interpolated) = Self::interpolate_cursor_value(transitions, *cursor_ns) {
                        match interpolated {
                            SignalValue::Present(data) => data,
                            SignalValue::Missing => "N/A".to_string(),
                        }
                    } else {
                        zoon::println!("❌ CURSOR: Interpolation failed for {}", signal_id_cloned);
                        "N/A".to_string()
                    }
                }
                // Check for pending request
                else if Self::has_pending_request(&cache, &signal_id_cloned, CacheRequestType::CursorValues) {
                    zoon::println!("⏳ CURSOR: Pending request for {}", signal_id_cloned);
                    "Loading...".to_string()
                }
                // No data available - trigger request for cursor values
                else {
                    zoon::println!("🚀 CURSOR: Triggering new request for {}", signal_id_cloned);
                    // Trigger async request for cursor values outside viewport
                    let signal_ids = vec![signal_id_cloned.clone()];
                    let cursor_time = *cursor_ns;
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
        zoon::println!("📦 UNIFIED: Received response - {} signals, {} cursor values", signal_data.len(), cursor_values.len());
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
                        transitions: raw_transitions,
                        viewport: request.viewport.unwrap_or(cache.metadata.current_viewport),
                        last_updated_ns: TimeNs::from_seconds(js_sys::Date::now() / 1000.0),
                        total_source_transitions: signal.total_transitions,
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
        // TODO: Implement error recovery strategies
    }
    
    /// Clear all cache data (for app restart)
    pub fn clear_all_caches() {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        *cache = TimelineCache::new();
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
        
        let cursor_seconds = cursor_time.to_seconds();
        let first_time = transitions[0].time_seconds;
        let last_time = transitions[transitions.len() - 1].time_seconds;
        
        cursor_seconds >= first_time && cursor_seconds <= last_time
    }
    
    fn interpolate_cursor_value(transitions: &[SignalTransition], cursor_time: TimeNs) -> Option<SignalValue> {
        if transitions.is_empty() {
            return None;
        }
        
        let cursor_seconds = cursor_time.to_seconds();
        
        // Find the most recent transition at or before cursor time
        for transition in transitions.iter().rev() {
            if transition.time_seconds <= cursor_seconds {
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
        let now = TimeNs::from_seconds(js_sys::Date::now() / 1000.0);
        let cutoff_threshold = crate::time_types::DurationNs::from_seconds(10.0); // 10 seconds
        
        cache.active_requests.retain(|_, request| {
            now.duration_since(request.timestamp_ns) < cutoff_threshold
        });
    }
    
    fn start_cache_invalidation_handlers() {
        // React to viewport changes and invalidate cache accordingly
        Task::start(async {
            crate::state::TIMELINE_VIEWPORT.signal().for_each(move |new_viewport| {
                async move {
                    let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
                    cache.invalidate_viewport(new_viewport);
                }
            }).await;
        });
        
        // React to cursor changes and invalidate cursor cache accordingly  
        Task::start(async {
            crate::state::TIMELINE_CURSOR_NS.signal().for_each(move |new_cursor| {
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
                        // TODO: Optimize to only clear data for removed variables
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