use zoon::*;
use crate::tokens::*;

// Kbd sizes
#[derive(Debug, Clone, Copy)]
pub enum KbdSize {
    Small,   // 12px font, 4px padding, 20px min-width, 2px border-radius
    Medium,  // 14px font, 8px padding, 24px min-width, 4px border-radius (default)
    Large,   // 16px font, 12px padding, 32px min-width, 4px border-radius
}

// Kbd variants
#[derive(Debug, Clone, Copy)]
pub enum KbdVariant {
    Default,  // Mimics physical keyboard keys with depth
    Outlined, // Simple border with transparent background
    Solid,    // Solid background with high contrast
}

// Kbd builder
pub struct KbdBuilder {
    size: KbdSize,
    variant: KbdVariant,
    text: String,
    aria_label: Option<String>,
}

impl KbdBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            size: KbdSize::Medium,
            variant: KbdVariant::Default,
            text: text.into(),
            aria_label: None,
        }
    }

    pub fn size(mut self, size: KbdSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: KbdVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn build(self) -> impl Element {
        // Size-based styling
        let (padding_x, padding_y, font_size, min_width, border_radius) = match self.size {
            KbdSize::Small => (SPACING_4, SPACING_4, FONT_SIZE_12, 20, CORNER_RADIUS_2),
            KbdSize::Medium => (SPACING_8, SPACING_4, FONT_SIZE_14, 24, CORNER_RADIUS_4),
            KbdSize::Large => (SPACING_12, SPACING_8, FONT_SIZE_16, 32, CORNER_RADIUS_4),
        };

        // Create the element with theme-aware styling
        El::new()
            .s(Width::growable().min(min_width))
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(Align::new().left())
            .s(Font::new()
                .size(font_size)
                .color_signal(
                    theme().map(move |t| match (t, self.variant) {
                        (Theme::Dark, KbdVariant::Solid) => "oklch(98% 0.01 250)",  // neutral_1
                        (Theme::Light, KbdVariant::Solid) => "oklch(98% 0.01 250)", // neutral_1
                        (Theme::Dark, KbdVariant::Outlined) => "oklch(96% 0.035 255)", // neutral_10
                        (Theme::Light, KbdVariant::Outlined) => "oklch(30% 0.07 255)", // neutral_9
                        (Theme::Dark, KbdVariant::Default) => "oklch(96% 0.035 255)", // neutral_10
                        (Theme::Light, KbdVariant::Default) => "oklch(30% 0.07 255)", // neutral_9
                    })
                )
                .weight(FontWeight::Medium)
                .family([FontFamily::new("FiraCode"), FontFamily::new("Menlo"), FontFamily::new("Monaco"), FontFamily::new("Consolas"), FontFamily::Monospace])
            )
            .s(Background::new().color_signal(
                theme().map(move |t| match (t, self.variant) {
                    (Theme::Dark, KbdVariant::Solid) => "oklch(45% 0.09 255)",    // neutral_7
                    (Theme::Light, KbdVariant::Solid) => "oklch(45% 0.09 255)",   // neutral_8
                    (_, KbdVariant::Outlined) => "transparent",
                    (Theme::Dark, KbdVariant::Default) => "oklch(25% 0.03 250)",  // neutral_3
                    (Theme::Light, KbdVariant::Default) => "oklch(98% 0.01 250)", // neutral_1
                })
            ))
            .s(Borders::all_signal(
                theme().map(move |t| {
                    let color = match (t, self.variant) {
                        (Theme::Dark, KbdVariant::Solid) => "oklch(45% 0.09 255)",    // neutral_7
                        (Theme::Light, KbdVariant::Solid) => "oklch(45% 0.09 255)",   // neutral_8
                        (Theme::Dark, KbdVariant::Outlined) => "oklch(55% 0.13 250)", // neutral_6
                        (Theme::Light, KbdVariant::Outlined) => "oklch(85% 0.07 250)", // neutral_4
                        (Theme::Dark, KbdVariant::Default) => "oklch(45% 0.10 250)",  // neutral_5
                        (Theme::Light, KbdVariant::Default) => "oklch(90% 0.05 250)", // neutral_3
                    };
                    Border::new().width(1).color(color)
                })
            ))
            .s(RoundedCorners::all(border_radius))
            .s(Align::center())
            .update_raw_el(|raw_el| {
                let variant = self.variant;
                raw_el
                    .style("transition", "color 150ms cubic-bezier(0.4, 0, 0.2, 1), background-color 150ms cubic-bezier(0.4, 0, 0.2, 1), border-color 150ms cubic-bezier(0.4, 0, 0.2, 1)")
                    .style("white-space", "nowrap")
                    .style("user-select", "none")
                    .style("text-align", "center")
                    .style("vertical-align", "baseline")
                    .style("display", "inline-flex")
                    .style("align-items", "center")
                    .style("justify-content", "center")
                    .style_signal("box-shadow",
                        theme().map(move |t| match (t, variant) {
                            (Theme::Dark, KbdVariant::Solid) => "0 1px 2px oklch(0% 0 0 / 0.3)",
                            (Theme::Light, KbdVariant::Solid) => "0 1px 2px oklch(0% 0 0 / 0.1)",
                            (_, KbdVariant::Outlined) => "none",
                            (Theme::Dark, KbdVariant::Default) => {
                                "inset 0 1px 0 oklch(65% 0.13 250), inset 0 -1px 0 oklch(35% 0.07 250), 0 1px 2px oklch(0% 0 0 / 0.2)"
                            },
                            (Theme::Light, KbdVariant::Default) => {
                                "inset 0 1px 0 oklch(95% 0.03 250), inset 0 -1px 0 oklch(85% 0.07 250), 0 1px 2px oklch(0% 0 0 / 0.05)"
                            },
                        })
                    )
                    .apply(|el| {
                        if let Some(aria_label) = &self.aria_label {
                            el.attr("aria-label", aria_label)
                        } else {
                            el
                        }
                    })
            })
            .child(self.build_content())
    }

    fn build_content(&self) -> impl Element {
        // Check if the text contains key combinations
        let mut formatted_text = self.text.clone();

        // Handle regular + combinations (Ctrl+C, Alt+F4, etc.)
        // Mac shortcuts like ⌘K, ⌘Enter should remain as-is without spaces or plus signs
        if formatted_text.contains('+') && !formatted_text.contains('⌘') {
            formatted_text = formatted_text.replace("+", " + ");
        }

        Text::new(&formatted_text)
    }
}

// Convenience function
pub fn kbd(text: impl Into<String>) -> KbdBuilder {
    KbdBuilder::new(text)
}

// Common keyboard shortcuts
pub fn ctrl_c() -> KbdBuilder {
    kbd("Ctrl+C")
}

pub fn ctrl_v() -> KbdBuilder {
    kbd("Ctrl+V")
}

pub fn ctrl_s() -> KbdBuilder {
    kbd("Ctrl+S")
}

pub fn enter() -> KbdBuilder {
    kbd("Enter")
}

pub fn escape() -> KbdBuilder {
    kbd("Esc")
}

pub fn ctrl_z() -> KbdBuilder {
    kbd("Ctrl+Z")
}

pub fn tab() -> KbdBuilder {
    kbd("Tab")
}

pub fn shift_tab() -> KbdBuilder {
    kbd("Shift+Tab")
}

pub fn arrow_up() -> KbdBuilder {
    kbd("↑")
}

pub fn arrow_down() -> KbdBuilder {
    kbd("↓")
}

pub fn arrow_left() -> KbdBuilder {
    kbd("←")
}

pub fn arrow_right() -> KbdBuilder {
    kbd("→")
}

pub fn cmd_k() -> KbdBuilder {
    kbd("⌘K")
}

pub fn cmd_enter() -> KbdBuilder {
    kbd("⌘Enter")
}
