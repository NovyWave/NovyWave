use zoon::*;
use crate::tokens::*;

// Simplified List component - focusing on getting it working first
pub struct ListBuilder {
    items: Vec<String>,
}

pub struct ListItemBuilder {
    text: String,
}

impl ListBuilder {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }

    pub fn item(mut self, text: impl Into<String>) -> Self {
        self.items.push(text.into());
        self
    }

    pub fn build(self) -> impl Element {
        Column::new()
            .s(Width::fill())
            .s(Gap::new().y(SPACING_8))
            .items(self.items.into_iter().map(|text| {
                El::new()
                    .s(Width::fill())
                    .s(Padding::new().x(SPACING_16).y(SPACING_12))
                    .s(Font::new()
                        .size(FONT_SIZE_16)
                        .color_signal(neutral_11())
                    )
                    .child(Text::new(&text))
            }))
    }
}

impl ListItemBuilder {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
        }
    }
}

// Convenience functions
pub fn list() -> ListBuilder {
    ListBuilder::new()
}

pub fn list_item(text: impl Into<String>) -> ListItemBuilder {
    ListItemBuilder::new(text)
}
