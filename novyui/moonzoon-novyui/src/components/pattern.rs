// Pattern Component System
// Provides background pattern integration using HeroPatterns

// Removed unused assets import since we use direct URLs for patterns
use crate::tokens::*;
use crate::components::typography::*;
use zoon::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatternType {
    Hexagons,
    Jigsaw,
    Overcast,
    Topography,
    Wiggle,
}

impl PatternType {
    pub fn name(&self) -> &'static str {
        match self {
            PatternType::Hexagons => "Hexagons",
            PatternType::Jigsaw => "Jigsaw",
            PatternType::Overcast => "Overcast",
            PatternType::Topography => "Topography",
            PatternType::Wiggle => "Wiggle",
        }
    }
}

// Simplified pattern functions that work with MoonZoon constraints
pub fn get_pattern_url(pattern_type: PatternType) -> &'static str {
    match pattern_type {
        PatternType::Hexagons => "/patterns/hexagons.svg",
        PatternType::Jigsaw => "/patterns/jigsaw.svg",
        PatternType::Overcast => "/patterns/overcast.svg",
        PatternType::Topography => "/patterns/topography.svg",
        PatternType::Wiggle => "/patterns/wiggle.svg",
    }
}

pub fn create_pattern_background(pattern_type: PatternType) -> impl Style<'static> {
    Background::new().url(get_pattern_url(pattern_type))
}

// Convenience functions for common patterns
pub fn hexagons_background() -> impl Style<'static> {
    create_pattern_background(PatternType::Hexagons)
}

pub fn jigsaw_background() -> impl Style<'static> {
    create_pattern_background(PatternType::Jigsaw)
}

pub fn overcast_background() -> impl Style<'static> {
    create_pattern_background(PatternType::Overcast)
}

pub fn topography_background() -> impl Style<'static> {
    create_pattern_background(PatternType::Topography)
}

pub fn wiggle_background() -> impl Style<'static> {
    create_pattern_background(PatternType::Wiggle)
}

// Pattern showcase component (simplified)
pub fn pattern_showcase() -> impl Element {
    Column::new()
        .s(Gap::new().y(SPACING_16))
        .item(h4("Background Patterns"))
        .item(small("Available HeroPatterns:"))
        .item(
            Row::new()
                .s(Gap::new().x(SPACING_12))
                .item(pattern_demo_card(PatternType::Hexagons))
                .item(pattern_demo_card(PatternType::Jigsaw))
                .item(pattern_demo_card(PatternType::Overcast))
        )
        .item(
            Row::new()
                .s(Gap::new().x(SPACING_12))
                .item(pattern_demo_card(PatternType::Topography))
                .item(pattern_demo_card(PatternType::Wiggle))
        )
}

fn pattern_demo_card(pattern_type: PatternType) -> impl Element {
    El::new()
        .s(Width::exact(120))
        .s(Height::exact(80))
        .s(RoundedCorners::all(8))
        .s(Borders::all(Border::new().width(1).color("#e5e7eb")))
        .s(create_pattern_background(pattern_type))
        .s(Align::center())
        .child(Text::new(pattern_type.name()))
}
