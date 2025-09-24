use crate::dragging::{
    files_panel_height_signal, variables_name_column_width_signal,
    variables_value_column_width_signal,
};
use crate::selected_variables::{
    VariableWithContext, filter_variables_with_context, get_variables_from_tracked_files,
};
use crate::virtual_list::virtual_variables_list_pre_filtered;
use crate::visualizer::timeline::NsPerPixel;
use moonzoon_novyui::components::{KbdSize, KbdVariant, kbd};
use moonzoon_novyui::tokens::color::{neutral_8, neutral_11, primary_6};
use moonzoon_novyui::*;
use shared::{SelectedVariable, TrackedFile, VarFormat};
use zoon::*;

/// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

/// Context enum for Variables Panel display states
#[derive(Clone, Debug)]
pub enum VariableDisplayContext {
    NoScopeSelected,
    ScopeHasNoVariables,
    NoFilterMatches,
    Variables(Vec<VariableWithContext>),
}

/// Main variables panel for browsing and searching variables
pub fn variables_panel(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    let _waveform_timeline = waveform_timeline.clone();

    let search_filter_relay = selected_variables.search_filter_changed_relay.clone();
    let search_focus_relay = selected_variables.search_focus_changed_relay.clone();
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .child(crate::panel_layout::create_panel(
            Row::new()
                .s(Width::fill())
                .s(Gap::new().x(SPACING_8))
                .s(Align::new().left().center_y())
                .item(El::new().s(Font::new().no_wrap()).child("Variables"))
                .item(
                    El::new()
                        .s(Font::new().no_wrap().color_signal(neutral_8()).size(13))
                        .child_signal(
                            variables_display_signal(
                                tracked_files.clone(),
                                selected_variables.clone(),
                                app_config.clone(),
                            )
                            .map(|filtered_variables| filtered_variables.len().to_string()),
                        ),
                )
                .item(
                    El::new()
                        .s(Width::fill().max(230))
                        .s(Align::new().right())
                        .child(
                            input()
                                .placeholder("variable_name")
                                .value_signal(selected_variables.search_filter.signal())
                                .left_icon(IconName::Search)
                                .right_icon_signal(selected_variables.search_filter.signal().map(
                                    |text| {
                                        if text.is_empty() {
                                            None
                                        } else {
                                            Some(IconName::X)
                                        }
                                    },
                                ))
                                .on_right_icon_click({
                                    let relay = search_filter_relay.clone();
                                    move || relay.send(String::new())
                                })
                                .size(InputSize::Small)
                                .on_change({
                                    let relay = search_filter_relay.clone();
                                    move |text| relay.send(text)
                                })
                                .on_focus({
                                    let relay = search_focus_relay.clone();
                                    move || relay.send(true)
                                })
                                .on_blur({
                                    let relay = search_focus_relay.clone();
                                    move || relay.send(false)
                                })
                                .build(),
                        ),
                ),
            simple_variables_content(&tracked_files, &selected_variables, &app_config),
        ))
}

/// Selected variables with waveform panel - complex multi-column layout
pub fn selected_variables_with_waveform_panel(
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    tracked_files: crate::tracked_files::TrackedFiles,
    app_config: crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
    waveform_canvas: crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
) -> impl Element {
    let selected_variables_for_signals = selected_variables.clone();
    let tracked_files_broadcaster = tracked_files
        .files
        .signal_vec()
        .to_signal_cloned()
        .broadcast();

    let name_column_width_signal = variables_name_column_width_signal(app_config.clone());
    let value_column_width_signal = variables_value_column_width_signal(app_config.clone());

    Column::new()
        .s(Width::growable())
        .s(Height::fill())
        .item(
            El::new()
                .s(Width::growable())
                .s(Height::fill())
                .child(
                    crate::panel_layout::create_panel(
                        Row::new()
                            .s(Gap::new().x(SPACING_8))
                            .s(Align::new().center_y())
                            .item(
                                El::new()
                                    .s(Font::new().no_wrap())
                                    .child("Selected Variables")
                            )
                            .item(
                                El::new()
                                    .s(Width::growable())
                            )
                            .item(
                                crate::action_buttons::theme_toggle_button(&app_config)
                            )
                            .item(
                                crate::action_buttons::dock_toggle_button(&app_config)
                            )
                            .item(
                                El::new()
                                    .s(Width::growable())
                            )
                            .item(
                                crate::action_buttons::clear_all_variables_button(&selected_variables)
                            ),
                        El::new()
                            .s(Height::exact_signal(
                                selected_variables_for_signals.variables.signal_vec().to_signal_cloned().map(|vars| {
                                    (vars.len() + 1) as u32 * SELECTED_VARIABLES_ROW_HEIGHT
                                })
                            ))
                            .s(Width::fill())
                            .s(Scrollbars::x_and_clip_y())
                            .child(
                                Row::new()
                                    .s(Height::fill())
                                    .s(Width::fill())
                                    .s(Align::new().top())
                                    .item(
                                        Column::new()
                                            .s(Width::exact_signal(name_column_width_signal.map(|w| w as u32)))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .s(Scrollbars::x_and_clip_y())
                                            .update_raw_el(|raw_el| {
                                                raw_el.style("scrollbar-width", "thin")
                                            })
                                            .items_signal_vec({
                                                let selected_variables_for_items = selected_variables_for_signals.clone();
                                                let tracked_files_broadcaster_for_items = tracked_files_broadcaster.clone();
                                                selected_variables_for_signals.variables.signal_vec().map(move |selected_var| {
                                                    Row::new()
                                                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                        .s(Width::fill())
                                                        .s(Padding::new().x(SPACING_2).y(SPACING_4))
                                                        .s(Gap::new().x(SPACING_4))
                                                        .item({
                                                            let unique_id = selected_var.unique_id.clone();
                                                            button()
                                                                .left_icon(IconName::X)
                                                                .variant(ButtonVariant::DestructiveGhost)
                                                                .size(ButtonSize::Small)
                                                                .custom_padding(2, 2)
                                                                .on_press({
                                                                    let remove_relay = selected_variables_for_items.variable_removed_relay.clone();
                                                                    move || {
                                                                        remove_relay.send(unique_id.clone());
                                                                    }
                                                                })
                                                                .build()
                                                        })
                                                        .item(
                                                            Row::new()
                                                                .s(Gap::new().x(SPACING_8))
                                                                .s(Width::fill())
                                                                .item(
                                                                    El::new()
                                                                        .s(Font::new().color_signal(neutral_11()).size(13).no_wrap())
                                                                        .s(Width::growable())
                                                                        .update_raw_el(|raw_el| {
                                                                            raw_el.style("white-space", "nowrap")
                                                                        })
                                                                        .child(&selected_var.variable_name().unwrap_or_default())
                                                                )
                                                                .item(
                                                                    El::new()
                                                                        .s(Font::new().color_signal(primary_6()).size(11).no_wrap())
                                                                        .s(Align::new().right())
                                                                        .s(Padding::new().right(8))
                                                                        .update_raw_el(|raw_el| {
                                                                            raw_el
                                                                                .style("text-overflow", "ellipsis") // Show ellipsis for long text
                                                                                .style("max-width", "100%") // Ensure it doesn't exceed container
                                                                        })
                                                                        .child_signal({
                                                                            let selected_var = selected_var.clone();
                                                                            tracked_files_broadcaster_for_items.signal_cloned().map(move |files: Vec<TrackedFile>| {
                                                                                crate::signal_processing::get_signal_type_for_selected_variable_from_files(&selected_var, &files)
                                                                            })
                                                                        })
                                                                )
                                                                .update_raw_el({
                                                                    let selected_var = selected_var.clone();
                                                                    let tracked_files_broadcaster = tracked_files_broadcaster_for_items.clone();
                                                                    move |raw_el| {
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
                                                })
                                            })
                                            .item(
                                                El::new()
                                                    .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                    .s(Width::fill())
                                                    .s(Padding::all(8))
                                                    .s(Font::new().color_signal(neutral_8()).size(12).center())
                                                    .s(Transform::new().move_up(4))
                                                    .child(
                                                        Row::new()
                                                            .s(Align::new().center_y())
                                                            .item(kbd("Z").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Reset zoom center to time 0").build())
                                                            .item(El::new().s(Width::fill()))
                                                            .item(
                                                                Row::new()
                                                                    .s(Align::center())
                                                                    .s(Gap::new().x(SPACING_6))
                                                                    .item(kbd("W").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Zoom in • Shift+W: Zoom in faster").build())
                                                                    .item(
                                                                        El::new()
                                                                            .update_raw_el(|raw_el| {
                                                                                raw_el
                                                                                    .style("min-width", "45px")
                                                                                    .style("width", "fit-content")
                                                                                    .style("max-width", "80px")
                                                                            })
                                                                            .s(Font::new().color_signal(neutral_11()).center())
                                                                            .child(
                                                                                // Connected to zoom level - displays dynamic ns/px
                                                                                Text::new("Dynamic ns/px")
                                                                            )
                                                                    )
                                                                    .item(kbd("S").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Zoom out • Shift+S: Zoom out faster").build())
                                                            )
                                                            .item(El::new().s(Width::fill()))
                                                            .item(
                                                                El::new()
                                                                    .on_click({
                                                                        let relay = waveform_timeline.reset_zoom_pressed_relay.clone();
                                                                        move || {
                                                                            relay.send(());
                                                                        }
                                                                    })
                                                                    .child(kbd("R").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Reset zoom to 1x, fit all data, and center cursor").build())
                                                            )
                                                    )
                                            )
                                    )
                                    .item(crate::panel_layout::variables_name_vertical_divider(&app_config, dragging_system.clone()))
                                    .item(
                                        Column::new()
                                            .s(Width::exact_signal(value_column_width_signal.map(|w| w as u32)))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .s(Scrollbars::x_and_clip_y())
                                            .update_raw_el(|raw_el| {
                                                raw_el.style("scrollbar-width", "thin")
                                            })
                                            .items_signal_vec({
                                                let selected_variables_for_values = selected_variables.clone();
                                                let waveform_timeline_for_values = waveform_timeline.clone();
                                                let app_config_for_values = app_config.clone();
                                                selected_variables_for_values.variables.signal_vec().map(move |selected_var| {
                                                    El::new()
                                                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                        .s(Width::fill())
                                                        .child(
                                                            crate::format_selection::create_format_dropdown(
                                                                &selected_var.unique_id,
                                                                selected_var.formatter.unwrap_or(VarFormat::Hexadecimal),
                                                                &selected_variables_for_values,
                                                                &waveform_timeline_for_values,
                                                                app_config_for_values.clone()
                                                            )
                                                        )
                                                })
                                            })
                                            .item(timeline_footer(waveform_timeline.clone()))
                                    )
                                    .item(crate::panel_layout::variables_value_vertical_divider(&app_config, dragging_system.clone()))
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_2()))
                                            .child(crate::visualizer::canvas::waveform_canvas::waveform_canvas(&waveform_canvas, &waveform_timeline))
                                    )
                            )
                    )
                )
        )
}

/// Variables panel with dynamic height based on dock mode
pub fn variables_panel_with_fill(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    let waveform_timeline = waveform_timeline.clone();
    let waveform_canvas = waveform_canvas.clone();
    let app_config = app_config.clone();

    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Scrollbars::both())
        .child_signal(app_config.dock_mode_actor.signal().map(move |dock_mode| {
            let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
            if is_docked {
                El::new()
                    .s(Width::fill())
                    .s(Height::exact_signal(
                        files_panel_height_signal(app_config.clone()).map(|h| h as u32),
                    ))
                    .update_raw_el(|raw_el| {
                        raw_el.style("scrollbar-width", "thin").style_signal(
                            "scrollbar-color",
                            primary_6()
                                .map(|thumb| {
                                    moonzoon_novyui::tokens::color::primary_3()
                                        .map(move |track| format!("{} {}", thumb, track))
                                })
                                .flatten(),
                        )
                    })
                    .child(variables_panel(
                        &tracked_files,
                        &selected_variables,
                        &waveform_timeline,
                        &waveform_canvas,
                        &app_config,
                    ))
                    .into_element()
            } else {
                El::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .update_raw_el(|raw_el| {
                        raw_el.style("scrollbar-width", "thin").style_signal(
                            "scrollbar-color",
                            primary_6()
                                .map(|thumb| {
                                    moonzoon_novyui::tokens::color::primary_3()
                                        .map(move |track| format!("{} {}", thumb, track))
                                })
                                .flatten(),
                        )
                    })
                    .child(variables_panel(
                        &tracked_files,
                        &selected_variables,
                        &waveform_timeline,
                        &waveform_canvas,
                        &app_config,
                    ))
                    .into_element()
            }
        }))
}

/// Simple variables content for the variables panel with improved empty state handling
pub fn simple_variables_content(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    let app_config = app_config.clone();
    Column::new()
        .s(Gap::new().y(0))
        .s(Height::fill())
        .s(Width::fill())
        .item(
            El::new().s(Height::fill()).s(Width::fill()).child_signal(
                variables_display_context_signal(
                    tracked_files.clone(),
                    selected_variables.clone(),
                    app_config.clone(),
                )
                .map({
                    let selected_variables = selected_variables.clone();
                    move |context| match context {
                        VariableDisplayContext::NoScopeSelected => Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(crate::virtual_list::empty_state_hint(
                                "Select scope in the Files & Scopes panel",
                            )),
                        VariableDisplayContext::ScopeHasNoVariables => Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(crate::virtual_list::empty_state_hint(
                                "Selected scope does not have any variables",
                            )),
                        VariableDisplayContext::NoFilterMatches => Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(crate::virtual_list::empty_state_hint(
                                "No variables match search filter",
                            )),
                        VariableDisplayContext::Variables(filtered_variables) => {
                            virtual_variables_list_pre_filtered(
                                filtered_variables,
                                &selected_variables,
                            )
                        }
                    }
                }),
            ),
        )
}

fn timeline_footer(
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
) -> impl Element {
    let viewport_actor = waveform_timeline.viewport_actor();
    let start_signal = viewport_actor
        .clone()
        .signal()
        .map(|viewport| format_time_ns(viewport.start.nanos()));

    let end_signal = viewport_actor
        .signal()
        .map(|viewport| format_time_ns(viewport.end.nanos()));

    let cursor_signal = waveform_timeline
        .cursor_actor()
        .signal()
        .map(|cursor| format_time_ns(cursor.nanos()));

    let zoom_signal = {
        let viewport_actor = waveform_timeline.viewport_actor();
        let width_actor = waveform_timeline.canvas_width_actor();
        map_ref! {
            let viewport = viewport_actor.signal(),
            let width = width_actor.signal() => {
                let range = viewport.duration().nanos();
                let width_px = width.max(1.0) as u64;
                let ns_per_pixel = if width_px == 0 { 1 } else { (range / width_px.max(1)).max(1) };
                NsPerPixel(ns_per_pixel).to_string()
            }
        }
    };

    El::new()
        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
        .s(Width::fill())
        .s(Padding::all(8))
        .s(Transform::new().move_up(4))
        .child(
            Row::new()
                .s(Align::new().center_y())
                .s(Font::new().color_signal(neutral_8()).size(12))
                .item(
                    Row::new()
                        .s(Gap::new().x(SPACING_6))
                        .item(
                            El::new()
                                .s(Font::new().color_signal(neutral_11()).center().size(11))
                                .update_raw_el(|raw_el| raw_el.style("width", "max-content"))
                                .child_signal(start_signal.map(Text::new)),
                        )
                        .item(
                            kbd("A")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Pan left • Shift+A: Pan left faster")
                                .build(),
                        ),
                )
                .item(El::new().s(Width::fill()))
                .item(
                    Row::new()
                        .s(Gap::new().x(SPACING_2))
                        .item(
                            kbd("Q")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Move cursor left • Shift+Q: Jump to previous transition")
                                .build(),
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
                                .child_signal(cursor_signal.map(Text::new)),
                        )
                        .item(
                            kbd("E")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Move cursor right • Shift+E: Jump to next transition")
                                .build(),
                        ),
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
                        .child_signal(zoom_signal.map(Text::new)),
                )
                .item(El::new().s(Width::fill()))
                .item(
                    Row::new()
                        .s(Gap::new().x(SPACING_6))
                        .item(
                            kbd("D")
                                .size(KbdSize::Small)
                                .variant(KbdVariant::Outlined)
                                .title("Pan right • Shift+D: Pan right faster")
                                .build(),
                        )
                        .item(
                            El::new()
                                .s(Font::new().color_signal(neutral_11()).center().size(11))
                                .update_raw_el(|raw_el| raw_el.style("width", "max-content"))
                                .child_signal(end_signal.map(Text::new)),
                        ),
                ),
        )
}

fn format_time_ns(ns: u64) -> String {
    if ns >= 1_000_000_000 {
        format!("{:.1}s", ns as f64 / 1_000_000_000.0)
    } else if ns >= 1_000_000 {
        format!("{:.1}ms", ns as f64 / 1_000_000.0)
    } else if ns >= 1_000 {
        format!("{:.1}us", ns as f64 / 1_000.0)
    } else {
        format!("{}ns", ns)
    }
}

/// Signal for loading variables from tracked files
pub fn variables_loading_signal(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = Vec<VariableWithContext>> {
    let files_signal = tracked_files.files.signal_vec().to_signal_cloned();
    // Merge SelectedVariables.selected_scope with TreeView selection snapshot from AppConfig
    let selected_scope_from_sv = selected_variables.selected_scope.signal();
    let selected_scope_from_tree = app_config
        .files_selected_scope
        .signal_vec_cloned()
        .to_signal_cloned()
        .map(|vec| {
            vec.into_iter()
                .rev()
                .find(|id| id.starts_with("scope_"))
                .clone()
        })
        .map(|opt| opt.and_then(|raw| raw.strip_prefix("scope_").map(|s| s.to_string())));

    map_ref! {
        let sv_scope = selected_scope_from_sv,
        let tree_scope = selected_scope_from_tree,
        let tracked_files = files_signal => {
            let effective_scope = sv_scope.clone().or_else(|| tree_scope.clone());
            if let Some(scope_id) = effective_scope {
                get_variables_from_tracked_files(scope_id.as_str(), &tracked_files)
            } else {
                Vec::new()
            }
        }
    }
}

/// Signal for displaying filtered variables
pub fn variables_display_signal(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = Vec<VariableWithContext>> {
    map_ref! {
        let variables = variables_loading_signal(tracked_files.clone(), selected_variables.clone(), app_config.clone()),
        let search_filter = selected_variables.search_filter.signal() => {
            filter_variables_with_context(&variables, &search_filter)
        }
    }
}

/// Signal providing context for variables panel display with proper empty state handling
pub fn variables_display_context_signal(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = VariableDisplayContext> {
    map_ref! {
        let selected_scope_sv = selected_variables.selected_scope.signal(),
        let selected_scope_tree = app_config.files_selected_scope
            .signal_vec_cloned()
            .to_signal_cloned()
            .map(|vec| vec.into_iter().rev().find(|id| id.starts_with("scope_")).clone())
            .map(|opt| opt.and_then(|raw| raw.strip_prefix("scope_").map(|s| s.to_string()))),
        let unfiltered_variables = variables_loading_signal(tracked_files.clone(), selected_variables.clone(), app_config.clone()),
        let search_filter = selected_variables.search_filter.signal() => {
            let selected_scope_id = selected_scope_sv.clone().or_else(|| selected_scope_tree.clone());
            zoon::println!(
                "🔎 VARIABLES_CONTEXT: selected_scope_id={:?}, unfiltered_count={}, filter='{}'",
                selected_scope_id,
                unfiltered_variables.len(),
                search_filter
            );
            // Determine the appropriate context based on state
            if selected_scope_id.is_none() {
                // No scope selected at all
                VariableDisplayContext::NoScopeSelected
            } else if unfiltered_variables.is_empty() {
                // Scope selected but it has no variables
                VariableDisplayContext::ScopeHasNoVariables
            } else {
                // Scope has variables, apply filter
                let filtered_variables = filter_variables_with_context(&unfiltered_variables, &search_filter);
                if filtered_variables.is_empty() && !search_filter.is_empty() {
                    // Variables exist but filter matches none
                    VariableDisplayContext::NoFilterMatches
                } else {
                    // Show filtered variables (could be all if no filter)
                    VariableDisplayContext::Variables(filtered_variables)
                }
            }
        }
    }
}
