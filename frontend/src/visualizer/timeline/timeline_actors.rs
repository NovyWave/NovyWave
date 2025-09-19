//! WaveformTimeline domain for timeline management using Actor+Relay architecture
//!
//! Consolidated timeline management domain to replace global mutables with event-driven architecture.
//! Manages cursor position, viewport ranges, zoom levels, and cached waveform data.

use crate::dataflow::{Actor, ActorMap, Relay, relay};
use futures::{StreamExt, select};
use shared::{SignalTransition, SignalValue, VarFormat, WaveformFile};
use std::collections::{BTreeMap, HashMap};
use zoon::*;

// Import time domain
use super::time_domain::{NsPerPixel, TimeNs, Viewport};

// Import extracted domain modules
use super::canvas_state::{CanvasStateController, TimelineStats};
use super::cursor_animation::CursorAnimationController;
use super::maximum_timeline_range::MaximumTimelineRange;
use super::panning_controller::PanningController;
use super::timeline_cache::{TimelineCache, TimelineCacheController};
use super::zoom_controller::ZoomController;

/// Timeline management with Actor+Relay architecture
#[derive(Clone, Debug)]
pub struct WaveformTimeline {
    // Core timeline state
    /// Current cursor position in nanoseconds
    cursor_position: Actor<TimeNs>,

    /// Timeline viewport (visible time range)
    viewport: Actor<Viewport>,

    /// Zoom controller
    zoom_controller: ZoomController,

    /// Smooth cursor movement control flags
    cursor_moving_left: Actor<bool>,
    cursor_moving_right: Actor<bool>,

    /// Shift key state tracking for modifier combinations
    shift_pressed: Actor<bool>,

    /// Mouse position tracking for zoom center
    mouse_x: Actor<f32>,
    mouse_time: Actor<TimeNs>,

    /// Canvas dimensions
    canvas_width: Actor<f32>,
    canvas_height: Actor<f32>,

    /// Current signal values at cursor position
    signal_values: ActorMap<String, SignalValue>,

    /// Format selections for selected variables
    variable_formats: ActorMap<String, VarFormat>,

    // === EXTRACTED DOMAIN CONTROLLERS ===
    /// Maximum timeline range for zoom calculations
    maximum_timeline_range: MaximumTimelineRange,

    /// Timeline cache controller
    timeline_cache_controller: TimelineCacheController,

    /// Cursor animation controller
    cursor_animation_controller: CursorAnimationController,

    /// Panning controller
    panning_controller: PanningController,

    /// Canvas state controller
    canvas_state_controller: CanvasStateController,

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
    pub signal_values_updated_relay: Relay<HashMap<String, SignalValue>>,

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

impl WaveformTimeline {
    /// Access to ns_per_pixel from zoom controller
    pub fn ns_per_pixel(&self) -> &Actor<NsPerPixel> {
        &self.zoom_controller.ns_per_pixel
    }

    /// Access to viewport signal
    pub fn viewport_signal(&self) -> impl zoon::Signal<Item = Viewport> {
        self.viewport.signal()
    }

    /// Access to canvas width signal
    pub fn canvas_width_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.canvas_width.signal()
    }

    // TODO: Remove these architectural violations - no "non-reactive contexts" in Actor+Relay architecture
    // pub fn current_viewport(&self) -> Viewport {
    //     self.viewport.signal().to_stream().next().await
    // }
    //
    // pub fn current_canvas_width(&self) -> f32 {
    //     self.canvas_width.signal().to_stream().next().await
    // }

    /// Create a new WaveformTimeline domain with Actor+Relay architecture
    pub async fn new(maximum_timeline_range: MaximumTimelineRange) -> Self {
        // Clone-at-entry pattern: Create clones for multiple Actor usage
        let maximum_timeline_range_for_viewport = maximum_timeline_range.clone();
        let maximum_timeline_range_for_ns_per_pixel = maximum_timeline_range.clone();

        // Create relays for comprehensive user interactions
        let (cursor_clicked_relay, cursor_clicked_stream) = relay::<TimeNs>();
        let (cursor_moved_relay, cursor_moved_stream) = relay::<TimeNs>();
        // Note: zoom_in_started_relay and zoom_out_started_relay will come from zoom_controller
        let (pan_left_started_relay, pan_left_started_stream) = relay::<()>();
        let (pan_right_started_relay, pan_right_started_stream) = relay::<()>();
        let (mouse_moved_relay, mouse_moved_stream) = relay::<(f32, TimeNs)>();
        let (canvas_resized_relay, _canvas_resized_stream) = relay::<(f32, f32)>();
        let (redraw_requested_relay, redraw_requested_stream) = relay::<()>();
        let (signal_values_updated_relay, signal_values_updated_stream) =
            relay::<HashMap<String, SignalValue>>();
        let (variable_format_updated_relay, variable_format_updated_stream) =
            relay::<(String, VarFormat)>();

        // Create relays for keyboard navigation
        let (left_key_pressed_relay, left_key_pressed_stream) = relay::<()>();
        let (right_key_pressed_relay, right_key_pressed_stream) = relay::<()>();
        // Note: zoom_in_pressed_relay and zoom_out_pressed_relay will come from zoom_controller
        let (pan_left_pressed_relay, pan_left_pressed_stream) = relay::<()>();
        let (pan_right_pressed_relay, pan_right_pressed_stream) = relay::<()>();
        let (jump_to_previous_pressed_relay, _jump_to_previous_pressed_stream) = relay::<()>();
        let (jump_to_next_pressed_relay, _jump_to_next_pressed_stream) = relay::<()>();
        // Note: reset_zoom_pressed_relay and reset_zoom_center_pressed_relay will come from zoom_controller
        let (cursor_center_at_viewport_relay, cursor_center_at_viewport_stream) = relay::<()>();
        let (zoom_center_reset_to_zero_relay, zoom_center_reset_to_zero_stream) = relay::<()>();
        let (shift_key_pressed_relay_var, shift_key_pressed_stream) = relay::<()>();
        let (shift_key_released_relay_var, shift_key_released_stream) = relay::<()>();

        // Create animation state relays
        let (panning_left_started_relay_var, panning_left_started_stream) = relay::<()>();
        let (panning_left_stopped_relay_var, panning_left_stopped_stream) = relay::<()>();
        let (panning_right_started_relay_var, panning_right_started_stream) = relay::<()>();
        let (panning_right_stopped_relay_var, panning_right_stopped_stream) = relay::<()>();
        let (cursor_moving_left_started_relay_var, cursor_moving_left_started_stream) =
            relay::<()>();
        let (cursor_moving_left_stopped_relay_var, cursor_moving_left_stopped_stream) =
            relay::<()>();
        let (cursor_moving_right_started_relay_var, cursor_moving_right_started_stream) =
            relay::<()>();
        let (cursor_moving_right_stopped_relay_var, cursor_moving_right_stopped_stream) =
            relay::<()>();
        // Create relays for system events - will be connected to controllers later
        let (timeline_bounds_calculated_relay, timeline_bounds_calculated_stream) =
            relay::<(f64, f64)>();
        // Note: ns_per_pixel_changed_relay will come from zoom_controller

        // Create cursor position actor with comprehensive event handling
        let initial_cursor_position = TimeNs::ZERO;

        let cursor_position = Actor::new(initial_cursor_position, {
            let cursor_moving_left_started = cursor_moving_left_started_relay_var.clone();
            let cursor_moving_right_started = cursor_moving_right_started_relay_var.clone();
            async move |cursor_handle| {
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
                                Some(()) => {
                                    // Trigger smooth cursor movement animation
                                    cursor_moving_left_started.send(());

                                    // Move cursor by step
                                    cursor_handle.update_mut(|current| {
                                        let step_size = 1_000_000; // 1 millisecond in nanoseconds
                                        let new_time = current.nanos().saturating_sub(step_size);
                                        *current = TimeNs::from_nanos(new_time);
                                    });
                                },
                                None => break,
                            }
                        }
                        event = right_key_pressed.next() => {
                            match event {
                                Some(()) => {
                                    // Trigger smooth cursor movement animation
                                    cursor_moving_right_started.send(());

                                    // Move cursor by step
                                    cursor_handle.update_mut(|current| {
                                        let step_size = 1_000_000; // 1 millisecond in nanoseconds
                                        let old_time = current.nanos();
                                        let new_time = old_time.saturating_add(step_size);
                                        *current = TimeNs::from_nanos(new_time);
                                    });
                                },
                                None => break,
                            }
                        }
                        event = cursor_center_at_viewport.next() => {
                            match event {
                                Some(()) => {
                                    // TODO: Cursor centering will be handled by viewport Actor
                                    // which has access to viewport state. For now, set to zero.
                                    cursor_handle.set(TimeNs::ZERO);
                                },
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        // Clone relays needed by multiple actors and struct field before moving
        let cursor_center_at_viewport_relay_for_ns_per_pixel =
            cursor_center_at_viewport_relay.clone();
        let cursor_center_at_viewport_relay_for_struct = cursor_center_at_viewport_relay.clone();

        // Helper function to get initial viewport from file data
        let get_initial_viewport = || {
            // During initialization, return None viewport since range will be computed from actual data
            // The range will update via timeline_range_stream once files are loaded
            Viewport::new(TimeNs::ZERO, TimeNs::ZERO) // Will be updated when real data loads
        };

        // Create viewport actor with comprehensive event handling
        let initial_viewport = get_initial_viewport();
        let viewport = Actor::new(initial_viewport, {
            let cursor_center_at_viewport_relay_clone = cursor_center_at_viewport_relay.clone();
            async move |viewport_handle| {
                let mut timeline_bounds_calculated = timeline_bounds_calculated_stream;
                let mut timeline_range_stream = maximum_timeline_range_for_viewport
                    .range
                    .signal()
                    .to_stream()
                    .fuse();

                // TODO: ZoomController synchronization will be handled through relays

                // Cache current values pattern for timeline range
                let mut cached_timeline_range: Option<(f64, f64)> = None;

                loop {
                    select! {
                        event = timeline_bounds_calculated.next() => {
                            match event {
                                Some((min_time, max_time)) => {
                                    let new_viewport = Viewport::new(
                                        TimeNs::from_external_seconds(min_time),
                                        TimeNs::from_external_seconds(max_time)
                                    );
                                    viewport_handle.set(new_viewport);

                                    // Center cursor at viewport center
                                    let _center_time = (min_time + max_time) / 2.0;
                                    cursor_center_at_viewport_relay_clone.send(());
                                }
                                None => break,
                            }
                        }
                        range_update = timeline_range_stream.next() => {
                            match range_update {
                                Some(new_range) => {
                                    cached_timeline_range = new_range;
                                }
                                None => break,
                            }
                        }
                        // TODO: ZoomController viewport change integration will be handled through relays
                        // TODO: ZoomController fit-all functionality integration will be handled through relays
                        complete => break,
                    }
                }
            }
        });

        // Initialize zoom controller
        let zoom_controller = ZoomController::new(
            maximum_timeline_range_for_ns_per_pixel,
            viewport.clone(),
            cursor_center_at_viewport_relay_for_ns_per_pixel.clone(),
            zoom_center_reset_to_zero_relay.clone(),
            canvas_resized_relay.clone(),
        )
        .await;

        // Initialize timeline cache controller
        let timeline_cache_controller =
            TimelineCacheController::new(cursor_position.clone(), viewport.clone()).await;

        // Initialize panning controller
        let panning_controller = PanningController::new().await;

        // Connect keyboard pan events to panning controller
        {
            let pan_left_started = panning_controller.pan_left_started_relay.clone();
            let pan_right_started = panning_controller.pan_right_started_relay.clone();
            zoon::Task::start(async move {
                let mut pan_left_pressed = pan_left_pressed_stream;
                let mut pan_right_pressed = pan_right_pressed_stream;

                loop {
                    select! {
                        event = pan_left_pressed.next() => {
                            match event {
                                Some(()) => pan_left_started.send(()),
                                None => break,
                            }
                        }
                        event = pan_right_pressed.next() => {
                            match event {
                                Some(()) => pan_right_started.send(()),
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            });
        }

        // Create cursor moving actors for controller dependency
        let cursor_moving_left = Actor::new(false, async move |handle| {
            let mut cursor_started = cursor_moving_left_started_stream;
            let mut cursor_stopped = cursor_moving_left_stopped_stream;

            loop {
                select! {
                    event = cursor_started.next() => {
                        match event {
                            Some(()) => {
                                handle.set(true);
                            }
                            None => break,
                        }
                    }
                    event = cursor_stopped.next() => {
                        match event {
                            Some(()) => {
                                handle.set(false);
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
                            }
                            None => break,
                        }
                    }
                    event = cursor_stopped.next() => {
                        match event {
                            Some(()) => {
                                handle.set(false);
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });

        // Initialize cursor animation controller
        let cursor_animation_controller = CursorAnimationController::new(
            cursor_position.clone(),
            cursor_moving_left.clone(),
            cursor_moving_right.clone(),
        )
        .await;

        let shift_pressed = Actor::new(false, async move |handle| {
            let mut shift_pressed_stream = shift_key_pressed_stream;
            let mut shift_released_stream = shift_key_released_stream;

            loop {
                select! {
                    event = shift_pressed_stream.next() => {
                        match event {
                            Some(()) => {
                                handle.set(true);
                            },
                            None => break,
                        }
                    }
                    event = shift_released_stream.next() => {
                        match event {
                            Some(()) => {
                                handle.set(false);
                            },
                            None => break,
                        }
                    }
                }
            }
        });

        // Mouse tracking actors - create separate subscriptions from the relay
        let mouse_moved_broadcaster_for_x = mouse_moved_relay.clone();
        let mouse_moved_broadcaster_for_time = mouse_moved_relay.clone();
        let mouse_x = Actor::new(0.0_f32, async move |mouse_x_handle| {
            let mut mouse_moved = mouse_moved_broadcaster_for_x.subscribe();

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

        let mouse_time = Actor::new(TimeNs::ZERO, async move |mouse_time_handle| {
            let mut mouse_moved = mouse_moved_broadcaster_for_time.subscribe();

            loop {
                select! {
                    event = mouse_moved.next() => {
                        match event {
                            Some((_x_pos, time)) => mouse_time_handle.set(time),
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
            }
        });

        let canvas_height = Actor::new(0.0_f32, {
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

        // Actor+Relay architecture - no duplicate signal patterns needed

        // Signal values ActorMap
        let signal_values = ActorMap::new(BTreeMap::new(), {
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
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        // Variable formats ActorMap
        let variable_formats = ActorMap::new(BTreeMap::new(), {
            async move |formats_handle| {
                let mut variable_format_updated = variable_format_updated_stream;

                loop {
                    select! {
                        event = variable_format_updated.next() => {
                            match event {
                                Some((unique_id, format)) => {
                                    formats_handle.lock_mut().insert_cloned(unique_id, format);
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        // Initialize canvas state controller
        let canvas_state_controller = CanvasStateController::new(
            viewport.clone(),
            signal_values.clone(),
            timeline_cache_controller.cache.clone(),
        )
        .await;

        // Extract relay fields before moving controllers into struct
        let pan_left_started_relay = panning_controller.pan_left_started_relay.clone();
        let pan_right_started_relay = panning_controller.pan_right_started_relay.clone();
        let canvas_resized_relay_for_struct = canvas_state_controller.canvas_resized_relay.clone();
        let redraw_requested_relay_for_struct =
            canvas_state_controller.redraw_requested_relay.clone();
        let cursor_values_updated_relay = timeline_cache_controller
            .cursor_values_updated_relay
            .clone();
        let cursor_moving_right_stopped_relay = cursor_animation_controller
            .cursor_moving_right_stopped_relay
            .clone();
        let panning_right_stopped_relay = panning_controller.panning_right_stopped_relay.clone();

        // Extract remaining controller fields
        let zoom_in_started_relay = zoom_controller.zoom_in_started_relay.clone();
        let zoom_out_started_relay = zoom_controller.zoom_out_started_relay.clone();
        let zoom_in_pressed_relay = zoom_controller.zoom_in_pressed_relay.clone();
        let zoom_out_pressed_relay = zoom_controller.zoom_out_pressed_relay.clone();
        let reset_zoom_pressed_relay = zoom_controller.reset_zoom_pressed_relay.clone();
        let reset_zoom_center_pressed_relay =
            zoom_controller.reset_zoom_center_pressed_relay.clone();
        let panning_left_started_relay = panning_controller.panning_left_started_relay.clone();
        let panning_left_stopped_relay = panning_controller.panning_left_stopped_relay.clone();
        let panning_right_started_relay = panning_controller.panning_right_started_relay.clone();
        let cursor_moving_left_started_relay = cursor_animation_controller
            .cursor_moving_left_started_relay
            .clone();
        let cursor_moving_left_stopped_relay = cursor_animation_controller
            .cursor_moving_left_stopped_relay
            .clone();
        let cursor_moving_right_started_relay = cursor_animation_controller
            .cursor_moving_right_started_relay
            .clone();
        let zoom_center_follow_mouse_relay = zoom_controller.zoom_center_follow_mouse_relay.clone();
        let fit_all_clicked_relay = zoom_controller.fit_all_clicked_relay.clone();
        let data_loaded_relay = timeline_cache_controller.data_loaded_relay.clone();
        let transitions_cached_relay = timeline_cache_controller.transitions_cached_relay.clone();
        let viewport_changed_relay = zoom_controller.viewport_changed_relay.clone();
        let ns_per_pixel_changed_relay = zoom_controller.ns_per_pixel_changed_relay.clone();

        Self {
            // Core timeline state
            cursor_position,
            viewport,
            zoom_controller: zoom_controller.clone(),
            cursor_moving_left,
            cursor_moving_right,
            shift_pressed,

            // Mouse tracking
            mouse_x,
            mouse_time,

            // Zoom/pan state
            canvas_width,
            canvas_height,
            signal_values,
            variable_formats,

            // === EXTRACTED DOMAIN CONTROLLERS ===
            maximum_timeline_range,
            timeline_cache_controller,
            cursor_animation_controller,
            panning_controller,
            canvas_state_controller,

            // User interaction relays
            cursor_clicked_relay,
            cursor_moved_relay,
            zoom_in_started_relay,
            zoom_out_started_relay,
            pan_left_started_relay,
            pan_right_started_relay,
            mouse_moved_relay,
            canvas_resized_relay: canvas_resized_relay_for_struct,
            redraw_requested_relay: redraw_requested_relay_for_struct,
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

            // System event relays (from controllers)
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
            ns_per_pixel_changed_relay,
        }
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

impl WaveformTimeline {
    /// Process loaded waveform data and cache transitions
    fn process_waveform_data(
        transitions_handle: &zoon::MutableBTreeMap<String, Vec<SignalTransition>>,
        file_id: String,
        waveform_data: WaveformFile,
    ) {
        // Process each scope and signal in the waveform data
        for scope_data in &waveform_data.scopes {
            for signal_data in &scope_data.variables {
                let signal_id = format!("{}|{}|{}", file_id, scope_data.name, signal_data.name);

                // For now, create empty transitions - actual loading will happen via different mechanism
                let transitions: Vec<SignalTransition> = Vec::new();

                transitions_handle
                    .lock_mut()
                    .insert_cloned(signal_id, transitions);
            }
        }
    }

    /// Update cursor values based on current position and cached transitions
    fn update_cursor_values_from_cache(
        cursor_position: f64,
        transitions: &BTreeMap<String, Vec<SignalTransition>>,
        values_handle: &zoon::MutableBTreeMap<String, SignalValue>,
    ) {
        for (signal_id, signal_transitions) in transitions {
            // Find the most recent transition at or before cursor position
            let mut current_value = None;
            let cursor_ns = cursor_position * super::time_domain::NS_PER_SECOND; // Convert seconds to ns

            for transition in signal_transitions.iter() {
                if transition.time_ns as f64 <= cursor_ns {
                    current_value = Some(SignalValue::Present(transition.value.clone()));
                } else {
                    break;
                }
            }

            if let Some(value) = current_value {
                values_handle
                    .lock_mut()
                    .insert_cloned(signal_id.clone(), value);
            } else {
                values_handle
                    .lock_mut()
                    .insert_cloned(signal_id.clone(), SignalValue::Missing);
            }
        }
    }

    /// Calculate timeline bounds from all loaded data
    fn calculate_timeline_bounds(
        transitions: &BTreeMap<String, Vec<SignalTransition>>,
    ) -> (f64, f64) {
        let mut min_time = f64::MAX;
        let mut max_time = f64::MIN;

        for signal_transitions in transitions.values() {
            if let Some(first) = signal_transitions.first() {
                min_time = min_time.min(first.time_ns as f64 / super::time_domain::NS_PER_SECOND);
            }
            if let Some(last) = signal_transitions.last() {
                max_time = max_time.max(last.time_ns as f64 / super::time_domain::NS_PER_SECOND);
            }
        }

        if min_time == f64::MAX || max_time == f64::MIN {
            // No fallback values - return None through Option wrapper instead
            (0.0, 0.0) // Caller should handle this as invalid/no data case
        } else {
            (min_time, max_time)
        }
    }
}
