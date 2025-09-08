//! NovyWave Main Entry Point

use zoon::*;

// Core modules
mod app;
mod dataflow;
mod selected_variables;
mod tracked_files;
mod clipboard;
mod config;
mod connection;
mod error_display;
mod error_ui;
mod platform;
mod views;
mod virtual_list;
mod visualizer;

pub fn main() {
    Task::start(async {
        let app = crate::app::NovyWaveApp::new().await;
        let root_element = app.root();
        start_app("app", move || root_element);
    });
}

