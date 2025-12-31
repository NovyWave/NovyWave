/*!
 * Selected Variables Panel Implementation
 *
 * Comprehensive three-column Selected Variables Panel as specified:
 * - Header: Title, Theme button, Dock toggle, Remove All button
 * - Name Column: Variable names with remove buttons, keyboard shortcuts footer
 * - Value Column: Format dropdowns with values, timeline boundaries footer
 * - Wave Column: Fast2D canvas for waveform visualization
 */

use crate::dragging::{variables_name_column_width_signal, variables_value_column_width_signal};
use crate::visualizer::timeline::TimePerPixel;
use moonzoon_novyui::components::{KbdSize, KbdVariant, kbd};
use moonzoon_novyui::tokens::color::{neutral_8, neutral_11, primary_6};
use moonzoon_novyui::*;
use shared::{SelectedVariable, TrackedFile, VarFormat};
use zoon::*;

/// Selected Variables panel row height constant
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

/// Enhanced Selected Variables Panel with proper three-column layout
pub fn selected_variables_panel(
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    tracked_files: crate::tracked_files::TrackedFiles,
    app_config: crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
    waveform_canvas: crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
) -> impl Element {
    let selected_variables_for_header = selected_variables.clone();
    let app_config_for_header = app_config.clone();

    Column::new()
        .s(Width::fill())
        .s(Height::fill())
        .item(crate::panel_layout::create_panel(
            // Header with title and action buttons
            selected_variables_panel_header(&selected_variables_for_header, &app_config_for_header),
            // Three-column content area
            selected_variables_panel_content(
                selected_variables,
                waveform_timeline,
                tracked_files,
                app_config,
                dragging_system,
                waveform_canvas,
            ),
        ))
}

/// Panel header with title and action buttons
fn selected_variables_panel_header(
    selected_variables: &crate::selected_variables::SelectedVariables,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    Row::new()
        .s(Gap::new().x(SPACING_8))
        .s(Align::new().center_y())
        .s(Width::fill())
        .item(
            // Title - left aligned
            El::new()
                .s(Font::new().no_wrap())
                .child("Selected Variables"),
        )
        .item(
            // Center section with version and dock button
            Row::new()
                .s(Width::fill())
                .s(Align::new().center_y())
                .s(Gap::new().x(SPACING_8))
                .item(El::new().s(Width::growable()))
                .item(
                    // Version display with less contrast
                    El::new()
                        .s(Font::new().no_wrap().color_signal(neutral_8()))
                        .child_signal(
                            signal::from_future(Box::pin(crate::platform::get_app_version()))
                                .map(|version| match version {
                                    Some(v) => format!("v{}", v),
                                    None => "...".to_string(),
                                }),
                        ),
                )
                .item(crate::action_buttons::dock_toggle_button(app_config))
                .item(El::new().s(Width::growable())),
        )
        .item(crate::action_buttons::clear_all_variables_button(
            selected_variables,
        ))
}

/// Three-column content area with proper layout
fn selected_variables_panel_content(
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    tracked_files: crate::tracked_files::TrackedFiles,
    app_config: crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
    waveform_canvas: crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
) -> impl Element {
    let selected_variables_for_height = selected_variables.clone();
    let _name_column_width_signal = variables_name_column_width_signal(app_config.clone());
    let _value_column_width_signal = variables_value_column_width_signal(app_config.clone());

    El::new()
        .s(Height::exact_signal(
            selected_variables_for_height
                .variables
                .signal_vec()
                .to_signal_cloned()
                .map(|vars| {
                    let rows = if vars.is_empty() { 2 } else { vars.len() + 1 };
                    (rows as u32) * SELECTED_VARIABLES_ROW_HEIGHT
                }),
        ))
        .s(Width::fill())
        .s(Scrollbars::x_and_clip_y())
        .child_signal({
            let selected_variables = selected_variables.clone();
            let tracked_files = tracked_files.clone();
            let app_config = app_config.clone();
            let dragging_system = dragging_system.clone();
            let waveform_timeline = waveform_timeline.clone();
            let waveform_canvas = waveform_canvas.clone();
            selected_variables_for_height
                .variables
                .signal_vec()
                .to_signal_cloned()
                .map(move |vars| {
                    if vars.is_empty() {
                        crate::file_management::empty_state_hint(
                            "Select variables in the Variables panel to show them here.",
                        )
                        .into_raw()
                    } else {
                        // Recompute width signals inside the branch to avoid moving non-Copy signals into the closure
                        let name_signal = variables_name_column_width_signal(app_config.clone());
                        let value_signal = variables_value_column_width_signal(app_config.clone());
                        zoon::RawElOrText::RawHtmlEl(
                            Row::new()
                                .s(Height::fill())
                                .s(Width::fill())
                                .s(Align::new().top())
                                .item(selected_variables_name_column(
                                    selected_variables.clone(),
                                    tracked_files.clone(),
                                    waveform_timeline.clone(),
                                    name_signal,
                                ))
                                .item(crate::panel_layout::variables_name_vertical_divider(
                                    &app_config,
                                    dragging_system.clone(),
                                ))
                                .item(selected_variables_value_column(
                                    selected_variables.clone(),
                                    waveform_timeline.clone(),
                                    app_config.clone(),
                                    value_signal,
                                ))
                                .item(crate::panel_layout::variables_value_vertical_divider(
                                    &app_config,
                                    dragging_system.clone(),
                                ))
                                .item(selected_variables_wave_column(
                                    &selected_variables,
                                    &waveform_timeline,
                                    &waveform_canvas,
                                    &app_config,
                                ))
                                .into_raw_el(),
                        )
                    }
                })
        })
}

/// Name Column with remove buttons and keyboard shortcuts footer
fn selected_variables_name_column(
    selected_variables: crate::selected_variables::SelectedVariables,
    tracked_files: crate::tracked_files::TrackedFiles,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    width_signal: impl Signal<Item = f32> + Unpin + 'static,
) -> impl Element {
    let selected_variables_for_items = selected_variables.clone();

    Column::new()
        .s(Width::exact_signal(width_signal.map(|w| w as u32)))
        .s(Height::fill())
        .s(Align::new().top())
        .s(Scrollbars::x_and_clip_y())
        .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
        .items_signal_vec({
            let tracked_files_for_items = tracked_files.clone();
            selected_variables
                .variables
                .signal_vec()
                .map(move |selected_var| {
                    name_column_variable_row(
                        selected_var,
                        selected_variables_for_items.clone(),
                        tracked_files_for_items.clone(),
                    )
                })
        })
        .item(
            // Name Column Footer with keyboard shortcuts
            name_column_footer(waveform_timeline),
        )
}

/// Individual variable row in Name Column
fn name_column_variable_row(
    selected_var: SelectedVariable,
    selected_variables: crate::selected_variables::SelectedVariables,
    tracked_files: crate::tracked_files::TrackedFiles,
) -> impl Element {
    let unique_id = selected_var.unique_id.clone();
    let selected_variables_for_remove = selected_variables.clone();
    let tracked_files_broadcaster = tracked_files
        .files
        .signal_vec()
        .to_signal_cloned()
        .broadcast();

    Row::new()
        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
        .s(Width::fill())
        .s(Padding::new().x(SPACING_2).y(SPACING_4))
        .s(Gap::new().x(SPACING_4))
        .item(
            // Remove button
            button()
                .left_icon(IconName::X)
                .variant(ButtonVariant::DestructiveGhost)
                .size(ButtonSize::Small)
                .custom_padding(2, 2)
                .on_press({
                    let remove_relay = selected_variables_for_remove.variable_removed_relay.clone();
                    move || {
                        remove_relay.send(unique_id.clone());
                    }
                })
                .build()
        )
        .item(
            // Variable name and type
            Row::new()
                .s(Gap::new().x(SPACING_8))
                .s(Width::fill())
                .item(
                    // Variable name
                    El::new()
                        .s(Font::new().color_signal(neutral_11()).size(13).no_wrap())
                        .s(Width::growable())
                        .update_raw_el(|raw_el| {
                            raw_el.style("white-space", "nowrap")
                        })
                        .child(&selected_var.variable_name().unwrap_or_default())
                )
                .item(
                    // Variable type (right-aligned)
                    El::new()
                        .s(Font::new().color_signal(primary_6()).size(11).no_wrap())
                        .s(Align::new().right())
                        .s(Padding::new().right(8))
                        .update_raw_el(|raw_el| {
                            raw_el
                                .style("text-overflow", "ellipsis")
                                .style("max-width", "100%")
                        })
                        .child_signal({
                            let selected_var = selected_var.clone();
                            tracked_files_broadcaster.signal_cloned().map(move |files: Vec<TrackedFile>| {
                                crate::signal_processing::get_signal_type_for_selected_variable_from_files(&selected_var, &files)
                            })
                        })
                )
                .update_raw_el({
                    let selected_var = selected_var.clone();
                    let tracked_files_broadcaster = tracked_files_broadcaster.clone();
                    move |raw_el| {
                        // Tooltip with full variable information
                        let title_signal = tracked_files_broadcaster.signal_cloned().map({
                            let selected_var = selected_var.clone();
                            move |files: Vec<TrackedFile>| {
                                let signal_type = crate::signal_processing::get_signal_type_for_selected_variable_from_files(&selected_var, &files);
                                format!("{} - {} - {}",
                                    selected_var.file_path().unwrap_or_default(),
                                    selected_var.scope_path().unwrap_or_default(),
                                    signal_type
                                )
                            }
                        });
                        raw_el.attr_signal("title", title_signal)
                    }
                })
        )
}

/// Name Column Footer with keyboard shortcuts for zoom
fn name_column_footer(
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
) -> impl Element {
    El::new()
        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
        .s(Width::fill())
        .s(Padding::all(1))
        .s(Font::new().color_signal(neutral_8()).size(12).center())
        .s(Transform::new().move_up(4))
        .child(
            Row::new()
                // .s(Align::new().center_y())
                .item(
                    // Z key - Reset zoom center
                    kbd("Z")
                        .size(KbdSize::Small)
                        .variant(KbdVariant::Outlined)
                        .title("Press Z to move zoom center to 0")
                        .build()
                )
                .item(El::new().s(Width::fill()))
                .item(
                    // Zoom controls section
                    Row::new()
                        .s(Align::center())
                        .s(Gap::new().x(SPACING_6))
                        .item(
                            // W key - Zoom in
                            kbd("W")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press W to zoom in. Press Shift+W to zoom in faster.")
                                .build()
                        )
                        .item(
                            El::new()
                                .update_raw_el(|raw_el| {
                                    raw_el
                                        .style("min-width", "45px")
                                        .style("width", "fit-content")
                                        .style("max-width", "80px")
                                })
                                .s(Font::new().color_signal(neutral_11()).center())
                                .child_signal({
                                    let viewport_actor = waveform_timeline.viewport_actor();
                                    let width_actor = waveform_timeline.canvas_width_actor();
                                    map_ref! {
                                        let viewport = viewport_actor.signal(),
                                        let width = width_actor.signal() => {
                                            let range_ps = viewport.duration().picoseconds();
                                            let width_px = width.max(1.0).round().max(1.0) as u32;
                                            TimePerPixel::formatted_from_duration_and_width(range_ps, width_px)
                                        }
                                    }
                                    .map(Text::new)
                                })
                        )
                        .item(
                            // S key - Zoom out
                            kbd("S")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press S to zoom out. Press Shift+S to zoom out faster.")
                                .build()
                        )
                )
                .item(El::new().s(Width::fill()))
                .item(
                    // R key - Reset zoom, T key - Toggle tooltip visibility
                    Row::new()
                        .s(Align::center())
                        .s(Gap::new().x(SPACING_4))
                        .item(
                            El::new()
                                .on_click({
                                    let relay = waveform_timeline.reset_zoom_pressed_relay.clone();
                                    move || {
                                        relay.send(());
                                    }
                                })
                                .child(
                                    kbd("R")
                                        .size(KbdSize::Small)
                                        .variant(KbdVariant::Outlined)
                                        .title("Press R to reset to default zoom center, zoom and cursor position.")
                                        .build()
                                )
                        )
                        .item(
                            kbd("T")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press T to toggle waveform tooltip visibility")
                                .build()
                        )
                )
        )
}

/// Value Column with format dropdowns and timeline footer
fn selected_variables_value_column(
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: crate::config::AppConfig,
    width_signal: impl Signal<Item = f32> + Unpin + 'static,
) -> impl Element {
    let selected_variables_for_values = selected_variables.clone();
    let waveform_timeline_for_values = waveform_timeline.clone();
    let app_config_for_values = app_config.clone();

    Column::new()
        .s(Width::exact_signal(width_signal.map(|w| w as u32)))
        .s(Height::fill())
        .s(Align::new().top())
        .s(Scrollbars::x_and_clip_y())
        .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
        .items_signal_vec({
            let timeline_for_rows = waveform_timeline_for_values.clone();
            selected_variables
                .variables
                .signal_vec()
                .map(move |selected_var| {
                    value_column_variable_row(
                        selected_var,
                        selected_variables_for_values.clone(),
                        timeline_for_rows.clone(),
                        app_config_for_values.clone(),
                    )
                })
        })
        .item(value_column_footer(waveform_timeline_for_values))
}

/// Individual variable row in Value Column with format dropdown
fn value_column_variable_row(
    selected_var: SelectedVariable,
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: crate::config::AppConfig,
) -> impl Element {
    El::new()
        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
        .s(Width::fill())
        .child(crate::format_selection::create_format_dropdown(
            &selected_var.unique_id,
            selected_var.formatter.unwrap_or(VarFormat::Hexadecimal),
            &selected_variables,
            &waveform_timeline,
            app_config,
        ))
}

/// Value Column Footer with timeline boundaries and cursor controls
fn value_column_footer(
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
) -> impl Element {
    let viewport_actor = waveform_timeline.viewport_actor();
    let start_signal = viewport_actor.clone().signal().map(|viewport| {
        format_time_with_range(viewport.start.nanos(), viewport.duration().nanos())
    });

    let end_signal = viewport_actor
        .signal()
        .map(|viewport| format_time_with_range(viewport.end.nanos(), viewport.duration().nanos()));

    let cursor_signal = {
        let viewport_actor = viewport_actor.clone();
        map_ref! {
            let cursor = waveform_timeline.cursor_actor().signal(),
            let viewport = viewport_actor.signal() => {
                format_time_with_range(cursor.nanos(), viewport.duration().nanos())
            }
        }
    };

    El::new()
        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
        .s(Width::fill())
        .s(Padding::all(1))
        .s(Transform::new().move_up(4))
        .child(
            Row::new()
                .s(Align::new().center_y())
                .s(Font::new().color_signal(neutral_8()).size(12))
                .item(
                    // Left boundary and pan left controls
                    Row::new()
                        .s(Gap::new().x(SPACING_6))
                        .item(
                            // Timeline start boundary (0s)
                            El::new()
                                .s(Font::new().color_signal(neutral_11()).center().size(11))
                                .update_raw_el(|raw_el| {
                                    raw_el.style("width", "max-content")
                                })
                                .child_signal(start_signal.map(Text::new))
                        )
                        .item(
                            // A key - Pan left
                            kbd("A")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press A to pan left. Press Shift+A to pan left faster.")
                                .build()
                        )
                )
                .item(El::new().s(Width::fill()))
                .item(
                    // Cursor controls section
                    Row::new()
                        .s(Gap::new().x(SPACING_2))
                        .item(
                            // Q key - Move cursor left
                            kbd("Q")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press Q to move cursor left. Press Shift+Q to jump to the previous transition.")
                                .build()
                        )
                        .item(
                            El::new()
                                .update_raw_el(|raw_el| {
                                    raw_el
                                        .style("min-width", "45px")
                                        .style("width", "fit-content")
                                        .style("max-width", "90px")
                                })
                                .s(Font::new().color_signal(neutral_11()).center())
                                .child_signal(cursor_signal.map(Text::new))
                        )
                        .item(
                            // E key - Move cursor right
                            kbd("E")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press E to move cursor right. Press Shift+E to jump to the next transition.")
                                .build()
                        )
                )
                .item(El::new().s(Width::fill()))
                .item(
                    // Right boundary and pan right controls
                    Row::new()
                        .s(Gap::new().x(SPACING_6))
                        .item(
                            // D key - Pan right
                            kbd("D")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Press D to pan right. Press Shift+D to pan right faster.")
                                .build()
                        )
                        .item(
                            El::new()
                                .s(Font::new().color_signal(neutral_11()).center().size(11))
                                .update_raw_el(|raw_el| {
                                    raw_el.style("width", "max-content")
                                })
                                .child_signal(end_signal.map(Text::new))
                        )
                )
        )
}

/// Wave Column with Fast2D canvas integration
fn selected_variables_wave_column(
    _selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
    _app_config: &crate::config::AppConfig,
) -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_2()))
        .child(crate::visualizer::canvas::waveform_canvas::waveform_canvas(
            waveform_canvas,
            waveform_timeline,
        ))
}

fn format_time_with_range(ns: u64, range_ns: u64) -> String {
    let unit = TimeDisplayUnit::from_range(range_ns);
    let value = ns as f64 / unit.base_ns();
    let mut formatted = format_axis_number(value);
    formatted.push_str(unit.suffix());
    formatted
}

fn format_axis_number(value: f64) -> String {
    let mut s = if value.abs() >= 100.0 {
        format!("{:.0}", value.round())
    } else if value.abs() >= 10.0 {
        format!("{:.1}", value)
    } else if value.abs() >= 1.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.3}", value)
    };

    if let Some(pos) = s.find('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.len() > pos && s.ends_with('.') {
            s.pop();
        }
    }

    if s.is_empty() { "0".to_string() } else { s }
}

#[derive(Clone, Copy, Debug)]
enum TimeDisplayUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl TimeDisplayUnit {
    fn from_range(range_ns: u64) -> Self {
        if range_ns >= 1_000_000_000 {
            TimeDisplayUnit::Seconds
        } else if range_ns >= 1_000_000 {
            TimeDisplayUnit::Milliseconds
        } else if range_ns >= 1_000 {
            TimeDisplayUnit::Microseconds
        } else {
            TimeDisplayUnit::Nanoseconds
        }
    }

    fn base_ns(self) -> f64 {
        match self {
            TimeDisplayUnit::Seconds => 1_000_000_000.0,
            TimeDisplayUnit::Milliseconds => 1_000_000.0,
            TimeDisplayUnit::Microseconds => 1_000.0,
            TimeDisplayUnit::Nanoseconds => 1.0,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            TimeDisplayUnit::Seconds => "s",
            TimeDisplayUnit::Milliseconds => "ms",
            TimeDisplayUnit::Microseconds => "us",
            TimeDisplayUnit::Nanoseconds => "ns",
        }
    }
}
