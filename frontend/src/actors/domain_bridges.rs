//! Domain Bridges for Actor+Relay Migration
//!
//! Provides synchronization between new Actor+Relay domains and legacy global mutables
//! during the transition period. These bridges will be removed once migration is complete.

use crate::actors::{tracked_files_domain};
use crate::actors::waveform_timeline::{initialize_waveform_timeline, cursor_position_seconds_signal, cursor_moved_relay};
use crate::state::TRACKED_FILES;
use crate::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates};
use zoon::{Task, SignalExt};
use std::sync::{Mutex, OnceLock};

/// Initialize bridges between Actor+Relay domains and legacy global state
/// This ensures backward compatibility during the migration period
pub async fn initialize_domain_bridges() {
    initialize_tracked_files_bridge();
    initialize_waveform_timeline().await;
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

/// Cache for current canvas dimensions
static CACHED_CANVAS_WIDTH: OnceLock<Mutex<f32>> = OnceLock::new();
static CACHED_CANVAS_HEIGHT: OnceLock<Mutex<f32>> = OnceLock::new();

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

// === CANVAS DIMENSION ACCESS ===

/// Get cached canvas width (replacement for CANVAS_WIDTH.get())
pub fn get_canvas_width() -> f32 {
    *CACHED_CANVAS_WIDTH.get_or_init(|| Mutex::new(800.0)).lock().unwrap()
}

/// Get cached canvas height (replacement for CANVAS_HEIGHT.get())
pub fn get_canvas_height() -> f32 {
    *CACHED_CANVAS_HEIGHT.get_or_init(|| Mutex::new(400.0)).lock().unwrap()
}

/// Set canvas dimensions through WaveformTimeline actor
pub fn set_canvas_dimensions(width: f32, height: f32) {
    use crate::actors::waveform_timeline::canvas_resized_relay;
    canvas_resized_relay().send((width, height));
}

/// Get cached cursor position as TimeNs (direct replacement for TIMELINE_CURSOR_NS.get())
pub fn get_cached_cursor_position() -> TimeNs {
    let seconds = *CACHED_CURSOR_POSITION.get_or_init(|| Mutex::new(0.0)).lock().unwrap();
    TimeNs::from_external_seconds(seconds)
}

/// Set cursor position through domain event (replacement for TIMELINE_CURSOR_NS.set())
pub fn set_cursor_position(time_ns: TimeNs) {
    cursor_moved_relay().send(time_ns);
}

/// Set cursor position through domain event (replacement for TIMELINE_CURSOR_NS.set_neq())
pub fn set_cursor_position_if_changed(time_ns: TimeNs) {
    let current_ns = get_cached_cursor_position();
    
    // Only emit event if value actually changed
    if current_ns != time_ns {
        cursor_moved_relay().send(time_ns);
    }
}

/// Set cursor position from f64 seconds (convenience function)
pub fn set_cursor_position_seconds(seconds: f64) {
    let time_ns = TimeNs::from_external_seconds(seconds);
    cursor_moved_relay().send(time_ns);
}

// Removed old static domain instance - now using direct function calls

/// Get cursor position signal (helper to avoid temporary value borrow issues)  
pub fn cursor_position_signal() -> impl zoon::Signal<Item = f64> {
    cursor_position_seconds_signal()
}

/// Set viewport through domain event (replacement for TIMELINE_VIEWPORT.set())
pub fn _set_viewport(viewport: Viewport) {
    use crate::actors::waveform_timeline::viewport_changed_relay;
    let viewport_tuple = (viewport.start.display_seconds(), viewport.end.display_seconds());
    viewport_changed_relay().send(viewport_tuple);
}

/// Set viewport if changed (replacement for TIMELINE_VIEWPORT.set_neq())
pub fn set_viewport_if_changed(new_viewport: Viewport) {
    let current_viewport = get_cached_viewport();
    
    // Only emit event if value actually changed
    if current_viewport != new_viewport {
        use crate::actors::waveform_timeline::viewport_changed_relay;
        let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        viewport_changed_relay().send(viewport_tuple);
    }
}

/// Get viewport signal (replacement for TIMELINE_VIEWPORT.signal())
pub fn viewport_signal_bridge() -> impl zoon::Signal<Item = Viewport> {
    viewport_signal()
}

/// Set ns_per_pixel through domain event (replacement for TIMELINE_NS_PER_PIXEL.set())
pub fn _set_ns_per_pixel(_ns_per_pixel: NsPerPixel) {
    use crate::actors::waveform_timeline::zoom_in_started_relay;
    // Use the timeline position for zoom center - simplified for now
    let zoom_center = get_cached_cursor_position();
    zoom_in_started_relay().send(zoom_center);
}

/// Set ns_per_pixel if changed (replacement for TIMELINE_NS_PER_PIXEL.set_neq())
pub fn set_ns_per_pixel_if_changed(new_ns_per_pixel: NsPerPixel) {
    let current_ns_per_pixel = get_cached_ns_per_pixel();
    
    // Only emit event if value actually changed
    if current_ns_per_pixel != new_ns_per_pixel {
        use crate::actors::waveform_timeline::zoom_in_started_relay;
        let zoom_center = get_cached_cursor_position();
        zoom_in_started_relay().send(zoom_center);
    }
}

/// Get ns_per_pixel signal (replacement for TIMELINE_NS_PER_PIXEL.signal())
pub fn ns_per_pixel_signal_bridge() -> impl zoon::Signal<Item = NsPerPixel> {
    ns_per_pixel_signal()
}

/// Get coordinates signal (replacement for TIMELINE_COORDINATES.signal())
#[allow(dead_code)]
pub fn coordinates_signal_bridge() -> impl zoon::Signal<Item = TimelineCoordinates> {
    coordinates_signal()
}

/// Bridge WaveformTimeline domain changes to timeline-related global mutables
fn initialize_waveform_timeline_bridge() {
    // Value caching pattern: Cache cursor position as it flows through signals
    Task::start(async move {
        cursor_position_seconds_signal()
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
        viewport_signal()
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
        ns_per_pixel_signal()
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
        coordinates_signal()
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
    
    // Value caching pattern: Cache canvas dimensions as they flow through signals
    Task::start(async move {
        use crate::actors::waveform_timeline::canvas_width_signal;
        canvas_width_signal()
            .for_each(move |width| {
                // Cache the current value for synchronous access
                *CACHED_CANVAS_WIDTH.get_or_init(|| Mutex::new(800.0)).lock().unwrap() = width;
                
                // Value cached for synchronous access
                
                async {}
            })
            .await;
    });
    
    Task::start(async move {
        use crate::actors::waveform_timeline::canvas_height_signal;
        canvas_height_signal()
            .for_each(move |height| {
                // Cache the current value for synchronous access
                *CACHED_CANVAS_HEIGHT.get_or_init(|| Mutex::new(400.0)).lock().unwrap() = height;
                
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

// === PUBLIC RE-EXPORTS FOR LEGACY COMPATIBILITY ===

/// Re-export viewport signal for legacy imports
pub fn viewport_signal() -> impl zoon::Signal<Item = Viewport> {
    crate::actors::waveform_timeline::viewport_signal()
}

/// Re-export ns_per_pixel signal for legacy imports  
pub fn ns_per_pixel_signal() -> impl zoon::Signal<Item = NsPerPixel> {
    crate::actors::waveform_timeline::ns_per_pixel_signal()
}

/// Re-export coordinates signal for legacy imports
pub fn coordinates_signal() -> impl zoon::Signal<Item = TimelineCoordinates> {
    crate::actors::waveform_timeline::coordinates_signal()
}