use moonzoon_novyui::tokens::color::{neutral_8, neutral_11, primary_6};
use moonzoon_novyui::components::{KbdSize, KbdVariant, kbd};
use moonzoon_novyui::*;
use zoon::*;
use shared::{VarFormat, SelectedVariable, TrackedFile};
use crate::selected_variables::{VariableWithContext, filter_variables_with_context, get_variables_from_tracked_files};
use crate::virtual_list::virtual_variables_list_pre_filtered;
use crate::visualizer::interaction::dragging::{
    files_panel_height_signal, variables_name_column_width_signal,
    variables_value_column_width_signal,
};

/// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

/// Main variables panel for browsing and searching variables
pub fn variables_panel(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
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
                            variables_display_signal(tracked_files.clone(), selected_variables.clone())
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
                                .right_icon_signal(selected_variables.search_filter.signal().map(|text| {
                                    if text.is_empty() {
                                        None
                                    } else {
                                        Some(IconName::X)
                                    }
                                }))
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
            simple_variables_content(&tracked_files, &selected_variables),
        ))
}

/// Selected variables with waveform panel - complex multi-column layout
pub fn selected_variables_with_waveform_panel(
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    tracked_files: crate::tracked_files::TrackedFiles,
    app_config: crate::config::AppConfig,
) -> impl Element {
    let selected_variables_for_signals = selected_variables.clone();
    let tracked_files_broadcaster = tracked_files.files.signal_vec().to_signal_cloned().broadcast();
    let waveform_timeline_clone = waveform_timeline.clone();
    
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
                                    .item(crate::panel_layout::variables_name_vertical_divider(&app_config))
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
                                                selected_variables_for_values.variables.signal_vec().map(move |selected_var| {
                                                    El::new()
                                                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                        .s(Width::fill())
                                                        .child(
                                                            crate::format_selection::create_format_dropdown(
                                                                &selected_var.unique_id,
                                                                selected_var.formatter.unwrap_or(VarFormat::Hexadecimal), 
                                                                &selected_variables_for_values, 
                                                                &waveform_timeline_for_values
                                                            )
                                                        )
                                                })
                                            })
                                            .item(
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
                                                                            .update_raw_el(|raw_el| {
                                                                                raw_el.style("width", "max-content")
                                                                            })
                                                                            .child(
                                                                                Text::new("0s")
                                                                            )
                                                                    )
                                                                    .item(kbd("A").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Pan left • Shift+A: Pan left faster").build())
                                                            )
                                                            .item(El::new().s(Width::fill()))
                                                            .item(
                                                                Row::new()
                                                                    .s(Gap::new().x(SPACING_2))
                                                                    .item(kbd("Q").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Move cursor left • Shift+Q: Jump to previous transition").build())
                                                                    .item(
                                                                        El::new()
                                                                            .update_raw_el(|raw_el| {
                                                                                raw_el
                                                                                    .style("min-width", "45px")
                                                                                    .style("width", "fit-content")
                                                                                    .style("max-width", "90px")
                                                                            })
                                                                            .s(Font::new().color_signal(neutral_11()).center())
                                                                            .child(
                                                                                // Connected to cursor position - displays dynamic time
                                                                                Text::new("Dynamic time")
                                                                            )
                                                                    )
                                                                    .item(kbd("E").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Move cursor right • Shift+E: Jump to next transition").build())
                                                            )
                                                            .item(El::new().s(Width::fill()))
                                                            .item(
                                                                Row::new()
                                                                    .s(Gap::new().x(SPACING_6))
                                                                    .item(kbd("D").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Pan right • Shift+D: Pan right faster").build())
                                                                    .item(
                                                                        El::new()
                                                                            .s(Font::new().color_signal(neutral_11()).center().size(11))
                                                                            .update_raw_el(|raw_el| {
                                                                                raw_el.style("width", "max-content")
                                                                            })
                                                                            .child(
                                                                                Text::new("1s")
                                                                            )
                                                                    )
                                                            )
                                                    )
                                            )
                                    )
                                    .item(crate::panel_layout::variables_value_vertical_divider(&app_config))
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_2()))
                                            .child(crate::visualizer::canvas::waveform_canvas::waveform_canvas(&selected_variables, &waveform_timeline, &app_config))
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
    app_config: &crate::config::AppConfig,
) -> impl Element {
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    let waveform_timeline = waveform_timeline.clone();
    let app_config = app_config.clone();
    
    El::new()
        .s(Width::growable())
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
                                    moonzoon_novyui::tokens::color::primary_3().map(move |track| format!("{} {}", thumb, track))
                                })
                                .flatten(),
                        )
                    })
                    .child(variables_panel(&tracked_files, &selected_variables, &waveform_timeline))
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
                                    moonzoon_novyui::tokens::color::primary_3().map(move |track| format!("{} {}", thumb, track))
                                })
                                .flatten(),
                        )
                    })
                    .child(variables_panel(&tracked_files, &selected_variables, &waveform_timeline))
                    .into_element()
            }
        }))
}

/// Simple variables content for the variables panel
pub fn simple_variables_content(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    Column::new()
        .s(Gap::new().y(0))
        .s(Height::fill())
        .s(Width::fill())
        .item(El::new().s(Height::fill()).s(Width::fill()).child_signal(
            variables_display_signal(tracked_files.clone(), selected_variables.clone()).map({
                let selected_variables = selected_variables.clone();
                move |filtered_variables| {
                    virtual_variables_list_pre_filtered(filtered_variables, &selected_variables).into_element()
                }
            }),
        ))
}

/// Signal for loading variables from tracked files
pub fn variables_loading_signal(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Signal<Item = Vec<VariableWithContext>> {
    let files_signal = tracked_files.files_vec_signal.signal_cloned();
    let selected_scope_signal = selected_variables.selected_scope.signal();
    
    map_ref! {
        let selected_scope_id = selected_scope_signal,
        let tracked_files = files_signal => {
            if let Some(scope_id) = selected_scope_id {
                get_variables_from_tracked_files(&scope_id, &tracked_files)
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
) -> impl Signal<Item = Vec<VariableWithContext>> {
    map_ref! {
        let variables = variables_loading_signal(tracked_files.clone(), selected_variables.clone()),
        let search_filter = selected_variables.search_filter.signal() => {
            filter_variables_with_context(&variables, &search_filter)
        }
    }
}