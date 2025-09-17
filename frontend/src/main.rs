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
mod selected_variables_panel;
mod signal_processing;
mod variable_selection_ui;

/// Main application layout function
///
/// Implements dock-responsive 3-panel layout as specified:
/// - Default (dock to bottom): Files & Scopes + Variables (top row), Selected Variables (bottom)
/// - Dock to right: Files & Scopes over Variables (left column), Selected Variables (right)
pub fn main_layout(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
    dragging_system: &crate::dragging::DraggingSystem,
    waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
    file_dialog_visible: &crate::dataflow::atom::Atom<bool>,
) -> impl Element {
    use moonzoon_novyui::*;
    use crate::file_management::files_panel_with_dialog;
    use crate::variable_selection_ui::{selected_variables_with_waveform_panel, variables_panel_with_fill};

    El::new().s(Width::fill()).s(Height::fill()).child_signal(
        app_config.dock_mode_actor.signal().map({
            let tracked_files = tracked_files.clone();
            let selected_variables = selected_variables.clone();
            let waveform_timeline = waveform_timeline.clone();
            let app_config = app_config.clone();
            let dragging_system = dragging_system.clone();
            let waveform_canvas = waveform_canvas.clone();
            let file_dialog_visible = file_dialog_visible.clone();

            move |dock_mode| {
                match dock_mode {
                    // Default layout: Files & Variables (top row), Selected Variables (bottom)
                    shared::DockMode::Bottom => {
                        El::new()
                            .s(Width::fill())
                            .s(Height::fill())
                            .child(
                                Column::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .item(
                                        Row::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .item(files_panel_with_dialog(
                                                tracked_files.clone(),
                                                selected_variables.clone(),
                                                file_dialog_visible.clone(),
                                                app_config.clone(),
                                            ))
                                            .item(crate::panel_layout::files_panel_vertical_divider(&app_config, dragging_system.clone()))
                                            .item(variables_panel_with_fill(
                                                &tracked_files,
                                                &selected_variables,
                                                &waveform_timeline,
                                                &waveform_canvas,
                                                &app_config,
                                            ))
                                    )
                                    .item(crate::panel_layout::files_panel_horizontal_divider(&app_config, dragging_system.clone()))
                                    .item(crate::selected_variables_panel::selected_variables_panel(
                                        selected_variables.clone(),
                                        waveform_timeline.clone(),
                                        tracked_files.clone(),
                                        app_config.clone(),
                                        dragging_system.clone(),
                                        waveform_canvas.clone(),
                                    ))
                            )
                    }
                    
                    // Right dock layout: Files over Variables (left), Selected Variables (right)
                    shared::DockMode::Right => {
                        El::new()
                            .s(Width::fill())
                            .s(Height::fill())
                            .child(
                                Row::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .item(
                                        Column::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .item(files_panel_with_dialog(
                                                tracked_files.clone(),
                                                selected_variables.clone(),
                                                file_dialog_visible.clone(),
                                                app_config.clone(),
                                            ))
                                            .item(crate::panel_layout::files_panel_horizontal_divider(&app_config, dragging_system.clone()))
                                            .item(variables_panel_with_fill(
                                                &tracked_files,
                                                &selected_variables,
                                                &waveform_timeline,
                                                &waveform_canvas,
                                                &app_config,
                                            ))
                                    )
                                    .item(crate::panel_layout::files_panel_vertical_divider(&app_config, dragging_system.clone()))
                                    .item(crate::selected_variables_panel::selected_variables_panel(
                                        selected_variables.clone(),
                                        waveform_timeline.clone(),
                                        tracked_files.clone(),
                                        app_config.clone(),
                                        dragging_system.clone(),
                                        waveform_canvas.clone(),
                                    ))
                            )
                    }
                }
            }
        })
    )
}

pub fn main() {
    Task::start(async {
        let app = crate::app::NovyWaveApp::new().await;
        let root_element = app.root();
        start_app("app", move || root_element);
    });
}

// Rebuild trigger
