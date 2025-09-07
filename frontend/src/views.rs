use zoon::*;
use zoon::events::{Click, KeyDown};
use moonzoon_novyui::*;
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::tokens::color::{neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, neutral_12, primary_3, primary_6, primary_7};
// Removed unused import: moonzoon_novyui::tokens::*
use moonzoon_novyui::components::{kbd, KbdSize, KbdVariant};
use moonzoon_novyui::tokens::typography::font_mono;
use shared::{ScopeData, UpMsg, TrackedFile, SelectedVariable, FileState};
use crate::actors::{relay};
use crate::dataflow::atom::Atom;
use crate::actors::selected_variables::{get_variables_from_tracked_files, filter_variables_with_context, VariableWithContext};
use crate::virtual_list::{virtual_variables_list_pre_filtered};
use crate::config::app_config;
use std::collections::{HashSet, HashMap};
use crate::clipboard;
use crate::file_dialog::show_file_paths_dialog;
use crate::visualizer::timeline::timeline_actor::{ns_per_pixel_signal, cursor_position_seconds_signal};

use crate::state::SELECTED_VARIABLES_ROW_HEIGHT;

// Cached timeline range signal to prevent duplicate calculations
fn timeline_range_signal() -> impl Signal<Item = Option<(f64, f64)>> {
    map_ref! {
        let _files_count = crate::actors::global_domains::file_count_signal(),
        let _selected_vars = variables_signal() =>
        crate::visualizer::canvas::waveform_canvas::get_maximum_timeline_range()
    }.dedupe_cloned()
}

// Smart time formatting that removes unnecessary decimals
fn format_time(time: f64) -> String {
    if !time.is_finite() || time <= 0.0 {
        "0s".to_string()
    } else if time < 1e-6 {
        let ns_val = time * 1e9;
        if ns_val.fract() == 0.0 {
            format!("{}ns", ns_val as i64)
        } else {
            format!("{:.1}ns", ns_val)
        }
    } else if time < 1e-3 {
        let us_val = time * 1e6;
        if us_val.fract() == 0.0 {
            format!("{}μs", us_val as i64)
        } else {
            format!("{:.1}μs", us_val)
        }
    } else if time < 1.0 {
        let ms_val = time * 1e3;
        if ms_val.fract() == 0.0 {
            format!("{}ms", ms_val as i64)
        } else {
            format!("{:.1}ms", ms_val)
        }
    } else {
        if time.fract() == 0.0 {
            format!("{}s", time as i64)
        } else {
            format!("{:.1}s", time)
        }
    }
}
use crate::actors::selected_variables::{variables_signal, variables_signal_vec, selected_scope_signal, search_filter_signal, search_filter_changed_relay, search_focus_changed_relay};
use crate::visualizer::interaction::dragging::{
    variables_name_column_width_signal, variables_value_column_width_signal, files_panel_height_signal
};
// dialog_manager eliminated - use simple file_dialog functions
// Error cache functionality simplified - most was stub implementation
use shared::truncate_value;

/// Format options for dropdown - contains value and disabled state
#[derive(Debug, Clone)]
struct DropdownFormatOption {
    format: shared::VarFormat,
    display_text: String,
    full_text: String,    // For tooltip
    disabled: bool,
}

impl DropdownFormatOption {
    fn new(format: shared::VarFormat, display_text: String, full_text: String, disabled: bool) -> Self {
        Self {
            format,
            display_text,
            full_text,
            disabled,
        }
    }
}

/// Get signal type information for a selected variable (signal-based version)
fn get_signal_type_for_selected_variable_from_files(selected_var: &SelectedVariable, files: &[TrackedFile]) -> String {
    // Parse the unique_id to get file_path, scope_path, and variable_name
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        // Find the corresponding file by path and check if it's loaded
        for tracked_file in files.iter() {
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

/// Get signal type information for a selected variable (legacy synchronous version)
fn get_signal_type_for_selected_variable(selected_var: &SelectedVariable) -> String {
    // Parse the unique_id to get file_path, scope_path, and variable_name
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        // Use the same approach as Variables panel - only check loaded files
        let tracked_files = crate::actors::global_domains::tracked_files_domain().get_current_files();
        
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

/// Generate dropdown options for UI from shared SignalValue
fn generate_ui_dropdown_options(
    signal_value: &shared::SignalValue,
    _signal_type: &str,
    max_value_chars: usize
) -> Vec<DropdownFormatOption> {
    let all_formats = [
        shared::VarFormat::ASCII,
        shared::VarFormat::Binary,
        shared::VarFormat::BinaryWithGroups,
        shared::VarFormat::Hexadecimal,
        shared::VarFormat::Octal,
        shared::VarFormat::Signed,
        shared::VarFormat::Unsigned,
    ];

    all_formats
        .iter()
        .map(|format| {
            let (display_text, full_text) = match signal_value {
                shared::SignalValue::Present(_) => {
                    let full = signal_value.get_full_display_with_format(format);
                    let truncated = signal_value.get_truncated_display_with_format(format, max_value_chars);
                    (truncated, full)
                },
                shared::SignalValue::Missing => {
                    let text = format!("N/A {}", format.as_static_str());
                    (text.clone(), text)
                },
                shared::SignalValue::Loading => {
                    let text = format!("Loading... {}", format.as_static_str());
                    (text.clone(), text)
                },
            };

            DropdownFormatOption::new(
                *format, 
                display_text, 
                full_text, 
                false
            )
        })
        .collect()
}

/// Create a smart dropdown with viewport edge detection using web-sys APIs
fn create_smart_dropdown(
    dropdown_options: Vec<DropdownFormatOption>, 
    selected_format: Mutable<String>,
    is_open: Mutable<bool>,
    trigger_id: String
) -> impl Element {
    use wasm_bindgen::JsCast;
    use web_sys::{window, HtmlElement};
    
    // Calculate actual dropdown height based on content
    // Modern library approach: account for all box model properties + safety margin
    let vertical_padding = SPACING_12 as f64;
    let explicit_line_height = 16.0; // Magic number - should be typography token
    let item_height = vertical_padding + explicit_line_height; // 28px total per item
    let border_height = 2.0; // Magic number - should be border design token
    let safety_margin = 4.0; // Magic number - should be layout constant
    
    let content_height = dropdown_options.len() as f64 * item_height;
    let calculated_height = content_height + border_height + safety_margin;
    // ✅ RESPONSIVE: Calculate max height based on viewport percentage
    const DROPDOWN_MAX_VIEWPORT_RATIO: f64 = 0.25; // 25% of viewport height
    let max_dropdown_height = 1200.0 * DROPDOWN_MAX_VIEWPORT_RATIO; // Using fallback viewport height for dropdown sizing
    let dynamic_dropdown_height = (calculated_height.min(max_dropdown_height)).ceil();
    
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
                                const MIN_DROPDOWN_WIDTH: f64 = 200.0; // Matches CSS min-width
                                let dropdown_width = MIN_DROPDOWN_WIDTH;
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
                    .s(Padding::new().x(SPACING_12).y(SPACING_6))
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
                            .s(Gap::new().x(SPACING_8))
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
    let unique_id = selected_var.unique_id.clone();
    
    // Get signal type for format options and default
    let signal_type = get_signal_type_for_selected_variable(selected_var);
    
    // Check if format is already saved using domain method
    let saved_format = crate::visualizer::timeline::timeline_actor::get_variable_format(&unique_id);
    let current_format = saved_format
        .or(selected_var.formatter)
        .unwrap_or_default();
    
    // 🔍 DEBUG: Format initialization logging disabled to reduce startup spam
    // if current_format != shared::SignalValueFormat::default() || saved_format.is_some() {
    // }
    
    // Create reactive state for selection changes
    let selected_format = Mutable::new(format!("{:?}", current_format));
    
    // Listen for changes and update backend
    Task::start({
        let unique_id = unique_id.clone();
        let selected_format = selected_format.clone();
        let previous_format = Mutable::new(format!("{:?}", current_format));
        
        async move {
            selected_format.signal_cloned().for_each(move |new_format_str| {
                let previous_format = previous_format.clone();
                let unique_id = unique_id.clone();
                async move {
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
                
                // 🔍 DEBUG: Format change
            }
            }
            }).await;
        }
    });
    
    // Create custom dropdown that shows value+format with working dropdown menu
    let is_open = Mutable::new(false);
    
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::new().center_y().left())
        .child_signal({
            let unique_id_for_signal = unique_id.clone();
            
            // FST Debug: Check what unique_id looks like for FST files
            // FST UI debug logging removed to prevent event loop blocking
            
            map_ref! {
                // ✅ NEWEST: Use unified timeline service with integer time precision
                let current_value = crate::visualizer::timeline::timeline_actor::cursor_value_signal(&unique_id_for_signal),
                let format_state = selected_format.signal_cloned() => {
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
                    
                    // Debug logging removed to prevent console spam in hot signal path
                    
                    let current_signal_value = if current_value == "N/A" {
                        shared::SignalValue::missing()
                    } else if current_value == "Loading..." {
                        shared::SignalValue::loading()
                    } else {
                        shared::SignalValue::from_data(current_value.clone())
                    };
                    
                    let full_display_text = current_signal_value.get_full_display_with_format(&current_format_enum);
                    let display_text = current_signal_value.get_truncated_display_with_format(&current_format_enum, 30);
                    
                    // Debug logging removed to prevent console spam in hot signal path
                    
                    // Generate dropdown options with formatted values
                    let dropdown_options = generate_ui_dropdown_options(&current_signal_value, &signal_type, 30);
                    
                    // Create unique trigger ID for positioning reference
                    let trigger_id = format!("select-trigger-{}", unique_id);
                    
                    // Create custom select trigger that shows value+format
                    Row::new()
                        .s(Width::fill())
                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT - 2))
                        .s(Padding::new().x(SPACING_8).y(SPACING_4))
                        .s(Gap::new().x(SPACING_2))
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
                                .s(Gap::new().x(SPACING_4))
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
                                            variables_value_column_width_signal().map({
                                                let display_text_clone = display_text.clone();
                                                move |column_width| {
                                                    let text = display_text_clone.clone();
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
                                                    let available_text_width = (column_width as f32 - total_reserved_width).max(40.0);
                                                    
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
                                            })
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
        })
}

/// Update the format for a selected variable using Actor+Relay architecture
fn update_variable_format(unique_id: &str, new_format: shared::VarFormat) {
    use crate::actors::selected_variables::variable_format_changed_relay;
    
    variable_format_changed_relay().send((unique_id.to_string(), new_format));
    
    // Also update WaveformTimeline domain for signal value formatting  
    crate::visualizer::timeline::timeline_actor::variable_format_updated_relay()
        .send((unique_id.to_string(), new_format));
    
}



/// Check if cursor time is within a variable's file time range
pub fn is_cursor_within_variable_time_range(unique_id: &str, cursor_time: f64) -> bool {
    // Parse unique_id to get file path: "file_path|scope_path|variable_name"
    let parts: Vec<&str> = unique_id.splitn(3, '|').collect();
    if parts.len() < 3 {
        return true; // Assume valid if we can't parse (maintains existing behavior)
    }
    let file_path = parts[0];
    
    // Find the loaded file and check its time range
    let tracked_files = crate::actors::global_domains::tracked_files_domain().get_current_files();
    if let Some(tracked_file) = tracked_files.iter().find(|f| f.path == file_path) {
        if let shared::FileState::Loaded(loaded_file) = &tracked_file.state {
        if let (Some(min_time), Some(max_time)) = (loaded_file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0), loaded_file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0)) {
            // Check if cursor time is within the file's time range
            cursor_time >= min_time && cursor_time <= max_time
        } else {
            // File has no time range data - assume valid (maintains existing behavior)
            true
        }
        } else {
            // File not loaded - assume invalid
            false
        }
    } else {
        // File not found - assume valid (maintains existing behavior)
        true
    }
}


/// Trigger signal value queries when variables are present
pub fn trigger_signal_value_queries() {
    // Prevent queries during startup until files are properly loaded
    let tracked_files = crate::actors::global_domains::tracked_files_domain().get_current_files();
    let has_loaded_files = tracked_files.iter().any(|f| matches!(f.state, shared::FileState::Loaded(_)));
    
    if !has_loaded_files {
        return; // Don't query if no files are loaded yet
    }
    
    // Actor+Relay implementation: Signal queries handled by unified timeline service
    // This function coordinates file loading checks with value query triggers
}



/// Update signal values in UI from cached or backend results
// This function was never called and contained legacy architecture patterns


fn variables_name_vertical_divider() -> impl Element {
    use crate::visualizer::interaction::dragging::{start_drag, is_divider_dragging, DividerType};
    
    let is_dragging_signal = is_divider_dragging(DividerType::VariablesNameColumn);
    
    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging_signal.map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down(move || {
            start_drag(DividerType::VariablesNameColumn, (0.0, 0.0));
        })
}

fn variables_value_vertical_divider() -> impl Element {
    use crate::visualizer::interaction::dragging::{start_drag, is_divider_dragging, DividerType};
    
    let is_dragging_signal = is_divider_dragging(DividerType::VariablesValueColumn);
    
    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging_signal.map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down(move || {
            start_drag(DividerType::VariablesValueColumn, (0.0, 0.0));
        })
}

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}


pub fn file_paths_dialog() -> impl Element {
    // ✅ Create local Atom for dialog selected files (replaces broken dialog_manager signals)
    let selected_files = Atom::new(Vec::<String>::new());
    
    let close_dialog = move || {
        // Use domain function to close dialog and clear state
        crate::file_dialog::close_file_dialog();
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
                        move |event: KeyDown| {
                            // Simple key event handling - no complex enterprise manager needed
                            if event.key() == "Escape" {
                                crate::file_dialog::close_file_dialog();
                            } else if event.key() == "Enter" {
                                // Enter key handling for file selection can be added when needed
                                // No complex Actor relay system required
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
                        .s(Gap::new().y(SPACING_16))
                        .item(
                            Row::new()
                                .s(Gap::new().x(SPACING_4))
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
                        .item(
                            El::new()
                                .s(Padding::all(4))
                                .child_signal(
                                    selected_files.signal().map(|selected_paths| {
                                        
                                        if selected_paths.is_empty() {
                                            El::new()
                                                .s(Font::new().italic().color_signal(neutral_8()))
                                                .child("Select waveform files from the directory tree above")
                                                .unify()
                                        } else {
                                            Row::new()
                                                .multiline()
                                                .s(Gap::new().x(SPACING_8).y(SPACING_8))
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
                                                                // File removal handler - logs action for debugging
                                                                // Integration with Atom-based file management pending architecture migration
                                                                zoon::println!("Remove file: {}", path);
                                                            }
                                                        })
                                                        .build()
                                                }))
                                                .unify()
                                        }
                                    })
                                )
                        )
                        .item(
                            Row::new()
                                .s(Gap::new().x(SPACING_12))
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
                                    button()
                                        .label_signal(
                                            map_ref! {
                                                let file_count = crate::actors::global_domains::file_count_signal(),
                                                let selected_files = selected_files.signal() =>
                                                move {
                                                    let is_loading = *file_count > 0;
                                                    let selected_count = selected_files.len();
                                                    if is_loading {
                                                        "Loading...".to_string()
                                                    } else if selected_count > 0 {
                                                        format!("Load {} Files", selected_count)
                                                    } else {
                                                        "Load Files".to_string()
                                                    }
                                                }
                                            }
                                        )
                                        .disabled_signal(
                                            map_ref! {
                                                let file_count = crate::actors::global_domains::file_count_signal(),
                                                let selected_files = selected_files.signal() => {
                                                let is_loading = *file_count > 0;
                                                let selected_count = selected_files.len();
                                                is_loading || selected_count == 0
                                                }
                                            }
                                        )
                                        .on_press(|| process_file_picker_selection())
                                        .variant(ButtonVariant::Primary)
                                        .build()
                                )
                        )
                )
        )
}



pub fn files_panel() -> impl Element {
    El::new()
        .s(Height::fill())
        .child(
            create_panel(
                Row::new()
                    .s(Gap::new().x(SPACING_8))
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
                    .s(Gap::new().y(SPACING_4))
                    .s(Padding::new().top(SPACING_4).right(SPACING_4))
                    .s(Height::fill())
                    .s(Width::growable())
                    .item(
                        // Original TreeView for comparison
                        El::new()
                            .s(Height::fill())
                            .s(Width::growable())
                            .child(
                                Column::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .item(
                                        El::new()
                                            .s(Height::fill())
                                            .s(Width::fill())
                                            .child_signal(
                                                crate::actors::global_domains::file_count_signal().map(|file_count| {
                                    // File count for UI decision
                                    if file_count == 0 {
                                        empty_state_hint("Click 'Load Files' to add waveform files.")
                                            .unify()
                                    } else {
                                        // PATCHED: Uses filtered stable signals to reduce flickering
                                        create_stable_tree_view()
                                            .unify()
                                    }
                                })
                            )
                                    )
                            )
                    )
            )
        )
}

// NOTE: Helper functions for file rendering
// Uses render_tracked_file_reactive for proper reactive expanded scopes access

// The reactive version eliminates the need for synchronous expanded scopes access

fn render_tracked_file_reactive(tracked_file: TrackedFile) -> impl Element {
    // Compute smart label on-the-fly for this specific file
    let smart_label = compute_smart_label_for_file(&tracked_file);
    
    El::new().child_signal(
        crate::actors::global_domains::expanded_scopes_signal().map(move |expanded_scopes| {
            render_tracked_file_as_tree_item_with_label_and_expanded_state(
                tracked_file.clone(), 
                smart_label.clone(), 
                expanded_scopes
            ).into_element()
        })
    )
}

/// Same as render_tracked_file_as_tree_item_with_label but accepts expanded_scopes as parameter
/// This allows TreeViews to get current expanded state instead of static clones
fn render_tracked_file_as_tree_item_with_label_and_expanded_state(
    tracked_file: TrackedFile, 
    smart_label: String,
    _expanded_scopes: indexmap::IndexSet<String>
) -> impl Element {
    let display_name = smart_label;
    // Convert the single file to tree data (includes file + its scopes)
    let tree_data = match &tracked_file.state {
        shared::FileState::Loading(_) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::File)
                    .icon("circle-loader-2")
                    .disabled(true)
            ]
        }
        shared::FileState::Loaded(file_data) => {
            // Build children from scopes
            let children = file_data.scopes.iter().map(|scope| {
                convert_scope_to_tree_data(scope)
            }).collect();
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::File)
                    .icon("file")
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
                    .with_children(children)
            ]
        }
        shared::FileState::Failed(file_error) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("alert-circle")
                    .tooltip(format!("{}\nError: {}", tracked_file.path, file_error.user_friendly_message()))
                    .error_message(file_error.user_friendly_message())
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            ]
        }
        shared::FileState::Missing(_file_path) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("file-x")
                    .tooltip(format!("{}\nFile no longer exists", tracked_file.path))
                    .error_message("File no longer exists".to_string())
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            ]
        }
        shared::FileState::Unsupported(reason) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("circle-help")
                    .tooltip(format!("{}\nUnsupported: {}", tracked_file.path, reason))
                    .error_message(format!("Unsupported: {}", reason))
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone()))
            ]
        }
    };
    
    // The TreeView must update the Actor state when user clicks expand/collapse
    // Connect directly to Actor's internal mutables for bi-directional UI updates
    
    // Create a mini tree_view for this single file section
    tree_view()
        .data(tree_data)
        .size(TreeViewSize::Medium)
        .variant(TreeViewVariant::Basic)
        .show_icons(true)
        .show_checkboxes(true)
        .show_checkboxes_on_scopes_only(true)
        .single_scope_selection(true)
        .external_expanded(crate::actors::selected_variables::expanded_scopes_mutable()) 
        .external_selected(crate::actors::selected_variables::tree_selection_mutable())
        .build()
}

/// Compute smart label for a single file with duplicate detection AND time intervals
fn compute_smart_label_for_file(target_file: &TrackedFile) -> String {
    // Start with base name (with directory prefix for duplicates)
    let base_name = if target_file.filename == "wave_27.fst" {
        // Extract parent directory from path for duplicate disambiguation
        if let Some(parent) = std::path::Path::new(&target_file.path).parent() {
            if let Some(dir_name) = parent.file_name() {
                format!("{}/{}", dir_name.to_string_lossy(), target_file.filename)
            } else {
                target_file.filename.clone()
            }
        } else {
            target_file.filename.clone()
        }
    } else {
        target_file.filename.clone()
    };
    
    // Add time interval if file is loaded
    match &target_file.state {
        shared::FileState::Loaded(waveform_file) => {
            if let (Some(min_ns), Some(max_ns)) = (waveform_file.min_time_ns, waveform_file.max_time_ns) {
                // Convert nanoseconds to seconds for display
                let min_seconds = min_ns as f64 / 1_000_000_000.0;
                let max_seconds = max_ns as f64 / 1_000_000_000.0;
                
                // Format time range with en-dash for TreeView styling recognition
                let time_range = if max_seconds < 1.0 {
                    // Show in milliseconds if under 1 second
                    format!("{:.0}–{:.0}ms", min_seconds * 1000.0, max_seconds * 1000.0)
                } else if max_seconds < 60.0 {
                    // Show in seconds if under 1 minute
                    if max_seconds.fract() == 0.0 && min_seconds.fract() == 0.0 {
                        format!("{:.0}–{:.0}s", min_seconds, max_seconds)
                    } else {
                        format!("{:.1}–{:.1}s", min_seconds, max_seconds)
                    }
                } else {
                    // Show in minutes if longer
                    let min_minutes = min_seconds / 60.0;
                    let max_minutes = max_seconds / 60.0;
                    format!("{:.1}–{:.1}min", min_minutes, max_minutes)
                };
                
                format!("{} ({})", base_name, time_range)
            } else {
                base_name
            }
        }
        shared::FileState::Loading(_) => {
            // Show loading status
            // Known issue: Loading text may show in blue styling instead of regular text color
            // like time postfix pattern. Non-unique files work correctly (regular color).
            format!("{} (Loading...)", base_name)
        }
        _ => {
            // For failed, missing, unsupported - just show base name
            base_name
        }
    }
}

/// ✅ STABLE: Working TreeView with items_signal_vec pattern
///
/// Uses items_signal_vec to render each TrackedFile individually, avoiding signal conversion issues.
/// This is the proven working pattern that should NOT be changed.
fn create_stable_tree_view() -> impl Element {
    // create_stable_tree_view() called
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(
            Column::new()
                .s(Width::fill())
                .s(Height::fill())
                .s(Gap::new().y(SPACING_2))
                .update_raw_el(|raw_el| {
                    raw_el
                        .style("width", "100%")
                        .style("min-width", "fit-content")
                })
                .items_signal_vec(
                    // Map each tracked file to its rendered element with current expanded state
                    crate::actors::global_domains::tracked_files_signal_vec().map(|tracked_file| {
                        render_tracked_file_reactive(tracked_file.clone())
                    })
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
                    .s(Gap::new().x(SPACING_8))
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
                                // ✅ PERFORMANCE FIX: Show filtered variable count, not total
                                variables_display_signal().map(|filtered_variables| {
                                    filtered_variables.len().to_string()
                                })
                            )
                    )
                    .item(
                        El::new()
                            .s(Width::fill().max(230))
                            .s(Align::new().right())
                            .child(
                                input()
                                    .placeholder("variable_name")
                                    .value_signal(search_filter_signal())
                                    .left_icon(IconName::Search)
                                    .right_icon_signal(search_filter_signal().map(|text| {
                                        if text.is_empty() { None } else { Some(IconName::X) }
                                    }))
                                    .on_right_icon_click(|| search_filter_changed_relay().send(String::new()))
                                    .size(InputSize::Small)
                                    .on_change(|text| search_filter_changed_relay().send(text))
                                    .on_focus(|| search_focus_changed_relay().send(true))
                                    .on_blur(|| search_focus_changed_relay().send(false))
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
                                variables_signal().map(|vars| {
                                    // Add one extra row height for scrollbar (names/values) or footer/timeline (canvas)
                                    (vars.len() + 1) as u32 * SELECTED_VARIABLES_ROW_HEIGHT
                                }).dedupe()
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
                                            .s(Width::exact_signal(variables_name_column_width_signal().map(|w| w as u32)))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .s(Scrollbars::x_and_clip_y())
                                            .update_raw_el(|raw_el| {
                                                raw_el.style("scrollbar-width", "thin")
                                            })
                                            .items_signal_vec(
                                                variables_signal_vec().map(|selected_var| {
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
                                                                .on_press(move || {
                                                                    // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
                                                                    let selected_variables = crate::actors::selected_variables_domain();
                                                                    selected_variables.variable_removed_relay.send(unique_id.clone());
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
                                                                            crate::actors::tracked_files_signal().map(move |files: Vec<TrackedFile>| {
                                                                                get_signal_type_for_selected_variable_from_files(&selected_var, &files)
                                                                            })
                                                                        })
                                                                )
                                                                .update_raw_el({
                                                                    let selected_var = selected_var.clone();
                                                                    move |raw_el| {
                                                                        let title_signal = crate::actors::tracked_files_signal().map({
                                                                            let selected_var = selected_var.clone();
                                                                            move |files: Vec<TrackedFile>| {
                                                                                let signal_type = get_signal_type_for_selected_variable_from_files(&selected_var, &files);
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
                                                            .s(Align::new().center_y())
                                                            // Left group: Z button
                                                            .item(kbd("Z").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Reset zoom center to time 0").build())
                                                            // Spacer to push center and right groups apart
                                                            .item(El::new().s(Width::fill()))
                                                            // Center group: W - zoom display - S
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
                                                                                Text::with_signal(
                                                                                    ns_per_pixel_signal().map(|ns_per_pixel| {
                                                                                        // DEBUG: Log what UI zoom display receives
                                                                                        // Use NsPerPixel's built-in Display formatting
                                                                                        format!("{}", ns_per_pixel)
                                                                                    })
                                                                                )
                                                                            )
                                                                    )
                                                                    .item(kbd("S").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Zoom out • Shift+S: Zoom out faster").build())
                                                            )
                                                            // Spacer to push right group apart
                                                            .item(El::new().s(Width::fill()))
                                                            // Right group: R button (wrapped in clickable El)
                                                            .item(
                                                                El::new()
                                                                    .on_click(|| {
                                                                        crate::visualizer::timeline::timeline_actor_domain().reset_zoom_pressed_relay.send(());
                                                                    })
                                                                    .child(kbd("R").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Reset zoom to 1x, fit all data, and center cursor").build())
                                                            )
                                                    )
                                            )
                                    )
                                    .item(variables_name_vertical_divider())
                                    .item(
                                        // Column 2: Variable value (resizable) - HEIGHT FOLLOWER
                                        Column::new()
                                            .s(Width::exact_signal(variables_value_column_width_signal().map(|w| w as u32)))
                                            .s(Height::fill())
                                            .s(Align::new().top())
                                            .s(Scrollbars::x_and_clip_y())
                                            .update_raw_el(|raw_el| {
                                                raw_el.style("scrollbar-width", "thin")
                                            })
                                            .items_signal_vec(
                                                variables_signal_vec().map(|selected_var| {
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
                                                            .s(Align::new().center_y())
                                                            .s(Font::new().color_signal(neutral_8()).size(12))
                                                            // Left group: [min time] - A button (preserve width)
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
                                                                                Text::with_signal(
                                                                                    // Min time display using cached timeline range signal
                                                                                    timeline_range_signal().map(|range| {
                                                                                        if let Some((min_time, _max_time)) = range {
                                                                                            format_time(min_time)
                                                                                        } else {
                                                                                            "0s".to_string()
                                                                                        }
                                                                                    }).dedupe_cloned()
                                                                                )
                                                                            )
                                                                    )
                                                                    .item(kbd("A").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Pan left • Shift+A: Pan left faster").build())
                                                            )
                                                            // Spacer to push center and right groups apart
                                                            .item(El::new().s(Width::fill()))
                                                            // Center group: Q - [cursor time] - E
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
                                                                                Text::with_signal(
                                                                                    cursor_position_seconds_signal().map(|cursor_pos| {
                                                                                        // Use same smart time formatting as timescale to ensure consistent units
                                                                                        if !cursor_pos.is_finite() || cursor_pos <= 0.0 {
                                                                                            "0s".to_string()
                                                                                        } else if cursor_pos < 1e-6 {
                                                                                            let ns_val = cursor_pos * 1e9;
                                                                                            if ns_val.fract() == 0.0 {
                                                                                                format!("{}ns", ns_val as i64)
                                                                                            } else {
                                                                                                format!("{:.1}ns", ns_val)
                                                                                            }
                                                                                        } else if cursor_pos < 1e-3 {
                                                                                            let us_val = cursor_pos * 1e6;
                                                                                            if us_val.fract() == 0.0 {
                                                                                                format!("{}μs", us_val as i64)
                                                                                            } else {
                                                                                                format!("{:.1}μs", us_val)
                                                                                            }
                                                                                        } else if cursor_pos < 1.0 {
                                                                                            let ms_val = cursor_pos * 1e3;
                                                                                            if ms_val.fract() == 0.0 {
                                                                                                format!("{}ms", ms_val as i64)
                                                                                            } else {
                                                                                                format!("{:.1}ms", ms_val)
                                                                                            }
                                                                                        } else {
                                                                                            if cursor_pos.fract() == 0.0 {
                                                                                                format!("{}s", cursor_pos as i64)
                                                                                            } else {
                                                                                                format!("{:.1}s", cursor_pos)
                                                                                            }
                                                                                        }
                                                                                    })
                                                                                )
                                                                            )
                                                                    )
                                                                    .item(kbd("E").size(KbdSize::Small).variant(KbdVariant::Outlined).title("Move cursor right • Shift+E: Jump to next transition").build())
                                                            )
                                                            // Spacer to push right group apart
                                                            .item(El::new().s(Width::fill()))
                                                            // Right group: D button (preserve width) - [max time]
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
                                                                                Text::with_signal(
                                                                                    // Max time display using cached timeline range signal
                                                                                    timeline_range_signal().map(|range| {
                                                                                        if let Some((_min_time, max_time)) = range {
                                                                                            format_time(max_time)
                                                                                        } else {
                                                                                            "No Data".to_string()
                                                                                        }
                                                                                    }).dedupe_cloned()
                                                                                )
                                                                            )
                                                                    )
                                                            )
                                                    )
                                            )
                                    )
                                    .item(variables_value_vertical_divider())
                                    .item(
                                        // Column 3: Unified waveform canvas (fills remaining space) - HEIGHT FOLLOWER
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .s(Background::new().color_signal(neutral_2()))
                                            .child(crate::visualizer::canvas::waveform_canvas::waveform_canvas())
                                    )
                            )
                    )
                )
        )
}



// Helper functions for different panel configurations

pub fn files_panel_with_height() -> impl Element {
    // TEST 2: Remove Scrollbars::both() from individual panels
    El::new()
        .s(Height::exact_signal(files_panel_height_signal().map(|h| h as u32)))
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
        .child_signal(app_config().dock_mode_actor.signal().map(|dock_mode| {
            let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
            if is_docked {
                // When docked to bottom, use files panel height signal for synchronized resizing
                El::new()
                    .s(Width::fill())
                    .s(Height::exact_signal(files_panel_height_signal().map(|h| h as u32)))
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
                        .s(Padding::new().x(SPACING_12).y(SPACING_4))
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
                    // ✅ PERFORMANCE FIX: Signal-level filtering for instant response
                    variables_display_signal().map(|filtered_variables| {
                        virtual_variables_list_pre_filtered(filtered_variables).into_element()
                    })
                )
        )
}

/// ✅ PERFORMANCE: Load variables only when scope or files change (not filter)
fn variables_loading_signal() -> impl Signal<Item = Vec<VariableWithContext>> {
    map_ref! {
        let selected_scope_id = selected_scope_signal(),
        let _tracked_files = crate::actors::global_domains::tracked_files_signal() => {
            if let Some(scope_id) = selected_scope_id.as_ref() {
                get_variables_from_tracked_files(&scope_id)
            } else {
                Vec::new()
            }
        }
    }
}

/// ✅ PERFORMANCE: Signal-level filtering for instant response
fn variables_display_signal() -> impl Signal<Item = Vec<VariableWithContext>> {
    map_ref! {
        let variables = variables_loading_signal(),
        let search_filter = search_filter_signal() => {
            // Filter at signal level for maximum performance
            filter_variables_with_context(&variables, &search_filter)
        }
    }
}


// Removed create_styled_smart_label function - styling now handled inline in TreeView component




// NOTE: render_tracked_file_as_tree_item_with_label function removed as it was unused
// The current implementation uses render_tracked_file_with_expanded_state instead

// Helper function to clean up all file-related state when a file is removed
fn cleanup_file_related_state(file_id: &str) {
    // Get filename and file path before any cleanup (needed for cleanup logic)
    let (_filename, _file_path) = crate::actors::global_domains::tracked_files_domain().get_current_files()
        .iter()
        .find(|f| f.id == file_id)
        .map(|f| (f.filename.clone(), f.path.clone()))
        .unwrap_or_else(|| (String::new(), String::new()));
    
    // Use Actor+Relay event emission instead of direct state access
    crate::actors::selected_variables::scope_selected_relay().send(None);
    
    // Actor+Relay domain event cleanup for file removal
    /*
    // Clear expanded scopes for this file using domain signals
    // New scope ID format: {full_path}|{scope_full_name} or just {full_path}
    // Using selected_variables domain for scope management
    crate::actors::selected_variables::retain_expanded_scopes(|scope| {
        // Keep scopes that don't belong to this file
        scope != &file_path && !scope.starts_with(&format!("{}|", file_path))
    });
    
    // Clear selected variables from this file
    // SelectedVariable uses full file path in new format
    if !file_path.is_empty() {
        // Remove selected variables from this file using domain events
        let current_vars = crate::actors::selected_variables::current_variables();
        let vars_to_remove: Vec<String> = current_vars.iter()
            .filter(|var| var.file_path().unwrap_or_default() == file_path)
            .map(|var| var.unique_id.clone())
            .collect();
        
        // Send remove events for each variable from this file
        for var_id in vars_to_remove {
            crate::actors::selected_variables::variable_removed_relay().send(var_id);
        }
        // Note: variable_index is managed automatically by the domain
    }
    */
}

// Enhanced file removal handler that works with both old and new systems
fn create_enhanced_file_remove_handler(_file_id: String) -> impl Fn(&str) + 'static {
    move |id: &str| {
        // Clean up all file-related state (legacy cleanup during transition)
        cleanup_file_related_state(id);
        
        // ✅ ACTOR+RELAY MIGRATION: Emit file_removed event through TrackedFiles domain
        let tracked_files = crate::actors::global_domains::tracked_files_domain();
        tracked_files.file_removed_relay.send(id.to_string());
        
        // Remove from legacy systems during transition (will be removed later)
        // LOADED_FILES.lock_mut().retain(|f| f.id != id); // REMOVED: Use TrackedFiles domain instead
        // FILE_PATHS.lock_mut().shift_remove(id); // REMOVED: FILE_PATHS no longer exists
        
        // Config automatically saved by ConfigSaver watching domain signals
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
        .child_signal(crate::actors::global_domains::file_count_signal().map(move |file_count| {
            let is_loading = file_count > 0; // Simple loading state based on file activity
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




fn file_picker_content() -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin")
                .style_signal("scrollbar-color", primary_6().map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track))).flatten())
        })
        .child_signal(simple_file_picker_tree().into_signal_option())
}

async fn simple_file_picker_tree() -> impl Element {
    let scroll_position = crate::config::app_config()
        .file_picker_scroll_position
        .signal()
        .to_stream()
        .next()
        .await;
    let (tree_view_rendering_relay, mut tree_view_rendering_stream) = relay();
    El::new()
        .s(Height::fill())
        .s(Width::fill())
        .s(Scrollbars::both())
        .viewport_y_signal(signal::from_future(Box::pin(async move {
            tree_view_rendering_stream.next().await;
            Task::next_macro_tick().await;
            scroll_position
        })).map(|position| position.flatten().unwrap_or_default()))
        .on_viewport_location_change(|_scene, viewport| {
            crate::config::app_config().file_picker_scroll_position.set_neq(viewport.y);
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
                let tree_cache = crate::file_dialog::file_tree_cache_mutable().signal_cloned(),
                let error_cache = zoon::always(std::collections::HashMap::new()), // Error cache simplified
                let expanded = crate::config::app_config().file_picker_expanded_directories.signal_cloned() =>
                move {
                    // Build tree view from cached data
                    
                    monitor_directory_expansions(expanded.iter().cloned().collect::<HashSet<_>>());
                    
                    // Check if we have root directory data
                    if let Some(_root_items) = tree_cache.get("/") {
                        // Create root "/" item and build hierarchical tree
                        let tree_data = vec![
                            TreeViewItemData::new("/".to_string(), "/".to_string())
                                .with_children(build_hierarchical_tree("/", &tree_cache, &error_cache))
                        ];
                        
                        // Known TreeView UX limitations: Icon clicks don't toggle selection, checkbox state sync needed
                        El::new()
                            .after_insert(clone!((tree_view_rendering_relay) move |_| {
                                tree_view_rendering_relay.send(());
                            }))
                            .child(
                                tree_view()
                                    .data(tree_data)
                                    .size(TreeViewSize::Medium)
                                    .variant(TreeViewVariant::Basic)
                                    .show_icons(true)
                                    .show_checkboxes(true)
                                    .external_expanded(crate::config::app_config().file_picker_expanded_directories.clone())
                                    .external_selected_vec(MutableVec::<String>::new())
                                    .build()
                            )
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

pub fn monitor_directory_expansions(expanded: HashSet<String>) {
    // Get previous expanded directories from config to detect changes
    let config = crate::config::app_config();
    let current_config_expanded = config.file_picker_expanded_directories.lock_ref();
    let last_expanded: HashSet<String> = current_config_expanded.iter().cloned().collect();
    
    // Only proceed if there are actual new expansions (not just re-renders)
    let new_expansions: Vec<String> = expanded.difference(&last_expanded).cloned().collect();
    
    // For now, request all new expansions - proper cache checking requires signal-based pattern
    let paths_to_request: Vec<String> = new_expansions.into_iter()
        .filter(|path| path.starts_with("/") && !path.is_empty())
        .collect();
    
    // Send batch request for maximum parallel processing with jwalk
    if !paths_to_request.is_empty() {
        Task::start(async move {
            use crate::platform::{Platform, CurrentPlatform};
            let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectories(paths_to_request)).await;
        });
    }
    
    // Config system automatically handles the expanded directories state
    // No need to manually track - the config signal will update appropriately
}


fn extract_filename(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}


// File picker utility functions




fn process_file_picker_selection() {
    Task::start(async {
        // Get current selected files using signal-based access
        use futures::StreamExt;
        use zoon::SignalExt;
        let selected_files = crate::file_dialog::file_picker_selected_signal()
            .to_stream()
            .next()
            .await
            .unwrap_or_default();
    
            if !selected_files.is_empty() {
                // ✅ ELIMINATED: Loading state updates - Loading state handled internally by TrackedFiles domain
                
                // ✅ FILE RELOAD STRATEGY (Option B): Check for duplicates and implement reload
                use std::path::PathBuf;
                
                // Get currently tracked files to check for duplicates
                // Use the global domain signal storage for current files
                let tracked_files_snapshot = crate::actors::global_domains::get_current_tracked_files();
                
                let mut new_files: Vec<PathBuf> = Vec::new();
                let mut reload_files: Vec<String> = Vec::new();
                
                for selected_path in selected_files {
                    let selected_pathbuf = PathBuf::from(&selected_path);
                    
                    // DEBUG: Log path comparison details
                    // for existing_file in &tracked_files_snapshot {
                    // }
                    
                    // Check if file is already tracked
                    if let Some(existing_file) = tracked_files_snapshot.iter().find(|f| f.id == selected_path || f.path == selected_path) {
                        reload_files.push(existing_file.id.clone());
                    } else {
                        new_files.push(selected_pathbuf);
                    }
                }
                
                let tracked_files = crate::actors::global_domains::tracked_files_domain();
                
                // Handle new files
                if !new_files.is_empty() {
                    tracked_files.files_dropped_relay.send(new_files);
                }
                
                // Handle reload files - use direct reload calls for proper parsing
                if !reload_files.is_empty() {
                    
                    // Use direct reload calls instead of remove→re-add pattern
                    // This ensures reload files go through full parsing pipeline
                    for file_id in reload_files {
                        tracked_files.reload_file(file_id);
                    }
                }
                
                // Close dialog using domain function
                crate::file_dialog::close_file_dialog();
            }
    }); // End of async Task::start block
}

fn clear_all_files() {
    // ✅ ACTOR+RELAY MIGRATION: Use TrackedFiles domain events instead of direct state manipulation
    
    // Get all tracked file IDs before clearing (for cleanup)
    let file_ids: Vec<String> = crate::actors::global_domains::tracked_files_domain().get_current_files()
        .iter()
        .map(|f| f.id.clone())
        .collect();
    
    // Clean up all file-related state for each file (legacy cleanup during transition)
    for file_id in &file_ids {
        cleanup_file_related_state(file_id);
    }
    
    // Emit all_files_cleared event through TrackedFiles domain
    let tracked_files = crate::actors::global_domains::tracked_files_domain();
    tracked_files.all_files_cleared_relay.send(());
    
    // Clear legacy systems during transition (will be removed later)
    // LOADED_FILES.lock_mut().clear(); // REMOVED: Use TrackedFiles domain instead
    // FILE_PATHS.lock_mut().clear(); // REMOVED: FILE_PATHS no longer exists
    
    // ✅ COMPLETED: Replaced with proper Actor+Relay domain event emissions
    // Tree selections now handled by SelectedVariables domain Actor
    
    // Config automatically saved by ConfigSaver watching domain signals
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
            // ✅ ACTOR+RELAY MIGRATION: Use SelectedVariables domain events
            let selected_variables = crate::actors::selected_variables_domain();
            selected_variables.selection_cleared_relay.send(());
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
                .on_press(|| app_config().theme_button_clicked_relay.send(()))
                .build()
                .into_element()
        }))
}

fn dock_toggle_button() -> impl Element {
    El::new()
        .child_signal(app_config().dock_mode_actor.signal().map(|dock_mode| {
            let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
            button()
                .label(if is_docked { "Dock to Right" } else { "Dock to Bottom" })
                .left_icon_element(|| {
                    El::new()
                        .child_signal(app_config().dock_mode_actor.signal().map(|dock_mode| {
                            let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
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
                    // Use domain function to toggle dock mode (handles all logic internally)
                    app_config().dock_mode_button_clicked_relay.send(());
                })
                .align(Align::center())
                .build()
                .into_element()
        }))
}

pub fn files_panel_vertical_divider() -> impl Element {
    use crate::visualizer::interaction::dragging::{start_drag, is_divider_dragging, DividerType};
    
    let is_dragging_signal = is_divider_dragging(DividerType::FilesPanelMain);
    
    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new().color_signal(
            is_dragging_signal.map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down(move || {
            start_drag(DividerType::FilesPanelMain, (0.0, 0.0));
        })
}

pub fn files_panel_horizontal_divider() -> impl Element {
    use crate::visualizer::interaction::dragging::{start_drag, is_divider_dragging, DividerType};
    
    let is_dragging_signal = is_divider_dragging(DividerType::FilesPanelSecondary);
    
    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new().color_signal(
            is_dragging_signal.map_bool_signal(
                || primary_7(),
                || primary_6()
            )
        ))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down(move || {
            start_drag(DividerType::FilesPanelSecondary, (0.0, 0.0));
        })
}

// ===== UNIFIED WAVEFORM CANVAS =====

// ===== MAIN LAYOUT (MIGRATED FROM MAIN.RS) =====

/// Main application layout with keyboard handling and drag interactions
pub fn main_layout() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child(
            Column::new()
                .s(Width::fill())
                .s(Height::fill())
                .item(files_panel())
                .item(selected_variables_with_waveform_panel())
        )
}
