// Animation Token System
// Research-validated patterns using MoonZoon's comprehensive animation system

use zoon::*;

// Animation Durations
pub const DURATION_FAST: u32 = 150;
pub const DURATION_NORMAL: u32 = 300;
pub const DURATION_SLOW: u32 = 500;

// Animation Easing Functions
pub use zoon::ease;

// Helper functions for common animations
pub fn transition_fast() -> impl Style<'static> {
    Transitions::new([
        Transition::all().duration(DURATION_FAST),
    ])
}

pub fn transition_normal() -> impl Style<'static> {
    Transitions::new([
        Transition::all().duration(DURATION_NORMAL),
    ])
}

pub fn transition_slow() -> impl Style<'static> {
    Transitions::new([
        Transition::all().duration(DURATION_SLOW),
    ])
}

// Specific transition helpers
pub fn transition_colors() -> impl Style<'static> {
    Transitions::new([
        Transition::property("background-color").duration(DURATION_NORMAL),
        Transition::property("border-color").duration(DURATION_NORMAL),
        Transition::property("color").duration(DURATION_NORMAL),
    ])
}

pub fn transition_transform() -> impl Style<'static> {
    Transitions::new([
        Transition::property("transform").duration(DURATION_NORMAL),
    ])
}

pub fn transition_opacity() -> impl Style<'static> {
    Transitions::new([
        Transition::property("opacity").duration(DURATION_NORMAL),
    ])
}

// Animation helper for height changes (accordion-style)
pub fn create_height_animation(initial_height: f64) -> (Tweened, impl Signal<Item = f64>) {
    Tweened::new_and_signal(
        initial_height,
        Duration::milliseconds(DURATION_NORMAL as i64),
        ease::cubic_out,
    )
}

// Animation helper for rotation (chevron icons, etc.)
pub fn create_rotation_animation(initial_rotation: f64) -> (Tweened, impl Signal<Item = f64>) {
    Tweened::new_and_signal(
        initial_rotation,
        Duration::milliseconds(DURATION_NORMAL as i64),
        ease::cubic_out,
    )
}

// Continuous spinner animation
pub fn create_spinner() -> Oscillator {
    let oscillator = Oscillator::new(Duration::seconds(1));
    oscillator.cycle_wrap();
    oscillator
}
