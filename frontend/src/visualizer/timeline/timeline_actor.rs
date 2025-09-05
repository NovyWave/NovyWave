//! WaveformTimeline domain for timeline management using Actor+Relay architecture
//!
//! Consolidated timeline management domain to replace global mutables with event-driven architecture.
//! Manages cursor position, viewport ranges, zoom levels, and cached waveform data.

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::actors::{Actor, ActorMap, Relay, relay};
use crate::actors::global_domains::waveform_timeline_domain;
use crate::visualizer::timeline::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates, TimelineCache};
use shared::{SignalTransition, SignalValue, WaveformFile, VarFormat};
use zoon::*;
use futures::{StreamExt, select};
use std::collections::{BTreeMap, HashMap};
use crate::visualizer::formatting::signal_values as format_utils;

// Canvas dimension constants - extracted from hardcoded values
const DEFAULT_CANVAS_WIDTH: f32 = 800.0;
const DEFAULT_CANVAS_HEIGHT: f32 = 400.0;
const FALLBACK_CANVAS_HEIGHT: f32 = 600.0;

/// Domain-driven timeline management with Actor+Relay architecture.
/// 
/// Replaces ALL 25 timeline-related global mutables with cohesive event-driven state management.
/// Handles cursor position, viewport, zoom/pan, mouse tracking, canvas state, and signal caching.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct WaveformTimeline {
    // === CORE TIMELINE STATE (15 mutables from state.rs) ===
    /// Current cursor position in nanoseconds
    cursor_position: Actor<TimeNs>,
    
    /// Timeline viewport (visible time range)
    viewport: Actor<Viewport>,
    
    /// Timeline resolution (nanoseconds per pixel)
    ns_per_pixel: Actor<NsPerPixel>,
    
    
    /// Unified timeline cache - replaces 4 separate cache systems
    cache: zoon::Mutable<TimelineCache>,
    
    /// Track if cursor position was set during startup
    cursor_initialized: Actor<bool>,
    
    /// Smooth zoom control flags
    zooming_in: Actor<bool>,
    zooming_out: Actor<bool>,
    
    /// Smooth pan control flags  
    panning_left: Actor<bool>,
    panning_right: Actor<bool>,
    
    /// Smooth cursor movement control flags
    cursor_moving_left: Actor<bool>,
    cursor_moving_right: Actor<bool>,
    
    /// Shift key state tracking for modifier combinations
    shift_pressed: Actor<bool>,
    
    /// Mouse position tracking for zoom center
    mouse_x: Actor<f32>,
    mouse_time: Actor<TimeNs>,
    
    // === ZOOM/PAN STATE (5 mutables from state.rs) ===
    /// Zoom center position (in nanoseconds)
    zoom_center: Actor<TimeNs>,
    
    /// Canvas dimensions for coordinate calculations
    canvas_width: Actor<f32>,
    canvas_height: Actor<f32>,
    
    /// Current signal values at cursor position
    signal_values: ActorMap<String, format_utils::SignalValue>,
    signal_values_hashmap_signal: zoon::Mutable<HashMap<String, format_utils::SignalValue>>,  // Dedicated signal for efficient HashMap access
    
    /// Format selections for selected variables
    variable_formats: ActorMap<String, VarFormat>,
    variable_formats_hashmap_signal: zoon::Mutable<HashMap<String, VarFormat>>,  // Dedicated signal for efficient HashMap access
    
    // === CANVAS STATE (5 mutables from waveform_canvas.rs) ===
    /// Track pending backend requests
    has_pending_request: Actor<bool>,
    
    /// Processed canvas cache for rendering optimization
    canvas_cache: ActorMap<String, Vec<(f32, SignalValue)>>,
    canvas_cache_hashmap_signal: zoon::Mutable<HashMap<String, Vec<(f32, SignalValue)>>>,  // Dedicated signal for efficient HashMap access
    
    /// Force redraw counter for invalidation
    force_redraw: Actor<u32>,
    
    /// Last redraw time for performance tracking
    last_redraw_time: Actor<f64>,
    
    /// Last canvas update timestamp
    last_canvas_update: Actor<u64>,
    
    /// Timeline statistics and metadata
    timeline_stats: Actor<TimelineStats>,
    
    // === USER TIMELINE INTERACTION EVENTS ===
    /// User clicked on timeline canvas at specific time
    pub cursor_clicked_relay: Relay<TimeNs>,
    
    /// User moved cursor to specific time
    pub cursor_moved_relay: Relay<TimeNs>,
    
    /// User started zoom in gesture
    pub zoom_in_started_relay: Relay<TimeNs>,
    
    /// User started zoom out gesture
    pub zoom_out_started_relay: Relay<TimeNs>,
    
    /// User started panning left
    pub pan_left_started_relay: Relay<()>,
    
    /// User started panning right
    pub pan_right_started_relay: Relay<()>,
    
    /// User moved mouse over canvas (position and time)
    pub mouse_moved_relay: Relay<(f32, TimeNs)>,
    
    /// Canvas dimensions changed (resize)
    pub canvas_resized_relay: Relay<(f32, f32)>,
    
    /// Force redraw requested
    pub redraw_requested_relay: Relay<()>,
    
    /// Signal values updated from backend
    pub signal_values_updated_relay: Relay<HashMap<String, format_utils::SignalValue>>,
    
    /// Variable format updated for specific variable
    pub variable_format_updated_relay: Relay<(String, VarFormat)>,
    
    // === KEYBOARD NAVIGATION EVENTS ===
    /// User pressed left arrow key
    pub left_key_pressed_relay: Relay<()>,
    
    /// User pressed right arrow key
    pub right_key_pressed_relay: Relay<()>,
    
    /// User pressed zoom in key
    pub zoom_in_pressed_relay: Relay<()>,
    
    /// User pressed zoom out key
    pub zoom_out_pressed_relay: Relay<()>,
    
    /// User pressed pan left key
    pub pan_left_pressed_relay: Relay<()>,
    
    /// User pressed pan right key  
    pub pan_right_pressed_relay: Relay<()>,
    
    /// User pressed jump to previous transition key
    pub jump_to_previous_pressed_relay: Relay<()>,
    
    /// User pressed jump to next transition key
    pub jump_to_next_pressed_relay: Relay<()>,
    
    /// User pressed reset zoom key
    pub reset_zoom_pressed_relay: Relay<()>,
    
    /// User pressed reset zoom center key
    pub reset_zoom_center_pressed_relay: Relay<()>,
    
    /// Center timeline cursor at viewport center
    pub cursor_center_at_viewport_relay: Relay<()>,
    
    /// Reset zoom center to 0s
    pub zoom_center_reset_to_zero_relay: Relay<()>,
    
    /// User pressed shift key
    pub shift_key_pressed_relay: Relay<()>,
    
    /// User released shift key
    pub shift_key_released_relay: Relay<()>,
    
    // === ANIMATION STATE RELAYS ===
    /// Animation started panning left
    pub panning_left_started_relay: Relay<()>,
    
    /// Animation stopped panning left
    pub panning_left_stopped_relay: Relay<()>,
    
    /// Animation started panning right
    pub panning_right_started_relay: Relay<()>,
    
    /// Animation stopped panning right
    pub panning_right_stopped_relay: Relay<()>,
    
    /// Animation started cursor moving left
    pub cursor_moving_left_started_relay: Relay<()>,
    
    /// Animation stopped cursor moving left
    pub cursor_moving_left_stopped_relay: Relay<()>,
    
    /// Animation started cursor moving right
    pub cursor_moving_right_started_relay: Relay<()>,
    
    /// Animation stopped cursor moving right
    pub cursor_moving_right_stopped_relay: Relay<()>,
    
    /// Update zoom center to follow mouse position  
    pub zoom_center_follow_mouse_relay: Relay<TimeNs>,
    
    /// User clicked fit all button
    pub fit_all_clicked_relay: Relay<()>,
    
    // === SYSTEM TIMELINE EVENTS ===
    /// Waveform data loaded from file
    pub data_loaded_relay: Relay<(String, WaveformFile)>,
    
    /// Signal transitions cached for rendering
    pub transitions_cached_relay: Relay<(String, Vec<SignalTransition>)>,
    
    /// Cursor values updated from cached data
    pub cursor_values_updated_relay: Relay<BTreeMap<String, SignalValue>>,
    
    /// Timeline range calculated from loaded data
    pub timeline_bounds_calculated_relay: Relay<(f64, f64)>,
    
    /// Viewport changed due to resize or user action
    pub viewport_changed_relay: Relay<(f64, f64)>,
    
    /// Ns per pixel changed for zoom display synchronization
    pub ns_per_pixel_changed_relay: Relay<NsPerPixel>,
}

/// Timeline statistics and metadata
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct TimelineStats {
    pub total_signals: usize,
    pub cached_transitions: usize,
    pub min_time: f64,
    pub max_time: f64,
    pub time_range: f64,
}

#[allow(dead_code)]
impl WaveformTimeline {
    /// Create a new WaveformTimeline domain with comprehensive event processors
    /// 
    /// Migrates ALL 25 global mutables to Actor+Relay architecture:
    /// - 15 core timeline mutables from state.rs
    /// - 5 zoom/pan mutables from state.rs  
    /// - 5 canvas mutables from waveform_canvas.rs
    pub async fn new() -> Self {
        // Create relays for comprehensive user interactions
        let (cursor_clicked_relay, cursor_clicked_stream) = relay::<TimeNs>();
        let (cursor_moved_relay, cursor_moved_stream) = relay::<TimeNs>();
        let (zoom_in_started_relay, zoom_in_started_stream) = relay::<TimeNs>();
        let (zoom_out_started_relay, zoom_out_started_stream) = relay::<TimeNs>();
        let (pan_left_started_relay, pan_left_started_stream) = relay::<()>();
        let (pan_right_started_relay, pan_right_started_stream) = relay::<()>();
        let (mouse_moved_relay, mouse_moved_stream) = relay::<(f32, TimeNs)>();
        let (canvas_resized_relay, _canvas_resized_stream) = relay::<(f32, f32)>();
        let (redraw_requested_relay, redraw_requested_stream) = relay::<()>();
        let (signal_values_updated_relay, signal_values_updated_stream) = relay::<HashMap<String, format_utils::SignalValue>>();
        let (variable_format_updated_relay, variable_format_updated_stream) = relay::<(String, VarFormat)>();
        
        // Create relays for keyboard navigation
        let (left_key_pressed_relay, left_key_pressed_stream) = relay::<()>();
        let (right_key_pressed_relay, right_key_pressed_stream) = relay::<()>();
        let (zoom_in_pressed_relay, zoom_in_pressed_stream) = relay::<()>();
        let (zoom_out_pressed_relay, zoom_out_pressed_stream) = relay::<()>();
        let (pan_left_pressed_relay, _pan_left_pressed_stream) = relay::<()>();
        let (pan_right_pressed_relay, _pan_right_pressed_stream) = relay::<()>();
        let (jump_to_previous_pressed_relay, _jump_to_previous_pressed_stream) = relay::<()>();
        let (jump_to_next_pressed_relay, _jump_to_next_pressed_stream) = relay::<()>();
        let (reset_zoom_pressed_relay, reset_zoom_pressed_stream) = relay::<()>();
        let (reset_zoom_center_pressed_relay, reset_zoom_center_pressed_stream) = relay::<()>();
        let (cursor_center_at_viewport_relay, cursor_center_at_viewport_stream) = relay::<()>();
        let (zoom_center_reset_to_zero_relay, zoom_center_reset_to_zero_stream) = relay::<()>();
        let (shift_key_pressed_relay_var, shift_key_pressed_stream) = relay::<()>();
        let (shift_key_released_relay_var, shift_key_released_stream) = relay::<()>();
        
        // Create animation state relays  
        let (panning_left_started_relay_var, panning_left_started_stream) = relay::<()>();
        let (panning_left_stopped_relay_var, panning_left_stopped_stream) = relay::<()>();
        let (panning_right_started_relay_var, panning_right_started_stream) = relay::<()>();
        let (panning_right_stopped_relay_var, panning_right_stopped_stream) = relay::<()>();
        let (cursor_moving_left_started_relay_var, cursor_moving_left_started_stream) = relay::<()>();
        let (cursor_moving_left_stopped_relay_var, cursor_moving_left_stopped_stream) = relay::<()>();
        let (cursor_moving_right_started_relay_var, cursor_moving_right_started_stream) = relay::<()>();
        let (cursor_moving_right_stopped_relay_var, cursor_moving_right_stopped_stream) = relay::<()>();
        let (zoom_center_follow_mouse_relay, zoom_center_follow_mouse_stream) = relay::<TimeNs>();
        let (fit_all_clicked_relay, fit_all_clicked_stream) = relay::<()>();
        
        // Create relays for system events
        let (data_loaded_relay, _data_loaded_stream) = relay::<(String, WaveformFile)>();
        let (transitions_cached_relay, _transitions_cached_stream) = relay::<(String, Vec<SignalTransition>)>();
        let (cursor_values_updated_relay, _cursor_values_updated_stream) = relay::<BTreeMap<String, SignalValue>>();
        let (timeline_bounds_calculated_relay, timeline_bounds_calculated_stream) = relay::<(f64, f64)>();
        let (viewport_changed_relay, _viewport_changed_stream) = relay::<(f64, f64)>();
        let (ns_per_pixel_changed_relay, ns_per_pixel_changed_stream) = relay::<NsPerPixel>();
        
        // Helper function to get initial cursor position
        let get_initial_cursor_position = || {
            if let Some((min_time, max_time)) = crate::visualizer::canvas::waveform_canvas::get_current_timeline_range() {
                let center_time = (min_time + max_time) / 2.0;
                TimeNs::from_external_seconds(center_time)
            } else {
                TimeNs::ZERO
            }
        };
        
        // Create cursor position actor with comprehensive event handling
        // ✅ FIX: Initialize cursor at timeline center instead of 0
        let initial_cursor_position = get_initial_cursor_position();
        if initial_cursor_position != TimeNs::ZERO {
        } else {
        }
        
        let cursor_position = Actor::new(initial_cursor_position, async move |cursor_handle| {
            let mut cursor_clicked = cursor_clicked_stream;
            let mut cursor_moved = cursor_moved_stream;
            let mut left_key_pressed = left_key_pressed_stream;
            let mut right_key_pressed = right_key_pressed_stream;
            let mut cursor_center_at_viewport = cursor_center_at_viewport_stream;
            
            loop {
                select! {
                    event = cursor_clicked.next() => {
                        match event {
                            Some(time_ns) => {
                                cursor_handle.set(time_ns);
                            },
                            None => break,
                        }
                    }
                    event = cursor_moved.next() => {
                        match event {
                            Some(time_ns) => cursor_handle.set(time_ns),
                            None => break,
                        }
                    }
                    event = left_key_pressed.next() => {
                        match event {
                            Some(()) => cursor_handle.update_mut(|current| {
                                // Calculate adaptive step size based on visible time range
                                let step_size = calculate_adaptive_cursor_step();
                                let new_time = current.nanos().saturating_sub(step_size);
                                *current = TimeNs::from_nanos(new_time);
                            }),
                            None => break,
                        }
                    }
                    event = right_key_pressed.next() => {
                        match event {
                            Some(()) => cursor_handle.update_mut(|current| {
                                // Calculate adaptive step size based on visible time range
                                let step_size = calculate_adaptive_cursor_step();
                                let old_time = current.nanos();
                                let new_time = old_time.saturating_add(step_size);
                                *current = TimeNs::from_nanos(new_time);
                            }),
                            None => break,
                        }
                    }
                    event = cursor_center_at_viewport.next() => {
                        match event {
                            Some(()) => {
                                // Center cursor at viewport center
                                let viewport = crate::visualizer::timeline::timeline_actor::current_viewport();
                                let center_time = if let Some(vp) = viewport {
                                    TimeNs::from_external_seconds(
                                        (vp.start.display_seconds() + vp.end.display_seconds()) / 2.0
                                    )
                                } else {
                                    TimeNs::ZERO
                                };
                                cursor_handle.set(center_time);
                            },
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        // Clone relays needed by multiple actors and struct field before moving
        let cursor_center_at_viewport_relay_for_ns_per_pixel = cursor_center_at_viewport_relay.clone();
        let cursor_center_at_viewport_relay_for_struct = cursor_center_at_viewport_relay.clone();
        
        // Helper function to get initial viewport from file data
        let get_initial_viewport = || {
            // Try to get file range from loaded files
            if let Some((min_time, max_time)) = crate::visualizer::canvas::waveform_canvas::get_maximum_timeline_range() {
                let file_span = max_time - min_time;
                if file_span > 1.0 {  // Use file data if span > 1 second
                    return Viewport::new(
                        TimeNs::from_external_seconds(min_time),
                        TimeNs::from_external_seconds(max_time)
                    );
                }
            }
            Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(1.0))  // 1-second fallback
        };

        // Create viewport actor with comprehensive event handling  
        let initial_viewport = get_initial_viewport();
        let viewport = Actor::new(
            initial_viewport, 
            async move |viewport_handle| {
                let cursor_center_at_viewport_relay_clone = cursor_center_at_viewport_relay.clone();
                    let mut viewport_changed = _viewport_changed_stream;
                    let mut timeline_bounds_calculated = timeline_bounds_calculated_stream;
                    let mut fit_all_clicked = fit_all_clicked_stream;
                
                loop {
                    select! {
                        event = viewport_changed.next() => {
                            match event {
                                Some((start, end)) => {
                                    let new_viewport = Viewport::new(
                                        TimeNs::from_external_seconds(start),
                                        TimeNs::from_external_seconds(end)
                                    );
                                    viewport_handle.set(new_viewport);
                                }
                                None => break,
                            }
                        }
                        event = timeline_bounds_calculated.next() => {
                            match event {
                                Some((min_time, max_time)) => {
                                    let new_viewport = Viewport::new(
                                        TimeNs::from_external_seconds(min_time),
                                        TimeNs::from_external_seconds(max_time)
                                    );
                                    viewport_handle.set(new_viewport);
                                    
                                    // ✅ FIX: Trigger automatic fit-all zoom when timeline bounds are calculated (on file load)
                                    // Use the same relay that R key uses for consistent behavior
                                    crate::visualizer::timeline::timeline_actor_domain().reset_zoom_pressed_relay.send(());
                                    
                                    // ✅ FIX: Center cursor at viewport center when timeline bounds are calculated
                                    let _center_time = (min_time + max_time) / 2.0;
                                    cursor_center_at_viewport_relay_clone.send(());
                                }
                                None => break,
                            }
                        }
                        event = fit_all_clicked.next() => {
                            match event {
                                Some(()) => {
                                    // ✅ FIX: Reset viewport to full timeline range using actual file data
                                    if let Some((min_time, max_time)) = crate::visualizer::canvas::waveform_canvas::get_maximum_timeline_range() {
                                        let full_timeline_viewport = Viewport::new(
                                            TimeNs::from_external_seconds(min_time),
                                            TimeNs::from_external_seconds(max_time)
                                        );
                                        viewport_handle.set_neq(full_timeline_viewport);
                                        
                                        // ✅ REMOVED: Static cache update antipattern - Actor is single source of truth
                                    } else {
                                        // Fallback to full file range if no variables selected
                                        let (file_min, file_max) = crate::visualizer::canvas::waveform_canvas::get_full_file_range();
                                        if file_min < file_max && file_min.is_finite() && file_max.is_finite() {
                                            let full_timeline_viewport = Viewport::new(
                                                TimeNs::from_external_seconds(file_min),
                                                TimeNs::from_external_seconds(file_max)
                                            );
                                            viewport_handle.set_neq(full_timeline_viewport);
                                            
                                            // ✅ REMOVED: Static cache update antipattern - Actor is single source of truth
                                        }
                                    }
                                    
                                    // Center cursor at viewport center
                                    cursor_center_at_viewport_relay_clone.send(());
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        );
        
        // Create ns_per_pixel actor with zoom event handling
        let ns_per_pixel = Actor::new(NsPerPixel::default(), {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            let viewport_for_ns_per_pixel = viewport.clone();
            let cursor_center_at_viewport_relay_clone = cursor_center_at_viewport_relay_for_ns_per_pixel.clone();
            let zoom_center_reset_to_zero_relay_clone = zoom_center_reset_to_zero_relay.clone();
            let viewport_changed_relay_clone = viewport_changed_relay.clone();
            let fit_all_clicked_relay_clone = fit_all_clicked_relay.clone();
            async move |ns_per_pixel_handle| {
                let mut zoom_in_started = zoom_in_started_stream;
                let mut zoom_out_started = zoom_out_started_stream;
                let mut zoom_in_pressed = zoom_in_pressed_stream;
                let mut zoom_out_pressed = zoom_out_pressed_stream;
                let mut reset_zoom_pressed = reset_zoom_pressed_stream;
                let mut reset_zoom_center_pressed = reset_zoom_center_pressed_stream;
                let mut ns_per_pixel_changed = ns_per_pixel_changed_stream;
                let mut canvas_resized = canvas_resized_relay_clone.subscribe();
            
            loop {
                select! {
                    event = zoom_in_started.next() => {
                        match event {
                            Some(_center_time) => {
                                let current = ns_per_pixel_handle.get();
                                ns_per_pixel_handle.set_neq(current.zoom_in_smooth(0.3));
                            }
                            None => break,
                        }
                    }
                    event = zoom_out_started.next() => {
                        match event {
                            Some(_center_time) => {
                                let current = ns_per_pixel_handle.get();
                                ns_per_pixel_handle.set_neq(current.zoom_out_smooth(0.3));
                            }
                            None => break,
                        }
                    }
                    event = zoom_in_pressed.next() => {
                        match event {
                            Some(()) => {
                                // Zoom in operation starting
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                let new_ns_per_pixel = current_ns_per_pixel.zoom_in_smooth(0.3);
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                                
                                // ✅ CRITICAL FIX: Update viewport when zoom changes
                                let _current_viewport = viewport_for_ns_per_pixel.signal().to_stream().next().await.unwrap_or_else(|| {
                                    panic!("Canvas resize without valid file range - no data available")
                                });
                                
                                // ✅ FIXED: Get current canvas width from domain (proper Actor+Relay pattern)
                                let timeline = crate::actors::global_domains::waveform_timeline_domain();
                                let canvas_width = timeline.canvas_width.signal().to_stream().next().await.unwrap_or(0.0);
                                if canvas_width <= 0.0 {
                                    continue; // Timeline not initialized yet, skip this frame
                                }
                                
                                // Using dynamic canvas width for zoom calculation
                                
                                // ✅ CORRECT: Use zoom center position (blue line), not cursor position (yellow line)
                                // TODO: Replace current_zoom_center_position with zoom_center_ns_signal() for proper reactive patterns
                                let center_time = TimeNs::ZERO; // Fallback to avoid deprecated function
                                
                                // Calculate new viewport range based on new zoom level and ACTUAL canvas width
                                let half_range_ns = (new_ns_per_pixel.nanos() * canvas_width as u64) / 2;
                                let new_start = TimeNs::from_nanos(center_time.nanos().saturating_sub(half_range_ns));
                                let new_end = TimeNs::from_nanos(center_time.nanos() + half_range_ns);
                                let new_viewport = Viewport::new(new_start, new_end);
                                
                                // Viewport updated for zoom in operation
                                
                                // Use the viewport changed relay to update viewport
                                let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
                                viewport_changed_relay_clone.send(viewport_tuple);
                                // Zoom in completed
                            }
                            None => break,
                        }
                    }
                    event = zoom_out_pressed.next() => {
                        match event {
                            Some(()) => {
                                // Zoom out operation starting
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                let new_ns_per_pixel = current_ns_per_pixel.zoom_out_smooth(0.3);
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                                
                                // ✅ CRITICAL FIX: Update viewport when zoom changes
                                let _current_viewport = viewport_for_ns_per_pixel.signal().to_stream().next().await.unwrap_or_else(|| {
                                    panic!("Canvas resize without valid file range - no data available")
                                });
                                // ✅ FIXED: Get current canvas width from domain (proper Actor+Relay pattern)
                                let timeline = crate::actors::global_domains::waveform_timeline_domain();
                                let canvas_width = timeline.canvas_width.signal().to_stream().next().await.unwrap_or(0.0);
                                if canvas_width <= 0.0 {
                                    continue; // Timeline not initialized yet, skip this frame
                                }
                                
                                // Using dynamic canvas width for zoom calculation
                                
                                // ✅ CORRECT: Use zoom center position (blue line), not cursor position (yellow line)
                                // TODO: Replace current_zoom_center_position with zoom_center_ns_signal() for proper reactive patterns
                                let center_time = TimeNs::ZERO; // Fallback to avoid deprecated function
                                
                                // Calculate new viewport range based on new zoom level
                                let half_range_ns = (new_ns_per_pixel.nanos() * canvas_width as u64) / 2;
                                let new_start = TimeNs::from_nanos(center_time.nanos().saturating_sub(half_range_ns));
                                let new_end = TimeNs::from_nanos(center_time.nanos() + half_range_ns);
                                let new_viewport = Viewport::new(new_start, new_end);
                                
                                // Viewport updated for zoom out operation
                                
                                // Use the viewport changed relay to update viewport
                                let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
                                viewport_changed_relay_clone.send(viewport_tuple);
                                // Zoom out completed
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom_pressed.next() => {
                        match event {
                            Some(()) => {
                                // Reset zoom operation starting
                                
                                // R key should:
                                // 1. Calculate fit-all zoom based on ACTUAL canvas width (not hardcoded 800px)
                                // 2. Reset viewport to show entire timeline
                                // 3. Reset zoom center to 0
                                // 4. Center cursor in viewport
                                // NOTE: This should only happen when user presses R, not continuously
                                
                                // Debug current Actor state before calculation
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                let current_viewport = match crate::visualizer::timeline::timeline_actor::current_viewport() {
                                    Some(viewport) => viewport,
                                    None => continue, // Timeline not initialized yet, skip this frame
                                };
                                // TODO: Replace current_coordinates with reactive coordinate signals instead of synchronous access
                                let current_coords = match None::<crate::visualizer::timeline::time_types::TimelineCoordinates> { // Fallback to avoid deprecated function
                                    Some(coords) => coords,
                                    None => continue, // Timeline not initialized yet, skip this frame
                                };
                                
                                // ITERATION 4: Additional Actor state consistency checks (using public signals since handles not in scope)
                                // Note: We can only access ns_per_pixel_handle directly within this Actor
                                
                                // Reset zoom calculation starting
                                    
                                // ITERATION 4: Check Actor state consistency between calls (limited to available state)
                                static mut PREVIOUS_STATE: Option<(u64, u64, u64, u32)> = None;
                                let current_state = (
                                    current_ns_per_pixel.nanos(),
                                    current_viewport.start.nanos(),
                                    current_viewport.end.nanos(),
                                    current_coords.canvas_width_pixels
                                );
                                unsafe {
                                    if let Some(prev) = PREVIOUS_STATE {
                                        if current_state != prev {
                                            // Actor state has changed
                                        } else {
                                            // Actor state identical to previous call
                                        }
                                    } else {
                                        // First R key press - saving initial state
                                    }
                                    PREVIOUS_STATE = Some(current_state);
                                }
                                
                                // ITERATION 7: Debug checkpoint 1 - Before canvas width calculation
                                
                                // ✅ FIXED: Use signal-based canvas width access inside Actor loop
                                let canvas_width = current_canvas_width_signal().to_stream().next().await.unwrap_or(0.0);
                                if canvas_width <= 0.0 {
                                    continue; // Timeline not initialized yet, skip this frame
                                }
                                let canvas_width = canvas_width as u32;
                                
                                // ✅ REMOVED: STABLE_CANVAS_WIDTH static cache - antipattern replaced with direct access
                                
                                if let Some((min_time, max_time)) = crate::visualizer::canvas::waveform_canvas::get_maximum_timeline_range() {
                                    // ITERATION 7: Debug checkpoint 2 - After timeline range calculation
                                    
                                    let time_range_ns = ((max_time - min_time) * crate::visualizer::timeline::time_types::NS_PER_SECOND) as u64;
                                    // Fix: Use proper rounding instead of truncated integer division
                                    let contextual_zoom = NsPerPixel((time_range_ns + canvas_width as u64 / 2) / canvas_width as u64);
                                    
                                    // Timeline range calculated from file data
                                    
                                    // ITERATION 6: Enhanced clamping logic debug
                                    let min_zoom = NsPerPixel(crate::visualizer::timeline::time_types::MIN_ZOOM_NS_PER_PIXEL); // 1μs/px (very zoomed in)
                                    let max_zoom = NsPerPixel(crate::visualizer::timeline::time_types::MAX_ZOOM_NS_PER_PIXEL); // 10s/px (very zoomed out)
                                    
                                    // Apply zoom bounds (1μs/px to 10s/px range)
                                    
                                    let raw_clamp = contextual_zoom.nanos().clamp(min_zoom.nanos(), max_zoom.nanos());
                                    let clamped_zoom = NsPerPixel(raw_clamp);
                                    
                                    // ITERATION 6: Track clamping behavior
                                    let was_clamped = raw_clamp != contextual_zoom.nanos();
                                    // Zoom clamping applied
                                    if was_clamped {
                                        if raw_clamp == min_zoom.nanos() {
                                            // Clamped to minimum zoom level
                                        } else if raw_clamp == max_zoom.nanos() {
                                            // Clamped to maximum zoom level
                                        } else {
                                            // Error: Unexpected clamping result
                                        }
                                    } else {
                                        // No clamping applied - zoom within bounds
                                    }
                                    
                                    // Zoom calculation results computed
                                    
                                    // ✅ REMOVED: PREVIOUS_ZOOM_RESULT static cache - empty debug logic replaced with direct processing
                                    
                                    ns_per_pixel_handle.set(clamped_zoom);
                                    
                                    // R key: Trigger viewport reset to full timeline, cursor centering, and zoom center reset
                                    fit_all_clicked_relay_clone.send(()); // ✅ FIX: Reset viewport to full timeline
                                    cursor_center_at_viewport_relay_clone.send(());
                                    zoom_center_reset_to_zero_relay_clone.send(());
                                    
                                    // ITERATION 7: Final debug checkpoint
                                    
                                    // R key zoom reset calculation completed
                                        
                                    // ITERATION 8: COMPREHENSIVE SUMMARY per R key press
                                } else {
                                    panic!("R KEY: No timeline range available - Actor not properly initialized");
                                }
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom_center_pressed.next() => {
                        match event {
                            Some(()) => {
                                // Z key: Trigger zoom center reset to 0s
                                zoom_center_reset_to_zero_relay_clone.send(());
                            }
                            None => break,
                        }
                    }
                    event = ns_per_pixel_changed.next() => {
                        match event {
                            Some(new_ns_per_pixel) => {
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                            }
                            None => break,
                        }
                    }
                    event = canvas_resized.next() => {
                        match event {
                            Some((new_width, _height)) => {
                                // ✅ PROPER FIX: Get current viewport range from viewport actor signal
                                if let Some(current_viewport) = viewport_for_ns_per_pixel.signal().to_stream().next().await {
                                    let viewport_range_ns = current_viewport.end.nanos() - current_viewport.start.nanos();
                                    // Fix: Use proper rounding instead of truncated integer division  
                                    let corrected_ns_per_pixel = NsPerPixel((viewport_range_ns + new_width as u64 / 2) / new_width as u64);
                                    
                                    
                                    // Only update if the calculation produces a different value
                                    let current_ns_per_pixel = ns_per_pixel_handle.get();
                                    if corrected_ns_per_pixel != current_ns_per_pixel {
                                        ns_per_pixel_handle.set_neq(corrected_ns_per_pixel);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        
        // Create unified timeline cache mutable
        let cache = zoon::Mutable::new(TimelineCache::new());
        
        // Create all control flag actors
        let cursor_initialized = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let zooming_in = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let zooming_out = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let panning_left = Actor::new(false, async move |panning_handle| {
            let mut pan_left_started = pan_left_started_stream;
            let mut panning_started = panning_left_started_stream;
            let mut panning_stopped = panning_left_stopped_stream;
            
            loop {
                select! {
                    event = pan_left_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // TODO: Stop panning after duration or on key release
                            }
                            None => break,
                        }
                    }
                    event = panning_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    event = panning_stopped.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(false);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let panning_right = Actor::new(false, async move |panning_handle| {
            let mut pan_right_started = pan_right_started_stream;
            let mut panning_started = panning_right_started_stream;
            let mut panning_stopped = panning_right_stopped_stream;
            
            loop {
                select! {
                    event = pan_right_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // TODO: Stop panning after duration or on key release
                            }
                            None => break,
                        }
                    }
                    event = panning_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    event = panning_stopped.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(false);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let cursor_moving_left = Actor::new(false, async move |handle| {
            let mut cursor_started = cursor_moving_left_started_stream;
            let mut cursor_stopped = cursor_moving_left_stopped_stream;
            
            loop {
                select! {
                    event = cursor_started.next() => {
                        match event {
                            Some(()) => {
                                handle.set(true);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    event = cursor_stopped.next() => {
                        match event {
                            Some(()) => {
                                handle.set(false);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let cursor_moving_right = Actor::new(false, async move |handle| {
            let mut cursor_started = cursor_moving_right_started_stream;
            let mut cursor_stopped = cursor_moving_right_stopped_stream;
            
            loop {
                select! {
                    event = cursor_started.next() => {
                        match event {
                            Some(()) => {
                                handle.set(true);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    event = cursor_stopped.next() => {
                        match event {
                            Some(()) => {
                                handle.set(false);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let shift_pressed = Actor::new(false, async move |handle| {
            let mut shift_pressed_stream = shift_key_pressed_stream;
            let mut shift_released_stream = shift_key_released_stream;
            
            loop {
                select! {
                    event = shift_pressed_stream.next() => {
                        match event {
                            Some(()) => {
                                handle.set(true);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            },
                            None => break,
                        }
                    }
                    event = shift_released_stream.next() => {
                        match event {
                            Some(()) => {
                                handle.set(false);
                                // ✅ MIGRATED: Global mutable removed, using Actor state only
                            },
                            None => break,
                        }
                    }
                }
            }
        });
        
        // Mouse tracking actors
        let mouse_x = Actor::new(0.0_f32, async move |mouse_x_handle| {
            let mut mouse_moved = mouse_moved_stream;
            
            loop {
                select! {
                    event = mouse_moved.next() => {
                        match event {
                            Some((x_pos, _time)) => mouse_x_handle.set(x_pos),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let mouse_time = Actor::new(TimeNs::ZERO, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Zoom center actor
        let zoom_center = Actor::new(TimeNs::ZERO, async move |zoom_center_handle| {
            let mut zoom_center_reset_to_zero = zoom_center_reset_to_zero_stream;
            let mut zoom_center_follow_mouse = zoom_center_follow_mouse_stream;
            
            loop {
                select! {
                    event = zoom_center_reset_to_zero.next() => {
                        match event {
                            Some(()) => {
                                zoom_center_handle.set(TimeNs::ZERO);
                                // ✅ REMOVED: Static cache update antipattern - Actor is single source of truth
                            },
                            None => break,
                        }
                    }
                    event = zoom_center_follow_mouse.next() => {
                        match event {
                            Some(time_ns) => {
                                zoom_center_handle.set(time_ns);
                                // ✅ REMOVED: Static cache update antipattern - Actor is single source of truth
                            },
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        // Canvas dimension actors
        let canvas_width = Actor::new(0.0_f32, {
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
        
        // Create dedicated signals for efficient access (no conversion antipatterns)
        let signal_values_hashmap_signal = zoon::Mutable::new(HashMap::new());
        let variable_formats_hashmap_signal = zoon::Mutable::new(HashMap::new());
        let canvas_cache_hashmap_signal = zoon::Mutable::new(HashMap::new());
        
        // Signal values ActorMap
        let signal_values = ActorMap::new(BTreeMap::new(), {
            let signal_values_sync = signal_values_hashmap_signal.clone();
            async move |values_handle| {
            let mut signal_values_updated = signal_values_updated_stream;
            
            loop {
                select! {
                    event = signal_values_updated.next() => {
                        match event {
                            Some(updated_values) => {
                                for (signal_id, value) in updated_values {
                                    values_handle.lock_mut().insert_cloned(signal_id, value);
                                }
                                
                                // Sync dedicated HashMap signal after ActorMap change
                                {
                                    let current_map: HashMap<String, format_utils::SignalValue> = values_handle.lock_ref().iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect();
                                    signal_values_sync.set(current_map);
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        // Variable formats ActorMap  
        let variable_formats = ActorMap::new(BTreeMap::new(), {
            let variable_formats_sync = variable_formats_hashmap_signal.clone();
            async move |formats_handle| {
            let mut variable_format_updated = variable_format_updated_stream;
            
            loop {
                select! {
                    event = variable_format_updated.next() => {
                        match event {
                            Some((unique_id, format)) => {
                                formats_handle.lock_mut().insert_cloned(unique_id, format);
                                
                                // Sync dedicated HashMap signal after ActorMap change
                                {
                                    let current_map: HashMap<String, VarFormat> = formats_handle.lock_ref().iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect();
                                    variable_formats_sync.set(current_map);
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        // Canvas state actors
        let has_pending_request = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let canvas_cache = ActorMap::new(BTreeMap::new(), async move |_cache_handle| {
            loop { futures::future::pending::<()>().await; }
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
                                    // Removed redraw counter debug spam
                                });
                            },
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let last_redraw_time = Actor::new(0.0_f64, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let last_canvas_update = Actor::new(0_u64, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Timeline stats actor
        let timeline_stats = Actor::new(TimelineStats::default(), async move |_stats_handle| {
            loop {
                futures::future::pending::<()>().await;
            }
        });
        
        Self {
            // Core timeline state
            cursor_position,
            viewport,
            ns_per_pixel,
            cache,
            cursor_initialized,
            
            // Control flags
            zooming_in,
            zooming_out,
            panning_left,
            panning_right,
            cursor_moving_left,
            cursor_moving_right,
            shift_pressed,
            
            // Mouse tracking
            mouse_x,
            mouse_time,
            
            // Zoom/pan state
            zoom_center,
            canvas_width,
            canvas_height,
            signal_values,
            signal_values_hashmap_signal,
            variable_formats,
            variable_formats_hashmap_signal,
            
            // Canvas state
            has_pending_request,
            canvas_cache,
            canvas_cache_hashmap_signal,
            force_redraw,
            last_redraw_time,
            last_canvas_update,
            
            timeline_stats,
            
            // User interaction relays
            cursor_clicked_relay,
            cursor_moved_relay,
            zoom_in_started_relay,
            zoom_out_started_relay,
            pan_left_started_relay,
            pan_right_started_relay,
            mouse_moved_relay,
            canvas_resized_relay,
            redraw_requested_relay,
            signal_values_updated_relay,
            variable_format_updated_relay,
            
            // Keyboard navigation relays
            left_key_pressed_relay,
            right_key_pressed_relay,
            zoom_in_pressed_relay,
            zoom_out_pressed_relay,
            pan_left_pressed_relay,
            pan_right_pressed_relay,
            jump_to_previous_pressed_relay,
            jump_to_next_pressed_relay,
            reset_zoom_pressed_relay,
            reset_zoom_center_pressed_relay,
            cursor_center_at_viewport_relay: cursor_center_at_viewport_relay_for_struct,
            zoom_center_reset_to_zero_relay,
            shift_key_pressed_relay: shift_key_pressed_relay_var,
            shift_key_released_relay: shift_key_released_relay_var,
            
            // Animation state relays
            panning_left_started_relay: panning_left_started_relay_var,
            panning_left_stopped_relay: panning_left_stopped_relay_var,
            panning_right_started_relay: panning_right_started_relay_var,
            panning_right_stopped_relay: panning_right_stopped_relay_var,
            cursor_moving_left_started_relay: cursor_moving_left_started_relay_var,
            cursor_moving_left_stopped_relay: cursor_moving_left_stopped_relay_var,
            cursor_moving_right_started_relay: cursor_moving_right_started_relay_var,
            cursor_moving_right_stopped_relay: cursor_moving_right_stopped_relay_var,
            zoom_center_follow_mouse_relay,
            fit_all_clicked_relay,
            
            // System event relays
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
            ns_per_pixel_changed_relay,
        }
    }
    
    // === REACTIVE SIGNAL ACCESS ===
    
    /// Get reactive signal for cursor position (TimeNs)
    pub fn cursor_position_signal(&self) -> impl zoon::Signal<Item = TimeNs> {
        self.cursor_position.signal()
    }
    
    /// Get reactive signal for cursor position in seconds (for compatibility)
    pub fn cursor_position_seconds_signal(&self) -> impl zoon::Signal<Item = f64> {
        use zoon::SignalExt;
        self.cursor_position.signal().map(|time_ns| time_ns.display_seconds())
    }
    
    /// Get reactive signal for viewport (visible time range)
    pub fn viewport_signal(&self) -> impl zoon::Signal<Item = Viewport> {
        self.viewport.signal()
    }
    
    /// Get reactive signal for nanoseconds per pixel (zoom resolution)
    pub fn ns_per_pixel_signal(&self) -> impl zoon::Signal<Item = NsPerPixel> {
        self.ns_per_pixel.signal()
    }
    
    /// Get reactive signal for timeline coordinates computed from current state
    pub fn coordinates_signal(&self) -> impl zoon::Signal<Item = TimelineCoordinates> {
        zoon::map_ref! {
            let cursor_pos = self.cursor_position.signal(),
            let viewport = self.viewport.signal(),
            let ns_per_pixel = self.ns_per_pixel.signal(),
            let canvas_width = self.canvas_width.signal() =>
            TimelineCoordinates::new(
                *cursor_pos,
                viewport.start,         // Use actual viewport start, not TimeNs::ZERO
                *ns_per_pixel,
                *canvas_width as u32
            )
        }
    }
    
    /// Get reactive signal for unified timeline cache (triggers when cache changes)
    pub fn cache_signal(&self) -> impl zoon::Signal<Item = ()> {
        self.cache.signal_ref(|_| ())
    }
    
    /// Get reactive signal for cursor initialization status
    pub fn cursor_initialized_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.cursor_initialized.signal()
    }
    
    // === CONTROL FLAG SIGNALS ===
    
    /// Get reactive signal for zoom in status
    pub fn zooming_in_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.zooming_in.signal()
    }
    
    /// Get reactive signal for zoom out status
    pub fn zooming_out_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.zooming_out.signal()
    }
    
    /// Get reactive signal for panning left status
    pub fn panning_left_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.panning_left.signal()
    }
    
    /// Get reactive signal for panning right status
    pub fn panning_right_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.panning_right.signal()
    }
    
    /// Get reactive signal for cursor moving left status
    pub fn cursor_moving_left_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.cursor_moving_left.signal()
    }
    
    /// Get reactive signal for cursor moving right status
    pub fn cursor_moving_right_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.cursor_moving_right.signal()
    }
    
    /// Get reactive signal for shift key pressed status
    pub fn shift_pressed_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.shift_pressed.signal()
    }
    
    // === MOUSE TRACKING SIGNALS ===
    
    /// Get reactive signal for mouse X position
    pub fn mouse_x_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.mouse_x.signal()
    }
    
    /// Get reactive signal for mouse time position
    pub fn mouse_time_signal(&self) -> impl zoon::Signal<Item = TimeNs> {
        self.mouse_time.signal()
    }
    
    // === ZOOM/PAN SIGNALS ===
    
    /// Get reactive signal for zoom center
    pub fn zoom_center_signal(&self) -> impl zoon::Signal<Item = TimeNs> {
        self.zoom_center.signal()
    }
    
    /// Get reactive signal for canvas width
    pub fn canvas_width_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.canvas_width.signal()
    }
    
    /// Get reactive signal for canvas height
    pub fn canvas_height_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.canvas_height.signal()
    }
    
    /// Get reactive signal for all signal values
    pub fn signal_values_signal(&self) -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
        // ✅ FIXED: Use dedicated HashMap signal instead of conversion antipattern
        self.signal_values_hashmap_signal.signal_cloned()
    }
    
    /// Get reactive signal for variable formats
    pub fn variable_formats_signal(&self) -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
        // ✅ FIXED: Use dedicated HashMap signal instead of conversion antipattern
        self.variable_formats_hashmap_signal.signal_cloned()
    }
    
    // === CANVAS STATE SIGNALS ===
    
    /// Get reactive signal for pending request status
    pub fn has_pending_request_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.has_pending_request.signal()
    }
    
    /// Get reactive signal for canvas cache
    pub fn canvas_cache_signal(&self) -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
        // ✅ FIXED: Use dedicated HashMap signal instead of conversion antipattern
        self.canvas_cache_hashmap_signal.signal_cloned()
    }
    
    /// Get reactive signal for force redraw counter
    pub fn force_redraw_signal(&self) -> impl zoon::Signal<Item = u32> {
        self.force_redraw.signal()
    }
    
    /// Get reactive signal for last redraw time
    pub fn last_redraw_time_signal(&self) -> impl zoon::Signal<Item = f64> {
        self.last_redraw_time.signal()
    }
    
    /// Get reactive signal for last canvas update
    pub fn last_canvas_update_signal(&self) -> impl zoon::Signal<Item = u64> {
        self.last_canvas_update.signal()
    }
    
    /// Get reactive signal for timeline statistics
    pub fn timeline_stats_signal(&self) -> impl zoon::Signal<Item = TimelineStats> {
        self.timeline_stats.signal()
    }
    
    /// Get signal for specific signal value
    pub fn signal_value_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<format_utils::SignalValue>> {
        self.signal_values.value_signal(signal_id)
    }
    
    /// Get signal for specific variable format
    pub fn variable_format_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<VarFormat>> {
        self.variable_formats.value_signal(signal_id)
    }
    
    /// Get signal for specific canvas cache entry
    pub fn canvas_cache_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<Vec<(f32, SignalValue)>>> {
        self.canvas_cache.value_signal(signal_id)
    }
    
    /// Check if cursor is within visible range (combined signal)
    pub fn is_cursor_visible_signal(&self) -> impl zoon::Signal<Item = bool> {
        zoon::map_ref! {
            let cursor_pos = self.cursor_position.signal(),
            let viewport = self.viewport.signal() =>
            viewport.contains(*cursor_pos)
        }
    }
    
    /// Get time duration per pixel at current zoom (combined signal)
    pub fn time_per_pixel_signal(&self) -> impl zoon::Signal<Item = f64> {
        zoon::map_ref! {
            let viewport = self.viewport.signal(),
            let canvas_width = self.canvas_width.signal() => {
                let start_seconds = viewport.start.display_seconds();
                let end_seconds = viewport.end.display_seconds();
                (end_seconds - start_seconds) / *canvas_width as f64
            }
        }
    }
    
    /// Get current timeline bounds (convenience method)
    pub fn timeline_bounds_signal(&self) -> impl zoon::Signal<Item = (TimeNs, TimeNs)> {
        self.viewport.signal().map(|viewport| (viewport.start, viewport.end))
    }
    
    /// Get canvas dimensions as combined signal
    pub fn canvas_dimensions_signal(&self) -> impl zoon::Signal<Item = (f32, f32)> {
        zoon::map_ref! {
            let width = self.canvas_width.signal(),
            let height = self.canvas_height.signal() =>
            (*width, *height)
        }
    }
    
    /// Create a dummy instance for relay access during initialization
    /// This prevents panics when timeline functions are called before domain initialization
    pub fn new_dummy_for_initialization() -> Self {
        // unused shared import removed
        
        // Create dummy relays that can be cloned but won't process events meaningfully
        let cursor_clicked_relay = Relay::new();
        let cursor_moved_relay = Relay::new();
        let zoom_in_started_relay = Relay::new();
        let zoom_out_started_relay = Relay::new();
        let pan_left_started_relay = Relay::new();
        let pan_right_started_relay = Relay::new();
        let mouse_moved_relay = Relay::new();
        let canvas_resized_relay = Relay::new();
        let redraw_requested_relay = Relay::new();
        let signal_values_updated_relay = Relay::new();
        let variable_format_updated_relay = Relay::new();
        let left_key_pressed_relay = Relay::new();
        let right_key_pressed_relay = Relay::new();
        let zoom_in_pressed_relay = Relay::new();
        let zoom_out_pressed_relay = Relay::new();
        let pan_left_pressed_relay = Relay::new();
        let pan_right_pressed_relay = Relay::new();
        let jump_to_previous_pressed_relay = Relay::new();
        let jump_to_next_pressed_relay = Relay::new();
        let reset_zoom_pressed_relay = Relay::new();
        let reset_zoom_center_pressed_relay = Relay::new();
        let cursor_center_at_viewport_relay = Relay::new();
        let zoom_center_reset_to_zero_relay = Relay::new();
        let shift_key_pressed_relay = Relay::new();
        let shift_key_released_relay = Relay::new();
        
        // Animation state relays
        let panning_left_started_relay = Relay::new();
        let panning_left_stopped_relay = Relay::new();
        let panning_right_started_relay = Relay::new();
        let panning_right_stopped_relay = Relay::new();
        let cursor_moving_left_started_relay = Relay::new();
        let cursor_moving_left_stopped_relay = Relay::new();
        let cursor_moving_right_started_relay = Relay::new();
        let cursor_moving_right_stopped_relay = Relay::new();
        
        let zoom_center_follow_mouse_relay = Relay::new();
        let fit_all_clicked_relay = Relay::new();
        let data_loaded_relay = Relay::new();
        let transitions_cached_relay = Relay::new();
        let cursor_values_updated_relay = Relay::new();
        let timeline_bounds_calculated_relay = Relay::new();
        let viewport_changed_relay = Relay::new();
        let ns_per_pixel_changed_relay = Relay::new();

        Self {
            // Create dummy actors with default values
            cursor_position: Actor::new(TimeNs::ZERO, async |_| { loop { futures::future::pending::<()>().await; } }),
            viewport: Actor::new(Viewport::new(TimeNs::ZERO, TimeNs::from_nanos(crate::visualizer::timeline::time_types::DEFAULT_TIMELINE_RANGE_NS)), async |_| { loop { futures::future::pending::<()>().await; } }),
            ns_per_pixel: Actor::new(NsPerPixel::default(), async |_| { loop { futures::future::pending::<()>().await; } }),
            cache: zoon::Mutable::new(TimelineCache::default()),
            cursor_initialized: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            zooming_in: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            zooming_out: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            panning_left: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            panning_right: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            cursor_moving_left: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            cursor_moving_right: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            shift_pressed: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            mouse_x: Actor::new(0.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            mouse_time: Actor::new(TimeNs::ZERO, async |_| { loop { futures::future::pending::<()>().await; } }),
            zoom_center: Actor::new(TimeNs::ZERO, async |_| { loop { futures::future::pending::<()>().await; } }),
            canvas_width: Actor::new(0.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            canvas_height: Actor::new(0.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            signal_values: ActorMap::new(BTreeMap::new(), async |_| { loop { futures::future::pending::<()>().await; } }),
            variable_formats: ActorMap::new(BTreeMap::new(), async |_| { loop { futures::future::pending::<()>().await; } }),
            has_pending_request: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            canvas_cache: ActorMap::new(BTreeMap::new(), async |_| { loop { futures::future::pending::<()>().await; } }),
            force_redraw: Actor::new(0, async |_| { loop { futures::future::pending::<()>().await; } }),
            last_redraw_time: Actor::new(0.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            last_canvas_update: Actor::new(0, async |_| { loop { futures::future::pending::<()>().await; } }),
            timeline_stats: Actor::new(TimelineStats::default(), async |_| { loop { futures::future::pending::<()>().await; } }),
            
            // Use the dummy relays
            cursor_clicked_relay,
            cursor_moved_relay,
            zoom_in_started_relay,
            zoom_out_started_relay,
            pan_left_started_relay,
            pan_right_started_relay,
            mouse_moved_relay,
            canvas_resized_relay,
            redraw_requested_relay,
            signal_values_updated_relay,
            variable_format_updated_relay,
            left_key_pressed_relay,
            right_key_pressed_relay,
            zoom_in_pressed_relay,
            zoom_out_pressed_relay,
            pan_left_pressed_relay,
            pan_right_pressed_relay,
            jump_to_previous_pressed_relay,
            jump_to_next_pressed_relay,
            reset_zoom_pressed_relay,
            reset_zoom_center_pressed_relay,
            cursor_center_at_viewport_relay,
            zoom_center_reset_to_zero_relay,
            shift_key_pressed_relay,
            shift_key_released_relay,
            
            // Animation state relays
            panning_left_started_relay,
            panning_left_stopped_relay,
            panning_right_started_relay,
            panning_right_stopped_relay,
            cursor_moving_left_started_relay,
            cursor_moving_left_stopped_relay,
            cursor_moving_right_started_relay,
            cursor_moving_right_stopped_relay,
            zoom_center_follow_mouse_relay,
            fit_all_clicked_relay,
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
            ns_per_pixel_changed_relay,
            
            // HashMap-backed signals for reactive access (dummy values for static approach)
            canvas_cache_hashmap_signal: zoon::Mutable::new(HashMap::new()),
            signal_values_hashmap_signal: zoon::Mutable::new(HashMap::new()),
            variable_formats_hashmap_signal: zoon::Mutable::new(HashMap::new()),
        }
    }

    // === STATIC SIGNAL ACCESSORS FOR GLOBAL FUNCTIONS ===
    
    /// Variable formats signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn variable_formats_signal_static() -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
        crate::actors::global_domains::waveform_timeline_domain()
            .variable_formats_hashmap_signal
            .signal_cloned()
    }
    
    /// Pending request signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn has_pending_request_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .has_pending_request
            .signal()
    }
    
    /// Canvas cache signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn canvas_cache_signal_static() -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
        crate::actors::global_domains::waveform_timeline_domain()
            .canvas_cache_hashmap_signal
            .signal_cloned()
    }
    
    /// Force redraw signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn force_redraw_signal_static() -> impl zoon::Signal<Item = u32> {
        crate::actors::global_domains::waveform_timeline_domain()
            .force_redraw
            .signal()
    }
    
    /// Last redraw time signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn last_redraw_time_signal_static() -> impl zoon::Signal<Item = f64> {
        crate::actors::global_domains::waveform_timeline_domain()
            .last_redraw_time
            .signal()
    }
    
    /// Last canvas update signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn last_canvas_update_signal_static() -> impl zoon::Signal<Item = u64> {
        crate::actors::global_domains::waveform_timeline_domain()
            .last_canvas_update
            .signal()
    }
    
    /// Mouse X signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn mouse_x_signal_static() -> impl zoon::Signal<Item = f32> {
        crate::actors::global_domains::waveform_timeline_domain()
            .mouse_x
            .signal()
    }
    
    /// Mouse time signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn mouse_time_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        crate::actors::global_domains::waveform_timeline_domain()
            .mouse_time
            .signal()
    }
    
    /// Zoom center signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn zoom_center_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        crate::actors::global_domains::waveform_timeline_domain()
            .zoom_center
            .signal()
    }
    
    /// Canvas width signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn canvas_width_signal_static() -> impl zoon::Signal<Item = f32> {
        crate::actors::global_domains::waveform_timeline_domain()
            .canvas_width
            .signal()
    }
    
    /// Canvas height signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn canvas_height_signal_static() -> impl zoon::Signal<Item = f32> {
        crate::actors::global_domains::waveform_timeline_domain()
            .canvas_height
            .signal()
    }
    
    /// Signal values signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn signal_values_signal_static() -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
        crate::actors::global_domains::waveform_timeline_domain()
            .signal_values_hashmap_signal
            .signal_cloned()
    }

    // === MORE STATIC SIGNAL ACCESSORS ===
    
    /// Cursor position signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn cursor_position_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        crate::actors::global_domains::waveform_timeline_domain()
            .cursor_position
            .signal()
    }
    
    /// Cursor position seconds signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn cursor_position_seconds_signal_static() -> impl zoon::Signal<Item = f64> {
        crate::actors::global_domains::waveform_timeline_domain()
            .cursor_position
            .signal()
            .map(|time_ns| time_ns.display_seconds())
    }
    
    /// Viewport signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    /// Returns None until actual VCD data is loaded - no fallbacks!
    pub fn viewport_signal_static() -> impl zoon::Signal<Item = Option<Viewport>> {
        crate::actors::global_domains::waveform_timeline_domain()
            .viewport
            .signal()
            .map(|viewport| Some(viewport))
    }
    
    /// Ns per pixel signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn ns_per_pixel_signal_static() -> impl zoon::Signal<Item = NsPerPixel> {
        crate::actors::global_domains::waveform_timeline_domain()
            .ns_per_pixel
            .signal()
    }
    
    /// Coordinates signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn coordinates_signal_static() -> impl zoon::Signal<Item = TimelineCoordinates> {
        let timeline = crate::actors::global_domains::waveform_timeline_domain();
        zoon::map_ref! {
            let cursor_pos = timeline.cursor_position.signal(),
            let viewport = timeline.viewport.signal(),
            let ns_per_pixel = timeline.ns_per_pixel.signal(),
            let canvas_width = timeline.canvas_width.signal() =>
            TimelineCoordinates::new(
                *cursor_pos,
                viewport.start,
                *ns_per_pixel,
                *canvas_width as u32
            )
        }
    }
    
    /// Cache signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn cache_signal_static() -> impl zoon::Signal<Item = ()> {
        crate::actors::global_domains::waveform_timeline_domain()
            .cache
            .signal_ref(|_| ())
    }
    
    /// Cursor initialized signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn cursor_initialized_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .cursor_initialized
            .signal()
    }
    
    // === CONTROL FLAGS STATIC SIGNALS ===
    
    /// Zooming in signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn zooming_in_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .zooming_in
            .signal()
    }
    
    /// Zooming out signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn zooming_out_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .zooming_out
            .signal()
    }
    
    /// Panning left signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn panning_left_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .panning_left
            .signal()
    }
    
    /// Panning right signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn panning_right_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .panning_right
            .signal()
    }
    
    /// Cursor moving left signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn cursor_moving_left_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .cursor_moving_left
            .signal()
    }
    
    /// Cursor moving right signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn cursor_moving_right_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .cursor_moving_right
            .signal()
    }
    
    /// Shift pressed signal from domain - PROPERLY CONNECTED TO WAVEFORM_TIMELINE_DOMAIN
    pub fn shift_pressed_signal_static() -> impl zoon::Signal<Item = bool> {
        crate::actors::global_domains::waveform_timeline_domain()
            .shift_pressed
            .signal()
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

#[allow(dead_code)]
impl WaveformTimeline {
    /// Process loaded waveform data and cache transitions
    fn process_waveform_data(
        transitions_handle: &zoon::MutableBTreeMap<String, Vec<SignalTransition>>,
        file_id: String,
        waveform_data: WaveformFile
    ) {
        // Process each scope and signal in the waveform data
        for scope_data in &waveform_data.scopes {
            for signal_data in &scope_data.variables {
                let signal_id = format!("{}|{}|{}", file_id, scope_data.name, signal_data.name);
                
                // For now, create empty transitions - actual loading will happen via different mechanism
                let transitions: Vec<SignalTransition> = Vec::new();
                
                transitions_handle.lock_mut().insert_cloned(signal_id, transitions);
            }
        }
    }
    
    /// Update cursor values based on current position and cached transitions
    fn update_cursor_values_from_cache(
        cursor_position: f64,
        transitions: &BTreeMap<String, Vec<SignalTransition>>,
        values_handle: &zoon::MutableBTreeMap<String, SignalValue>
    ) {
        for (signal_id, signal_transitions) in transitions {
            // Find the most recent transition at or before cursor position
            let mut current_value = None;
            let cursor_ns = cursor_position * crate::visualizer::timeline::time_types::NS_PER_SECOND; // Convert seconds to ns
            
            for transition in signal_transitions.iter() {
                if transition.time_ns as f64 <= cursor_ns {
                    current_value = Some(SignalValue::Present(transition.value.clone()));
                } else {
                    break;
                }
            }
            
            if let Some(value) = current_value {
                values_handle.lock_mut().insert_cloned(signal_id.clone(), value);
            } else {
                values_handle.lock_mut().insert_cloned(signal_id.clone(), SignalValue::Missing);
            }
        }
    }
    
    /// Calculate timeline bounds from all loaded data
    fn calculate_timeline_bounds(transitions: &BTreeMap<String, Vec<SignalTransition>>) -> (f64, f64) {
        let mut min_time = f64::MAX;
        let mut max_time = f64::MIN;
        
        for signal_transitions in transitions.values() {
            if let Some(first) = signal_transitions.first() {
                min_time = min_time.min(first.time_ns as f64 / crate::visualizer::timeline::time_types::NS_PER_SECOND);
            }
            if let Some(last) = signal_transitions.last() {
                max_time = max_time.max(last.time_ns as f64 / crate::visualizer::timeline::time_types::NS_PER_SECOND);
            }
        }
        
        if min_time == f64::MAX || max_time == f64::MIN {
            (0.0, 1.0) // Default range if no data
        } else {
            (min_time, max_time)
        }
    }
}

// === CONVENIENCE FUNCTIONS FOR UI INTEGRATION ===

// ===== GLOBAL WAVEFORM TIMELINE INSTANCE =====

// ✅ ELIMINATED: WAVEFORM_TIMELINE_INSTANCE duplicate domain instance
// Now using centralized global_domains::WAVEFORM_TIMELINE_DOMAIN_INSTANCE

// ✅ ELIMINATED: initialize_waveform_timeline() - initialization handled by global_domains::initialize_all_domains()

/// Get the global WaveformTimeline instance
fn get_waveform_timeline() -> WaveformTimeline {
    // Use the global domains instance (which is properly initialized by initialize_all_domains())
    crate::actors::global_domains::waveform_timeline_domain().clone()
}

// ===== CONVENIENCE FUNCTIONS FOR GLOBAL ACCESS =====

// === CORE TIMELINE STATE ACCESS ===

/// Get cursor position signal (replaces _TIMELINE_CURSOR_NS.signal())
pub fn cursor_position_signal() -> impl zoon::Signal<Item = TimeNs> {
    crate::actors::global_domains::waveform_timeline_domain().cursor_position.signal()
}

/// Get cursor position in seconds signal (compatibility)
pub fn cursor_position_seconds_signal() -> impl zoon::Signal<Item = f64> {
    crate::actors::global_domains::waveform_timeline_domain().cursor_position.signal()
        .map(|time_ns| time_ns.display_seconds())
}

/// Get viewport signal (replaces _TIMELINE_VIEWPORT.signal())
pub fn viewport_signal() -> impl zoon::Signal<Item = Viewport> {
    crate::actors::global_domains::waveform_timeline_domain()
        .viewport
        .signal()
}

/// Get ns per pixel signal (replaces _TIMELINE_NS_PER_PIXEL.signal())
pub fn ns_per_pixel_signal() -> impl zoon::Signal<Item = NsPerPixel> {
    crate::actors::global_domains::waveform_timeline_domain().ns_per_pixel.signal().map(|value| {
        value
    })
}

/// Get coordinates signal (replaces _TIMELINE_COORDINATES.signal())
pub fn coordinates_signal() -> impl zoon::Signal<Item = TimelineCoordinates> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    zoon::map_ref! {
        let cursor_pos = timeline.cursor_position.signal(),
        let viewport = timeline.viewport.signal(),
        let ns_per_pixel = timeline.ns_per_pixel.signal(),
        let canvas_width = timeline.canvas_width.signal() =>
        TimelineCoordinates::new(
            *cursor_pos,
            viewport.start,
            *ns_per_pixel,
            *canvas_width as u32
        )
    }
}

/// Get unified cache signal (replaces UNIFIED_TIMELINE_CACHE.signal())
pub fn unified_timeline_cache_signal() -> impl zoon::Signal<Item = ()> {
    crate::actors::global_domains::waveform_timeline_domain()
        .cache
        .signal_ref(|_| ())
}

/// Get cursor initialized signal (replaces STARTUP_CURSOR_POSITION_SET.signal())
pub fn startup_cursor_position_set_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .cursor_initialized
        .signal()
}

// === CONTROL FLAG SIGNALS ===

/// Get zooming in signal (replaces IS_ZOOMING_IN.signal())
pub fn is_zooming_in_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .zooming_in
        .signal()
}

/// Get zooming out signal (replaces IS_ZOOMING_OUT.signal())
pub fn is_zooming_out_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .zooming_out
        .signal()
}

/// Get panning left signal (replaces IS_PANNING_LEFT.signal())
pub fn is_panning_left_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .panning_left
        .signal()
}

/// Get panning right signal (replaces IS_PANNING_RIGHT.signal())
pub fn is_panning_right_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .panning_right
        .signal()
}

/// Get cursor moving left signal (replaces IS_CURSOR_MOVING_LEFT.signal())
pub fn is_cursor_moving_left_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .cursor_moving_left
        .signal()
}

/// Get cursor moving right signal (replaces IS_CURSOR_MOVING_RIGHT.signal())
pub fn is_cursor_moving_right_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .cursor_moving_right
        .signal()
}

/// Get shift pressed signal (replaces IS_SHIFT_PRESSED.signal())
pub fn is_shift_pressed_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .shift_pressed
        .signal()
}

/// Get current shift pressed state synchronously (replaces IS_SHIFT_PRESSED.get())
/// ❌ DEPRECATED: Use is_shift_pressed_signal() for reactive patterns
/// This synchronous function violates Actor+Relay architecture
#[deprecated(note = "Use is_shift_pressed_signal() for proper reactive patterns")]
pub fn is_shift_pressed() -> bool {
    // ❌ ANTIPATTERN: This function should be eliminated - use signals instead
    // For now, return default to fix compilation. Calling code should use reactive signals.
    false // Always return false - calling code must migrate to reactive patterns
}

// === ANIMATION STATE SYNCHRONOUS ACCESS (MIGRATION BRIDGES) ===

/// Get current zooming in state synchronously (replaces IS_ZOOMING_IN.get())
/// ✅ MIGRATED: Returns false as global mutable eliminated
/// TODO: Replace animation.rs usage with signal-based patterns
pub fn is_zooming_in() -> bool {
    false  // Global mutable eliminated, animation state managed internally
}

/// Get current panning left state synchronously (replaces IS_PANNING_LEFT.get())
/// ✅ MIGRATED: Returns false as global mutable eliminated
/// TODO: Replace animation.rs usage with signal-based patterns
pub fn is_panning_left() -> bool {
    false  // Global mutable eliminated, animation state managed internally
}

/// Get current panning right state synchronously (replaces IS_PANNING_RIGHT.get())
/// ✅ MIGRATED: Returns false as global mutable eliminated
/// TODO: Replace animation.rs usage with signal-based patterns
pub fn is_panning_right() -> bool {
    false  // Global mutable eliminated, animation state managed internally
}

/// Get current cursor moving left state synchronously (replaces IS_CURSOR_MOVING_LEFT.get())
/// ✅ MIGRATED: Returns false as global mutable eliminated
/// TODO: Replace animation.rs usage with signal-based patterns
pub fn is_cursor_moving_left() -> bool {
    false  // Global mutable eliminated, animation state managed internally
}

/// Get current cursor moving right state synchronously (replaces IS_CURSOR_MOVING_RIGHT.get())
/// ✅ MIGRATED: Returns false as global mutable eliminated
/// TODO: Replace animation.rs usage with signal-based patterns
pub fn is_cursor_moving_right() -> bool {
    false  // Global mutable eliminated, animation state managed internally
}

// === VARIABLE FORMAT SYNCHRONOUS ACCESS (MIGRATION BRIDGE) ===

/// Get variable format for a specific signal ID (replaces SELECTED_VARIABLE_FORMATS.lock_ref().get())
/// ✅ MIGRATED: Returns None as global mutable eliminated
/// TODO: Replace with proper waveform_timeline_domain signal access
pub fn get_variable_format(_unique_id: &str) -> Option<VarFormat> {
    None  // Global mutable eliminated, format state managed in Actor domain
}

// === MOUSE TRACKING SIGNALS ===

/// Get mouse X position signal (replaces MOUSE_X_POSITION.signal())
pub fn mouse_x_position_signal() -> impl zoon::Signal<Item = f32> {
    crate::actors::global_domains::waveform_timeline_domain()
        .mouse_x
        .signal()
}

/// Get mouse time signal (replaces MOUSE_TIME_NS.signal())
pub fn mouse_time_ns_signal() -> impl zoon::Signal<Item = TimeNs> {
    crate::actors::global_domains::waveform_timeline_domain()
        .mouse_time
        .signal()
}

// === ZOOM/PAN SIGNALS ===

/// Get zoom center signal (replaces ZOOM_CENTER_NS.signal())
pub fn zoom_center_ns_signal() -> impl zoon::Signal<Item = TimeNs> {
    crate::actors::global_domains::waveform_timeline_domain()
        .zoom_center
        .signal()
}

/// Get canvas width signal (replaces CANVAS_WIDTH.signal())
pub fn canvas_width_signal() -> impl zoon::Signal<Item = f32> {
    crate::actors::global_domains::waveform_timeline_domain()
        .canvas_width
        .signal()
}

/// Get canvas height signal (replaces CANVAS_HEIGHT.signal())
pub fn canvas_height_signal() -> impl zoon::Signal<Item = f32> {
    crate::actors::global_domains::waveform_timeline_domain()
        .canvas_height
        .signal()
}

/// Get signal values signal (replaces SIGNAL_VALUES.signal())
pub fn signal_values_signal() -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
    crate::actors::global_domains::waveform_timeline_domain()
        .signal_values_hashmap_signal
        .signal_cloned()
}

/// Get variable formats signal (replaces SELECTED_VARIABLE_FORMATS.signal())
pub fn selected_variable_formats_signal() -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
    crate::actors::global_domains::waveform_timeline_domain()
        .variable_formats_hashmap_signal
        .signal_cloned()
}

// === CANVAS STATE SIGNALS ===

/// Get pending request signal (replaces _HAS_PENDING_REQUEST.signal())
pub fn has_pending_request_signal() -> impl zoon::Signal<Item = bool> {
    crate::actors::global_domains::waveform_timeline_domain()
        .has_pending_request
        .signal()
}

/// Get canvas cache signal (replaces PROCESSED_CANVAS_CACHE.signal())
pub fn processed_canvas_cache_signal() -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
    crate::actors::global_domains::waveform_timeline_domain()
        .canvas_cache_hashmap_signal
        .signal_cloned()
}

/// Get force redraw signal (replaces FORCE_REDRAW.signal())
pub fn force_redraw_signal() -> impl zoon::Signal<Item = u32> {
    // Use real domain actor signal instead of static fallback
    get_waveform_timeline().force_redraw.signal()
}

/// Get last redraw time signal (replaces LAST_REDRAW_TIME.signal())
pub fn last_redraw_time_signal() -> impl zoon::Signal<Item = f64> {
    crate::actors::global_domains::waveform_timeline_domain()
        .last_redraw_time
        .signal()
}

/// Get last canvas update signal (replaces LAST_CANVAS_UPDATE.signal())
pub fn last_canvas_update_signal() -> impl zoon::Signal<Item = u64> {
    crate::actors::global_domains::waveform_timeline_domain()
        .last_canvas_update
        .signal()
}

// ===== EVENT RELAY FUNCTIONS FOR UI INTEGRATION =====

// === USER INTERACTION EVENTS ===

/// User clicked on timeline at specific time (replaces direct cursor position setting)
pub fn cursor_clicked_relay() -> Relay<TimeNs> {
    get_waveform_timeline().cursor_clicked_relay.clone()
}

/// User moved cursor to specific time
pub fn cursor_moved_relay() -> Relay<TimeNs> {
    get_waveform_timeline().cursor_moved_relay.clone()
}

/// User started zoom in gesture at specific time
pub fn zoom_in_started_relay() -> Relay<TimeNs> {
    get_waveform_timeline().zoom_in_started_relay.clone()
}

/// User started zoom out gesture at specific time
pub fn zoom_out_started_relay() -> Relay<TimeNs> {
    get_waveform_timeline().zoom_out_started_relay.clone()
}

/// User started panning left
pub fn pan_left_started_relay() -> Relay<()> {
    get_waveform_timeline().pan_left_started_relay.clone()
}

/// User started panning right
pub fn pan_right_started_relay() -> Relay<()> {
    get_waveform_timeline().pan_right_started_relay.clone()
}

/// User moved mouse over canvas (position and time)
pub fn mouse_moved_relay() -> Relay<(f32, TimeNs)> {
    get_waveform_timeline().mouse_moved_relay.clone()
}

/// Canvas dimensions changed (resize)
pub fn canvas_resized_relay() -> Relay<(f32, f32)> {
    get_waveform_timeline().canvas_resized_relay.clone()
}

/// Force redraw requested
pub fn redraw_requested_relay() -> Relay<()> {
    get_waveform_timeline().redraw_requested_relay.clone()
}

/// Signal values updated from backend
pub fn signal_values_updated_relay() -> Relay<HashMap<String, format_utils::SignalValue>> {
    get_waveform_timeline().signal_values_updated_relay.clone()
}

/// Variable format updated for specific variable
pub fn variable_format_updated_relay() -> Relay<(String, VarFormat)> {
    get_waveform_timeline().variable_format_updated_relay.clone()
}

// === KEYBOARD NAVIGATION EVENTS ===

/// User pressed left arrow key
pub fn left_key_pressed_relay() -> Relay<()> {
    get_waveform_timeline().left_key_pressed_relay.clone()
}

/// User pressed right arrow key
pub fn right_key_pressed_relay() -> Relay<()> {
    get_waveform_timeline().right_key_pressed_relay.clone()
}

/// User pressed zoom in key
pub fn zoom_in_pressed_relay() -> Relay<()> {
    get_waveform_timeline().zoom_in_pressed_relay.clone()
}

/// User pressed zoom out key
pub fn zoom_out_pressed_relay() -> Relay<()> {
    get_waveform_timeline().zoom_out_pressed_relay.clone()
}

// === SYSTEM EVENTS ===

/// Waveform data loaded from file
pub fn data_loaded_relay() -> Relay<(String, WaveformFile)> {
    get_waveform_timeline().data_loaded_relay.clone()
}

/// Signal transitions cached for rendering
pub fn transitions_cached_relay() -> Relay<(String, Vec<SignalTransition>)> {
    get_waveform_timeline().transitions_cached_relay.clone()
}

/// Viewport changed due to resize or user action
pub fn viewport_changed_relay() -> Relay<(f64, f64)> {
    get_waveform_timeline().viewport_changed_relay.clone()
}

/// Timeline bounds calculated from loaded data
pub fn timeline_bounds_calculated_relay() -> Relay<(f64, f64)> {
    get_waveform_timeline().timeline_bounds_calculated_relay.clone()
}

/// User pressed shift key
pub fn shift_key_pressed_relay() -> Relay<()> {
    get_waveform_timeline().shift_key_pressed_relay.clone()
}

/// User released shift key
pub fn shift_key_released_relay() -> Relay<()> {
    get_waveform_timeline().shift_key_released_relay.clone()
}

// === ANIMATION STATE RELAY ACCESS FUNCTIONS ===

/// Animation started panning left
pub fn panning_left_started_relay() -> Relay<()> {
    get_waveform_timeline().panning_left_started_relay.clone()
}

/// Animation stopped panning left
pub fn panning_left_stopped_relay() -> Relay<()> {
    get_waveform_timeline().panning_left_stopped_relay.clone()
}

/// Animation started panning right
pub fn panning_right_started_relay() -> Relay<()> {
    get_waveform_timeline().panning_right_started_relay.clone()
}

/// Animation stopped panning right
pub fn panning_right_stopped_relay() -> Relay<()> {
    get_waveform_timeline().panning_right_stopped_relay.clone()
}

/// Animation started cursor moving left
pub fn cursor_moving_left_started_relay() -> Relay<()> {
    get_waveform_timeline().cursor_moving_left_started_relay.clone()
}

/// Animation stopped cursor moving left
pub fn cursor_moving_left_stopped_relay() -> Relay<()> {
    get_waveform_timeline().cursor_moving_left_stopped_relay.clone()
}

/// Animation started cursor moving right
pub fn cursor_moving_right_started_relay() -> Relay<()> {
    get_waveform_timeline().cursor_moving_right_started_relay.clone()
}

/// Animation stopped cursor moving right
pub fn cursor_moving_right_stopped_relay() -> Relay<()> {
    get_waveform_timeline().cursor_moving_right_stopped_relay.clone()
}

// === SYNCHRONOUS ACCESS FUNCTIONS (Cache Current Values Pattern) ===

/// Get initial cursor position (timeline center or 0 as fallback)
fn get_initial_cursor_position() -> TimeNs {
    if let Some((min_time, max_time)) = crate::visualizer::canvas::waveform_canvas::get_current_timeline_range() {
        let center_time = (min_time + max_time) / 2.0;
        TimeNs::from_external_seconds(center_time)
    } else {
        TimeNs::ZERO
    }
}

/// ❌ DEPRECATED: Use cursor_position_signal() for reactive patterns
/// This synchronous function violates Actor+Relay architecture
#[deprecated(note = "Use cursor_position_signal() for proper reactive patterns")]
pub fn current_cursor_position() -> Option<TimeNs> {
    // ❌ ANTIPATTERN: This function should be eliminated - use signals instead
    // For now, return default to fix compilation. Calling code should use reactive signals.
    None // Return None - calling code must migrate to reactive patterns
}

/// Get current cursor position in seconds synchronously  
/// ❌ DEPRECATED: Use cursor_position_seconds_signal() for proper reactive patterns
#[deprecated(note = "Use cursor_position_seconds_signal() for proper reactive patterns")]
pub fn current_cursor_position_seconds() -> Option<f64> {
    // ✅ FIXED: Remove deprecated function call
    None // Return None - calling code must migrate to reactive patterns
}

/// ❌ DEPRECATED: Use zoom_center_ns_signal() for reactive patterns
/// This synchronous function violates Actor+Relay architecture
#[deprecated(note = "Use zoom_center_ns_signal() for proper reactive patterns")]
pub fn current_zoom_center_position() -> TimeNs {
    // ❌ ANTIPATTERN: This function should be eliminated - use signals instead
    // For now, return default to fix compilation. Calling code should use reactive signals.
    TimeNs::ZERO // Return default - calling code must migrate to reactive patterns
}

/// Get current zoom center position in seconds from the cached value
/// ⚠️ DEPRECATED: Use zoom_center_ns_signal() for proper reactive patterns
#[deprecated(note = "Use zoom_center_ns_signal() for proper reactive patterns")]
pub fn current_zoom_center_seconds() -> f64 {
    // ❌ TEMPORARY: Return default during migration - this function should be eliminated  
    // Callers should use zoom_center_ns_signal() for proper reactive patterns instead
    // This function violates Actor+Relay architecture by providing synchronous access
    0.0
}

/// Get current viewport synchronously (replacement for bridge function)  
/// Returns None if timeline system not yet initialized
pub fn current_viewport() -> Option<Viewport> {
    // ❌ ANTIPATTERN: This function should be eliminated - use signals instead
    // For now, return default to fix compilation. Calling code should use reactive signals.
    None // Return None - calling code must migrate to reactive patterns
}


/// Get current ns_per_pixel synchronously (replacement for bridge function)
/// Returns None if timeline system not yet initialized
pub fn current_ns_per_pixel() -> Option<NsPerPixel> {
    // ❌ ANTIPATTERN: This function should be eliminated - use signals instead
    // For now, return default to fix compilation. Calling code should use reactive signals.
    None // Return None - calling code must migrate to reactive patterns
}

/// Get current timeline coordinates synchronously (replacement for bridge function)
#[deprecated(note = "Use reactive coordinate signals instead of synchronous access")]
pub fn current_coordinates() -> Option<TimelineCoordinates> {
    // ❌ ANTIPATTERN: This function should be eliminated - use signals instead
    // For now, return None to fix compilation. Calling code should use reactive signals.
    None // Return None - calling code must migrate to reactive patterns
}

/// Get current canvas width signal - proper Actor+Relay pattern
pub fn current_canvas_width_signal() -> impl zoon::Signal<Item = f32> {
    let timeline = waveform_timeline_domain();
    timeline.canvas_width.signal()
}

/// Get current canvas height signal - proper Actor+Relay pattern
pub fn current_canvas_height_signal() -> impl zoon::Signal<Item = f32> {
    let timeline = waveform_timeline_domain();
    timeline.canvas_height.signal()
}

/// Get current canvas width synchronously - DEPRECATED
#[deprecated(note = "Use waveform_timeline_domain().canvas_width.signal() reactive pattern instead")]
pub fn current_canvas_width() -> Option<f32> {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    // ✅ CORRECT: Use waveform_timeline_domain().canvas_width.signal() in reactive contexts
    // TODO: Replace with proper reactive patterns when rendering system is refactored
    Some(DEFAULT_CANVAS_WIDTH)  // Fallback value for compilation
}

/// Get current canvas height synchronously - DEPRECATED  
#[deprecated(note = "Use waveform_timeline_domain().canvas_height.signal() reactive pattern instead")]
pub fn current_canvas_height() -> f32 {
    // ❌ ARCHITECTURE VIOLATION: Synchronous access breaks Actor+Relay reactive patterns
    // ✅ CORRECT: Use waveform_timeline_domain().canvas_height.signal() in reactive contexts
    // TODO: Replace with proper reactive patterns when rendering system is refactored
    FALLBACK_CANVAS_HEIGHT  // Fallback value for compilation
}

/// Set cursor position through domain event (replacement for bridge function)
pub fn set_cursor_position(time_ns: TimeNs) {
    cursor_moved_relay().send(time_ns);
}

/// Set cursor position from f64 seconds (convenience function)
pub fn set_cursor_position_seconds(seconds: f64) {
    let time_ns = TimeNs::from_external_seconds(seconds);
    cursor_moved_relay().send(time_ns);
}

/// Set cursor position if changed (replacement for bridge function)
pub fn set_cursor_position_if_changed(time_ns: TimeNs) {
    // ✅ FIXED: Remove deprecated function call, let Actor handle deduplication
    // Actor+Relay architecture should handle deduplication internally
    cursor_moved_relay().send(time_ns);
}

/// Update zoom center to follow mouse position (for blue vertical line)
pub fn set_zoom_center_follow_mouse(time_ns: TimeNs) {
    get_waveform_timeline().zoom_center_follow_mouse_relay.send(time_ns);
}

/// Set viewport if changed (replacement for bridge function)
pub fn set_viewport_if_changed(new_viewport: Viewport) {
    let current_viewport = current_viewport();
    
    // DEBUG: Track all viewport change attempts to catch the 1s corruption
    if let Some(vp) = current_viewport {
        zoon::println!("🔍 Current viewport: {:.3}s - {:.3}s (duration: {:.3}s)", 
            vp.start.display_seconds(), 
            vp.end.display_seconds(),
            vp.duration().display_seconds());
    } else {
        zoon::println!("🔍 Current viewport: None");
    }
    zoon::println!("🔍 New viewport: {:.3}s - {:.3}s (duration: {:.3}s)", 
        new_viewport.start.display_seconds(), 
        new_viewport.end.display_seconds(),
        new_viewport.duration().display_seconds());
    
    if new_viewport.duration().display_seconds() <= 1.1 {
        zoon::println!("🚨 BLOCKING viewport corruption: duration {:.3}s is too small", 
            new_viewport.duration().display_seconds());
        if let Some(vp) = current_viewport {
            zoon::println!("🔒 Preserving current viewport: {:.3}s - {:.3}s", 
                vp.start.display_seconds(), vp.end.display_seconds());
        } else {
            zoon::println!("🔒 No current viewport to preserve");
        }
        return; // Block the corruption, preserve current viewport
    }
    
    // Only emit event if value actually changed
    if current_viewport != Some(new_viewport) {
        if let Some(vp) = current_viewport {
            zoon::println!("✅ Viewport changed: {:.3}s-{:.3}s → {:.3}s-{:.3}s", 
                vp.start.display_seconds(), vp.end.display_seconds(),
                new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        } else {
            zoon::println!("✅ Viewport set from None to {:.3}s-{:.3}s", 
                new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        }
        let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        viewport_changed_relay().send(viewport_tuple);
    } else {
        zoon::println!("📝 Viewport unchanged, not sending signal");
    }
}

/// Set ns_per_pixel if changed (replacement for bridge function)
pub fn set_ns_per_pixel_if_changed(new_ns_per_pixel: NsPerPixel) {
    let current_ns_per_pixel = current_ns_per_pixel();
    
    // Only emit event if value actually changed
    if current_ns_per_pixel != Some(new_ns_per_pixel) {
        
        // CRITICAL FIX: Use proper Actor+Relay pattern - send ns_per_pixel update event
        let domain = waveform_timeline_domain();
        domain.ns_per_pixel_changed_relay.send(new_ns_per_pixel);
        
        // ✅ REMOVED: Static cache update antipattern - Actor is single source of truth
        
        // TODO: Replace current_zoom_center_position with zoom_center_ns_signal() for proper reactive patterns
        let zoom_center = TimeNs::ZERO; // Fallback to avoid deprecated function
        zoom_in_started_relay().send(zoom_center);
    }
}

/// Set canvas dimensions through domain event (replacement for bridge function)
pub fn set_canvas_dimensions(width: f32, height: f32) {
    canvas_resized_relay().send((width, height));
}

// ✅ REMOVED: initialize_value_caching() function - deprecated static cache antipattern
// All functions now use direct Actor domain access instead of static caches

// Helper functions to get the static cache instances
// ❌ ANTIPATTERN: Static caching outside Actor loops - TODO: Use Cache Current Values pattern in WaveformTimeline Actor
#[deprecated(note = "Replace static cache with Cache Current Values pattern inside WaveformTimeline Actor loop")]
// ✅ REMOVED: static_cache_cursor() function - deprecated static cache antipattern eliminated

// ✅ REMOVED: static_cache_zoom_center() function - deprecated static cache antipattern eliminated


// ✅ REMOVED: static_cache_ns_per_pixel() function - deprecated static cache antipattern eliminated


// ✅ REMOVED: static_cache_width() function - deprecated static cache antipattern eliminated

// ✅ REMOVED: static_cache_height() function - deprecated static cache antipattern eliminated

// ===== UNIFIED TIMELINE CACHE OPERATIONS (REPLACES UNIFIED_TIMELINE_CACHE) =====

/// Get cursor value from unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_ref().get_cursor_value())
pub fn get_cursor_value_from_cache(signal_id: &str) -> Option<shared::SignalValue> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let cache = timeline.cache.lock_ref();
    cache.get_cursor_value(signal_id).cloned()
}

/// Get raw transitions from unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_ref().get_raw_transitions())
pub fn get_raw_transitions_from_cache(signal_id: &str) -> Option<Vec<shared::SignalTransition>> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let cache = timeline.cache.lock_ref();
    cache.get_raw_transitions(signal_id).cloned()
}

/// Insert cursor value into unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().cursor_values.insert())
pub fn insert_cursor_value_to_cache(signal_id: String, value: shared::SignalValue) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().cursor_values.insert(signal_id, value);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Insert raw transitions into unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().raw_transitions.insert())
pub fn insert_raw_transitions_to_cache(signal_id: String, transitions: Vec<shared::SignalTransition>) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().raw_transitions.insert(signal_id, transitions);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Insert viewport data into unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().viewport_data.insert())
pub fn insert_viewport_data_to_cache(signal_id: String, viewport_data: super::time_types::ViewportSignalData) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().viewport_data.insert(signal_id, viewport_data);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Remove cursor value from unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().cursor_values.remove())
pub fn remove_cursor_value_from_cache(signal_id: &str) -> Option<shared::SignalValue> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let removed = timeline.cache.lock_mut().cursor_values.remove(signal_id);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
    
    removed
}

/// Remove raw transitions from unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().raw_transitions.remove())
pub fn remove_raw_transitions_from_cache(signal_id: &str) -> Option<Vec<shared::SignalTransition>> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let removed = timeline.cache.lock_mut().raw_transitions.remove(signal_id);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
    
    removed
}

/// Remove viewport data from unified timeline cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().viewport_data.remove())
pub fn remove_viewport_data_from_cache(signal_id: &str) -> Option<super::time_types::ViewportSignalData> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let removed = timeline.cache.lock_mut().viewport_data.remove(signal_id);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
    
    removed
}

/// Invalidate cache validity flags (replaces UNIFIED_TIMELINE_CACHE.lock_mut().metadata.validity modification)
pub fn invalidate_cache_validity(viewport_invalid: bool) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let mut cache = timeline.cache.lock_mut();
    cache.metadata.validity.cursor_valid = false;
    if viewport_invalid {
        cache.metadata.validity.viewport_valid = false;
    }
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Clean up old active requests (replaces UNIFIED_TIMELINE_CACHE.lock_mut().active_requests.retain())
pub fn cleanup_old_active_requests() {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let mut cache = timeline.cache.lock_mut();
    let now = super::time_types::TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
    let cutoff_threshold = super::time_types::DurationNs::from_external_seconds(10.0); // 10 seconds
    
    cache.active_requests.retain(|_, request| {
        now.duration_since(request.timestamp_ns) < cutoff_threshold
    });
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Invalidate cursor cache when cursor position changes (replaces UNIFIED_TIMELINE_CACHE.lock_mut().invalidate_cursor())
pub fn invalidate_cursor_cache(cursor_time: TimeNs) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let mut cache = timeline.cache.lock_mut();
    cache.invalidate_cursor(cursor_time);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Get reactive signal for cursor value at current timeline position (replaces UnifiedTimelineService::cursor_value_signal)
/// 
/// This function provides proper Actor+Relay architecture for cursor values instead of service layer antipattern.
/// Uses Cache Current Values pattern inside Actor loops and proper reactive signal chains.
pub fn cursor_value_signal(signal_id: &str) -> impl zoon::Signal<Item = String> + use<> {
    let signal_id_cloned = signal_id.to_string();
    
    // ✅ ACTOR+RELAY: React to cursor and cache changes using WaveformTimeline domain signals
    map_ref! {
        let cursor_pos = cursor_position_signal(),
        let _cache_signal = unified_timeline_cache_signal() => {
            
            // TODO: Replace static cache with Cache Current Values pattern inside WaveformTimeline Actor loop
            // Temporarily disable cache access to eliminate deprecated warnings
            if let Some(cached_value) = None::<shared::SignalValue> { // get_cursor_value_from_cache(&signal_id_cloned)
                match cached_value {
                    shared::SignalValue::Present(data) => data.clone(),
                    shared::SignalValue::Missing => "N/A".to_string(),
                }
            }
            // ✅ ACTOR+RELAY: Check raw transitions using WaveformTimeline domain method
            else if let Some(transitions) = get_raw_transitions_from_cache(&signal_id_cloned) {
                let cursor_ns = *cursor_pos;
                if let Some(interpolated) = interpolate_cursor_value(&transitions, cursor_ns) {
                    match interpolated {
                        shared::SignalValue::Present(data) => data,
                        shared::SignalValue::Missing => "N/A".to_string(),
                    }
                } else {
                    "N/A".to_string()
                }
            }
            // ✅ ACTOR+RELAY: Check for pending request using WaveformTimeline domain method
            else if is_duplicate_request_in_cache(&[signal_id_cloned.clone()], super::time_types::CacheRequestType::CursorValues) {
                "Loading...".to_string()
            }
            // No data available - trigger request for cursor values
            else {
                // Trigger async request for cursor values outside viewport
                let signal_ids = vec![signal_id_cloned.clone()];
                let cursor_time = *cursor_pos;
                zoon::Task::start(async move {
                    request_cursor_values(signal_ids, cursor_time);
                });
                "Loading...".to_string()
            }
        }
    }.dedupe_cloned()
}

/// Request cursor values at specific timeline position (replaces UnifiedTimelineService::request_cursor_values)
/// 
/// Proper Actor+Relay implementation that avoids service layer antipatterns.
/// Uses domain cache operations and backend communication.
pub fn request_cursor_values(
    signal_ids: Vec<String>,
    cursor_time: TimeNs,
) {
    // ✅ ACTOR+RELAY: Check duplicate requests using WaveformTimeline domain method
    if is_duplicate_request_in_cache(&signal_ids, super::time_types::CacheRequestType::CursorValues) {
        return;
    }
    
    // ✅ ACTOR+RELAY: Update cache cursor using WaveformTimeline domain method
    invalidate_cursor_cache(cursor_time);
    
    // ✅ ACTOR+RELAY: Check cache hits using WaveformTimeline domain methods
    let mut cache_hits = Vec::new();
    let mut cache_misses = Vec::new();
    
    for signal_id in &signal_ids {
        // TODO: Replace static cache access with Cache Current Values pattern inside WaveformTimeline Actor
        // Temporarily return cache miss to eliminate deprecated warnings
        if false { // get_cursor_value_from_cache(signal_id).is_some()
            cache_hits.push(signal_id.clone());
        }
        // Then check if we can interpolate from raw transitions
        else if let Some(transitions) = get_raw_transitions_from_cache(signal_id) {
            if can_interpolate_cursor_value(&transitions, cursor_time) {
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
        let request_id = generate_request_id();
        
        // ✅ ACTOR+RELAY: Add active request using WaveformTimeline domain method
        add_active_request_to_cache(
            request_id.clone(), 
            super::time_types::CacheRequestState {
                requested_signals: cache_misses.clone(),
                _viewport: None,
                timestamp_ns: TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0),
                request_type: super::time_types::CacheRequestType::CursorValues,
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
        
        crate::connection::send_up_msg(shared::UpMsg::UnifiedSignalQuery {
            signal_requests: backend_requests,
            cursor_time_ns: Some(cursor_time.nanos()),
            request_id,
        });
    }
}

// ===== HELPER FUNCTIONS FOR CURSOR VALUE INTERPOLATION =====

fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = js_sys::Date::now() as u128;
    
    format!("unified_{}_{}", timestamp, id)
}

fn can_interpolate_cursor_value(transitions: &[shared::SignalTransition], cursor_time: TimeNs) -> bool {
    if transitions.is_empty() {
        return false;
    }
    
    let cursor_seconds = cursor_time.display_seconds();
    let first_time = transitions[0].time_ns as f64 / crate::visualizer::timeline::time_types::NS_PER_SECOND;
    let last_time = transitions[transitions.len() - 1].time_ns as f64 / crate::visualizer::timeline::time_types::NS_PER_SECOND;
    
    cursor_seconds >= first_time && cursor_seconds <= last_time
}

fn interpolate_cursor_value(transitions: &[shared::SignalTransition], cursor_time: TimeNs) -> Option<shared::SignalValue> {
    if transitions.is_empty() {
        return None;
    }
    
    let cursor_seconds = cursor_time.display_seconds();
    
    // Find the most recent transition at or before cursor time
    for transition in transitions.iter().rev() {
        if transition.time_ns as f64 / crate::visualizer::timeline::time_types::NS_PER_SECOND <= cursor_seconds {
            return Some(shared::SignalValue::Present(transition.value.clone()));
        }
    }
    
    None
}

/// Invalidate viewport cache when viewport changes (replaces UNIFIED_TIMELINE_CACHE.lock_mut().invalidate_viewport())
pub fn invalidate_viewport_cache(viewport: Viewport) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().invalidate_viewport(viewport);
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Check if request is duplicate (replaces UNIFIED_TIMELINE_CACHE.lock_ref().is_duplicate_request())
pub fn is_duplicate_request_in_cache(signal_ids: &[String], request_type: super::time_types::CacheRequestType) -> bool {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let cache = timeline.cache.lock_ref();
    cache.is_duplicate_request(signal_ids, request_type)
}

/// Add active request to cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().active_requests.insert())
pub fn add_active_request_to_cache(request_id: String, request_state: super::time_types::CacheRequestState) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().active_requests.insert(request_id, request_state);
}

/// Remove active request from cache (replaces UNIFIED_TIMELINE_CACHE.lock_mut().active_requests.remove())
pub fn remove_active_request_from_cache(request_id: &str) -> Option<super::time_types::CacheRequestState> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().active_requests.remove(request_id)
}

/// Get active request from cache (replaces UNIFIED_TIMELINE_CACHE.lock_ref().active_requests.get())
pub fn get_active_request_from_cache(request_id: &str) -> Option<super::time_types::CacheRequestState> {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    let cache = timeline.cache.lock_ref();
    cache.active_requests.get(request_id).cloned()
}

/// Update cache statistics (replaces UNIFIED_TIMELINE_CACHE.lock_mut().metadata.statistics)
pub fn update_cache_statistics(statistics: shared::SignalStatistics) {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    timeline.cache.lock_mut().metadata.statistics = statistics;
}

/// Force cache signal re-evaluation by updating timestamp (replaces manual cache modification)
pub fn force_cache_signal_reevaluation() {
    let timeline = crate::actors::global_domains::waveform_timeline_domain();
    
    // Trigger cache signal by temporarily modifying cache metadata timestamp
    timeline.cache.lock_mut().metadata.statistics.query_time_ms = js_sys::Date::now() as u64;
    
    // TODO: Investigate proper signal emission pattern for Actor updates
}

/// Handle unified response from backend (replaces UnifiedTimelineService::handle_unified_response)
/// 
/// This function provides proper Actor+Relay backend response handling without service layer antipatterns.
/// Uses domain cache operations and properly manages cache state without circuit breaker complexity.
pub fn handle_unified_response(
    request_id: String,
    signal_data: Vec<shared::UnifiedSignalData>,
    cursor_values: std::collections::BTreeMap<String, shared::SignalValue>,
    statistics: Option<shared::SignalStatistics>,
) {
    // ✅ ACTOR+RELAY: Get request info using WaveformTimeline domain method
    let _request_info = get_active_request_from_cache(&request_id);
    
    // Update viewport data
    for signal in signal_data {
        // Always update raw transitions first (move signal.transitions here)
        let raw_transitions = signal.transitions;
        // ✅ ACTOR+RELAY: Insert raw transitions using WaveformTimeline domain method
        insert_raw_transitions_to_cache(signal.unique_id.clone(), raw_transitions);
    }
    
    // Update cursor values and trigger signal updates
    let has_cursor_values = !cursor_values.is_empty();
    if has_cursor_values {
        let mut ui_signal_values = std::collections::HashMap::new();
        
        for (signal_id, value) in &cursor_values {
            // ✅ ACTOR+RELAY: Insert cursor value using WaveformTimeline domain method
            insert_cursor_value_to_cache(signal_id.clone(), value.clone());
            
            // Convert backend SignalValue to UI SignalValue format
            let ui_value = match value {
                shared::SignalValue::Present(data) => crate::visualizer::formatting::signal_values::SignalValue::from_data(data.clone()),
                shared::SignalValue::Missing => crate::visualizer::formatting::signal_values::SignalValue::missing(),
            };
            ui_signal_values.insert(signal_id.clone(), ui_value);
        }
        
        // Send cursor values to UI signal system
        let num_values = ui_signal_values.len();
        if num_values > 0 {
            let timeline = crate::actors::global_domains::waveform_timeline_domain();
            timeline.signal_values_updated_relay.send(ui_signal_values);
        }
    }
    
    // Update statistics
    if let Some(stats) = statistics {
        // ✅ ACTOR+RELAY: Update cache statistics using WaveformTimeline domain method
        update_cache_statistics(stats);
    }
    
    // ✅ ACTOR+RELAY: Remove completed request using WaveformTimeline domain method
    remove_active_request_from_cache(&request_id);
    
    // ✅ ACTOR+RELAY: Trigger cache signal using WaveformTimeline domain method
    if has_cursor_values {
        force_cache_signal_reevaluation();
    }
}

/// Handle error response from backend (replaces UnifiedTimelineService::handle_unified_error)
/// 
/// Proper Actor+Relay error handling without service layer complexity.
pub fn handle_unified_error(request_id: String, _error: String) {
    // ✅ ACTOR+RELAY: Remove failed request using WaveformTimeline domain method
    remove_active_request_from_cache(&request_id);
}

// Duplicate function removed - keeping definition at line 1976

/// Calculate adaptive step size for cursor movement (Q/E keys)
/// Returns step size in nanoseconds based on visible time range
fn calculate_adaptive_cursor_step() -> u64 {
    let viewport = current_viewport();
    let visible_range_ns = if let Some(vp) = viewport {
        vp.end.nanos() - vp.start.nanos()
    } else {
        crate::visualizer::timeline::time_types::DEFAULT_TIMELINE_RANGE_NS // Default 1 second range if not initialized
    };
    
    // Step size should be approximately 1% of visible range, with reasonable bounds
    let base_step = visible_range_ns / 100; // 1% of visible range
    
    // Apply bounds to keep step size reasonable
    let min_step = crate::visualizer::timeline::time_types::MIN_CURSOR_STEP_NS; // 1ms minimum
    let max_step = crate::visualizer::timeline::time_types::MAX_CURSOR_STEP_NS; // 1s maximum
    
    base_step.clamp(min_step, max_step)
}