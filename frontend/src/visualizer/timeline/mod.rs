// Timeline data and state management

// Timeline Actor+Relay domain (moved from actors/waveform_timeline.rs)
pub mod timeline_actor;

// Core time types and coordinates (will be moved from time_types.rs)
pub mod time_types;

// Timeline utility functions for calculations and formatting
pub mod time_utils;

// Re-exports for API compatibility
// pub use timeline_actor::WaveformTimeline; // Unused re-export

// Removed - use NovyWaveApp.waveform_timeline directly

// Future coordinate calculation utilities
// pub mod coordinates;
