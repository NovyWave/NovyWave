//! Canvas state management for timeline rendering and performance tracking
//!
//! Manages all canvas-related state including dimensions, cache, redraw coordination,
//! and performance tracking for timeline rendering optimization.

use crate::dataflow::{Actor, ActorMap, Relay, relay};
use shared::SignalValue;
use zoon::*;
use futures::{StreamExt, select};
use std::collections::BTreeMap;

// Import time domain
use super::time_domain::Viewport;

// Canvas dimension constants - extracted from hardcoded values
const DEFAULT_CANVAS_WIDTH: f32 = 800.0;
const DEFAULT_CANVAS_HEIGHT: f32 = 400.0;

/// Timeline statistics and metadata
#[derive(Clone, Debug, Default)]
pub struct TimelineStats {
    pub total_signals: usize,
    pub cached_transitions: usize,
    pub min_time: f64,
    pub max_time: f64,
    pub time_range: f64,
}

/// Canvas state controller with Actor+Relay architecture
#[derive(Clone, Debug)]
pub struct CanvasStateController {
    /// Canvas dimensions for coordinate calculations
    pub canvas_width: Actor<f32>,
    pub canvas_height: Actor<f32>,
    
    /// Canvas state actors
    pub has_pending_request: Actor<bool>,
    pub canvas_cache: ActorMap<String, Vec<(f32, SignalValue)>>,
    pub force_redraw: Actor<u32>,
    pub last_redraw_time: Actor<f64>,
    pub last_canvas_update: Actor<u64>,
    pub timeline_stats: Actor<TimelineStats>,
    
    /// Canvas event relays
    pub canvas_resized_relay: Relay<(f32, f32)>,
    pub redraw_requested_relay: Relay<()>,
}

impl CanvasStateController {
    pub async fn new(
        viewport: Actor<Viewport>,
        signal_values: ActorMap<String, SignalValue>,
        cache: Actor<super::timeline_cache::TimelineCache>,
    ) -> Self {
        // Create canvas event relays
        let (canvas_resized_relay, canvas_resized_stream) = relay::<(f32, f32)>();
        let (redraw_requested_relay, redraw_requested_stream) = relay::<()>();
        
        // Canvas dimension actors
        let canvas_width = Actor::new(DEFAULT_CANVAS_WIDTH, {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            async move |width_handle| {
                let mut canvas_resized = canvas_resized_relay_clone.subscribe();
            
            loop {
                select! {
                    event = canvas_resized.next() => {
                        match event {
                            Some((width, _height)) => {
                                width_handle.set(width);
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        let canvas_height = Actor::new(DEFAULT_CANVAS_HEIGHT, {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            async move |height_handle| {
                let mut canvas_resized_stream = canvas_resized_relay_clone.subscribe();
                
                loop {
                    select! {
                        event = canvas_resized_stream.next() => {
                            match event {
                                Some((_width, height)) => height_handle.set(height),
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        let has_pending_request = Actor::new(false, {
            let cache_for_request_tracking = cache.clone();
            async move |request_handle| {
                // Watch cache active_requests to track pending status
                let mut cache_stream = cache_for_request_tracking.signal().to_stream().fuse();
                
                loop {
                    select! {
                        cache_update = cache_stream.next() => {
                            match cache_update {
                                Some(cache_state) => {
                                    let has_requests = !cache_state.active_requests.is_empty();
                                    request_handle.set_neq(has_requests);
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        let canvas_cache = ActorMap::new(BTreeMap::new(), {
            let viewport_for_canvas_cache = viewport.clone();
            let signal_values_for_canvas = signal_values.clone();
            async move |canvas_cache_handle| {
                // Watch viewport and signal values changes to update canvas cache
                let mut viewport_stream = viewport_for_canvas_cache.signal().to_stream().fuse();
                let mut signal_values_stream = signal_values_for_canvas.entries_signal_vec().to_signal_cloned().to_stream().fuse();
                
                loop {
                    select! {
                        viewport_update = viewport_stream.next() => {
                            match viewport_update {
                                Some(_new_viewport) => {
                                    // Clear canvas cache when viewport changes significantly
                                    canvas_cache_handle.lock_mut().clear();
                                }
                                None => break,
                            }
                        }
                        signal_update = signal_values_stream.next() => {
                            match signal_update {
                                Some(values) => {
                                    // Update canvas cache with processed rendering data
                                    for (signal_id, value) in values {
                                        // Create processed canvas data for rendering optimization
                                        let canvas_data = vec![(0.0f32, value)]; // Placeholder rendering data
                                        canvas_cache_handle.lock_mut().insert_cloned(signal_id, canvas_data);
                                    }
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        let force_redraw = Actor::new(0_u32, async move |redraw_handle| {
            let mut redraw_requested = redraw_requested_stream;
            
            loop {
                select! {
                    event = redraw_requested.next() => {
                        match event {
                            Some(()) => {
                                redraw_handle.update_mut(|counter| {
                                    *counter += 1;
                                });
                            },
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let last_redraw_time = Actor::new(0.0_f64, {
            let force_redraw_for_timing = force_redraw.clone();
            async move |redraw_time_handle| {
                // Watch force_redraw counter to track redraw timing
                let mut redraw_stream = force_redraw_for_timing.signal().to_stream().fuse();
                
                loop {
                    select! {
                        redraw_update = redraw_stream.next() => {
                            match redraw_update {
                                Some(_counter) => {
                                    // Record current timestamp when redraw happens
                                    let current_time = js_sys::Date::now() / 1000.0; // Convert to seconds
                                    redraw_time_handle.set(current_time);
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        let last_canvas_update = Actor::new(0_u64, {
            let canvas_cache_for_update_tracking = canvas_cache.clone();
            async move |canvas_update_handle| {
                // Watch canvas_cache changes to track last update time
                let mut canvas_stream = canvas_cache_for_update_tracking.entries_signal_vec().to_signal_cloned().to_stream().fuse();
                
                loop {
                    select! {
                        canvas_update = canvas_stream.next() => {
                            match canvas_update {
                                Some(_cache_state) => {
                                    // Record current timestamp when canvas cache updates
                                    let current_time = js_sys::Date::now() as u64;
                                    canvas_update_handle.set(current_time);
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        let timeline_stats = Actor::new(TimelineStats::default(), {
            let cache_for_stats = cache.clone();
            let signal_values_for_stats = signal_values.clone();
            let viewport_for_stats = viewport.clone();
            async move |stats_handle| {
                // Watch cache, signal values, and viewport to calculate statistics
                let mut cache_stream = cache_for_stats.signal().to_stream().fuse();
                let mut signal_values_stream = signal_values_for_stats.entries_signal_vec().to_signal_cloned().to_stream().fuse();
                let mut viewport_stream = viewport_for_stats.signal().to_stream().fuse();
                
                loop {
                    select! {
                        cache_update = cache_stream.next() => {
                            match cache_update {
                                Some(cache_state) => {
                                    stats_handle.update_mut(|stats| {
                                        stats.cached_transitions = cache_state.raw_transitions.len();
                                    });
                                }
                                None => break,
                            }
                        }
                        signal_update = signal_values_stream.next() => {
                            match signal_update {
                                Some(values) => {
                                    stats_handle.update_mut(|stats| {
                                        stats.total_signals = values.len();
                                    });
                                }
                                None => break,
                            }
                        }
                        viewport_update = viewport_stream.next() => {
                            match viewport_update {
                                Some(viewport) => {
                                    stats_handle.update_mut(|stats| {
                                        stats.min_time = viewport.start.display_seconds();
                                        stats.max_time = viewport.end.display_seconds();
                                        stats.time_range = stats.max_time - stats.min_time;
                                    });
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
            canvas_width,
            canvas_height,
            has_pending_request,
            canvas_cache,
            force_redraw,
            last_redraw_time,
            last_canvas_update,
            timeline_stats,
            canvas_resized_relay,
            redraw_requested_relay,
        }
    }
}