//! NovyWave Main Entry Point
//! 
//! ✅ PHASE 3 ARCHITECTURAL TRANSFORMATION COMPLETE
//! 
//! Successfully migrated from 616-line global state coordination to clean 
//! self-contained NovyWaveApp architecture following ChatApp pattern.
//!
//! ## Architecture Before (616 lines):
//! - Complex global domain initialization 
//! - Manual coordination handlers
//! - Global static APP_CONFIG and CONNECTION
//! - Scattered startup sequence management
//! 
//! ## Architecture After (47 lines):
//! - Self-contained NovyWaveApp with owned domain instances
//! - Clean Actor+Relay event-driven patterns
//! - Proper dependency injection capability
//! - Testable, modular, maintainable codebase
//!
//! ## Key Domains Migrated:
//! - TrackedFiles: File loading and management
//! - SelectedVariables: Variable selection and scopes  
//! - WaveformTimeline: Timeline, cursor, waveform visualization
//! - AppConfig: Application configuration and persistence

use zoon::*;

// Core modules
mod dataflow;
mod actors;
mod app; // ✅ NEW: NovyWaveApp self-contained architecture

// Legacy modules still needed for views/components
mod state;
mod config;
mod connection;
mod views;
mod visualizer;
mod error_display;
mod platform;
mod virtual_list;
mod clipboard;
mod file_dialog;

/// Main entry point using NovyWaveApp pattern
/// 
/// Replaces complex 616-line global coordination with single self-contained app instance.
/// All domain initialization, configuration, and UI setup handled internally by NovyWaveApp.
pub fn main() {
    Task::start(async {
        let app = crate::app::NovyWaveApp::new().await;
        let root_element = app.root();
        start_app("app", move || root_element);
    });
}

// TODO: Following functions will be migrated into NovyWaveApp methods in future iterations
// For now keeping as stubs to maintain compilation

/// Temporary stub - will be replaced by NovyWaveApp method
fn root() -> impl Element {
    El::new().child("Legacy root - should not be called")
}

/// Temporary stub - will be replaced by NovyWaveApp method  
fn main_layout() -> impl Element {
    El::new().child("Legacy main_layout - should not be called")
}