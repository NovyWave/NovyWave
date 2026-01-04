use crate::dragging::files_panel_height_signal;
use crate::selected_variables::{
    VariableWithContext, filter_variables_with_context, get_variables_from_tracked_files,
};
use crate::virtual_list::virtual_variables_list_pre_filtered;
use moonzoon_novyui::tokens::color::neutral_8;
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
use moonzoon_novyui::tokens::color::{primary_3, primary_6};
use moonzoon_novyui::*;
use zoon::*;
use zoon::{JsCast, Key, RawKeyboardEvent};

/// Context enum for Variables Panel display states
#[derive(Clone, Debug)]
pub enum VariableDisplayContext {
    NoScopeSelected,
    ScopeHasNoVariables,
    NoFilterMatches,
    Variables(Vec<VariableWithContext>),
}

#[cfg(NOVYWAVE_PLATFORM = "WEB")]
fn apply_scrollbar_colors(
    raw_el: zoon::RawHtmlEl<web_sys::HtmlElement>,
) -> zoon::RawHtmlEl<web_sys::HtmlElement> {
    raw_el.style_signal(
        "scrollbar-color",
        primary_6()
            .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
            .flatten(),
    )
}

#[cfg(not(NOVYWAVE_PLATFORM = "WEB"))]
fn apply_scrollbar_colors(
    raw_el: zoon::RawHtmlEl<web_sys::HtmlElement>,
) -> zoon::RawHtmlEl<web_sys::HtmlElement> {
    raw_el
}

/// Main variables panel for browsing and searching variables
pub fn variables_panel(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    _waveform_canvas: &crate::visualizer::canvas::waveform_canvas::WaveformCanvas,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    let tracked_files = tracked_files.clone();
    let selected_variables = selected_variables.clone();
    let _waveform_timeline = waveform_timeline.clone();

    let sv_for_filter = selected_variables.clone();
    let sv_for_focus = selected_variables.clone();
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
                                .value_signal(selected_variables.search_filter.signal_cloned())
                                .left_icon(IconName::Search)
                                .right_icon_signal(selected_variables.search_filter.signal_cloned().map(
                                    |text| {
                                        if text.is_empty() {
                                            None
                                        } else {
                                            Some(IconName::X)
                                        }
                                    },
                                ))
                                .on_right_icon_click({
                                    let sv = sv_for_filter.clone();
                                    move || sv.set_search_filter(String::new())
                                })
                                .size(InputSize::Small)
                                .on_change({
                                    let sv = sv_for_filter.clone();
                                    move |text| sv.set_search_filter(text)
                                })
                                .on_focus({
                                    let sv = sv_for_focus.clone();
                                    move || sv.set_search_focus(true)
                                })
                                .on_blur({
                                    let sv = sv_for_focus.clone();
                                    move || sv.set_search_focus(false)
                                })
                                .on_key_down_event({
                                    let sv = sv_for_focus.clone();
                                    move |event| {
                                        if event.key() == &Key::Escape {
                                            event.pass_to_parent(false);
                                            match &event.raw_event {
                                                RawKeyboardEvent::KeyDown(native_event) => {
                                                    native_event.prevent_default();
                                                }
                                            }

                                            if let Some(window) = web_sys::window() {
                                                if let Some(document) = window.document() {
                                                    if let Some(active_element) =
                                                        document.active_element()
                                                    {
                                                        if let Ok(html_element) =
                                                            active_element
                                                                .dyn_into::<web_sys::HtmlElement>()
                                                        {
                                                            let _ = html_element.blur();
                                                        }
                                                    }
                                                }
                                            }

                                            sv.set_search_focus(false);
                                        }
                                    }
                                })
                                .build(),
                        ),
                ),
            simple_variables_content(&tracked_files, &selected_variables, &app_config),
        ))
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
        .child_signal(app_config.dock_mode.signal_cloned().map(move |dock_mode| {
            let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
            if is_docked {
                El::new()
                    .s(Width::fill())
                    .s(Height::exact_signal(
                        files_panel_height_signal(app_config.clone()).map(|h| h as u32),
                    ))
                    .update_raw_el(|raw_el| {
                        raw_el
                            .style("scrollbar-width", "thin")
                            .apply(|raw_el| apply_scrollbar_colors(raw_el))
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
                        raw_el
                            .style("scrollbar-width", "thin")
                            .apply(|raw_el| apply_scrollbar_colors(raw_el))
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

/// Signal for loading variables from tracked files
pub fn variables_loading_signal(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = Vec<VariableWithContext>> {
    let files_signal = tracked_files.files.signal_vec_cloned().to_signal_cloned();
    // Merge SelectedVariables.selected_scope with TreeView selection snapshot from AppConfig
    let selected_scope_from_sv = selected_variables.selected_scope.signal_cloned();
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
        let search_filter = selected_variables.search_filter.signal_cloned() => {
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
        let selected_scope_sv = selected_variables.selected_scope.signal_cloned(),
        let selected_scope_tree = app_config.files_selected_scope
            .signal_vec_cloned()
            .to_signal_cloned()
            .map(|vec| vec.into_iter().rev().find(|id| id.starts_with("scope_")).clone())
            .map(|opt| opt.and_then(|raw| raw.strip_prefix("scope_").map(|s| s.to_string()))),
        let unfiltered_variables = variables_loading_signal(tracked_files.clone(), selected_variables.clone(), app_config.clone()),
        let search_filter = selected_variables.search_filter.signal_cloned() => {
            let selected_scope_id = selected_scope_sv.clone().or_else(|| selected_scope_tree.clone());
            // Debug-only: variables panel context (silenced by default)
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
