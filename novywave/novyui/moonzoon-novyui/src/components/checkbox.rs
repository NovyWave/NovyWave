use zoon::*;
use crate::tokens::*;
use crate::components::icon::{IconBuilder, IconName, IconSize, IconColor};

// Checkbox sizes
#[derive(Debug, Clone, Copy)]
pub enum CheckboxSize {
    Small,   // 16px
    Medium,  // 20px
    Large,   // 24px
}

impl CheckboxSize {
    pub fn to_px(self) -> u32 {
        match self {
            CheckboxSize::Small => 20,   // Increased from 16
            CheckboxSize::Medium => 24,  // Increased from 20
            CheckboxSize::Large => 28,   // Increased from 24
        }
    }

    pub fn font_size(self) -> u32 {
        match self {
            CheckboxSize::Small => FONT_SIZE_12,
            CheckboxSize::Medium => FONT_SIZE_14,
            CheckboxSize::Large => FONT_SIZE_16,
        }
    }
}

// Checkbox states
#[derive(Debug, Clone, Copy)]
pub enum CheckboxState {
    Unchecked,
    Checked,
    Indeterminate, // For partial selection in groups
}

// Checkbox builder
pub struct CheckboxBuilder {
    size: CheckboxSize,
    state: CheckboxState,
    disabled: bool,
    label: Option<String>,
    description: Option<String>,
    error: bool,
    required: bool,
}

impl CheckboxBuilder {
    pub fn new() -> Self {
        Self {
            size: CheckboxSize::Medium,
            state: CheckboxState::Unchecked,
            disabled: false,
            label: None,
            description: None,
            error: false,
            required: false,
        }
    }

    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.state = if checked {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        self
    }

    pub fn state(mut self, state: CheckboxState) -> Self {
        self.state = state;
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

    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    // Signal-based methods for reactive state
    pub fn checked_signal<S>(self, checked_signal: S) -> CheckboxBuilderWithSignal<S>
    where
        S: Signal<Item = bool> + Unpin + 'static,
    {
        CheckboxBuilderWithSignal {
            builder: self,
            checked_signal,
            on_change: None,
        }
    }
}

// Signal-based checkbox builder for reactive state
pub struct CheckboxBuilderWithSignal<S>
where
    S: Signal<Item = bool> + Unpin + 'static,
{
    builder: CheckboxBuilder,
    checked_signal: S,
    on_change: Option<Box<dyn Fn(bool)>>,
}

impl<S> CheckboxBuilderWithSignal<S>
where
    S: Signal<Item = bool> + Unpin + 'static + Clone,
{
    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(bool) + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.builder.disabled = disabled;
        self
    }

    pub fn build(self) -> impl Element {
        let CheckboxBuilder {
            size,
            state: _,
            disabled,
            label,
            description,
            error,
            required: _,
        } = self.builder;

        let on_change = self.on_change;

        // Size-dependent values
        let (size_px, font_size) = match size {
            CheckboxSize::Small => (16, FONT_SIZE_14),
            CheckboxSize::Medium => (20, FONT_SIZE_16),
            CheckboxSize::Large => (24, FONT_SIZE_18),
        };

        // Create the checkbox box
        let checkbox_box = El::new()
            .s(Width::exact(size_px))
            .s(Height::exact(size_px))
            .s(RoundedCorners::all(4))
            .s(Cursor::new(if disabled {
                CursorIcon::NotAllowed
            } else {
                CursorIcon::Pointer
            }))
            .s(Borders::all_signal(
                map_ref! {
                    let theme = theme(),
                    let is_checked = self.checked_signal.clone() =>
                    if error {
                        match *theme {
                            Theme::Light => Border::new().width(2).color("oklch(55% 0.22 25)"), // error_7 light
                            Theme::Dark => Border::new().width(2).color("oklch(65% 0.22 25)"), // error_7 dark
                        }
                    } else if *is_checked {
                        match *theme {
                            Theme::Light => Border::new().width(2).color("oklch(55% 0.22 250)"), // primary_7 light
                            Theme::Dark => Border::new().width(2).color("oklch(65% 0.22 250)"), // primary_7 dark
                        }
                    } else {
                        match *theme {
                            Theme::Light => Border::new().width(1).color("oklch(65% 0.14 250)"), // neutral_6 light
                            Theme::Dark => Border::new().width(1).color("oklch(45% 0.14 250)"), // neutral_6 dark
                        }
                    }
                }
            ))
            .s(Background::new().color_signal(
                map_ref! {
                    let theme = theme(),
                    let is_checked = self.checked_signal.clone() =>
                    if disabled {
                        match *theme {
                            Theme::Light => "oklch(95% 0.14 250)", // neutral_2 light
                            Theme::Dark => "oklch(15% 0.14 250)", // neutral_2 dark
                        }
                    } else if *is_checked {
                        match *theme {
                            Theme::Light => "oklch(55% 0.22 250)", // primary_7 light
                            Theme::Dark => "oklch(65% 0.22 250)", // primary_7 dark
                        }
                    } else {
                        match *theme {
                            Theme::Light => "oklch(100% 0 0)", // neutral_1 light
                            Theme::Dark => "oklch(10% 0 0)", // neutral_1 dark
                        }
                    }
                }
            ))
            .child_signal(
                self.checked_signal.clone().map(move |is_checked| {
                    if is_checked {
                        let icon_size = match size_px {
                            16 => IconSize::Small,
                            20 => IconSize::Medium,
                            24 => IconSize::Large,
                            _ => IconSize::Medium,
                        };

                        Some(
                            IconBuilder::new(IconName::Check)
                                .size(icon_size)
                                .color(if disabled {
                                    IconColor::Muted
                                } else {
                                    IconColor::Primary
                                })
                                .build()
                        )
                    } else {
                        None
                    }
                })
            )
            .on_click({
                move || {
                    if !disabled {
                        if let Some(ref handler) = on_change {
                            // For now, just toggle to true - proper state management can be added later
                            handler(true);
                        }
                    }
                }
            });

        // Return just the checkbox for now - label support can be added later
        checkbox_box.unify()
    }
}

impl CheckboxBuilder {
    pub fn build(self) -> impl Element {
        let checked = Mutable::new(matches!(self.state, CheckboxState::Checked));
        let checked_signal = checked.signal();
        let checked_clone1 = checked.clone();
        let checked_clone2 = checked.clone();
        let checked_clone3 = checked.clone();
        let focused = Mutable::new(false);
        let focused_signal = focused.signal();

        let size_px = self.size.to_px();
        let font_size = self.size.font_size();
        let disabled = self.disabled;
        let error = self.error;
        let state = self.state;

        // Create the checkbox box with proper theming
        let checkbox_box = El::new()
            .s(Width::exact(size_px))
            .s(Height::exact(size_px))
            .s(RoundedCorners::all(4))
            .s(Borders::all_signal(
                checked_clone1.signal().map(move |checked_state| {
                    theme().map(move |t| {
                        let (color, width) = if error {
                            match t {
                                Theme::Light => ("oklch(50% 0.21 30)", 2), // error_7 light
                                Theme::Dark => ("oklch(70% 0.21 30)", 2), // error_7 dark
                            }
                        } else if disabled {
                            match t {
                                Theme::Light => ("oklch(85% 0.14 250)", 1), // neutral_4 light
                                Theme::Dark => ("oklch(25% 0.14 250)", 1), // neutral_4 dark
                            }
                        } else if checked_state {
                            match t {
                                Theme::Light => ("oklch(55% 0.22 250)", 2), // primary_7 light
                                Theme::Dark => ("oklch(65% 0.22 250)", 2), // primary_7 dark
                            }
                        } else {
                            match t {
                                Theme::Light => ("oklch(75% 0.14 250)", 1), // neutral_5 light
                                Theme::Dark => ("oklch(35% 0.14 250)", 1), // neutral_5 dark
                            }
                        };
                        Border::new().width(width).color(color)
                    })
                }).flatten()
            ))
            .s(Background::new().color_signal(
                checked_clone2.signal().map(move |checked_state| {
                    theme().map(move |t| {
                        if disabled {
                            match t {
                                Theme::Light => "oklch(95% 0.005 250)", // neutral_2 light - lighter gray, less saturated
                                Theme::Dark => "oklch(20% 0.005 250)", // darker neutral - less saturated
                            }
                        } else {
                            match state {
                                CheckboxState::Checked => match t {
                                    Theme::Light => "oklch(55% 0.22 250)", // primary_7 light
                                    Theme::Dark => "oklch(65% 0.22 250)", // primary_7 dark
                                },
                                CheckboxState::Indeterminate => match t {
                                    Theme::Light => "oklch(100% 0 0)", // pure white for better contrast with indeterminate icon
                                    Theme::Dark => "oklch(25% 0.14 250)", // neutral_4 dark - keep original
                                },
                                CheckboxState::Unchecked => {
                                    if checked_state {
                                        // This is a normal checkbox that's checked
                                        match t {
                                            Theme::Light => "oklch(55% 0.22 250)", // primary_7 light
                                            Theme::Dark => "oklch(65% 0.22 250)", // primary_7 dark
                                        }
                                    } else {
                                        // This is a normal checkbox that's unchecked
                                        match t {
                                            Theme::Light => "oklch(100% 0 0)", // pure white for consistency
                                            Theme::Dark => "oklch(25% 0.14 250)", // neutral_4 dark - keep original
                                        }
                                    }
                                },
                            }
                        }
                    })
                }).flatten()
            ))
            .s(Shadows::with_signal(
                if disabled {
                    // No shadows for disabled checkboxes
                    always(vec![]).boxed_local()
                } else {
                    // Add shadows based on state and theme
                    checked_clone2.signal().map(move |checked_state| {
                        theme().map(move |t| {
                            match state {
                                CheckboxState::Indeterminate => {
                                    // Indeterminate checkboxes get primary shadows like buttons
                                    match t {
                                        Theme::Light => vec![
                                            Shadow::new().y(1).x(0).blur(2).spread(0).color("rgba(59, 130, 246, 0.25)"),
                                        ],
                                        Theme::Dark => vec![
                                            Shadow::new().y(1).x(0).blur(2).spread(0).color("rgba(59, 130, 246, 0.4)"),
                                        ],
                                    }
                                },
                                _ => {
                                    if checked_state {
                                        // Checked checkboxes get primary shadows like buttons
                                        match t {
                                            Theme::Light => vec![
                                                Shadow::new().y(1).x(0).blur(2).spread(0).color("rgba(59, 130, 246, 0.25)"),
                                            ],
                                            Theme::Dark => vec![
                                                Shadow::new().y(1).x(0).blur(2).spread(0).color("rgba(59, 130, 246, 0.4)"),
                                            ],
                                        }
                                    } else {
                                        // Unchecked checkboxes get subtle neutral shadows
                                        match t {
                                            Theme::Light => vec![
                                                Shadow::new().y(1).x(0).blur(2).spread(0).color("rgba(0, 0, 0, 0.08)"),
                                            ],
                                            Theme::Dark => vec![
                                                Shadow::new().y(1).x(0).blur(2).spread(0).color("rgba(0, 0, 0, 0.3)"),
                                            ],
                                        }
                                    }
                                },
                            }
                        })
                    }).flatten().boxed_local()
                }
            ))
            .s(Align::center())
            .s(Cursor::new(if disabled {
                CursorIcon::NotAllowed
            } else {
                CursorIcon::Pointer
            }))
            .s(Outline::with_signal_self(
                if disabled {
                    // No outline for disabled checkboxes
                    always(None).boxed_local()
                } else {
                    focused_signal.map(|is_focused| {
                        if is_focused {
                            Some(Outline::inner().width(2).color("oklch(0.7 0.15 250)"))
                        } else {
                            None
                        }
                    }).boxed_local()
                }
            ))
            .update_raw_el(move |raw_el| {
                // Make the element focusable
                if !disabled {
                    raw_el.attr("tabindex", "0")
                } else {
                    raw_el.attr("tabindex", "-1")
                }
            })
            .update_raw_el({
                let focused_clone = focused.clone();
                move |raw_el| {
                    if !disabled {
                        let focused_for_focus = focused_clone.clone();
                        let focused_for_blur = focused_clone.clone();
                        raw_el
                            .event_handler(move |_: events::Focus| {
                                focused_for_focus.set(true);
                            })
                            .event_handler(move |_: events::Blur| {
                                focused_for_blur.set(false);
                            })
                    } else {
                        raw_el
                    }
                }
            })
            .child_signal(
                checked_signal.map(move |is_checked| {
                    match state {
                        CheckboxState::Indeterminate => {
                            // Always show indeterminate symbol regardless of checked state
                            let icon_size = match size_px {
                                20 => IconSize::Small,
                                24 => IconSize::Medium,
                                _ => IconSize::Large,
                            };

                            El::new()
                                .s(Padding::all(2)) // Add padding around the icon
                                .child_signal(
                                    theme().map(move |t| {
                                        IconBuilder::new(IconName::Minus)
                                            .size(icon_size)
                                            .color(if disabled {
                                                IconColor::Muted
                                            } else {
                                                match t {
                                                    Theme::Light => IconColor::Custom("oklch(55% 0.22 250)"), // Blue color for light theme
                                                    Theme::Dark => IconColor::Custom("oklch(98% 0.14 250)"), // Light color for dark theme (original)
                                                }
                                            })
                                            .build()
                                    })
                                )
                                .unify()
                        }
                        _ => {
                            // For normal checkboxes, show checkmark based on checked signal
                            if is_checked {
                                let icon_size = match size_px {
                                    20 => IconSize::Small,
                                    24 => IconSize::Medium,
                                    _ => IconSize::Large,
                                };

                                El::new()
                                    .s(Padding::all(2)) // Add padding around the icon
                                    .child(
                                        IconBuilder::new(IconName::Check)
                                            .size(icon_size)
                                            .color(if disabled {
                                                IconColor::Muted
                                            } else {
                                                IconColor::Custom("oklch(98% 0.14 250)") // White/light color for contrast
                                            })
                                            .build()
                                    )
                                    .unify()
                            } else {
                                El::new().unify() // Empty element when unchecked
                            }
                        }
                    }
                })
            )
            .on_click({
                move || {
                    if !disabled {
                        checked_clone3.update(|current| !current);
                    }
                }
            });

        // Build the complete component
        if let Some(label_text) = &self.label {
            let mut items = Vec::new();

            // Main row with checkbox and label
            let main_row = Row::new()
                .s(Gap::new().x(SPACING_8))
                .s(Align::new().center_y()) // Center checkbox vertically with the first line of text
                .item(checkbox_box)
                .item(
                    Column::new()
                        .s(Gap::new().y(SPACING_2))
                        .item(
                            Row::new()
                                .s(Gap::new().x(SPACING_4))
                                .item(
                                    El::new()
                                        .child(Text::new(label_text))
                                        .s(Font::new()
                                            .size(font_size)
                                            .weight(FontWeight::Number(FONT_WEIGHT_4))
                                            .color_signal(theme().map(move |t| {
                                                if disabled {
                                                    match t {
                                                        Theme::Light => "oklch(45% 0.14 250)", // neutral_5 light
                                                        Theme::Dark => "oklch(55% 0.14 250)", // neutral_5 dark
                                                    }
                                                } else {
                                                    match t {
                                                        Theme::Light => "oklch(15% 0.14 250)", // neutral_9 light
                                                        Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                    }
                                                }
                                            }))
                                        )
                                        .s(Cursor::new(if disabled {
                                            CursorIcon::NotAllowed
                                        } else {
                                            CursorIcon::Pointer
                                        }))
                                        .on_click({
                                            let checked_clone = checked.clone();
                                            move || {
                                                if !disabled {
                                                    checked_clone.update(|current| !current);
                                                }
                                            }
                                        })
                                )
                                .item_signal(always(self.required).map(move |required| {
                                    if required {
                                        Some(
                                            El::new()
                                                .child(Text::new("*"))
                                                .s(Font::new()
                                                    .size(font_size)
                                                    .weight(FontWeight::Number(FONT_WEIGHT_5))
                                                    .color_signal(theme().map(|t| match t {
                                                        Theme::Light => "oklch(50% 0.21 30)", // error_7 light
                                                        Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark
                                                    }))
                                                )
                                        )
                                    } else {
                                        None
                                    }
                                }))
                        )
                        .item_signal(always(self.description.clone()).map(move |desc| {
                            desc.map(|description| {
                                El::new()
                                    .child(Text::new(&description))
                                    .s(Font::new()
                                        .size(font_size - 2)
                                        .weight(FontWeight::Number(FONT_WEIGHT_4))
                                        .color_signal(theme().map(move |t| {
                                            if disabled {
                                                match t {
                                                    Theme::Light => "oklch(45% 0.14 250)", // neutral_5 light
                                                    Theme::Dark => "oklch(55% 0.14 250)", // neutral_5 dark
                                                }
                                            } else {
                                                match t {
                                                    Theme::Light => "oklch(35% 0.14 250)", // neutral_7 light
                                                    Theme::Dark => "oklch(75% 0.14 250)", // neutral_9 dark
                                                }
                                            }
                                        }))
                                    )
                            })
                        }))
                );

            items.push(main_row.unify());

            Column::new()
                .s(Gap::new().y(SPACING_4))
                .items(items)
                .unify()
        } else {
            checkbox_box.unify()
        }
    }
}

// Convenience function
pub fn checkbox() -> CheckboxBuilder {
    CheckboxBuilder::new()
}
