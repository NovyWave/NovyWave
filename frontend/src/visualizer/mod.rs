// Visualizer Module - Canvas and Timeline Rendering
//
// This module contains all code related to canvas rendering and timeline visualization,
// organized into logical sub-modules for maintainability.

// Canvas rendering and Fast2D integration
pub mod canvas;

// Timeline data and state management
pub mod timeline;

// User interaction handling (keyboard, mouse, drag)
pub mod interaction;

// Value and display formatting
pub mod formatting;

// UI components and integration
pub mod ui;

// Testing utilities and patterns
pub mod testing;

// Re-export commonly used types for convenience  
// pub use timeline::time_types::*;        // Unused re-export
// pub use timeline::timeline_service::*;  // Unused re-export
// pub use formatting::signal_values::*;   // Unused re-export
// pub use canvas::waveform_canvas::*;     // Unused re-export
// pub use interaction::dragging::*;       // Unused re-export