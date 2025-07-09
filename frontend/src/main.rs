use zoon::{*, futures_util::future::try_join_all};
mod virtual_list;
use virtual_list::*;

mod file_utils;
use file_utils::*;

mod connection;
use connection::*;

mod config;
use config::*;

mod types;
use shared::{UpMsg, DownMsg};

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
        
        start_app("app", root);
        init_connection();
        
        // Load configuration on startup
        send_up_msg(UpMsg::LoadConfig);
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
            } else {
                SELECTED_SCOPE_ID.set_neq(None);
            }
        }).await
    });
    
    // Auto-save when selected scope changes
    Task::start(async {
        SELECTED_SCOPE_ID.signal_cloned().for_each_sync(|_| {
            config::save_current_config();
        }).await
    });
    
    // Auto-save when expanded scopes change
    Task::start(async {
        EXPANDED_SCOPES.signal_ref(|expanded_scopes| {
            expanded_scopes.clone()
        }).for_each_sync(|_expanded_scopes| {
            config::save_current_config();
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
    Stack::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Background::new().color(hsluv!(220, 15, 8)))
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
                config::save_current_config();
            } else if HORIZONTAL_DIVIDER_DRAGGING.get() {
                if IS_DOCKED_TO_BOTTOM.get() {
                    // In "Docked to Bottom" mode, horizontal divider controls files panel height
                    FILES_PANEL_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                    config::save_current_config();
                } else {
                    // In "Docked to Right" mode, horizontal divider controls files panel height
                    FILES_PANEL_HEIGHT.update(|height| {
                        let new_height = height as i32 + event.movement_y();
                        u32::max(50, u32::try_from(new_height).unwrap_or(50))
                    });
                    config::save_current_config();
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

