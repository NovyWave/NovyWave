//! WaveformTimeline domain for timeline management using Actor+Relay architecture
//!
//! Consolidated timeline management domain to replace global mutables with event-driven architecture.
//! Manages cursor position, viewport ranges, zoom levels, and cached waveform data.

use crate::actors::{Actor, ActorVec, ActorMap, ActorMapHandle, Relay, relay};
use crate::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates};
use shared::{SignalTransition, SignalValue, WaveformFile};
use zoon::Task;
use indexmap::IndexSet;
use futures::StreamExt;
use std::collections::{HashMap, BTreeMap};

/// Domain-driven timeline management with Actor+Relay architecture.
/// 
/// Replaces timeline-related global mutables with cohesive event-driven state management.
/// Tracks cursor position, viewport ranges, zoom levels, and cached signal data.
#[derive(Clone, Debug)]
pub struct WaveformTimeline {
    /// Current cursor position in nanoseconds
    cursor_position: Actor<f64>,
    
    /// Timeline viewport (visible time range)
    viewport: Actor<Viewport>,
    
    /// Timeline resolution (nanoseconds per pixel)
    ns_per_pixel: Actor<NsPerPixel>,
    
    /// Unified timeline coordinates for integer-based calculations
    coordinates: Actor<TimelineCoordinates>,
    
    /// Cached signal transitions for timeline rendering
    signal_transitions: ActorMap<String, Vec<SignalTransition>>,
    
    /// Current signal values at cursor position
    cursor_values: ActorMap<String, SignalValue>,
    
    /// Timeline statistics and metadata
    timeline_stats: Actor<TimelineStats>,
    
    // === USER TIMELINE INTERACTION EVENTS ===
    /// User clicked on timeline canvas (relative coordinates)
    pub cursor_clicked_relay: Relay<(f64, f64)>,
    
    /// User moved mouse over timeline canvas
    pub mouse_moved_relay: Relay<(f32, f32)>,
    
    /// User dragged timeline cursor
    pub cursor_dragged_relay: Relay<f64>,
    
    /// User performed zoom gesture
    pub zoom_changed_relay: Relay<f32>,
    
    /// User panned the timeline view
    pub pan_performed_relay: Relay<(f64, f64)>,
    
    /// User double-clicked to fit all data
    pub fit_all_clicked_relay: Relay<()>,
    
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
pub struct TimelineStats {
    pub total_signals: usize,
    pub cached_transitions: usize,
    pub min_time: f64,
    pub max_time: f64,
    pub time_range: f64,
}

impl WaveformTimeline {
    /// Create a new WaveformTimeline domain with event processors
    pub async fn new() -> Self {
        // Create relays for user interactions
        let (cursor_clicked_relay, cursor_clicked_stream) = relay::<(f64, f64)>();
        let (mouse_moved_relay, _mouse_moved_stream) = relay::<(f32, f32)>();
        let (cursor_dragged_relay, cursor_dragged_stream) = relay::<f64>();
        let (zoom_changed_relay, zoom_changed_stream) = relay::<f32>();
        let (pan_performed_relay, pan_performed_stream) = relay::<(f64, f64)>();
        let (fit_all_clicked_relay, fit_all_clicked_stream) = relay::<()>();
        
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
        
        // Create relays for system events
        let (data_loaded_relay, data_loaded_stream) = relay::<(String, WaveformFile)>();
        let (transitions_cached_relay, transitions_cached_stream) = relay::<(String, Vec<SignalTransition>)>();
        let (cursor_values_updated_relay, cursor_values_updated_stream) = relay::<BTreeMap<String, SignalValue>>();
        let (timeline_bounds_calculated_relay, timeline_bounds_calculated_stream) = relay::<(f64, f64)>();
        let (viewport_changed_relay, viewport_changed_stream) = relay::<(f64, f64)>();
        
        // Create cursor position actor with event handling
        let cursor_position = Actor::new(0.0, async move |cursor_handle| {
            let mut cursor_clicked = cursor_clicked_stream;
            let mut cursor_dragged = cursor_dragged_stream;
            let mut left_key_pressed = left_key_pressed_stream;
            let mut right_key_pressed = right_key_pressed_stream;
            
            loop {
                if let Some((click_x, _canvas_left)) = cursor_clicked.next().await {
                    // Convert click coordinates to timeline time (simplified for now)
                    // TODO: Use proper coordinate conversion once legacy systems are removed
                    cursor_handle.set(click_x);
                } else if let Some(time) = cursor_dragged.next().await {
                    cursor_handle.set(time);
                } else if let Some(()) = left_key_pressed.next().await {
                    cursor_handle.update(|current| *current = (*current - 1000.0_f64).max(0.0_f64)); // Move left by 1μs
                } else if let Some(()) = right_key_pressed.next().await {
                    cursor_handle.update(|current| *current = *current + 1000.0_f64); // Move right by 1μs
                } else {
                    break;
                }
            }
        });
        
        // Create viewport actor (replacing visible_range)
        let viewport = Actor::new(
            Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(100.0)), 
            async move |viewport_handle| {
                let mut pan_performed = pan_performed_stream;
                let mut viewport_changed = viewport_changed_stream;
                let mut timeline_bounds_calculated = timeline_bounds_calculated_stream;
                let mut fit_all_clicked = fit_all_clicked_stream;
                
                loop {
                    if let Some((start, end)) = pan_performed.next().await {
                        let new_viewport = Viewport::new(
                            TimeNs::from_external_seconds(start),
                            TimeNs::from_external_seconds(end)
                        );
                        viewport_handle.set(new_viewport);
                    } else if let Some((start, end)) = viewport_changed.next().await {
                        let new_viewport = Viewport::new(
                            TimeNs::from_external_seconds(start),
                            TimeNs::from_external_seconds(end)
                        );
                        viewport_handle.set(new_viewport);
                    } else if let Some((min_time, max_time)) = timeline_bounds_calculated.next().await {
                        let new_viewport = Viewport::new(
                            TimeNs::from_external_seconds(min_time),
                            TimeNs::from_external_seconds(max_time)
                        );
                        viewport_handle.set(new_viewport);
                    } else if let Some(()) = fit_all_clicked.next().await {
                        // Will be updated by timeline bounds calculation
                    } else {
                        break;
                    }
                }
            }
        );
        
        // Create ns_per_pixel actor (replacing zoom levels)
        let ns_per_pixel = Actor::new(NsPerPixel::MEDIUM_ZOOM, async move |ns_per_pixel_handle| {
            let mut zoom_changed = zoom_changed_stream;
            let mut zoom_in_pressed = zoom_in_pressed_stream;
            let mut zoom_out_pressed = zoom_out_pressed_stream;
            
            loop {
                if let Some(new_zoom) = zoom_changed.next().await {
                    // Convert zoom factor to nanoseconds per pixel
                    let ns_per_pixel_value = (1_000_000.0 / new_zoom).max(1.0) as u64;
                    ns_per_pixel_handle.set(NsPerPixel(ns_per_pixel_value));
                } else if let Some(()) = zoom_in_pressed.next().await {
                    ns_per_pixel_handle.update(|current| *current = current.zoom_in_smooth(0.3));
                } else if let Some(()) = zoom_out_pressed.next().await {
                    ns_per_pixel_handle.update(|current| *current = current.zoom_out_smooth(0.3));
                } else {
                    break;
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
                // TODO: Update coordinates when cursor, viewport, or zoom changes
                // This will be implemented as coordination between the other actors
                loop {
                    // Placeholder - coordinates will be updated reactively
                    break;
                }
            }
        );
        
        // Create signal transitions cache
        let signal_transitions = ActorMap::new(BTreeMap::new(), async move |transitions_handle| {
            let mut transitions_cached = transitions_cached_stream;
            let mut data_loaded = data_loaded_stream;
            
            loop {
                if let Some((signal_id, transitions)) = transitions_cached.next().await {
                    transitions_handle.insert(signal_id, transitions);
                } else if let Some((file_id, waveform_data)) = data_loaded.next().await {
                    // Process waveform data and extract transitions
                    Self::process_waveform_data(&transitions_handle, file_id, waveform_data);
                } else {
                    break;
                }
            }
        });
        
        // Create cursor values cache
        let cursor_values = ActorMap::new(BTreeMap::new(), async move |values_handle| {
            let mut cursor_values_updated = cursor_values_updated_stream;
            
            while let Some(updated_values) = cursor_values_updated.next().await {
                for (signal_id, value) in updated_values {
                    values_handle.insert(signal_id, value);
                }
            }
        });
        
        // Create timeline stats actor
        let timeline_stats = Actor::new(TimelineStats::default(), async move |_stats_handle| {
            // Stats will be updated through separate events
            loop {
                futures::future::pending::<()>().await;
            }
        });
        
        Self {
            cursor_position,
            viewport,
            ns_per_pixel,
            coordinates,
            signal_transitions,
            cursor_values,
            timeline_stats,
            
            cursor_clicked_relay,
            mouse_moved_relay,
            cursor_dragged_relay,
            zoom_changed_relay,
            pan_performed_relay,
            fit_all_clicked_relay,
            
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
            
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
        }
    }
    
    // === REACTIVE SIGNAL ACCESS ===
    
    /// Get reactive signal for cursor position
    pub fn cursor_position_signal(&self) -> impl zoon::Signal<Item = f64> {
        self.cursor_position.signal()
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
    
    /// Get reactive signal for cached transitions
    pub fn signal_transitions_signal(&self) -> impl zoon::Signal<Item = BTreeMap<String, Vec<SignalTransition>>> {
        self.signal_transitions.signal()
    }
    
    /// Get reactive signal for cursor values
    pub fn cursor_values_signal(&self) -> impl zoon::Signal<Item = BTreeMap<String, SignalValue>> {
        self.cursor_values.signal()
    }
    
    /// Get reactive signal for timeline statistics
    pub fn timeline_stats_signal(&self) -> impl zoon::Signal<Item = TimelineStats> {
        self.timeline_stats.signal()
    }
    
    /// Get signal for specific signal transitions
    pub fn signal_transitions_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<Vec<SignalTransition>>> {
        use zoon::SignalExt;
        self.signal_transitions.signal().map(move |transitions| {
            transitions.get(&signal_id).cloned()
        })
    }
    
    /// Get signal for cursor value of specific signal
    pub fn cursor_value_for_signal(&self, signal_id: String) -> impl zoon::Signal<Item = Option<SignalValue>> {
        use zoon::SignalExt;
        self.cursor_values.signal().map(move |values| {
            values.get(&signal_id).cloned()
        })
    }
    
    /// Check if cursor is within visible range
    pub fn is_cursor_visible_signal(&self) -> impl zoon::Signal<Item = bool> {
        use zoon::SignalExt;
        // Simplified version - can be enhanced later with proper signal combining
        self.cursor_position.signal().map(|_| true) // Placeholder implementation
    }
    
    /// Get time duration per pixel at current zoom
    pub fn time_per_pixel_signal(&self, canvas_width: f32) -> impl zoon::Signal<Item = f64> {
        use zoon::SignalExt;
        self.viewport.signal()
            .map(move |viewport| {
                let start_seconds = viewport.start.display_seconds();
                let end_seconds = viewport.end.display_seconds();
                (end_seconds - start_seconds) / canvas_width as f64
            })
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

impl WaveformTimeline {
    /// Process loaded waveform data and cache transitions
    fn process_waveform_data(
        transitions_handle: &ActorMapHandle<String, Vec<SignalTransition>>,
        file_id: String,
        waveform_data: WaveformFile
    ) {
        // Process each scope and signal in the waveform data
        for scope_data in &waveform_data.scopes {
            for signal_data in &scope_data.variables {
                let signal_id = format!("{}|{}|{}", file_id, scope_data.name, signal_data.name);
                
                // For now, create empty transitions - actual loading will happen via different mechanism
                let transitions: Vec<SignalTransition> = Vec::new();
                
                transitions_handle.insert(signal_id, transitions);
            }
        }
    }
    
    /// Update cursor values based on current position and cached transitions
    fn update_cursor_values_from_cache(
        cursor_position: f64,
        transitions: &BTreeMap<String, Vec<SignalTransition>>,
        values_handle: &ActorMapHandle<String, SignalValue>
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
                values_handle.insert(signal_id.clone(), value);
            } else {
                values_handle.insert(signal_id.clone(), SignalValue::Missing);
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

/// Global WaveformTimeline instance
static WAVEFORM_TIMELINE_INSTANCE: std::sync::OnceLock<WaveformTimeline> = std::sync::OnceLock::new();

/// Initialize the WaveformTimeline domain (call once on app startup)
pub async fn initialize_waveform_timeline() -> WaveformTimeline {
    let waveform_timeline = WaveformTimeline::new().await;
    WAVEFORM_TIMELINE_INSTANCE.set(waveform_timeline.clone())
        .expect("WaveformTimeline already initialized - initialize_waveform_timeline() should only be called once");
    waveform_timeline
}

/// Get the global WaveformTimeline instance
pub fn get_waveform_timeline() -> WaveformTimeline {
    WAVEFORM_TIMELINE_INSTANCE.get()
        .expect("WaveformTimeline not initialized - call initialize_waveform_timeline() first")
        .clone()
}