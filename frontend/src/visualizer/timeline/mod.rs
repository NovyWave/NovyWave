// Timeline data and state management

// Timeline Actor+Relay domain (moved from actors/waveform_timeline.rs)
pub mod timeline_actor;

// Core time types and coordinates (will be moved from time_types.rs)
pub mod time_types;

// Timeline data service and caching (moved from unified_timeline_service.rs)
pub mod timeline_service;

// Re-exports for API compatibility
// pub use timeline_actor::WaveformTimeline; // Unused re-export

// Re-export the global domain function with compatible name
pub fn timeline_actor_domain() -> timeline_actor::WaveformTimeline {
    crate::actors::global_domains::waveform_timeline_domain()
}

// Future coordinate calculation utilities
// pub mod coordinates;