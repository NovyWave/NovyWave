// Theme Management System
// Research-validated pattern from ringrev_private

use zoon::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

static THEME: Lazy<Mutable<Theme>> = Lazy::new(|| {
    // Load from localStorage or default to Light
    let stored_theme = local_storage()
        .get("novyui-theme")
        .unwrap_or(Ok(String::new()))
        .unwrap_or_default();

    let initial_theme = match stored_theme.as_str() {
        "light" => Theme::Light,
        _ => Theme::Dark,  // Default to dark theme for easier debugging
    };

    Mutable::new(initial_theme)
});

/// Get the current theme as a signal for reactive updates
pub fn theme() -> impl Signal<Item = Theme> {
    THEME.signal()
}

/// Set the current theme and persist to localStorage
pub fn set_theme(new_theme: Theme) {
    THEME.set(new_theme);

    // Persist to localStorage
    let theme_str = match new_theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
    };
    let _ = local_storage().insert("novyui-theme", theme_str);
}

/// Get the current theme value (non-reactive)
pub fn current_theme() -> Theme {
    THEME.get()
}

/// Toggle between light and dark themes
pub fn toggle_theme() {
    let current = current_theme();
    let new_theme = match current {
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::Light,
    };
    set_theme(new_theme);
}
