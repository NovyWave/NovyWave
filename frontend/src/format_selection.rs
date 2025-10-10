use crate::dataflow::Atom;
use moonzoon_novyui::components::icon::{IconColor, IconName, IconSize, icon};
use moonzoon_novyui::tokens::color::{
    neutral_1, neutral_2, neutral_3, neutral_4, neutral_8, neutral_11, primary_6,
};
use moonzoon_novyui::*;
use shared::{SignalValue, VarFormat, truncate_value};
use zoon::events::{Click, KeyDown, PointerDown};
use zoon::map_ref;
use zoon::*;

const COLLAPSED_VALUE_MAX_CHARS: usize = 32;
const DROPDOWN_VALUE_MAX_CHARS: usize = 56;

fn sanitize_dom_id(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn format_ui_label(format: VarFormat) -> &'static str {
    match format {
        VarFormat::ASCII => "Text",
        VarFormat::Binary => "Bin",
        VarFormat::BinaryWithGroups => "Bins",
        VarFormat::Hexadecimal => "Hex",
        VarFormat::Octal => "Oct",
        VarFormat::Signed => "Int",
        VarFormat::Unsigned => "UInt",
    }
}

fn sanitize_tooltip_text(value: &str) -> String {
    let filtered: String = value
        .chars()
        .filter(|&c| c == ' ' || c == '\n' || (c.is_ascii() && c.is_ascii_graphic()))
        .collect();
    let trimmed = filtered.trim();
    if trimmed.is_empty() {
        value.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn special_state_tooltip(value: &SignalValue) -> Option<&'static str> {
    match value {
        SignalValue::Present(raw) => match raw.trim().to_ascii_uppercase().as_str() {
            "Z" => Some(
                "High-Impedance (Z)\nSignal is disconnected or floating.\nCommon in tri-state buses and disabled outputs.",
            ),
            "X" => Some(
                "Unknown (X)\nSignal value cannot be determined.\nOften caused by timing violations or uninitialized logic.",
            ),
            "U" => Some(
                "Uninitialized (U)\nSignal has not been assigned a value.\nTypically seen during power-up or before reset.",
            ),
            _ => None,
        },
        _ => None,
    }
}

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
    app_config: crate::config::AppConfig,
) -> impl Element {
    let unique_id = variable_unique_id.to_string();
    let trigger_id = format!("format-dropdown-trigger-{}", sanitize_dom_id(&unique_id));

    let is_open = Atom::new(false);
    let latest_value = Atom::new(SignalValue::Loading);
    let app_config_for_copy = app_config.clone();

    let chevron_icon = El::new().child_signal(is_open.signal().map(|open| {
        if open {
            icon(IconName::ChevronUp)
                .size(IconSize::Small)
                .color(IconColor::Primary)
                .build()
                .unify()
        } else {
            icon(IconName::ChevronDown)
                .size(IconSize::Small)
                .color(IconColor::Primary)
                .build()
                .unify()
        }
    }));

    let copy_button = button()
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Small)
        .left_icon(IconName::Copy)
        .custom_padding(4, 4)
        .on_press({
            let latest_value = latest_value.clone();
            let app_config = app_config_for_copy.clone();
            move || {
                let value = latest_value.get_cloned();
                let formatted = value.get_formatted(&current_format);
                crate::clipboard::copy_variable_value(&formatted, &app_config);
            }
        })
        .build()
        .into_element();

    let toggle_open = {
        let is_open = is_open.clone();
        move || {
            let currently_open = is_open.get_cloned();
            is_open.set(!currently_open);
        }
    };

    let cursor_values_actor_for_dropdown = waveform_timeline.cursor_values_actor();
    let unique_id_for_dropdown = unique_id.clone();
    let value_signal_for_dropdown = cursor_values_actor_for_dropdown
        .state
        .signal_cloned()
        .map(move |map| map.get(&unique_id_for_dropdown).cloned());

    let dropdown_options_signal = map_ref! {
        let open = is_open.signal(),
        let value = value_signal_for_dropdown => {
            if !*open {
                None
            } else {
                Some(value.clone().unwrap_or(SignalValue::Loading))
            }
        }
    };

    let cursor_values_actor_for_display = waveform_timeline.cursor_values_actor();
    let unique_id_for_display = unique_id.clone();
    let value_signal_for_display = cursor_values_actor_for_display
        .state
        .signal_cloned()
        .map(move |map| map.get(&unique_id_for_display).cloned());

    let overlay_signal = {
        let is_open_for_signal = is_open.clone();
        is_open.signal().map(move |open| {
            if open {
                let is_open_for_handlers = is_open_for_signal.clone();
                El::new()
                    .update_raw_el(|raw_el| {
                        raw_el
                            .style("position", "fixed")
                            .style("inset", "0")
                            .style("z-index", "29999")
                            .style("background-color", "rgba(0,0,0,0)")
                    })
                    .on_pointer_down({
                        let is_open = is_open_for_handlers.clone();
                        move || {
                            is_open.set(false);
                        }
                    })
                    .update_raw_el({
                        let is_open = is_open_for_handlers.clone();
                        move |raw_el| {
                            raw_el.global_event_handler(move |event: KeyDown| {
                                if event.key() == "Escape" {
                                    is_open.set(false);
                                }
                            })
                        }
                    })
            } else {
                El::new()
            }
        })
    };

    Column::new()
        .s(Width::fill())
        .item(El::new().child_signal(overlay_signal.map(|overlay| overlay.unify())))
        .item(
            El::new()
                .s(Width::fill())
                .s(Height::fill())
                .s(Background::new().color_signal(
                    is_open
                        .signal()
                        .map_bool_signal(|| neutral_3(), || neutral_2()),
                ))
                .s(Borders::all_signal(
                    is_open
                        .signal()
                        .map_bool_signal(|| primary_6(), || neutral_4())
                        .map(|color| Border::new().width(1).color(color)),
                ))
                .s(RoundedCorners::all(4))
                .s(Cursor::new(CursorIcon::Pointer))
                .update_raw_el({
                    let trigger_id = trigger_id.clone();
                    move |raw_el| {
                        raw_el
                            .attr("id", &trigger_id)
                            .style("position", "relative")
                            .style("z-index", "10000")
                    }
                })
                .on_click(toggle_open)
                .child(
                    Row::new()
                        .s(Width::fill())
                        .s(Height::fill())
                        .s(Align::new().center_y())
                        .s(Padding::new().x(SPACING_8))
                        .s(Gap::new().x(SPACING_8))
                        .item(El::new().s(Width::growable()).child_signal(
                            value_signal_for_display.map({
                                let latest_value = latest_value.clone();
                                move |maybe_value| {
                                    let signal_value = maybe_value.unwrap_or(SignalValue::Loading);
                                    latest_value.set(signal_value.clone());

                                    let formatted = signal_value.get_formatted(&current_format);
                                    let truncated =
                                        truncate_value(&formatted, COLLAPSED_VALUE_MAX_CHARS);
                                    let filtered_full = sanitize_tooltip_text(&formatted);
                                    let display = if truncated.trim().is_empty() {
                                        "-".to_string()
                                    } else {
                                        truncated
                                    };
                                    let is_placeholder = matches!(
                                        signal_value,
                                        SignalValue::Loading | SignalValue::Missing,
                                    ) || display.trim() == "-";

                                    let tooltip_string = special_state_tooltip(&signal_value)
                                        .map(|text| text.to_string())
                                        .unwrap_or_else(|| filtered_full.clone());

                                    El::new()
                                        .s(Font::new().size(13).no_wrap().color_signal(
                                            always(is_placeholder)
                                                .map_bool_signal(|| neutral_8(), || neutral_11()),
                                        ))
                                        .update_raw_el(move |raw_el| {
                                            if !tooltip_string.is_empty() {
                                                raw_el.attr("title", &tooltip_string)
                                            } else {
                                                raw_el
                                            }
                                        })
                                        .child(Text::new(display))
                                        .unify()
                                }
                            }),
                        ))
                        .item(El::new().child(copy_button).update_raw_el(|raw_el| {
                            raw_el
                                .event_handler(|event: PointerDown| {
                                    event.stop_propagation();
                                })
                                .event_handler(|event: Click| {
                                    event.stop_propagation();
                                })
                                .attr("title", "Copy value to clipboard")
                        }))
                        .item(
                            El::new()
                                .s(Font::new().size(11).color_signal(neutral_8()))
                                .s(Align::new().right())
                                .child(format_ui_label(current_format)),
                        )
                        .item(chevron_icon),
                ),
        )
        .item(El::new().child_signal(dropdown_options_signal.map({
            let selected_variables = selected_variables.clone();
            let waveform_timeline = waveform_timeline.clone();
            let trigger_id = trigger_id.clone();
            let is_open = is_open.clone();
            let unique_id_outer = unique_id.clone();
            move |maybe_signal_value| {
                let selected_variables_for_map = selected_variables.clone();
                let unique_id_for_map = unique_id_outer.clone();
                let trigger_id_for_map = trigger_id.clone();
                let waveform_timeline_for_map = waveform_timeline.clone();
                let is_open_for_map = is_open.clone();
                maybe_signal_value.map(move |signal_value| {
                    let selected_variables = selected_variables_for_map.clone();
                    let waveform_timeline = waveform_timeline_for_map.clone();
                    let options =
                        generate_ui_dropdown_options(&signal_value, "", DROPDOWN_VALUE_MAX_CHARS);
                    let unique_id_for_call = unique_id_for_map.clone();
                    let trigger_id_for_call = trigger_id_for_map.clone();
                    let is_open_for_call = is_open_for_map.clone();
                    create_smart_dropdown(
                        options,
                        current_format,
                        selected_variables,
                        waveform_timeline,
                        is_open_for_call,
                        trigger_id_for_call,
                        unique_id_for_call,
                    )
                    .into_element()
                })
            }
        })))
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
    current_format: VarFormat,
    selected_variables: crate::selected_variables::SelectedVariables,
    waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    is_open: Atom<bool>,
    trigger_id: String,
    unique_id: String,
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
    let selected_variables_shared = selected_variables.clone();
    let waveform_timeline_shared = waveform_timeline.clone();

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
                    let _ = style.set_property("z-index", "30000");
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
                                let trigger_width = trigger_rect.width();
                                let dropdown_width = trigger_width.max(MIN_DROPDOWN_WIDTH);
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

                                let _ =
                                    style.set_property("width", &format!("{}px", dropdown_width));
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
                .map(move |option| {
                    let option_display = option.display_text.clone();
                    let option_full_text = option.full_text.clone();
                    let option_disabled = option.disabled;
                    let is_selected = option.format == current_format;
                    let selected_variables = selected_variables_shared.clone();
                    let waveform_timeline = waveform_timeline_shared.clone();

                    El::new()
                        .s(Width::fill())
                        .s(Height::exact(28))
                        .s(Padding::new().x(SPACING_12).y(SPACING_6))
                        .s(Background::new().color_signal(
                            always(is_selected).map_bool_signal(|| primary_6(), || neutral_1()),
                        ))
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
                        .s(Font::new().size(12))
                        .child(
                            Row::new()
                                .s(Width::fill())
                                .s(Gap::new().x(SPACING_8))
                                .item(
                                    El::new()
                                        .s(Font::new()
                                            .color_signal(always(option_disabled).map_bool_signal(
                                                || neutral_4(),
                                                move || {
                                                    let is_selected = is_selected;
                                                    always(is_selected).map_bool_signal(
                                                        || neutral_1(),
                                                        || neutral_11(),
                                                    )
                                                },
                                            ))
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
                                                                || neutral_8(),
                                                                move || {
                                                                    let is_selected = is_selected;
                                                                    always(is_selected)
                                                                        .map_bool_signal(
                                                                            || neutral_1(),
                                                                            || neutral_11(),
                                                                        )
                                                                },
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
                                            .color_signal(always(option_disabled).map_bool_signal(
                                                || neutral_4(),
                                                move || {
                                                    let is_selected = is_selected;
                                                    always(is_selected).map_bool_signal(
                                                        || neutral_1(),
                                                        || primary_6(),
                                                    )
                                                },
                                            ))
                                            .size(11)
                                            .line_height(16)
                                            .no_wrap())
                                        .s(Align::new().right())
                                        .child({
                                            let display_text = option.display_text.clone();
                                            let format_name =
                                                if let Some(space_pos) = display_text.rfind(' ') {
                                                    display_text[space_pos + 1..].to_string()
                                                } else {
                                                    format_ui_label(option.format).to_string()
                                                };
                                            Text::new(&format_name)
                                        }),
                                ),
                        )
                        .on_click({
                            let unique_id_for_relay = unique_id.clone();
                            let is_open = is_open.clone();
                            let option_format_enum = option.format; // Use the actual enum, not the string
                            let selected_variables = selected_variables.clone();
                            let waveform_timeline = waveform_timeline.clone();
                            move || {
                                if !option_disabled {
                                    update_variable_format(
                                        &unique_id_for_relay,
                                        option_format_enum,
                                        &selected_variables,
                                        &waveform_timeline,
                                    );
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
    waveform_timeline: &crate::visualizer::timeline::timeline_actor::WaveformTimeline,
) {
    selected_variables
        .variable_format_changed_relay
        .send((unique_id.to_string(), new_format));

    waveform_timeline
        .variable_format_updated_relay
        .send((unique_id.to_string(), new_format));
}
