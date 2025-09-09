//! Timeline cache management for signal data storage and optimization
//!
//! Centralized caching system for timeline signal data to eliminate scattered cache logic
//! and provide unified cache invalidation and performance tracking.

use crate::dataflow::{Actor, Relay, relay};
use shared::{SignalTransition, SignalValue, WaveformFile};
use zoon::*;
use futures::{StreamExt, select};
use std::collections::HashMap;

// Import time domain
use super::time_domain::{TimeNs, Viewport};

/// Main timeline cache structure for signal data storage and management
#[derive(Clone, Debug)]
pub struct TimelineCache {
    pub viewport_data: HashMap<String, ViewportSignalData>,
    pub cursor_values: HashMap<String, shared::SignalValue>,
    pub raw_transitions: HashMap<String, Vec<shared::SignalTransition>>,
    pub active_requests: HashMap<String, CacheRequestState>,
    pub metadata: CacheMetadata,
}

/// Signal data optimized for viewport rendering
#[derive(Clone, Debug)]
pub struct ViewportSignalData {
}

/// Request state for cache deduplication
#[derive(Clone, Debug)]
pub struct CacheRequestState {
    pub requested_signals: Vec<String>,
    pub _viewport: Option<Viewport>,
    pub timestamp_ns: TimeNs,
    pub request_type: CacheRequestType,
}

/// Types of requests to the cache system
#[derive(Clone, Debug, PartialEq)]
pub enum CacheRequestType {
    CursorValues,
}

/// Cache performance and validity metadata
#[derive(Clone, Debug)]
pub struct CacheMetadata {
    pub current_viewport: Viewport,
    pub current_cursor: TimeNs,
    pub statistics: shared::SignalStatistics,
    pub last_invalidation_ns: TimeNs,
    pub validity: CacheValidity,
}

/// Cache validity tracking
#[derive(Clone, Debug)]
pub struct CacheValidity {
    pub viewport_valid: bool,
    pub cursor_valid: bool,
}

impl TimelineCache {
    pub fn new() -> Self {
        TimelineCache {
            viewport_data: HashMap::new(),
            cursor_values: HashMap::new(),
            raw_transitions: HashMap::new(),
            active_requests: HashMap::new(),
            metadata: CacheMetadata {
                current_viewport: Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(10.0)),
                current_cursor: TimeNs::ZERO,
                statistics: shared::SignalStatistics {
                    total_signals: 0,
                    cached_signals: 0,
                    query_time_ms: 0,
                    cache_hit_ratio: 0.0,
                },
                last_invalidation_ns: TimeNs::ZERO,
                validity: CacheValidity {
                    viewport_valid: false,
                    cursor_valid: false,
                },
            },
        }
    }
    
    /// Invalidate cache when cursor position changes
    pub fn invalidate_cursor(&mut self, new_cursor: TimeNs) {
        self.metadata.current_cursor = new_cursor;
        self.metadata.validity.cursor_valid = false;
        self.metadata.last_invalidation_ns = new_cursor;
    }
    
    /// Invalidate cache when viewport changes
    pub fn invalidate_viewport(&mut self, new_viewport: Viewport) {
        self.metadata.current_viewport = new_viewport;
        self.metadata.validity.viewport_valid = false;
        self.metadata.last_invalidation_ns = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
    }
}

impl Default for TimelineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline cache controller with Actor+Relay architecture
#[derive(Clone, Debug)]
pub struct TimelineCacheController {
    /// Unified timeline cache - replaces 4 separate cache systems
    pub cache: Actor<TimelineCache>,
    
    /// System event relays
    pub data_loaded_relay: Relay<(String, WaveformFile)>,
    pub transitions_cached_relay: Relay<(String, Vec<SignalTransition>)>,
    pub cursor_values_updated_relay: Relay<std::collections::BTreeMap<String, SignalValue>>,
}

impl TimelineCacheController {
    pub async fn new(
        cursor_position: Actor<TimeNs>,
        viewport: Actor<Viewport>,
    ) -> Self {
        // Create relays for system events
        let (data_loaded_relay, data_loaded_stream) = relay::<(String, WaveformFile)>();
        let (transitions_cached_relay, transitions_cached_stream) = relay::<(String, Vec<SignalTransition>)>();
        let (cursor_values_updated_relay, cursor_values_updated_stream) = relay::<std::collections::BTreeMap<String, SignalValue>>();
        
        // Create timeline cache actor (Actor+Relay pattern)
        let cache = Actor::new(TimelineCache::new(), {
            let cursor_position_for_cache = cursor_position.clone();
            let viewport_for_cache = viewport.clone();
            async move |cache_handle| {
                let mut transitions_cached = transitions_cached_stream;
                let mut data_loaded = data_loaded_stream;
                let mut cursor_values_updated = cursor_values_updated_stream;
                
                // Watch cursor and viewport changes
                let mut cursor_stream = cursor_position_for_cache.signal().to_stream().fuse();
                let mut viewport_stream = viewport_for_cache.signal().to_stream().fuse();
                
                loop {
                    select! {
                        event = transitions_cached.next() => {
                            match event {
                                Some((signal_id, transitions)) => {
                                    cache_handle.lock_mut().raw_transitions.insert(signal_id, transitions);
                                }
                                None => break,
                            }
                        }
                        event = data_loaded.next() => {
                            match event {
                                Some((file_id, waveform_data)) => {
                                    // Process waveform data into cache
                                    for scope_data in &waveform_data.scopes {
                                        for signal_data in &scope_data.variables {
                                            let signal_id = format!("{}|{}|{}", file_id, scope_data.name, signal_data.name);
                                            // Initialize empty transitions for this signal
                                            cache_handle.lock_mut().raw_transitions.entry(signal_id).or_insert_with(Vec::new);
                                        }
                                    }
                                }
                                None => break,
                            }
                        }
                        event = cursor_values_updated.next() => {
                            match event {
                                Some(values) => {
                                    let mut cache = cache_handle.lock_mut();
                                    for (signal_id, value) in values {
                                        cache.cursor_values.insert(signal_id, value);
                                    }
                                }
                                None => break,
                            }
                        }
                        cursor_update = cursor_stream.next() => {
                            match cursor_update {
                                Some(new_cursor) => {
                                    cache_handle.lock_mut().invalidate_cursor(new_cursor);
                                }
                                None => break,
                            }
                        }
                        viewport_update = viewport_stream.next() => {
                            match viewport_update {
                                Some(new_viewport) => {
                                    cache_handle.lock_mut().invalidate_viewport(new_viewport);
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        Self {
            cache,
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
        }
    }
}