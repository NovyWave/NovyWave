use zoon::*;
use zoon::events::{Click, KeyDown};
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::{Theme, toggle_theme, theme};
use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_10, neutral_11, neutral_12, primary_3, primary_6, primary_7};
use moonzoon_novyui::components::{kbd, KbdSize, KbdVariant};
use moonzoon_novyui::tokens::typography::font_mono;
use shared::{ScopeData, UpMsg, TrackedFile, SelectedVariable, FileState, SignalValueQuery};
use crate::types::{get_variables_from_tracked_files, filter_variables_with_context};
use crate::virtual_list::virtual_variables_list;
use crate::config;
use std::collections::{HashSet, HashMap};
use crate::{
    IS_DOCKED_TO_BOTTOM, FILES_PANEL_WIDTH, FILES_PANEL_HEIGHT,
    VERTICAL_DIVIDER_DRAGGING, HORIZONTAL_DIVIDER_DRAGGING,
    VARIABLES_NAME_COLUMN_WIDTH, VARIABLES_VALUE_COLUMN_WIDTH,
    VARIABLES_NAME_DIVIDER_DRAGGING, VARIABLES_VALUE_DIVIDER_DRAGGING,
    VARIABLES_SEARCH_FILTER, VARIABLES_SEARCH_INPUT_FOCUSED, SHOW_FILE_DIALOG, IS_LOADING,
    LOADED_FILES, SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, EXPANDED_SCOPES,
    FILE_PATHS, show_file_paths_dialog, LOAD_FILES_VIEWPORT_Y,
    FILE_PICKER_EXPANDED, FILE_PICKER_SELECTED,
    FILE_PICKER_ERROR, FILE_PICKER_ERROR_CACHE, FILE_TREE_CACHE, send_up_msg, DOCK_TOGGLE_IN_PROGRESS,
    TRACKED_FILES, state, file_validation::validate_file_state, clipboard
};
use crate::state::TIMELINE_ZOOM_LEVEL;
use crate::state::SELECTED_VARIABLES_ROW_HEIGHT;
use crate::state::{SELECTED_VARIABLES, clear_selected_variables, remove_selected_variable};
use crate::format_utils::truncate_value;

/// Get signal type information for a selected variable
fn get_signal_type_for_selected_variable(selected_var: &SelectedVariable) -> String {
    // Parse the unique_id to get file_path, scope_path, and variable_name
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        // Use the same approach as Variables panel - only check loaded files
        let tracked_files = TRACKED_FILES.lock_ref();
        
        // Find the corresponding file by path and check if it's loaded
        for tracked_file in tracked_files.iter() {
            // Match by file path (tracked_file.path is the file path)
            if tracked_file.path == file_path {
                if let FileState::Loaded(waveform_file) = &tracked_file.state {
                    // The scope IDs in waveform_file include the full file path prefix
                    // We need to construct the full scope ID to match what's stored
                    let full_scope_id = format!("{}|{}", file_path, scope_path);
                    
                    // Find variables in the specific scope using the full scope ID
                    if let Some(variables) = shared::find_variables_in_scope(&waveform_file.scopes, &full_scope_id) {
                        // Find the specific variable by name
                        if let Some(signal) = variables.iter().find(|v| v.name == variable_name) {
                            return format!("{} {}-bit", signal.signal_type, signal.width);
                        }
                    }
                }
                break; // Found the file, no need to continue searching
            }
        }
    }
    
    // Fallback if variable not found or file not loaded
    "Loading...".to_string()
}

/// Get the default format for a signal type based on docs/signal_type_aware_formatting.md

// Format options and display functions moved to format_utils.rs

/// Create a smart dropdown with viewport edge detection using web-sys APIs
fn create_smart_dropdown(
    dropdown_options: Vec<crate::format_utils::DropdownFormatOption>, 
    selected_format: Mutable<String>,
    is_open: Mutable<bool>,
    trigger_id: String
) -> impl Element {
    use wasm_bindgen::JsCast;
    use web_sys::{window, HtmlElement};
    
    // Calculate actual dropdown height based on content
    // Modern library approach: account for all box model properties + safety margin
    let vertical_padding = 12.0; // .y(6) = 6px top + 6px bottom
    let explicit_line_height = 16.0; // Set explicit line-height to avoid browser variations
    let item_height = vertical_padding + explicit_line_height; // 28px total per item
    let border_height = 2.0; // Border::new().width(1) = 1px top + 1px bottom
    let safety_margin = 4.0; // Safety buffer for fractional pixel rendering
    
    let content_height = dropdown_options.len() as f64 * item_height;
    let calculated_height = content_height + border_height + safety_margin;
    let dynamic_dropdown_height = (calculated_height.min(300.0)).ceil(); // Math.ceil() for fractional pixels
    
    // Create unique ID for positioning calculations
    let dropdown_id = format!("smart-dropdown-{}", js_sys::Date::now() as u64);
    
    // Create dropdown with smart edge detection positioning
    Column::new()
        .s(Transform::new().move_down(0))
        .s(Background::new().color_signal(neutral_1()))
        .s(Borders::all_signal(neutral_4().map(|color| 
            Border::new().width(1).color(color)
        )))
        .s(RoundedCorners::all(4))
        .s(Shadows::new([
            Shadow::new()
                .y(4)
                .blur(6)
                .spread(-1)
                .color("oklch(70% 0.09 255 / 0.22)"),
            Shadow::new()
                .y(2)
                .blur(4) 
                .spread(-2)
                .color("oklch(70% 0.09 255 / 0.22)")
        ]))
        .s(Scrollbars::both())
        .update_raw_el({
            let dropdown_id = dropdown_id.clone();
            move |raw_el| {
                if let Some(html_el) = raw_el.dom_element().dyn_ref::<HtmlElement>() {
                    html_el.set_id(&dropdown_id);
                    
                    // Apply initial positioning
                    let style = html_el.style();
                    let _ = style.set_property("position", "fixed");
                    let _ = style.set_property("z-index", "9999");
                    let _ = style.set_property("min-width", "200px");
                    let _ = style.set_property("max-height", &format!("{}px", dynamic_dropdown_height));
                    let _ = style.set_property("overflow-y", "auto");
                    
                    // Edge detection and smart positioning using web-sys
                    if let Some(window) = window() {
                        if let Some(document) = window.document() {
                            if let Some(trigger_element) = document.get_element_by_id(&trigger_id) {
                                let viewport_width = window.inner_width().unwrap().as_f64().unwrap_or(1024.0);
                                let viewport_height = window.inner_height().unwrap().as_f64().unwrap_or(768.0);
                                
                                // Get trigger's bounding rect for positioning reference
                                let trigger_rect = trigger_element.get_bounding_client_rect();
                                let dropdown_width = 200.0; // min-width from CSS
                                let dropdown_height = dynamic_dropdown_height; // Use the calculated height
                                
                                // Start with default positioning below trigger
                                let mut x = trigger_rect.left();
                                let mut y = trigger_rect.bottom() + 1.0; // 1px gap below trigger
                                
                                // Right edge detection - shift left if dropdown would overflow
                                if x + dropdown_width > viewport_width {
                                    x = viewport_width - dropdown_width - 8.0; // 8px margin from edge
                                }
                                
                                // Left edge protection - ensure dropdown doesn't go off-screen left
                                if x < 8.0 {
                                    x = 8.0;
                                }
                                
                                // Bottom edge detection - flip to above trigger if insufficient space below
                                if y + dropdown_height > viewport_height {
                                    let space_above = trigger_rect.top();
                                    
                                    if space_above >= dropdown_height + 1.0 {
                                        // Enough space above - position above trigger
                                        y = trigger_rect.top() - dropdown_height - 1.0; // 1px gap above
                                    } else {
                                        // Not enough space above either - constrain within viewport
                                        y = viewport_height - dropdown_height - 8.0; // 8px margin from bottom
                                    }
                                }
                                
                                // Top edge protection
                                if y < 8.0 {
                                    y = 8.0;
                                }
                                
                                // Apply calculated position
                                let _ = style.set_property("left", &format!("{}px", x));
                                let _ = style.set_property("top", &format!("{}px", y));
                            }
                        }
                    }
                }
                
                raw_el
            }
        })
        .items(
            dropdown_options.iter().map(|option| {
                let option_format = format!("{:?}", option.format);
                let option_display = option.display_text.clone();
                let option_full_text = option.full_text.clone();
                let option_disabled = option.disabled;
                
                El::new()
                    .s(Width::fill())
                    .s(Height::exact(28))
                    .s(Padding::new().x(12).y(6))
                    .s(Cursor::new(if option_disabled {
                        CursorIcon::NotAllowed
                    } else {
                        CursorIcon::Pointer
                    }))
                    .update_raw_el({
                        let full_text = option_full_text.clone();
                        let display_text = option_display.clone();
                        move |raw_el| {
                            // Add tooltip with full text if it differs from display text
                            if full_text != display_text {
                                // Extract value-only part from full_text (remove format name)
                                let value_only = if let Some(space_pos) = full_text.rfind(' ') {
                                    full_text[..space_pos].to_string()
                                } else {
                                    full_text.clone()
                                };
                                // Apply same unicode filtering as display text
                                let filtered_tooltip = value_only
                                    .chars()
                                    .filter(|&c| {
                                        // Keep regular spaces and visible characters only
                                        c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                    })
                                    .collect::<String>()
                                    .trim()
                                    .to_string();
                                
                                // Only show tooltip if it differs from the displayed text
                                let display_value_only = if let Some(space_pos) = display_text.rfind(' ') {
                                    display_text[..space_pos].to_string()
                                } else {
                                    display_text.clone()
                                };
                                let filtered_display = display_value_only
                                    .chars()
                                    .filter(|&c| {
                                        c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                    })
                                    .collect::<String>()
                                    .trim()
                                    .to_string();
                                
                                if filtered_tooltip != filtered_display {
                                    if let Some(html_el) = raw_el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                                        html_el.set_title(&filtered_tooltip);
                                    }
                                }
                            }
                            raw_el
                        }
                    })
                    .s(Font::new()
                        .color_signal(
                            always(option_disabled).map_bool_signal(
                                || neutral_4(),
                                || neutral_8()
                            )
                        )
                        .size(12)
                    )
                    .child(
                        // Use Variables panel styling pattern: value left, format right
                        Row::new()
                            .s(Width::fill())
                            .s(Gap::new().x(8))
                            .item(
                                // Value - left aligned, contrasting color (like variable name)
                                El::new()
                                    .s(Font::new().color_signal(
                                        always(option_disabled).map_bool_signal(
                                            || neutral_4(),
                                            || neutral_11()
                                        )
                                    ).size(12).line_height(16).no_wrap())
                                    .s(font_mono())
                                    .s(Width::growable())
                                    .child({
                                        // Extract just the value part (before the format name)
                                        let display_text = option.display_text.clone();
                                        let value_only = if let Some(space_pos) = display_text.rfind(' ') {
                                            display_text[..space_pos].to_string()
                                        } else {
                                            display_text.clone()
                                        };
                                        // Remove invisible characters that cause UI layout issues
                                        let filtered_value = value_only
                                            .chars()
                                            .filter(|&c| {
                                                // Keep regular spaces and visible characters only
                                                c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                            })
                                            .collect::<String>()
                                            .trim()
                                            .to_string();
                                        El::new()
                                            .s(Font::new()
                                                .color_signal(
                                                    always(filtered_value.trim() == "-").map_bool_signal(
                                                        || neutral_8(),  // Muted color for placeholder
                                                        || neutral_11()  // Normal color for real values
                                                    )
                                                )
                                                .no_wrap()
                                            )
                                            .child(Text::new(&filtered_value))
                                    })
                            )
                            .item(El::new().s(Width::fill())) // Spacer to push format to right
                            .item(
                                // Format name - right aligned, blueish color (like variable type)
                                El::new()
                                    .s(Font::new().color_signal(
                                        always(option_disabled).map_bool_signal(
                                            || neutral_4(),
                                            || primary_6()
                                        )
                                    ).size(11).line_height(16).no_wrap())
                                    .s(Align::new().right())
                                    .child({
                                        // Extract just the format name (after the last space)
                                        let display_text = option.display_text.clone();
                                        let format_name = if let Some(space_pos) = display_text.rfind(' ') {
                                            display_text[space_pos + 1..].to_string()
                                        } else {
                                            // If no space, show the format enum name
                                            match option.format {
                                                shared::VarFormat::ASCII => "ASCII",
                                                shared::VarFormat::Binary => "Bin",
                                                shared::VarFormat::BinaryWithGroups => "Bin",
                                                shared::VarFormat::Hexadecimal => "Hex",
                                                shared::VarFormat::Octal => "Oct",
                                                shared::VarFormat::Signed => "Signed",
                                                shared::VarFormat::Unsigned => "Unsigned",
                                            }.to_string()
                                        };
                                        Text::new(&format_name)
                                    })
                            )
                    )
                    .on_click({
                        let selected_format = selected_format.clone();
                        let is_open = is_open.clone();
                        let option_format = option_format.clone();
                        move || {
                            if !option_disabled {
                                selected_format.set(option_format.clone());
                                is_open.set(false);
                            }
                        }
                    })
            }).collect::<Vec<_>>()
        )
}

/// Create a format selection component for a selected variable using NovyUI Select
fn create_format_select_component(selected_var: &SelectedVariable) -> impl Element {
    use crate::state::SIGNAL_VALUES;
    
    let unique_id = selected_var.unique_id.clone();
    
    // Get signal type for format options and default
    let signal_type = get_signal_type_for_selected_variable(selected_var);
    
    // Use the formatter exactly as set by user, or default to Hexadecimal
    let current_format = selected_var.formatter.unwrap_or_default();
    
    // Format options are now generated dynamically in the reactive signal based on MultiFormatValue
    
    // Get current multi-format signal value
    let signal_values = SIGNAL_VALUES.lock_ref();
    let multi_value = signal_values.get(&unique_id).cloned();
    drop(signal_values);
    
    // Create default multi-value if not available yet
    let _multi_value = multi_value.unwrap_or_else(|| {
        crate::format_utils::MultiFormatValue::new("Loading...".to_string())
    });
    
    // Create reactive state for selection changes
    let selected_format = Mutable::new(format!("{:?}", current_format));
    
    // Listen for changes and update backend
    Task::start({
        let unique_id = unique_id.clone();
        let selected_format = selected_format.clone();
        let previous_format = Mutable::new(format!("{:?}", current_format));
        
        selected_format.signal_cloned().for_each_sync(move |new_format_str| {
            if new_format_str != previous_format.get_cloned() {
                previous_format.set(new_format_str.clone());
                
                // Parse the format string back to VarFormat
                let new_format = match new_format_str.as_str() {
                    "ASCII" => shared::VarFormat::ASCII,
                    "Binary" => shared::VarFormat::Binary,
                    "BinaryWithGroups" => shared::VarFormat::BinaryWithGroups,
                    "Hexadecimal" => shared::VarFormat::Hexadecimal,
                    "Octal" => shared::VarFormat::Octal,
                    "Signed" => shared::VarFormat::Signed,
                    "Unsigned" => shared::VarFormat::Unsigned,
                    _ => shared::VarFormat::Hexadecimal, // Default fallback
                };
                
                update_variable_format(&unique_id, new_format);
            }
        })
    });
    
    // Create custom dropdown that shows value+format with working dropdown menu
    let is_open = Mutable::new(false);
    
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::new().center_y().left())
        .child_signal(
            map_ref! {
                let signal_values = SIGNAL_VALUES.signal_cloned(),
                let format_state = selected_format.signal_cloned() => {
                    // Get current multi-format value
                    let current_multi_value = signal_values.get(&unique_id).cloned()
                        .unwrap_or_else(|| crate::format_utils::MultiFormatValue::new("Loading...".to_string()));
                    
                    // Parse current format for proper display
                    let current_format_enum = match format_state.as_str() {
                        "ASCII" => shared::VarFormat::ASCII,
                        "Binary" => shared::VarFormat::Binary,
                        "BinaryWithGroups" => shared::VarFormat::BinaryWithGroups,
                        "Hexadecimal" => shared::VarFormat::Hexadecimal,
                        "Octal" => shared::VarFormat::Octal,
                        "Signed" => shared::VarFormat::Signed,
                        "Unsigned" => shared::VarFormat::Unsigned,
                        _ => shared::VarFormat::Hexadecimal,
                    };
                    
                    // Use full display text - CSS ellipsis will handle truncation dynamically
                    let display_text = current_multi_value.get_full_display_with_format(&current_format_enum);
                    let full_display_text = current_multi_value.get_full_display_with_format(&current_format_enum);
                    
                    // Generate dropdown options with formatted values
                    let dropdown_options = crate::format_utils::generate_dropdown_options(&current_multi_value, &signal_type);
                    
                    // Create unique trigger ID for positioning reference
                    let trigger_id = format!("select-trigger-{}", unique_id);
                    
                    // Create custom select trigger that shows value+format
                    Row::new()
                        .s(Width::fill())
                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT - 2))
                        .s(Padding::new().x(8).y(4))
                        .s(Gap::new().x(2))
                        .s(Borders::all_signal(neutral_4().map(|color| 
                            Border::new().width(1).color(color)
                        )))
                        .s(RoundedCorners::all(6))
                        .s(Background::new().color_signal(neutral_1()))
                        .s(Cursor::new(CursorIcon::Pointer))
                        .s(Align::new().center_y().left())
                        .update_raw_el({
                            let trigger_id = trigger_id.clone();
                            let full_text = full_display_text.clone();
                            let display_text_for_tooltip = display_text.clone();
                            move |raw_el| {
                                if let Some(html_el) = raw_el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                                    html_el.set_id(&trigger_id);
                                    // Add tooltip if text is truncated
                                    if full_text != display_text_for_tooltip {
                                        // Extract value-only part from full_text (remove format name)
                                        let value_only = if let Some(space_pos) = full_text.rfind(' ') {
                                            full_text[..space_pos].to_string()
                                        } else {
                                            full_text.clone()
                                        };
                                        // Apply same unicode filtering as display text
                                        let filtered_tooltip = value_only
                                            .chars()
                                            .filter(|&c| {
                                                // Keep regular spaces and visible characters only
                                                c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                            })
                                            .collect::<String>()
                                            .trim()
                                            .to_string();
                                        
                                        // Only show tooltip if it differs from the displayed text
                                        let display_value_only = if let Some(space_pos) = display_text_for_tooltip.rfind(' ') {
                                            display_text_for_tooltip[..space_pos].to_string()
                                        } else {
                                            display_text_for_tooltip.clone()
                                        };
                                        let filtered_display = display_value_only
                                            .chars()
                                            .filter(|&c| {
                                                c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                            })
                                            .collect::<String>()
                                            .trim()
                                            .to_string();
                                        
                                        if filtered_tooltip != filtered_display {
                                            html_el.set_title(&filtered_tooltip);
                                        }
                                    }
                                }
                                raw_el
                            }
                        })
                        .item(
                            // Pure flexbox approach - value, format, gap for chevron
                            Row::new()
                                .s(Width::fill())
                                .s(Gap::new().x(4))
                                .s(Align::new().center_y())
                                .item(
                                    // Value with programmatic truncation - fixed flex-grow to prevent jumping
                                    El::new()
                                        .s(Font::new().color_signal(neutral_11()).size(13).no_wrap())
                                        .s(font_mono())
                                        .s(Width::fill())
                                        .update_raw_el(|raw_el| {
                                            raw_el
                                                .style("flex-grow", "1")
                                                .style("flex-shrink", "1")
                                                .style("flex-basis", "0")
                                                .style("min-width", "0")
                                        })
                                        .child_signal(
                                            map_ref! {
                                                let text = zoon::always(display_text.clone()),
                                                let column_width = VARIABLES_VALUE_COLUMN_WIDTH.signal() => {
                                                    // Extract just the value part (before the format name)
                                                    let value_only = if let Some(space_pos) = text.rfind(' ') {
                                                        text[..space_pos].to_string()
                                                    } else {
                                                        text.clone()
                                                    };
                                                    let filtered_value = value_only
                                                        .chars()
                                                        .filter(|&c| {
                                                            c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                                        })
                                                        .collect::<String>()
                                                        .trim()
                                                        .to_string();
                                                    
                                                    // Dynamic truncation constants - adjust these to fine-tune layout
                                                    const MONOSPACE_CHAR_WIDTH_PX: f32 = 8.0;  // Width per character in monospace font
                                                    const TRIGGER_PADDING_PX: f32 = 16.0;      // Row padding (.x(8).y(4) = 8px each side)
                                                    const ELEMENT_GAPS_PX: f32 = 12.0;         // Gaps between value/copy/format/chevron (4px * 3 gaps)
                                                    const COPY_BUTTON_WIDTH_PX: f32 = 24.0;    // Small ghost button width
                                                    const FORMAT_TEXT_WIDTH_PX: f32 = 30.0;    // "Hex", "Bin", etc. text width
                                                    const CHEVRON_ICON_WIDTH_PX: f32 = 20.0;   // Dropdown chevron icon width
                                                    const LAYOUT_BUFFER_PX: f32 = 8.0;         // Safety margin for stable layout
                                                    
                                                    // Calculate available space for value text
                                                    let total_reserved_width = TRIGGER_PADDING_PX + ELEMENT_GAPS_PX + COPY_BUTTON_WIDTH_PX 
                                                        + FORMAT_TEXT_WIDTH_PX + CHEVRON_ICON_WIDTH_PX + LAYOUT_BUFFER_PX;
                                                    let available_text_width = (*column_width as f32 - total_reserved_width).max(40.0);
                                                    
                                                    // Convert width to character count with minimum safety threshold
                                                    const MIN_VISIBLE_CHARS: usize = 6;
                                                    let max_displayable_chars = ((available_text_width / MONOSPACE_CHAR_WIDTH_PX) as usize).max(MIN_VISIBLE_CHARS);
                                                    
                                                    // Apply truncation with ellipsis if text exceeds available space
                                                    let truncated_text = truncate_value(&filtered_value, max_displayable_chars);
                                                    El::new()
                                                        .s(Font::new()
                                                            .color_signal(
                                                                always(truncated_text.trim() == "-").map_bool_signal(
                                                                    || neutral_8(),  // Muted color for placeholder
                                                                    || neutral_11()  // Normal color for real values
                                                                )
                                                            )
                                                            .no_wrap()
                                                        )
                                                        .update_raw_el({
                                                            let filtered_value = filtered_value.clone();
                                                            let truncated_text = truncated_text.clone();
                                                            move |raw_el| {
                                                                // Add tooltip with full text if truncated
                                                                if filtered_value != truncated_text {
                                                                    if let Some(html_el) = raw_el.dom_element().dyn_ref::<web_sys::HtmlElement>() {
                                                                        html_el.set_title(&filtered_value);
                                                                    }
                                                                }
                                                                raw_el
                                                            }
                                                        })
                                                        .child(Text::new(&truncated_text))
                                                }
                                            }
                                        )
                                )
                                .item(
                                    // Copy button - small, minimal, wrapped to prevent event bubbling
                                    El::new()
                                        .update_raw_el(|raw_el| {
                                            raw_el.event_handler(|event: Click| {
                                                event.stop_propagation();
                                            })
                                        })
                                        .child(
                                            button()
                                                .left_icon(IconName::Copy)
                                                .variant(ButtonVariant::Ghost)
                                                .size(ButtonSize::Small)
                                                .custom_padding(4, 2)
                                                .on_press({
                                                    let display_text = display_text.clone();
                                                    move || {
                                                        // Extract just the value part for copying
                                                        let value_only = if let Some(space_pos) = display_text.rfind(' ') {
                                                            display_text[..space_pos].to_string()
                                                        } else {
                                                            display_text.clone()
                                                        };
                                                        let filtered_value = value_only
                                                            .chars()
                                                            .filter(|&c| {
                                                                c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                                            })
                                                            .collect::<String>()
                                                            .trim()
                                                            .to_string();
                                                        
                                                        // Copy to clipboard
                                                        clipboard::copy_variable_value(&filtered_value);
                                                    }
                                                })
                                                .build()
                                        )
                                )
                                .item(
                                    // Format name - fixed width, no shrinking
                                    El::new()
                                        .s(Font::new().color_signal(primary_6()).size(11).no_wrap())
                                        .update_raw_el(|raw_el| {
                                            raw_el.style("flex-shrink", "0") // Don't shrink
                                        })
                                        .child({
                                            // Extract just the format name (after the last space)
                                            let format_name = if let Some(space_pos) = display_text.rfind(' ') {
                                                display_text[space_pos + 1..].to_string()
                                            } else {
                                                // If no space, show the format enum name
                                                match current_format_enum {
                                                    shared::VarFormat::ASCII => "ASCII",
                                                    shared::VarFormat::Binary => "Bin",
                                                    shared::VarFormat::BinaryWithGroups => "Bin",
                                                    shared::VarFormat::Hexadecimal => "Hex",
                                                    shared::VarFormat::Octal => "Oct",
                                                    shared::VarFormat::Signed => "Signed",
                                                    shared::VarFormat::Unsigned => "Unsigned",
                                                }.to_string()
                                            };
                                            Text::new(&format_name)
                                        })
                                )
                        )
                        .item(
                            El::new()
                                .child(
                                    IconBuilder::new(IconName::ChevronDown)
                                        .size(IconSize::Small)
                                        .color(IconColor::Muted)
                                        .build()
                                )
                                .update_raw_el({
                                    let is_open = is_open.clone();
                                    move |raw_el| {
                                        raw_el.style_signal("transform", is_open.signal().map_bool(
                                            || "rotate(180deg)".to_string(),
                                            || "rotate(0deg)".to_string()
                                        ))
                                        .style("transition", "transform 0.2s ease")
                                    }
                                })
                        )
                        .element_below_signal(is_open.signal().map_true({
                            let selected_format = selected_format.clone();
                            let is_open = is_open.clone();
                            let trigger_id = trigger_id.clone();
                            
                            move || {
                                create_smart_dropdown(dropdown_options.clone(), selected_format.clone(), is_open.clone(), trigger_id.clone())
                            }
                        }))
                        .on_click({
                            let is_open = is_open.clone();
                            move || {
                                is_open.set_neq(!is_open.get());
                            }
                        })
                        .on_click_outside({
                            let is_open = is_open.clone();
                            move || is_open.set(false)
                        })
                }
            }
        )
}

/// Update the format for a selected variable and trigger config save + query refresh
fn update_variable_format(unique_id: &str, new_format: shared::VarFormat) {
    use crate::state::{SELECTED_VARIABLES, SELECTED_VARIABLE_FORMATS};
    
    // Update the global format tracking
    let mut formats = SELECTED_VARIABLE_FORMATS.lock_mut();
    formats.insert(unique_id.to_string(), new_format);
    drop(formats);
    
    // Update the SELECTED_VARIABLES vec by finding and updating the specific variable
    let mut selected_vars = SELECTED_VARIABLES.lock_mut();
    if let Some(var_index) = selected_vars.iter().position(|var| var.unique_id == unique_id) {
        let mut updated_var = selected_vars[var_index].clone();
        updated_var.formatter = Some(new_format); // Mark as explicitly set by user
        selected_vars.set_cloned(var_index, updated_var);
    }
    drop(selected_vars);
    
    // Save config immediately  
    crate::state::save_selected_variables();
    
    // Trigger signal value query refresh with new format
    trigger_signal_value_queries();
}

/// Query signal values for selected variables at a specific time
pub fn query_signal_values_at_time(time_seconds: f64) {
    let selected_vars = SELECTED_VARIABLES.lock_ref();
    // Process signal value queries for selected variables
    
    if selected_vars.is_empty() {
        return;
    }
    
    // Group queries by file path for efficient batch processing
    let mut queries_by_file: HashMap<String, Vec<SignalValueQuery>> = HashMap::new();
    
    for selected_var in selected_vars.iter() {
        // Parse unique_id for signal query
        if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
            // Create signal value query for variable
            let query = SignalValueQuery {
                scope_path,
                variable_name,
                time_seconds,
                format: selected_var.formatter.unwrap_or_default(),
            };
            
            queries_by_file.entry(file_path).or_insert_with(Vec::new).push(query);
        } else {
            // Skip variable with invalid unique_id format
        }
    }
    
    // Send batch queries for each file
    for (file_path, queries) in queries_by_file {
        // Send batch signal value queries to backend
        send_up_msg(UpMsg::QuerySignalValues { file_path, queries });
    }
}

/// Trigger signal value queries when variables are present
pub fn trigger_signal_value_queries() {
    // Trigger signal value queries for selected variables
    let selected_vars = SELECTED_VARIABLES.lock_ref();
    for _var in selected_vars.iter() {
        // Process selected variable for signal query
    }
    drop(selected_vars);
    
    // Query at current timeline cursor position
    query_signal_values_at_time(crate::state::TIMELINE_CURSOR_POSITION.get() as f64);
}


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
                                    .on_focus(|| VARIABLES_SEARCH_INPUT_FOCUSED.set_neq(true))
                                    .on_blur(|| VARIABLES_SEARCH_INPUT_FOCUSED.set_neq(false))
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
                                SELECTED_VARIABLES.signal_vec_cloned().len().map(|vars_count| {
                                    // Add one extra row height for scrollbar (names/values) or footer/timeline (canvas)
                                    (vars_count + 1) as u32 * SELECTED_VARIABLES_ROW_HEIGHT
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
                                        // Column 1: Variable name (resizable) with footer
                                        Column::new()
                                            .s(Width::exact_signal(VARIABLES_NAME_COLUMN_WIDTH.signal()))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .s(Scrollbars::x_and_clip_y())
                                            .update_raw_el(|raw_el| {
                                                raw_el.style("scrollbar-width", "thin")
                                            })
                                            .items_signal_vec(
                                                SELECTED_VARIABLES.signal_vec_cloned().map(|selected_var| {
                                                    Row::new()
                                                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                        .s(Width::fill())
                                                        .s(Padding::new().x(2).y(4))
                                                        .s(Gap::new().x(4))
                                                        .item({
                                                            let unique_id = selected_var.unique_id.clone();
                                                            button()
                                                                .left_icon(IconName::X)
                                                                .variant(ButtonVariant::DestructiveGhost)
                                                                .size(ButtonSize::Small)
                                                                .custom_padding(4, 2)
                                                                .on_press(move || {
                                                                    remove_selected_variable(&unique_id);
                                                                })
                                                                .build()
                                                        })
                                                        .item(
                                                            Row::new()
                                                                .s(Gap::new().x(8))
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
                                                                        .update_raw_el(|raw_el| {
                                                                            raw_el
                                                                                .style("text-overflow", "ellipsis") // Show ellipsis for long text
                                                                                .style("max-width", "100%") // Ensure it doesn't exceed container
                                                                        })
                                                                        .child_signal({
                                                                            let selected_var = selected_var.clone();
                                                                            use crate::state::FILE_LOADING_TRIGGER;
                                                                            FILE_LOADING_TRIGGER.signal().map(move |_trigger| {
                                                                                get_signal_type_for_selected_variable(&selected_var)
                                                                            })
                                                                        })
                                                                )
                                                                .update_raw_el({
                                                                    let selected_var = selected_var.clone();
                                                                    move |raw_el| {
                                                                        use crate::state::FILE_LOADING_TRIGGER;
                                                                        let title_signal = FILE_LOADING_TRIGGER.signal().map({
                                                                            let selected_var = selected_var.clone();
                                                                            move |_trigger| {
                                                                                let signal_type = get_signal_type_for_selected_variable(&selected_var);
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
                                            )
                                            .item(
                                                // Footer row with zoom percentage
                                                El::new()
                                                    .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                    .s(Width::fill())
                                                    .s(Padding::all(8))
                                                    .s(Font::new().color_signal(neutral_8()).size(12).center())
                                                    .s(Transform::new().move_up(4))
                                                    .child(
                                                        Row::new()
                                                            .s(Align::center())
                                                            .s(Gap::new().x(6))
                                                            .item(kbd("W").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
                                                            .item(
                                                                El::new()
                                                                    .s(Width::exact(45))
                                                                    .s(Font::new().color_signal(neutral_11()).center())
                                                                    .child(
                                                                        Text::with_signal(
                                                                            TIMELINE_ZOOM_LEVEL.signal().map(|zoom_level| {
                                                                                let percentage = (zoom_level * 100.0) as u32;
                                                                                format!("{}%", percentage)
                                                                            })
                                                                        )
                                                                    )
                                                            )
                                                            .item(kbd("S").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
                                                    )
                                            )
                                    )
                                    .item(variables_vertical_divider(VARIABLES_NAME_DIVIDER_DRAGGING.clone()))
                                    .item(
                                        // Column 2: Variable value (resizable) - HEIGHT FOLLOWER
                                        Column::new()
                                            .s(Width::exact_signal(VARIABLES_VALUE_COLUMN_WIDTH.signal()))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .s(Scrollbars::x_and_clip_y())
                                            .update_raw_el(|raw_el| {
                                                raw_el.style("scrollbar-width", "thin")
                                            })
                                            .items_signal_vec(
                                                SELECTED_VARIABLES.signal_vec_cloned().map(|selected_var| {
                                                    El::new()
                                                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                        .s(Width::fill())
                                                        .child(
                                                            create_format_select_component(&selected_var)
                                                        )
                                                })
                                            )
                                            .item(
                                                // Footer row with selected time and zoom percentage display
                                                El::new()
                                                    .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                    .s(Width::fill())
                                                    .s(Padding::all(8))
                                                    .s(Transform::new().move_up(4))
                                                    .child(
                                                        Row::new()
                                                            .s(Align::center())
                                                            .s(Gap::new().x(6))
                                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                                            .item(kbd("A").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
                                                            .item(
                                                                El::new()
                                                                    .s(Font::new().color_signal(neutral_11()))
                                                                    .child(
                                                                        Text::with_signal(
                                                                            crate::state::TIMELINE_CURSOR_POSITION.signal().map(|cursor_pos| {
                                                                                format!("{}s", cursor_pos.round() as i32)
                                                                            })
                                                                        )
                                                                    )
                                                            )
                                                            .item(kbd("D").size(KbdSize::Small).variant(KbdVariant::Outlined).build())
                                                    )
                                            )
                                    )
                                    .item(variables_vertical_divider(VARIABLES_VALUE_DIVIDER_DRAGGING.clone()))
                                    .item(
                                        // Column 3: Unified waveform canvas (fills remaining space) - HEIGHT FOLLOWER
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .s(Background::new().color_signal(neutral_2()))
                                            .child(crate::waveform_canvas::waveform_canvas())
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
                                    .custom_padding(4, 2)
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
                                    .custom_padding(4, 2)
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
                            .on_press(|| {
                                crate::waveform_canvas::zoom_in();
                            })
                            .build()
                    )
                    .item(
                        button()
                            .label("Zoom Out")
                            .left_icon(IconName::ZoomOut)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Small)
                            .on_press(|| {
                                crate::waveform_canvas::zoom_out();
                            })
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
    // TEST 2: Remove Scrollbars::both() from individual panels
    El::new()
        .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
        .s(Width::growable())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child(files_panel())
}

pub fn variables_panel_with_fill() -> impl Element {
    // TEST 2: Remove Scrollbars::both() from individual panels
    El::new()
        .s(Width::growable())
        .s(Height::fill())
        .s(Scrollbars::both())
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            if is_docked {
                // When docked to bottom, use FILES_PANEL_HEIGHT signal for synchronized resizing
                El::new()
                    .s(Width::fill())
                    .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
                    .update_raw_el(|raw_el| {
                        raw_el.style("scrollbar-width", "thin")
                            .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
                    })
                    .child(variables_panel())
                    .into_element()
            } else {
                // When docked to right, fill available height
                El::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .update_raw_el(|raw_el| {
                        raw_el.style("scrollbar-width", "thin")
                            .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
                    })
                    .child(variables_panel())
                    .into_element()
            }
        }))
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
        .s(Scrollbars::both())
        .s(Background::new().color_signal(neutral_2()))
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
        })
        .s(Borders::all_signal(neutral_4().map(|color| {
            Border::new().width(1).color(color)
        })))
        .child(
            Column::new()
                .s(Height::fill())
                .s(Scrollbars::both())
                .update_raw_el(|raw_el| {
                    raw_el.style("scrollbar-width", "thin")
                })
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
// Helper function to get timeline info for a file
fn get_file_timeline_info(file_path: &str, _waveform_file: &shared::WaveformFile) -> String {
    // Phase 11: Extract timeline information from file
    // TODO: Extract actual time ranges from backend's time_table data
    // IMPORTANT: Files can start at ANY time value, not necessarily 0!
    // Backend has time_table[0] (min_time) and time_table.last() (max_time)
    
    let _file_name = file_path.split('/').last().unwrap_or("unknown");
    
    // Extract real timeline data from loaded files instead of hardcoded values
    let loaded_files = LOADED_FILES.lock_ref();
    
    let (min_time_f64, max_time_f64, unit) = if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
        if let (Some(min_time), Some(max_time)) = (loaded_file.min_time, loaded_file.max_time) {
            // Determine appropriate time unit based on time range magnitude
            let time_range = max_time - min_time;
            
            if time_range >= 1.0 {
                // Seconds range
                (min_time, max_time, "s")
            } else if time_range >= 0.001 {
                // Milliseconds range  
                (min_time * 1000.0, max_time * 1000.0, "ms")
            } else if time_range >= 0.000001 {
                // Microseconds range
                (min_time * 1_000_000.0, max_time * 1_000_000.0, "μs")
            } else {
                // Nanoseconds range
                (min_time * 1_000_000_000.0, max_time * 1_000_000_000.0, "ns")
            }
        } else {
            // File loaded but no timeline data available
            (0.0, 0.0, "loading...")
        }
    } else {
        // File not yet loaded
        (0.0, 0.0, "loading...")
    };
    
    // Convert to integers for display, handle loading state
    let (min_time, max_time) = if unit == "loading..." {
        (0, 0)
    } else {
        (min_time_f64 as i32, max_time_f64 as i32)
    };
    
    // Use space + parentheses format - TreeView will render as regular text
    // TODO: Enhance TreeView component to style timeline info with lower contrast
    if unit == "loading..." {
        format!(" ({})", unit)
    } else {
        format!(" ({}{}\u{2013}{}{})", min_time, unit, max_time, unit)
    }
}

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
                // Successfully loaded file - show with scopes and timeline info
                let children = waveform_file.scopes.iter().map(|scope| {
                    convert_scope_to_tree_data(scope)
                }).collect();
                
                // Create enhanced label with timeline information
                let timeline_info = get_file_timeline_info(&tracked_file.path, waveform_file);
                let enhanced_label = format!("{}{}", tracked_file.smart_label, timeline_info);
                
                TreeViewItemData::new(tracked_file.id.clone(), enhanced_label)
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
                            // File validation failed - handle error
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
    // Clear all loaded files and related state
    
    // Get all tracked file IDs before clearing
    let file_ids: Vec<String> = state::TRACKED_FILES.lock_ref()
        .iter()
        .map(|f| f.id.clone())
        .collect();
    
    // Cleanup tracked files
    
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


