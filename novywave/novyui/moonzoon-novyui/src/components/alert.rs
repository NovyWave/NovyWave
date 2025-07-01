use zoon::*;
use crate::tokens::*;
use crate::tokens::color::*;

// Alert variants
#[derive(Debug, Clone, Copy)]
pub enum AlertVariant {
    Info,       // Blue theme
    Success,    // Green theme
    Warning,    // Yellow theme
    Error,      // Red theme
    Default,    // Neutral theme
}

impl AlertVariant {
    pub fn background_color(self) -> impl Signal<Item = &'static str> {
        match self {
            AlertVariant::Info => primary_1().boxed_local(),
            AlertVariant::Success => success_1().boxed_local(),
            AlertVariant::Warning => warning_1().boxed_local(), 
            AlertVariant::Error => error_1().boxed_local(),
            AlertVariant::Default => neutral_2().boxed_local(),
        }
    }

    pub fn border_color(self) -> impl Signal<Item = &'static str> {
        match self {
            AlertVariant::Info => primary_7().boxed_local(),
            AlertVariant::Success => success_7().boxed_local(),
            AlertVariant::Warning => warning_7().boxed_local(),
            AlertVariant::Error => error_7().boxed_local(),
            AlertVariant::Default => neutral_6().boxed_local(),
        }
    }

    pub fn text_color(self) -> impl Signal<Item = &'static str> {
        match self {
            AlertVariant::Info => primary_9().boxed_local(),
            AlertVariant::Success => success_9().boxed_local(),
            AlertVariant::Warning => warning_9().boxed_local(),
            AlertVariant::Error => error_9().boxed_local(),
            AlertVariant::Default => neutral_10().boxed_local(),
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            AlertVariant::Info => "ℹ️",
            AlertVariant::Success => "✅",
            AlertVariant::Warning => "⚠️",
            AlertVariant::Error => "❌",
            AlertVariant::Default => "📝",
        }
    }
}

// Alert builder
pub struct AlertBuilder {
    variant: AlertVariant,
    title: Option<String>,
    message: String,
    dismissible: bool,
    show_icon: bool,
    on_dismiss: Option<Box<dyn Fn()>>,
}

impl AlertBuilder {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            variant: AlertVariant::Default,
            title: None,
            message: message.into(),
            dismissible: false,
            show_icon: true,
            on_dismiss: None,
        }
    }

    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    pub fn show_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;
        self
    }

    pub fn on_dismiss<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static
    {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn build(self) -> impl Element {
        // Simplified alert implementation
        let icon_text = if self.show_icon {
            format!("{} ", self.variant.icon())
        } else {
            String::new()
        };

        let display_text = if let Some(title_text) = &self.title {
            format!("{}{}: {}", icon_text, title_text, self.message)
        } else {
            format!("{}{}", icon_text, self.message)
        };

        let dismiss_text = if self.dismissible { " ✕" } else { "" };

        let mut alert = Row::new()
            .s(Width::fill())
            .s(Padding::new().x(SPACING_16).y(SPACING_12))
            .s(Gap::new().x(SPACING_12))
            .s(Borders::new().left_signal(self.variant.border_color().map(|color| Border::new().width(4).color(color))))
            .s(Background::new().color_signal(self.variant.background_color()))
            .s(RoundedCorners::all(CORNER_RADIUS_6))
            .s(Align::new().center_y())
            .item(
                El::new()
                    .s(Width::fill())
                    .s(Font::new()
                        .size(FONT_SIZE_14)
                        .color_signal(self.variant.text_color())
                    )
                    .child(Text::new(&display_text))
            );

        if self.dismissible {
            let mut dismiss_button = El::new()
                .s(Font::new()
                    .size(FONT_SIZE_14)
                    .color_signal(self.variant.text_color())
                )
                .s(Cursor::new(CursorIcon::Pointer))
                .child(Text::new(dismiss_text));

            if let Some(handler) = self.on_dismiss {
                dismiss_button = dismiss_button.on_click(move || handler());
            }

            alert = alert.item(dismiss_button);
        }

        alert
    }
}

// Convenience functions
pub fn alert(message: impl Into<String>) -> AlertBuilder {
    AlertBuilder::new(message)
}

pub fn info_alert(message: impl Into<String>) -> AlertBuilder {
    AlertBuilder::new(message).variant(AlertVariant::Info)
}

pub fn success_alert(message: impl Into<String>) -> AlertBuilder {
    AlertBuilder::new(message).variant(AlertVariant::Success)
}

pub fn warning_alert(message: impl Into<String>) -> AlertBuilder {
    AlertBuilder::new(message).variant(AlertVariant::Warning)
}

pub fn error_alert(message: impl Into<String>) -> AlertBuilder {
    AlertBuilder::new(message).variant(AlertVariant::Error)
}
