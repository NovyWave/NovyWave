// Select Component
// Dropdown select component with proper dropdown functionality

use crate::tokens::*;
use crate::components::icon::{IconBuilder, IconName, IconSize, IconColor};
use zoon::*;

// Select sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectSize {
    Small,
    Medium,
    Large,
}

// Individual select option
#[derive(Debug, Clone, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// Helper to create options from simple strings
impl From<&str> for SelectOption {
    fn from(value: &str) -> Self {
        SelectOption::new(value, value)
    }
}

impl From<String> for SelectOption {
    fn from(value: String) -> Self {
        SelectOption::new(value.clone(), value)
    }
}

// Dropdown select builder
pub struct SelectBuilder {
    size: SelectSize,
    placeholder: Option<String>,
    selected_value: Option<String>,
    options: Vec<SelectOption>,
    disabled: bool,
    label: Option<String>,
    description: Option<String>,
    min_width: Option<u32>,
}

impl SelectBuilder {
    pub fn new() -> Self {
        Self {
            size: SelectSize::Medium,
            placeholder: None,
            selected_value: None,
            options: Vec::new(),
            disabled: false,
            label: None,
            description: None,
            min_width: Some(320), // Default min-width of 320px
        }
    }

    pub fn size(mut self, size: SelectSize) -> Self {
        self.size = size;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn selected_value(mut self, value: impl Into<String>) -> Self {
        self.selected_value = Some(value.into());
        self
    }

    pub fn options<T: Into<SelectOption>>(mut self, options: impl IntoIterator<Item = T>) -> Self {
        self.options = options.into_iter().map(|opt| opt.into()).collect();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn min_width(mut self, width: u32) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn width(mut self, width: u32) -> Self {
        // Alias for min_width for convenience
        self.min_width = Some(width);
        self
    }

    // Placeholder methods for compatibility
    pub fn multiple(self, _multiple: bool) -> Self { self }
    pub fn searchable(self, _searchable: bool) -> Self { self }
    pub fn search_placeholder(self, _placeholder: impl Into<String>) -> Self { self }
    pub fn selected_values(self, _values: Vec<String>) -> Self { self }
    pub fn error(self, _error: bool) -> Self { self }
    pub fn required(self, _required: bool) -> Self { self }
    pub fn no_options_text(self, _text: impl Into<String>) -> Self { self }

    pub fn build(self) -> impl Element {
        // State management
        let selected_value = Mutable::new(self.selected_value.clone());
        let is_open = Mutable::new(false);
        let focused_index = Mutable::new(0usize);

        // Component configuration
        let (padding_x, padding_y, font_size) = match self.size {
            SelectSize::Small => (SPACING_12, SPACING_8, FONT_SIZE_14),
            SelectSize::Medium => (SPACING_16, SPACING_10, FONT_SIZE_16),
            SelectSize::Large => (SPACING_20, SPACING_12, FONT_SIZE_18),
        };

        let disabled = self.disabled;
        let options = self.options.clone();
        let placeholder_text = self.placeholder.clone().unwrap_or_else(|| "Select an option...".to_string());

        // Create the select trigger (the clickable part)
        let min_width = self.min_width.unwrap_or(320);
        let select_trigger = Row::new()
            .s(Width::default().min(min_width)) // Configurable min-width
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(Borders::all_signal(theme().map(|t| {
                Border::new()
                    .width(1)
                    .color(match t {
                        Theme::Light => "oklch(75% 0.14 250)", // neutral_4
                        Theme::Dark => "oklch(25% 0.14 250)", // neutral_4 dark
                    })
            })))
            .s(RoundedCorners::all(6))
            .s(Background::new().color_signal(theme().map(move |t| match t {
                Theme::Light => if disabled {
                    "oklch(92% 0.02 0)" // neutral gray without cyan tint
                } else {
                    "oklch(100% 0 0)" // white
                },
                Theme::Dark => if disabled {
                    "oklch(18% 0.02 0)" // dark neutral gray without cyan tint
                } else {
                    "oklch(10% 0.14 250)" // neutral_2 dark
                },
            })))
            .s(Cursor::new(if disabled {
                CursorIcon::NotAllowed
            } else {
                CursorIcon::Pointer
            }))
            .s(Align::new().center_y().left())
            .item(
                El::new()
                    .s(Width::fill())
                    .child_signal(
                        selected_value.signal_cloned().map({
                            let options = options.clone();
                            let placeholder_text = placeholder_text.clone();
                            move |selected| {
                                let display_text = if let Some(ref value) = selected {
                                    // Find the label for the selected value
                                    options
                                        .iter()
                                        .find(|opt| opt.value == *value)
                                        .map(|opt| opt.label.clone())
                                        .unwrap_or_else(|| value.clone())
                                } else {
                                    placeholder_text.clone()
                                };

                                El::new()
                                    .child(Text::new(&display_text))
                                    .s(Font::new()
                                        .size(font_size)
                                        .color_signal(theme().map(move |t| {
                                            if disabled {
                                                match t {
                                                    Theme::Light => "oklch(45% 0.14 250)", // neutral_6
                                                    Theme::Dark => "oklch(55% 0.14 250)", // neutral_6 dark
                                                }
                                            } else if selected.is_none() {
                                                match t {
                                                    Theme::Light => "oklch(65% 0.14 250)", // neutral_7 (placeholder)
                                                    Theme::Dark => "oklch(45% 0.14 250)", // neutral_7 dark
                                                }
                                            } else {
                                                match t {
                                                    Theme::Light => "oklch(15% 0.14 250)", // neutral_9
                                                    Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                }
                                            }
                                        }))
                                    )
                            }
                        })
                    )
            )
            .item(
                // Chevron icon
                El::new()
                    .child_signal(
                        map_ref! {
                            let theme = theme(),
                            let is_open = is_open.signal() => {
                                let color = if disabled {
                                    match *theme {
                                        Theme::Light => "oklch(45% 0.14 250)", // neutral_6
                                        Theme::Dark => "oklch(55% 0.14 250)", // neutral_6 dark
                                    }
                                } else {
                                    match *theme {
                                        Theme::Light => "oklch(65% 0.14 250)", // neutral_7
                                        Theme::Dark => "oklch(45% 0.14 250)", // neutral_7 dark
                                    }
                                };

                                let icon_name = if *is_open {
                                    IconName::ChevronUp
                                } else {
                                    IconName::ChevronDown
                                };

                                IconBuilder::new(icon_name)
                                    .size(IconSize::Small)
                                    .color(IconColor::Custom(color))
                                    .build()
                            }
                        }
                    )
            )
            .on_click({
                let is_open = is_open.clone();
                move || {
                    if !disabled {
                        is_open.set_neq(!is_open.get());
                    }
                }
            });

        // Create the main container with dropdown using element_below_signal
        let select_container = select_trigger
            .element_below_signal(is_open.signal().map_true({
                let selected_value = selected_value.clone();
                let is_open = is_open.clone();
                let options = options.clone();
                let select_disabled = disabled; // Capture disabled state
                move || {
                    // Create the dropdown menu with proper positioning
                    Column::new()
                        // Dropdown naturally sizes to match trigger width
                        .s(Transform::new().move_down(4)) // 4px gap between trigger and dropdown
                        .s(Background::new().color_signal(theme().map(|theme| {
                            match theme {
                                Theme::Light => "oklch(100% 0 0)", // neutral_1 (white)
                                Theme::Dark => "oklch(10% 0.14 250)", // neutral_2 dark
                            }
                        })))
                        .s(Borders::all_signal(theme().map(|theme| {
                            Border::new()
                                .width(1)
                                .color(match theme {
                                    Theme::Light => "oklch(85% 0.14 250)", // neutral_3
                                    Theme::Dark => "oklch(25% 0.14 250)", // neutral_3 dark
                                })
                        })))
                        .s(RoundedCorners::all(4)) // 4px border radius to match Vue
                        .s(Shadows::new([
                            Shadow::new()
                                .y(4)
                                .blur(6)
                                .spread(-1)
                                .color("oklch(70% 0.09 255 / 0.22)"), // shadow.color.neutral
                            Shadow::new()
                                .y(2)
                                .blur(4)
                                .spread(-2)
                                .color("oklch(70% 0.09 255 / 0.22)") // shadow.color.neutral
                        ]))
                        // Let the dropdown naturally size to its content
                        .s(Scrollbars::both())
                        .items(
                            options.iter().map(|option| {
                                let option_value = option.value.clone();
                                let option_label = option.label.clone();
                                let option_disabled = option.disabled;

                                let (hovered, hovered_signal) = Mutable::new_and_signal(false);

                                El::new()
                                    .s(Width::fill())
                                    .s(Padding::new().x(12).y(8)) // 8px vertical, 12px horizontal to match Vue
                                    .s(Cursor::new(if option_disabled {
                                        CursorIcon::NotAllowed
                                    } else {
                                        CursorIcon::Pointer
                                    }))
                                    .s(Background::new().color_signal(
                                        map_ref! {
                                            let theme = theme(),
                                            let selected = selected_value.signal_cloned(),
                                            let hovered = hovered_signal,
                                            let option_value = always(option_value.clone()) => {
                                                if *selected == Some(option_value.clone()) {
                                                    // Use muted background for disabled select, normal primary for enabled
                                                    if select_disabled {
                                                        Some(match *theme {
                                                            Theme::Light => "oklch(92% 0.02 0)", // neutral gray without cyan tint
                                                            Theme::Dark => "oklch(18% 0.02 0)", // dark neutral gray without cyan tint
                                                        })
                                                    } else {
                                                        Some("oklch(95% 0.16 250)") // primary_2 - selected background
                                                    }
                                                } else if *hovered && !option_disabled {
                                                    // Hover background for non-selected options
                                                    Some(match *theme {
                                                        Theme::Light => "oklch(85% 0.14 250)", // neutral_3
                                                        Theme::Dark => "oklch(25% 0.14 250)", // neutral_3 dark
                                                    })
                                                } else {
                                                    None
                                                }
                                            }
                                        }
                                    ))
                                    .on_hovered_change({
                                        let hovered = hovered.clone();
                                        move |is_hovered| {
                                            if !option_disabled {
                                                hovered.set(is_hovered);
                                            }
                                        }
                                    })
                                    .s(Font::new()
                                        .size(14) // 14px font size to match Vue
                                        .color_signal(
                                            map_ref! {
                                                let theme = theme(),
                                                let selected = selected_value.signal_cloned(),
                                                let option_value = always(option_value.clone()) => {
                                                    if option_disabled {
                                                        match *theme {
                                                            Theme::Light => "oklch(45% 0.14 250)", // neutral_6
                                                            Theme::Dark => "oklch(55% 0.14 250)", // neutral_6 dark
                                                        }
                                                    } else if *selected == Some(option_value.clone()) {
                                                        "oklch(25% 0.16 250)" // primary_8 - selected text color
                                                    } else {
                                                        match *theme {
                                                            Theme::Light => "oklch(15% 0.14 250)", // neutral_9
                                                            Theme::Dark => "oklch(85% 0.14 250)", // neutral_10 dark
                                                        }
                                                    }
                                                }
                                            }
                                        )
                                        .weight(FontWeight::Medium)
                                    )
                                    .child(Text::with_signal(always(option_label)))
                                    .on_click({
                                        let selected_value = selected_value.clone();
                                        let is_open = is_open.clone();
                                        let option_value = option_value.clone();
                                        move || {
                                            if !option_disabled {
                                                selected_value.set(Some(option_value.clone()));
                                                is_open.set(false);
                                            }
                                        }
                                    })
                            }).collect::<Vec<_>>()
                        )
                }
            }))
            .on_click_outside({
                let is_open = is_open.clone();
                move || is_open.set(false)
            });

        // Build the complete component
        if let Some(label_text) = &self.label {
            let mut items = Vec::new();

            // Label
            let label_element = El::new()
                .child(Text::new(label_text))
                .s(Font::new()
                    .size(FONT_SIZE_14)
                    .weight(FontWeight::Number(FONT_WEIGHT_5))
                    .color_signal(theme().map(move |t| {
                        if disabled {
                            match t {
                                Theme::Light => "oklch(45% 0.14 250)", // neutral_6
                                Theme::Dark => "oklch(55% 0.14 250)", // neutral_6 dark
                            }
                        } else {
                            match t {
                                Theme::Light => "oklch(15% 0.14 250)", // neutral_9
                                Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                            }
                        }
                    }))
                );

            items.push(label_element.unify());
            items.push(select_container.unify());

            // Description
            if let Some(description) = &self.description {
                let desc_element = El::new()
                    .child(Text::new(description))
                    .s(Font::new()
                        .size(FONT_SIZE_12)
                        .weight(FontWeight::Number(FONT_WEIGHT_4))
                        .color_signal(theme().map(move |t| {
                            if disabled {
                                match t {
                                    Theme::Light => "oklch(45% 0.14 250)", // neutral_6
                                    Theme::Dark => "oklch(55% 0.14 250)", // neutral_6 dark
                                }
                            } else {
                                match t {
                                    Theme::Light => "oklch(35% 0.14 250)", // neutral_7
                                    Theme::Dark => "oklch(65% 0.14 250)", // neutral_7 dark
                                }
                            }
                        }))
                    );
                items.push(desc_element.unify());
            }

            Column::new()
                .s(Gap::new().y(SPACING_4))
                .s(Align::new().left())
                .s(Width::default().min(min_width)) // Consistent min-width
                .items(items)
                .unify()
        } else {
            select_container.unify()
        }
    }
}

// Convenience functions
pub fn select() -> SelectBuilder {
    SelectBuilder::new()
}

pub fn select_option(value: impl Into<String>, label: impl Into<String>) -> SelectOption {
    SelectOption::new(value, label)
}
