use zoon::*;
use crate::tokens::*;

// FileInput variants
#[derive(Debug, Clone, Copy)]
pub enum FileInputVariant {
    Default,    // Standard file input
    Dropzone,   // Drag and drop area
    Button,     // Button-style file input
}

// FileInput sizes
#[derive(Debug, Clone, Copy)]
pub enum FileInputSize {
    Small,   // Compact size
    Medium,  // Default size
    Large,   // Larger size
}

// File type restrictions
#[derive(Debug, Clone)]
pub enum FileType {
    Any,
    Images,
    Documents,
    Custom(Vec<String>), // Custom MIME types
}

impl FileType {
    pub fn accept_string(&self) -> String {
        match self {
            FileType::Any => "*/*".to_string(),
            FileType::Images => "image/*".to_string(),
            FileType::Documents => ".pdf,.doc,.docx,.txt".to_string(),
            FileType::Custom(types) => types.join(","),
        }
    }
}

// FileInput builder
pub struct FileInputBuilder {
    variant: FileInputVariant,
    size: FileInputSize,
    accept: FileType,
    multiple: bool,
    disabled: bool,
    placeholder: Option<String>,
    max_size: Option<u64>, // Max file size in bytes
    on_change: Option<Box<dyn Fn(Vec<String>)>>, // File names for demo
    on_error: Option<Box<dyn Fn(String)>>,
}

impl FileInputBuilder {
    pub fn new() -> Self {
        Self {
            variant: FileInputVariant::Default,
            size: FileInputSize::Medium,
            accept: FileType::Any,
            multiple: false,
            disabled: false,
            placeholder: None,
            max_size: None,
            on_change: None,
            on_error: None,
        }
    }

    pub fn variant(mut self, variant: FileInputVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: FileInputSize) -> Self {
        self.size = size;
        self
    }

    pub fn accept(mut self, accept: FileType) -> Self {
        self.accept = accept;
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn max_size(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }

    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(Vec<String>) + 'static
    {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn on_error<F>(mut self, handler: F) -> Self
    where
        F: Fn(String) + 'static
    {
        self.on_error = Some(Box::new(handler));
        self
    }

    pub fn build(self) -> impl Element {
        match self.variant {
            FileInputVariant::Default => self.build_default().unify(),
            FileInputVariant::Dropzone => self.build_dropzone().unify(),
            FileInputVariant::Button => self.build_button().unify(),
        }
    }

    fn build_default(self) -> impl Element {
        let (padding_x, padding_y, font_size) = match self.size {
            FileInputSize::Small => (SPACING_8, SPACING_6, FONT_SIZE_14),
            FileInputSize::Medium => (SPACING_12, SPACING_8, FONT_SIZE_16),
            FileInputSize::Large => (SPACING_16, SPACING_12, FONT_SIZE_18),
        };

        let placeholder_text = self.placeholder.unwrap_or_else(|| {
            if self.multiple {
                "Choose files...".to_string()
            } else {
                "Choose file...".to_string()
            }
        });

        let border_color = if self.disabled {
            "#d1d5db" // gray-300
        } else {
            "#e5e7eb" // gray-200
        };

        let background_color = if self.disabled {
            "#f9fafb" // gray-50
        } else {
            "#ffffff" // white
        };

        let text_color = if self.disabled {
            "#9ca3af" // gray-400
        } else {
            "#6b7280" // gray-500
        };

        let mut file_input = Row::new()
            .s(Width::fill())
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(Borders::all(Border::new().width(1).color(border_color)))
            .s(RoundedCorners::all(6))
            .s(Background::new().color(background_color))
            .s(Align::new().center_y())
            .item(
                El::new()
                    .s(Width::fill())
                    .s(Font::new()
                        .size(font_size)
                        .color(text_color)
                    )
                    .child(Text::new(&placeholder_text))
            )
            .item(
                El::new()
                    .s(Font::new()
                        .size(FONT_SIZE_14)
                        .color("#6b7280") // gray-500
                    )
                    .child(Text::new("📁"))
            );

        if !self.disabled {
            file_input = file_input.s(Cursor::new(CursorIcon::Pointer));

            if let Some(handler) = self.on_change {
                file_input = file_input.on_click(move || {
                    // In a real implementation, this would open file dialog
                    handler(vec!["demo-file.txt".to_string()]);
                });
            }
        }

        file_input
    }

    fn build_dropzone(self) -> impl Element {
        let (padding, font_size) = match self.size {
            FileInputSize::Small => (SPACING_16, FONT_SIZE_14),
            FileInputSize::Medium => (SPACING_24, FONT_SIZE_16),
            FileInputSize::Large => (SPACING_32, FONT_SIZE_18),
        };

        let placeholder_text = self.placeholder.unwrap_or_else(|| {
            "Drag and drop files here, or click to select".to_string()
        });

        let border_color = if self.disabled {
            "#d1d5db" // gray-300
        } else {
            "#e5e7eb" // gray-200
        };

        let background_color = if self.disabled {
            "#f9fafb" // gray-50
        } else {
            "#fafafa" // gray-50
        };

        let text_color = if self.disabled {
            "#9ca3af" // gray-400
        } else {
            "#6b7280" // gray-500
        };

        let mut dropzone = Column::new()
            .s(Width::fill())
            .s(Padding::all(padding))
            .s(Borders::all(Border::new().width(2).color(border_color)))
            .s(RoundedCorners::all(8))
            .s(Background::new().color(background_color))
            .s(Align::center())
            .s(Gap::new().y(SPACING_8))
            .item(
                El::new()
                    .s(Font::new()
                        .size(32)
                        .color(text_color)
                    )
                    .child(Text::new("📤"))
            )
            .item(
                El::new()
                    .s(Font::new()
                        .size(font_size)
                        .color(text_color)
                        .center()
                    )
                    .child(Text::new(&placeholder_text))
            );

        if !self.disabled {
            dropzone = dropzone.s(Cursor::new(CursorIcon::Pointer));

            if let Some(handler) = self.on_change {
                dropzone = dropzone.on_click(move || {
                    // In a real implementation, this would handle file drop/selection
                    handler(vec!["dropped-file.pdf".to_string()]);
                });
            }
        }

        dropzone
    }

    fn build_button(self) -> impl Element {
        let (padding_x, padding_y, font_size) = match self.size {
            FileInputSize::Small => (SPACING_12, SPACING_6, FONT_SIZE_14),
            FileInputSize::Medium => (SPACING_16, SPACING_8, FONT_SIZE_16),
            FileInputSize::Large => (SPACING_20, SPACING_12, FONT_SIZE_18),
        };

        let button_text = self.placeholder.unwrap_or_else(|| {
            if self.multiple {
                "Select Files".to_string()
            } else {
                "Select File".to_string()
            }
        });

        let (background_color, text_color, border_color) = if self.disabled {
            ("#f3f4f6", "#9ca3af", "#d1d5db") // gray colors
        } else {
            ("#3b82f6", "#ffffff", "#3b82f6") // blue colors
        };

        let mut button = Row::new()
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(Background::new().color(background_color))
            .s(Borders::all(Border::new().width(1).color(border_color)))
            .s(RoundedCorners::all(6))
            .s(Align::center())
            .s(Gap::new().x(SPACING_8))
            .item(
                El::new()
                    .s(Font::new()
                        .size(font_size)
                        .color(text_color)
                    )
                    .child(Text::new("📁"))
            )
            .item(
                El::new()
                    .s(Font::new()
                        .size(font_size)
                        .color(text_color)
                        .weight(FontWeight::Medium)
                    )
                    .child(Text::new(&button_text))
            );

        if !self.disabled {
            button = button.s(Cursor::new(CursorIcon::Pointer));

            if let Some(handler) = self.on_change {
                button = button.on_click(move || {
                    // In a real implementation, this would open file dialog
                    handler(vec!["selected-file.jpg".to_string()]);
                });
            }
        }

        button
    }
}

// Convenience functions
pub fn file_input() -> FileInputBuilder {
    FileInputBuilder::new()
}

pub fn file_dropzone() -> FileInputBuilder {
    FileInputBuilder::new().variant(FileInputVariant::Dropzone)
}

pub fn file_button() -> FileInputBuilder {
    FileInputBuilder::new().variant(FileInputVariant::Button)
}
