use zoon::*;
use crate::tokens::*;
use crate::components::icon::{IconBuilder, IconName, IconSize, IconColor};

// Switch sizes - Made larger to match Vue Storybook and accommodate icons better
#[derive(Debug, Clone, Copy)]
pub enum SwitchSize {
    Small,   // 40x24px - Increased for better icon visibility
    Medium,  // 52x28px - Increased for better proportions
    Large,   // 64x32px - Increased for better icon space
}

impl SwitchSize {
    pub fn track_width(self) -> u32 {
        match self {
            SwitchSize::Small => 49,   // Updated to 49px
            SwitchSize::Medium => 52,  // +8px wider
            SwitchSize::Large => 64,   // +12px wider
        }
    }

    pub fn track_height(self) -> u32 {
        match self {
            SwitchSize::Small => 24,   // +4px taller
            SwitchSize::Medium => 28,  // +4px taller
            SwitchSize::Large => 32,   // +4px taller
        }
    }

    pub fn thumb_size(self) -> u32 {
        match self {
            SwitchSize::Small => 20,   // +4px larger
            SwitchSize::Medium => 24,  // +4px larger
            SwitchSize::Large => 28,   // +4px larger
        }
    }

    pub fn font_size(self) -> u32 {
        match self {
            SwitchSize::Small => FONT_SIZE_12,
            SwitchSize::Medium => FONT_SIZE_12,
            SwitchSize::Large => FONT_SIZE_14,
        }
    }
}

// Label position
#[derive(Debug, Clone, Copy)]
pub enum LabelPosition {
    Left,
    Right,
}

// Switch builder
pub struct SwitchBuilder {
    size: SwitchSize,
    checked: bool,
    disabled: bool,
    label: Option<String>,
    description: Option<String>,
    show_icons: bool,
    required: bool,
    checked_icon: IconName,
    unchecked_icon: IconName,
    thumb_icon: Option<IconName>,
    label_position: LabelPosition,
}

impl SwitchBuilder {
    pub fn new() -> Self {
        Self {
            size: SwitchSize::Medium,
            checked: false,
            disabled: false,
            label: None,
            description: None,
            show_icons: true,
            required: false,
            checked_icon: IconName::Eye,
            unchecked_icon: IconName::EyeOff,
            thumb_icon: None,
            label_position: LabelPosition::Right,
        }
    }

    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
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

    pub fn show_icons(mut self, show_icons: bool) -> Self {
        self.show_icons = show_icons;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn icons(mut self, checked_icon: IconName, unchecked_icon: IconName) -> Self {
        self.checked_icon = checked_icon;
        self.unchecked_icon = unchecked_icon;
        self
    }

    pub fn thumb_icon(mut self, thumb_icon: IconName) -> Self {
        self.thumb_icon = Some(thumb_icon);
        self
    }

    pub fn label_position(mut self, position: LabelPosition) -> Self {
        self.label_position = position;
        self
    }

    pub fn build(self) -> impl Element {
        let checked = Mutable::new(self.checked);
        let checked_signal = checked.signal();
        let checked_signal_clone = checked.signal();
        let checked_signal_clone2 = checked.signal();
        let checked_clone = checked.clone();
        let checked_clone_for_label = checked.clone();
        let focused = Mutable::new(false);
        let hovered = Mutable::new(false);
        let focused_signal = focused.signal();
        let hovered_signal = hovered.signal();

        let track_width = self.size.track_width();
        let track_height = self.size.track_height();
        let thumb_size = self.size.thumb_size();
        let font_size = self.size.font_size();
        let track_radius = track_height / 2;
        let thumb_radius = thumb_size / 2;
        let disabled = self.disabled;
        let show_icons = self.show_icons;
        let checked_icon = self.checked_icon;
        let unchecked_icon = self.unchecked_icon;
        let thumb_icon = self.thumb_icon;

        // Calculate thumb position (2px padding from edges)
        let thumb_padding = 2;
        let thumb_travel = track_width - thumb_size - (thumb_padding * 2);

        // Create the switch track
        let switch_track = El::new()
            .s(Width::exact(track_width))
            .s(Height::exact(track_height))
            .s(RoundedCorners::all(track_radius))
            .s(Cursor::new(if disabled {
                CursorIcon::NotAllowed
            } else {
                CursorIcon::Pointer
            }))
            // Make focusable for keyboard navigation
            .update_raw_el(|raw_el| raw_el.attr("tabindex", if disabled { "-1" } else { "0" }))
            .update_raw_el(|raw_el| raw_el.style("outline", "none")) // Remove default browser focus outline
            // Add proper focus event handlers for keyboard navigation
            .update_raw_el({
                let focused = focused.clone();
                move |raw_el| {
                    raw_el.event_handler(move |_: events::Focus| {
                        if !disabled {
                            focused.set(true);
                        }
                    })
                }
            })
            .update_raw_el({
                let focused = focused.clone();
                move |raw_el| {
                    raw_el.event_handler(move |_: events::Blur| {
                        focused.set(false);
                    })
                }
            })
            // Add hover event handlers for focus ring on hover
            .update_raw_el({
                let hovered = hovered.clone();
                move |raw_el| {
                    raw_el.event_handler(move |_: events::MouseEnter| {
                        if !disabled {
                            hovered.set(true);
                        }
                    })
                }
            })
            .update_raw_el({
                let hovered = hovered.clone();
                move |raw_el| {
                    raw_el.event_handler(move |_: events::MouseLeave| {
                        hovered.set(false);
                    })
                }
            })
            // Add keyboard support (Space and Enter to toggle)
            .update_raw_el({
                let checked_clone = checked_clone.clone();
                move |raw_el| {
                    raw_el.event_handler(move |event: events::KeyDown| {
                        if !disabled {
                            match event.key().as_str() {
                                " " | "Enter" => {
                                    event.prevent_default();
                                    checked_clone.update(|current| !current);
                                }
                                _ => {}
                            }
                        }
                    })
                }
            })
            .s(Background::new().color_signal(
                map_ref! {
                    let theme = theme(),
                    let checked_state = checked_signal_clone =>
                    if disabled {
                        match *theme {
                            Theme::Light => "oklch(85% 0.14 250)", // neutral_4 light
                            Theme::Dark => "oklch(25% 0.14 250)", // neutral_4 dark
                        }
                    } else if *checked_state {
                        match *theme {
                            Theme::Light => "oklch(55% 0.22 250)", // primary_7 light
                            Theme::Dark => "oklch(65% 0.22 250)", // primary_7 dark
                        }
                    } else {
                        match *theme {
                            Theme::Light => "oklch(75% 0.14 250)", // neutral_5 light
                            Theme::Dark => "oklch(35% 0.14 250)", // neutral_5 dark
                        }
                    }
                }
            ))
            // Add subtle border for definition
            .s(Borders::all_signal(theme().map(move |t| {
                if disabled {
                    Border::new().width(1).color("transparent")
                } else {
                    match t {
                        Theme::Light => Border::new().width(1).color("oklch(85% 0.14 250)"), // neutral_4 light
                        Theme::Dark => Border::new().width(1).color("oklch(45% 0.14 250)"), // neutral_6 dark
                    }
                }
            })))
            // Add box shadows for depth and focus ring
            .s(Shadows::with_signal(
                map_ref! {
                    let theme = theme(),
                    let focused_state = focused.signal(),
                    let hovered_state = hovered.signal(),
                    let checked_state = checked_signal_clone2 => {
                        let mut shadows = Vec::new();

                        // Base shadow for depth
                        if !disabled {
                            match *theme {
                                Theme::Light => {
                                    shadows.push(Shadow::new().y(1).blur(2).color("rgba(0, 0, 0, 0.05)"));
                                    shadows.push(Shadow::new().y(1).blur(1).color("rgba(0, 0, 0, 0.06)"));
                                },
                                Theme::Dark => {
                                    shadows.push(Shadow::new().y(1).blur(3).color("rgba(0, 0, 0, 0.3)"));
                                    shadows.push(Shadow::new().y(1).blur(2).color("rgba(0, 0, 0, 0.2)"));
                                }
                            }
                        }

                        // Focus ring with offset - Show on both focus and hover
                        if (*focused_state || *hovered_state) && !disabled {
                            let focus_color = match *theme {
                                Theme::Light => "oklch(55% 0.22 250 / 0.6)", // primary_7 with higher opacity
                                Theme::Dark => "oklch(65% 0.22 250 / 0.6)", // primary_7 with higher opacity
                            };
                            // Add multiple shadow layers for better visibility
                            shadows.push(Shadow::new().blur(0).spread(2).color(focus_color));
                            shadows.push(Shadow::new().blur(4).spread(1).color(focus_color));
                        }

                        // Inner glow for checked state
                        if *checked_state && !disabled {
                            match *theme {
                                Theme::Light => {
                                    shadows.push(Shadow::new().inner().blur(4).color("rgba(255, 255, 255, 0.1)"));
                                },
                                Theme::Dark => {
                                    shadows.push(Shadow::new().inner().blur(4).color("rgba(255, 255, 255, 0.05)"));
                                }
                            }
                        }

                        shadows
                    }
                }
            ))
            // Add smooth transitions
            .s(Transitions::new([
                Transition::property("background-color").duration(300),
                Transition::property("border-color").duration(300),
                Transition::property("box-shadow").duration(300),
            ]))
            .child(
                // Combined track with icons and thumb
                Row::new()
                    .s(Width::fill())
                    .s(Height::fill())
                    .s(Align::new().center_y())
                    .s(Padding::all(thumb_padding))
                    .item_signal(checked_signal.map(move |is_checked| {
                        if is_checked {
                            // Thumb on the right
                            Row::new()
                                .s(Width::fill())
                                .s(Align::new().center_y())
                                .item_signal(always(show_icons).map(move |show| {
                                    if show {
                                        Some(
                                            El::new()
                                                .s(Padding::new().x(4)) // Add horizontal padding for track icon
                                                .child(
                                                    IconBuilder::new(checked_icon)
                                                        .size(IconSize::Small)
                                                        .color(if disabled {
                                                            IconColor::Muted
                                                        } else {
                                                            IconColor::Custom("oklch(98% 0.14 250)")
                                                        })
                                                        .build()
                                                )
                                        )
                                    } else {
                                        None
                                    }
                                }))
                                .item(El::new().s(Width::fill())) // Spacer
                                .item(
                                    El::new()
                                        .s(Width::exact(thumb_size))
                                        .s(Height::exact(thumb_size))
                                        .s(RoundedCorners::all(thumb_radius))
                                        .s(Align::center())
                                        .s(Background::new().color_signal(theme().map(move |t| {
                                            if disabled {
                                                match t {
                                                    Theme::Light => "oklch(65% 0.14 250)", // neutral_6 light
                                                    Theme::Dark => "oklch(45% 0.14 250)", // neutral_6 dark
                                                }
                                            } else {
                                                match t {
                                                    Theme::Light => "oklch(98% 0.01 250)", // primary_1 light
                                                    Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                }
                                            }
                                        })))
                                        // Add thumb shadow for depth
                                        .s(Shadows::with_signal(theme().map(move |t| {
                                            if disabled {
                                                vec![]
                                            } else {
                                                match t {
                                                    Theme::Light => vec![
                                                        Shadow::new().y(1).blur(2).color("rgba(0, 0, 0, 0.1)"),
                                                        Shadow::new().y(1).blur(1).color("rgba(0, 0, 0, 0.06)"),
                                                    ],
                                                    Theme::Dark => vec![
                                                        Shadow::new().y(1).blur(3).color("rgba(0, 0, 0, 0.4)"),
                                                        Shadow::new().y(1).blur(2).color("rgba(0, 0, 0, 0.3)"),
                                                    ]
                                                }
                                            }
                                        })))
                                        // Add smooth transitions for thumb
                                        .s(Transitions::new([
                                            Transition::property("background-color").duration(300),
                                            Transition::property("box-shadow").duration(300),
                                            Transition::property("transform").duration(300),
                                        ]))
                                        .child_signal(always(thumb_icon).map(move |icon| {
                                            icon.map(|icon_name| {
                                                IconBuilder::new(icon_name)
                                                    .size(IconSize::Small)
                                                    .color(if disabled {
                                                        IconColor::Muted
                                                    } else {
                                                        // Use darker color for better contrast against light thumb
                                                        IconColor::Custom("oklch(35% 0.14 250)") // neutral_7 for good contrast
                                                    })
                                                    .build()
                                            })
                                        }))
                                )
                        } else {
                            // Thumb on the left
                            Row::new()
                                .s(Width::fill())
                                .s(Align::new().center_y())
                                .item(
                                    El::new()
                                        .s(Width::exact(thumb_size))
                                        .s(Height::exact(thumb_size))
                                        .s(RoundedCorners::all(thumb_radius))
                                        .s(Align::center())
                                        .s(Background::new().color_signal(theme().map(move |t| {
                                            if disabled {
                                                match t {
                                                    Theme::Light => "oklch(65% 0.14 250)", // neutral_6 light
                                                    Theme::Dark => "oklch(45% 0.14 250)", // neutral_6 dark
                                                }
                                            } else {
                                                match t {
                                                    Theme::Light => "oklch(98% 0.01 250)", // primary_1 light
                                                    Theme::Dark => "oklch(95% 0.14 250)", // neutral_11 dark
                                                }
                                            }
                                        })))
                                        // Add thumb shadow for depth
                                        .s(Shadows::with_signal(theme().map(move |t| {
                                            if disabled {
                                                vec![]
                                            } else {
                                                match t {
                                                    Theme::Light => vec![
                                                        Shadow::new().y(1).blur(2).color("rgba(0, 0, 0, 0.1)"),
                                                        Shadow::new().y(1).blur(1).color("rgba(0, 0, 0, 0.06)"),
                                                    ],
                                                    Theme::Dark => vec![
                                                        Shadow::new().y(1).blur(3).color("rgba(0, 0, 0, 0.4)"),
                                                        Shadow::new().y(1).blur(2).color("rgba(0, 0, 0, 0.3)"),
                                                    ]
                                                }
                                            }
                                        })))
                                        // Add smooth transitions for thumb
                                        .s(Transitions::new([
                                            Transition::property("background-color").duration(300),
                                            Transition::property("box-shadow").duration(300),
                                            Transition::property("transform").duration(300),
                                        ]))
                                        .child_signal(always(thumb_icon).map(move |icon| {
                                            icon.map(|icon_name| {
                                                IconBuilder::new(icon_name)
                                                    .size(IconSize::Small)
                                                    .color(if disabled {
                                                        IconColor::Muted
                                                    } else {
                                                        // Use darker color for better contrast against light thumb
                                                        IconColor::Custom("oklch(35% 0.14 250)") // neutral_7 for good contrast
                                                    })
                                                    .build()
                                            })
                                        }))
                                )
                                .item(El::new().s(Width::fill())) // Spacer
                                .item_signal(always(show_icons).map(move |show| {
                                    if show {
                                        Some(
                                            El::new()
                                                .s(Padding::new().x(4)) // Add horizontal padding for track icon
                                                .child(
                                                    IconBuilder::new(unchecked_icon)
                                                        .size(IconSize::Small)
                                                        .color(if disabled {
                                                            IconColor::Muted
                                                        } else {
                                                            IconColor::Secondary
                                                        })
                                                        .build()
                                                )
                                        )
                                    } else {
                                        None
                                    }
                                }))
                        }
                    }))
            )
            .on_click({
                let checked_clone = checked_clone.clone();
                move || {
                    if !disabled {
                        checked_clone.update(|current| !current);
                    }
                }
            });

        // Build the complete component
        if let Some(label_text) = &self.label {
            let mut items = Vec::new();

            // Create the label column
            let label_column = Column::new()
                .s(Gap::new().y(SPACING_2))
                .item(
                    Row::new()
                        .s(Gap::new().x(SPACING_4))
                        .item(
                            El::new()
                                .child(Text::new(label_text))
                                .s(Font::new()
                                    .size(FONT_SIZE_16)
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
                                // Make label clickable to toggle switch
                                .s(Cursor::new(if disabled {
                                    CursorIcon::NotAllowed
                                } else {
                                    CursorIcon::Pointer
                                }))
                                .on_click({
                                    let checked_clone_for_label = checked_clone_for_label.clone();
                                    move || {
                                        if !disabled {
                                            checked_clone_for_label.update(|current| !current);
                                        }
                                    }
                                })
                        )
                        .item_signal(always(self.required).map(|required| {
                            if required {
                                Some(
                                    El::new()
                                        .child(Text::new("*"))
                                        .s(Font::new()
                                            .size(FONT_SIZE_16)
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
                .item_signal(always(self.description.clone()).map({
                    let checked_clone_for_desc = checked_clone_for_label.clone();
                    move |desc| {
                        desc.map(|description| {
                            El::new()
                                .child(Text::new(&description))
                                .s(Font::new()
                                    .size(FONT_SIZE_14)
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
                                // Make description clickable to toggle switch
                                .s(Cursor::new(if disabled {
                                    CursorIcon::NotAllowed
                                } else {
                                    CursorIcon::Pointer
                                }))
                                .on_click({
                                    let checked_clone_for_desc = checked_clone_for_desc.clone();
                                    move || {
                                        if !disabled {
                                            checked_clone_for_desc.update(|current| !current);
                                        }
                                    }
                                })
                        })
                    }
                }));

            // Main row with switch and label - order based on label_position
            let main_row = match self.label_position {
                LabelPosition::Left => {
                    Row::new()
                        .s(Gap::new().x(SPACING_12))
                        .s(Align::new().top())
                        .item(label_column)
                        .item(switch_track)
                }
                LabelPosition::Right => {
                    Row::new()
                        .s(Gap::new().x(SPACING_12))
                        .s(Align::new().top())
                        .item(switch_track)
                        .item(label_column)
                }
            };

            items.push(main_row.unify());

            Column::new()
                .s(Gap::new().y(SPACING_4))
                .items(items)
                .unify()
        } else {
            switch_track.unify()
        }
    }
}

// Convenience function
pub fn switch() -> SwitchBuilder {
    SwitchBuilder::new()
}
