use crate::dataflow::atom::Atom;
use moonzoon_novyui::tokens::color::neutral_8;
use moonzoon_novyui::tokens::theme::Theme;
use moonzoon_novyui::*;
use zoon::*;

/// Load files button with progress indicator
pub fn load_files_button_with_progress(
    tracked_files: crate::tracked_files::TrackedFiles,
    variant: ButtonVariant,
    size: ButtonSize,
    icon: Option<IconName>,
    file_dialog_visible: Atom<bool>,
) -> impl Element {
    // Count files that are actually in loading state
    // Use files.signal_vec().to_signal_cloned() as per TrackedFiles architecture
    let loading_count_signal =
        tracked_files
            .files
            .signal_vec()
            .to_signal_cloned()
            .map(move |files| {
                files
                    .iter()
                    .filter(|file| matches!(file.state, shared::FileState::Loading(_)))
                    .count()
            });

    El::new().child_signal(loading_count_signal.map(move |loading_count| {
        let is_loading = loading_count > 0; // Only true when files are actively loading
        let mut btn = button();

        if is_loading {
            btn = btn.label("Loading...").disabled(true);
            if let Some(icon) = icon {
                btn = btn.left_icon(icon);
            }
        } else {
            btn = btn.label("Load Files").on_press({
                let file_dialog_visible = file_dialog_visible.clone();
                move || file_dialog_visible.set(true)
            });
            if let Some(icon) = icon {
                btn = btn.left_icon(icon);
            }
        }

        btn.variant(variant.clone())
            .size(size.clone())
            .build()
            .into_element()
    }))
}

/// Clear all files button
pub fn clear_all_files_button(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    let tracked_files_clone = tracked_files.clone();
    let selected_variables_clone = selected_variables.clone();
    button()
        .label("Clear All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(move || {
            crate::file_operations::clear_all_files(
                &tracked_files_clone,
                &selected_variables_clone,
            );
        })
        .build()
}

/// Clear all selected variables button
pub fn clear_all_variables_button(
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Element {
    let selected_variables_clone = selected_variables.clone();
    button()
        .label("Clear All")
        .left_icon(IconName::X)
        .variant(ButtonVariant::DestructiveGhost)
        .size(ButtonSize::Small)
        .on_press(move || {
            selected_variables_clone.selection_cleared_relay.send(());
        })
        .build()
}

/// Theme toggle button
pub fn theme_toggle_button(app_config: &crate::config::AppConfig) -> impl Element {
    let app_config = app_config.clone();
    El::new().child_signal(theme().map(move |current_theme| {
        button()
            .left_icon(match current_theme {
                Theme::Light => IconName::Moon,
                Theme::Dark => IconName::Sun,
            })
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Small)
            .on_press({
                let theme_relay = app_config.theme_button_clicked_relay.clone();
                move || theme_relay.send(())
            })
            .build()
            .into_element()
    }))
}

/// Dock mode toggle button
pub fn dock_toggle_button(app_config: &crate::config::AppConfig) -> impl Element {
    let app_config = app_config.clone();
    El::new().child_signal(app_config.dock_mode_actor.signal().map(move |dock_mode| {
        let app_config_for_icon = app_config.clone();
        let app_config_for_press = app_config.clone();
        let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
        button()
            .label(if is_docked {
                "Dock to Right"
            } else {
                "Dock to Bottom"
            })
            .left_icon_element(move || {
                El::new()
                    .child_signal(
                        app_config_for_icon
                            .dock_mode_actor
                            .signal()
                            .map(|dock_mode| {
                                let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
                                let icon_el = icon(IconName::ArrowDownToLine)
                                    .size(IconSize::Small)
                                    .color(IconColor::Primary)
                                    .build();
                                if is_docked {
                                    El::new()
                                        .s(Transform::new().rotate(-90))
                                        .child(icon_el)
                                        .into_element()
                                } else {
                                    El::new().child(icon_el).into_element()
                                }
                            }),
                    )
                    .unify()
            })
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Small)
            .on_press({
                let dock_relay = app_config_for_press.dock_mode_button_clicked_relay.clone();
                move || {
                    dock_relay.send(());
                }
            })
            .align(Align::center())
            .build()
            .into_element()
    }))
}
