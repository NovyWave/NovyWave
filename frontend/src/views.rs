use zoon::*;
use zoon::events::{Click, KeyDown};
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::{Theme, toggle_theme, theme};
use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_9, neutral_10, neutral_11, neutral_12, primary_3, primary_6, primary_7};
use shared::{ScopeData, UpMsg, TrackedFile};
use crate::types::{get_variables_from_tracked_files, filter_variables_with_context};
use crate::virtual_list::virtual_variables_list;
use crate::config;
use std::collections::{HashSet, HashMap};
use crate::{
    IS_DOCKED_TO_BOTTOM, FILES_PANEL_WIDTH, FILES_PANEL_HEIGHT,
    VERTICAL_DIVIDER_DRAGGING, HORIZONTAL_DIVIDER_DRAGGING,
    VARIABLES_NAME_COLUMN_WIDTH, VARIABLES_VALUE_COLUMN_WIDTH,
    VARIABLES_NAME_DIVIDER_DRAGGING, VARIABLES_VALUE_DIVIDER_DRAGGING,
    VARIABLES_SEARCH_FILTER, SHOW_FILE_DIALOG, IS_LOADING,
    LOADED_FILES, SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, EXPANDED_SCOPES,
    FILE_PATHS, show_file_paths_dialog, LOAD_FILES_VIEWPORT_Y,
    FILE_PICKER_EXPANDED, FILE_PICKER_SELECTED,
    FILE_PICKER_ERROR, FILE_PICKER_ERROR_CACHE, FILE_TREE_CACHE, send_up_msg, DOCK_TOGGLE_IN_PROGRESS,
    TRACKED_FILES, state, file_validation::validate_file_state
};
use crate::state::{SELECTED_VARIABLES, clear_selected_variables, remove_selected_variable};

fn variables_vertical_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down(move || is_dragging.set_neq(true))
}

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}


pub fn file_paths_dialog() -> impl Element {
    let close_dialog = move || {
        SHOW_FILE_DIALOG.set(false);
        // Clear file picker state on close
        FILE_PICKER_SELECTED.lock_mut().clear();
        FILE_PICKER_ERROR.set_neq(None);
        // DON'T clear error cache - preserve error state for next dialog opening
        // This ensures users see error indicators immediately on subsequent opens
    };


    El::new()
        .s(Background::new().color_signal(theme().map(|t| match t {
            Theme::Light => "rgba(255, 255, 255, 0.8)",  // Light overlay
            Theme::Dark => "rgba(0, 0, 0, 0.8)",          // Dark overlay
        })))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .s(Padding::all(40))
        .update_raw_el(|raw_el| {
            raw_el
                .style("display", "flex")
                .style("justify-content", "center")
                .style("align-items", "center")
        })
        // Overlay click handler and keyboard event handler
        .update_raw_el({
            let close_dialog = close_dialog.clone();
            move |raw_el| {
                raw_el
                    .event_handler({
                        let close_dialog = close_dialog.clone();
                        move |_event: Click| {
                            close_dialog();
                        }
                    })
                    .global_event_handler({
                        let close_dialog = close_dialog.clone();
                        move |event: KeyDown| {
                            if SHOW_FILE_DIALOG.get() {  // Only handle when dialog is open
                                if event.key() == "Escape" {
                                    close_dialog();
                                } else if event.key() == "Enter" {
                                    process_file_picker_selection();
                                }
                            }
                        }
                    })
            }
        })
        .child(
            El::new()
                .s(Background::new().color_signal(neutral_2()))
                .s(RoundedCorners::all(8))
                .s(Borders::all_signal(neutral_4().map(|color| {
                    Border::new().width(1).color(color)
                })))
                .s(Padding::all(16))
                .s(Width::fill().min(500).max(600))
                .s(Height::fill().max(800))
                // Prevent event bubbling for dialog content clicks
                .update_raw_el(|raw_el| {
                    raw_el
                        .event_handler(|event: Click| {
                            event.stop_propagation();
                        })
                })
                .child(
                    Column::new()
                        .s(Height::fill())
                        .s(Gap::new().y(16))
                        .item(
                            Row::new()
                                .s(Gap::new().x(4))
                                .item(
                                    El::new()
                                        .s(Font::new().size(16).weight(FontWeight::Bold).color_signal(neutral_12()))
                                        .child("Select Waveform Files ")
                                )
                                .item(
                                    El::new()
                                        .s(Font::new().size(16).weight(FontWeight::Bold).color_signal(neutral_8()))
                                        .child("(*.vcd, *.fst)")
                                )
                        )
                        .item(
                            El::new()
                                .s(Height::fill())
                                .s(Background::new().color_signal(neutral_1()))
                                .s(Borders::all_signal(neutral_4().map(|color| {
                                    Border::new().width(1).color(color)
                                })))
                                .s(RoundedCorners::all(4))
                                .s(Padding::all(8))
                                .update_raw_el(|raw_el| {
                                    raw_el
                                        .style("min-height", "0")      // Allow flex shrinking
                                        .style("overflow-x", "auto")   // Enable horizontal scroll
                                        .style("overflow-y", "hidden") // Prevent double scrollbars
                                })
                                .child(file_picker_content())
                        )
                        .item(selected_files_display())
                        .item(
                            Row::new()
                                .s(Gap::new().x(12))
                                .s(Align::new().right())
                                .item(
                                    button()
                                        .label("Cancel")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Small)
                                        .on_press(close_dialog)
                                        .build()
                                )
                                .item(
                                    load_files_picker_button()
                                )
                        )
                )
        )
}

#[allow(dead_code)]
pub fn app_header() -> impl Element {
    Row::new()
        .s(Height::exact(40))
        .s(Width::fill())
        .s(Background::new().color_signal(neutral_2()))
        .s(Borders::new().bottom_signal(neutral_4().map(|color| {
            Border::new().width(1).color(color)
        })))
        .s(Padding::new().x(16).y(8))
        .item(
            Row::new()
                .s(Gap::new().x(8))
                .s(Align::center())
                .item(
                    button()
                        .label("📁 Load files")
                        .variant(ButtonVariant::Secondary)
                        .size(ButtonSize::Small)
                        .on_press(|| show_file_paths_dialog())
                        .build()
                )
        )
        .item(
            El::new()
                .s(Width::fill())
        )
}

#[allow(dead_code)]
pub fn docked_layout() -> impl Element {
    Column::new()
        .s(Height::fill())
        .s(Width::fill())
        .item(
            Row::new()
                .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
                .s(Width::fill())
                .item(files_panel_docked())
                .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                .item(variables_panel_docked())
        )
        .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
        .item(selected_variables_with_waveform_panel())
}

#[allow(dead_code)]
pub fn undocked_layout() -> impl Element {
    Row::new()
        .s(Height::fill())
        .s(Width::fill())
        .item(
            Column::new()
                .s(Width::exact_signal(FILES_PANEL_WIDTH.signal()))
                .s(Height::fill())
                .item(files_panel_with_height())
                .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                .item(variables_panel_with_fill())
        )
        .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
        .item(selected_variables_with_waveform_panel())
}

pub fn files_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Files & Scopes")
                    )
                    .item(
                        El::new()
                            .s(Width::growable())
                    )
                    .item(
                        load_files_button_with_progress(
                            ButtonVariant::Outline,
                            ButtonSize::Small,
                            Some(IconName::Folder)
                        )
                    )
                    .item(
                        El::new()
                            .s(Width::growable())
                    )
                    .item(
                        clear_all_files_button()
                    ),
                Column::new()
                    .s(Gap::new().y(4))
                    .s(Padding::new().top(4).right(4))
                    .s(Height::fill())
                    .s(Width::growable())
                    .item(
                        El::new()
                            .s(Height::fill())
                            .s(Width::growable())
                            .child_signal(
                                TRACKED_FILES.signal_vec_cloned()
                                    .to_signal_map(|tracked_files: &[TrackedFile]| {
                                        let tree_data = convert_tracked_files_to_tree_data(&tracked_files);
                                        
                                        if tree_data.is_empty() {
                                            empty_state_hint("Click 'Load Files' to add waveform files.")
                                                .unify()
                                        } else {
                                            tree_view()
                                                .data(tree_data)
                                                .size(TreeViewSize::Medium)
                                                .variant(TreeViewVariant::Basic)
                                                .show_icons(true)
                                                .show_checkboxes(true)
                                                .show_checkboxes_on_scopes_only(true)
                                                .single_scope_selection(true)
                                                .external_expanded(EXPANDED_SCOPES.clone())
                                                .external_selected(TREE_SELECTED_ITEMS.clone())
                                                .build()
                                                .unify()
                                        }
                                    })
                            )
                    )
            )
        )
}

pub fn variables_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Width::fill())
                    .s(Gap::new().x(8))
                    .s(Align::new().left().center_y())
                    .item(
                        El::new()
                            .s(Font::new().no_wrap())
                            .child("Variables")
                    )
                    .item(
                        El::new()
                            .s(Font::new().no_wrap().color_signal(neutral_8()).size(13))
                            .child_signal(
                                map_ref! {
                                    let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
                                    let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned() =>
                                    {
                                        if let Some(scope_id) = selected_scope_id {
                                            let variables = get_variables_from_tracked_files(&scope_id);
                                            let filtered_variables = filter_variables_with_context(&variables, &search_filter);
                                            filtered_variables.len().to_string()
                                        } else {
                                            "0".to_string()
                                        }
                                    }
                                }
                            )
                    )
                    .item(
                        El::new()
                            .s(Width::fill().max(230))
                            .s(Align::new().right())
                            .child(
                                input()
                                    .placeholder("variable_name")
                                    .value_signal(VARIABLES_SEARCH_FILTER.signal_cloned())
                                    .left_icon(IconName::Search)
                                    .right_icon_signal(VARIABLES_SEARCH_FILTER.signal_cloned().map(|text| {
                                        if text.is_empty() { None } else { Some(IconName::X) }
                                    }))
                                    .on_right_icon_click(|| VARIABLES_SEARCH_FILTER.set_neq(String::new()))
                                    .size(InputSize::Small)
                                    .on_change(|text| VARIABLES_SEARCH_FILTER.set_neq(text))
                                    .build()
                            )
                    ),
                simple_variables_content()
            )
        )
}

pub fn selected_variables_with_waveform_panel() -> impl Element {
    Column::new()
        .s(Width::growable())
        .s(Height::fill())
        .item(
            El::new()
                .s(Width::growable())
                .s(Height::fill())
                .child(
                    create_panel(
                        Row::new()
                            .s(Gap::new().x(8))
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
                                theme_toggle_button()
                            )
                            .item(
                                dock_toggle_button()
                            )
                            .item(
                                El::new()
                                    .s(Width::growable())
                            )
                            .item(
                                clear_all_variables_button()
                            ),
                        // Resizable columns layout with draggable separators
                        El::new()
                            .s(Height::exact_signal(
                                SELECTED_VARIABLES.signal_vec_cloned().to_signal_map(|vars| {
                                    let row_height = 40u32;
                                    let computed_height = vars.len() as u32 * row_height;
                                    computed_height
                                })
                            ))
                            .s(Width::fill())
                            .s(Scrollbars::both())
                            .child(
                                Row::new()
                                    .s(Height::fill())
                                    .s(Width::fill())
                                    .s(Align::new().top())
                                    .item(
                                        // Column 1: Variable name (resizable)
                                        Column::new()
                                            .s(Width::exact_signal(VARIABLES_NAME_COLUMN_WIDTH.signal()))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .items_signal_vec(
                                                SELECTED_VARIABLES.signal_vec_cloned().map(|selected_var| {
                                                    Row::new()
                                                        .s(Height::exact(40))
                                                        .s(Width::fill())
                                                        .s(Padding::all(8))
                                                        .s(Borders::new().bottom_signal(neutral_4().map(|color| 
                                                            Border::new().width(1).color(color)
                                                        )))
                                                        .s(Gap::new().x(8))
                                                        .item({
                                                            let unique_id = selected_var.unique_id.clone();
                                                            button()
                                                                .left_icon(IconName::X)
                                                                .variant(ButtonVariant::DestructiveGhost)
                                                                .size(ButtonSize::Small)
                                                                .on_press(move || {
                                                                    remove_selected_variable(&unique_id);
                                                                })
                                                                .build()
                                                        })
                                                        .item(
                                                            El::new()
                                                                .s(Font::new().color_signal(neutral_11()).size(13).no_wrap())
                                                                .child(&selected_var.variable_name().unwrap_or_default())
                                                                .update_raw_el({
                                                                    let full_info = format!("{}: {}.{} (type unknown)", 
                                                                        selected_var.file_name().unwrap_or_default(), 
                                                                        selected_var.scope_path().unwrap_or_default(), 
                                                                        selected_var.variable_name().unwrap_or_default()
                                                                    );
                                                                    move |raw_el| {
                                                                        raw_el.attr("title", &full_info)
                                                                    }
                                                                })
                                                        )
                                                })
                                            )
                                    )
                                    .item(variables_vertical_divider(VARIABLES_NAME_DIVIDER_DRAGGING.clone()))
                                    .item(
                                        // Column 2: Variable value (resizable) - HEIGHT FOLLOWER
                                        Column::new()
                                            .s(Width::exact_signal(VARIABLES_VALUE_COLUMN_WIDTH.signal()))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .items_signal_vec(
                                                SELECTED_VARIABLES.signal_vec_cloned().map(|_selected_var| {
                                                    El::new()
                                                        .s(Height::exact(40))
                                                        .s(Width::fill())
                                                        .s(Padding::all(8))
                                                        .s(Borders::new().bottom_signal(neutral_4().map(|color| 
                                                            Border::new().width(1).color(color)
                                                        )))
                                                        .s(Font::new().color_signal(neutral_9()).size(13).no_wrap())
                                                        .child("Value")
                                                })
                                            )
                                    )
                                    .item(variables_vertical_divider(VARIABLES_VALUE_DIVIDER_DRAGGING.clone()))
                                    .item(
                                        // Column 3: Unified waveform canvas (fills remaining space) - HEIGHT FOLLOWER
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .s(Background::new().color_signal(neutral_2()))
                                            .child(
                                                El::new()
                                                    .s(Padding::all(20))
                                                    .s(Font::new().color_signal(neutral_8()))
                                                    .child("Unified Waveform Canvas")
                                            )
                                    )
                            )
                    )
                )
        )
}

#[allow(dead_code)]
pub fn selected_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(10))
                    .item(
                        Text::new("Selected Variables")
                    )
                    .item(
                        dock_toggle_button()
                    ),
                Column::new()
                    .s(Gap::new().y(8))
                    .s(Padding::all(16))
                    .item(
                        Row::new()
                            .s(Gap::new().x(8))
                            .s(Align::new().center_y())
                            .item("⋮⋮")
                            .item(
                                El::new()
                                    .s(Font::new().color_signal(neutral_10()).size(14))
                                    .child("clock")
                            )
                            .item(
                                button()
                                    .label("×")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Small)
                                    .on_press(|| {})
                                    .build()
                            )
                    )
                    .item(
                        Row::new()
                            .s(Gap::new().x(8))
                            .s(Align::new().center_y())
                            .item("⋮⋮")
                            .item(
                                El::new()
                                    .s(Font::new().color_signal(neutral_10()).size(14))
                                    .child("reset")
                            )
                            .item(
                                button()
                                    .label("×")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Small)
                                    .on_press(|| {})
                                    .build()
                            )
                    )
            )
        )
}

#[allow(dead_code)]
pub fn waveform_panel() -> impl Element {
    El::new()
        .s(Width::fill().min(500))
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(10))
                    .item(
                        Text::new("Waveform")
                    )
                    .item(
                        button()
                            .label("Zoom In")
                            .left_icon(IconName::ZoomIn)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| {})
                            .build()
                    )
                    .item(
                        button()
                            .label("Zoom Out")
                            .left_icon(IconName::ZoomOut)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| {})
                            .build()
                    ),
                Column::new()
                    .s(Gap::new().y(16))
                    .s(Padding::all(16))
                    .item(
                        Row::new()
                            .s(Gap::new().x(20))
                            .item("0s")
                            .item("10s")
                            .item("20s")
                            .item("30s")
                            .item("40s")
                            .item("50s")
                    )
                    .item(
                        El::new()
                            .s(Background::new().color_signal(neutral_1()))
                            .s(Height::exact(200))
                            .s(Width::fill())
                            .s(Align::center())
                            .s(RoundedCorners::all(4))
                            .child(
                                El::new()
                                    .s(Font::new().color_signal(neutral_8()).size(16))
                                    .child("Waveform display area")
                            )
                    )
            )
        )
}

// Helper functions for different panel configurations

pub fn files_panel_with_height() -> impl Element {
    El::new()
        .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
        .s(Width::growable())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child(files_panel())
}

pub fn variables_panel_with_fill() -> impl Element {
    El::new()
        .s(Width::growable())
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child(variables_panel())
}

pub fn files_panel_docked() -> impl Element {
    El::new()
        .s(Width::exact_signal(FILES_PANEL_WIDTH.signal()))
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child(files_panel())
}

pub fn variables_panel_docked() -> impl Element {
    El::new()
        .s(Width::growable())
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child(variables_panel())
}

// Supporting functions
fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::growable())
        .s(Background::new().color_signal(neutral_2()))
        .s(Borders::all_signal(neutral_4().map(|color| {
            Border::new().width(1).color(color)
        })))
        .child(
            Column::new()
                .s(Height::fill())
                .item(
                    El::new()
                        .s(Padding::new().x(12).y(4))
                        .s(Background::new().color_signal(neutral_4()))
                        .s(Borders::new().bottom_signal(neutral_4().map(|color| {
                            Border::new().width(1).color(color)
                        })))
                        .s(Font::new().weight(FontWeight::SemiBold).size(14).color_signal(neutral_11()))
                        .child(header_content)
                )
                .item(
                    El::new()
                        .s(Height::fill())
                        .s(Width::fill())
                        .s(Scrollbars::both())
                        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style("overflow-x", "auto")
                .style("min-height", "0")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
                        .child(content)
                )
        )
}

fn simple_variables_content() -> impl Element {
    Column::new()
        .s(Gap::new().y(0))
        .s(Height::fill())
        .s(Width::fill())
        .item(
            El::new()
                .s(Height::fill())
                .s(Width::fill())
                .child_signal(
                    map_ref! {
                        let selected_scope_id = SELECTED_SCOPE_ID.signal_ref(|id| id.clone()),
                        let search_filter = VARIABLES_SEARCH_FILTER.signal_cloned() =>
                        {
                            if let Some(scope_id) = selected_scope_id {
                                let variables = get_variables_from_tracked_files(&scope_id);
                                virtual_variables_list(variables, search_filter.clone()).into_element()
                            } else {
                                virtual_variables_list(Vec::new(), "Select a scope to view variables".to_string()).into_element()
                            }
                        }
                    }
                )
        )
}


// Removed create_styled_smart_label function - styling now handled inline in TreeView component

// Parse smart label to separate prefix from filename for sorting purposes
fn parse_smart_label_for_sorting(smart_label: &str) -> (String, String) {
    if let Some(last_slash) = smart_label.rfind('/') {
        let prefix = &smart_label[..last_slash]; // Exclude trailing slash for sorting
        let filename = &smart_label[last_slash + 1..];
        (prefix.to_string(), filename.to_string())
    } else {
        ("".to_string(), smart_label.to_string()) // No prefix, just filename
    }
}

// Enhanced conversion function using TrackedFile with smart labels, tooltips, and error states
fn convert_tracked_files_to_tree_data(tracked_files: &[TrackedFile]) -> Vec<TreeViewItemData> {
    // Sort files: primary by filename, secondary by prefix for better organization
    let mut file_refs: Vec<&TrackedFile> = tracked_files.iter().collect();
    file_refs.sort_by(|a, b| {
        // Extract filename (part after last slash) and prefix (part before last slash)
        let (a_prefix, a_filename) = parse_smart_label_for_sorting(&a.smart_label);
        let (b_prefix, b_filename) = parse_smart_label_for_sorting(&b.smart_label);
        
        // Primary sort: filename (case-insensitive)
        let filename_cmp = a_filename.to_lowercase().cmp(&b_filename.to_lowercase());
        if filename_cmp != std::cmp::Ordering::Equal {
            return filename_cmp;
        }
        
        // Secondary sort: prefix (case-insensitive)
        a_prefix.to_lowercase().cmp(&b_prefix.to_lowercase())
    });
    
    file_refs.iter().map(|tracked_file| {
        match &tracked_file.state {
            shared::FileState::Loaded(waveform_file) => {
                // Successfully loaded file - show with scopes
                let children = waveform_file.scopes.iter().map(|scope| {
                    convert_scope_to_tree_data(scope)
                }).collect();
                
                TreeViewItemData::new(tracked_file.id.clone(), tracked_file.smart_label.clone())
                    .item_type(TreeViewItemType::File)
                    .tooltip(tracked_file.path.clone()) // Full path on hover
                    // Smart label styling now handled inline in TreeView component
                    .with_children(children)
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            }
            shared::FileState::Loading(status) => {
                // File currently loading - show with loading indicator
                let status_text = match status {
                    shared::LoadingStatus::Starting => "Starting...",
                    shared::LoadingStatus::Parsing => "Parsing...",
                    shared::LoadingStatus::Completed => "Completed",
                    shared::LoadingStatus::Error(_) => "Error",
                };
                
                TreeViewItemData::new(tracked_file.id.clone(), format!("{} ({})", tracked_file.smart_label, status_text))
                    .item_type(TreeViewItemType::File)
                    .tooltip(tracked_file.path.clone())
                    .disabled(true) // Disable interaction while loading
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            }
            shared::FileState::Failed(error) => {
                // File failed to load - show with error styling
                let error_message = error.user_friendly_message();
                
                TreeViewItemData::new(tracked_file.id.clone(), tracked_file.smart_label.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon(error.icon_name())
                    .tooltip(format!("{}\nError: {}", tracked_file.path, error_message))
                    .error_message(error_message.clone())
                    // Smart label styling now handled inline in TreeView component
                    .with_children(vec![
                        TreeViewItemData::new(format!("{}_error_detail", tracked_file.id), error_message)
                            .item_type(TreeViewItemType::Default)
                            .disabled(true)
                    ])
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            }
            shared::FileState::Missing(path) => {
                // File no longer exists - show with missing indicator
                TreeViewItemData::new(tracked_file.id.clone(), tracked_file.smart_label.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("file")
                    .tooltip(format!("{}\nFile not found", path))
                    .error_message("File not found".to_string())
                    // Smart label styling now handled inline in TreeView component
                    .with_children(vec![
                        TreeViewItemData::new(format!("{}_missing_detail", tracked_file.id), "File no longer exists")
                            .item_type(TreeViewItemType::Default)
                            .disabled(true)
                    ])
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            }
            shared::FileState::Unsupported(reason) => {
                // Unsupported file format - show with unsupported indicator
                TreeViewItemData::new(tracked_file.id.clone(), tracked_file.smart_label.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("circle-help")
                    .tooltip(format!("{}\nUnsupported: {}", tracked_file.path, reason))
                    .error_message(format!("Unsupported: {}", reason))
                    // Smart label styling now handled inline in TreeView component
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            }
        }
    }).collect()
}


// Helper function to clean up all file-related state when a file is removed
fn cleanup_file_related_state(file_id: &str) {
    // Get filename and file path before any cleanup (needed for cleanup logic)
    let (_filename, file_path) = state::TRACKED_FILES.lock_ref()
        .iter()
        .find(|f| f.id == file_id)
        .map(|f| (f.filename.clone(), f.path.clone()))
        .unwrap_or_else(|| (String::new(), String::new()));
    
    // Clear related scope selections if removed file contained selected scope
    if let Some(selected_scope) = SELECTED_SCOPE_ID.get_cloned() {
        // New format: {full_path}|{scope} - check if scope belongs to this file
        if selected_scope == file_path || selected_scope.starts_with(&format!("{}|", file_path)) {
            SELECTED_SCOPE_ID.set(None);
        }
    }
    
    // Clear expanded scopes for this file
    // New scope ID format: {full_path}|{scope_full_name} or just {full_path}
    EXPANDED_SCOPES.lock_mut().retain(|scope| {
        // Keep scopes that don't belong to this file
        scope != &file_path && !scope.starts_with(&format!("{}|", file_path))
    });
    
    // Clear selected variables from this file
    // SelectedVariable uses full file path in new format
    if !file_path.is_empty() {
        state::SELECTED_VARIABLES.lock_mut().retain(|var| var.file_path().unwrap_or_default() != file_path);
        state::SELECTED_VARIABLES_INDEX.lock_mut().retain(|unique_id| {
            !unique_id.starts_with(&format!("{}|", file_path))
        });
    }
}

// Enhanced file removal handler that works with both old and new systems
fn create_enhanced_file_remove_handler(_file_id: String) -> impl Fn(&str) + 'static {
    move |id: &str| {
        // Clean up all file-related state
        cleanup_file_related_state(id);
        
        // Remove from new TRACKED_FILES system
        state::remove_tracked_file(id);
        
        // Remove from legacy systems during transition
        LOADED_FILES.lock_mut().retain(|f| f.id != id);
        FILE_PATHS.lock_mut().shift_remove(id);
        
        // Save file list and scope selection after removal
        config::save_file_list();
        config::save_scope_selection();
    }
}

fn convert_scope_to_tree_data(scope: &ScopeData) -> TreeViewItemData {
    let mut children = Vec::new();
    
    // Sort child scopes alphabetically by name (case-insensitive)
    let mut child_refs: Vec<&ScopeData> = scope.children.iter().collect();
    child_refs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    // Add sorted child scopes
    for child_scope in child_refs {
        children.push(convert_scope_to_tree_data(child_scope));
    }
    
    // Signals are NOT shown in Files & Scopes - they belong in the Variables panel
    
    // Add "scope_" prefix to make IDs distinguishable for TreeView logic
    let scope_tree_id = format!("scope_{}", scope.id);
    
    TreeViewItemData::new(scope_tree_id, scope.name.clone())
        .item_type(TreeViewItemType::Folder)
        .with_children(children)
}

fn load_files_button_with_progress(variant: ButtonVariant, size: ButtonSize, icon: Option<IconName>) -> impl Element {
    El::new()
        .child_signal(IS_LOADING.signal().map(move |is_loading| {
            let mut btn = button();
            
            if is_loading {
                btn = btn.label("Loading...")
                    .disabled(true);
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            } else {
                btn = btn.label("Load Files")
                    .on_press(|| show_file_paths_dialog());
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            }
            
            btn.variant(variant.clone())
                .size(size.clone())
                .build()
                .into_element()
        }))
}


fn load_files_picker_button() -> impl Element {
    button()
        .label_signal(
            map_ref! {
                let is_loading = IS_LOADING.signal(),
                let selected_count = FILE_PICKER_SELECTED.signal_vec_cloned().len() =>
                move {
                    if *is_loading {
                        "Loading...".to_string()
                    } else if *selected_count > 0 {
                        format!("Load {} Files", selected_count)
                    } else {
                        "Load Files".to_string()
                    }
                }
            }
        )
        .disabled_signal(
            map_ref! {
                let is_loading = IS_LOADING.signal(),
                let selected_count = FILE_PICKER_SELECTED.signal_vec_cloned().len() =>
                *is_loading || *selected_count == 0
            }
        )
        .on_press(|| process_file_picker_selection())
        .variant(ButtonVariant::Primary)
        .size(ButtonSize::Small)
        .build()
}


fn file_picker_content() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child(simple_file_picker_tree())
}

fn simple_file_picker_tree() -> impl Element {
    // Note: Root directory "/" is already requested by show_file_paths_dialog()
    // No need for duplicate request here
    
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Scrollbars::both())
        .viewport_y_signal(LOAD_FILES_VIEWPORT_Y.signal())
        .on_viewport_location_change(|_scene, viewport| {
            // Only update viewport Y if initialization is complete to prevent overwriting loaded scroll position
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                LOAD_FILES_VIEWPORT_Y.set_neq(viewport.y);
            } else {
            }
        })
        .update_raw_el(|raw_el| {
            raw_el
                .style("min-width", "fit-content")
                .style("width", "100%")
                .style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child_signal(
            map_ref! {
                let tree_cache = FILE_TREE_CACHE.signal_cloned(),
                let error_cache = FILE_PICKER_ERROR_CACHE.signal_cloned(),
                let expanded = FILE_PICKER_EXPANDED.signal_cloned() =>
                move {
                    monitor_directory_expansions(expanded.iter().cloned().collect::<HashSet<_>>());
                    
                    // Check if we have root directory data
                    if let Some(_root_items) = tree_cache.get("/") {
                        // Create root "/" item and build hierarchical tree
                        let tree_data = vec![
                            TreeViewItemData::new("/".to_string(), "/".to_string())
                                .with_children(build_hierarchical_tree("/", &tree_cache, &error_cache))
                        ];
                        
                        tree_view()
                            .data(tree_data)
                            .size(TreeViewSize::Medium)
                            .variant(TreeViewVariant::Basic)
                            .show_icons(true)
                            .show_checkboxes(true)
                            .external_expanded(FILE_PICKER_EXPANDED.clone())
                            .external_selected_vec(FILE_PICKER_SELECTED.clone())
                            .build()
                            .unify()
                    } else {
                        empty_state_hint("Loading directory contents...")
                            .unify()
                    }
                }
            }
        )
}

fn should_disable_folder(
    path: &str, 
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>
) -> bool {
    // Simple logic: disable folder if it has NO subfolders AND NO waveform files
    if let Some(items) = tree_cache.get(path) {
        let has_subfolders = items.iter().any(|item| item.is_directory);
        let has_waveform_files = items.iter().any(|item| !item.is_directory && item.is_waveform_file);
        
        // Only disable if BOTH conditions are false
        return !has_subfolders && !has_waveform_files;
    }
    
    // If no cached data, don't disable (allow expansion to load data)
    false
}

fn build_hierarchical_tree(
    path: &str, 
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>,
    error_cache: &HashMap<String, String>
) -> Vec<TreeViewItemData> {
    if let Some(items) = tree_cache.get(path) {
        items.iter().map(|item| {
            if item.is_directory {
                // Check if we have an error for this directory
                if let Some(_error_msg) = error_cache.get(&item.path) {
                    // Show error as a child item
                    let data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("folder".to_string())
                        .item_type(TreeViewItemType::Folder)
                        .with_children(vec![
                            TreeViewItemData::new("access_denied", &error_cache.get(&item.path).map(|err| {
                                crate::state::make_error_user_friendly(err)
                            }).unwrap_or_else(|| "Cannot access this directory".to_string()))
                                .item_type(TreeViewItemType::Default)
                                .disabled(true)
                        ]);
                    data
                } else if let Some(_children) = tree_cache.get(&item.path) {
                    // Build actual hierarchical children
                    let children = build_hierarchical_tree(&item.path, tree_cache, error_cache);
                    let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("folder".to_string())
                        .item_type(TreeViewItemType::Folder)
                        .with_children(children);
                    
                    // Show "No supported files" placeholder for empty folders instead of disabling
                    if should_disable_folder(&item.path, tree_cache) {
                        data = data.with_children(vec![
                            TreeViewItemData::new("no_supported_files", "No supported files")
                                .item_type(TreeViewItemType::Default)
                                .disabled(true)
                        ]);
                    }
                    
                    data
                } else {
                    // No cached contents - only show expand arrow if directory has expandable content
                    let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("folder".to_string())
                        .item_type(TreeViewItemType::Folder);
                    
                    // Use backend's has_expandable_content field directly
                    if item.has_expandable_content {
                        data = data.with_children(vec![
                            TreeViewItemData::new("loading", "Loading...")
                                .item_type(TreeViewItemType::Default)
                                .disabled(true)
                        ]);
                    } else {
                        // Directory has no subfolders AND no waveform files - show "No supported files" placeholder
                        data = data.with_children(vec![
                            TreeViewItemData::new("no_supported_files", "No supported files")
                                .item_type(TreeViewItemType::Default)
                                .disabled(true)
                        ]);
                    }
                    
                    data
                }
            } else {
                // File item
                let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                    .icon("file".to_string())
                    .item_type(TreeViewItemType::File)
                    .is_waveform_file(item.is_waveform_file);
                
                // Disable non-waveform files
                if !item.is_waveform_file {
                    data = data.disabled(true);
                }
                
                data
            }
        }).collect()
    } else {
        vec![]
    }
}

fn monitor_directory_expansions(expanded: HashSet<String>) {
    static LAST_EXPANDED: Lazy<Mutable<HashSet<String>>> = lazy::default();
    
    let last_expanded = LAST_EXPANDED.lock_ref().clone();
    let new_expansions: Vec<String> = expanded.difference(&last_expanded).cloned().collect();
    
    // CACHE-AWARE REQUESTS - Only request directories if not already cached
    let cache = FILE_TREE_CACHE.lock_ref();
    let paths_to_request: Vec<String> = new_expansions.into_iter()
        .filter(|path| path.starts_with("/") && !path.is_empty() && !cache.contains_key(path))
        .collect();
    drop(cache); // Release lock before sending requests
    
    // Send batch request for maximum parallel processing with jwalk
    if !paths_to_request.is_empty() {
        send_up_msg(UpMsg::BrowseDirectories(paths_to_request));
    }
    
    // Update last expanded set
    LAST_EXPANDED.set_neq(expanded);
}


fn extract_filename(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}

fn selected_files_display() -> impl Element {
    El::new()
        .s(Padding::all(4))
        .child_signal(
            FILE_PICKER_SELECTED.signal_vec_cloned().to_signal_map(|selected_paths| {
                if selected_paths.is_empty() {
                    El::new()
                        .s(Font::new().italic().color_signal(neutral_8()))
                        .child("Select waveform files from the directory tree above")
                        .unify()
                } else {
                    Row::new()
                        .multiline()
                        .s(Gap::new().x(8).y(8))
                        .s(Align::new().left().top())
                        .items(selected_paths.iter().map(|path| {
                            let filename = extract_filename(path);
                            badge(filename)
                                .variant(BadgeVariant::Outline)
                                .size(BadgeSize::Small)
                                .removable()
                                .on_remove({
                                    let path = path.clone();
                                    move || {
                                        FILE_PICKER_SELECTED.lock_mut().retain(|p| p != &path);
                                    }
                                })
                                .build()
                        }))
                        .unify()
                }
            })
        )
}

// File picker utility functions




fn process_file_picker_selection() {
    let selected_files = FILE_PICKER_SELECTED.lock_ref().to_vec();
    
    if !selected_files.is_empty() {
        IS_LOADING.set(true);
        
        // Get currently tracked file IDs for duplicate detection using new system
        let tracked_file_ids: HashSet<String> = TRACKED_FILES.lock_ref()
            .iter()
            .map(|f| f.id.clone())
            .collect();
        
        // Process each selected file path with validation
        for file_path in selected_files.iter() {
            let file_id = shared::generate_file_id(file_path);
            
            // Check for duplicates and handle reload vs new load
            if tracked_file_ids.contains(&file_id) {
                // RELOAD: Remove existing file first, then load fresh
                
                // Clean up all file-related state (scopes, variables, selections)
                cleanup_file_related_state(&file_id);
                
                // Remove from tracked files and legacy systems
                state::remove_tracked_file(&file_id);
                LOADED_FILES.lock_mut().retain(|f| f.id != file_id);
                FILE_PATHS.lock_mut().shift_remove(&file_id);
            }
            
            // CRITICAL: Validate files BEFORE sending to backend (from file picker)
            // This prevents non-existent files from being misclassified as UnsupportedFormat
            Task::start({
                let file_path = file_path.clone();
                let file_id = file_id.clone();
                async move {
                    // Validate file state before adding to tracked system
                    match validate_file_state(&file_path).await {
                        Ok(()) => {
                            // File is valid - proceed with normal loading flow
                            state::add_tracked_file(file_path.clone(), shared::FileState::Loading(shared::LoadingStatus::Starting));
                            
                            // Also maintain legacy systems for backward compatibility during transition
                            FILE_PATHS.lock_mut().insert(file_id.clone(), file_path.clone());
                            config::save_file_list();
                            send_up_msg(UpMsg::LoadWaveformFile(file_path));
                        },
                        Err(error) => {
                            // File validation failed - add with error state immediately, don't send to backend
                            zoon::println!("File validation failed for {}: {:?}", file_path, error);
                            state::add_tracked_file(file_path.clone(), shared::FileState::Failed(error));
                            
                            // Still add to legacy system for consistency, but mark as failed
                            FILE_PATHS.lock_mut().insert(file_id.clone(), file_path.clone());
                            config::save_file_list();
                            
                            // NOTE: We deliberately do NOT send UpMsg::LoadWaveformFile for failed validation
                            // This prevents the backend from wasting time on files we know are invalid
                        }
                    }
                }
            });
        }
        
        // Close dialog and clear selection
        SHOW_FILE_DIALOG.set(false);
        FILE_PICKER_SELECTED.lock_mut().clear();
        FILE_PICKER_ERROR.set_neq(None);
    }
}

fn clear_all_files() {
    zoon::println!("clear_all_files() called - clearing all loaded files");
    
    // Get all tracked file IDs before clearing
    let file_ids: Vec<String> = state::TRACKED_FILES.lock_ref()
        .iter()
        .map(|f| f.id.clone())
        .collect();
    
    zoon::println!("Found {} files to clear: {:?}", file_ids.len(), file_ids);
    
    // Clean up all file-related state for each file
    for file_id in &file_ids {
        cleanup_file_related_state(file_id);
    }
    
    // Clear all tracked files
    state::TRACKED_FILES.lock_mut().clear();
    
    // Clear legacy systems during transition
    LOADED_FILES.lock_mut().clear();
    FILE_PATHS.lock_mut().clear();
    
    // Clear any remaining scope/tree selections
    SELECTED_SCOPE_ID.set(None);
    EXPANDED_SCOPES.lock_mut().clear();
    TREE_SELECTED_ITEMS.lock_mut().clear();
    
    // Save the empty file list
    config::save_file_list();
    config::save_scope_selection();
}

fn clear_all_files_button() -> impl Element {
    button()
        .label("Clear All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(|| {
            clear_all_files();
        })
        .build()
}

fn clear_all_variables_button() -> impl Element {
    button()
        .label("Clear All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(|| {
            clear_selected_variables();
        })
        .build()
}

fn theme_toggle_button() -> impl Element {
    El::new()
        .child_signal(theme().map(|current_theme| {
            button()
                .left_icon(match current_theme {
                    Theme::Light => IconName::Moon,
                    Theme::Dark => IconName::Sun,
                })
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(|| toggle_theme())
                .build()
                .into_element()
        }))
}

fn dock_toggle_button() -> impl Element {
    El::new()
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            button()
                .label(if is_docked { "Dock to Right" } else { "Dock to Bottom" })
                .left_icon_element(|| {
                    El::new()
                        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
                            let icon_el = icon(IconName::ArrowDownToLine).size(IconSize::Small).color(IconColor::Primary).build();
                            if is_docked {
                                El::new()
                                    .s(Transform::new().rotate(-90))
                                    .child(icon_el)
                                    .into_element()
                            } else {
                                El::new().child(icon_el).into_element()
                            }
                        }))
                        .unify()
                })
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_press(|| {
                    DOCK_TOGGLE_IN_PROGRESS.set(true);
                    let new_is_docked = !IS_DOCKED_TO_BOTTOM.get();
                    
                    // Atomically switch dock mode while preserving dimensions
                    config::switch_dock_mode_preserving_dimensions(new_is_docked);
                    
                    // Update UI state after config is saved
                    IS_DOCKED_TO_BOTTOM.set_neq(new_is_docked);
                    
                    DOCK_TOGGLE_IN_PROGRESS.set(false);
                })
                .align(Align::center())
                .build()
                .into_element()
        }))
}

pub fn vertical_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down(move || is_dragging.set_neq(true))
}

pub fn horizontal_divider(is_dragging: Mutable<bool>) -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new().color_signal(
            is_dragging.signal().map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down(move || is_dragging.set_neq(true))
}

// ===== UNIFIED WAVEFORM CANVAS =====


