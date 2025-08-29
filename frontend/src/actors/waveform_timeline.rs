//! WaveformTimeline domain for timeline management using Actor+Relay architecture
//!
//! Consolidated timeline management domain to replace global mutables with event-driven architecture.
//! Manages cursor position, viewport ranges, zoom levels, and cached waveform data.

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::actors::{Actor, ActorMap, Relay, relay};
use crate::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates, TimelineCache};
use shared::{SignalTransition, SignalValue, WaveformFile, VarFormat};
use zoon::{MutableExt, SignalVecExt, SignalExt};
use futures::{StreamExt, select};
use std::collections::{BTreeMap, HashMap};
use crate::format_utils;

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
    
    /// Unified timeline coordinates for integer-based calculations
    coordinates: Actor<TimelineCoordinates>,
    
    /// Unified timeline cache - replaces 4 separate cache systems
    cache: Actor<TimelineCache>,
    
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
    
    /// Format selections for selected variables
    variable_formats: ActorMap<String, VarFormat>,
    
    // === CANVAS STATE (5 mutables from waveform_canvas.rs) ===
    /// Track pending backend requests
    has_pending_request: Actor<bool>,
    
    /// Processed canvas cache for rendering optimization
    canvas_cache: ActorMap<String, Vec<(f32, SignalValue)>>,
    
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
        let (canvas_resized_relay, canvas_resized_stream) = relay::<(f32, f32)>();
        let (redraw_requested_relay, redraw_requested_stream) = relay::<()>();
        let (signal_values_updated_relay, signal_values_updated_stream) = relay::<HashMap<String, format_utils::SignalValue>>();
        
        // Create relays for keyboard navigation
        let (left_key_pressed_relay, left_key_pressed_stream) = relay::<()>();
        let (right_key_pressed_relay, right_key_pressed_stream) = relay::<()>();
        let (zoom_in_pressed_relay, zoom_in_pressed_stream) = relay::<()>();
        let (zoom_out_pressed_relay, zoom_out_pressed_stream) = relay::<()>();
        let (pan_left_pressed_relay, _pan_left_pressed_stream) = relay::<()>();
        let (pan_right_pressed_relay, _pan_right_pressed_stream) = relay::<()>();
        let (jump_to_previous_pressed_relay, _jump_to_previous_pressed_stream) = relay::<()>();
        let (jump_to_next_pressed_relay, _jump_to_next_pressed_stream) = relay::<()>();
        let (reset_zoom_pressed_relay, _reset_zoom_pressed_stream) = relay::<()>();
        let (reset_zoom_center_pressed_relay, _reset_zoom_center_pressed_stream) = relay::<()>();
        let (fit_all_clicked_relay, fit_all_clicked_stream) = relay::<()>();
        
        // Create relays for system events
        let (data_loaded_relay, _data_loaded_stream) = relay::<(String, WaveformFile)>();
        let (transitions_cached_relay, _transitions_cached_stream) = relay::<(String, Vec<SignalTransition>)>();
        let (cursor_values_updated_relay, _cursor_values_updated_stream) = relay::<BTreeMap<String, SignalValue>>();
        let (timeline_bounds_calculated_relay, _timeline_bounds_calculated_stream) = relay::<(f64, f64)>();
        let (viewport_changed_relay, _viewport_changed_stream) = relay::<(f64, f64)>();
        
        // Create cursor position actor with comprehensive event handling
        let cursor_position = Actor::new(TimeNs::ZERO, async move |cursor_handle| {
            let mut cursor_clicked = cursor_clicked_stream.fuse();
            let mut cursor_moved = cursor_moved_stream.fuse();
            let mut left_key_pressed = left_key_pressed_stream.fuse();
            let mut right_key_pressed = right_key_pressed_stream.fuse();
            
            loop {
                select! {
                    event = cursor_clicked.next() => {
                        match event {
                            Some(time_ns) => cursor_handle.set(time_ns),
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
                                let new_time = current.nanos().saturating_sub(1_000); // Move left by 1μs
                                *current = TimeNs::from_nanos(new_time);
                            }),
                            None => break,
                        }
                    }
                    event = right_key_pressed.next() => {
                        match event {
                            Some(()) => cursor_handle.update_mut(|current| {
                                let new_time = current.nanos().saturating_add(1_000); // Move right by 1μs
                                *current = TimeNs::from_nanos(new_time);
                            }),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        // Create viewport actor with comprehensive event handling
        let viewport = Actor::new(
            Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(100.0)), 
            async move |viewport_handle| {
                let mut viewport_changed = _viewport_changed_stream.fuse();
                let mut timeline_bounds_calculated = _timeline_bounds_calculated_stream.fuse();
                let mut fit_all_clicked = fit_all_clicked_stream.fuse();
                
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
                                }
                                None => break,
                            }
                        }
                        event = fit_all_clicked.next() => {
                            match event {
                                Some(()) => {
                                    // Will be updated by timeline bounds calculation
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
        let ns_per_pixel = Actor::new(NsPerPixel::MEDIUM_ZOOM, async move |ns_per_pixel_handle| {
            let mut zoom_in_started = zoom_in_started_stream.fuse();
            let mut zoom_out_started = zoom_out_started_stream.fuse();
            let mut zoom_in_pressed = zoom_in_pressed_stream.fuse();
            let mut zoom_out_pressed = zoom_out_pressed_stream.fuse();
            
            loop {
                select! {
                    event = zoom_in_started.next() => {
                        match event {
                            Some(_center_time) => ns_per_pixel_handle.update_mut(|current| *current = current.zoom_in_smooth(0.3)),
                            None => break,
                        }
                    }
                    event = zoom_out_started.next() => {
                        match event {
                            Some(_center_time) => ns_per_pixel_handle.update_mut(|current| *current = current.zoom_out_smooth(0.3)),
                            None => break,
                        }
                    }
                    event = zoom_in_pressed.next() => {
                        match event {
                            Some(()) => ns_per_pixel_handle.update_mut(|current| *current = current.zoom_in_smooth(0.3)),
                            None => break,
                        }
                    }
                    event = zoom_out_pressed.next() => {
                        match event {
                            Some(()) => ns_per_pixel_handle.update_mut(|current| *current = current.zoom_out_smooth(0.3)),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        // Create coordinates actor for unified timeline coordinate system
        let coordinates = Actor::new(
            TimelineCoordinates::new(
                TimeNs::ZERO,               // cursor_ns
                TimeNs::ZERO,               // viewport_start_ns  
                NsPerPixel::MEDIUM_ZOOM,    // ns_per_pixel
                800                         // canvas_width_pixels (initial)
            ),
            async move |_coords_handle| {
                // Coordinates are updated by other actors
                loop {
                    futures::future::pending::<()>().await;
                }
            }
        );
        
        // Create unified timeline cache actor
        let cache = Actor::new(TimelineCache::new(), async move |_cache_handle| {
            // Cache is updated by other actors
            loop {
                futures::future::pending::<()>().await;
            }
        });
        
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
            let mut pan_left_started = pan_left_started_stream.fuse();
            
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
                    complete => break,
                }
            }
        });
        
        let panning_right = Actor::new(false, async move |panning_handle| {
            let mut pan_right_started = pan_right_started_stream.fuse();
            
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
                    complete => break,
                }
            }
        });
        
        let cursor_moving_left = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let cursor_moving_right = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let shift_pressed = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Mouse tracking actors
        let mouse_x = Actor::new(0.0_f32, async move |mouse_x_handle| {
            let mut mouse_moved = mouse_moved_stream.fuse();
            
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
        let zoom_center = Actor::new(TimeNs::ZERO, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Canvas dimension actors
        let canvas_width = Actor::new(800.0_f32, async move |width_handle| {
            let mut canvas_resized = canvas_resized_stream.fuse();
            
            loop {
                select! {
                    event = canvas_resized.next() => {
                        match event {
                            Some((width, _height)) => width_handle.set(width),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let canvas_height = Actor::new(400.0_f32, async move |_height_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Signal values ActorMap
        let signal_values = ActorMap::new(BTreeMap::new(), async move |values_handle| {
            let mut signal_values_updated = signal_values_updated_stream.fuse();
            
            loop {
                select! {
                    event = signal_values_updated.next() => {
                        match event {
                            Some(updated_values) => {
                                for (signal_id, value) in updated_values {
                                    values_handle.lock_mut().insert_cloned(signal_id, value);
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        // Variable formats ActorMap
        let variable_formats = ActorMap::new(BTreeMap::new(), async move |_formats_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Canvas state actors
        let has_pending_request = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let canvas_cache = ActorMap::new(BTreeMap::new(), async move |_cache_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let force_redraw = Actor::new(0_u32, async move |redraw_handle| {
            let mut redraw_requested = redraw_requested_stream.fuse();
            
            loop {
                select! {
                    event = redraw_requested.next() => {
                        match event {
                            Some(()) => redraw_handle.update_mut(|counter| *counter += 1),
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
            coordinates,
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
            variable_formats,
            
            // Canvas state
            has_pending_request,
            canvas_cache,
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
            fit_all_clicked_relay,
            
            // System event relays
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
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
    
    /// Get reactive signal for timeline coordinates
    pub fn coordinates_signal(&self) -> impl zoon::Signal<Item = TimelineCoordinates> {
        self.coordinates.signal()
    }
    
    /// Get reactive signal for unified timeline cache
    pub fn cache_signal(&self) -> impl zoon::Signal<Item = TimelineCache> {
        self.cache.signal()
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
        use zoon::SignalExt;
        self.signal_values.entries_signal_vec().to_signal_cloned().map(|entries| {
            entries.into_iter().collect()
        })
    }
    
    /// Get reactive signal for variable formats
    pub fn variable_formats_signal(&self) -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
        use zoon::SignalExt;
        self.variable_formats.entries_signal_vec().to_signal_cloned().map(|entries| {
            entries.into_iter().collect()
        })
    }
    
    // === CANVAS STATE SIGNALS ===
    
    /// Get reactive signal for pending request status
    pub fn has_pending_request_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.has_pending_request.signal()
    }
    
    /// Get reactive signal for canvas cache
    pub fn canvas_cache_signal(&self) -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
        use zoon::SignalExt;
        self.canvas_cache.entries_signal_vec().to_signal_cloned().map(|entries| {
            entries.into_iter().collect()
        })
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

    // === STATIC SIGNAL ACCESSORS FOR GLOBAL FUNCTIONS ===
    
    /// Static version of variable_formats_signal for global access
    pub fn variable_formats_signal_static() -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
        use zoon::SignalExt;
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .variable_formats.entries_signal_vec().to_signal_cloned().map(|entries| {
                entries.into_iter().collect()
            })
    }
    
    /// Static version of has_pending_request_signal for global access
    pub fn has_pending_request_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .has_pending_request.signal()
    }
    
    /// Static version of canvas_cache_signal for global access  
    pub fn canvas_cache_signal_static() -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
        use zoon::SignalExt;
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .canvas_cache.entries_signal_vec().to_signal_cloned().map(|entries| {
                entries.into_iter().collect()
            })
    }
    
    /// Static version of force_redraw_signal for global access
    pub fn force_redraw_signal_static() -> impl zoon::Signal<Item = u32> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .force_redraw.signal()
    }
    
    /// Static version of last_redraw_time_signal for global access
    pub fn last_redraw_time_signal_static() -> impl zoon::Signal<Item = f64> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .last_redraw_time.signal()
    }
    
    /// Static version of last_canvas_update_signal for global access
    pub fn last_canvas_update_signal_static() -> impl zoon::Signal<Item = u64> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .last_canvas_update.signal()
    }
    
    /// Static version of mouse_x_signal for global access
    pub fn mouse_x_signal_static() -> impl zoon::Signal<Item = f32> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .mouse_x.signal()
    }
    
    /// Static version of mouse_time_signal for global access
    pub fn mouse_time_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .mouse_time.signal()
    }
    
    /// Static version of zoom_center_signal for global access  
    pub fn zoom_center_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .zoom_center.signal()
    }
    
    /// Static version of canvas_width_signal for global access
    pub fn canvas_width_signal_static() -> impl zoon::Signal<Item = f32> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .canvas_width.signal()
    }
    
    /// Static version of canvas_height_signal for global access
    pub fn canvas_height_signal_static() -> impl zoon::Signal<Item = f32> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .canvas_height.signal()
    }
    
    /// Static version of signal_values_signal for global access
    pub fn signal_values_signal_static() -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
        use zoon::SignalExt;
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .signal_values.entries_signal_vec().to_signal_cloned().map(|entries| {
                entries.into_iter().collect()
            })
    }

    // === MORE STATIC SIGNAL ACCESSORS ===
    
    /// Static version of cursor_position_signal for global access
    pub fn cursor_position_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .cursor_position.signal()
    }
    
    /// Static version of cursor_position_seconds_signal for global access  
    pub fn cursor_position_seconds_signal_static() -> impl zoon::Signal<Item = f64> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .cursor_position.signal_ref(|ns| ns.display_seconds())
    }
    
    /// Static version of viewport_signal for global access
    pub fn viewport_signal_static() -> impl zoon::Signal<Item = Viewport> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .viewport.signal()
    }
    
    /// Static version of ns_per_pixel_signal for global access
    pub fn ns_per_pixel_signal_static() -> impl zoon::Signal<Item = NsPerPixel> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .ns_per_pixel.signal()
    }
    
    /// Static version of coordinates_signal for global access
    pub fn coordinates_signal_static() -> impl zoon::Signal<Item = TimelineCoordinates> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .coordinates.signal()
    }
    
    /// Static version of cache_signal for global access
    pub fn cache_signal_static() -> impl zoon::Signal<Item = TimelineCache> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .cache.signal()
    }
    
    /// Static version of cursor_initialized_signal for global access
    pub fn cursor_initialized_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .cursor_initialized.signal()
    }
    
    // === CONTROL FLAGS STATIC SIGNALS ===
    
    /// Static version of zooming_in_signal for global access
    pub fn zooming_in_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .zooming_in.signal()
    }
    
    /// Static version of zooming_out_signal for global access
    pub fn zooming_out_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .zooming_out.signal()
    }
    
    /// Static version of panning_left_signal for global access
    pub fn panning_left_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .panning_left.signal()
    }
    
    /// Static version of panning_right_signal for global access
    pub fn panning_right_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .panning_right.signal()
    }
    
    /// Static version of cursor_moving_left_signal for global access
    pub fn cursor_moving_left_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .cursor_moving_left.signal()
    }
    
    /// Static version of cursor_moving_right_signal for global access
    pub fn cursor_moving_right_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .cursor_moving_right.signal()
    }
    
    /// Static version of shift_pressed_signal for global access
    pub fn shift_pressed_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .expect("WaveformTimeline not initialized")
            .shift_pressed.signal()
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
            let cursor_ns = cursor_position * 1_000_000_000.0; // Convert seconds to ns
            
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
                min_time = min_time.min(first.time_ns as f64 / 1_000_000_000.0);
            }
            if let Some(last) = signal_transitions.last() {
                max_time = max_time.max(last.time_ns as f64 / 1_000_000_000.0);
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

/// Global WaveformTimeline instance
static WAVEFORM_TIMELINE_INSTANCE: std::sync::OnceLock<WaveformTimeline> = std::sync::OnceLock::new();

/// Initialize the WaveformTimeline domain (call once on app startup)
/// Replaces all 25 global mutables with unified Actor+Relay architecture
pub async fn initialize_waveform_timeline() -> WaveformTimeline {
    let waveform_timeline = WaveformTimeline::new().await;
    WAVEFORM_TIMELINE_INSTANCE.set(waveform_timeline.clone())
        .expect("WaveformTimeline already initialized - initialize_waveform_timeline() should only be called once");
    waveform_timeline
}

/// Get the global WaveformTimeline instance
fn get_waveform_timeline() -> WaveformTimeline {
    WAVEFORM_TIMELINE_INSTANCE.get()
        .expect("WaveformTimeline not initialized - call initialize_waveform_timeline() first")
        .clone()
}

// ===== CONVENIENCE FUNCTIONS FOR GLOBAL ACCESS =====

// === CORE TIMELINE STATE ACCESS ===

/// Get cursor position signal (replaces _TIMELINE_CURSOR_NS.signal())
pub fn cursor_position_signal() -> impl zoon::Signal<Item = TimeNs> {
    WaveformTimeline::cursor_position_signal_static()
}

/// Get cursor position in seconds signal (compatibility)
pub fn cursor_position_seconds_signal() -> impl zoon::Signal<Item = f64> {
    WaveformTimeline::cursor_position_seconds_signal_static()
}

/// Get viewport signal (replaces _TIMELINE_VIEWPORT.signal())
pub fn viewport_signal() -> impl zoon::Signal<Item = Viewport> {
    WaveformTimeline::viewport_signal_static()
}

/// Get ns per pixel signal (replaces _TIMELINE_NS_PER_PIXEL.signal())
pub fn ns_per_pixel_signal() -> impl zoon::Signal<Item = NsPerPixel> {
    WaveformTimeline::ns_per_pixel_signal_static()
}

/// Get coordinates signal (replaces _TIMELINE_COORDINATES.signal())
pub fn coordinates_signal() -> impl zoon::Signal<Item = TimelineCoordinates> {
    WaveformTimeline::coordinates_signal_static()
}

/// Get unified cache signal (replaces UNIFIED_TIMELINE_CACHE.signal())
pub fn unified_timeline_cache_signal() -> impl zoon::Signal<Item = TimelineCache> {
    WaveformTimeline::cache_signal_static()
}

/// Get cursor initialized signal (replaces STARTUP_CURSOR_POSITION_SET.signal())
pub fn startup_cursor_position_set_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::cursor_initialized_signal_static()
}

// === CONTROL FLAG SIGNALS ===

/// Get zooming in signal (replaces IS_ZOOMING_IN.signal())
pub fn is_zooming_in_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::zooming_in_signal_static()
}

/// Get zooming out signal (replaces IS_ZOOMING_OUT.signal())
pub fn is_zooming_out_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::zooming_out_signal_static()
}

/// Get panning left signal (replaces IS_PANNING_LEFT.signal())
pub fn is_panning_left_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::panning_left_signal_static()
}

/// Get panning right signal (replaces IS_PANNING_RIGHT.signal())
pub fn is_panning_right_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::panning_right_signal_static()
}

/// Get cursor moving left signal (replaces IS_CURSOR_MOVING_LEFT.signal())
pub fn is_cursor_moving_left_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::cursor_moving_left_signal_static()
}

/// Get cursor moving right signal (replaces IS_CURSOR_MOVING_RIGHT.signal())
pub fn is_cursor_moving_right_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::cursor_moving_right_signal_static()
}

/// Get shift pressed signal (replaces IS_SHIFT_PRESSED.signal())
pub fn is_shift_pressed_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::shift_pressed_signal_static()
}

// === MOUSE TRACKING SIGNALS ===

/// Get mouse X position signal (replaces MOUSE_X_POSITION.signal())
pub fn mouse_x_position_signal() -> impl zoon::Signal<Item = f32> {
    WaveformTimeline::mouse_x_signal_static()
}

/// Get mouse time signal (replaces MOUSE_TIME_NS.signal())
pub fn mouse_time_ns_signal() -> impl zoon::Signal<Item = TimeNs> {
    WaveformTimeline::mouse_time_signal_static()
}

// === ZOOM/PAN SIGNALS ===

/// Get zoom center signal (replaces ZOOM_CENTER_NS.signal())
pub fn zoom_center_ns_signal() -> impl zoon::Signal<Item = TimeNs> {
    WaveformTimeline::zoom_center_signal_static()
}

/// Get canvas width signal (replaces CANVAS_WIDTH.signal())
pub fn canvas_width_signal() -> impl zoon::Signal<Item = f32> {
    WaveformTimeline::canvas_width_signal_static()
}

/// Get canvas height signal (replaces CANVAS_HEIGHT.signal())
pub fn canvas_height_signal() -> impl zoon::Signal<Item = f32> {
    WaveformTimeline::canvas_height_signal_static()
}

/// Get signal values signal (replaces SIGNAL_VALUES.signal())
pub fn signal_values_signal() -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
    WaveformTimeline::signal_values_signal_static()
}

/// Get variable formats signal (replaces SELECTED_VARIABLE_FORMATS.signal())
pub fn selected_variable_formats_signal() -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
    WaveformTimeline::variable_formats_signal_static()
}

// === CANVAS STATE SIGNALS ===

/// Get pending request signal (replaces _HAS_PENDING_REQUEST.signal())
pub fn has_pending_request_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::has_pending_request_signal_static()
}

/// Get canvas cache signal (replaces PROCESSED_CANVAS_CACHE.signal())
pub fn processed_canvas_cache_signal() -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
    WaveformTimeline::canvas_cache_signal_static()
}

/// Get force redraw signal (replaces FORCE_REDRAW.signal())
pub fn force_redraw_signal() -> impl zoon::Signal<Item = u32> {
    WaveformTimeline::force_redraw_signal_static()
}

/// Get last redraw time signal (replaces LAST_REDRAW_TIME.signal())
pub fn last_redraw_time_signal() -> impl zoon::Signal<Item = f64> {
    WaveformTimeline::last_redraw_time_signal_static()
}

/// Get last canvas update signal (replaces LAST_CANVAS_UPDATE.signal())
pub fn last_canvas_update_signal() -> impl zoon::Signal<Item = u64> {
    WaveformTimeline::last_canvas_update_signal_static()
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