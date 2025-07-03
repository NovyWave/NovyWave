// Badge Component
// Small status indicator with various styles and colors
// MIGRATED FROM VUE STORYBOOK - EXACT VISUAL PARITY

use crate::tokens::*;
use crate::components::icon::{IconBuilder, IconName, IconSize, IconColor};
use zoon::*;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BadgeVariant {
    Default,     // Neutral gray badge
    Primary,     // Primary color badge
    Secondary,   // Secondary color badge
    Success,     // Green success badge
    Warning,     // Yellow warning badge
    Error,       // Red error badge
    Outline,     // Outlined badge
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BadgeSize {
    Small,       // Compact badge
    Medium,      // Standard badge
    Large,       // Larger badge
}

pub struct BadgeBuilder {
    text: String,
    variant: BadgeVariant,
    size: BadgeSize,
    left_icon: Option<IconName>,
    right_icon: Option<IconName>,
    removable: bool,
    on_remove: Option<Rc<dyn Fn() + 'static>>,
}

impl BadgeBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BadgeVariant::Default,
            size: BadgeSize::Medium,
            left_icon: None,
            right_icon: None,
            removable: false,
            on_remove: None,
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
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

    pub fn removable(mut self) -> Self {
        self.removable = true;
        self
    }

    pub fn on_remove<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static
    {
        self.on_remove = Some(Rc::new(handler));
        self
    }

    pub fn build(self) -> impl Element {
        // Size-based styling matching Vue component exactly - badges should be pills (fully rounded)
        let (padding_x, padding_y, font_size, icon_size_px, min_height) = match self.size {
            BadgeSize::Small => (SPACING_8, SPACING_4, FONT_SIZE_12, 12, 28),   // 12px icons, 28px height to match Vue
            BadgeSize::Medium => (SPACING_12, SPACING_4, FONT_SIZE_14, 14, 32), // 14px icons, 32px height to match Vue
            BadgeSize::Large => (SPACING_16, SPACING_8, FONT_SIZE_16, 16, 48),  // 16px icons, 48px height to match Vue
        };

        // Create badge content with proper layout
        self.create_badge_element(padding_x, padding_y, font_size, icon_size_px, min_height)
    }

    fn create_badge_element(
        self,
        padding_x: u32,
        padding_y: u32,
        font_size: u32,
        icon_size_px: u32,
        min_height: u32
    ) -> impl Element {
        // Store values for use in closures
        let left_icon = self.left_icon;
        let right_icon = self.right_icon;
        let removable = self.removable;
        let text = self.text.clone();
        let variant = self.variant;

        // Get color signals and shadows before moving self
        let background_color = self.get_background_color();
        let text_color = self.get_text_color();
        let border = self.get_border();
        let shadows = self.get_shadows();

        // Extract on_remove callback
        let on_remove = self.on_remove;

        // Create badge content using item_signal for conditional items
        let content = Row::new()
            .s(Gap::new().x(SPACING_4))
            .s(Align::center())
            .item_signal(always(left_icon).map(move |icon| {
                icon.map(|i| {
                    // Create custom icon with exact pixel size to match Vue Storybook
                    create_badge_icon(i, icon_size_px)
                })
            }))
            .item(Text::new(&text))
            .item_signal(always(right_icon).map(move |icon| {
                icon.map(|i| {
                    // Create custom icon with exact pixel size to match Vue Storybook
                    create_badge_icon(i, icon_size_px)
                })
            }))
            .item_signal(always(removable).map(move |is_removable| {
                if is_removable {
                    // Create a shared callback that can be moved into the closure
                    let callback = on_remove.as_ref().map(|cb| Rc::clone(cb));

                    Some(El::new()
                        .s(Cursor::new(CursorIcon::Pointer))
                        .s(Width::exact(16))
                        .s(Height::exact(16))
                        .s(RoundedCorners::all(50))
                        .s(Align::center()) // Center the SVG icon
                        .s(transition_colors())
                        .s(Background::new().color_signal(
                            theme().map(|t| match t {
                                Theme::Light => "oklch(0% 0% 0 / 0.1)",
                                Theme::Dark => "oklch(100% 0% 0 / 0.1)",
                            })
                        ))
                        .child(
                            IconBuilder::new(IconName::X)
                                .size(IconSize::Small)
                                .color(IconColor::Current)
                                .build()
                        )
                        .on_click(move || {
                            if let Some(cb) = &callback {
                                cb();
                            }
                        }))
                } else {
                    None
                }
            }));

        // Wrap in badge container with pill shape (fully rounded corners)
        El::new()
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(Height::default().min(min_height)) // Use default height with min-height like Vue
            .s(RoundedCorners::all_max()) // Make badges pill-shaped
            .s(Align::center())
            .s(Font::new()
                .size(font_size)
                .weight(FontWeight::SemiBold) // Use 600 weight like Vue (typography.weight['6'])
                .line_height((font_size as f32 * 1.2) as u32) // Use 120% line height like Vue
                .tracking(1) // Add letter spacing like Vue (0.025em ≈ 1px for these font sizes)
            )
            .s(transition_colors())
            .s(Background::new().color_signal(background_color))
            .s(Font::new().color_signal(text_color))
            .s(Borders::all_signal(border))
            .s(Shadows::with_signal(shadows)) // Add shadows matching Vue Storybook
            .child(content)
    }





    // Theme-aware color methods with improved contrast (simulating gradient effect)
    fn get_background_color(&self) -> impl Signal<Item = &'static str> + use<> {
        let variant = self.variant;
        theme().map(move |t| match (variant, t) {
            // Primary: Light theme uses much darker shade, dark theme uses lighter shade for better contrast
            (BadgeVariant::Primary, Theme::Light) => "oklch(55% 0.13 250)",  // primary.7 (much darker for better contrast)
            (BadgeVariant::Primary, Theme::Dark) => "oklch(65% 0.13 250)",   // primary.8 (lighter for better contrast)

            // Secondary: Light theme uses darker neutral, dark theme uses darker neutral for distinction
            (BadgeVariant::Secondary, Theme::Light) => "oklch(65% 0.09 255)", // neutral.6 (darker for better contrast, darker than Default)
            (BadgeVariant::Secondary, Theme::Dark) => "oklch(75% 0.07 255)",  // neutral.7 (darker than Default for visual hierarchy)

            // Success: Light theme uses darker green, dark theme uses lighter green
            (BadgeVariant::Success, Theme::Light) => "oklch(50% 0.16 140)",   // success.7 (darker for contrast)
            (BadgeVariant::Success, Theme::Dark) => "oklch(70% 0.16 140)",    // success.7 (lighter for contrast)

            // Warning: Light theme uses darker amber, dark theme uses lighter amber
            (BadgeVariant::Warning, Theme::Light) => "oklch(65% 0.19 85)",    // warning.7 (darker for contrast)
            (BadgeVariant::Warning, Theme::Dark) => "oklch(65% 0.19 85)",     // warning.7 (lighter for contrast)

            // Error: Light theme uses darker red, dark theme uses lighter red
            (BadgeVariant::Error, Theme::Light) => "oklch(55% 0.18 30)",      // error.7 (darker for contrast)
            (BadgeVariant::Error, Theme::Dark) => "oklch(75% 0.18 30)",       // error.7 (lighter for contrast)

            // Default: Light theme uses darker neutral (more subtle than Secondary), dark theme uses lighter neutral
            (BadgeVariant::Default, Theme::Light) => "oklch(70% 0.07 255)",   // neutral.6 (darker for better contrast, but lighter than Secondary)
            (BadgeVariant::Default, Theme::Dark) => "oklch(85% 0.025 255)",   // neutral.11 (closest to Vue gradient average: oklch(84% 0.10 255))

            (BadgeVariant::Outline, _) => "transparent",
        })
    }

    fn get_text_color(&self) -> impl Signal<Item = &'static str> + use<> {
        let variant = self.variant;
        theme().map(move |t| match (variant, t) {
            // In light theme: ALL colored badges use white text on colored backgrounds
            // In dark theme: ALL badges use dark text for better readability
            (BadgeVariant::Primary, Theme::Light) => "oklch(99% 0.025 255)",    // neutral.1 (white)
            (BadgeVariant::Primary, Theme::Dark) => "oklch(25% 0.025 255)",     // neutral.11 (dark text)

            (BadgeVariant::Secondary, Theme::Light) => "oklch(99% 0.025 255)",  // neutral.1 (white)
            (BadgeVariant::Secondary, Theme::Dark) => "oklch(25% 0.025 255)",   // neutral.11 (dark text)

            (BadgeVariant::Success, Theme::Light) => "oklch(99% 0.025 255)",    // neutral.1 (white)
            (BadgeVariant::Success, Theme::Dark) => "oklch(25% 0.025 255)",     // neutral.11 (dark text)

            (BadgeVariant::Warning, Theme::Light) => "oklch(99% 0.025 255)",    // neutral.1 (white)
            (BadgeVariant::Warning, Theme::Dark) => "oklch(25% 0.025 255)",     // neutral.11 (dark text)

            (BadgeVariant::Error, Theme::Light) => "oklch(99% 0.025 255)",      // neutral.1 (white)
            (BadgeVariant::Error, Theme::Dark) => "oklch(25% 0.025 255)",       // neutral.11 (dark text)

            (BadgeVariant::Default, Theme::Light) => "oklch(99% 0.025 255)",    // neutral.1 (white)
            (BadgeVariant::Default, Theme::Dark) => "oklch(25% 0.025 255)",     // neutral.11 (dark text)

            // Outline badge uses theme-appropriate text colors
            (BadgeVariant::Outline, Theme::Light) => "oklch(25% 0.025 255)",  // neutral.11 (dark text)
            (BadgeVariant::Outline, Theme::Dark) => "oklch(85% 0.025 255)",   // neutral.11 dark (light text)
        })
    }

    fn get_border(&self) -> impl Signal<Item = Border> + use<> {
        let variant = self.variant;
        theme().map(move |t| {
            if variant == BadgeVariant::Outline {
                let color = match t {
                    Theme::Light => "oklch(70% 0.09 255)",  // neutral.6
                    Theme::Dark => "oklch(70% 0.09 255)",   // neutral.6
                };
                Border::new().width(1).color(color)
            } else {
                Border::new().width(0).color("transparent")
            }
        })
    }

    fn get_shadows(&self) -> impl Signal<Item = Vec<Shadow>> + use<> {
        let variant = self.variant;
        theme().map(move |t| {
            match (variant, t) {
                // Primary badge shadows - blue color (59, 130, 246 = rgb for primary)
                (BadgeVariant::Primary, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(59, 130, 246, 0.25)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(59, 130, 246, 0.15)"),
                ],
                (BadgeVariant::Primary, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(59, 130, 246, 0.3)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(59, 130, 246, 0.2)"),
                ],

                // Secondary badge shadows - neutral black
                (BadgeVariant::Secondary, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(0, 0, 0, 0.15)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(0, 0, 0, 0.1)"),
                ],
                (BadgeVariant::Secondary, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(0, 0, 0, 0.4)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(0, 0, 0, 0.3)"),
                ],

                // Success badge shadows - green color (34, 197, 94 = rgb for success)
                (BadgeVariant::Success, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(34, 197, 94, 0.25)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(34, 197, 94, 0.15)"),
                ],
                (BadgeVariant::Success, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(34, 197, 94, 0.3)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(34, 197, 94, 0.2)"),
                ],

                // Warning badge shadows - amber color (245, 158, 11 = rgb for warning)
                (BadgeVariant::Warning, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(245, 158, 11, 0.25)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(245, 158, 11, 0.15)"),
                ],
                (BadgeVariant::Warning, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(245, 158, 11, 0.3)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(245, 158, 11, 0.2)"),
                ],

                // Error badge shadows - red color (239, 68, 68 = rgb for error)
                (BadgeVariant::Error, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(239, 68, 68, 0.25)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(239, 68, 68, 0.15)"),
                ],
                (BadgeVariant::Error, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(239, 68, 68, 0.3)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(239, 68, 68, 0.2)"),
                ],

                // Default badge shadows - neutral black (same as Secondary)
                (BadgeVariant::Default, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(0, 0, 0, 0.15)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(0, 0, 0, 0.1)"),
                ],
                (BadgeVariant::Default, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color("rgba(0, 0, 0, 0.4)"),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(0, 0, 0, 0.3)"),
                ],

                // Outline badge shadows - lighter shadows matching Vue Storybook
                (BadgeVariant::Outline, Theme::Light) => vec![
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(0, 0, 0, 0.1)"),
                    Shadow::new().y(1).x(0).blur(2).spread(-1).color("rgba(0, 0, 0, 0.05)"),
                ],
                (BadgeVariant::Outline, Theme::Dark) => vec![
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color("rgba(0, 0, 0, 0.2)"),
                    Shadow::new().y(1).x(0).blur(2).spread(-1).color("rgba(0, 0, 0, 0.1)"),
                ],
            }
        })
    }
}

// Custom icon creation function for badges with exact pixel sizes
fn create_badge_icon(icon_name: IconName, size_px: u32) -> impl Element {
    // Create SVG icon element with custom size and currentColor - no extra alignment
    RawHtmlEl::new("div")
        .style("color", "currentColor")
        .style("width", &format!("{}px", size_px))
        .style("height", &format!("{}px", size_px))
        .style("display", "flex")
        .style("align-items", "center")
        .style("justify-content", "center")
        .inner_markup(&get_svg_content_for_badge(icon_name, size_px))
        .into_element()
}

// Get inline SVG content for badge icons with proper sizing
fn get_svg_content_for_badge(name: IconName, size_px: u32) -> String {
    let svg_template = match name {
        IconName::Check => include_str!("../../assets/icons/check.svg"),
        IconName::X => include_str!("../../assets/icons/x.svg"),
        IconName::TriangleAlert => include_str!("../../assets/icons/triangle-alert.svg"),
        IconName::CircleAlert => include_str!("../../assets/icons/circle-alert.svg"),
        IconName::CircleCheck => include_str!("../../assets/icons/circle-check.svg"),
        IconName::Info => include_str!("../../assets/icons/info.svg"),
        IconName::Star => include_str!("../../assets/icons/star.svg"),
        IconName::Heart => include_str!("../../assets/icons/heart.svg"),
        IconName::Tag => include_str!("../../assets/icons/tag.svg"),
        IconName::User => include_str!("../../assets/icons/user.svg"),
        IconName::Settings => include_str!("../../assets/icons/settings.svg"),
        // Add more icons as needed for badges
        _ => include_str!("../../assets/icons/circle-help.svg"), // Default fallback
    };

    // Process the SVG to set proper size and ensure currentColor works
    process_svg_for_badge(svg_template, size_px)
}

// Process SVG content for badge icons
fn process_svg_for_badge(svg_content: &str, size_px: u32) -> String {
    let mut processed = svg_content.to_string();

    // Replace width and height attributes with the desired size
    processed = processed.replace("width=\"24\"", &format!("width=\"{}\"", size_px));
    processed = processed.replace("height=\"24\"", &format!("height=\"{}\"", size_px));

    // Ensure stroke="currentColor" is preserved for proper color inheritance
    processed
}

// Convenience functions
pub fn badge(text: impl Into<String>) -> BadgeBuilder {
    BadgeBuilder::new(text)
}
