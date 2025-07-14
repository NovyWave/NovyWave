use zoon::*;
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::{Theme, toggle_theme, theme};
use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_9, neutral_10, neutral_11, neutral_12, primary_3, primary_6, primary_7};
use shared::{WaveformFile, ScopeData, filter_variables, UpMsg};
use crate::types::{get_variables_from_selected_scope};
use crate::virtual_list::virtual_variables_list;
use crate::config;
use std::collections::{HashSet, HashMap};
use crate::{
    IS_DOCKED_TO_BOTTOM, FILES_PANEL_WIDTH, FILES_PANEL_HEIGHT,
    VERTICAL_DIVIDER_DRAGGING, HORIZONTAL_DIVIDER_DRAGGING,
    VARIABLES_SEARCH_FILTER, SHOW_FILE_DIALOG, IS_LOADING,
    LOADED_FILES, SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, EXPANDED_SCOPES,
    FILE_PATHS, show_file_paths_dialog,
    FILE_PICKER_EXPANDED, FILE_PICKER_SELECTED,
    FILE_PICKER_ERROR, FILE_TREE_CACHE, send_up_msg, DOCK_TOGGLE_IN_PROGRESS
};

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}


pub fn file_paths_dialog() -> impl Element {
    El::new()
        .s(Background::new().color_signal(theme().map(|t| match t {
            Theme::Light => "rgba(255, 255, 255, 0.8)",  // Light overlay
            Theme::Dark => "rgba(0, 0, 0, 0.8)",          // Dark overlay
        })))
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .child(
            El::new()
                .s(Background::new().color_signal(neutral_2()))
                .s(RoundedCorners::all(8))
                .s(Borders::all_signal(neutral_4().map(|color| {
                    Border::new().width(2).color(color)
                })))
                .s(Padding::all(24))
                .s(Width::exact(700))
                .s(Height::exact(500))
                .child(
                    Column::new()
                        .s(Height::fill())
                        .s(Gap::new().y(16))
                        .item(
                            El::new()
                                .s(Font::new().size(18).weight(FontWeight::Bold).color_signal(neutral_12()))
                                .child("Browse and Select Waveform Files")
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
                                        .size(ButtonSize::Medium)
                                        .on_press(|| {
                                            SHOW_FILE_DIALOG.set(false);
                                            // Clear file picker state on cancel
                                            FILE_PICKER_SELECTED.lock_mut().clear();
                                            FILE_PICKER_ERROR.set_neq(None);
                                        })
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
                        remove_all_button()
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
                                LOADED_FILES.signal_vec_cloned()
                                    .to_signal_map(|files: &[WaveformFile]| {
                                        let tree_data = convert_files_to_tree_data(&files);
                                        
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
                                            let _loaded_files: Vec<WaveformFile> = LOADED_FILES.lock_ref().iter().cloned().collect();
                                            let variables = get_variables_from_selected_scope(&scope_id);
                                            let filtered_variables = filter_variables(&variables, &search_filter);
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
                        remove_all_button()
                    ),
                // 3-column table layout: Variable Name | Value | Waveform
                El::new()
                    .s(Height::fill())
                    .child(
                        Column::new()
                            .s(Gap::new().y(0))
                            .s(Padding::all(8))
                            .item(
                                // Timeline header
                        Row::new()
                            .s(Gap::new().x(0))
                            .s(Align::new().center_y())
                            .s(Padding::new().y(2))
                            .item(
                                // Variable Name column header
                                El::new()
                                    .s(Width::exact(250))
                                    .s(Font::new().color_signal(neutral_8()).size(12))
                                    .child("Variable")
                            )
                            .item(
                                // Value column header  
                                El::new()
                                    .s(Width::exact(60))
                                    .s(Font::new().color_signal(neutral_8()).size(12))
                                    .child("Value")
                            )
                            .item(
                                // Timeline markers for waveform column
                                Row::new()
                                    .s(Width::fill())
                                    .s(Gap::new().x(40))
                                    .s(Padding::new().x(10))
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("0s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("10s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("20s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("30s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("40s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("50s")
                                    )
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                            .child("60s")
                                    )
                            )
                    )
                    .items((0..8).map(|i| {
                        let var_names = [
                            "LsuPlugin_logic_bus_rsp_payload_error",
                            "LsuPlugin_logic_bus_rsp_payload_data",
                            "io_writes_0_payload_data", 
                            "logic_logic_onDebugCd_dmiStat_value_string",
                            "LsuPlugin_logic_bus_rsp_payload_error",
                            "LsuPlugin_logic_bus_rsp_payload_data",
                            "io_writes_0_payload_data",
                            "clk"
                        ];
                        
                        let values = ["0", "14x2106624", "0", "success", "0", "14x2106624", "0", "1"];
                        
                        // Each row: Variable Name | Value | Waveform
                        Row::new()
                            .s(Gap::new().x(0))
                            .s(Align::new().center_y())
                            .s(Padding::new().y(0))
                            .item(
                                // Variable Name column (250px width)
                                Row::new()
                                    .s(Width::exact(250))
                                    .s(Gap::new().x(8))
                                    .s(Align::new().center_y())
                                    .item("⋮⋮")
                                    .item(
                                        El::new()
                                            .s(Font::new().color_signal(neutral_11()).size(13))
                                            .child(var_names[i as usize])
                                    )
                                    )
                            .item(
                                // Value column (60px width)
                                El::new()
                                    .s(Width::exact(60))
                                    .s(Font::new().color_signal(neutral_9()).size(13))
                                    .child(values[i as usize])
                            )
                            .item(
                                // Waveform column (fills remaining width)
                                Row::new()
                                    .s(Width::fill())
                                    .s(Height::exact(20))
                                    .s(Gap::new().x(1))
                                    .s(Padding::new().x(10))
                                    .items((0..12).map(|j| {
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::exact(18))
                                            .s(Background::new().color_signal(theme().map(move |t| {
                                                match (i + j) % 3 {
                                                    0 => match t {
                                                        Theme::Light => "oklch(55% 0.13 250)", // Primary blue
                                                        Theme::Dark => "oklch(55% 0.13 250)",
                                                    },
                                                    1 => match t {
                                                        Theme::Light => "oklch(65% 0.16 250)", // Lighter blue
                                                        Theme::Dark => "oklch(65% 0.16 250)",
                                                    },
                                                    _ => match t {
                                                        Theme::Light => "oklch(97% 0.025 255)", // Light background
                                                        Theme::Dark => "oklch(18% 0.035 255)",  // Dark background
                                                    }
                                                }
                                            })))
                                            .s(RoundedCorners::all(2))
                                    }))
                            )
                    }))
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
                                let _loaded_files: Vec<WaveformFile> = LOADED_FILES.lock_ref().iter().cloned().collect();
                                let variables = get_variables_from_selected_scope(&scope_id);
                                virtual_variables_list(variables, search_filter.clone()).into_element()
                            } else {
                                virtual_variables_list(Vec::new(), "Select a scope to view variables".to_string()).into_element()
                            }
                        }
                    }
                )
        )
}

fn convert_files_to_tree_data(files: &[WaveformFile]) -> Vec<TreeViewItemData> {
    files.iter().map(|file| {
        let children = file.scopes.iter().map(|scope| {
            convert_scope_to_tree_data(scope)
        }).collect();
        
        TreeViewItemData::new(file.id.clone(), file.filename.clone())
            .item_type(TreeViewItemType::File)
            .with_children(children)
            .on_remove(move |id| {
                // Remove file from LOADED_FILES
                LOADED_FILES.lock_mut().retain(|f| f.id != id);
                
                // Remove from FILE_PATHS
                FILE_PATHS.lock_mut().remove(id);
                
                // Clear related scope selections if removed file contained selected scope
                if let Some(selected_scope) = SELECTED_SCOPE_ID.get_cloned() {
                    if selected_scope.starts_with(&format!("{}_", id)) {
                        SELECTED_SCOPE_ID.set(None);
                    }
                }
                
                // Clear expanded scopes for this file
                EXPANDED_SCOPES.lock_mut().retain(|scope| !scope.starts_with(id));
                
                // Save file list and scope selection after removal
                config::save_file_list();
                config::save_scope_selection();
                
                zoon::println!("Removed file: {}", id);
            })
    }).collect()
}

fn convert_scope_to_tree_data(scope: &ScopeData) -> TreeViewItemData {
    let mut children = Vec::new();
    
    // Add child scopes first
    for child_scope in &scope.children {
        children.push(convert_scope_to_tree_data(child_scope));
    }
    
    // Signals are NOT shown in Files & Scopes - they belong in the Variables panel
    
    TreeViewItemData::new(scope.id.clone(), scope.name.clone())
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
    El::new()
        .child_signal(
            map_ref! {
                let is_loading = IS_LOADING.signal(),
                let selected = FILE_PICKER_SELECTED.signal_ref(|s| s.len()) =>
                move {
                    let has_selection = *selected > 0;
                    let is_loading = *is_loading;
                    
                    let mut btn = button();
                    
                    if is_loading {
                        btn = btn.label("Loading...")
                            .disabled(true);
                    } else if has_selection {
                        btn = btn.label(format!("Load {} Files", selected))
                            .on_press(|| process_file_picker_selection());
                    } else {
                        btn = btn.label("Load Files")
                            .disabled(true);
                    }
                    
                    btn.variant(ButtonVariant::Primary)
                        .size(ButtonSize::Medium)
                        .build()
                        .into_element()
                }
            }
        )
}


fn file_picker_content() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .child_signal(
            FILE_PICKER_ERROR.signal_cloned().map(|error| {
                if let Some(error_msg) = error {
                    Column::new()
                        .s(Gap::new().y(16))
                        .s(Align::center())
                        .s(Padding::all(32))
                        .item(
                            icon(IconName::TriangleAlert)
                                .size(IconSize::Large)
                                .color(IconColor::Error)
                                .build()
                        )
                        .item(
                            El::new()
                                .s(Font::new().size(16).color_signal(neutral_11()))
                                .child(error_msg)
                        )
                        .item(
                            button()
                                .label("Retry")
                                .variant(ButtonVariant::Secondary)
                                .size(ButtonSize::Medium)
                                .on_press(|| {
                                    FILE_PICKER_ERROR.set_neq(None);
                                    send_up_msg(UpMsg::BrowseDirectory("/".to_string()));
                                })
                                .build()
                        )
                        .unify()
                } else {
                    simple_file_picker_tree().unify()
                }
            })
        )
}

fn simple_file_picker_tree() -> impl Element {
    // Note: Root directory "/" is already requested by show_file_paths_dialog()
    // No need for duplicate request here
    
    El::new()
        .s(Height::fill())
        .child_signal(
            map_ref! {
                let tree_cache = FILE_TREE_CACHE.signal_cloned(),
                let expanded = FILE_PICKER_EXPANDED.signal_cloned() =>
                move {
                    monitor_directory_expansions(expanded.clone());
                    
                    // Check if we have root directory data
                    if let Some(_root_items) = tree_cache.get("/") {
                        // Create root "/" item and build hierarchical tree
                        let tree_data = vec![
                            TreeViewItemData::new("/".to_string(), "/".to_string())
                                .with_children(build_hierarchical_tree("/", &tree_cache))
                        ];
                        
                        tree_view()
                            .data(tree_data)
                            .size(TreeViewSize::Medium)
                            .variant(TreeViewVariant::Basic)
                            .show_icons(true)
                            .show_checkboxes(true)
                            .external_expanded(FILE_PICKER_EXPANDED.clone())
                            .external_selected(FILE_PICKER_SELECTED.clone())
                            .build()
                            .unify()
                    } else {
                        El::new()
                            .s(Padding::all(16))
                            .s(Font::new().size(14).color_signal(neutral_9()))
                            .child("Loading filesystem...")
                            .unify()
                    }
                }
            }
        )
}

fn build_hierarchical_tree(
    path: &str, 
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>
) -> Vec<TreeViewItemData> {
    if let Some(items) = tree_cache.get(path) {
        items.iter().map(|item| {
            if item.is_directory {
                // Check if we have cached contents for this directory
                if let Some(_children) = tree_cache.get(&item.path) {
                    // Build actual hierarchical children
                    let children = build_hierarchical_tree(&item.path, tree_cache);
                    TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("folder".to_string())
                        .item_type(TreeViewItemType::Folder)
                        .with_children(children)
                } else {
                    // No cached contents - only show expand arrow if directory has expandable content
                    let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("folder".to_string())
                        .item_type(TreeViewItemType::Folder);
                    
                    // Only add placeholder children if directory has expandable content
                    if item.has_expandable_content {
                        data = data.with_children(vec![
                            TreeViewItemData::new("loading", "Loading...")
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
                    .item_type(TreeViewItemType::File);
                
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
    
    // Send browse requests for newly expanded directories
    for path in new_expansions {
        if path.starts_with("/") && !path.is_empty() {
            zoon::println!("TreeView: Newly expanded directory: {}", path);
            send_up_msg(UpMsg::BrowseDirectory(path));
        }
    }
    
    // Update last expanded set
    LAST_EXPANDED.set_neq(expanded);
}


fn selected_files_display() -> impl Element {
    El::new()
        .s(Height::exact(120))
        .s(Width::fill())
        .s(Background::new().color_signal(neutral_2()))
        .s(Borders::all_signal(neutral_4().map(|color| {
            Border::new().width(1).color(color)
        })))
        .s(RoundedCorners::all(4))
        .s(Padding::all(8))
        .child(
            Column::new()
                .s(Height::fill())
                .s(Gap::new().y(8))
                .item(
                    Row::new()
                        .s(Gap::new().x(8))
                        .s(Align::new().center_y())
                        .item(
                            El::new()
                                .s(Font::new().size(14).weight(FontWeight::SemiBold).color_signal(neutral_11()))
                                .child("Selected Files")
                        )
                        .item(
                            El::new()
                                .s(Font::new().size(13).color_signal(neutral_9()))
                                .child_signal(
                                    FILE_PICKER_SELECTED.signal_ref(|selected| {
                                        if selected.is_empty() {
                                            "No files selected".to_string()
                                        } else {
                                            format!("{} file{} selected", selected.len(), if selected.len() == 1 { "" } else { "s" })
                                        }
                                    })
                                )
                        )
                        .item(
                            El::new()
                                .s(Width::fill())
                        )
                        .item_signal(
                            FILE_PICKER_SELECTED.signal_ref(|selected| !selected.is_empty()).map(|has_selection| {
                                if has_selection {
                                    Some(
                                        button()
                                            .label("Clear All")
                                            .left_icon(IconName::X)
                                            .variant(ButtonVariant::Ghost)
                                            .size(ButtonSize::Small)
                                            .on_press(|| {
                                                FILE_PICKER_SELECTED.lock_mut().clear();
                                            })
                                            .build()
                                    )
                                } else {
                                    None
                                }
                            })
                        )
                )
                .item(
                    El::new()
                        .s(Height::fill())
                        .s(Scrollbars::both())
                        .child_signal(
                            FILE_PICKER_SELECTED.signal_ref(|selected| selected.clone()).map(|selected_paths| {
                                if selected_paths.is_empty() {
                                    El::new()
                                        .s(Height::fill())
                                        .s(Align::center())
                                        .s(Font::new().italic().color_signal(neutral_8()))
                                        .child("Select waveform files from the directory tree above")
                                        .unify()
                                } else {
                                    Column::new()
                                        .s(Gap::new().y(4))
                                        .items(selected_paths.into_iter().map(|path| {
                                            Row::new()
                                                .s(Gap::new().x(8))
                                                .s(Align::new().center_y())
                                                .s(Padding::new().x(8).y(4))
                                                .s(Background::new().color_signal(neutral_1()))
                                                .s(RoundedCorners::all(3))
                                                .item(
                                                    icon(IconName::File)
                                                        .size(IconSize::Small)
                                                        .color(IconColor::Secondary)
                                                        .build()
                                                )
                                                .item(
                                                    El::new()
                                                        .s(Font::new().size(13).color_signal(neutral_11()).no_wrap())
                                                        .s(Width::fill())
                                                        .child(path.clone())
                                                )
                                                .item(
                                                    button()
                                                        .left_icon(IconName::X)
                                                        .variant(ButtonVariant::DestructiveGhost)
                                                        .size(ButtonSize::Small)
                                                        .on_press({
                                                            let path = path.clone();
                                                            move || {
                                                                FILE_PICKER_SELECTED.lock_mut().remove(&path);
                                                            }
                                                        })
                                                        .build()
                                                )
                                        }))
                                        .unify()
                                }
                            })
                        )
                )
        )
}

// File picker utility functions




fn process_file_picker_selection() {
    let selected_files = FILE_PICKER_SELECTED.lock_ref().clone();
    
    if !selected_files.is_empty() {
        IS_LOADING.set(true);
        
        // Process each selected file path
        for file_path in selected_files.iter() {
            // Generate file ID and store path mapping for config persistence
            let file_id = shared::generate_file_id(file_path);
            FILE_PATHS.lock_mut().insert(file_id, file_path.clone());
            config::save_file_list();
            
            // Send load request
            send_up_msg(UpMsg::LoadWaveformFile(file_path.clone()));
        }
        
        // Close dialog and clear selection
        SHOW_FILE_DIALOG.set(false);
        FILE_PICKER_SELECTED.lock_mut().clear();
        FILE_PICKER_ERROR.set_neq(None);
    }
}

fn remove_all_button() -> impl Element {
    button()
        .label("Remove All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(|| {
            LOADED_FILES.lock_mut().clear();
            FILE_PATHS.lock_mut().clear();
            EXPANDED_SCOPES.lock_mut().clear();
            SELECTED_SCOPE_ID.set(None);
            config::save_file_list();
            config::save_scope_selection();
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