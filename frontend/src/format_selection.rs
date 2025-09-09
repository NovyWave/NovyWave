use moonzoon_novyui::tokens::color::{
    neutral_1, neutral_3, neutral_4, neutral_6, neutral_8, neutral_11, primary_6, primary_8,
};
use moonzoon_novyui::*;
use zoon::*;
use shared::{VarFormat, truncate_value};

/// Format options for dropdown - contains value and disabled state
#[derive(Debug, Clone)]
pub struct DropdownFormatOption {
    pub format: shared::VarFormat,
    pub display_text: String,
    pub full_text: String, // For tooltip
    pub disabled: bool,
}

impl DropdownFormatOption {
    pub fn new(
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

/// Create a format selection dropdown for a variable
pub fn create_format_dropdown(
    variable_unique_id: &str,
    current_format: VarFormat,
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
) -> impl Element {
    let unique_id = variable_unique_id.to_string();
    let format_options = vec![
        (VarFormat::Binary, "Bin"),
        (VarFormat::Hexadecimal, "Hex"),
        (VarFormat::Octal, "Oct"),
        (VarFormat::Signed, "Signed"),
        (VarFormat::Unsigned, "Unsigned"),
        (VarFormat::ASCII, "ASCII"),
    ];
    
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .child(
            Row::new()
                .s(Gap::new().x(4))
                .items(format_options.into_iter().map(|(format, label)| {
                    let is_current = format == current_format;
                    let unique_id_clone = unique_id.clone();
                    let selected_variables_clone = selected_variables.clone();
                    let waveform_timeline_clone = waveform_timeline.clone();
                    
                    El::new()
                        .s(Background::new().color_signal(
                            if is_current { primary_6().boxed_local() } else { neutral_3().boxed_local() }
                        ))
                        .s(Borders::all_signal(
                            (if is_current { primary_8().boxed_local() } else { neutral_6().boxed_local() })
                            .map(|color| Border::new().width(1).color(color))
                        ))
                        .s(Padding::new().x(6).y(2))
                        .s(RoundedCorners::all(4))
                        .s(Font::new().size(11).color_signal(
                            if is_current { neutral_1().boxed_local() } else { neutral_11().boxed_local() }
                        ))
                        .s(Cursor::new(CursorIcon::Pointer))
                        .on_click(move || {
                            if !is_current {
                                update_variable_format(
                                    &unique_id_clone, 
                                    format, 
                                    &selected_variables_clone, 
                                    &waveform_timeline_clone
                                );
                            }
                        })
                        .child(Text::new(label))
                }))
        )
}

/// Generate dropdown options for UI from shared SignalValue
pub fn generate_ui_dropdown_options(
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
pub fn create_smart_dropdown(
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
                                        .s(moonzoon_novyui::tokens::typography::font_mono())
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
pub fn update_variable_format(
    unique_id: &str, 
    new_format: shared::VarFormat, 
    selected_variables: &crate::selected_variables::SelectedVariables,
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline
) {
    selected_variables.variable_format_changed_relay.send((unique_id.to_string(), new_format));

    crate::visualizer::timeline::timeline_actor::variable_format_updated_relay(waveform_timeline)
        .send((unique_id.to_string(), new_format));
}