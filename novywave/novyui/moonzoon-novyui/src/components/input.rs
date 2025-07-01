// Input Component
// Matches Vue Storybook Input component exactly

use crate::tokens::*;
use crate::components::icon::{icon_str, IconName, IconSize, IconColor};
use zoon::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputKind {
    Text,
    Email,
    Password,
    Search,
    Number,
    Tel,
    Url,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputState {
    Default,
    Error,
    Disabled,
    Readonly,
}

pub struct InputBuilder {
    placeholder: String,
    value: String,
    size: InputSize,
    input_kind: InputKind,
    state: InputState,
    label: Option<String>,
    error_message: Option<String>,
    required: bool,
    left_icon: Option<IconName>,
    right_icon: Option<IconName>,
    on_change: Option<Box<dyn Fn(String)>>,
    on_focus: Option<Box<dyn Fn()>>,
    on_blur: Option<Box<dyn Fn()>>,
}

impl InputBuilder {
    pub fn new() -> Self {
        Self {
            placeholder: String::new(),
            value: String::new(),
            size: InputSize::Medium,
            input_kind: InputKind::Text,
            state: InputState::Default,
            label: None,
            error_message: None,
            required: false,
            left_icon: None,
            right_icon: None,
            on_change: None,
            on_focus: None,
            on_blur: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    pub fn input_kind(mut self, input_kind: InputKind) -> Self {
        self.input_kind = input_kind;
        self
    }

    pub fn state(mut self, state: InputState) -> Self {
        self.state = state;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn error_message(mut self, message: impl Into<String>) -> Self {
        self.error_message = Some(message.into());
        self.state = InputState::Error;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.state = InputState::Disabled;
        self
    }

    pub fn readonly(mut self) -> Self {
        self.state = InputState::Readonly;
        self
    }

    pub fn left_icon(mut self, icon: IconName) -> Self {
        self.left_icon = Some(icon);
        self
    }

    pub fn right_icon(mut self, icon: IconName) -> Self {
        self.right_icon = Some(icon);
        self
    }

    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) + 'static,
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn on_focus<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_focus = Some(Box::new(handler));
        self
    }

    pub fn on_blur<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_blur = Some(Box::new(handler));
        self
    }

    pub fn build(self) -> impl Element {
        let (focused, focused_signal) = Mutable::new_and_signal(false);

        // Size-based styling matching Vue implementation exactly
        let (container_height, padding_x, padding_y, font_size) = match self.size {
            InputSize::Small => (32, SPACING_8, SPACING_4, FONT_SIZE_14),
            InputSize::Medium => (44, SPACING_12, SPACING_6, FONT_SIZE_16),
            InputSize::Large => (48, SPACING_16, SPACING_8, FONT_SIZE_18),
        };

        // Use signal-based approach to handle optional components
        Column::new()
            .s(Width::fill())
            .s(Gap::new().y(SPACING_2))
            .item_signal(always(self.label.clone()).map(move |label_opt| {
                label_opt.map(|label| {
                    let mut label_row = Row::new()
                        .s(Gap::new().x(SPACING_2))
                        .item(
                            El::new()
                                .child(Text::new(&label))
                                .s(Font::new()
                                    .size(match self.size {
                                        InputSize::Small => FONT_SIZE_12,
                                        InputSize::Medium => FONT_SIZE_14,
                                        InputSize::Large => FONT_SIZE_16,
                                    })
                                    .weight(FontWeight::Number(FONT_WEIGHT_6))
                                    .color_signal(theme().map(|t| match t {
                                        Theme::Light => "oklch(30% 0.07 255)", // neutral_9 light (Vue exact)
                                        Theme::Dark => "oklch(92% 0.07 255)", // neutral_9 dark (Vue exact)
                                    }))
                                )
                        );

                    // Add required indicator if needed
                    if self.required {
                        label_row = label_row.item(
                            El::new()
                                .child(Text::new("*"))
                                .s(Font::new()
                                    .color_signal(theme().map(|t| match t {
                                        Theme::Light => "oklch(50% 0.21 30)", // error_7 light (Vue exact)
                                        Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark (Vue exact)
                                    }))
                                )
                        );
                    }

                    label_row
                })
            }))
            .item(self.build_input_container(focused, focused_signal, container_height, padding_x, padding_y, font_size))
            .item_signal(always(self.error_message.clone()).map(move |error_opt| {
                error_opt.map(|error_msg| {
                    El::new()
                        .child(Text::new(&error_msg))
                        .s(Font::new()
                            .size(match self.size {
                                InputSize::Small => FONT_SIZE_12,
                                InputSize::Medium => FONT_SIZE_12,
                                InputSize::Large => FONT_SIZE_14,
                            })
                            .weight(FontWeight::Number(FONT_WEIGHT_5))
                            .color_signal(theme().map(|t| match t {
                                Theme::Light => "oklch(50% 0.21 30)", // error_7 light (Vue exact)
                                Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark (Vue exact)
                            }))
                        )
                })
            }))
    }

    fn build_input_container(
        &self,
        focused: Mutable<bool>,
        focused_signal: impl Signal<Item = bool> + Unpin + 'static,
        container_height: u32,
        padding_x: u32,
        padding_y: u32,
        font_size: u32,
    ) -> impl Element {
        let state = self.state;
        let placeholder = self.placeholder.clone();
        let value = self.value.clone();
        let left_icon = self.left_icon;
        let right_icon = self.right_icon;
        let size = self.size;

        // Container with proper styling matching Vue implementation
        Row::new()
            .s(Width::fill())
            .s(Height::exact(container_height))
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(Gap::new().x(SPACING_8))
            .s(Align::new().center_y())
            .s(RoundedCorners::all(CORNER_RADIUS_4)) // cornerRadius['4px']
            .s(transition_colors())
            .s(Background::new().color_signal(
                theme().map(move |t| match (state, t) {
                    // Disabled: More muted background (revert to original)
                    (InputState::Disabled, Theme::Light) => "oklch(92% 0.045 255)", // neutral_3 light (more muted)
                    (InputState::Disabled, Theme::Dark) => "oklch(30% 0.045 255)", // neutral_3 dark (more muted) - reverted
                    // Read-only: Subtle background - adjust dark theme to be closer to disabled
                    (InputState::Readonly, Theme::Light) => "oklch(96% 0.035 255)", // neutral_2 light (Vue exact)
                    (InputState::Readonly, Theme::Dark) => "oklch(28% 0.045 255)", // closer to disabled for similar appearance
                    (InputState::Error, Theme::Light) => "oklch(98% 0.03 30)", // error_1 light (Vue exact)
                    (InputState::Error, Theme::Dark) => "oklch(12% 0.03 30)", // error_1 dark (Vue exact)
                    (_, Theme::Light) => "oklch(99% 0.025 255)", // neutral_1 light (Vue exact)
                    (_, Theme::Dark) => "oklch(12% 0.025 255)", // neutral_1 dark (Vue exact)
                })
            ))
            .s(Outline::with_signal_self(
                map_ref! {
                    let focused = focused_signal,
                    let theme = theme() => move {
                        let width = 1;
                        let color = match (state, *focused, theme) {
                            (InputState::Error, _, _) => match theme {
                                Theme::Light => "oklch(50% 0.21 30)", // error_7 light (Vue exact)
                                Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark (Vue exact)
                            },
                            (InputState::Disabled, _, _) => match theme {
                                Theme::Light => "oklch(88% 0.055 255)", // neutral_4 light (more muted for disabled)
                                Theme::Dark => "oklch(35% 0.055 255)", // neutral_4 dark (more muted for disabled) - reverted
                            },
                            (InputState::Readonly, _, _) => match theme {
                                Theme::Light => "oklch(92% 0.045 255)", // neutral_3 light (Vue exact)
                                Theme::Dark => "oklch(33% 0.055 255)", // closer to disabled border for similar appearance
                            },
                            (_, true, _) => match theme {
                                Theme::Light => "oklch(55% 0.16 250)", // primary_7 light (Vue exact)
                                Theme::Dark => "oklch(65% 0.16 250)", // primary_7 dark (Vue exact)
                            },
                            (_, false, _) => match theme {
                                Theme::Light => "oklch(90% 0.05 250)", // neutral_3 light (Vue exact)
                                Theme::Dark => "oklch(30% 0.05 250)", // neutral_3 dark (Vue exact)
                            },
                        };
                        Some(Outline::inner().width(width).color(color))
                    }
                }
            ))
            .s(Cursor::new(match state {
                InputState::Disabled => CursorIcon::NotAllowed,
                _ => CursorIcon::Text,
            }))
            .s(Shadows::with_signal(
                theme().map(move |t| {
                    if matches!(state, InputState::Disabled) {
                        // No shadows for disabled inputs
                        vec![]
                    } else {
                        // Add inner shadows to create inset appearance (like form fields) - enhanced visibility
                        match t {
                            Theme::Light => vec![
                                Shadow::new().inner().y(2).x(0).blur(4).spread(0).color("rgba(0, 0, 0, 0.12)"),
                                Shadow::new().inner().y(1).x(0).blur(2).spread(0).color("rgba(0, 0, 0, 0.18)"),
                                Shadow::new().inner().y(0).x(1).blur(2).spread(0).color("rgba(0, 0, 0, 0.08)"),
                            ],
                            Theme::Dark => vec![
                                Shadow::new().inner().y(-2).x(0).blur(4).spread(0).color("rgba(255, 255, 255, 0.25)"),
                                Shadow::new().inner().y(-1).x(0).blur(2).spread(0).color("rgba(255, 255, 255, 0.35)"),
                                Shadow::new().inner().y(0).x(-1).blur(2).spread(0).color("rgba(255, 255, 255, 0.2)"),
                            ],
                        }
                    }
                })
            ))
            .update_raw_el(move |raw_el| {
                if matches!(state, InputState::Disabled) {
                    // Disabled state - use opacity token
                    raw_el.style("opacity", OPACITY_DISABLED)
                } else {
                    raw_el.style("opacity", OPACITY_ENABLED)
                }
            })
            .item_signal(always(left_icon).map(move |icon_opt| {
                icon_opt.map(|icon_name| {
                    icon_str(&icon_name.to_kebab_case())
                        .size(match size {
                            InputSize::Small => IconSize::Small,
                            InputSize::Medium => IconSize::Medium,
                            InputSize::Large => IconSize::Large,
                        })
                        .color(match state {
                            InputState::Disabled => IconColor::Muted,
                            InputState::Readonly => IconColor::Muted, // Slightly muted for readonly
                            InputState::Error => IconColor::Error,
                            _ => IconColor::Secondary,
                        })
                        .build()
                })
            }))
            .item(
                TextInput::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .s(Font::new()
                        .size(font_size)
                        .weight(FontWeight::Number(FONT_WEIGHT_5))
                        .color_signal(
                            theme().map(move |t| match (state, t) {
                                (InputState::Disabled, _) => "oklch(70% 0.09 255)", // neutral_6 (muted for disabled)
                                (InputState::Readonly, Theme::Light) => "oklch(35% 0.08 255)", // slightly muted for readonly
                                (InputState::Readonly, Theme::Dark) => "oklch(90% 0.04 255)", // slightly muted for readonly
                                (_, Theme::Light) => "oklch(30% 0.07 255)", // neutral_9 light (Vue exact)
                                (_, Theme::Dark) => "oklch(96% 0.035 255)", // neutral_10 dark (Vue exact)
                            })
                        )
                    )
                    .s(Background::new().color("transparent"))
                    .s(Borders::new())
                    .placeholder(
                        Placeholder::new(&placeholder)
                            .s(Font::new().color_signal(theme().map(|t| match t {
                                Theme::Light => "oklch(35% 0.14 250)", // primary_9 light (Vue exact)
                                Theme::Dark => "oklch(85% 0.14 250)", // primary_9 dark (Vue exact)
                            })))
                    )
                    .text(&value)
                    .read_only(matches!(state, InputState::Readonly))
                    .label_hidden("Input")
                    .on_change({
                        let focused = focused.clone();
                        move |new_value| {
                            // Handle change
                        }
                    })
                    .on_focus({
                        let focused = focused.clone();
                        move || {
                            focused.set(true);
                        }
                    })
                    .on_blur({
                        let focused = focused.clone();
                        move || {
                            focused.set(false);
                        }
                    })
            )
            .item_signal(always(right_icon).map(move |icon_opt| {
                icon_opt.map(|icon_name| {
                    icon_str(&icon_name.to_kebab_case())
                        .size(match size {
                            InputSize::Small => IconSize::Small,
                            InputSize::Medium => IconSize::Medium,
                            InputSize::Large => IconSize::Large,
                        })
                        .color(match state {
                            InputState::Disabled => IconColor::Muted,
                            InputState::Readonly => IconColor::Muted, // Slightly muted for readonly
                            InputState::Error => IconColor::Error,
                            _ => IconColor::Secondary,
                        })
                        .build()
                })
            }))
    }
}

// Convenience function
pub fn input() -> InputBuilder {
    InputBuilder::new()
}
