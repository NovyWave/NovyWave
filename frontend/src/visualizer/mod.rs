// Visualizer Module - Canvas and Timeline Rendering
//
// This module contains all code related to canvas rendering and timeline visualization,
// organized into logical sub-modules for maintainability.

// Canvas rendering and Fast2D integration
pub mod canvas;

// Timeline data and state management
pub mod timeline;

// User interaction handling moved to frontend root (dragging.rs)

// Testing utilities and patterns
pub mod testing;

// Re-export commonly used types for convenience  
// All timeline types now integrated directly in timeline_actor domain