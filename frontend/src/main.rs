use zoon::{*, futures_util::future::try_join_all};
use moonzoon_novyui::tokens::theme::{Theme, init_theme};
use moonzoon_novyui::tokens::color::{neutral_1};


mod virtual_list;

mod debug_utils;

mod clipboard;

mod file_utils;
use file_utils::*;

mod format_utils;

mod waveform_canvas;


mod connection;
use connection::*;

mod platform;

mod config;
use config::{CONFIG_LOADED, config_store, create_config_triggers, sync_theme_to_novyui};

mod types;

mod time_types;

use shared::{UpMsg};

mod views;
use views::*;

mod state;
use state::*;
use state::VARIABLES_SEARCH_INPUT_FOCUSED;
pub use state::CONFIG_INITIALIZATION_COMPLETE;


mod unified_timeline_service;
use unified_timeline_service::*;

mod utils;
use utils::*;

mod error_display;
use error_display::*;


mod error_ui;
use error_ui::*;


fn init_timeline_signal_handlers() {
    // Enhanced timeline cursor signal handler that directly calls SignalDataService
    // This extends the existing cursor handling with direct service integration
    
    Task::start(async {
        // Track last cursor position and request time to avoid duplicate requests
        let last_position = Mutable::new(0.0);
        let last_request_time = Mutable::new(0.0);
        
        
        // Monitor timeline cursor position changes with intelligent debouncing
        crate::state::TIMELINE_CURSOR_NS.signal().for_each_sync(move |cursor_ns| {
            let cursor_pos = cursor_ns.display_seconds();
            let last_position_clone = last_position.clone();
            let last_request_time_clone = last_request_time.clone();
            
            let old_pos = last_position_clone.get();
            let position_delta = (cursor_pos - old_pos).abs();
            
            // Skip if position hasn't changed significantly (avoid noise)
            if position_delta < 0.0000001 { // Ultra-low threshold for microsecond-level changes (0.1μs)
                return;
            }
            
            
            // Skip during active cursor movement to prevent excessive requests
            let is_moving = crate::state::IS_CURSOR_MOVING_LEFT.get() || crate::state::IS_CURSOR_MOVING_RIGHT.get();
            
            if !is_moving {
                // Simple time-based debouncing using js_sys::Date::now()
                let current_time = js_sys::Date::now();
                let time_since_last = current_time - last_request_time_clone.get();
                
                // Only proceed if enough time has passed (300ms debounce)
                if time_since_last >= 300.0 {
                    
                    // Get current selected variables for direct service call
                    let selected_vars = crate::state::SELECTED_VARIABLES.lock_ref();
                    if !selected_vars.is_empty() {
                        
                        // Create signal requests for direct SignalDataService call
                        let signal_requests: Vec<crate::unified_timeline_service::SignalRequest> = selected_vars
                            .iter()
                            .filter_map(|var| {
                                // Parse unique_id: "/path/file.ext|scope|variable"
                                let parts: Vec<&str> = var.unique_id.split('|').collect();
                                if parts.len() != 3 {
                                    return None;
                                }
                                
                                Some(crate::unified_timeline_service::SignalRequest {
                                    file_path: parts[0].to_string(),
                                    scope_path: parts[1].to_string(),  
                                    variable_name: parts[2].to_string(),
                                    time_range_ns: None, // Point query for cursor position
                                    max_transitions: None, // Use service defaults
                                    format: var.formatter.unwrap_or_default(), // Use VarFormat default
                                })
                            })
                            .collect();
                        
                        if !signal_requests.is_empty() {
                            
                            // Convert to signal IDs and request cursor values
                            let signal_ids: Vec<String> = signal_requests.iter()
                                .map(|req| format!("{}|{}|{}", req.file_path, req.scope_path, req.variable_name))
                                .collect();
                            let cursor_time_ns = crate::time_types::TimeNs::from_nanos((cursor_pos * 1_000_000_000.0) as u64);
                            crate::unified_timeline_service::UnifiedTimelineService::request_cursor_values(signal_ids, cursor_time_ns);
                            
                            // Update last request time
                            last_request_time_clone.set(current_time);
                        } else {
                        }
                    } else {
                    }
                } else {
                }
            } else {
            }
            
            // Always update last position
            last_position_clone.set(cursor_pos);
        }).await;
    });
}

/// Initialize reactive handlers that bridge SELECTED_VARIABLES state to SignalDataService
/// This ensures that when variables are added/removed, SignalDataService is automatically updated
fn init_selected_variables_signal_service_bridge() {
    
    Task::start(async {
        // Track previous state to detect additions and removals
        let previous_vars: Mutable<Vec<shared::SelectedVariable>> = Mutable::new(Vec::new());
        
        // Use MutableVec.signal_vec_cloned().to_signal_cloned() to get Vec instead of VecDiff
        SELECTED_VARIABLES.signal_vec_cloned().to_signal_cloned().for_each(move |current_vars| {
            let previous_vars = previous_vars.clone();
            async move {
                // Only process changes after config initialization is complete
                if !CONFIG_LOADED.get() || IS_LOADING.get() {
                    return;
                }
                
                let current_count = current_vars.len();
                let previous_state = previous_vars.get_cloned();
                let _previous_count = previous_state.len();
                
                
                if current_count > 0 {
                    // Identify removed variables for targeted cleanup
                    let previous_ids: std::collections::HashSet<String> = previous_state
                        .iter()
                        .map(|var| var.unique_id.clone())
                        .collect();
                        
                    let current_ids: std::collections::HashSet<String> = current_vars
                        .iter()
                        .map(|var| var.unique_id.clone())
                        .collect();
                    
                    // Find removed variables
                    let removed_ids: Vec<String> = previous_ids
                        .difference(&current_ids)
                        .cloned()
                        .collect();
                    
                    // Find added variables  
                    let added_ids: Vec<String> = current_ids
                        .difference(&previous_ids)
                        .cloned()
                        .collect();
                    
                    if !removed_ids.is_empty() {
                        crate::unified_timeline_service::UnifiedTimelineService::cleanup_variables(&removed_ids);
                    }
                    
                    if !added_ids.is_empty() || (!removed_ids.is_empty() && current_count > 0) {
                        // Variables were added OR some removed but others remain - request data for current variables
                        let current_cursor = TIMELINE_CURSOR_NS.get().display_seconds();
                        
                        // Create signal requests for all currently selected variables  
                        let signal_requests: Vec<crate::unified_timeline_service::SignalRequest> = current_vars
                            .iter()
                            .filter_map(|var| {
                                // Parse unique_id: "/path/file.ext|scope|variable"
                                let parts: Vec<&str> = var.unique_id.split('|').collect();
                                if parts.len() != 3 {
                                    return None;
                                }
                                
                                Some(crate::unified_timeline_service::SignalRequest {
                                    file_path: parts[0].to_string(),
                                    scope_path: parts[1].to_string(),  
                                    variable_name: parts[2].to_string(),
                                    time_range_ns: None, // Point query at current cursor position
                                    max_transitions: None, // Use service defaults
                                    format: var.formatter.unwrap_or_default(), // Use VarFormat default
                                })
                            })
                            .collect();
                        
                        if !signal_requests.is_empty() {
                            if !added_ids.is_empty() {
                            }
                            
                            // Convert to signal IDs and request cursor values  
                            let signal_ids: Vec<String> = signal_requests.iter()
                                .map(|req| format!("{}|{}|{}", req.file_path, req.scope_path, req.variable_name))
                                .collect();
                            let cursor_time_ns = crate::time_types::TimeNs::from_nanos((current_cursor * 1_000_000_000.0) as u64);
                            crate::unified_timeline_service::UnifiedTimelineService::request_cursor_values(signal_ids, cursor_time_ns);
                        } else {
                        }
                    }
                }
                
                // Update previous state for next comparison
                previous_vars.set_neq(current_vars);
            }
        }).await;
    });
}

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
        
        
        // Initialize timeline cursor signal value queries
        init_timeline_signal_handlers();
        
        // Initialize SELECTED_VARIABLES -> SignalDataService bridge
        init_selected_variables_signal_service_bridge();
        
        // Initialize error display system
        init_error_display_system();
        
        // Initialize unified timeline service with integer time architecture
        initialize_unified_timeline_service();
        
        init_connection();
        
        // Load actual config from backend
        debug_utils::debug_conditional("Loading real config from backend");
        send_up_msg(UpMsg::LoadConfig);
        
        // Wait for CONFIG_LOADED flag, then set up reactive system
        Task::start(async {
            // Wait for config to actually load from backend
            CONFIG_LOADED.signal().for_each_sync(|loaded| {
                if loaded {
                
                    
                    // Initialize reactive config persistence system
                    config::setup_reactive_config_persistence();
                    
                    // Initialize reactive triggers AFTER config is loaded and synced
                    create_config_triggers();
                    
                    // Initialize theme synchronization from config store to NovyUI
                    sync_theme_to_novyui();
                    
                    // Initialize theme system with unified config persistence  
                    // NOTE: Access config_store() AFTER apply_config() has loaded real values
                    let current_theme = config_store().ui.lock_ref().theme.get_cloned();
                    let novyui_theme = match current_theme {
                        config::Theme::Light => Theme::Light,
                        config::Theme::Dark => Theme::Dark,
                    };
                    
                    init_theme(
                        Some(novyui_theme), // Use loaded theme, not default
                        Some(Box::new(|novyui_theme| {
                            // Convert NovyUI theme to config theme and update store
                            let config_theme = match novyui_theme {
                                Theme::Light => config::Theme::Light,
                                Theme::Dark => config::Theme::Dark,
                            };
                            config_store().ui.lock_mut().theme.set_neq(config_theme.clone());
                            
                            // Only save if initialization is complete to prevent startup overwrites
                            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                                config::save_config_to_backend();
                            }
                        }))
                    );
                    
                    
                    // NOW start the app after config is fully loaded and reactive system is set up
                    start_app("app", root);
                    
                    // Initialize the complete application flow with proper phases
                    Task::start(initialize_complete_app_flow());
                }
            }).await
        });
        
        // Query signal values when cursor movement stops (listen to movement flags directly)
        Task::start(async {
            let was_moving = Mutable::new(false);
            
            // Listen to movement flags directly instead of cursor position changes
            let movement_signal = map_ref! {
                let left = crate::state::IS_CURSOR_MOVING_LEFT.signal(),
                let right = crate::state::IS_CURSOR_MOVING_RIGHT.signal() =>
                *left || *right
            };
            
            movement_signal.for_each_sync(move |is_moving| {
                if is_moving {
                    // Movement started - just track state, don't query
                    was_moving.set(true);
                } else if was_moving.get() {
                    // Movement just stopped - use unified caching logic with built-in range checking
                    was_moving.set(false);
                    crate::views::trigger_signal_value_queries();
                }
            }).await;
        });
        
        // Separate handler for direct cursor position changes (mouse clicks)
        Task::start(async {
            let last_position = Mutable::new(0.0);
            
            crate::state::TIMELINE_CURSOR_NS.signal().for_each_sync(move |cursor_ns| {
                let cursor_pos = cursor_ns.display_seconds();
                let is_moving = crate::state::IS_CURSOR_MOVING_LEFT.get() || crate::state::IS_CURSOR_MOVING_RIGHT.get();
                
                // Only query for direct position changes (not during Q/E movement)
                if !is_moving && (cursor_pos - last_position.get()).abs() > 0.001 {
                    // Use the unified caching logic with built-in range checking
                    crate::views::trigger_signal_value_queries();
                }
                
                last_position.set(cursor_pos);
            }).await;
        });
    });
}

// Helper functions for optimized variable value updates

/// Check if cursor is within the currently visible timeline range
pub fn is_cursor_in_visible_range(cursor_time: f64) -> bool {
    let viewport = crate::state::TIMELINE_VIEWPORT.get();
    let cursor_ns = crate::time_types::TimeNs::from_nanos((cursor_time * 1_000_000_000.0) as u64);
    viewport.contains(cursor_ns)
}




fn init_scope_selection_handlers() {
    Task::start(async {
        TREE_SELECTED_ITEMS.signal_ref(|selected_items| {
            selected_items.clone()
        }).for_each_sync(|selected_items| {
            // Find the first selected scope (not a file)
            // Files are tracked in TRACKED_FILE_IDS cache, scopes are not
            if let Some(tree_id) = selected_items.iter().find(|id| {
                // Check if this ID is NOT a tracked file ID - use cache to prevent recursive locking
                !TRACKED_FILE_IDS.lock_ref().contains(*id)
            }) {
                // Convert TreeView scope ID back to original scope ID
                let scope_id = if tree_id.starts_with("scope_") {
                    tree_id.strip_prefix("scope_").unwrap_or(tree_id).to_string()
                } else {
                    tree_id.clone()
                };
                
                SELECTED_SCOPE_ID.set_neq(Some(scope_id));
                // Clear the flag when a scope is selected
                USER_CLEARED_SELECTION.set_neq(false);
            } else {
                // No scope selected - check if this is user action or startup
                SELECTED_SCOPE_ID.set_neq(None);
                
                // Only set flag if config is loaded (prevents startup interference)
                if CONFIG_LOADED.get() {
                    USER_CLEARED_SELECTION.set_neq(true);
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
    
    // Auto-query signal values when selected variables change
    Task::start(async {
        SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_| async move {
            if CONFIG_LOADED.get() && !IS_LOADING.get() {
                // Variable selection changed - use unified caching logic
                crate::views::trigger_signal_value_queries();
            }
        }).await
    });
}

fn init_file_picker_handlers() {
    // Watch for file selection events (double-click to browse directories)
    Task::start(async {
        FILE_PICKER_SELECTED.signal_vec_cloned().for_each(|_| async move {
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
        fast2d::fetch_file("/_api/public/fonts/Inter-Italic.ttf"),
        fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
    ]).await.unwrap_throw();
    fast2d::register_fonts(fonts).unwrap_throw();
}


/// Complete application initialization with proper phases to fix N/A bug
async fn initialize_complete_app_flow() {
    // Phase 1: Load restored files from config (if any)
    let config_store = config_store();
    let opened_files = config_store.session.lock_ref().opened_files.lock_ref().to_vec();
    
    if !opened_files.is_empty() {
        // Start loading files
        for file_path in &opened_files {
            send_up_msg(UpMsg::LoadWaveformFile(file_path.clone()));
        }
        
        // Wait for all files to complete loading
        wait_for_files_loaded(&opened_files).await;
    }
    
    // Phase 4: Variable data requests are handled automatically by signal handlers
    // Config sync restores selected variables, which triggers signal handlers that request data
    // No manual SignalDataService call needed here (prevents duplicate requests)
    
    // Phase 5: Mark initialization as complete
    crate::CONFIG_INITIALIZATION_COMPLETE.set_neq(true);
}

/// Wait for specific files to finish loading
async fn wait_for_files_loaded(file_paths: &[String]) {
    if file_paths.is_empty() {
        return;
    }
    
    // Wait until all files are either loaded or failed
    let mut check_count = 0;
    let max_checks = 300; // 30 seconds timeout (100ms * 300)
    
    loop {
        // Check if all files are finished loading - minimize lock time
        let all_finished = {
            let tracked_files = crate::state::TRACKED_FILES.lock_ref();
            file_paths.iter().all(|file_path| {
                tracked_files.iter().any(|tracked| {
                    tracked.path == *file_path && 
                    matches!(tracked.state, shared::FileState::Loaded(_) | shared::FileState::Failed(_))
                })
            })
        }; // Lock is released here
        
        if all_finished {
            break;
        }
        
        check_count += 1;
        if check_count >= max_checks {
            break;
        }
        
        // Yield to allow queue processor to run, then wait
        Task::next_macro_tick().await;
        Timer::sleep(100).await; // Check every 100ms
    }
}

fn root() -> impl Element {
    // One-time Load Files dialog opening for development/debug
    
    Stack::new()
        .s(Height::screen())
        .s(Width::fill())
        .s(Background::new().color_signal(neutral_1()))
        .s(Font::new().family([FontFamily::new("Inter"), FontFamily::new("system-ui"), FontFamily::new("Segoe UI"), FontFamily::new("Arial"), FontFamily::SansSerif]))
        .layer(main_layout())
        .layer_signal(SHOW_FILE_DIALOG.signal().map_true(
            || file_paths_dialog()
        ))
        .layer(toast_notifications_container())
}


fn main_layout() -> impl Element {
    let is_any_divider_dragging = map_ref! {
        let vertical = VERTICAL_DIVIDER_DRAGGING.signal(),
        let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal(),
        let vars_name = VARIABLES_NAME_DIVIDER_DRAGGING.signal(),
        let vars_value = VARIABLES_VALUE_DIVIDER_DRAGGING.signal() =>
        *vertical || *horizontal || *vars_name || *vars_value
    };

    El::new()
        .s(Height::screen())
        .s(Width::fill())
        // TEST 3: Remove root container scrollbars
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
                let horizontal = HORIZONTAL_DIVIDER_DRAGGING.signal(),
                let vars_name = VARIABLES_NAME_DIVIDER_DRAGGING.signal(),
                let vars_value = VARIABLES_VALUE_DIVIDER_DRAGGING.signal() =>
                if *vertical || *vars_name || *vars_value {
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
            VARIABLES_NAME_DIVIDER_DRAGGING.set_neq(false);
            VARIABLES_VALUE_DIVIDER_DRAGGING.set_neq(false);
        })
        .on_pointer_leave(|| {
            VERTICAL_DIVIDER_DRAGGING.set_neq(false);
            HORIZONTAL_DIVIDER_DRAGGING.set_neq(false);
            VARIABLES_NAME_DIVIDER_DRAGGING.set_neq(false);
            VARIABLES_VALUE_DIVIDER_DRAGGING.set_neq(false);
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
            } else if VARIABLES_NAME_DIVIDER_DRAGGING.get() {
                VARIABLES_NAME_COLUMN_WIDTH.update(|width| {
                    let new_width = width as i32 + event.movement_x();
                    u32::max(50, u32::try_from(new_width).unwrap_or(50))
                });
            } else if VARIABLES_VALUE_DIVIDER_DRAGGING.get() {
                VARIABLES_VALUE_COLUMN_WIDTH.update(|width| {
                    let new_width = width as i32 + event.movement_x();
                    u32::max(50, u32::try_from(new_width).unwrap_or(50))
                });
            }
        })
        .update_raw_el(move |raw_el| {
            raw_el.global_event_handler(move |event: zoon::events::KeyDown| {
                // Skip timeline controls if typing in search input
                if VARIABLES_SEARCH_INPUT_FOCUSED.get() {
                    return;
                }
                
                match event.key().as_str() {
                    "Shift" => {
                        // Track Shift key state
                        crate::state::IS_SHIFT_PRESSED.set_neq(true);
                    },
                    "w" | "W" => {
                        // Start smooth zoom in
                        crate::waveform_canvas::start_smooth_zoom_in();
                    },
                    "s" | "S" => {
                        // Start smooth zoom out
                        crate::waveform_canvas::start_smooth_zoom_out();
                    },
                    "a" | "A" => {
                        // Start smooth pan left
                        crate::waveform_canvas::start_smooth_pan_left();
                    },
                    "d" | "D" => {
                        // Start smooth pan right
                        crate::waveform_canvas::start_smooth_pan_right();
                    },
                    "q" | "Q" => {
                        if crate::state::IS_SHIFT_PRESSED.get() {
                            // Shift+Q: Jump to previous transition
                            crate::waveform_canvas::jump_to_previous_transition();
                        } else {
                            // Q: Start smooth cursor left
                            crate::waveform_canvas::start_smooth_cursor_left();
                        }
                    },
                    "e" | "E" => {
                        if crate::state::IS_SHIFT_PRESSED.get() {
                            // Shift+E: Jump to next transition
                            crate::waveform_canvas::jump_to_next_transition();
                        } else {
                            // E: Start smooth cursor right
                            crate::waveform_canvas::start_smooth_cursor_right();
                        }
                    },
                    "r" | "R" => {
                        // R: Reset zoom to 1x and fit all data
                        crate::waveform_canvas::reset_zoom_to_fit_all();
                    },
                    "z" | "Z" => {
                        // Z: Reset zoom center to 0
                        crate::waveform_canvas::reset_zoom_center();
                    },
                    _ => {} // Ignore other keys
                }
            })
            .global_event_handler(move |event: zoon::events::KeyUp| {
                // Skip timeline controls if typing in search input
                if VARIABLES_SEARCH_INPUT_FOCUSED.get() {
                    return;
                }
                
                match event.key().as_str() {
                    "Shift" => {
                        // Track Shift key state
                        crate::state::IS_SHIFT_PRESSED.set_neq(false);
                    },
                    "w" | "W" => {
                        // Stop smooth zoom in
                        crate::waveform_canvas::stop_smooth_zoom_in();
                    },
                    "s" | "S" => {
                        // Stop smooth zoom out
                        crate::waveform_canvas::stop_smooth_zoom_out();
                    },
                    "a" | "A" => {
                        // Stop smooth pan left
                        crate::waveform_canvas::stop_smooth_pan_left();
                    },
                    "d" | "D" => {
                        // Stop smooth pan right
                        crate::waveform_canvas::stop_smooth_pan_right();
                    },
                    "q" | "Q" => {
                        // Always stop smooth cursor when Q is released 
                        // (Shift+Q is instantaneous, normal Q is continuous)
                        crate::waveform_canvas::stop_smooth_cursor_left();
                    },
                    "e" | "E" => {
                        // Always stop smooth cursor when E is released
                        // (Shift+E is instantaneous, normal E is continuous) 
                        crate::waveform_canvas::stop_smooth_cursor_right();
                    },
                    _ => {} // Ignore other keys
                }
            })
        })
        .child(docked_layout_wrapper())
}

// Wrapper function that switches between docked and undocked layouts
fn docked_layout_wrapper() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        // TEST 3: Remove root container scrollbars
        .child_signal(IS_DOCKED_TO_BOTTOM.signal().map(|is_docked| {
            if is_docked {
                // Docked to Bottom layout
                El::new()
                    .s(Height::fill())
                    .child(
                        Column::new()
                            .s(Height::fill())
                            .s(Width::fill())
                            .item(
                                Row::new()
                                    .s(Height::exact_signal(FILES_PANEL_HEIGHT.signal()))
                                    .s(Width::fill())
                                    .item(
                                        El::new()
                                            .s(Width::exact_signal(FILES_PANEL_WIDTH.signal()))
                                            .s(Height::fill())
                                            .child(files_panel_with_height())
                                    )
                                    .item(vertical_divider(VERTICAL_DIVIDER_DRAGGING.clone()))
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .child(variables_panel_with_fill())
                                    )
                            )
                            .item(horizontal_divider(HORIZONTAL_DIVIDER_DRAGGING.clone()))
                            .item(
                                El::new()
                                    .s(Width::fill())
                                    .s(Height::fill())
                                    .s(Scrollbars::both())
                                    .child(selected_variables_with_waveform_panel())
                            )
                    )
            } else {
                // Docked to Right layout
                El::new()
                    .s(Height::fill())
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
