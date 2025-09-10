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
mod virtual_list;
mod visualizer;

mod action_buttons;
mod dragging;
mod file_management;
mod file_operations;
mod file_picker;
mod format_selection;
mod panel_layout;
mod signal_processing;
mod variable_selection_ui;

/// Main application layout function
/// 
/// This is the root layout that orchestrates the main panels:
/// - Files panel for waveform file management
/// - Variables panel with integrated waveform visualization
pub fn main_layout(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
    dragging_system: &crate::dragging::DraggingSystem,
    waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
) -> impl Element {
    use moonzoon_novyui::*;
    use crate::file_management::files_panel;
    use crate::variable_selection_ui::selected_variables_with_waveform_panel;
    
    El::new().s(Width::fill()).s(Height::fill()).child(
        Column::new()
            .s(Width::fill())
            .s(Height::fill())
            .item(files_panel(
                tracked_files.clone(), 
                selected_variables.clone(),
                button().label("Load Files").disabled(true).build() // Placeholder - no file_dialog_visible access
            ))
            .item(selected_variables_with_waveform_panel(
                selected_variables.clone(),
                waveform_timeline.clone(),
                tracked_files.clone(),
                app_config.clone(),
                dragging_system.clone(),
                waveform_canvas.clone(),
            )),
    )
}

pub fn main() {
    Task::start(async {
        let app = crate::app::NovyWaveApp::new().await;
        let root_element = app.root();
        start_app("app", move || root_element);
    });
}

