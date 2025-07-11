use zoon::*;
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::{Theme, toggle_theme, theme};
use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_3, neutral_4, neutral_6, neutral_8, neutral_9, neutral_10, neutral_11, neutral_12, primary_6, primary_7};
use shared::{WaveformFile, ScopeData, filter_variables};
use crate::types::{get_variables_from_selected_scope};
use crate::virtual_list::virtual_variables_list;
use crate::config;
use crate::{
    IS_DOCKED_TO_BOTTOM, FILES_PANEL_WIDTH, FILES_PANEL_HEIGHT,
    VERTICAL_DIVIDER_DRAGGING, HORIZONTAL_DIVIDER_DRAGGING,
    VARIABLES_SEARCH_FILTER, SHOW_FILE_DIALOG, FILE_PATHS_INPUT, IS_LOADING,
    LOADED_FILES, SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, EXPANDED_SCOPES,
    FILE_PATHS, show_file_paths_dialog, process_file_paths
};

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
                .s(Width::exact(500))
                .child(
                    Column::new()
                        .s(Gap::new().y(16))
                        .item(
                            El::new()
                                .s(Font::new().size(18).weight(FontWeight::Bold).color_signal(neutral_12()))
                                .child("Load Waveform Files")
                        )
                        .item(
                            El::new()
                                .s(Font::new().size(14).color_signal(neutral_10()))
                                .child("Enter absolute file paths, separated by commas:")
                        )
                        .item(
                            input()
                                .placeholder("/path/to/file1.vcd, /path/to/file2.fst")
                                .on_change(|text| FILE_PATHS_INPUT.set_neq(text))
                                .size(InputSize::Medium)
                                .build()
                        )
                        .item(
                            Row::new()
                                .s(Gap::new().x(12))
                                .s(Align::new().right())
                                .item(
                                    button()
                                        .label("Cancel")
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Medium)
                                        .on_press(|| SHOW_FILE_DIALOG.set(false))
                                        .build()
                                )
                                .item(
                                    load_files_dialog_button()
                                )
                        )
                )
        )
}

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
                            .s(Width::fill())
                    )
                    .item(
                        load_files_button_with_progress(
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            Some(IconName::Folder)
                        )
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
                    )
                    .item(
                        remove_all_button()
                    ),
                Column::new()
                    .s(Gap::new().y(4))
                    .s(Padding::all(12))
                    .s(Height::fill())
                    .item(
                        El::new()
                            .s(Height::fill())
                            .child_signal(
                                LOADED_FILES.signal_vec_cloned()
                                    .to_signal_map(|files: &[WaveformFile]| {
                                        let tree_data = convert_files_to_tree_data(&files);
                                        
                                        if tree_data.is_empty() {
                                            El::new()
                                                .s(Padding::all(20))
                                                .s(Font::new().color_signal(neutral_8()).italic())
                                                .child("No files loaded. Click 'Load Files' to add waveform files.")
                                                .unify()
                                        } else {
                                            tree_view()
                                                .data(tree_data)
                                                .size(TreeViewSize::Medium)
                                                .variant(TreeViewVariant::Basic)
                                                .show_icons(true)
                                                .show_checkboxes(true)
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
                    .s(Gap::new().x(8))
                    .s(Align::new().center_y())
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
                                            let loaded_files: Vec<WaveformFile> = LOADED_FILES.lock_ref().iter().cloned().collect();
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
                            .s(Width::fill())
                    )
                    .item(
                        input()
                            .placeholder("variable_name")
                            .left_icon(IconName::Search)
                            .size(InputSize::Small)
                            .on_change(|text| VARIABLES_SEARCH_FILTER.set_neq(text))
                            .build()
                    ),
                simple_variables_content()
            )
        )
}

pub fn selected_variables_with_waveform_panel() -> impl Element {
    El::new()
        .s(Width::fill())
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
                            .s(Width::fill())
                    )
                    .item(
                        theme_toggle_button()
                    )
                    .item(
                        dock_toggle_button()
                    )
                    .item(
                        El::new()
                            .s(Width::fill())
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
pub fn files_panel_with_width() -> impl Element {
    El::new()
        .s(Width::exact_signal(FILES_PANEL_WIDTH.signal()))
        .s(Height::fill())
        .child(files_panel())
}

pub fn files_panel_with_height() -> impl Element {
    El::new()
        .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
        .s(Width::fill())
        .s(Scrollbars::both())
        .child(files_panel())
}

pub fn variables_panel_with_fill() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Scrollbars::both())
        .child(variables_panel())
}

pub fn files_panel_docked() -> impl Element {
    El::new()
        .s(Width::exact_signal(FILES_PANEL_WIDTH.signal()))
        .s(Height::fill())
        .s(Scrollbars::both())
        .child(files_panel())
}

pub fn variables_panel_docked() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Scrollbars::both())
        .child(variables_panel())
}

// Supporting functions
fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Background::new().color_signal(neutral_2()))
        .s(RoundedCorners::all(6))
        .s(Borders::all_signal(neutral_4().map(|color| {
            Border::new().width(1).color(color)
        })))
        .child(
            Column::new()
                .s(Height::fill())
                .item(
                    El::new()
                        .s(Padding::new().x(12).y(8))
                        .s(Background::new().color_signal(neutral_3()))
                        .s(Borders::new().bottom_signal(neutral_4().map(|color| {
                            Border::new().width(1).color(color)
                        })))
                        .s(RoundedCorners::new().top(6))
                        .s(Font::new().weight(FontWeight::SemiBold).size(14).color_signal(neutral_11()))
                        .child(header_content)
                )
                .item(
                    El::new()
                        .s(Height::fill())
                        .s(Scrollbars::both())
                        .child(content)
                )
        )
}

fn simple_variables_content() -> impl Element {
    Column::new()
        .s(Gap::new().y(0))
        .s(Padding::all(12))
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
                                let loaded_files: Vec<WaveformFile> = LOADED_FILES.lock_ref().iter().cloned().collect();
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
    }).collect()
}

fn convert_scope_to_tree_data(scope: &ScopeData) -> TreeViewItemData {
    let mut children = Vec::new();
    
    // Add child scopes first
    for child_scope in &scope.children {
        children.push(convert_scope_to_tree_data(child_scope));
    }
    
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

fn load_files_dialog_button() -> impl Element {
    El::new()
        .child_signal(IS_LOADING.signal().map(|is_loading| {
            let mut btn = button();
            
            if is_loading {
                btn = btn.label("Loading...")
                    .disabled(true);
            } else {
                btn = btn.label("Load Files")
                    .on_press(|| process_file_paths());
            }
            
            btn.variant(ButtonVariant::Primary)
                .size(ButtonSize::Medium)
                .build()
                .into_element()
        }))
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
            config::save_current_config();
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
                .variant(ButtonVariant::Secondary)
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
                            let icon_el = icon(IconName::ArrowDownToLine).size(IconSize::Small).build();
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
                    let new_is_docked = !IS_DOCKED_TO_BOTTOM.get();
                    IS_DOCKED_TO_BOTTOM.set_neq(new_is_docked);
                    
                    // Load appropriate panel sizes for the new mode
                    if let Some(config) = config::LOADED_CONFIG.lock_ref().clone() {
                        if new_is_docked {
                            // Switching to "Docked to Bottom" mode
                            FILES_PANEL_WIDTH.set_neq(u32::max(50, config.workspace.docked_to_bottom.files_panel_width as u32));
                            FILES_PANEL_HEIGHT.set_neq(u32::max(50, config.workspace.docked_to_bottom.files_panel_height as u32));
                        } else {
                            // Switching to "Docked to Right" mode  
                            FILES_PANEL_WIDTH.set_neq(u32::max(50, config.workspace.docked_to_right.files_panel_width as u32));
                            FILES_PANEL_HEIGHT.set_neq(u32::max(50, config.workspace.docked_to_right.files_panel_height as u32));
                        }
                    } else {
                        // Fallback to defaults if no config available
                        if new_is_docked {
                            FILES_PANEL_HEIGHT.set_neq(300);
                        } else {
                            FILES_PANEL_WIDTH.set_neq(470);
                            FILES_PANEL_HEIGHT.set_neq(300);
                        }
                    }
                    
                    config::save_current_config();
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