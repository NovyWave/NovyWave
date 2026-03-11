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
use moonzoon_novyui::components::input::{InputSize, input};
use moonzoon_novyui::components::{KbdSize, KbdVariant, kbd};
use moonzoon_novyui::tokens::color::{neutral_8, neutral_11, primary_6};
use moonzoon_novyui::*;
use shared::{AnalogLimits, SelectedVariable, SignalValue, TrackedFile, VarFormat};
use std::rc::Rc;
use zoon::*;

/// Selected Variables panel row height constant
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;

#[derive(Clone)]
struct GroupDialogState {
    visible: Mutable<bool>,
    editing_index: Mutable<Option<usize>>,
    name_input: Mutable<String>,
}

#[derive(Clone)]
struct AnalogLimitsDialogState {
    visible: Mutable<bool>,
    target_unique_id: Mutable<Option<String>>,
    target_label: Mutable<String>,
    auto: Mutable<bool>,
    min_input: Mutable<String>,
    max_input: Mutable<String>,
    error_message: Mutable<Option<String>>,
}

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
    let waveform_timeline_for_header = waveform_timeline.clone();
    let app_config_for_header = app_config.clone();
    let group_dialog = GroupDialogState {
        visible: Mutable::new(false),
        editing_index: Mutable::new(None),
        name_input: Mutable::new(String::new()),
    };
    let analog_dialog = AnalogLimitsDialogState {
        visible: Mutable::new(false),
        target_unique_id: Mutable::new(None),
        target_label: Mutable::new(String::new()),
        auto: Mutable::new(true),
        min_input: Mutable::new(String::new()),
        max_input: Mutable::new(String::new()),
        error_message: Mutable::new(None),
    };
    let marker_manager_visible = Mutable::new(false);

    Stack::new()
        .s(Width::fill())
        .s(Height::fill())
        .layer(Column::new().s(Width::fill()).s(Height::fill()).item(
            crate::panel_layout::create_panel(
                selected_variables_panel_header(
                    &selected_variables_for_header,
                    &waveform_timeline_for_header,
                    &app_config_for_header,
                    group_dialog.clone(),
                    marker_manager_visible.clone(),
                ),
                selected_variables_panel_content(
                    selected_variables,
                    waveform_timeline.clone(),
                    tracked_files,
                    app_config.clone(),
                    dragging_system,
                    waveform_canvas,
                    group_dialog.clone(),
                    analog_dialog.clone(),
                ),
            ),
        ))
        .layer_signal(group_dialog.visible.signal().map_true({
            let selected_variables = selected_variables_for_header.clone();
            let app_config = app_config_for_header.clone();
            move || {
                group_name_dialog(
                    selected_variables.clone(),
                    app_config.clone(),
                    group_dialog.clone(),
                )
            }
        }))
        .layer_signal(analog_dialog.visible.signal().map_true({
            let selected_variables = selected_variables_for_header.clone();
            move || analog_limits_dialog(selected_variables.clone(), analog_dialog.clone())
        }))
        .layer_signal(marker_manager_visible.signal().map_true({
            let timeline = waveform_timeline_for_header.clone();
            let app_config = app_config_for_header.clone();
            move || {
                marker_manager_dialog(
                    timeline.clone(),
                    app_config.clone(),
                    marker_manager_visible.clone(),
                )
            }
        }))
}

/// Panel header with title and action buttons
fn selected_variables_panel_header(
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
    group_dialog: GroupDialogState,
    marker_manager_visible: Mutable<bool>,
) -> impl Element {
    let sv_for_group_toggle = selected_variables.clone();
    let sv_for_group_create = selected_variables.clone();
    let timeline_for_markers = waveform_timeline.clone();

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
            // Center section with version, group controls, and dock button
            Row::new()
                .s(Width::fill())
                .s(Align::new().center_y())
                .s(Gap::new().x(SPACING_8))
                .item(El::new().s(Width::growable()))
                .item(
                    button()
                        .label_signal(sv_for_group_toggle.grouping_mode_active.signal().map(
                            |active| {
                                if active {
                                    "Cancel".to_string()
                                } else {
                                    "Group".to_string()
                                }
                            },
                        ))
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Small)
                        .on_press(move || {
                            let current = sv_for_group_toggle.grouping_mode_active.get();
                            if current {
                                sv_for_group_toggle.grouping_mode_active.set(false);
                                sv_for_group_toggle
                                    .selected_for_grouping
                                    .set(indexmap::IndexSet::new());
                            } else {
                                sv_for_group_toggle.grouping_mode_active.set(true);
                            }
                        })
                        .build(),
                )
                .item_signal(
                    sv_for_group_create
                        .selected_for_grouping
                        .signal_cloned()
                        .map({
                            let sv = sv_for_group_create.clone();
                            move |selected| {
                                if selected.len() >= 2 {
                                    Some(
                                        button()
                                            .label(format!("Create Group ({})", selected.len()))
                                            .size(ButtonSize::Small)
                                            .on_press({
                                                let sv = sv.clone();
                                                let dialog = group_dialog.clone();
                                                move || {
                                                    let count =
                                                        sv.signal_groups.lock_ref().len() + 1;
                                                    dialog.editing_index.set(None);
                                                    dialog.name_input.set(format!("Group {count}"));
                                                    dialog.visible.set(true);
                                                }
                                            })
                                            .build()
                                            .into_raw(),
                                    )
                                } else {
                                    None
                                }
                            }
                        }),
                )
                .item(
                    button()
                        .label_signal(timeline_for_markers.markers.signal_vec_cloned().len().map(
                            |count| {
                                if count == 0 {
                                    "Markers".to_string()
                                } else {
                                    format!("Markers ({count})")
                                }
                            },
                        ))
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Small)
                        .on_press({
                            let marker_manager_visible = marker_manager_visible.clone();
                            move || marker_manager_visible.set(true)
                        })
                        .build(),
                )
                .item(
                    // Version display with less contrast
                    El::new()
                        .s(Font::new().no_wrap().color_signal(neutral_8()))
                        .child_signal(
                            signal::from_future(Box::pin(crate::platform::get_app_version())).map(
                                |version| match version {
                                    Some(v) => format!("v{}", v),
                                    None => "...".to_string(),
                                },
                            ),
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
    group_dialog: GroupDialogState,
    analog_dialog: AnalogLimitsDialogState,
) -> impl Element {
    let selected_variables_for_height = selected_variables.clone();
    let _name_column_width_signal = variables_name_column_width_signal(app_config.clone());
    let _value_column_width_signal = variables_value_column_width_signal(app_config.clone());

    El::new()
        .s(Height::exact_signal(
            selected_variables_for_height
                .visible_items
                .signal_cloned()
                .map(|items| {
                    if items.is_empty() {
                        2 * SELECTED_VARIABLES_ROW_HEIGHT
                    } else {
                        let var_count = items.iter().filter(|i| matches!(i, crate::selected_variables::SelectedVariableOrGroup::Variable(_))).count() as u32;
                        let item_heights: u32 = items.iter().map(|item| {
                            match item {
                                crate::selected_variables::SelectedVariableOrGroup::Variable(v) =>
                                    v.row_height.unwrap_or(SELECTED_VARIABLES_ROW_HEIGHT),
                                crate::selected_variables::SelectedVariableOrGroup::GroupHeader { .. } =>
                                    SELECTED_VARIABLES_ROW_HEIGHT,
                            }
                        }).sum();
                        item_heights + var_count * 3 + SELECTED_VARIABLES_ROW_HEIGHT // items + dividers + footer
                    }
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
                .variables_vec_actor
                .signal_cloned()
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
                                    app_config.clone(),
                                    dragging_system.clone(),
                                    group_dialog.clone(),
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
                                    analog_dialog.clone(),
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
    app_config: crate::config::AppConfig,
    dragging_system: crate::dragging::DraggingSystem,
    group_dialog: GroupDialogState,
    width_signal: impl Signal<Item = f32> + Unpin + 'static,
) -> impl Element {
    Column::new()
        .s(Width::exact_signal(width_signal.map(|w| w as u32)))
        .s(Height::fill())
        .s(Align::new().top())
        .s(Scrollbars::x_and_clip_y())
        .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
        .item(El::new().s(Width::fill()).child_signal({
            let sv = selected_variables.clone();
            let tf = tracked_files.clone();
            let cfg = app_config.clone();
            let ds = dragging_system.clone();
            sv.visible_items.signal_cloned().map(move |items| {
                let sv = sv.clone();
                let tf = tf.clone();
                let cfg = cfg.clone();
                let ds = ds.clone();
                let mut elements: Vec<zoon::RawElOrText> = Vec::new();
                for item in items {
                    match item {
                        crate::selected_variables::SelectedVariableOrGroup::Variable(var) => {
                            let uid = var.unique_id.clone();
                            elements.push(
                                name_column_variable_row(var, sv.clone(), tf.clone()).into_raw(),
                            );
                            elements.push(signal_row_divider(uid, ds.clone()).into_raw());
                        }
                        crate::selected_variables::SelectedVariableOrGroup::GroupHeader {
                            index,
                            name,
                            collapsed,
                            member_count,
                        } => {
                            elements.push(
                                name_column_group_header(
                                    index,
                                    name,
                                    collapsed,
                                    member_count,
                                    sv.clone(),
                                    cfg.clone(),
                                    group_dialog.clone(),
                                )
                                .into_raw(),
                            );
                        }
                    }
                }
                Column::new().items(elements)
            })
        }))
        .item(name_column_footer(waveform_timeline))
}

/// Individual variable row in Name Column
fn name_column_variable_row(
    selected_var: SelectedVariable,
    selected_variables: crate::selected_variables::SelectedVariables,
    tracked_files: crate::tracked_files::TrackedFiles,
) -> impl Element {
    let unique_id = selected_var.unique_id.clone();
    let selected_variables_for_remove = selected_variables.clone();
    let var_row_height = selected_var
        .row_height
        .unwrap_or(SELECTED_VARIABLES_ROW_HEIGHT);
    let tracked_files_broadcaster = tracked_files
        .files
        .signal_vec_cloned()
        .to_signal_cloned()
        .broadcast();

    let is_grouped = {
        let groups = selected_variables.signal_groups.lock_ref();
        groups
            .iter()
            .any(|g| g.member_ids.contains(&selected_var.unique_id))
    };

    Row::new()
        .s(Height::exact(var_row_height))
        .s(Width::fill())
        .s(Padding::new().x(SPACING_2).y(SPACING_4))
        .s(Gap::new().x(SPACING_4))
        .update_raw_el({
            let indent = if is_grouped { "16px" } else { "0" };
            move |raw_el| raw_el.style("padding-left", indent)
        })
        .item_signal({
            let sv = selected_variables.clone();
            let uid = selected_var.unique_id.clone();
            sv.grouping_mode_active.signal().map(move |active| {
                if active {
                    let sv = sv.clone();
                    let uid = uid.clone();
                    Some(
                        El::new()
                            .s(Width::exact(18))
                            .s(Height::exact(18))
                            .s(Align::new().center_y())
                            .s(Cursor::new(CursorIcon::Pointer))
                            .s(RoundedCorners::all(3))
                            .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_4()))
                            .s(Borders::all_signal(neutral_8().map(|color| Border::new().width(1).color(color))))
                            .child_signal(sv.selected_for_grouping.signal_cloned().map({
                                let uid = uid.clone();
                                move |selected| {
                                    if selected.contains(&uid) {
                                        Some(El::new()
                                            .s(Width::exact(10))
                                            .s(Height::exact(10))
                                            .s(Align::center())
                                            .s(Background::new().color("oklch(65% 0.2 250)")))
                                    } else {
                                        None
                                    }
                                }
                            }))
                            .on_click({
                                let sv = sv.clone();
                                let uid = uid.clone();
                                move || sv.toggle_variable_selection(&uid)
                            })
                    )
                } else {
                    None
                }
            })
        })
        .item(
            button()
                .left_icon(IconName::X)
                .variant(ButtonVariant::DestructiveGhost)
                .size(ButtonSize::Small)
                .custom_padding(2, 2)
                .on_press({
                    let sv = selected_variables_for_remove.clone();
                    move || {
                        sv.remove_variable(unique_id.clone());
                    }
                })
                .build()
        )
        .item({
            let sv = selected_variables.clone();
            let uid = selected_var.unique_id.clone();
            Row::new()
                .s(Gap::new().x(SPACING_8))
                .s(Width::fill())
                .s(Cursor::new(CursorIcon::Pointer))
                .on_click(move || {
                    if sv.grouping_mode_active.get() {
                        sv.toggle_variable_selection(&uid);
                    }
                })
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
        })
}

fn name_column_group_header(
    group_index: usize,
    name: String,
    collapsed: bool,
    member_count: usize,
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
    group_dialog: GroupDialogState,
) -> impl Element {
    let chevron = if collapsed { "▶" } else { "▼" };

    Row::new()
        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
        .s(Width::fill())
        .s(Padding::new().x(SPACING_2).y(SPACING_4))
        .s(Gap::new().x(SPACING_4))
        .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_3().map(|c| c)))
        .item(
            El::new()
                .s(Font::new().size(11).color_signal(neutral_8()))
                .s(Cursor::new(CursorIcon::Pointer))
                .s(Width::exact(16))
                .child(chevron)
                .on_click({
                    let sv = selected_variables.clone();
                    let cfg = app_config.clone();
                    move || {
                        sv.toggle_group_collapse(group_index);
                        cfg.signal_groups_config.set(sv.signal_groups_as_config());
                    }
                }),
        )
        .item(
            El::new()
                .s(Font::new()
                    .color_signal(neutral_11())
                    .size(13)
                    .no_wrap()
                    .weight(FontWeight::SemiBold))
                .s(Width::growable())
                .child(format!("{name} ({member_count})")),
        )
        .item(
            button()
                .label("Rename")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .on_press({
                    let dialog = group_dialog.clone();
                    move || {
                        dialog.editing_index.set(Some(group_index));
                        dialog.name_input.set(name.clone());
                        dialog.visible.set(true);
                    }
                })
                .build(),
        )
        .item(
            button()
                .left_icon(IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .custom_padding(2, 2)
                .on_press({
                    let sv = selected_variables.clone();
                    let cfg = app_config.clone();
                    move || {
                        sv.ungroup(group_index);
                        cfg.signal_groups_config.set(sv.signal_groups_as_config());
                    }
                })
                .build(),
        )
}

fn signal_row_divider(
    unique_id: String,
    dragging_system: crate::dragging::DraggingSystem,
) -> impl Element {
    use crate::dragging::{DividerType, start_drag};

    El::new()
        .s(Width::fill())
        .s(Height::exact(3))
        .s(Cursor::new(CursorIcon::RowResize))
        .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_3().map(|c| c)))
        .on_pointer_down_event({
            let dragging_system = dragging_system.clone();
            move |event: PointerEvent| {
                let raw_pointer_down = match &event.raw_event {
                    RawPointerEvent::PointerDown(raw_event) => raw_event,
                    _ => return,
                };
                if raw_pointer_down.button() != events::MouseButton::Left {
                    return;
                }
                raw_pointer_down.prevent_default();
                start_drag(
                    &dragging_system,
                    DividerType::SignalRowDivider {
                        unique_id: unique_id.clone(),
                    },
                    (event.x() as f32, event.y() as f32),
                );
            }
        })
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
                                    let timeline = waveform_timeline.clone();
                                    move || {
                                        timeline.reset_zoom();
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
    analog_dialog: AnalogLimitsDialogState,
    width_signal: impl Signal<Item = f32> + Unpin + 'static,
) -> impl Element {
    let waveform_timeline_for_values = waveform_timeline.clone();
    let app_config_for_values = app_config.clone();

    Column::new()
        .s(Width::exact_signal(width_signal.map(|w| w as u32)))
        .s(Height::fill())
        .s(Align::new().top())
        .s(Scrollbars::x_and_clip_y())
        .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
        .item(El::new().s(Width::fill()).child_signal({
            let sv = selected_variables.clone();
            let tl = waveform_timeline_for_values.clone();
            let cfg = app_config_for_values.clone();
            sv.visible_items.signal_cloned().map(move |items| {
                let sv = sv.clone();
                let tl = tl.clone();
                let cfg = cfg.clone();
                let mut elements: Vec<zoon::RawElOrText> = Vec::new();
                for item in items {
                    match item {
                        crate::selected_variables::SelectedVariableOrGroup::Variable(var) => {
                            elements.push(
                                value_column_variable_row(
                                    var,
                                    sv.clone(),
                                    tl.clone(),
                                    cfg.clone(),
                                    analog_dialog.clone(),
                                )
                                .into_raw(),
                            );
                            elements
                                .push(El::new().s(Width::fill()).s(Height::exact(3)).into_raw());
                        }
                        crate::selected_variables::SelectedVariableOrGroup::GroupHeader {
                            name,
                            ..
                        } => {
                            elements.push(
                                El::new()
                                    .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                    .s(Width::fill())
                                    .s(Padding::new().x(SPACING_4).y(SPACING_4))
                                    .s(Font::new().color_signal(neutral_8()).size(12).no_wrap())
                                    .s(Background::new().color_signal(
                                        moonzoon_novyui::tokens::color::neutral_3().map(|c| c),
                                    ))
                                    .child(name)
                                    .into_raw(),
                            );
                        }
                    }
                }
                Column::new().items(elements)
            })
        }))
        .item(value_column_footer(waveform_timeline_for_values))
}

/// Individual variable row in Value Column with format dropdown
fn value_column_variable_row(
    selected_var: SelectedVariable,
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: crate::config::AppConfig,
    analog_dialog: AnalogLimitsDialogState,
) -> impl Element {
    let var_row_height = selected_var
        .row_height
        .unwrap_or(SELECTED_VARIABLES_ROW_HEIGHT);
    let is_real_signal = selected_var.signal_type.as_deref() == Some("Real");
    El::new()
        .s(Height::exact(var_row_height))
        .s(Width::fill())
        .child(if is_real_signal {
            analog_value_row(
                selected_var,
                selected_variables,
                waveform_timeline,
                analog_dialog,
            )
            .into_raw()
        } else {
            crate::format_selection::create_format_dropdown(
                &selected_var.unique_id,
                selected_var.formatter.unwrap_or(VarFormat::Hexadecimal),
                &selected_variables,
                &waveform_timeline,
                app_config,
            )
            .into_raw()
        })
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

fn analog_value_row(
    selected_var: SelectedVariable,
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    analog_dialog: AnalogLimitsDialogState,
) -> impl Element {
    let unique_id = selected_var.unique_id.clone();
    let unique_id_for_value = unique_id.clone();
    let limits = selected_var
        .analog_limits
        .clone()
        .unwrap_or_else(AnalogLimits::auto);

    Column::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Padding::new().x(SPACING_8).y(SPACING_6))
        .s(Gap::new().y(SPACING_6))
        .item(
            El::new()
                .s(Font::new()
                    .size(13)
                    .weight(FontWeight::SemiBold)
                    .color_signal(neutral_11()))
                .child_signal(waveform_timeline.cursor_values_actor().signal_cloned().map(
                    move |values| {
                        format_analog_signal_value(
                            values
                                .get(&unique_id_for_value)
                                .cloned()
                                .unwrap_or(SignalValue::Loading),
                        )
                    },
                )),
        )
        .item(
            Row::new()
                .s(Align::new().center_y())
                .s(Gap::new().x(SPACING_6))
                .item(
                    El::new()
                        .s(Font::new().size(11).color_signal(neutral_8()).no_wrap())
                        .child(analog_limits_summary(&limits)),
                )
                .item(El::new().s(Width::growable()))
                .item_signal(always(!limits.auto).map(move |show_auto| {
                    if show_auto {
                        Some(
                            button()
                                .label("Auto")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Small)
                                .on_press({
                                    let selected_variables = selected_variables.clone();
                                    let unique_id = unique_id.clone();
                                    move || {
                                        selected_variables.update_analog_limits(
                                            &unique_id,
                                            Some(AnalogLimits::auto()),
                                        );
                                    }
                                })
                                .build()
                                .into_raw(),
                        )
                    } else {
                        None
                    }
                }))
                .item(
                    button()
                        .label(if limits.auto { "Manual" } else { "Edit" })
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Small)
                        .on_press({
                            let dialog = analog_dialog.clone();
                            let selected_var = selected_var.clone();
                            move || {
                                populate_analog_dialog(&dialog, &selected_var);
                                dialog.visible.set(true);
                            }
                        })
                        .build(),
                ),
        )
}

fn format_analog_signal_value(value: SignalValue) -> String {
    match value {
        SignalValue::Present(raw) => format_numeric_label(raw.trim().parse::<f64>().ok()),
        SignalValue::Missing => "N/A".to_string(),
        SignalValue::Loading => "Loading...".to_string(),
    }
}

fn format_numeric_label(value: Option<f64>) -> String {
    match value {
        Some(value) if value.is_finite() => {
            let mut text = format!("{value:.6}");
            while text.contains('.') && text.ends_with('0') {
                text.pop();
            }
            if text.ends_with('.') {
                text.pop();
            }
            if text.is_empty() {
                "0".to_string()
            } else {
                text
            }
        }
        _ => "-".to_string(),
    }
}

fn analog_limits_summary(limits: &AnalogLimits) -> String {
    if limits.auto {
        "Auto range".to_string()
    } else {
        format!(
            "{} .. {}",
            format_numeric_label(Some(limits.min)),
            format_numeric_label(Some(limits.max))
        )
    }
}

fn populate_analog_dialog(dialog: &AnalogLimitsDialogState, selected_var: &SelectedVariable) {
    let limits = selected_var
        .analog_limits
        .clone()
        .unwrap_or_else(AnalogLimits::auto);
    dialog.error_message.set(None);
    dialog
        .target_unique_id
        .set(Some(selected_var.unique_id.clone()));
    dialog
        .target_label
        .set(selected_var.variable_name().unwrap_or_default());
    dialog.auto.set(limits.auto);
    dialog.min_input.set(if limits.auto {
        String::new()
    } else {
        format_numeric_label(Some(limits.min))
    });
    dialog.max_input.set(if limits.auto {
        String::new()
    } else {
        format_numeric_label(Some(limits.max))
    });
}

fn group_name_dialog(
    selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
    dialog: GroupDialogState,
) -> impl Element {
    let close_dialog = dialog.clone();
    let confirm_action = {
        let selected_variables = selected_variables.clone();
        let app_config = app_config.clone();
        let dialog = dialog.clone();
        Rc::new(move || {
            let name = dialog.name_input.get_cloned().trim().to_string();
            if name.is_empty() {
                return;
            }
            if let Some(index) = dialog.editing_index.get() {
                selected_variables.rename_group(index, name);
            } else {
                selected_variables.create_group(name);
            }
            app_config
                .signal_groups_config
                .set(selected_variables.signal_groups_as_config());
            dialog.visible.set(false);
        })
    };

    centered_modal(
        move || close_dialog.visible.set(false),
        Column::new()
            .s(Width::exact(340))
            .s(Padding::all(20))
            .s(Gap::new().y(16))
            .item(
                El::new()
                    .s(Font::new()
                        .size(14)
                        .weight(FontWeight::SemiBold)
                        .color_signal(neutral_11()))
                    .child_signal(dialog.editing_index.signal().map(|editing| {
                        if editing.is_some() {
                            "Rename Group".to_string()
                        } else {
                            "Create Group".to_string()
                        }
                    })),
            )
            .item(
                input()
                    .size(InputSize::Small)
                    .placeholder("Group name")
                    .value_signal(dialog.name_input.signal_cloned())
                    .on_change({
                        let dialog = dialog.clone();
                        move |text| dialog.name_input.set(text)
                    })
                    .build(),
            )
            .item(
                Row::new()
                    .s(Align::new().right())
                    .s(Gap::new().x(SPACING_8))
                    .item(
                        button()
                            .label("Cancel")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .on_press({
                                let dialog = dialog.clone();
                                move || dialog.visible.set(false)
                            })
                            .build(),
                    )
                    .item(
                        button()
                            .label_signal(dialog.editing_index.signal().map(|editing| {
                                if editing.is_some() {
                                    "Rename".to_string()
                                } else {
                                    "Create".to_string()
                                }
                            }))
                            .size(ButtonSize::Small)
                            .on_press(move || confirm_action())
                            .build(),
                    ),
            ),
    )
}

fn analog_limits_dialog(
    selected_variables: crate::selected_variables::SelectedVariables,
    dialog: AnalogLimitsDialogState,
) -> impl Element {
    let close_dialog = dialog.clone();
    let confirm_action = {
        let selected_variables = selected_variables.clone();
        let dialog = dialog.clone();
        Rc::new(move || {
            let Some(unique_id) = dialog.target_unique_id.get_cloned() else {
                dialog.visible.set(false);
                return;
            };

            if dialog.auto.get() {
                selected_variables.update_analog_limits(&unique_id, Some(AnalogLimits::auto()));
                dialog.error_message.set(None);
                dialog.visible.set(false);
                return;
            }

            let min = dialog.min_input.get_cloned().trim().parse::<f64>().ok();
            let max = dialog.max_input.get_cloned().trim().parse::<f64>().ok();
            let Some(min) = min else {
                dialog
                    .error_message
                    .set(Some("Enter a valid minimum.".to_string()));
                return;
            };
            let Some(max) = max else {
                dialog
                    .error_message
                    .set(Some("Enter a valid maximum.".to_string()));
                return;
            };
            if !min.is_finite() || !max.is_finite() || min >= max {
                dialog.error_message.set(Some(
                    "Manual limits require finite numbers where min < max.".to_string(),
                ));
                return;
            }

            selected_variables
                .update_analog_limits(&unique_id, Some(AnalogLimits::manual(min, max)));
            dialog.error_message.set(None);
            dialog.visible.set(false);
        })
    };

    centered_modal(
        move || close_dialog.visible.set(false),
        Column::new()
            .s(Width::exact(360))
            .s(Padding::all(20))
            .s(Gap::new().y(16))
            .item(
                El::new()
                    .s(Font::new()
                        .size(14)
                        .weight(FontWeight::SemiBold)
                        .color_signal(neutral_11()))
                    .child_signal(
                        dialog
                            .target_label
                            .signal_cloned()
                            .map(|label| format!("Analog Limits: {label}")),
                    ),
            )
            .item(
                Row::new()
                    .s(Align::new().center_y())
                    .s(Gap::new().x(SPACING_8))
                    .item(
                        button()
                            .label_signal(dialog.auto.signal().map(|auto| {
                                if auto {
                                    "Auto range".to_string()
                                } else {
                                    "Manual range".to_string()
                                }
                            }))
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .on_press({
                                let dialog = dialog.clone();
                                move || {
                                    let next = !dialog.auto.get();
                                    dialog.auto.set(next);
                                    dialog.error_message.set(None);
                                }
                            })
                            .build(),
                    )
                    .item(
                        El::new()
                            .s(Font::new().size(11).color_signal(neutral_8()))
                            .child("Switch to manual only when you want fixed Y-axis bounds."),
                    ),
            )
            .item_signal(dialog.auto.signal().map({
                let dialog = dialog.clone();
                move |auto| {
                    if auto {
                        Some(
                            El::new()
                                .s(Font::new().size(12).color_signal(neutral_8()))
                                .child("Auto mode uses only the visible waveform window.")
                                .into_raw(),
                        )
                    } else {
                        Some(
                            Column::new()
                                .s(Gap::new().y(SPACING_8))
                                .item(
                                    input()
                                        .size(InputSize::Small)
                                        .placeholder("Min")
                                        .value_signal(dialog.min_input.signal_cloned())
                                        .on_change({
                                            let dialog = dialog.clone();
                                            move |text| dialog.min_input.set(text)
                                        })
                                        .build(),
                                )
                                .item(
                                    input()
                                        .size(InputSize::Small)
                                        .placeholder("Max")
                                        .value_signal(dialog.max_input.signal_cloned())
                                        .on_change({
                                            let dialog = dialog.clone();
                                            move |text| dialog.max_input.set(text)
                                        })
                                        .build(),
                                )
                                .into_raw(),
                        )
                    }
                }
            }))
            .item_signal(dialog.error_message.signal_cloned().map(|message| {
                message.map(|message| {
                    El::new()
                        .s(Font::new().size(12).color("oklch(57% 0.2 27)"))
                        .child(message)
                        .into_raw()
                })
            }))
            .item(
                Row::new()
                    .s(Align::new().right())
                    .s(Gap::new().x(SPACING_8))
                    .item(
                        button()
                            .label("Cancel")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .on_press({
                                let dialog = dialog.clone();
                                move || dialog.visible.set(false)
                            })
                            .build(),
                    )
                    .item(
                        button()
                            .label("Apply")
                            .size(ButtonSize::Small)
                            .on_press(move || confirm_action())
                            .build(),
                    ),
            ),
    )
}

fn marker_manager_dialog(
    timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: crate::config::AppConfig,
    dialog_visible: Mutable<bool>,
) -> impl Element {
    let close_dialog_visible = dialog_visible.clone();
    let markers_signal = timeline
        .markers
        .signal_vec_cloned()
        .to_signal_cloned()
        .map(|markers| {
            let mut indexed: Vec<(usize, crate::visualizer::timeline::timeline_actor::Marker)> =
                markers.into_iter().enumerate().collect();
            indexed.sort_by_key(|(_, marker)| marker.time_ps);
            indexed
        });

    centered_modal(
        move || close_dialog_visible.set(false),
        Column::new()
            .s(Width::exact(520))
            .s(Height::exact(420))
            .s(Padding::all(20))
            .s(Gap::new().y(16))
            .item(
                Row::new()
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new()
                                .size(14)
                                .weight(FontWeight::SemiBold)
                                .color_signal(neutral_11()))
                            .child("Markers"),
                    )
                    .item(El::new().s(Width::growable()))
                    .item(
                        button()
                            .label("Close")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .on_press({
                                let dialog_visible = dialog_visible.clone();
                                move || dialog_visible.set(false)
                            })
                            .build(),
                    ),
            )
            .item(
                El::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .s(Scrollbars::both())
                    .child_signal(markers_signal.map(move |markers| {
                        if markers.is_empty() {
                            Column::new()
                                .s(Width::fill())
                                .s(Align::center())
                                .s(Gap::new().y(SPACING_8))
                                .item(
                                    El::new()
                                        .s(Font::new().size(13).color_signal(neutral_8()))
                                        .child("No markers yet. Press M to add one at the cursor."),
                                )
                                .into_raw()
                        } else {
                            Column::new()
                                .s(Width::fill())
                                .s(Gap::new().y(SPACING_8))
                                .items(markers.into_iter().enumerate().map(
                                    |(sorted_index, (original_index, marker))| {
                                        let name_input = Mutable::new(marker.name.clone());
                                        Row::new()
                                            .s(Width::fill())
                                            .s(Align::new().center_y())
                                            .s(Gap::new().x(SPACING_8))
                                            .item(
                                                El::new()
                                                    .s(Font::new()
                                                        .size(11)
                                                        .color_signal(neutral_8()))
                                                    .s(Width::exact(26))
                                                    .child(format!("{}", sorted_index + 1)),
                                            )
                                            .item(
                                                El::new()
                                                    .s(Font::new()
                                                        .size(12)
                                                        .color_signal(neutral_8())
                                                        .no_wrap())
                                                    .s(Width::exact(90))
                                                    .child(format_time_with_range(
                                                        marker.time_ps / 1_000,
                                                        1_000_000_000,
                                                    )),
                                            )
                                            .item(
                                                input()
                                                    .size(InputSize::Small)
                                                    .value_signal(name_input.signal_cloned())
                                                    .on_change({
                                                        let name_input = name_input.clone();
                                                        move |text| name_input.set(text)
                                                    })
                                                    .build(),
                                            )
                                            .item(
                                                button()
                                                    .label("Jump")
                                                    .variant(ButtonVariant::Ghost)
                                                    .size(ButtonSize::Small)
                                                    .on_press({
                                                        let timeline = timeline.clone();
                                                        move || {
                                                            timeline.jump_to_marker(sorted_index)
                                                        }
                                                    })
                                                    .build(),
                                            )
                                            .item(
                                                button()
                                                    .label("Save")
                                                    .variant(ButtonVariant::Ghost)
                                                    .size(ButtonSize::Small)
                                                    .on_press({
                                                        let timeline = timeline.clone();
                                                        let app_config = app_config.clone();
                                                        let name_input = name_input.clone();
                                                        move || {
                                                            timeline.rename_marker(
                                                                original_index,
                                                                name_input
                                                                    .get_cloned()
                                                                    .trim()
                                                                    .to_string(),
                                                            );
                                                            app_config
                                                                .markers_config
                                                                .set(timeline.markers_as_config());
                                                        }
                                                    })
                                                    .build(),
                                            )
                                            .item(
                                                button()
                                                    .label("Delete")
                                                    .variant(ButtonVariant::Ghost)
                                                    .size(ButtonSize::Small)
                                                    .on_press({
                                                        let timeline = timeline.clone();
                                                        let app_config = app_config.clone();
                                                        move || {
                                                            timeline.remove_marker(original_index);
                                                            app_config
                                                                .markers_config
                                                                .set(timeline.markers_as_config());
                                                        }
                                                    })
                                                    .build(),
                                            )
                                    },
                                ))
                                .into_raw()
                        }
                    })),
            ),
    )
}

fn centered_modal(
    close_action: impl Fn() + 'static,
    content: impl Element + 'static,
) -> impl Element {
    let close_action = Rc::new(close_action);

    El::new()
        .s(Background::new().color("rgba(0, 0, 0, 0.75)"))
        .s(Width::fill())
        .s(Height::fill())
        .update_raw_el(|raw_el| {
            raw_el
                .style("display", "flex")
                .style("position", "fixed")
                .style("inset", "0")
                .style("z-index", "22000")
                .style("justify-content", "center")
                .style("align-items", "center")
        })
        .update_raw_el({
            let close_action = close_action.clone();
            move |raw_el| raw_el.event_handler(move |_: zoon::events::Click| close_action())
        })
        .child(
            El::new()
                .s(RoundedCorners::all(8))
                .s(Background::new().color_signal(moonzoon_novyui::tokens::color::neutral_2()))
                .s(Borders::all_signal(
                    moonzoon_novyui::tokens::color::neutral_4()
                        .map(|color| Border::new().width(1).color(color)),
                ))
                .update_raw_el(|raw_el| {
                    raw_el.event_handler(|event: zoon::events::Click| event.stop_propagation())
                })
                .update_raw_el({
                    let close_action = close_action.clone();
                    move |raw_el| {
                        raw_el.global_event_handler(move |event: zoon::events::KeyDown| match event
                            .key()
                            .as_str()
                        {
                            "Escape" => close_action(),
                            _ => {}
                        })
                    }
                })
                .child(content),
        )
}
