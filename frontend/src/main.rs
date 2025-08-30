use zoon::{*, futures_util::future::try_join_all};
use moonzoon_novyui::tokens::theme::{Theme, init_theme};
use moonzoon_novyui::tokens::color::{neutral_1};

mod dataflow;
mod actors;

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
use shared;

mod types;

mod time_types;

use shared::{UpMsg};

mod views;
use views::*;

mod state;
use state::*;
use actors::waveform_timeline::{current_viewport};
use actors::panel_layout::{
    files_width_signal, files_height_signal, vertical_dragging_signal, horizontal_dragging_signal,
    name_divider_dragging_signal, value_divider_dragging_signal, docked_to_bottom_signal,
    vertical_divider_dragged_relay, horizontal_divider_dragged_relay,
    name_divider_dragged_relay, value_divider_dragged_relay,
    mouse_moved_relay, is_dock_transitioning
};
use actors::dialog_manager::{dialog_visible_signal, file_picker_selected_signal};
pub use state::CONFIG_INITIALIZATION_COMPLETE;


mod unified_timeline_service;
use unified_timeline_service::*;

mod utils;
use utils::*;

mod error_display;
use error_display::*;


mod error_ui;
use error_ui::*;




/// Entry point: loads fonts and starts the app.
pub fn main() {
    Task::start(async {
        load_and_register_fonts().await;

         init_connection();

    

        // Initialize AppConfig first
        let app_config = config::AppConfig::new().await;
        if config::APP_CONFIG.set(app_config).is_err() {
            zoon::println!("🚨 APP CONFIG INITIALIZATION FAILED: AppConfig already initialized");
            return;
        }
        zoon::println!("✅ AppConfig initialized successfully");
        
        // Initialize Actor+Relay domain instances  
        if let Err(error_msg) = crate::actors::initialize_all_domains().await {
            zoon::println!("🚨 DOMAIN INITIALIZATION FAILED: {}", error_msg);
            error_display::add_error_alert(crate::state::ErrorAlert {
                id: "domain_init_failure".to_string(),
                title: "Domain Initialization Failed".to_string(),
                message: format!("Critical startup error: {}", error_msg),
                technical_error: error_msg.to_string(),
                error_type: crate::state::ErrorType::ConfigError,
                timestamp: js_sys::Date::now() as u64,
                auto_dismiss_ms: None, // Critical errors should not auto-dismiss
            });
            return; // Exit gracefully instead of panic
        }

            // Start the app - config is already loaded with theme
        start_app("app", root);

        // Initialize value caching - domains are already initialized
        zoon::println!("🔄 Initializing value caching after domain verification");
        crate::actors::waveform_timeline::initialize_value_caching();
        
        // Note: init_scope_selection_handlers() function does not exist yet - skipping
        zoon::println!("⚠️ init_scope_selection_handlers() not implemented yet - skipping");
        
        // Initialize file picker directory browsing
        init_file_picker_handlers();
        
        // Initialize signal-based loading completion handling
        init_signal_chains();
        
        
        // Note: init_timeline_signal_handlers() and init_selected_variables_signal_service_bridge() 
        // functions do not exist yet - skipping
        zoon::println!("⚠️ Timeline and variables signal bridge functions not implemented yet");
        
        // Initialize error display system
        init_error_display_system();
        
        // Initialize unified timeline service with integer time architecture
        initialize_unified_timeline_service();
        
       
        
        
        
        // Query signal values when cursor movement stops (ENABLED - using domain signals)
        Task::start(async {
            let was_moving = Mutable::new(false);
            
            // Use domain signals instead of direct domain access
            let movement_signal = map_ref! {
                let left = crate::actors::waveform_timeline::is_cursor_moving_left_signal(),
                let right = crate::actors::waveform_timeline::is_cursor_moving_right_signal() =>
                *left || *right
            };
            
            movement_signal.for_each_sync(move |is_moving| {
                if is_moving {
                    // Movement started - just track state, don't query
                    was_moving.set(true);
                } else if was_moving.get() {
                    // Movement just stopped - use unified caching logic with built-in range checking
                    crate::views::trigger_signal_value_queries();
                    was_moving.set(false);
                }
            }).await;
        });
        
        // Direct cursor position handler (ENABLED - using domain signals)
        Task::start(async {
            let last_position = Mutable::new(0.0);
            
            // Use domain signal instead of direct domain access - combined approach
            let movement_and_position_signal = map_ref! {
                let cursor_pos = crate::actors::waveform_timeline::cursor_position_seconds_signal(),
                let left = crate::actors::waveform_timeline::is_cursor_moving_left_signal(),
                let right = crate::actors::waveform_timeline::is_cursor_moving_right_signal() =>
                (*cursor_pos, *left || *right)
            };
            
            movement_and_position_signal.for_each_sync(move |(cursor_pos, is_moving)| {
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
    let viewport = current_viewport();
    let cursor_ns = crate::time_types::TimeNs::from_nanos((cursor_time * 1_000_000_000.0) as u64);
    viewport.contains(cursor_ns)
}





fn init_file_picker_handlers() {
    // Watch for file selection events (double-click to browse directories)
    Task::start(async {
        file_picker_selected_signal().for_each(|_| async move {
            // Simple approach: For now, we'll implement manual directory browsing
            // via the breadcrumb navigation rather than automatic expansion
            // This avoids the complexity of tracking which directories have been loaded
        }).await
    });
    
    // Watch for directory expansions in the file picker dialog
    Task::start(async {
        use actors::dialog_manager::expanded_directories_signal;
        use std::collections::HashSet;
        
        expanded_directories_signal().for_each(|expanded_dirs| async move {
            let expanded_set: HashSet<String> = expanded_dirs.iter().cloned().collect();
            crate::views::monitor_directory_expansions(expanded_set);
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
    // Phase 1: This initialization happens once during startup
    // For now, skip file loading during migration to pure reactive patterns
    // TODO: Implement proper reactive file restoration when Actor+Relay migration is complete
    
    // Phase 5: Mark initialization as complete immediately during migration
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
        .layer_signal(dialog_visible_signal().map_true(
            || file_paths_dialog()
        ))
        .layer(toast_notifications_container())
}


fn main_layout() -> impl Element {
    let is_any_divider_dragging = map_ref! {
        let vertical = vertical_dragging_signal(),
        let horizontal = horizontal_dragging_signal(),
        let vars_name = name_divider_dragging_signal(),
        let vars_value = value_divider_dragging_signal() =>
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
                let vertical = vertical_dragging_signal(),
                let horizontal = horizontal_dragging_signal(),
                let vars_name = name_divider_dragging_signal(),
                let vars_value = value_divider_dragging_signal() =>
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
            vertical_divider_dragged_relay().send(0.0);
            horizontal_divider_dragged_relay().send(0.0);
            name_divider_dragged_relay().send(0.0);
            value_divider_dragged_relay().send(0.0);
        })
        .on_pointer_leave(|| {
            vertical_divider_dragged_relay().send(0.0);
            horizontal_divider_dragged_relay().send(0.0);
            name_divider_dragged_relay().send(0.0);
            value_divider_dragged_relay().send(0.0);
        })
        .on_pointer_move_event(|event| {
            // Send mouse movement to panel layout domain to handle dragging
            mouse_moved_relay().send((event.movement_x() as f32, event.movement_y() as f32));
            
            // TODO: Handle config saving when layout changes
            // This was previously done inline but should be handled via actor signals
            // Config loading checks and saving are now handled automatically by the ConfigSaver actor
        })
        .update_raw_el(move |raw_el| {
            raw_el.global_event_handler(move |event: zoon::events::KeyDown| {
                // Skip timeline controls if typing in search input
                if state::VARIABLES_SEARCH_INPUT_FOCUSED.get() {
                    return;
                }
                
                match event.key().as_str() {
                    "Shift" => {
                        // Track Shift key state
                        crate::state::IS_SHIFT_PRESSED.set_neq(true);
                    },
                    "w" | "W" => {
                        // Zoom in using WaveformTimeline domain
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        waveform_timeline.zoom_in_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::waveform_canvas::start_smooth_zoom_in();
                    },
                    "s" | "S" => {
                        // Zoom out using WaveformTimeline domain
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        waveform_timeline.zoom_out_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::waveform_canvas::start_smooth_zoom_out();
                    },
                    "a" | "A" => {
                        // Pan left using WaveformTimeline domain
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        waveform_timeline.pan_left_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::waveform_canvas::start_smooth_pan_left();
                    },
                    "d" | "D" => {
                        // Pan right using WaveformTimeline domain
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        waveform_timeline.pan_right_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::waveform_canvas::start_smooth_pan_right();
                    },
                    "q" | "Q" => {
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        if crate::state::IS_SHIFT_PRESSED.get() {
                            // Shift+Q: Jump to previous transition using WaveformTimeline domain
                            waveform_timeline.jump_to_previous_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::waveform_canvas::jump_to_previous_transition();
                        } else {
                            // Q: Cursor left using WaveformTimeline domain
                            waveform_timeline.left_key_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::waveform_canvas::start_smooth_cursor_left();
                        }
                    },
                    "e" | "E" => {
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        if crate::state::IS_SHIFT_PRESSED.get() {
                            // Shift+E: Jump to next transition using WaveformTimeline domain
                            waveform_timeline.jump_to_next_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::waveform_canvas::jump_to_next_transition();
                        } else {
                            // E: Cursor right using WaveformTimeline domain
                            waveform_timeline.right_key_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::waveform_canvas::start_smooth_cursor_right();
                        }
                    },
                    "r" | "R" => {
                        // R: Reset zoom using WaveformTimeline domain
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        waveform_timeline.reset_zoom_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::waveform_canvas::reset_zoom_to_fit_all();
                    },
                    "z" | "Z" => {
                        // Z: Reset zoom center using WaveformTimeline domain
                        let waveform_timeline = crate::actors::waveform_timeline_domain();
                        waveform_timeline.reset_zoom_center_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::waveform_canvas::reset_zoom_center();
                    },
                    _ => {} // Ignore other keys
                }
            })
            .global_event_handler(move |event: zoon::events::KeyUp| {
                // Skip timeline controls if typing in search input
                if state::VARIABLES_SEARCH_INPUT_FOCUSED.get() {
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
        .child_signal(docked_to_bottom_signal().map(|is_docked| {
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
                                    .s(Height::exact_signal(files_height_signal()))
                                    .s(Width::fill())
                                    .item(
                                        El::new()
                                            .s(Width::exact_signal(files_width_signal()))
                                            .s(Height::fill())
                                            .child(files_panel_with_height())
                                    )
                                    .item(vertical_divider(vertical_dragging_signal()))
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .child(variables_panel_with_fill())
                                    )
                            )
                            .item(horizontal_divider(horizontal_dragging_signal()))
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
                                    .s(Width::exact_signal(files_width_signal()))
                                    .s(Height::fill())
                                    .child(
                                        Column::new()
                                            .s(Height::fill())
                                            .item(files_panel_with_height())
                                            .item(horizontal_divider(horizontal_dragging_signal()))
                                            .item(variables_panel_with_fill())
                                    )
                            )
                            .item(vertical_divider(vertical_dragging_signal()))
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

