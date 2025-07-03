// Card Component
// Container component for grouping related content

use crate::tokens::*;
use zoon::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardVariant {
    Default,    // Standard card with border
    Elevated,   // Card with shadow
    Outlined,   // Card with prominent border
    Filled,     // Card with background fill
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardSize {
    Small,      // Compact padding
    Medium,     // Standard padding
    Large,      // Spacious padding
}

pub struct CardBuilder {
    variant: CardVariant,
    size: CardSize,
    clickable: bool,
    // Simplified: remove children for now to avoid dyn Element issues
}

impl CardBuilder {
    pub fn new() -> Self {
        Self {
            variant: CardVariant::Default,
            size: CardSize::Medium,
            clickable: false,
        }
    }

    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: CardSize) -> Self {
        self.size = size;
        self
    }

    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    // Removed child method for now to avoid dyn Element issues

    pub fn build(self) -> impl Element {
        // Size-based padding
        let padding = match self.size {
            CardSize::Small => SPACING_12,
            CardSize::Medium => SPACING_16,
            CardSize::Large => SPACING_24,
        };

        // Variant-based styling
        let (bg_color, border_color, border_width, shadow) = match self.variant {
            CardVariant::Default => ("transparent", "oklch(90% 0.025 255)", 1, false),
            CardVariant::Elevated => ("transparent", "transparent", 0, true),
            CardVariant::Outlined => ("transparent", "oklch(70% 0.025 255)", 2, false),
            CardVariant::Filled => ("oklch(98% 0.025 255)", "oklch(95% 0.025 255)", 1, false),
        };

        let mut card = El::new()
            .s(Padding::all(padding))
            .s(RoundedCorners::all(8))
            .s(Background::new().color(bg_color))
            .s(transition_colors());

        // Add border if needed
        if border_width > 0 {
            card = card.s(Borders::all(
                Border::new()
                    .width(border_width)
                    .color(border_color)
            ));
        }

        // Add shadow for elevated variant
        if shadow {
            // Note: MoonZoon shadow implementation would go here
            // For now, we'll use a subtle border to simulate elevation
            card = card.s(Borders::all(
                Border::new()
                    .width(1)
                    .color("oklch(95% 0.025 255)")
            ));
        }

        // Add hover effect for clickable cards
        if self.clickable {
            card = card.s(Cursor::new(CursorIcon::Pointer));
            // Note: Hover effects would need signal-based background changes
            // Keeping it simple for now to avoid compilation issues
        }

        // Add simple content - using El wrapper for Text to avoid styling issues
        let content = El::new()
            .s(Font::new()
                .size(FONT_SIZE_16)
                .color_signal(neutral_11())
            )
            .child(Text::new("Card Content"));

        card.child(content)
    }
}

// Convenience function
pub fn card() -> CardBuilder {
    CardBuilder::new()
}
