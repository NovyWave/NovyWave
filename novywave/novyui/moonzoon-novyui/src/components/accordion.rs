use zoon::*;
use crate::tokens::*;

// Accordion item data
#[derive(Debug, Clone)]
pub struct AccordionItem {
    pub title: String,
    pub content: String,
    pub disabled: bool,
}

impl AccordionItem {
    pub fn new(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: content.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// Accordion builder
pub struct AccordionBuilder {
    items: Vec<AccordionItem>,
    allow_multiple: bool,
    default_expanded: Vec<usize>,
}

impl AccordionBuilder {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            allow_multiple: false,
            default_expanded: Vec::new(),
        }
    }

    pub fn items(mut self, items: Vec<AccordionItem>) -> Self {
        self.items = items;
        self
    }

    pub fn item(mut self, item: AccordionItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn allow_multiple(mut self, allow_multiple: bool) -> Self {
        self.allow_multiple = allow_multiple;
        self
    }

    pub fn default_expanded(mut self, indices: Vec<usize>) -> Self {
        self.default_expanded = indices;
        self
    }

    pub fn build(self) -> impl Element {
        let expanded_states: Vec<Mutable<bool>> = self.items
            .iter()
            .enumerate()
            .map(|(index, _)| Mutable::new(self.default_expanded.contains(&index)))
            .collect();

        let allow_multiple = self.allow_multiple;
        let items = self.items;

        Column::new()
            .s(Width::fill())
            .s(Gap::new().y(SPACING_2))
            .items(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let expanded = expanded_states[index].clone();
                        let expanded_states_clone = expanded_states.clone();

                        build_accordion_item_simple(item, expanded)
                    })
                    .collect::<Vec<_>>()
            )
    }

}

fn build_accordion_item_simple(
    item: AccordionItem,
    expanded: Mutable<bool>,
) -> impl Element {
    let disabled = item.disabled;

    Column::new()
            .s(Width::fill())
            .s(Borders::all_signal(theme().map(|t| match t {
                Theme::Light => Border::new().width(1).color("oklch(85% 0.14 250)"), // neutral_4 light
                Theme::Dark => Border::new().width(1).color("oklch(25% 0.14 250)"), // neutral_4 dark
            })))
            .s(RoundedCorners::all(6))
            .s(Background::new().color_signal(theme().map(|t| match t {
                Theme::Light => "oklch(98% 0.14 250)", // neutral_1 light
                Theme::Dark => "oklch(8% 0.14 250)", // neutral_2 dark
            })))
            .item(
                // Header button
                Button::new()
                    .s(Width::fill())
                    .s(Padding::new().x(SPACING_16).y(SPACING_12))
                    .s(Background::new().color("transparent"))
                    .s(Borders::new())
                    .s(RoundedCorners::new().top(6))
                    .s(Cursor::new(if disabled {
                        CursorIcon::NotAllowed
                    } else {
                        CursorIcon::Pointer
                    }))
                    .s(Align::new().left())
                    .label(
                        Row::new()
                            .s(Width::fill())
                            .s(Align::new().center_y())
                            .s(Gap::new().x(SPACING_12))
                            .item(
                                El::new()
                                    .s(Width::fill())
                                    .child(Text::new(&item.title))
                                    .s(Font::new()
                                        .size(FONT_SIZE_16)
                                        .weight(FontWeight::Number(FONT_WEIGHT_5))
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
                            )
                            .item(
                                El::new()
                                    .child_signal(
                                        expanded.signal().map(|is_expanded| {
                                            Text::new(if is_expanded { "▼" } else { "▶" })
                                        })
                                    )
                                    .s(Font::new()
                                        .size(FONT_SIZE_12)
                                        .color_signal(theme().map(move |t| {
                                            if disabled {
                                                match t {
                                                    Theme::Light => "oklch(45% 0.14 250)", // neutral_5 light
                                                    Theme::Dark => "oklch(55% 0.14 250)", // neutral_5 dark
                                                }
                                            } else {
                                                match t {
                                                    Theme::Light => "oklch(65% 0.14 250)", // neutral_6 light
                                                    Theme::Dark => "oklch(55% 0.14 250)", // neutral_7 dark
                                                }
                                            }
                                        }))
                                    )
                            )
                    )
                    .on_press({
                        let expanded = expanded.clone();
                        move || {
                            if !disabled {
                                // Toggle current item
                                expanded.update(|current| !current);
                            }
                        }
                    })
            )
            .item_signal(
                expanded.signal().map(move |is_expanded| {
                    if is_expanded {
                        Some(
                            El::new()
                                .s(Padding::new().x(SPACING_16).y(SPACING_12))
                                .s(Borders::new().top_signal(theme().map(|t| match t {
                                    Theme::Light => Border::new().width(1).color("oklch(85% 0.14 250)"), // neutral_4 light
                                    Theme::Dark => Border::new().width(1).color("oklch(25% 0.14 250)"), // neutral_4 dark
                                })))
                                .child(Text::new(&item.content))
                                .s(Font::new()
                                    .size(FONT_SIZE_14)
                                    .weight(FontWeight::Number(FONT_WEIGHT_4))
                                    .color_signal(theme().map(|t| match t {
                                        Theme::Light => "oklch(35% 0.14 250)", // neutral_7 light
                                        Theme::Dark => "oklch(75% 0.14 250)", // neutral_9 dark
                                    }))
                                )
                        )
                    } else {
                        None
                    }
                })
            )
}

// Convenience functions
pub fn accordion() -> AccordionBuilder {
    AccordionBuilder::new()
}

pub fn accordion_item(title: impl Into<String>, content: impl Into<String>) -> AccordionItem {
    AccordionItem::new(title, content)
}
