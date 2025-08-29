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
use config::{CONFIG_LOADED, config_store, create_config_triggers, sync_theme_to_novyui};

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
        
        // Initialize Actor+Relay domain instances  
        if let Err(error_msg) = crate::actors::initialize_all_domains().await {
            zoon::println!("🚨 DOMAIN INITIALIZATION FAILED: {}", error_msg);
            panic!("Domain initialization failed - application cannot continue: {}", error_msg);
        }
        
        // Add delay to ensure domain storage is visible across async boundaries before any access
        Timer::sleep(100).await;
        
        // Verify domains are actually accessible before starting UI
        if !crate::actors::global_domains::_are_domains_initialized() {
            zoon::println!("🚨 CRITICAL: Domains not accessible after delay - extending wait");
            Timer::sleep(500).await;
        }
        
        // TEMPORARILY DISABLED: Value caching initialization causing startup panics
        // crate::actors::waveform_timeline::initialize_value_caching();  // ❌ Calls domain functions before domains ready
        
        // TEMPORARILY DISABLED: Scope selection handlers causing startup race conditions
        // if crate::actors::global_domains::_are_domains_initialized() {
        //     zoon::println!("🔄 Initializing scope selection handlers after domain verification");
        //     init_scope_selection_handlers();  // ❌ Calls domain functions before domains ready
        // } else {
        //     zoon::println!("⚠️ Domains not initialized after delay - skipping scope selection handlers");
        // }
        zoon::println!("⚠️ DISABLED: init_scope_selection_handlers() - prevents startup race conditions");
        
        // Initialize file picker directory browsing
        init_file_picker_handlers();
        
        // Initialize signal-based loading completion handling
        init_signal_chains();
        
        
        // TEMPORARILY DISABLED: Timeline signal handlers causing startup race conditions
        // init_timeline_signal_handlers();  // ❌ Calls waveform_timeline_domain() before domains ready
        
        // TEMPORARILY DISABLED: Selected variables signal service bridge causing startup race conditions
        // init_selected_variables_signal_service_bridge();  // ❌ Calls variables_signal() before domains ready
        
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
                    let current_theme = config_store().ui.current_value().theme;
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
                            let mut ui = config_store().ui.current_value();
                            ui.theme = config_theme;
                            config_store().ui.set(ui);
                            
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
        
        // TEMPORARILY DISABLED: Query signal values when cursor movement stops
        // Disabled to prevent startup panics from domain access race conditions
        // Task::start(async {
        //     let was_moving = Mutable::new(false);
        //     
        //     // Listen to movement flags directly instead of cursor position changes
        //     let movement_signal = map_ref! {
        //         let left = crate::actors::waveform_timeline::is_cursor_moving_left_signal(),  // ❌ Calls domain before ready
        //         let right = crate::actors::waveform_timeline::is_cursor_moving_right_signal() =>  // ❌ Calls domain before ready
        //         *left || *right
        //     };
        //     
        //     movement_signal.for_each_sync(move |is_moving| {
        //         if is_moving {
        //             // Movement started - just track state, don't query
        //             was_moving.set(true);
        //         } else if was_moving.get() {
        //             // Movement just stopped - use unified caching logic with built-in range checking
        //             was_moving.set(false);
        //             crate::views::trigger_signal_value_queries();
        //         }
        //     }).await;
        // });
        
        // TEMPORARILY DISABLED: Direct cursor position handler causing startup race conditions
        // Task::start(async {
        //     let last_position = Mutable::new(0.0);
        //     
        //     waveform_timeline_domain().cursor_position_seconds_signal().for_each_sync(move |cursor_pos| {  // ❌ Calls domain before ready
        //         // TODO: Replace with domain signal when uncommenting
        //         let is_moving = crate::state::IS_CURSOR_MOVING_LEFT.get() || crate::state::IS_CURSOR_MOVING_RIGHT.get();
        //         
        //         // Only query for direct position changes (not during Q/E movement)
        //         if !is_moving && (cursor_pos - last_position.get()).abs() > 0.001 {
        //             // Use the unified caching logic with built-in range checking
        //             crate::views::trigger_signal_value_queries();
        //         }
        //         
        //         last_position.set(cursor_pos);
        //     }).await;
        // });
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
    let opened_files = config_store.session.current_value().opened_files.clone();
    
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
            if CONFIG_LOADED.get() && !is_dock_transitioning() {
                // Check if any divider is dragging before saving
                // This could be improved by making the actor handle config saving
                config::save_panel_layout();
            }
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

