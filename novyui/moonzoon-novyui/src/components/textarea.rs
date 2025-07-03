use zoon::*;
use crate::tokens::*;

// TextArea sizes
#[derive(Debug, Clone, Copy)]
pub enum TextAreaSize {
    Small,   // Smaller padding and font
    Medium,  // Default size
    Large,   // Larger padding and font
}

// TextArea builder
pub struct TextAreaBuilder {
    size: TextAreaSize,
    placeholder: Option<String>,
    value: Option<String>,
    label: Option<String>,
    error_message: Option<String>,
    required: bool,
    rows: u32,
    disabled: bool,
    readonly: bool,
}

impl TextAreaBuilder {
    pub fn new() -> Self {
        Self {
            size: TextAreaSize::Medium,
            placeholder: None,
            value: None,
            label: None,
            error_message: None,
            required: false,
            rows: 4,
            disabled: false,
            readonly: false,
        }
    }

    pub fn size(mut self, size: TextAreaSize) -> Self {
        self.size = size;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn error_message(mut self, message: impl Into<String>) -> Self {
        self.error_message = Some(message.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows;
        self
    }



    pub fn build(self) -> impl Element {
        // Size-based styling
        let (padding, font_size, min_height) = match self.size {
            TextAreaSize::Small => (SPACING_8, FONT_SIZE_14, 80),
            TextAreaSize::Medium => (SPACING_12, FONT_SIZE_16, 100),
            TextAreaSize::Large => (SPACING_16, FONT_SIZE_18, 120),
        };

        // Calculate height based on rows
        let line_height = 20;
        let calculated_height = (self.rows * line_height) + (padding * 2);
        let height = calculated_height.max(min_height);

        // Capture values for closures
        let disabled = self.disabled;
        let readonly = self.readonly;
        let has_error = self.error_message.is_some();

        // Prepare all data upfront to avoid flag type issues
        let placeholder_text = self.placeholder.as_deref().unwrap_or("");
        let initial_value = self.value.as_deref().unwrap_or("");

        // Create textarea with all flags set at once
        let textarea = TextArea::new()
            .placeholder(
                Placeholder::new(placeholder_text)
                    .s(Font::new().color_signal(theme().map(|t| match t {
                        Theme::Light => "oklch(65% 0.14 250)", // neutral_6 light
                        Theme::Dark => "oklch(55% 0.14 250)", // neutral_7 dark
                    })))
            )
            .text(initial_value)
            .read_only(readonly)
            .label_hidden("textarea")
        .s(Width::fill())
        .s(Height::exact(height))
        .s(Padding::all(padding))
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
        .s(Background::new().color_signal(theme().map(move |t| match t {
            Theme::Light => "oklch(98% 0.14 250)", // neutral_1 light
            Theme::Dark => "oklch(8% 0.14 250)", // neutral_2 dark
        })))
        .s(Borders::all_signal(theme().map(move |t| {
            let color = if has_error {
                match t {
                    Theme::Light => "oklch(50% 0.21 30)", // error_7 light
                    Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark
                }
            } else if disabled {
                match t {
                    Theme::Light => "oklch(85% 0.14 250)", // neutral_4 light
                    Theme::Dark => "oklch(25% 0.14 250)", // neutral_4 dark
                }
            } else {
                match t {
                    Theme::Light => "oklch(75% 0.14 250)", // neutral_5 light
                    Theme::Dark => "oklch(35% 0.14 250)", // neutral_5 dark
                }
            };
            Border::new().width(1).color(color)
        })))
        .s(RoundedCorners::all(6))
        .s(Cursor::new(if disabled {
            CursorIcon::NotAllowed
        } else {
            CursorIcon::Text
        }));

        // Build container with all items at once to avoid flag issues
        let mut items = Vec::new();

        // Add label if provided
        if let Some(label_text) = &self.label {
            let label_items = if self.required {
                vec![
                    El::new()
                        .child(Text::new(label_text))
                        .s(Font::new()
                            .size(FONT_SIZE_14)
                            .weight(FontWeight::Number(FONT_WEIGHT_5))
                            .color_signal(theme().map(|t| match t {
                                Theme::Light => "oklch(25% 0.14 250)", // neutral_8 light
                                Theme::Dark => "oklch(75% 0.14 250)", // neutral_10 dark
                            }))
                        ),
                    El::new()
                        .child(Text::new("*"))
                        .s(Font::new()
                            .size(FONT_SIZE_14)
                            .weight(FontWeight::Number(FONT_WEIGHT_5))
                            .color_signal(theme().map(|t| match t {
                                Theme::Light => "oklch(50% 0.21 30)", // error_7 light
                                Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark
                            }))
                        ),
                ]
            } else {
                vec![
                    El::new()
                        .child(Text::new(label_text))
                        .s(Font::new()
                            .size(FONT_SIZE_14)
                            .weight(FontWeight::Number(FONT_WEIGHT_5))
                            .color_signal(theme().map(|t| match t {
                                Theme::Light => "oklch(25% 0.14 250)", // neutral_8 light
                                Theme::Dark => "oklch(75% 0.14 250)", // neutral_10 dark
                            }))
                        ),
                ]
            };

            let label_row = Row::new()
                .s(Gap::new().x(SPACING_4))
                .items(label_items);

            items.push(label_row.unify());
        }

        // Add textarea
        items.push(textarea.unify());

        // Add error message if provided
        if let Some(error_text) = &self.error_message {
            let error_element = El::new()
                .child(Text::new(error_text))
                .s(Font::new()
                    .size(match self.size {
                        TextAreaSize::Small => FONT_SIZE_12,
                        TextAreaSize::Medium => FONT_SIZE_12,
                        TextAreaSize::Large => FONT_SIZE_14,
                    })
                    .weight(FontWeight::Number(FONT_WEIGHT_5))
                    .color_signal(theme().map(|t| match t {
                        Theme::Light => "oklch(50% 0.21 30)", // error_7 light
                        Theme::Dark => "oklch(70% 0.21 30)", // error_7 dark
                    }))
                );
            items.push(error_element.unify());
        }

        // Build final container
        Column::new()
            .s(Gap::new().y(SPACING_6))
            .s(Width::fill())
            .items(items)
    }


}

// Convenience function
pub fn textarea() -> TextAreaBuilder {
    TextAreaBuilder::new()
}
