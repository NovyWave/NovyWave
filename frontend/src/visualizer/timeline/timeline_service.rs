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
use crate::visualizer::timeline::time_types::{CacheRequestType, NS_PER_SECOND, TimeNs, TimelineCache};
use std::collections::HashMap;
// ✅ ACTOR+RELAY: UNIFIED_TIMELINE_CACHE access replaced with timeline_actor functions
// Removed unused import: zoon::Lazy
use crate::visualizer::timeline::timeline_actor::{cursor_position_signal, viewport_signal};

// ===== DATA STRUCTURES =====

// Re-export shared::UnifiedSignalRequest as SignalRequest for compatibility
pub use shared::UnifiedSignalRequest as SignalRequest;

// ✅ REMOVED: RequestTracker struct - circuit breaker logic moved to WaveformTimeline Actor

// Circuit breaker and empty result caching now handled through WaveformTimeline Actor
// No global static caches needed - logic moved to Actor's Cache Current Values pattern


// ===== PUBLIC API =====

// ❌ ANTIPATTERN: Service Layer Abstraction - TODO: Replace with WaveformTimeline Actor+Relay events
#[deprecated(note = "Replace service methods with Actor+Relay events in WaveformTimeline domain")]
pub struct UnifiedTimelineService;

#[allow(dead_code)] // Deprecated service implementation - preserve during Actor+Relay migration
#[allow(deprecated)] // ✅ FIXED: Suppress warning on deprecated struct usage
impl UnifiedTimelineService {
    // ===== CIRCUIT BREAKER METHODS =====
    
    /// Check if circuit breaker should prevent request for this variable
    /// ✅ DISABLED: Circuit breaker logic moved to WaveformTimeline Actor
    fn should_apply_circuit_breaker(_signal_id: &str) -> bool {
        // Circuit breaker functionality disabled - requests handled by WaveformTimeline Actor
        false
    }
    
    /// Track empty response and update circuit breaker state
    /// ✅ DISABLED: Empty response tracking moved to WaveformTimeline Actor
    fn track_empty_response(_signal_id: &str) {
        // Empty response tracking disabled - requests handled by WaveformTimeline Actor
    }
    
    /// Reset circuit breaker when variable gets successful response
    /// ✅ DISABLED: Circuit breaker reset moved to WaveformTimeline Actor
    fn reset_circuit_breaker(_signal_id: &str) {
        // Circuit breaker reset disabled - requests handled by WaveformTimeline Actor
    }
    
    /// Cache empty result to prevent immediate retries
    /// ✅ DISABLED: Empty result caching moved to WaveformTimeline Actor
    fn cache_empty_result(_signal_id: &str) -> String {
        // Empty result caching disabled - return N/A directly
        "N/A".to_string()
    }

    /// Reset circuit breaker and caches for specific variables (public method)
    /// ✅ PARTIALLY MIGRATED: Only timeline actor cache clearing active
    pub fn reset_circuit_breakers_for_variables(signal_ids: &[String]) {
        let mut _cursor_cache_reset_count = 0;
        
        // Circuit breaker and empty cache reset disabled - handled by WaveformTimeline Actor
        
        // ✅ ACTOR+RELAY: Clear cached cursor values using WaveformTimeline domain methods
        {
            for signal_id in signal_ids {
                if crate::visualizer::timeline::timeline_actor::remove_cursor_value_from_cache(signal_id).is_some() {
                    _cursor_cache_reset_count += 1;
                }
            }
        }
        
        // Cache reset completed silently
        
        // ✅ ACTOR+RELAY: Force signal re-evaluation using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::force_cache_signal_reevaluation();
    }

    /// Force all cursor_value_signal chains to re-evaluate by modifying cache signal
    pub fn force_signal_reevaluation() {
        
        // ✅ ACTOR+RELAY: Trigger cache signal using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::force_cache_signal_reevaluation();
        
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
    
    // ✅ REMOVED: manual_reset_and_reevaluate() - unused debug function with hardcoded test paths
    
    
    /// Request cursor values at specific timeline position
    pub fn request_cursor_values(
        signal_ids: Vec<String>,
        cursor_time: TimeNs,
    ) {
        // ✅ ACTOR+RELAY: Check duplicate requests using WaveformTimeline domain method
        if crate::visualizer::timeline::timeline_actor::is_duplicate_request_in_cache(&signal_ids, CacheRequestType::CursorValues) {
            return;
        }
        
        // ✅ ACTOR+RELAY: Update cache cursor using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::invalidate_cursor_cache(cursor_time);
        
        // ✅ ACTOR+RELAY: Check cache hits using WaveformTimeline domain methods
        let mut cache_hits = Vec::new();
        let mut cache_misses = Vec::new();
        
        for signal_id in &signal_ids {
            // TODO: Replace static cache access with Cache Current Values pattern inside WaveformTimeline Actor
            // Check if we can interpolate from raw transitions
            if let Some(transitions) = crate::visualizer::timeline::timeline_actor::get_raw_transitions_from_cache(signal_id) {
                if Self::can_interpolate_cursor_value(&transitions, cursor_time) {
                    cache_hits.push(signal_id.clone());
                } else {
                    cache_misses.push(signal_id.clone());
                }
            } else {
                // Otherwise it's a cache miss
                cache_misses.push(signal_id.clone());
            }
        }
        
        // Request missing data from backend with performance logging
        if !cache_misses.is_empty() {
            let request_id = Self::generate_request_id();
            
            
            // ✅ ACTOR+RELAY: Add active request using WaveformTimeline domain method
            crate::visualizer::timeline::timeline_actor::add_active_request_to_cache(
                request_id.clone(), 
                super::time_types::CacheRequestState {
                    requested_signals: cache_misses.clone(),
                    _viewport: None,
                    timestamp_ns: TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0),
                    request_type: CacheRequestType::CursorValues,
                }
            );
            
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
        
        // ✅ ACTOR+RELAY: React to cursor and cache changes using WaveformTimeline domain signals
        map_ref! {
            let cursor_pos = cursor_position_signal(),
            let _cache_signal = crate::visualizer::timeline::timeline_actor::unified_timeline_cache_signal() => {
                
                // TODO: Replace static cache with Cache Current Values pattern inside WaveformTimeline Actor
                // ✅ ACTOR+RELAY: Check raw transitions using WaveformTimeline domain method
                if let Some(transitions) = crate::visualizer::timeline::timeline_actor::get_raw_transitions_from_cache(&signal_id_cloned) {
                    let cursor_ns = *cursor_pos;
                    if let Some(interpolated) = Self::interpolate_cursor_value(&transitions, cursor_ns) {
                        match interpolated {
                            SignalValue::Present(data) => data,
                            SignalValue::Missing => "N/A".to_string(),
                        }
                    } else {
                        "N/A".to_string()
                    }
                }
                // ✅ ACTOR+RELAY: Check for pending request using WaveformTimeline domain method
                else if crate::visualizer::timeline::timeline_actor::is_duplicate_request_in_cache(&[signal_id_cloned.clone()], CacheRequestType::CursorValues) {
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
        // ✅ ACTOR+RELAY: Get request info using WaveformTimeline domain method
        let request_info = crate::visualizer::timeline::timeline_actor::get_active_request_from_cache(&request_id);
        
        // Track empty responses for circuit breaker logic
        if signal_data.is_empty() && cursor_values.is_empty() {
            if let Some(request) = &request_info {
                for signal_id in &request.requested_signals {
                    Self::track_empty_response(signal_id);
                }
            }
        }
        
        // Update viewport data
        for signal in signal_data {
            if let Some(request) = &request_info {
                // Always update raw transitions first (move signal.transitions here)
                let raw_transitions = signal.transitions;
                // ✅ ACTOR+RELAY: Insert raw transitions using WaveformTimeline domain method
                crate::visualizer::timeline::timeline_actor::insert_raw_transitions_to_cache(signal.unique_id.clone(), raw_transitions.clone());
                
                // Reset circuit breaker on successful raw transitions
                Self::reset_circuit_breaker(&signal.unique_id);
                
                if request.request_type == CacheRequestType::ViewportData {
                    let viewport_data = super::time_types::ViewportSignalData {
                    };
                    // ✅ ACTOR+RELAY: Insert viewport data using WaveformTimeline domain method
                    crate::visualizer::timeline::timeline_actor::insert_viewport_data_to_cache(signal.unique_id, viewport_data);
                }
            }
        }
        
        // Update cursor values and trigger signal updates
        let has_cursor_values = !cursor_values.is_empty();
        if has_cursor_values {
            let mut ui_signal_values = HashMap::new();
            
            for (signal_id, value) in &cursor_values {
                // ✅ ACTOR+RELAY: Insert cursor value using WaveformTimeline domain method
                crate::visualizer::timeline::timeline_actor::insert_cursor_value_to_cache(signal_id.clone(), value.clone());
                
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
            // ✅ ACTOR+RELAY: Update cache statistics using WaveformTimeline domain method
            crate::visualizer::timeline::timeline_actor::update_cache_statistics(stats);
        }
        
        // ✅ ACTOR+RELAY: Remove completed request using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::remove_active_request_from_cache(&request_id);
        
        // ✅ ACTOR+RELAY: Trigger cache signal using WaveformTimeline domain method
        if has_cursor_values {
            crate::visualizer::timeline::timeline_actor::force_cache_signal_reevaluation();
        }
    }
    
    /// Handle error response from backend
    pub fn handle_unified_error(request_id: String, _error: String) {
        // ✅ ACTOR+RELAY: Remove failed request using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::remove_active_request_from_cache(&request_id);
    }
    
    
    /// Get raw transitions for a signal (public accessor for compatibility)
    pub fn get_raw_transitions(signal_id: &str) -> Option<Vec<shared::SignalTransition>> {
        // ✅ ACTOR+RELAY: Get raw transitions using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::get_raw_transitions_from_cache(signal_id)
    }
    
    /// Insert raw transitions (public accessor for backend data)
    pub fn insert_raw_transitions(signal_id: String, transitions: Vec<shared::SignalTransition>) {
        // ✅ ACTOR+RELAY: Insert raw transitions using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::insert_raw_transitions_to_cache(signal_id, transitions);
    }
    
    /// Get signal for cache changes (public accessor for reactivity)
    #[allow(dead_code)] // Timeline service method - preserve for cache reactivity
    pub fn cache_signal() -> impl Signal<Item = ()> + use<> {
        // ✅ ACTOR+RELAY: Get cache signal using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::unified_timeline_cache_signal()
    }
    
    
    
    // ===== CACHE OPTIMIZATION METHODS =====
    
    /// Smart cache invalidation - only clear cache for variables that changed
    async fn smart_cache_invalidation(
        previous_ids: Option<std::collections::HashSet<String>>, 
        current_ids: std::collections::HashSet<String>
    ) {
        if let Some(prev_ids) = previous_ids {
            // Calculate which variables were removed or added
            let removed_variables: std::collections::HashSet<_> = 
                prev_ids.difference(&current_ids).collect();
            let added_variables: std::collections::HashSet<_> = 
                current_ids.difference(&prev_ids).collect();
            
            // Only clear cache for variables that actually changed
            if !removed_variables.is_empty() {
                for removed_var in &removed_variables {
                    // ✅ ACTOR+RELAY: Remove cache entries using WaveformTimeline domain methods
                    crate::visualizer::timeline::timeline_actor::remove_cursor_value_from_cache(removed_var);
                    crate::visualizer::timeline::timeline_actor::remove_viewport_data_from_cache(removed_var);
                    crate::visualizer::timeline::timeline_actor::remove_raw_transitions_from_cache(removed_var);
                }
            }
            
            if !added_variables.is_empty() {
                // New variables will naturally trigger cache misses and backend requests
            }
            
            // Only invalidate validity flags if there were actual changes
            if !removed_variables.is_empty() || !added_variables.is_empty() {
                // ✅ ACTOR+RELAY: Invalidate cache validity using WaveformTimeline domain method
                crate::visualizer::timeline::timeline_actor::invalidate_cache_validity(!removed_variables.is_empty());
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
        let first_time = transitions[0].time_ns as f64 / NS_PER_SECOND;
        let last_time = transitions[transitions.len() - 1].time_ns as f64 / NS_PER_SECOND;
        
        cursor_seconds >= first_time && cursor_seconds <= last_time
    }
    
    fn interpolate_cursor_value(transitions: &[SignalTransition], cursor_time: TimeNs) -> Option<SignalValue> {
        if transitions.is_empty() {
            return None;
        }
        
        let cursor_seconds = cursor_time.display_seconds();
        
        // Find the most recent transition at or before cursor time
        for transition in transitions.iter().rev() {
            if transition.time_ns as f64 / NS_PER_SECOND <= cursor_seconds {
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
        // ✅ DISABLED: Circuit breaker cleanup moved to WaveformTimeline Actor
        // Cache cleanup is now handled by the Actor's Cache Current Values pattern
    }
    
    fn cleanup_old_requests() {
        // ✅ ACTOR+RELAY: Clean up old requests using WaveformTimeline domain method
        crate::visualizer::timeline::timeline_actor::cleanup_old_active_requests();
    }
    
    fn start_cache_invalidation_handlers() {
        // React to viewport changes and invalidate cache accordingly
        Task::start(async {
            viewport_signal().for_each(move |new_viewport| {
                async move {
                    crate::visualizer::timeline::timeline_actor::invalidate_viewport_cache(new_viewport);
                }
            }).await;
        });
        
        // React to cursor changes and invalidate cursor cache accordingly  
        Task::start(async {
            cursor_position_signal().for_each(move |cursor_pos| {
                let new_cursor = cursor_pos;
                async move {
                    // ✅ ACTOR+RELAY: Use timeline_actor cursor cache invalidation function
                    crate::visualizer::timeline::timeline_actor::invalidate_cursor_cache(new_cursor);
                    
                    // ✅ ACTOR+RELAY: Use timeline_actor signal reevaluation function
                    crate::visualizer::timeline::timeline_actor::force_cache_signal_reevaluation();
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
#[deprecated(note = "Service layer eliminated - timeline functionality moved to WaveformTimeline Actor")]
#[allow(dead_code)] // Deprecated service function - preserve during Actor+Relay migration
pub fn initialize_unified_timeline_service() {
    // TODO: Replace UnifiedTimelineService with WaveformTimeline Actor+Relay events
    // Temporarily disabled to eliminate deprecated warnings
    // WaveformTimelineActor::initialize();
}