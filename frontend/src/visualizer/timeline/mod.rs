//! Timeline Actor+Relay domain for waveform timeline management
//!
//! Domain-driven organization: Each module contains a complete domain object with its
//! types, business logic, and operations together.

// Time domain - complete time representation and calculations
pub mod time_domain;

// Main timeline actors domain - WaveformTimeline and related actors with operations
pub mod timeline_actors;

// MaximumTimelineRange domain - standalone derived state actor for range calculations
pub mod maximum_timeline_range;

// Timeline cache domain - signal data storage and cache management
pub mod timeline_cache;

// Cursor animation domain - smooth cursor movement animation control
pub mod cursor_animation;

// Panning controller domain - left/right viewport panning control
pub mod panning_controller;

// Canvas state domain - canvas dimensions and rendering state management
pub mod canvas_state;

// Zoom controller domain - zoom level management and ns_per_pixel calculations
pub mod zoom_controller;

// Re-exports for API compatibility from respective domains
pub use canvas_state::{CanvasStateController, TimelineStats};
pub use cursor_animation::CursorAnimationController;
pub use maximum_timeline_range::MaximumTimelineRange;
pub use panning_controller::PanningController;
pub use time_domain::{DurationNs, NsPerPixel, TimeNs, TimelineCoordinates, Viewport};
pub use timeline_actors::WaveformTimeline;
pub use timeline_cache::{
    CacheRequestState, CacheRequestType, TimelineCache, TimelineCacheController, ViewportSignalData,
};
pub use zoom_controller::ZoomController;

// Compatibility module alias for timeline_actor -> timeline_actors
pub mod timeline_actor {
    pub use super::timeline_actors::*;
    // Re-export types from time_domain for compatibility
    pub use super::time_domain::{DurationNs, NsPerPixel, TimeNs, TimelineCoordinates, Viewport};
}
