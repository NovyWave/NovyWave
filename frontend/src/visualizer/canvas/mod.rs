// Canvas rendering and Fast2D integration

// Main waveform canvas implementation
pub mod waveform_canvas;

// Canvas sub-modules organized by functionality
pub mod animation;      // Smooth zoom, pan, and cursor movement functions
pub mod rendering;      // Canvas drawing, waveform rendering, and theme handling
pub mod timeline;       // Timeline range calculations and coordinate transformations
pub mod navigation;     // Transition jumping and reset functions

// Re-exports for API compatibility
pub use waveform_canvas::*;