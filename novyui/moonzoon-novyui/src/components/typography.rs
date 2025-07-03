// Typography Components
// Based on NovyUI Vue typography components

use crate::tokens::*;
use zoon::*;

// Heading Components - Using proper token names
pub fn h1(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_36).weight(FontWeight::Bold))
        .s(Font::new().color_signal(neutral_12()))
        .child(Text::new(text.into()))
}

pub fn h2(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_30).weight(FontWeight::Bold))
        .s(Font::new().color_signal(neutral_12()))
        .child(Text::new(text.into()))
}

pub fn h3(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_24).weight(FontWeight::SemiBold))
        .s(Font::new().color_signal(neutral_12()))
        .child(Text::new(text.into()))
}

pub fn h4(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_20).weight(FontWeight::SemiBold))
        .s(Font::new().color_signal(neutral_12()))
        .child(Text::new(text.into()))
}

pub fn h5(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_18).weight(FontWeight::Medium))
        .s(Font::new().color_signal(neutral_12()))
        .child(Text::new(text.into()))
}

pub fn h6(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_16).weight(FontWeight::Medium))
        .s(Font::new().color_signal(neutral_12()))
        .child(Text::new(text.into()))
}

// Paragraph Component - Using proper token names
pub fn paragraph(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_16))
        .s(Font::new().color_signal(neutral_11()))
        .child(Text::new(text.into()))
}

// Small Text Component - Using proper token names
pub fn small(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_14))
        .s(Font::new().color_signal(neutral_9()))
        .child(Text::new(text.into()))
}

// Code Component - Using proper token names
pub fn code(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_14).family([FontFamily::new(FONT_FAMILY_MONO)]))
        .s(Padding::new().x(SPACING_4).y(SPACING_2))
        .s(RoundedCorners::all(4))
        .s(Background::new().color_signal(neutral_2()))
        .s(Font::new().color_signal(neutral_11()))
        .child(Text::new(text.into()))
}

// Lead Text Component (larger paragraph) - Using proper token names
pub fn lead(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_18))
        .s(Font::new().color_signal(neutral_11()))
        .child(Text::new(text.into()))
}

// Muted Text Component
pub fn muted(text: impl Into<String>) -> impl Element {
    El::new()
        .s(Font::new().size(FONT_SIZE_14))
        .s(Font::new().color_signal(neutral_8()))
        .child(Text::new(text.into()))
}
