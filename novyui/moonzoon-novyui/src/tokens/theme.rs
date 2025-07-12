// Theme Management System
// Research-validated pattern from ringrev_private

use zoon::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

// Custom persistence function type
type ThemePersistenceFn = Option<Box<dyn Fn(Theme) + Send + Sync>>;

static THEME: Lazy<Mutable<Theme>> = Lazy::new(|| {
    Mutable::new(Theme::Dark) // Will be initialized properly via init_theme
});

static CUSTOM_PERSISTENCE: Lazy<Mutable<ThemePersistenceFn>> = Lazy::new(|| {
    Mutable::new(None)
});

/// Initialize theme system with optional custom persistence
/// If custom_persistence is provided, localStorage will not be used
pub fn init_theme(initial_theme: Option<Theme>, custom_persistence: ThemePersistenceFn) {
    // Set custom persistence handler
    CUSTOM_PERSISTENCE.set(custom_persistence);
    
    let theme_to_use = if let Some(theme) = initial_theme {
        theme
    } else if CUSTOM_PERSISTENCE.lock_ref().is_some() {
        // If custom persistence is set but no initial theme provided, use default
        Theme::Dark
    } else {
        // Fallback to localStorage for apps without custom persistence
        let stored_theme = local_storage()
            .get("novyui-theme")
            .unwrap_or(Ok(String::new()))
            .unwrap_or_default();

        match stored_theme.as_str() {
            "light" => Theme::Light,
            _ => Theme::Dark,
        }
    };
    
    THEME.set(theme_to_use);
}

/// Initialize theme with localStorage (backward compatibility)
pub fn init_theme_with_localstorage() {
    init_theme(None, None);
}

/// Get the current theme as a signal for reactive updates
pub fn theme() -> impl Signal<Item = Theme> {
    THEME.signal()
}

/// Set the current theme and persist using configured method
pub fn set_theme(new_theme: Theme) {
    THEME.set(new_theme);

    // Use custom persistence if available, otherwise localStorage
    if let Some(ref persistence_fn) = CUSTOM_PERSISTENCE.lock_ref().as_ref() {
        persistence_fn(new_theme);
    } else {
        // Fallback to localStorage for apps without custom persistence
        let theme_str = match new_theme {
            Theme::Light => "light",
            Theme::Dark => "dark",
        };
        let _ = local_storage().insert("novyui-theme", theme_str);
    }
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

/// Set theme without triggering the persistence callback
/// Used internally to prevent circular updates when syncing from external sources
pub fn set_theme_without_callback(new_theme: Theme) {
    THEME.set(new_theme);
}
