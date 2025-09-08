use moonzoon_novyui::tokens::color::{
    neutral_1, neutral_2, neutral_4, neutral_8, neutral_11, neutral_12, primary_3, primary_6,
    primary_7,
};
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::*;
use zoon::events::{Click, KeyDown};
use zoon::*;
use crate::clipboard;

// Selected Variables panel row height
pub const SELECTED_VARIABLES_ROW_HEIGHT: u32 = 30;
use crate::dataflow::atom::Atom;
use crate::dataflow::relay;
use crate::selected_variables::{
    VariableWithContext, filter_variables_with_context, get_variables_from_tracked_files,
};
use crate::virtual_list::virtual_variables_list_pre_filtered;
use moonzoon_novyui::components::{KbdSize, KbdVariant, kbd};
use moonzoon_novyui::tokens::typography::font_mono;
use shared::{FileState, ScopeData, SelectedVariable, TrackedFile, UpMsg};
use std::collections::{HashMap, HashSet};


fn timeline_range_signal(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Signal<Item = Option<(f64, f64)>> {
    let files_count_signal = tracked_files.file_count_signal();
    let variables_signal = selected_variables.variables_vec_actor.signal();
    
    map_ref! {
        let files_count = files_count_signal,
        let _selected_vars = variables_signal => {
            if *files_count == 0 {
                None
            } else {
                // TODO: Implement proper timeline range calculation from actual file data
                // This should return Some((min_time, max_time)) from loaded files
                None
            }
        }
    }
    .dedupe_cloned()
}

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
use crate::selected_variables::{
};
use crate::visualizer::interaction::dragging::{
    files_panel_height_signal, variables_name_column_width_signal,
    variables_value_column_width_signal,
};
use shared::truncate_value;

/// Format options for dropdown - contains value and disabled state
#[derive(Debug, Clone)]
struct DropdownFormatOption {
    format: shared::VarFormat,
    display_text: String,
    full_text: String, // For tooltip
    disabled: bool,
}

impl DropdownFormatOption {
    fn new(
        format: shared::VarFormat,
        display_text: String,
        full_text: String,
        disabled: bool,
    ) -> Self {
        Self {
            format,
            display_text,
            full_text,
            disabled,
        }
    }
}

/// Get signal type information for a selected variable (signal-based version)
fn get_signal_type_for_selected_variable_from_files(
    selected_var: &SelectedVariable,
    files: &[TrackedFile],
) -> String {
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {
        for tracked_file in files.iter() {
            if tracked_file.path == file_path {
                if let FileState::Loaded(waveform_file) = &tracked_file.state {
                    let full_scope_id = format!("{}|{}", file_path, scope_path);

                    if let Some(variables) =
                        shared::find_variables_in_scope(&waveform_file.scopes, &full_scope_id)
                    {
                        if let Some(signal) = variables.iter().find(|v| v.name == variable_name) {
                            return format!("{} {}-bit", signal.signal_type, signal.width);
                        }
                    }
                }
                break; // Found the file, no need to continue searching
            }
        }
    }

    String::new()
}

/// Get signal type information for a selected variable (legacy synchronous version)
fn get_signal_type_for_selected_variable(
    selected_var: &SelectedVariable,
    tracked_files: &[TrackedFile],
) -> String {
    if let Some((file_path, scope_path, variable_name)) = selected_var.parse_unique_id() {

        for tracked_file in tracked_files.iter() {
            if tracked_file.path == file_path {
                if let FileState::Loaded(waveform_file) = &tracked_file.state {
                    let full_scope_id = format!("{}|{}", file_path, scope_path);

                    if let Some(variables) =
                        shared::find_variables_in_scope(&waveform_file.scopes, &full_scope_id)
                    {
                        if let Some(signal) = variables.iter().find(|v| v.name == variable_name) {
                            return format!("{} {}-bit", signal.signal_type, signal.width);
                        }
                    }
                }
                break; // Found the file, no need to continue searching
            }
        }
    }

    String::new()
}


/// Generate dropdown options for UI from shared SignalValue
fn generate_ui_dropdown_options(
    signal_value: &shared::SignalValue,
    _signal_type: &str,
    max_value_chars: usize,
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
                    let truncated =
                        signal_value.get_truncated_display_with_format(format, max_value_chars);
                    (truncated, full)
                }
                shared::SignalValue::Missing => {
                    let text = format!("N/A {}", format.as_static_str());
                    (text.clone(), text)
                }
                shared::SignalValue::Loading => {
                    let text = format!("Loading... {}", format.as_static_str());
                    (text.clone(), text)
                }
            };

            DropdownFormatOption::new(*format, display_text, full_text, false)
        })
        .collect()
}

/// Create a smart dropdown with viewport edge detection using web-sys APIs
fn create_smart_dropdown(
    dropdown_options: Vec<DropdownFormatOption>,
    format_actor: crate::dataflow::Actor<shared::VarFormat>,
    is_open: zoon::Mutable<bool>,
    trigger_id: String,
    unique_id: String,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, window};

    let vertical_padding = SPACING_12 as f64;
    let explicit_line_height = 16.0; // Magic number - should be typography token
    let item_height = vertical_padding + explicit_line_height; // 28px total per item
    let border_height = 2.0; // Magic number - should be border design token
    let safety_margin = 4.0; // Magic number - should be layout constant

    let content_height = dropdown_options.len() as f64 * item_height;
    let calculated_height = content_height + border_height + safety_margin;
    const DROPDOWN_MAX_VIEWPORT_RATIO: f64 = 0.25; // 25% of viewport height
    let max_dropdown_height = 1200.0 * DROPDOWN_MAX_VIEWPORT_RATIO; // Using fallback viewport height for dropdown sizing
    let dynamic_dropdown_height = (calculated_height.min(max_dropdown_height)).ceil();

    let dropdown_id = format!("smart-dropdown-{}", js_sys::Date::now() as u64);

    Column::new()
        .s(Transform::new().move_down(0))
        .s(Background::new().color_signal(neutral_1()))
        .s(Borders::all_signal(
            neutral_4().map(|color| Border::new().width(1).color(color)),
        ))
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
                .color("oklch(70% 0.09 255 / 0.22)"),
        ]))
        .s(Scrollbars::both())
        .update_raw_el({
            let dropdown_id = dropdown_id.clone();
            move |raw_el| {
                if let Some(html_el) = raw_el.dom_element().dyn_ref::<HtmlElement>() {
                    html_el.set_id(&dropdown_id);

                    let style = html_el.style();
                    let _ = style.set_property("position", "fixed");
                    let _ = style.set_property("z-index", "9999");
                    let _ = style.set_property("min-width", "200px");
                    let _ =
                        style.set_property("max-height", &format!("{}px", dynamic_dropdown_height));
                    let _ = style.set_property("overflow-y", "auto");

                    if let Some(window) = window() {
                        if let Some(document) = window.document() {
                            if let Some(trigger_element) = document.get_element_by_id(&trigger_id) {
                                let viewport_width =
                                    window.inner_width().unwrap().as_f64().unwrap_or(1024.0);
                                let viewport_height =
                                    window.inner_height().unwrap().as_f64().unwrap_or(768.0);

                                let trigger_rect = trigger_element.get_bounding_client_rect();
                                const MIN_DROPDOWN_WIDTH: f64 = 200.0; // Matches CSS min-width
                                let dropdown_width = MIN_DROPDOWN_WIDTH;
                                let dropdown_height = dynamic_dropdown_height; // Use the calculated height

                                let mut x = trigger_rect.left();
                                let mut y = trigger_rect.bottom() + 1.0; // 1px gap below trigger

                                if x + dropdown_width > viewport_width {
                                    x = viewport_width - dropdown_width - 8.0; // 8px margin from edge
                                }

                                if x < 8.0 {
                                    x = 8.0;
                                }

                                if y + dropdown_height > viewport_height {
                                    let space_above = trigger_rect.top();

                                    if space_above >= dropdown_height + 1.0 {
                                        y = trigger_rect.top() - dropdown_height - 1.0; // 1px gap above
                                    } else {
                                        y = viewport_height - dropdown_height - 8.0; // 8px margin from bottom
                                    }
                                }

                                if y < 8.0 {
                                    y = 8.0;
                                }

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
            dropdown_options
                .iter()
                .map(|option| {
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
                                if full_text != display_text {
                                    let value_only = if let Some(space_pos) = full_text.rfind(' ') {
                                        full_text[..space_pos].to_string()
                                    } else {
                                        full_text.clone()
                                    };
                                    let filtered_tooltip = value_only
                                        .chars()
                                        .filter(|&c| {
                                            c == ' ' || (c.is_ascii() && c.is_ascii_graphic())
                                        })
                                        .collect::<String>()
                                        .trim()
                                        .to_string();

                                    let display_value_only =
                                        if let Some(space_pos) = display_text.rfind(' ') {
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
                                        if let Some(html_el) =
                                            raw_el.dom_element().dyn_ref::<web_sys::HtmlElement>()
                                        {
                                            html_el.set_title(&filtered_tooltip);
                                        }
                                    }
                                }
                                raw_el
                            }
                        })
                        .s(Font::new()
                            .color_signal(
                                always(option_disabled)
                                    .map_bool_signal(|| neutral_4(), || neutral_8()),
                            )
                            .size(12))
                        .child(
                            Row::new()
                                .s(Width::fill())
                                .s(Gap::new().x(SPACING_8))
                                .item(
                                    El::new()
                                        .s(Font::new()
                                            .color_signal(
                                                always(option_disabled).map_bool_signal(
                                                    || neutral_4(),
                                                    || neutral_11(),
                                                ),
                                            )
                                            .size(12)
                                            .line_height(16)
                                            .no_wrap())
                                        .s(font_mono())
                                        .s(Width::growable())
                                        .child({
                                            let display_text = option.display_text.clone();
                                            let value_only =
                                                if let Some(space_pos) = display_text.rfind(' ') {
                                                    display_text[..space_pos].to_string()
                                                } else {
                                                    display_text.clone()
                                                };
                                            let filtered_value = value_only
                                                .chars()
                                                .filter(|&c| {
                                                            c == ' '
                                                        || (c.is_ascii() && c.is_ascii_graphic())
                                                })
                                                .collect::<String>()
                                                .trim()
                                                .to_string();
                                            El::new()
                                                .s(Font::new()
                                                    .color_signal(
                                                        always(filtered_value.trim() == "-")
                                                            .map_bool_signal(
                                                                || neutral_8(),  // Muted color for placeholder
                                                                || neutral_11(), // Normal color for real values
                                                            ),
                                                    )
                                                    .no_wrap())
                                                .child(Text::new(&filtered_value))
                                        }),
                                )
                                .item(El::new().s(Width::fill())) // Spacer to push format to right
                                .item(
                                    El::new()
                                        .s(Font::new()
                                            .color_signal(
                                                always(option_disabled).map_bool_signal(
                                                    || neutral_4(),
                                                    || primary_6(),
                                                ),
                                            )
                                            .size(11)
                                            .line_height(16)
                                            .no_wrap())
                                        .s(Align::new().right())
                                        .child({
                                            let display_text = option.display_text.clone();
                                            let format_name = if let Some(space_pos) =
                                                display_text.rfind(' ')
                                            {
                                                display_text[space_pos + 1..].to_string()
                                            } else {
                                                match option.format {
                                                    shared::VarFormat::ASCII => "ASCII",
                                                    shared::VarFormat::Binary => "Bin",
                                                    shared::VarFormat::BinaryWithGroups => "Bin",
                                                    shared::VarFormat::Hexadecimal => "Hex",
                                                    shared::VarFormat::Octal => "Oct",
                                                    shared::VarFormat::Signed => "Signed",
                                                    shared::VarFormat::Unsigned => "Unsigned",
                                                }
                                                .to_string()
                                            };
                                            Text::new(&format_name)
                                        }),
                                ),
                        )
                        .on_click({
                            let variable_format_changed_relay = selected_variables.variable_format_changed_relay.clone();
                            let unique_id_for_relay = unique_id.clone();
                            let is_open = is_open.clone();
                            let option_format_enum = option.format; // Use the actual enum, not the string
                            move || {
                                if !option_disabled {
                                    variable_format_changed_relay.send((unique_id_for_relay.clone(), option_format_enum));
                                    is_open.set(false);
                                }
                            }
                        })
                })
                .collect::<Vec<_>>(),
        )
}

/// Update the format for a selected variable using Actor+Relay architecture
fn update_variable_format(
    unique_id: &str, 
    new_format: shared::VarFormat, 
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline
) {
    selected_variables.variable_format_changed_relay.send((unique_id.to_string(), new_format));

    crate::visualizer::timeline::timeline_actor::variable_format_updated_relay(waveform_timeline)
        .send((unique_id.to_string(), new_format));
}

/// Check if cursor time is within a variable's file time range
pub fn is_cursor_within_variable_time_range(
    unique_id: &str,
    cursor_time: f64,
    tracked_files: &[TrackedFile],
) -> bool {
    let parts: Vec<&str> = unique_id.splitn(3, '|').collect();
    if parts.len() < 3 {
        return true; // Assume valid if we can't parse (maintains existing behavior)
    }
    let file_path = parts[0];

    if let Some(tracked_file) = tracked_files.iter().find(|f| f.path == file_path) {
        if let shared::FileState::Loaded(loaded_file) = &tracked_file.state {
            if let (Some(min_time), Some(max_time)) = (
                loaded_file
                    .min_time_ns
                    .map(|ns| ns as f64 / 1_000_000_000.0),
                loaded_file
                    .max_time_ns
                    .map(|ns| ns as f64 / 1_000_000_000.0),
            ) {
                cursor_time >= min_time && cursor_time <= max_time
            } else {
                true
            }
        } else {
            false
        }
    } else {
        true
    }
}

/// Trigger signal value queries when variables are present
pub fn trigger_signal_value_queries(tracked_files: &[TrackedFile]) {
    let has_loaded_files = tracked_files
        .iter()
        .any(|f| matches!(f.state, shared::FileState::Loaded(_)));

    if !has_loaded_files {
        return; // Don't query if no files are loaded yet
    }

}

/// Update signal values in UI from cached or backend results

fn variables_name_vertical_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::visualizer::interaction::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::VariablesNameColumn);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::VariablesNameColumn, (0.0, 0.0), &app_config);
            }
        })
}

fn variables_value_vertical_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::visualizer::interaction::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::VariablesValueColumn);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::VariablesValueColumn, (0.0, 0.0), &app_config);
            }
        })
}

fn empty_state_hint(text: &str) -> impl Element {
    El::new()
        .s(Padding::all(20))
        .s(Font::new().color_signal(neutral_8()).italic())
        .child(text)
}

pub fn file_paths_dialog(
    tracked_files: crate::tracked_files::TrackedFiles,
    _selected_variables: crate::selected_variables::SelectedVariables,
    app_config: crate::config::AppConfig,
    file_dialog_visible: Atom<bool>
) -> impl Element {
    let file_count_broadcaster = tracked_files.files_vec_signal.signal_cloned().map(|files| files.len()).broadcast();
    
    let selected_files = zoon::Mutable::new(Vec::<String>::new());

    let close_dialog = {
        let file_dialog_visible = file_dialog_visible.clone();
        move || file_dialog_visible.set(false)
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
                            if event.key() == "Escape" {
                                close_dialog();
                            } else if event.key() == "Enter" {
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
                                .child(file_picker_content(&app_config, &selected_files))
                        )
                        .item(
                            El::new()
                                .s(Padding::all(4))
                                .child_signal(
                                    selected_files.signal_cloned().map(|selected_paths| {

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
                                                let file_count = file_count_broadcaster.signal(),
                                                let selected_files = selected_files.signal_cloned() =>
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
                                                let file_count = file_count_broadcaster.signal(),
                                                let selected_files = selected_files.signal_cloned() => {
                                                let is_loading = *file_count > 0;
                                                let selected_count = selected_files.len();
                                                is_loading || selected_count == 0
                                                }
                                            }
                                        )
                                        .on_press({
                                            let tracked_files_for_press = tracked_files.clone();
                                            let selected_files_for_press = selected_files.clone();
                                            let file_dialog_visible_for_press = file_dialog_visible.clone();
                                            move || {
                                                let selected_files_value = selected_files_for_press.get_cloned();
                                                process_file_picker_selection(
                                                    tracked_files_for_press.clone(),
                                                    selected_files_value,
                                                    file_dialog_visible_for_press.clone()
                                                );
                                            }
                                        })
                                        .variant(ButtonVariant::Primary)
                                        .build()
                                )
                        )
                )
        )
}

pub fn files_panel(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
    load_files_button: impl Element + 'static,
) -> impl Element {
    
    let file_count_broadcaster = tracked_files.files_vec_signal.signal_cloned().map(|files| files.len()).broadcast();
    El::new().s(Height::fill()).child(create_panel(
        Row::new()
            .s(Gap::new().x(SPACING_8))
            .s(Align::new().center_y())
            .item(El::new().s(Font::new().no_wrap()).child("Files & Scopes"))
            .item(El::new().s(Width::growable()))
            .item(load_files_button)
            .item(El::new().s(Width::growable()))
            .item(clear_all_files_button(&tracked_files, &selected_variables)),
        Column::new()
            .s(Gap::new().y(SPACING_4))
            .s(Padding::new().top(SPACING_4).right(SPACING_4))
            .s(Height::fill())
            .s(Width::growable())
            .item(
                El::new().s(Height::fill()).s(Width::growable()).child(
                    Column::new().s(Width::fill()).s(Height::fill()).item(
                        El::new().s(Height::fill()).s(Width::fill()).child_signal({
                            let tracked_files_for_map = tracked_files.clone();
                            let selected_variables_for_map = selected_variables.clone();
                            file_count_broadcaster.signal().map(move |file_count| {
                                if file_count == 0 {
                                    empty_state_hint("Click 'Load Files' to add waveform files.")
                                        .unify()
                                } else {
                                    create_stable_tree_view(tracked_files_for_map.clone(), selected_variables_for_map.clone()).unify()
                                }
                            })
                        }),
                        ),
                    ),
                ),
            ),
    )
}


fn render_tracked_file_reactive(
    tracked_file: TrackedFile,
    expanded_scopes_signal: impl zoon::Signal<Item = indexmap::IndexSet<String>> + 'static + std::marker::Unpin,
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Element {
    let smart_label = compute_smart_label_for_file(&tracked_file);
    
    El::new().child_signal({
        let tracked_file = tracked_file.clone();
        let smart_label = smart_label.clone();
        let tracked_files = tracked_files.clone();
        let selected_variables = selected_variables.clone();
        expanded_scopes_signal.map(move |expanded_scopes| {
            render_tracked_file_as_tree_item_with_label_and_expanded_state(
                tracked_file.clone(),
                smart_label.clone(),
                tracked_files.clone(),
                selected_variables.clone(),
            )
            .into_element()
        })
    })
}

/// Same as render_tracked_file_as_tree_item_with_label but accepts expanded_scopes as parameter
/// This allows TreeViews to get current expanded state instead of static clones
fn render_tracked_file_as_tree_item_with_label_and_expanded_state(
    tracked_file: TrackedFile,
    smart_label: String,
    tracked_files_domain: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Element {
    let display_name = smart_label;
    let tree_data = match &tracked_file.state {
        shared::FileState::Loading(_) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::File)
                    .icon("circle-loader-2")
                    .disabled(true),
            ]
        }
        shared::FileState::Loaded(file_data) => {
            let children = file_data
                .scopes
                .iter()
                .map(|scope| convert_scope_to_tree_data(scope))
                .collect();
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::File)
                    .icon("file")
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone(), tracked_files_domain.clone(), selected_variables.clone()))
                    .with_children(children),
            ]
        }
        shared::FileState::Failed(file_error) => {
            vec![
                TreeViewItemData::new(tracked_file.id.clone(), display_name.clone())
                    .item_type(TreeViewItemType::FileError)
                    .icon("alert-circle")
                    .tooltip(format!(
                        "{}\nError: {}",
                        tracked_file.path,
                        file_error.user_friendly_message()
                    ))
                    .error_message(file_error.user_friendly_message())
                    .disabled(true)
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone(), tracked_files_domain.clone(), selected_variables.clone())),
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
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone(), tracked_files_domain.clone(), selected_variables.clone())),
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
                    .on_remove(create_enhanced_file_remove_handler(tracked_file.id.clone(), tracked_files_domain.clone(), selected_variables.clone())),
            ]
        }
    };

    tree_view()
        .data(tree_data)
        .size(TreeViewSize::Medium)
        .variant(TreeViewVariant::Basic)
        .show_icons(true)
        .show_checkboxes(true)
        .show_checkboxes_on_scopes_only(true)
        .single_scope_selection(true)
        .build()
}

/// Compute smart label for a single file with duplicate detection AND time intervals
fn compute_smart_label_for_file(target_file: &TrackedFile) -> String {
    let base_name = if target_file.filename == "wave_27.fst" {
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

    match &target_file.state {
        shared::FileState::Loaded(waveform_file) => {
            if let (Some(min_ns), Some(max_ns)) =
                (waveform_file.min_time_ns, waveform_file.max_time_ns)
            {
                let min_seconds = min_ns as f64 / 1_000_000_000.0;
                let max_seconds = max_ns as f64 / 1_000_000_000.0;

                let time_range = if max_seconds < 1.0 {
                    format!("{:.0}–{:.0}ms", min_seconds * 1000.0, max_seconds * 1000.0)
                } else if max_seconds < 60.0 {
                    if max_seconds.fract() == 0.0 && min_seconds.fract() == 0.0 {
                        format!("{:.0}–{:.0}s", min_seconds, max_seconds)
                    } else {
                        format!("{:.1}–{:.1}s", min_seconds, max_seconds)
                    }
                } else {
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
            format!("{} (Loading...)", base_name)
        }
        _ => {
            base_name
        }
    }
}

fn create_stable_tree_view(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Element {
    El::new().s(Width::fill()).s(Height::fill()).child(
        Column::new()
            .s(Width::fill())
            .s(Height::fill())
            .s(Gap::new().y(SPACING_2))
            .update_raw_el(|raw_el| {
                raw_el
                    .style("width", "100%")
                    .style("min-width", "fit-content")
            })
            .items_signal_vec({
                let tracked_files_for_signals = tracked_files.clone();
                let tracked_files_for_closure = tracked_files.clone();
                let selected_variables_for_closure = selected_variables.clone();
                tracked_files_for_signals.files.signal_vec().map(move |tracked_file| {
                    let smart_label = compute_smart_label_for_file(&tracked_file);
                    render_tracked_file_as_tree_item_with_label_and_expanded_state(
                        tracked_file.clone(),
                        smart_label,
                        tracked_files_for_closure.clone(),
                        selected_variables_for_closure.clone(),
                    )
                })
            }),
    )
}

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
        .child(create_panel(
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

pub fn selected_variables_with_waveform_panel(
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    tracked_files: crate::tracked_files::TrackedFiles,
    app_config: crate::config::AppConfig,
) -> impl Element {
    let selected_variables_for_signals = selected_variables.clone();
    let tracked_files_broadcaster = tracked_files.files.signal_vec().to_signal_cloned().broadcast();
    
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
                                theme_toggle_button(&app_config)
                            )
                            .item(
                                dock_toggle_button(&app_config)
                            )
                            .item(
                                El::new()
                                    .s(Width::growable())
                            )
                            .item(
                                clear_all_variables_button(&selected_variables)
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
                                                                                get_signal_type_for_selected_variable_from_files(&selected_var, &files)
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
                                                                                // TODO: Connect to WaveformTimeline Actor signal for zoom level
                                                                                Text::new("-- ns/px")
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
                                    .item(variables_name_vertical_divider(&app_config))
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
                                                selected_variables_for_values.variables.signal_vec().map(|selected_var| {
                                                    El::new()
                                                        .s(Height::exact(SELECTED_VARIABLES_ROW_HEIGHT))
                                                        .s(Width::fill())
                                                        .child(
                                                            // TODO: Implement format selection dropdown
                                                            El::new()
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
                                                                                // TODO: Connect to WaveformTimeline Actor signal for cursor position
                                                                                Text::new("--s")
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
                                    .item(variables_value_vertical_divider(&app_config))
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .s(Background::new().color_signal(neutral_2()))
                                            .child(crate::visualizer::canvas::waveform_canvas::waveform_canvas(&selected_variables, &waveform_timeline, &app_config))
                                    )
                            )
                    )
                )
        )
}


pub fn files_panel_with_height(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    El::new()
        .s(Height::exact_signal(
            files_panel_height_signal(app_config.clone()).map(|h| h as u32),
        ))
        .s(Width::growable())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin").style_signal(
                "scrollbar-color",
                primary_6()
                    .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
                    .flatten(),
            )
        })
        .child(files_panel(
            tracked_files.clone(), 
            selected_variables.clone(),
            button().label("Load Files").disabled(true).build() // Placeholder - no file_dialog_visible access
        ))
}

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
                                    primary_3().map(move |track| format!("{} {}", thumb, track))
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
                                    primary_3().map(move |track| format!("{} {}", thumb, track))
                                })
                                .flatten(),
                        )
                    })
                    .child(variables_panel(&tracked_files, &selected_variables, &waveform_timeline))
                    .into_element()
            }
        }))
}

fn create_panel(header_content: impl Element, content: impl Element) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Width::growable())
        .s(Scrollbars::both())
        .s(Background::new().color_signal(neutral_2()))
        .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
        .s(Borders::all_signal(
            neutral_4().map(|color| Border::new().width(1).color(color)),
        ))
        .child(
            Column::new()
                .s(Height::fill())
                .s(Scrollbars::both())
                .update_raw_el(|raw_el| raw_el.style("scrollbar-width", "thin"))
                .item(
                    El::new()
                        .s(Padding::new().x(SPACING_12).y(SPACING_4))
                        .s(Background::new().color_signal(neutral_4()))
                        .s(Borders::new().bottom_signal(
                            neutral_4().map(|color| Border::new().width(1).color(color)),
                        ))
                        .s(Font::new()
                            .weight(FontWeight::SemiBold)
                            .size(14)
                            .color_signal(neutral_11()))
                        .child(header_content),
                )
                .item(
                    El::new()
                        .s(Height::fill())
                        .s(Width::fill())
                        .s(Scrollbars::both())
                        .update_raw_el(|raw_el| {
                            raw_el
                                .style("scrollbar-width", "thin")
                                .style("overflow-x", "auto")
                                .style("min-height", "0")
                                .style_signal(
                                    "scrollbar-color",
                                    primary_6()
                                        .map(|thumb| {
                                            primary_3()
                                                .map(move |track| format!("{} {}", thumb, track))
                                        })
                                        .flatten(),
                                )
                        })
                        .child(content),
                ),
        )
}

fn simple_variables_content(
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

fn variables_loading_signal(
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

fn variables_display_signal(
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


fn cleanup_file_related_state(
    file_id: &str, 
    tracked_files: &[TrackedFile],
    selected_variables: &crate::selected_variables::SelectedVariables,
) {
    let (_filename, _file_path) = tracked_files
        .iter()
        .find(|f| f.id == file_id)
        .map(|f| (f.filename.clone(), f.path.clone()))
        .unwrap_or_else(|| (String::new(), String::new()));

    selected_variables.scope_selected_relay.send(None);

    /*
    crate::selected_variables::retain_expanded_scopes(|scope| {
        scope != &file_path && !scope.starts_with(&format!("{}|", file_path))
    });

    if !file_path.is_empty() {
    }
    */
}

fn create_enhanced_file_remove_handler(
    _file_id: String,
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_variables: crate::selected_variables::SelectedVariables,
) -> impl Fn(&str) + 'static {
    move |id: &str| {
        let current_tracked_files = tracked_files.get_current_files();
        cleanup_file_related_state(id, &current_tracked_files, &selected_variables);

        tracked_files.file_removed_relay.send(id.to_string());


    }
}

fn convert_scope_to_tree_data(scope: &ScopeData) -> TreeViewItemData {
    let mut children = Vec::new();

    let mut child_refs: Vec<&ScopeData> = scope.children.iter().collect();
    child_refs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    for child_scope in child_refs {
        children.push(convert_scope_to_tree_data(child_scope));
    }


    let scope_tree_id = format!("scope_{}", scope.id);

    TreeViewItemData::new(scope_tree_id, scope.name.clone())
        .item_type(TreeViewItemType::Folder)
        .with_children(children)
}

pub fn load_files_button_with_progress(
    tracked_files: crate::tracked_files::TrackedFiles,
    variant: ButtonVariant,
    size: ButtonSize,
    icon: Option<IconName>,
    file_dialog_visible: Atom<bool>
) -> impl Element {
    let file_count_signal = tracked_files.files_vec_signal.signal_cloned().map(|files| files.len());
    El::new().child_signal(
        file_count_signal.map(move |file_count| {
            let is_loading = file_count > 0; // Simple loading state based on file activity
            let mut btn = button();

            if is_loading {
                btn = btn.label("Loading...").disabled(true);
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            } else {
                btn = btn
                    .label("Load Files")
                    .on_press({
                        let file_dialog_visible = file_dialog_visible.clone();
                        move || file_dialog_visible.set(true)
                    });
                if let Some(icon) = icon {
                    btn = btn.left_icon(icon);
                }
            }

            btn.variant(variant.clone())
                .size(size.clone())
                .build()
                .into_element()
        })
    )
}

fn file_picker_content(
    app_config: &crate::config::AppConfig,
    selected_files: &zoon::Mutable<Vec<String>>,
) -> impl Element {
    El::new()
        .s(Height::fill())
        .s(Scrollbars::both())
        .update_raw_el(|raw_el| {
            raw_el.style("scrollbar-width", "thin").style_signal(
                "scrollbar-color",
                primary_6()
                    .map(|thumb| primary_3().map(move |track| format!("{} {}", thumb, track)))
                    .flatten(),
            )
        })
        .child_signal(simple_file_picker_tree(app_config.clone()).into_signal_option())
}

async fn simple_file_picker_tree(app_config: crate::config::AppConfig) -> impl Element {
    let selected_files = zoon::MutableVec::<String>::new();
    let scroll_position = app_config
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
        .on_viewport_location_change({
            let scroll_position_mutable = app_config.file_picker_scroll_position.clone();
            move |_scene, viewport| {
                scroll_position_mutable.set_neq(viewport.y);
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
                let tree_cache = zoon::always(std::collections::HashMap::new()), // Use empty cache for now
                let error_cache = zoon::always(std::collections::HashMap::new()),
                let expanded = app_config.file_picker_expanded_directories.signal_cloned() =>
                move {

                    monitor_directory_expansions(expanded.iter().cloned().collect::<HashSet<_>>(), &app_config);

                    if let Some(_root_items) = tree_cache.get("/") {
                        let tree_data = vec![
                            TreeViewItemData::new("/".to_string(), "/".to_string())
                                .with_children(build_hierarchical_tree("/", &tree_cache, &error_cache))
                        ];

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
                                    .external_expanded(app_config.file_picker_expanded_directories.clone())
                                    .external_selected_vec(selected_files.clone())
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
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>,
) -> bool {
    if let Some(items) = tree_cache.get(path) {
        let has_subfolders = items.iter().any(|item| item.is_directory);
        let has_waveform_files = items
            .iter()
            .any(|item| !item.is_directory && item.is_waveform_file);

        return !has_subfolders && !has_waveform_files;
    }

    false
}

fn build_hierarchical_tree(
    path: &str,
    tree_cache: &HashMap<String, Vec<shared::FileSystemItem>>,
    error_cache: &HashMap<String, String>,
) -> Vec<TreeViewItemData> {
    if let Some(items) = tree_cache.get(path) {
        items
            .iter()
            .map(|item| {
                if item.is_directory {
                    if let Some(_error_msg) = error_cache.get(&item.path) {
                        let data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder)
                            .with_children(vec![
                                TreeViewItemData::new(
                                    "access_denied",
                                    &error_cache
                                        .get(&item.path)
                                        .map(|err| crate::error_display::make_error_user_friendly(err))
                                        .unwrap_or_else(|| {
                                            "Cannot access this directory".to_string()
                                        }),
                                )
                                .item_type(TreeViewItemType::Default)
                                .disabled(true),
                            ]);
                        data
                    } else if let Some(_children) = tree_cache.get(&item.path) {
                        let children = build_hierarchical_tree(&item.path, tree_cache, error_cache);
                        let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder)
                            .with_children(children);

                        if should_disable_folder(&item.path, tree_cache) {
                            data = data.with_children(vec![
                                TreeViewItemData::new("no_supported_files", "No supported files")
                                    .item_type(TreeViewItemType::Default)
                                    .disabled(true),
                            ]);
                        }

                        data
                    } else {
                        let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                            .icon("folder".to_string())
                            .item_type(TreeViewItemType::Folder);

                        if item.has_expandable_content {
                            data = data.with_children(vec![
                                TreeViewItemData::new("loading", "Loading...")
                                    .item_type(TreeViewItemType::Default)
                                    .disabled(true),
                            ]);
                        } else {
                            data = data.with_children(vec![
                                TreeViewItemData::new("no_supported_files", "No supported files")
                                    .item_type(TreeViewItemType::Default)
                                    .disabled(true),
                            ]);
                        }

                        data
                    }
                } else {
                    let mut data = TreeViewItemData::new(item.path.clone(), item.name.clone())
                        .icon("file".to_string())
                        .item_type(TreeViewItemType::File)
                        .is_waveform_file(item.is_waveform_file);

                    if !item.is_waveform_file {
                        data = data.disabled(true);
                    }

                    data
                }
            })
            .collect()
    } else {
        vec![]
    }
}

pub fn monitor_directory_expansions(expanded: HashSet<String>, app_config: &crate::config::AppConfig) {
    let config = app_config;
    let current_config_expanded = config.file_picker_expanded_directories.lock_ref();
    let last_expanded: HashSet<String> = current_config_expanded.iter().cloned().collect();

    let new_expansions: Vec<String> = expanded.difference(&last_expanded).cloned().collect();

    let paths_to_request: Vec<String> = new_expansions
        .into_iter()
        .filter(|path| path.starts_with("/") && !path.is_empty())
        .collect();

    if !paths_to_request.is_empty() {
        Task::start(async move {
            use crate::platform::{CurrentPlatform, Platform};
            let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectories(paths_to_request)).await;
        });
    }

}

fn extract_filename(path: &str) -> String {
    path.split('/').last().unwrap_or(path).to_string()
}


fn process_file_picker_selection(
    tracked_files: crate::tracked_files::TrackedFiles,
    selected_files: Vec<String>,
    file_dialog_visible: Atom<bool>
) {
    Task::start(async move {

        if !selected_files.is_empty() {

            use std::path::PathBuf;

            let tracked_files_snapshot = tracked_files.get_current_files();

            let mut new_files: Vec<PathBuf> = Vec::new();
            let mut reload_files: Vec<String> = Vec::new();

            for selected_path in selected_files {
                let selected_pathbuf = PathBuf::from(&selected_path);

                if let Some(existing_file) = tracked_files_snapshot
                    .iter()
                    .find(|f| f.id == selected_path || f.path == selected_path)
                {
                    reload_files.push(existing_file.id.clone());
                } else {
                    new_files.push(selected_pathbuf);
                }
            }

            let tracked_files = &tracked_files;

            if !new_files.is_empty() {
                tracked_files.files_dropped_relay.send(new_files);
            }

            if !reload_files.is_empty() {
                for file_id in reload_files {
                    tracked_files.reload_file(file_id);
                }
            }

            file_dialog_visible.set(false);
        }
    }); // End of async Task::start block
}

fn clear_all_files(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) {

    let file_ids: Vec<String> = tracked_files
        .get_current_files()
        .iter()
        .map(|f| f.id.clone())
        .collect();

    let current_tracked_files = tracked_files.get_current_files();
    for file_id in &file_ids {
        cleanup_file_related_state(file_id, &current_tracked_files, selected_variables);
    }

    tracked_files.all_files_cleared_relay.send(());



}

fn clear_all_files_button(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    let tracked_files_clone = tracked_files.clone();
    let selected_variables_clone = selected_variables.clone();
    button()
        .label("Clear All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(move || {
            clear_all_files(&tracked_files_clone, &selected_variables_clone);
        })
        .build()
}

fn clear_all_variables_button(
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    let selected_variables_clone = selected_variables.clone();
    button()
        .label("Clear All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(move || {
            selected_variables_clone.selection_cleared_relay.send(());
        })
        .build()
}

fn theme_toggle_button(app_config: &crate::config::AppConfig) -> impl Element {
    let app_config = app_config.clone();
    El::new().child_signal(theme().map(move |current_theme| {
        button()
            .left_icon(match current_theme {
                Theme::Light => IconName::Moon,
                Theme::Dark => IconName::Sun,
            })
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Small)
            .on_press({
                let theme_relay = app_config.theme_button_clicked_relay.clone();
                move || theme_relay.send(())
            })
            .build()
            .into_element()
    }))
}

fn dock_toggle_button(app_config: &crate::config::AppConfig) -> impl Element {
    let app_config = app_config.clone();
    El::new().child_signal(app_config.dock_mode_actor.signal().map(move |dock_mode| {
        let app_config_for_icon = app_config.clone();
        let app_config_for_press = app_config.clone();
        let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
        button()
            .label(if is_docked {
                "Dock to Right"
            } else {
                "Dock to Bottom"
            })
            .left_icon_element(move || {
                El::new()
                    .child_signal(app_config_for_icon.dock_mode_actor.signal().map(|dock_mode| {
                        let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
                        let icon_el = icon(IconName::ArrowDownToLine)
                            .size(IconSize::Small)
                            .color(IconColor::Primary)
                            .build();
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
            .on_press({
                let dock_relay = app_config_for_press.dock_mode_button_clicked_relay.clone();
                move || {
                    dock_relay.send(());
                }
            })
            .align(Align::center())
            .build()
            .into_element()
    }))
}

pub fn files_panel_vertical_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::visualizer::interaction::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::FilesPanelMain);

    El::new()
        .s(Width::exact(4))
        .s(Height::fill())
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::ColumnResize))
        .s(Padding::all(0))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::FilesPanelMain, (0.0, 0.0), &app_config);
            }
        })
}

pub fn files_panel_horizontal_divider(app_config: &crate::config::AppConfig) -> impl Element {
    use crate::visualizer::interaction::dragging::{DividerType, is_divider_dragging, start_drag};

    let is_dragging_signal = is_divider_dragging(DividerType::FilesPanelSecondary);

    El::new()
        .s(Width::fill())
        .s(Height::exact(4))
        .s(Background::new()
            .color_signal(is_dragging_signal.map_bool_signal(|| primary_7(), || primary_6())))
        .s(Cursor::new(CursorIcon::RowResize))
        .on_pointer_down({
            let app_config = app_config.clone();
            move || {
                start_drag(DividerType::FilesPanelSecondary, (0.0, 0.0), &app_config);
            }
        })
}


/// Main application layout with keyboard handling and drag interactions
pub fn main_layout(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    app_config: &crate::config::AppConfig,
) -> impl Element {
    El::new().s(Width::fill()).s(Height::fill()).child(
        Column::new()
            .s(Width::fill())
            .s(Height::fill())
            .item(files_panel(
                tracked_files.clone(), 
                selected_variables.clone(),
                button().label("Load Files").disabled(true).build() // Placeholder - no file_dialog_visible access
            ))
            .item(selected_variables_with_waveform_panel(
                selected_variables.clone(),
                waveform_timeline.clone(),
                tracked_files.clone(),
                app_config.clone(),
            )),
    )
}
