// Canvas rendering and Fast2D integration

// Main waveform canvas implementation (moved from waveform_canvas.rs)
pub mod waveform_canvas;

// Re-exports for API compatibility
pub use waveform_canvas::*;

// Future sub-modules (will be split from waveform_canvas.rs if needed)
// pub mod animation;
// pub mod mouse_handling;
// pub mod rendering;