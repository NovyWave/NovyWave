// Button Component
// Research-validated pattern with MoonZoon styling and animation

use crate::tokens::*;
use crate::tokens::shadow::*;
use crate::components::icon::*;
use zoon::*;
use futures_signals::signal::{always, SignalExt};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Outline,
    Ghost,
    Link,
    Destructive,
    DestructiveGhost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

pub struct ButtonBuilder {
    label: Option<String>,
    label_signal: Option<Box<dyn Signal<Item = String> + Unpin>>,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    disabled_signal: Option<Box<dyn Signal<Item = bool> + Unpin>>,
    loading: bool,
    left_icon: Option<&'static str>,
    left_icon_element: Option<Box<dyn Fn() -> RawElOrText>>,
    right_icon: Option<&'static str>,
    right_icon_element: Option<Box<dyn Fn() -> RawElOrText>>,
    left_icon_aria_label: Option<String>,
    right_icon_aria_label: Option<String>,
    on_press: Option<Box<dyn Fn()>>,
    min_width: Option<u32>,
    align: Option<Align>,
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            label_signal: None,
            variant: ButtonVariant::Primary,
            size: ButtonSize::Medium,
            disabled: false,
            disabled_signal: None,
            loading: false,
            left_icon: None,
            left_icon_element: None,
            right_icon: None,
            right_icon_element: None,
            left_icon_aria_label: None,
            right_icon_aria_label: None,
            on_press: None,
            min_width: None,
            align: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.label_signal = None;
        self
    }

    pub fn label_signal<S>(mut self, label_signal: S) -> Self 
    where
        S: Signal<Item = String> + Unpin + 'static,
    {
        self.label_signal = Some(Box::new(label_signal));
        self.label = None;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.disabled_signal = None;
        self
    }

    pub fn disabled_signal<S>(mut self, disabled_signal: S) -> Self 
    where
        S: Signal<Item = bool> + Unpin + 'static,
    {
        self.disabled_signal = Some(Box::new(disabled_signal));
        self.disabled = false;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn left_icon(mut self, icon: IconName) -> Self {
        self.left_icon = Some(icon.to_kebab_case());
        self
    }

    pub fn left_icon_element(mut self, element_fn: impl Fn() -> RawElOrText + 'static) -> Self {
        self.left_icon_element = Some(Box::new(element_fn));
        self.left_icon = None; // Clear static icon when using dynamic element
        self
    }

    pub fn right_icon(mut self, icon: IconName) -> Self {
        self.right_icon = Some(icon.to_kebab_case());
        self
    }

    pub fn right_icon_element(mut self, element_fn: impl Fn() -> RawElOrText + 'static) -> Self {
        self.right_icon_element = Some(Box::new(element_fn));
        self.right_icon = None; // Clear static icon when using dynamic element
        self
    }

    pub fn left_icon_aria_label(mut self, label: impl Into<String>) -> Self {
        self.left_icon_aria_label = Some(label.into());
        self
    }

    pub fn right_icon_aria_label(mut self, label: impl Into<String>) -> Self {
        self.right_icon_aria_label = Some(label.into());
        self
    }

    pub fn min_width(mut self, width: u32) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn on_press<F>(mut self, handler: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_press = Some(Box::new(handler));
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }


    pub fn build(self) -> impl Element {
        let (hovered, hovered_signal) = Mutable::new_and_signal(false);
        let (focused, focused_signal) = Mutable::new_and_signal(false);

        // Size-based styling
        let (padding_x, padding_y, font_size, icon_size) = match self.size {
            ButtonSize::Small => (SPACING_12, SPACING_6, FONT_SIZE_14, IconSize::Small),
            ButtonSize::Medium => (SPACING_16, SPACING_8, FONT_SIZE_16, IconSize::Medium),
            ButtonSize::Large => (SPACING_20, SPACING_12, FONT_SIZE_18, IconSize::Large),
        };

        // Determine if this is an icon-only button
        let is_icon_only = self.label.is_none() && (self.left_icon.is_some() || self.left_icon_element.is_some() || self.right_icon.is_some() || self.right_icon_element.is_some());

        // Adjust padding for icon-only buttons
        let (final_padding_x, final_padding_y) = if is_icon_only {
            (padding_y, padding_y) // Square padding for icon-only buttons
        } else {
            (padding_x, padding_y)
        };

        // Create button content based on state and icons
        self.create_button_element(
            hovered_signal,
            focused_signal,
            final_padding_x,
            final_padding_y,
            font_size,
            icon_size,
            hovered,
            focused,
        )
    }

    fn create_button_element(
        mut self,
        hovered_signal: impl Signal<Item = bool> + Unpin + 'static,
        focused_signal: impl Signal<Item = bool> + Unpin + 'static,
        padding_x: u32,
        padding_y: u32,
        font_size: u32,
        icon_size: IconSize,
        hovered: Mutable<bool>,
        focused: Mutable<bool>,
    ) -> impl Element {
        // Variant-based colors - unified signal approach
        let variant = self.variant;
        let disabled = self.disabled;
        let disabled_signal = self.disabled_signal.take();
        let loading = self.loading;

        // Create a shared mutable for disabled state
        let (is_disabled_mutable, is_disabled_signal) = Mutable::new_and_signal(disabled || loading);
        
        // If we have a disabled_signal, update the mutable when it changes
        if let Some(signal) = disabled_signal {
            Task::start(signal.for_each(clone!((is_disabled_mutable) move |signal_disabled| {
                is_disabled_mutable.set(disabled || loading || signal_disabled);
                async {}
            })));
        }
        
        // Broadcast the signal for multiple use
        let is_disabled_broadcast = is_disabled_signal.broadcast();

        let bg_color_signal = match variant {
            ButtonVariant::Primary => primary_7().boxed_local(),
            ButtonVariant::Secondary => neutral_4().boxed_local(),
            ButtonVariant::Outline => always(transparent()).boxed_local(),
            ButtonVariant::Ghost => always(transparent()).boxed_local(),
            ButtonVariant::Link => always(transparent()).boxed_local(),
            ButtonVariant::Destructive => error_7().boxed_local(),
            ButtonVariant::DestructiveGhost => always(transparent()).boxed_local(),
        };

        let hover_bg_color_signal = match variant {
            ButtonVariant::Primary => primary_8().boxed_local(),
            ButtonVariant::Secondary => neutral_5().boxed_local(),
            ButtonVariant::Outline => primary_2().boxed_local(),
            ButtonVariant::Ghost => primary_2().boxed_local(),
            ButtonVariant::Link => primary_2().boxed_local(),
            ButtonVariant::Destructive => error_8().boxed_local(),
            ButtonVariant::DestructiveGhost => error_2().boxed_local(),
        };

        let text_color_signal = match variant {
            ButtonVariant::Primary => neutral_1().boxed_local(),
            ButtonVariant::Secondary => primary_7().boxed_local(),
            ButtonVariant::Outline => primary_7().boxed_local(),
            ButtonVariant::Ghost => primary_7().boxed_local(),
            ButtonVariant::Link => primary_7().boxed_local(),
            ButtonVariant::Destructive => neutral_1().boxed_local(),
            ButtonVariant::DestructiveGhost => error_7().boxed_local(),
        };

        // Border color for Outline and Secondary variants
        let border_color_signal = match variant {
            ButtonVariant::Outline => neutral_3().boxed_local(),
            ButtonVariant::Secondary => neutral_3().boxed_local(),
            _ => always(transparent()).boxed_local(),
        };

        // Extract values before building button to avoid partial move
        let align = self.align.take();
        let on_press = self.on_press.take();
        
        // Create shadow signal using the theme signal and disabled state
        let shadows_signal = map_ref! {
            let is_disabled = is_disabled_broadcast.signal(),
            let theme = theme() =>
            if *is_disabled {
                // No shadows for disabled buttons
                vec![]
            } else {
                // Add shadows based on variant and theme
                match (variant, theme) {
                    // Primary buttons get blue-tinted shadows
                    (ButtonVariant::Primary, Theme::Light) => vec![
                        Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_PRIMARY_LIGHT),
                        Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_NEUTRAL_LIGHT),
                    ],
                    (ButtonVariant::Primary, Theme::Dark) => vec![
                        Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_PRIMARY_DARK),
                        Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_PRIMARY_LIGHT),
                    ],
                    // Secondary buttons get neutral shadows
                    (ButtonVariant::Secondary, Theme::Light) => vec![
                        Shadow::new().y(3).x(0).blur(6).spread(-1).color(SHADOW_COLOR_BLACK_MEDIUM),
                        Shadow::new().y(1).x(0).blur(3).spread(-1).color(SHADOW_COLOR_BLACK_LIGHT),
                    ],
                    (ButtonVariant::Secondary, Theme::Dark) => vec![
                        Shadow::new().y(3).x(0).blur(6).spread(-1).color(SHADOW_COLOR_BLACK_STRONG),
                        Shadow::new().y(1).x(0).blur(3).spread(-1).color(SHADOW_COLOR_BLACK_DARK),
                    ],
                    // Outline buttons get subtle shadows
                    (ButtonVariant::Outline, Theme::Light) => vec![
                        Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_BLACK_LIGHT),
                        Shadow::new().y(1).x(0).blur(2).spread(-1).color(SHADOW_COLOR_BLACK_SUBTLE),
                    ],
                    (ButtonVariant::Outline, Theme::Dark) => vec![
                        Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_BLACK_STRONG),
                        Shadow::new().y(1).x(0).blur(2).spread(-1).color(SHADOW_COLOR_BLACK_DARK),
                    ],
                    // Destructive buttons get red-tinted shadows
                    (ButtonVariant::Destructive, Theme::Light) => vec![
                        Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_ERROR_LIGHT),
                        Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_NEUTRAL_LIGHT),
                    ],
                    (ButtonVariant::Destructive, Theme::Dark) => vec![
                        Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_ERROR_DARK),
                        Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_ERROR_LIGHT),
                    ],
                    // Ghost and Link buttons get no shadows for minimal appearance
                    (ButtonVariant::Ghost, _) | (ButtonVariant::Link, _) | (ButtonVariant::DestructiveGhost, _) => vec![],
                }
            }
        }.boxed_local();
        
        // Create button content with icons and text
        let button_content = self.create_button_content(icon_size);



        let mut button = Button::new()
            .s(Padding::new().x(padding_x).y(padding_y))
            .s(RoundedCorners::all(CORNER_RADIUS_6))
            .s(Font::new().size(font_size).weight(FontWeight::Medium))
            .s(transition_colors());

        // Add align if specified
        if let Some(align) = align {
            button = button.s(align);
        }

        button
            .s(Background::new().color_signal(
                map_ref! {
                    let is_disabled = is_disabled_broadcast.signal(),
                    let hovered = hovered_signal,
                    let bg_color = bg_color_signal,
                    let hover_bg_color = hover_bg_color_signal,
                    let neutral_disabled_color = neutral_5() =>
                    if *is_disabled {
                        // Disabled state - use exact Vue colors: neutral-5
                        *neutral_disabled_color
                    } else if *hovered {
                        *hover_bg_color
                    } else {
                        *bg_color
                    }
                }.boxed_local()
            ))
            .s(Borders::all_signal(
                map_ref! {
                    let is_disabled = is_disabled_broadcast.signal(),
                    let border_color = border_color_signal,
                    let neutral_disabled_border = neutral_5() =>
                    if *is_disabled {
                        // Disabled state - use exact Vue colors: neutral-5 for border
                        Border::new().width(1).color(*neutral_disabled_border)
                    } else {
                        // Always use 1px border to prevent size changes
                        Border::new().width(1).color(*border_color)
                    }
                }.boxed_local()
            ))
            .s(Outline::with_signal_self(
                map_ref! {
                    let is_disabled = is_disabled_broadcast.signal(),
                    let focused = focused_signal =>
                    if *is_disabled {
                        // No outline for disabled buttons
                        None
                    } else if *focused {
                        Some(Outline::inner().width(FOCUS_RING_WIDTH).color(FOCUS_RING_COLOR_DEFAULT))
                    } else {
                        None
                    }
                }.boxed_local()
            ))
            .s(Font::new().color_signal(
                map_ref! {
                    let is_disabled = is_disabled_broadcast.signal(),
                    let text_color = text_color_signal,
                    let neutral_disabled_text = neutral_7() =>
                    if *is_disabled {
                        // Disabled state - use exact Vue colors: neutral-7
                        *neutral_disabled_text
                    } else {
                        *text_color
                    }
                }.boxed_local()
            ))
            .s(Cursor::with_signal(
                is_disabled_broadcast.signal().map(|is_disabled| {
                    if is_disabled { CursorIcon::NotAllowed } else { CursorIcon::Pointer }
                })
            ))
            .s(Shadows::with_signal(shadows_signal))
            .update_raw_el(move |raw_el| {
                // Add underline for Link variant
                if variant == ButtonVariant::Link {
                    raw_el.style("text-decoration", "underline")
                } else {
                    raw_el.style("text-decoration", "none")
                }
            })
            .update_raw_el(move |raw_el| {
                raw_el.style_signal("opacity", is_disabled_broadcast.signal().map(|is_disabled| {
                    if is_disabled {
                        OPACITY_DISABLED
                    } else {
                        OPACITY_ENABLED
                    }
                }))
            })
            .on_hovered_change(clone!((is_disabled_mutable, hovered) move |is_hovered| {
                if !is_disabled_mutable.get() {
                    hovered.set(is_hovered);
                }
            }))
            .on_focused_change(clone!((is_disabled_mutable, focused) move |is_focused| {
                if !is_disabled_mutable.get() {
                    focused.set(is_focused);
                }
            }))
            .label(button_content)
            .on_press(clone!((is_disabled_mutable) move || {
                if !is_disabled_mutable.get() {
                    if let Some(handler) = &on_press {
                        handler();
                    }
                }
            }))
    }

    fn create_left_icon_element(&self, icon_size: IconSize) -> RawElOrText {
        if let Some(element_fn) = &self.left_icon_element {
            element_fn()
        } else if let Some(icon_name) = self.left_icon {
            icon_str(icon_name)
                .size(icon_size)
                .color(IconColor::Current)
                .build()
                .unify()
        } else {
            panic!("create_left_icon_element called when no left icon is set")
        }
    }

    fn create_right_icon_element(&self, icon_size: IconSize) -> RawElOrText {
        if let Some(element_fn) = &self.right_icon_element {
            element_fn()
        } else if let Some(icon_name) = self.right_icon {
            icon_str(icon_name)
                .size(icon_size)
                .color(IconColor::Current)
                .build()
                .unify()
        } else {
            panic!("create_right_icon_element called when no right icon is set")
        }
    }

    fn create_button_content(mut self, icon_size: IconSize) -> RawElOrText {
        let has_label = self.label.is_some() || self.label_signal.is_some();
        let has_left_icon = self.left_icon.is_some() || self.left_icon_element.is_some();
        let has_right_icon = self.right_icon.is_some() || self.right_icon_element.is_some();
        let loading = self.loading;

        // Extract the signal to own it
        let label_signal = self.label_signal.take();
        
        // If loading, show spinner instead of normal content
        if loading {
            return self.create_loading_content(icon_size, has_label, has_left_icon, has_right_icon, label_signal);
        }
        let label = self.label.clone();
        
        // Helper to create label based on what's available
        let create_label = || {
            if let Some(ref label) = label {
                El::new()
                    .s(Font::new().no_wrap())
                    .child(Text::new(label))
                    .unify()
            } else {
                panic!("create_label called when no label is set")
            }
        };

        // Handle signal case vs static case
        if let Some(signal) = label_signal {
            // Signal-based label (can only be used once)
            let signal_label_element = El::new()
                .s(Font::new().no_wrap())
                .child_signal(signal.map(|text| Text::new(text)))
                .unify();
            
            match (has_left_icon, has_label, has_right_icon) {
                // Icon-only buttons
                (true, false, false) => {
                    self.create_left_icon_element(icon_size)
                }
                (false, false, true) => {
                    self.create_right_icon_element(icon_size)
                }
                // Both icons, no label (rare case)
                (true, false, true) => {
                    Row::new()
                        .s(Align::new().center_y())
                        .s(Gap::new().x(SPACING_8))
                        .item(
                            self.create_left_icon_element(icon_size)
                        )
                        .item(
                            self.create_right_icon_element(icon_size)
                        )
                        .unify()
                }
                // Label with left icon
                (true, true, false) => {
                    Row::new()
                        .s(Align::new().center_y())
                        .s(Gap::new().x(SPACING_8))
                        .item(
                            self.create_left_icon_element(icon_size)
                        )
                        .item(signal_label_element)
                        .unify()
                }
                // Label with right icon
                (false, true, true) => {
                    Row::new()
                        .s(Align::new().center_y())
                        .s(Gap::new().x(SPACING_8))
                        .item(signal_label_element)
                        .item(
                            self.create_right_icon_element(icon_size)
                        )
                        .unify()
                }
                // Label with both icons
                (true, true, true) => {
                    Row::new()
                        .s(Align::new().center_y())
                        .s(Gap::new().x(SPACING_8))
                        .item(
                            self.create_left_icon_element(icon_size)
                        )
                        .item(signal_label_element)
                        .item(
                            self.create_right_icon_element(icon_size)
                        )
                        .unify()
                }
                // Label only
                (false, true, false) => {
                    signal_label_element
                }
                // Empty button (fallback)
                (false, false, false) => {
                    Text::new("").unify()
                }
            }
        } else {
            // Static label case
            match (has_left_icon, has_label, has_right_icon) {
            // Icon-only buttons
            (true, false, false) => {
                self.create_left_icon_element(icon_size)
            }
            (false, false, true) => {
                self.create_right_icon_element(icon_size)
            }
            // Both icons, no label (rare case)
            (true, false, true) => {
                Row::new()
                    .s(Align::new().center_y())
                    .s(Gap::new().x(SPACING_8))
                    .item(
                        self.create_left_icon_element(icon_size)
                    )
                    .item(
                        self.create_right_icon_element(icon_size)
                    )
                    .unify()
            }
            // Label with left icon
            (true, true, false) => {
                Row::new()
                    .s(Align::new().center_y())
                    .s(Gap::new().x(SPACING_8))
                    .item(
                        self.create_left_icon_element(icon_size)
                    )
                    .item(create_label())
                    .unify()
            }
            // Label with right icon
            (false, true, true) => {
                Row::new()
                    .s(Align::new().center_y())
                    .s(Gap::new().x(SPACING_8))
                    .item(create_label())
                    .item(
                        self.create_right_icon_element(icon_size)
                    )
                    .unify()
            }
            // Label with both icons
            (true, true, true) => {
                Row::new()
                    .s(Align::new().center_y())
                    .s(Gap::new().x(SPACING_8))
                    .item(
                        self.create_left_icon_element(icon_size)
                    )
                    .item(create_label())
                    .item(
                        self.create_right_icon_element(icon_size)
                    )
                    .unify()
            }
            // Label only
            (false, true, false) => {
                create_label()
            }
            // Empty button (fallback)
            (false, false, false) => {
                Text::new("").unify()
            }
            }
        }
    }

    #[allow(dead_code)]
    fn get_button_shadows_signal(&self, variant: ButtonVariant) -> impl Signal<Item = Vec<Shadow>> + use<> {
        theme().map(move |t| {
            match (variant, t) {
                // Primary buttons get blue-tinted shadows
                (ButtonVariant::Primary, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_PRIMARY_LIGHT),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_NEUTRAL_LIGHT),
                ],
                (ButtonVariant::Primary, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_PRIMARY_DARK),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_PRIMARY_LIGHT),
                ],

                // Secondary buttons get neutral shadows
                (ButtonVariant::Secondary, Theme::Light) => vec![
                    Shadow::new().y(3).x(0).blur(6).spread(-1).color(SHADOW_COLOR_BLACK_MEDIUM),
                    Shadow::new().y(1).x(0).blur(3).spread(-1).color(SHADOW_COLOR_BLACK_LIGHT),
                ],
                (ButtonVariant::Secondary, Theme::Dark) => vec![
                    Shadow::new().y(3).x(0).blur(6).spread(-1).color(SHADOW_COLOR_BLACK_STRONG),
                    Shadow::new().y(1).x(0).blur(3).spread(-1).color(SHADOW_COLOR_BLACK_DARK),
                ],

                // Outline buttons get subtle shadows
                (ButtonVariant::Outline, Theme::Light) => vec![
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_BLACK_LIGHT),
                    Shadow::new().y(1).x(0).blur(2).spread(-1).color(SHADOW_COLOR_BLACK_SUBTLE),
                ],
                (ButtonVariant::Outline, Theme::Dark) => vec![
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_BLACK_STRONG),
                    Shadow::new().y(1).x(0).blur(2).spread(-1).color(SHADOW_COLOR_BLACK_DARK),
                ],

                // Destructive buttons get red-tinted shadows
                (ButtonVariant::Destructive, Theme::Light) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_ERROR_LIGHT),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_NEUTRAL_LIGHT),
                ],
                (ButtonVariant::Destructive, Theme::Dark) => vec![
                    Shadow::new().y(4).x(0).blur(6).spread(-1).color(SHADOW_COLOR_ERROR_DARK),
                    Shadow::new().y(2).x(0).blur(4).spread(-1).color(SHADOW_COLOR_ERROR_LIGHT),
                ],

                // Ghost and Link buttons get no shadows for minimal appearance
                (ButtonVariant::Ghost, _) | (ButtonVariant::Link, _) | (ButtonVariant::DestructiveGhost, _) => vec![],
            }
        })
    }

    fn create_loading_content(
        self,
        icon_size: IconSize,
        has_label: bool,
        has_left_icon: bool,
        has_right_icon: bool,
        label_signal: Option<Box<dyn Signal<Item = String> + Unpin>>,
    ) -> RawElOrText {
        // Create spinning animation using MoonZoon's Oscillator
        use crate::tokens::animation::create_spinner;
        let spinner_oscillator = create_spinner();

        // Loading spinner icon with rotation animation
        let spinner = El::new()
            .s(Width::exact(icon_size.to_px()))
            .s(Height::exact(icon_size.to_px()))
            .s(Align::center())
            .s(Transform::with_signal_self(
                spinner_oscillator
                    .signal()
                    .map(|factor| Transform::new().rotate(factor * 360.))
            ))
            .child(
                refresh_cw()
                    .size(icon_size)
                    .color(IconColor::Current)
                    .build()
            );

        // Helper function to create label text for loading state
        let create_loading_label = || {
            if let Some(signal) = label_signal {
                El::new()
                    .child_signal(signal.map(|text| Text::new(text)))
                    .unify()
            } else if let Some(label) = &self.label {
                Text::new(label).unify()
            } else {
                Text::new("").unify()
            }
        };

        if has_right_icon && !has_left_icon && has_label {
            // Right icon position: label + spinner
            Row::new()
                .s(Align::new().center_y())
                .s(Gap::new().x(SPACING_8))
                .item(create_loading_label())
                .item(spinner)
                .unify()
        } else if has_label {
            // Default: spinner + label
            Row::new()
                .s(Align::new().center_y())
                .s(Gap::new().x(SPACING_8))
                .item(spinner)
                .item(create_loading_label())
                .unify()
        } else {
            // Spinner only
            spinner.unify()
        }
    }
}

// Convenience function for creating buttons
pub fn button() -> ButtonBuilder {
    ButtonBuilder::new()
}
