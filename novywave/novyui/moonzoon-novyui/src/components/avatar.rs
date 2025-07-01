use zoon::*;
use crate::tokens::*;

// Avatar sizes
#[derive(Debug, Clone, Copy)]
pub enum AvatarSize {
    Small,   // 24px
    Medium,  // 32px
    Large,   // 40px
    XLarge,  // 48px
}

impl AvatarSize {
    pub fn to_px(self) -> u32 {
        match self {
            AvatarSize::Small => 24,
            AvatarSize::Medium => 32,
            AvatarSize::Large => 40,
            AvatarSize::XLarge => 48,
        }
    }
}

// Avatar variants
#[derive(Debug, Clone, Copy)]
pub enum AvatarVariant {
    Initials,   // Show initials
    Image,      // Show image (simplified for now)
    Icon,       // Show icon
}

// Avatar builder
pub struct AvatarBuilder {
    variant: AvatarVariant,
    size: AvatarSize,
    text: String,
    image_url: Option<String>,
    icon: Option<&'static str>,
}

impl AvatarBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            variant: AvatarVariant::Initials,
            size: AvatarSize::Medium,
            text: text.into(),
            image_url: None,
            icon: None,
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image_url = Some(url.into());
        self.variant = AvatarVariant::Image;
        self
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self.variant = AvatarVariant::Icon;
        self
    }

    pub fn build(self) -> impl Element {
        let size_px = self.size.to_px();
        let radius = size_px / 2;

        let content = match self.variant {
            AvatarVariant::Initials => {
                // Extract initials from text
                let initials = self.text
                    .split_whitespace()
                    .take(2)
                    .map(|word| word.chars().next().unwrap_or('?'))
                    .collect::<String>()
                    .to_uppercase();

                El::new()
                    .s(Font::new()
                        .size(match self.size {
                            AvatarSize::Small => FONT_SIZE_12,
                            AvatarSize::Medium => FONT_SIZE_14,
                            AvatarSize::Large => FONT_SIZE_16,
                            AvatarSize::XLarge => FONT_SIZE_18,
                        })
                        .color_signal(neutral_11())
                        .weight(FontWeight::Medium)
                    )
                    .s(Align::center())
                    .child(Text::new(&initials))
            },
            AvatarVariant::Image => {
                // For now, just show initials as placeholder
                // In a real implementation, you'd use an img element
                let initials = self.text
                    .split_whitespace()
                    .take(2)
                    .map(|word| word.chars().next().unwrap_or('?'))
                    .collect::<String>()
                    .to_uppercase();

                El::new()
                    .s(Font::new()
                        .size(match self.size {
                            AvatarSize::Small => FONT_SIZE_12,
                            AvatarSize::Medium => FONT_SIZE_14,
                            AvatarSize::Large => FONT_SIZE_16,
                            AvatarSize::XLarge => FONT_SIZE_18,
                        })
                        .color_signal(neutral_11())
                        .weight(FontWeight::Medium)
                    )
                    .s(Align::center())
                    .child(Text::new(&initials))
            },
            AvatarVariant::Icon => {
                El::new()
                    .s(Font::new()
                        .size(match self.size {
                            AvatarSize::Small => FONT_SIZE_14,
                            AvatarSize::Medium => FONT_SIZE_16,
                            AvatarSize::Large => FONT_SIZE_18,
                            AvatarSize::XLarge => FONT_SIZE_20,
                        })
                        .color_signal(neutral_9())
                    )
                    .s(Align::center())
                    .child(Text::new(self.icon.unwrap_or("👤")))
            }
        };

        El::new()
            .s(Width::exact(size_px))
            .s(Height::exact(size_px))
            .s(RoundedCorners::all(radius))
            .s(Background::new().color_signal(primary_1()))
            .s(Align::center())
            .child(content)
    }
}

// Convenience function
pub fn avatar(text: impl Into<String>) -> AvatarBuilder {
    AvatarBuilder::new(text)
}
