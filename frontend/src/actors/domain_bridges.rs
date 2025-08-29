//! Domain Bridges for Actor+Relay Migration
//!
//! Provides synchronization between new Actor+Relay domains and legacy global mutables
//! during the transition period. These bridges will be removed once migration is complete.

use crate::actors::{tracked_files_domain, waveform_timeline_domain};
use crate::state::TRACKED_FILES;
use crate::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates};
use zoon::{Task, SignalExt};
use futures::stream::select;
use std::sync::{Mutex, OnceLock};

/// Initialize bridges between Actor+Relay domains and legacy global state
/// This ensures backward compatibility during the migration period
pub async fn initialize_domain_bridges() {
    initialize_tracked_files_bridge();
    initialize_waveform_timeline_bridge();
    // More domain bridges will be added here as other domains are migrated
}

/// Bridge TrackedFiles domain changes to global TRACKED_FILES mutable
/// Keeps legacy code working while new code uses the domain
fn initialize_tracked_files_bridge() {
    let tracked_files = tracked_files_domain();
    
    // Synchronize domain changes to global mutable
    Task::start(async move {
        tracked_files.files_signal()
            .for_each(move |domain_files| async move {
                // Update global TRACKED_FILES to match domain state
                let mut global_files = TRACKED_FILES.lock_mut();
                global_files.replace_cloned(domain_files);
            })
            .await;
    });
    
    // TrackedFiles domain bridge initialized
}

/// Bridge SelectedVariables domain changes to global SELECTED_VARIABLES mutable
/// Will be implemented when SelectedVariables domain migration starts
#[allow(dead_code)]
fn initialize_selected_variables_bridge() {
    // TODO: Implement when SelectedVariables migration begins
    // SelectedVariables domain bridge - placeholder for future implementation
}

/// Cache for current cursor position using value caching pattern from chat_example.md
static CACHED_CURSOR_POSITION: OnceLock<Mutex<f64>> = OnceLock::new();

/// Cache for current viewport using value caching pattern
static CACHED_VIEWPORT: OnceLock<Mutex<Viewport>> = OnceLock::new();

/// Cache for current ns_per_pixel using value caching pattern
static CACHED_NS_PER_PIXEL: OnceLock<Mutex<NsPerPixel>> = OnceLock::new();

/// Cache for current timeline coordinates using value caching pattern
static CACHED_COORDINATES: OnceLock<Mutex<TimelineCoordinates>> = OnceLock::new();

/// Get cached cursor position (replacement for TIMELINE_CURSOR_NS.get())
pub fn get_cached_cursor_position_seconds() -> f64 {
    *CACHED_CURSOR_POSITION.get_or_init(|| Mutex::new(0.0)).lock().unwrap()
}

/// Get cached viewport (replacement for TIMELINE_VIEWPORT.get())
pub fn get_cached_viewport() -> Viewport {
    *CACHED_VIEWPORT.get_or_init(|| Mutex::new(
        Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(100.0))
    )).lock().unwrap()
}

/// Get cached ns_per_pixel (replacement for TIMELINE_NS_PER_PIXEL.get())
pub fn get_cached_ns_per_pixel() -> NsPerPixel {
    *CACHED_NS_PER_PIXEL.get_or_init(|| Mutex::new(NsPerPixel::MEDIUM_ZOOM)).lock().unwrap()
}

/// Get cached coordinates (replacement for TIMELINE_COORDINATES.get())
pub fn get_cached_coordinates() -> TimelineCoordinates {
    *CACHED_COORDINATES.get_or_init(|| Mutex::new(
        TimelineCoordinates::new(TimeNs::ZERO, TimeNs::ZERO, NsPerPixel::MEDIUM_ZOOM, 800)
    )).lock().unwrap()
}

/// Get cached cursor position as TimeNs (direct replacement for TIMELINE_CURSOR_NS.get())
pub fn get_cached_cursor_position() -> TimeNs {
    let seconds = *CACHED_CURSOR_POSITION.get_or_init(|| Mutex::new(0.0)).lock().unwrap();
    TimeNs::from_external_seconds(seconds)
}

/// Set cursor position through domain event (replacement for TIMELINE_CURSOR_NS.set())
pub fn set_cursor_position(time_ns: TimeNs) {
    let waveform_timeline = waveform_timeline_domain();
    let seconds = time_ns.display_seconds();
    waveform_timeline.cursor_dragged_relay.send(seconds);
}

/// Set cursor position through domain event (replacement for TIMELINE_CURSOR_NS.set_neq())
pub fn set_cursor_position_if_changed(time_ns: TimeNs) {
    let seconds = time_ns.display_seconds();
    let current_seconds = get_cached_cursor_position_seconds();
    
    // Only emit event if value actually changed
    if (seconds - current_seconds).abs() > f64::EPSILON {
        let waveform_timeline = waveform_timeline_domain();
        waveform_timeline.cursor_dragged_relay.send(seconds);
    }
}

/// Set cursor position from f64 seconds (convenience function)
pub fn set_cursor_position_seconds(seconds: f64) {
    let waveform_timeline = waveform_timeline_domain();
    waveform_timeline.cursor_dragged_relay.send(seconds);
}

// Static domain instance to avoid temporary value borrow issues
static WAVEFORM_TIMELINE_FOR_SIGNALS: OnceLock<crate::actors::WaveformTimeline> = OnceLock::new();

/// Get cursor position signal (helper to avoid temporary value borrow issues)  
pub fn cursor_position_signal() -> impl zoon::Signal<Item = f64> {
    WAVEFORM_TIMELINE_FOR_SIGNALS.get_or_init(|| waveform_timeline_domain()).cursor_position_signal()
}

/// Set viewport through domain event (replacement for TIMELINE_VIEWPORT.set())
pub fn set_viewport(viewport: Viewport) {
    let waveform_timeline = waveform_timeline_domain();
    let viewport_tuple = (viewport.start.display_seconds(), viewport.end.display_seconds());
    waveform_timeline.viewport_changed_relay.send(viewport_tuple);
}

/// Set viewport if changed (replacement for TIMELINE_VIEWPORT.set_neq())
pub fn set_viewport_if_changed(new_viewport: Viewport) {
    let current_viewport = get_cached_viewport();
    
    // Only emit event if value actually changed
    if current_viewport != new_viewport {
        let waveform_timeline = waveform_timeline_domain();
        let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        waveform_timeline.viewport_changed_relay.send(viewport_tuple);
    }
}

/// Get viewport signal (replacement for TIMELINE_VIEWPORT.signal())
pub fn viewport_signal() -> impl zoon::Signal<Item = Viewport> {
    WAVEFORM_TIMELINE_FOR_SIGNALS.get_or_init(|| waveform_timeline_domain()).viewport_signal()
}

/// Set ns_per_pixel through domain event (replacement for TIMELINE_NS_PER_PIXEL.set())
pub fn set_ns_per_pixel(ns_per_pixel: NsPerPixel) {
    let waveform_timeline = waveform_timeline_domain();
    // Convert NsPerPixel to zoom factor for the zoom_changed_relay
    let zoom_factor = 1_000_000.0 / (ns_per_pixel.nanos() as f32);
    waveform_timeline.zoom_changed_relay.send(zoom_factor);
}

/// Set ns_per_pixel if changed (replacement for TIMELINE_NS_PER_PIXEL.set_neq())
pub fn set_ns_per_pixel_if_changed(new_ns_per_pixel: NsPerPixel) {
    let current_ns_per_pixel = get_cached_ns_per_pixel();
    
    // Only emit event if value actually changed
    if current_ns_per_pixel != new_ns_per_pixel {
        let waveform_timeline = waveform_timeline_domain();
        let zoom_factor = 1_000_000.0 / (new_ns_per_pixel.nanos() as f32);
        waveform_timeline.zoom_changed_relay.send(zoom_factor);
    }
}

/// Get ns_per_pixel signal (replacement for TIMELINE_NS_PER_PIXEL.signal())
pub fn ns_per_pixel_signal() -> impl zoon::Signal<Item = NsPerPixel> {
    WAVEFORM_TIMELINE_FOR_SIGNALS.get_or_init(|| waveform_timeline_domain()).ns_per_pixel_signal()
}

/// Get coordinates signal (replacement for TIMELINE_COORDINATES.signal())
pub fn coordinates_signal() -> impl zoon::Signal<Item = TimelineCoordinates> {
    WAVEFORM_TIMELINE_FOR_SIGNALS.get_or_init(|| waveform_timeline_domain()).coordinates_signal()
}

/// Bridge WaveformTimeline domain changes to timeline-related global mutables
fn initialize_waveform_timeline_bridge() {
    // Value caching pattern: Cache cursor position as it flows through signals
    Task::start(async move {
        let waveform_timeline = waveform_timeline_domain();
        waveform_timeline.cursor_position_signal()
            .for_each(move |cursor_position| {
                // Cache the current value for synchronous access
                *CACHED_CURSOR_POSITION.get_or_init(|| Mutex::new(0.0)).lock().unwrap() = cursor_position;
                
                // Value cached for synchronous access
                
                async {}
            })
            .await;
    });
    
    // Value caching pattern: Cache viewport as it flows through signals
    Task::start(async move {
        let waveform_timeline = waveform_timeline_domain();
        waveform_timeline.viewport_signal()
            .for_each(move |viewport| {
                // Cache the current value for synchronous access
                *CACHED_VIEWPORT.get_or_init(|| Mutex::new(
                    Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(100.0))
                )).lock().unwrap() = viewport;
                
                // Value cached for synchronous access
                
                async {}
            })
            .await;
    });
    
    // Value caching pattern: Cache ns_per_pixel as it flows through signals
    Task::start(async move {
        let waveform_timeline = waveform_timeline_domain();
        waveform_timeline.ns_per_pixel_signal()
            .for_each(move |ns_per_pixel| {
                // Cache the current value for synchronous access
                *CACHED_NS_PER_PIXEL.get_or_init(|| Mutex::new(NsPerPixel::MEDIUM_ZOOM)).lock().unwrap() = ns_per_pixel;
                
                // Value cached for synchronous access
                
                async {}
            })
            .await;
    });
    
    // Value caching pattern: Cache coordinates as it flows through signals
    Task::start(async move {
        let waveform_timeline = waveform_timeline_domain();
        waveform_timeline.coordinates_signal()
            .for_each(move |coordinates| {
                // Cache the current value for synchronous access
                *CACHED_COORDINATES.get_or_init(|| Mutex::new(
                    TimelineCoordinates::new(TimeNs::ZERO, TimeNs::ZERO, NsPerPixel::MEDIUM_ZOOM, 800)
                )).lock().unwrap() = coordinates;
                
                // Value cached for synchronous access
                
                async {}
            })
            .await;
    });
    
    // WaveformTimeline domain bridge initialized - all timeline value caching active
}

/// Bridge UserConfiguration domain changes to config-related global mutables
/// Will be implemented when UserConfiguration domain migration starts
#[allow(dead_code)]
fn initialize_user_configuration_bridge() {
    // TODO: Implement when UserConfiguration migration begins
    // UserConfiguration domain bridge - placeholder for future implementation
}