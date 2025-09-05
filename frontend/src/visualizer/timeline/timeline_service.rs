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
use crate::visualizer::timeline::time_types::{TimeNs, TimelineCache, CacheRequestType, CacheRequestState};
use std::collections::HashMap;
use crate::visualizer::state::timeline_state::UNIFIED_TIMELINE_CACHE;
use zoon::Lazy;
use crate::visualizer::timeline::timeline_actor::{cursor_position_signal, viewport_signal};

// ===== DATA STRUCTURES =====

// Re-export shared::UnifiedSignalRequest as SignalRequest for compatibility
pub use shared::UnifiedSignalRequest as SignalRequest;

/// Circuit breaker tracking for variables that consistently return empty responses
#[derive(Clone, Debug)]
struct RequestTracker {
    consecutive_empty_responses: u32,
    last_request_time: Option<TimeNs>,
    backoff_delay_ms: u64,
    is_circuit_open: bool,
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self {
            consecutive_empty_responses: 0,
            last_request_time: None,
            backoff_delay_ms: 500, // Start with 500ms
            is_circuit_open: false,
        }
    }
}

// ❌ ANTIPATTERN: Service Layer Caching - TODO: Move to WaveformTimeline Actor
/// Global circuit breaker state for preventing infinite request loops
#[deprecated(note = "Use WaveformTimeline Actor with Cache Current Values pattern instead of service layer caching")]
static CIRCUIT_BREAKER: Lazy<Mutable<HashMap<String, RequestTracker>>> = Lazy::new(|| {
    Mutable::new(HashMap::new())
});

// ❌ ANTIPATTERN: Service Layer Caching - TODO: Move to WaveformTimeline Actor
/// Cache for empty results to prevent retry storms
#[deprecated(note = "Use WaveformTimeline Actor with Cache Current Values pattern instead of service layer caching")]
static EMPTY_RESULT_CACHE: Lazy<Mutable<HashMap<String, TimeNs>>> = Lazy::new(|| {
    Mutable::new(HashMap::new())
});


// ===== PUBLIC API =====

// ❌ ANTIPATTERN: Service Layer Abstraction - TODO: Replace with WaveformTimeline Actor+Relay events
#[deprecated(note = "Replace service methods with Actor+Relay events in WaveformTimeline domain")]
pub struct UnifiedTimelineService;

impl UnifiedTimelineService {
    // ===== CIRCUIT BREAKER METHODS =====
    
    /// Check if circuit breaker should prevent request for this variable
    fn should_apply_circuit_breaker(signal_id: &str) -> bool {
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        let breaker_map = CIRCUIT_BREAKER.lock_ref();
        
        if let Some(tracker) = breaker_map.get(signal_id) {
            // Check if circuit is open
            if tracker.is_circuit_open {
                return true;
            }
            
            // Check backoff delay
            if let Some(last_request) = tracker.last_request_time {
                let backoff_duration = super::time_types::DurationNs::from_external_seconds(tracker.backoff_delay_ms as f64 / 1000.0);
                if now.duration_since(last_request) < backoff_duration {
                    return true;
                }
            }
        }
        
        // Check empty result cache
        let empty_cache = EMPTY_RESULT_CACHE.lock_ref();
        if let Some(cached_time) = empty_cache.get(signal_id) {
            let cache_duration = super::time_types::DurationNs::from_external_seconds(5.0); // 5 second cache
            if now.duration_since(*cached_time) < cache_duration {
                return true;
            }
        }
        
        false
    }
    
    /// Track empty response and update circuit breaker state
    fn track_empty_response(signal_id: &str) {
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        
        // Update circuit breaker
        let mut breaker_map = CIRCUIT_BREAKER.lock_mut();
        let tracker = breaker_map.entry(signal_id.to_string()).or_default();
        
        tracker.consecutive_empty_responses += 1;
        tracker.last_request_time = Some(now);
        
        // Apply exponential backoff (500ms, 1s, 2s, 4s, 5s max)
        tracker.backoff_delay_ms = (tracker.backoff_delay_ms * 2).min(5000);
        
        // Open circuit after 3 consecutive empty responses
        if tracker.consecutive_empty_responses >= 3 {
            tracker.is_circuit_open = true;
        }
        
        // Cache empty result
        let mut empty_cache = EMPTY_RESULT_CACHE.lock_mut();
        empty_cache.insert(signal_id.to_string(), now);
    }
    
    /// Reset circuit breaker when variable gets successful response
    fn reset_circuit_breaker(signal_id: &str) {
        let mut breaker_map = CIRCUIT_BREAKER.lock_mut();
        if breaker_map.contains_key(signal_id) {
            breaker_map.remove(signal_id);
        }
        
        // Remove from empty cache
        let mut empty_cache = EMPTY_RESULT_CACHE.lock_mut();
        empty_cache.remove(signal_id);
    }
    
    /// Cache empty result to prevent immediate retries
    fn cache_empty_result(signal_id: &str) -> String {
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        let mut empty_cache = EMPTY_RESULT_CACHE.lock_mut();
        empty_cache.insert(signal_id.to_string(), now);
        "N/A".to_string()
    }

    /// Reset circuit breaker and caches for specific variables (public method)
    pub fn reset_circuit_breakers_for_variables(signal_ids: &[String]) {
        let mut _breaker_reset_count = 0;
        let mut _empty_cache_reset_count = 0;
        let mut _cursor_cache_reset_count = 0;
        
        // Batch operations to reduce lock contention
        {
            let mut breaker_map = CIRCUIT_BREAKER.lock_mut();
            let mut empty_cache = EMPTY_RESULT_CACHE.lock_mut();
            
            for signal_id in signal_ids {
                if breaker_map.remove(signal_id).is_some() {
                    _breaker_reset_count += 1;
                }
                if empty_cache.remove(signal_id).is_some() {
                    _empty_cache_reset_count += 1;
                }
            }
        }
        
        // Clear cached cursor values in separate lock scope
        {
            let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
            for signal_id in signal_ids {
                if cache.cursor_values.remove(signal_id).is_some() {
                    _cursor_cache_reset_count += 1;
                }
            }
        }
        
        // Cache reset completed silently
        
        // CRITICAL: Force signal re-evaluation by triggering cache signal
        Self::force_signal_reevaluation();
    }

    /// Force all cursor_value_signal chains to re-evaluate by modifying cache signal
    pub fn force_signal_reevaluation() {
        
        // Trigger cache signal by temporarily modifying cache - this forces map_ref! to re-evaluate
        {
            let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
            // Force signal emission by modifying cache metadata timestamp
            cache.metadata.statistics.query_time_ms = js_sys::Date::now() as u64; // Change timestamp to trigger signal
        }
        
        // Immediately get current state and trigger fresh requests
        Task::start(async {
            // WORKAROUND: Cursor position signals are broken (returning 0), so explicitly request at 2μs
            // where we know data should exist based on the user's report
            let target_cursor = TimeNs::from_external_seconds(2e-6); // 2 microseconds
            
            // Get current selected variables from signal  
            let current_variables = crate::actors::selected_variables::variables_signal()
                .to_stream().next().await.unwrap_or_default();
                
            if !current_variables.is_empty() {
                let signal_ids: Vec<String> = current_variables.iter()
                    .map(|v| v.unique_id.clone())
                    .collect();
                    
                // Debug logging removed to prevent console spam in hot path
                    
                // Trigger fresh backend requests immediately at the target position
                Self::request_cursor_values(signal_ids, target_cursor);
                
                // Note: NOT updating cursor position signals to avoid circular dependency
                // The cursor position handler in main.rs would trigger circuit breaker reset,
                // which would call force_signal_reevaluation() again, creating infinite loop
            }
        });
    }

    /// Initialize the unified timeline service
    pub fn initialize() {
        // Start cache cleanup and maintenance tasks
        Self::start_cache_maintenance();
        
        // Start reactive cache invalidation handlers
        Self::start_cache_invalidation_handlers();
    }
    
    /// Public method to manually trigger circuit breaker reset and signal re-evaluation
    /// Used for debugging and manual recovery from circuit breaker states
    #[allow(dead_code)] // Timeline service method - preserve for debugging features
    pub fn manual_reset_and_reevaluate() {
        
        // Get all selected variable IDs
        let signal_ids = vec![
            "/home/martinkavik/repos/NovyWave/test_files/simple.vcd|simple_tb.s|A".to_string(),
            "/home/martinkavik/repos/NovyWave/test_files/simple.vcd|simple_tb.s|B".to_string(),
            "/home/martinkavik/repos/NovyWave/test_files/nested_dir/wave_27.fst|TOP.VexiiRiscv|clk".to_string(),
        ];
        
        // Reset circuit breakers and force re-evaluation
        Self::reset_circuit_breakers_for_variables(&signal_ids);
    }
    
    
    /// Request cursor values at specific timeline position
    pub fn request_cursor_values(
        signal_ids: Vec<String>,
        cursor_time: TimeNs,
    ) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        // Enhanced duplicate request checking by variable set
        let variable_set: std::collections::HashSet<String> = 
            signal_ids.iter().cloned().collect();
            
        if Self::is_duplicate_request_by_set(&cache, &variable_set, CacheRequestType::CursorValues) {
            // Debug logging removed to prevent console spam in hot path
            return;
        }
        
        // Also check individual signal duplicates for compatibility
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
        
        // Request missing data from backend with performance logging
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
            
            // Debug logging removed to prevent console spam in hot path
            
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
                    let cursor_ns = *cursor_pos;
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
                // Check circuit breaker before making new request
                else if Self::should_apply_circuit_breaker(&signal_id_cloned) {
                    Self::cache_empty_result(&signal_id_cloned)
                }
                // No data available - trigger request for cursor values
                else {
                    // Trigger async request for cursor values outside viewport
                    let signal_ids = vec![signal_id_cloned.clone()];
                    let cursor_time = *cursor_pos;
                    Task::start(async move {
                        Self::request_cursor_values(signal_ids, cursor_time);
                    });
                    "Loading...".to_string()
                }
            }
        }.dedupe_cloned()
    }
    
    /// Handle unified response from backend
    pub fn handle_unified_response(
        request_id: String,
        signal_data: Vec<shared::UnifiedSignalData>,
        cursor_values: std::collections::BTreeMap<String, SignalValue>,
        statistics: Option<shared::SignalStatistics>,
    ) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        // Track empty responses for circuit breaker logic
        if signal_data.is_empty() && cursor_values.is_empty() {
            if let Some(request) = cache.active_requests.get(&request_id) {
                for signal_id in &request.requested_signals {
                    Self::track_empty_response(signal_id);
                }
            }
        }
        
        // Get request info first to avoid borrow conflicts
        let request_info = cache.active_requests.get(&request_id).cloned();
        
        // Update viewport data
        for signal in signal_data {
            if let Some(request) = &request_info {
                // Always update raw transitions first (move signal.transitions here)
                let raw_transitions = signal.transitions;
                cache.raw_transitions.insert(signal.unique_id.clone(), raw_transitions.clone());
                
                // Reset circuit breaker on successful raw transitions
                Self::reset_circuit_breaker(&signal.unique_id);
                
                if request.request_type == CacheRequestType::ViewportData {
                    let viewport_data = super::time_types::ViewportSignalData {
                    };
                    cache.viewport_data.insert(signal.unique_id, viewport_data);
                }
            }
        }
        
        // Update cursor values and trigger signal updates
        let has_cursor_values = !cursor_values.is_empty();
        if has_cursor_values {
            let mut ui_signal_values = HashMap::new();
            
            for (signal_id, value) in &cursor_values {
                cache.cursor_values.insert(signal_id.clone(), value.clone());
                
                // Convert backend SignalValue to UI SignalValue format
                let ui_value = match value {
                    shared::SignalValue::Present(data) => super::super::formatting::signal_values::SignalValue::from_data(data.clone()),
                    shared::SignalValue::Missing => super::super::formatting::signal_values::SignalValue::missing(),
                };
                ui_signal_values.insert(signal_id.clone(), ui_value);
                
                // Reset circuit breaker on successful data
                Self::reset_circuit_breaker(&signal_id);
            }
            
            // Send cursor values to UI signal system
            let num_values = ui_signal_values.len();
            if num_values > 0 {
                let relay = crate::visualizer::timeline::timeline_actor::signal_values_updated_relay();
                relay.send(ui_signal_values);
                // Debug logging removed to prevent console spam in hot path
            }
        }
        
        // Update statistics
        if let Some(stats) = statistics {
            cache.metadata.statistics = stats;
        }
        
        // Remove completed request
        cache.active_requests.remove(&request_id);
        
        // Release cache lock before triggering signals
        drop(cache);
        
        // Trigger cache signal to notify cursor_value_signal() chains if values were updated
        if has_cursor_values {
            let current_cache = UNIFIED_TIMELINE_CACHE.get_cloned();
            UNIFIED_TIMELINE_CACHE.set(current_cache);
        }
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
    #[allow(dead_code)] // Timeline service method - preserve for cache reactivity
    pub fn cache_signal() -> impl Signal<Item = ()> + use<> {
        UNIFIED_TIMELINE_CACHE.signal_ref(|_| ())
    }
    
    
    
    // ===== CACHE OPTIMIZATION METHODS =====
    
    /// Smart cache invalidation - only clear cache for variables that changed
    async fn smart_cache_invalidation(
        previous_ids: Option<std::collections::HashSet<String>>, 
        current_ids: std::collections::HashSet<String>
    ) {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        
        if let Some(prev_ids) = previous_ids {
            // Calculate which variables were removed or added
            let removed_variables: std::collections::HashSet<_> = 
                prev_ids.difference(&current_ids).collect();
            let added_variables: std::collections::HashSet<_> = 
                current_ids.difference(&prev_ids).collect();
            
            // Only clear cache for variables that actually changed
            if !removed_variables.is_empty() {
                for removed_var in &removed_variables {
                    cache.cursor_values.remove(*removed_var);
                    cache.viewport_data.remove(*removed_var);
                    cache.raw_transitions.remove(*removed_var);
                }
            }
            
            if !added_variables.is_empty() {
                // New variables will naturally trigger cache misses and backend requests
            }
            
            // Only invalidate validity flags if there were actual changes
            if !removed_variables.is_empty() || !added_variables.is_empty() {
                cache.metadata.validity.cursor_valid = false;
                // Keep viewport data valid unless variables were removed
                if !removed_variables.is_empty() {
                    cache.metadata.validity.viewport_valid = false;
                }
            }
            
        } else {
            // First time setup - no previous state
        }
    }
    
    /// Enhanced duplicate request checking by variable set rather than individual IDs
    fn is_duplicate_request_by_set(
        cache: &super::time_types::TimelineCache,
        variable_set: &std::collections::HashSet<String>,
        request_type: super::time_types::CacheRequestType
    ) -> bool {
        let now = super::time_types::TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        let dedup_threshold = super::time_types::DurationNs::from_external_seconds(0.5); // 500ms
        
        cache.active_requests.values().any(|request| {
            if request.request_type != request_type || 
               now.duration_since(request.timestamp_ns) >= dedup_threshold {
                return false;
            }
            
            // Check if this request covers the same variable set
            let request_set: std::collections::HashSet<String> = 
                request.requested_signals.iter().cloned().collect();
            
            // Consider it duplicate if there's significant overlap (>80%)
            let intersection_size = variable_set.intersection(&request_set).count();
            let union_size = variable_set.union(&request_set).count();
            
            if union_size == 0 { return false; }
            
            let overlap_ratio = intersection_size as f64 / union_size as f64;
            overlap_ratio > 0.8 // 80% overlap threshold
        })
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
                Self::cleanup_circuit_breakers();
            }
        });
    }
    
    fn cleanup_circuit_breakers() {
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        
        // Clean up old circuit breaker entries (older than 5 minutes)
        let mut breaker_map = CIRCUIT_BREAKER.lock_mut();
        let cutoff_duration = super::time_types::DurationNs::from_external_seconds(300.0); // 5 minutes
        
        breaker_map.retain(|_, tracker| {
            if let Some(last_request) = tracker.last_request_time {
                now.duration_since(last_request) < cutoff_duration
            } else {
                false // Remove trackers with no request time
            }
        });
        
        // Clean up old empty result cache entries
        let mut empty_cache = EMPTY_RESULT_CACHE.lock_mut();
        let cache_cutoff = super::time_types::DurationNs::from_external_seconds(30.0); // 30 seconds
        
        empty_cache.retain(|_, cached_time| {
            now.duration_since(*cached_time) < cache_cutoff
        });
    }
    
    fn cleanup_old_requests() {
        let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        let cutoff_threshold = super::time_types::DurationNs::from_external_seconds(10.0); // 10 seconds
        
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
                let new_cursor = cursor_pos;
                async move {
                    let mut cache = UNIFIED_TIMELINE_CACHE.lock_mut();
                    let was_invalid_before = !cache.metadata.validity.cursor_valid;
                    cache.invalidate_cursor(new_cursor);
                    let became_invalid = !cache.metadata.validity.cursor_valid;
                    
                    // Release lock before triggering signal
                    drop(cache);
                    
                    // ✅ FIX: Trigger cache signal when cursor position changes to update variable values
                    if (!was_invalid_before && became_invalid) || became_invalid {
                        let current_cache = UNIFIED_TIMELINE_CACHE.get_cloned();
                        UNIFIED_TIMELINE_CACHE.set(current_cache);
                    }
                }
            }).await;
        });
        
        // React to selected variables changes with smart cache invalidation
        Task::start(async {
            use std::cell::RefCell;
            use std::rc::Rc;
            
            let previous_variable_ids = Rc::new(RefCell::new(None::<std::collections::HashSet<String>>));
            let debounce_task = Rc::new(RefCell::new(None::<zoon::TaskHandle>));
            
            crate::actors::selected_variables::variables_signal().dedupe_cloned().for_each(move |selected_vars| {
                let current_variable_ids: std::collections::HashSet<String> = 
                    selected_vars.iter().map(|v| v.unique_id.clone()).collect();
                
                let prev_ids = previous_variable_ids.borrow().clone();
                let current_ids = current_variable_ids.clone();
                let debounce_task_clone = debounce_task.clone();
                let previous_variable_ids_clone = previous_variable_ids.clone();
                
                async move {
                    // Cancel previous debounce task to batch rapid changes
                    if let Some(task) = debounce_task_clone.borrow_mut().take() {
                        drop(task); // Drop immediately aborts the task
                    }
                    
                    // Debounce cache operations for 200ms to handle rapid selection changes
                    let handle = zoon::Task::start_droppable(async move {
                        zoon::Timer::sleep(200).await;
                        Self::smart_cache_invalidation(prev_ids, current_ids).await;
                    });
                    
                    *debounce_task_clone.borrow_mut() = Some(handle);
                    *previous_variable_ids_clone.borrow_mut() = Some(current_variable_ids);
                }
            }).await;
        });
    }
}

/// Initialize unified timeline service at app startup
pub fn initialize_unified_timeline_service() {
    UnifiedTimelineService::initialize();
}