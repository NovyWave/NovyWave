use zoon::{*, futures_util::future::try_join_all};
use moonzoon_novyui::tokens::theme::{Theme, init_theme};
use moonzoon_novyui::tokens::color::{neutral_1};

mod virtual_list;

mod file_utils;
use file_utils::*;

mod connection;
use connection::*;

mod config;
use config::{CONFIG_LOADED, config_store, create_config_triggers, sync_config_to_globals, sync_globals_to_config, sync_theme_to_novyui};

mod types;
use shared::{UpMsg};

mod views;
use views::*;

mod state;
use state::*;

mod utils;
use utils::*;


/// Entry point: loads fonts and starts the app.
pub fn main() {
    Task::start(async {
        load_and_register_fonts().await;
        
        // Initialize scope selection handling
        init_scope_selection_handlers();
        
        // Initialize file picker directory browsing
        init_file_picker_handlers();
        
        // Initialize signal-based loading completion handling
        init_signal_chains();
        
        init_connection();
        
        // Load configuration FIRST before setting up reactive triggers
        send_up_msg(UpMsg::LoadConfig);
        
        // Wait for CONFIG_LOADED flag, then set up reactive system
        Task::start(async {
            // Wait for config to actually load from backend
            CONFIG_LOADED.signal().for_each_sync(|loaded| {
                if loaded {
                    zoon::println!("Config loaded from backend, setting up reactive system...");
                    
                    // Initialize bidirectional sync between config store and global state FIRST
                    sync_config_to_globals();
                    sync_globals_to_config();
                    
                    // Initialize reactive triggers AFTER config is loaded and synced
                    create_config_triggers();
                    
                    // Initialize theme synchronization from config store to NovyUI
                    sync_theme_to_novyui();
                    
                    // Initialize theme system with unified config persistence  
                    // NOTE: Access config_store() AFTER apply_config() has loaded real values
                    let current_theme = config_store().ui.lock_ref().theme.get_cloned();
                    let novyui_theme = match current_theme {
                        crate::config::Theme::Light => Theme::Light,
                        crate::config::Theme::Dark => Theme::Dark,
                    };
                    
                    init_theme(
                        Some(novyui_theme), // Use loaded theme, not default
                        Some(Box::new(|novyui_theme| {
                            zoon::println!("Theme callback triggered! NovyUI theme: {:?}", novyui_theme);
                            // Convert NovyUI theme to config theme and update store
                            let config_theme = match novyui_theme {
                                Theme::Light => crate::config::Theme::Light,
                                Theme::Dark => crate::config::Theme::Dark,
                            };
                            zoon::println!("Setting config theme to: {:?}", config_theme);
                            config_store().ui.lock_mut().theme.set_neq(config_theme);
                            zoon::println!("Config theme set!");
                        }))
                    );
                    
                    // NOW start the app after config is fully loaded and reactive system is set up
                    start_app("app", root);
                }
            }).await
        });
    });
}


fn init_scope_selection_handlers() {
    Task::start(async {
        TREE_SELECTED_ITEMS.signal_ref(|selected_items| {
            selected_items.clone()
        }).for_each_sync(|selected_items| {
            // Find the first selected scope (has _scope_ pattern, not just file_)
            if let Some(scope_id) = selected_items.iter().find(|id| id.contains("_scope_")) {
                SELECTED_SCOPE_ID.set_neq(Some(scope_id.clone()));
                // Clear the flag when a scope is selected
                USER_CLEARED_SELECTION.set(false);
            } else {
                // No scope selected - check if this is user action or startup
                SELECTED_SCOPE_ID.set_neq(None);
                
                // Only set flag if config is loaded (prevents startup interference)
                if CONFIG_LOADED.get() {
                    USER_CLEARED_SELECTION.set(true);
                    zoon::println!("TreeView: User cleared scope selection, setting flag to prevent restoration");
                }
            }
        }).await
    });
    
    // Auto-save when selected scope changes
    Task::start(async {
        SELECTED_SCOPE_ID.signal_cloned().for_each_sync(|_| {
            if CONFIG_LOADED.get() && !DOCK_TOGGLE_IN_PROGRESS.get() {
                config::save_scope_selection();
            }
        }).await
    });
    
    // Auto-save when expanded scopes change
    Task::start(async {
        EXPANDED_SCOPES.signal_ref(|expanded_scopes| {
            expanded_scopes.clone()
        }).for_each_sync(|_expanded_scopes| {
            if CONFIG_LOADED.get() && !DOCK_TOGGLE_IN_PROGRESS.get() {
                config::save_scope_selection();
            }
        }).await
    });
}

fn init_file_picker_handlers() {
    // Watch for file selection events (double-click to browse directories)
    Task::start(async {
        FILE_PICKER_SELECTED.signal_ref(|selected_items| {
            selected_items.clone()
        }).for_each_sync(|_selected_items| {
            // Simple approach: For now, we'll implement manual directory browsing
            // via the breadcrumb navigation rather than automatic expansion
            // This avoids the complexity of tracking which directories have been loaded
        }).await
    });
}

/// Loads and registers required fonts asynchronously.
async fn load_and_register_fonts() {
    let fonts = try_join_all([
        fast2d::fetch_file("/_api/public/fonts/FiraCode-Regular.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-Regular.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-Bold.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
    ]).await.unwrap_throw();
    fast2d::register_fonts(fonts).unwrap_throw();
}


fn root() -> impl Element {
    // One-time Load Files dialog opening for development/debug
    
    Stack::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Background::new().color_signal(neutral_1()))
        .layer(main_layout())
        .layer_signal(SHOW_FILE_DIALOG.signal().map_true(
            || file_paths_dialog()
        ))
}

fn main_layout() -> impl Element {
    let is_any_divider_dragging = map_ref! {
        let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
        let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal() =>
        *vertical || *horizontal
    };

    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Scrollbars::both())
        .text_content_selecting_signal(
            is_any_divider_dragging.map(|is_dragging| {
                if is_dragging {
                    TextContentSelecting::none()
                } else {
                    TextContentSelecting::auto()
                }
            })
        )
        .s(Cursor::with_signal(
            map_ref! {
                let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
                let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal() =>
                if *vertical {
                    Some(CursorIcon::ColumnResize)
                } else if *horizontal {
                    Some(CursorIcon::RowResize)
                } else {
                    None
                }
            }
        ))
        .on_pointer_up(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_leave(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_move_event(|event| {
            if VERTICAL_DIVIDER_DRAGGING.get() {
                FILES_PANEL_WIDTH.update(|width| {
                    let new_width = width as i32 + event.movement_x();
                    u32::max(50, u32::try_from(new_width).unwrap_or(50))
                });
                if CONFIG_LOADED.get() && !DOCK_TOGGLE_IN_PROGRESS.get() {
                    config::save_panel_layout();
                }
            } else if HORIZONTAL_DIVIDER_DRAGGING.get() {
                if IS_DOCKED_TO_BOTTOM.get() {
                    // In "Docked to Bottom" mode, horizontal divider controls files panel height
                    FILES_PANEL_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                } else {
                    // In "Docked to Right" mode, horizontal divider controls files panel height
                    FILES_PANEL_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                }
            }
        })
        .child(docked_layout_wrapper())
}

// Wrapper function that switches between docked and undocked layouts
fn docked_layout_wrapper() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Scrollbars::both())
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            if is_docked {
                // Docked to Bottom layout
                El::new()
                    .s(Height::fill())
                    .s(Scrollbars::both())
                    .child(
                        Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                Row::new()
                                    .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
                                    .s(Width::fill())
                                    .item(files_panel_docked())
                                    .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                                    .item(variables_panel_docked())
                            )
                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                            .item(selected_variables_with_waveform_panel())
                    )
            } else {
                // Docked to Right layout
                El::new()
                    .s(Height::fill())
                    .s(Scrollbars::both())
                    .child(
                        Row::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                El::new()
                                    .s(Width::exact_signal(FILES_PANEL_WIDTH.signal()))
                                    .s(Height::fill())
                                    .child(
                                        Column::new()
                                            .s(Height::fill())
                                            .item(files_panel_with_height())
                                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                                            .item(variables_panel_with_fill())
                                    )
                            )
                            .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                            .item(
                                El::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .child(selected_variables_with_waveform_panel())
                            )
                    )
            }
        }))
}

