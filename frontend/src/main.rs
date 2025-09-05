use zoon::{*, futures_util::future::try_join_all};
use moonzoon_novyui::tokens::theme::{Theme, init_theme};
use moonzoon_novyui::tokens::color::{neutral_1};
use crate::visualizer::timeline::time_types::TimeNs;

mod dataflow;
mod actors;

mod virtual_list;

mod debug_utils;

mod clipboard;

mod file_utils;
use file_utils::*;


// mod waveform_canvas; // MOVED to visualizer/canvas/waveform_canvas.rs


mod connection;
use connection::*;

mod platform;

mod config;
use shared;

// mod dragging; // MOVED to visualizer/interaction/dragging.rs

mod types;


mod visualizer;

use shared::{UpMsg};

mod views;
use views::*;

mod state;
use state::*;
use crate::visualizer::timeline::timeline_actor::{current_viewport};
use crate::visualizer::interaction::dragging::{
    files_panel_width_signal, files_panel_height_signal, 
    variables_name_column_width_signal, variables_value_column_width_signal,
    is_any_divider_dragging, active_divider_type_signal, process_drag_movement, DividerType
};
use config::app_config;
use actors::dialog_manager::{dialog_visible_signal, file_picker_selected_signal};


use crate::visualizer::timeline::timeline_service::{*, UnifiedTimelineService};

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
        // AppConfig initialized successfully
        
        // Initialize Actor+Relay domain instances BEFORE UI creation
        // This prevents canvas operations from triggering before domains are ready
        if let Err(error_msg) = crate::actors::initialize_all_domains().await {
            zoon::println!("🚨 DOMAIN INITIALIZATION FAILED: {}", error_msg);
            error_display::add_error_alert(crate::state::ErrorAlert {
                id: "domain_init_failure".to_string(),
                title: "Domain Initialization Failed".to_string(),
                message: format!("Critical startup error: {}", error_msg),
                technical_error: error_msg.to_string(),
                error_type: crate::state::ErrorType::ConfigError,
                timestamp: js_sys::Date::now() as u64,
                auto_dismiss_ms: 10000, // Critical errors get longer timeout
            });
            return; // Exit gracefully instead of panic
        }
        

        // ✅ RESTORE SELECTED VARIABLES FROM CONFIG (after domain initialization)
        let selected_variables = crate::state::SELECTED_VARIABLES_FOR_CONFIG.get_cloned();
        if !selected_variables.is_empty() {
            let variables_restored_relay = crate::actors::selected_variables::variables_restored_relay();
            variables_restored_relay.send(selected_variables);
        }

        // Start the app - domains are now guaranteed to be available for canvas operations
        start_app("app", root);

        // Initialize value caching - domains are already initialized
        crate::visualizer::timeline::timeline_actor::initialize_value_caching();
        
        // Initialize selected scope synchronization between UI and persistence
        crate::state::initialize_selected_scope_synchronization();
        
        // Initialize file picker directory browsing
        init_file_picker_handlers();
        
        // Initialize signal-based loading completion handling
        init_signal_chains();
        
        
        // Note: init_timeline_signal_handlers() and init_selected_variables_signal_service_bridge() 
        // functions do not exist yet - skipping
        // Timeline and variables signal bridge functions not implemented yet
        
        // Initialize error display system
        init_error_display_system();
        
        // Initialize unified timeline service with integer time architecture
        initialize_unified_timeline_service();
        
        // Reset circuit breakers for known variables to allow fresh requests to working backend
        let problematic_variables = vec![
            "/home/martinkavik/repos/NovyWave/test_files/simple.vcd|simple_tb.s|A".to_string(),
            "/home/martinkavik/repos/NovyWave/test_files/simple.vcd|simple_tb.s|B".to_string(),
            "/home/martinkavik/repos/NovyWave/test_files/nested_dir/wave_27.fst|TOP.VexiiRiscv|clk".to_string(),
        ];
        UnifiedTimelineService::reset_circuit_breakers_for_variables(&problematic_variables);
        
        // Reset circuit breakers and trigger queries when cursor moves
        Task::start(async {
            // Wait for cursor to move from initial position
            let cursor_signal = crate::visualizer::timeline::timeline_actor::cursor_position_signal();
            cursor_signal.for_each(move |cursor_pos| {
                async move {
                    if cursor_pos.nanos() > 0 {
                        // Cursor moved - triggering queries
                        
                        // Reset circuit breakers for all known variables
                        let problematic_variables = vec![
                            "/home/martinkavik/repos/NovyWave/test_files/simple.vcd|simple_tb.s|A".to_string(),
                            "/home/martinkavik/repos/NovyWave/test_files/simple.vcd|simple_tb.s|B".to_string(),
                            "/home/martinkavik/repos/NovyWave/test_files/nested_dir/wave_27.fst|TOP.VexiiRiscv|clk".to_string(),
                        ];
                        UnifiedTimelineService::reset_circuit_breakers_for_variables(&problematic_variables);
                        
                        // Trigger fresh queries at new cursor position
                        Timer::sleep(100).await; // Brief delay for reset to take effect
                        crate::views::trigger_signal_value_queries();
                        // Fresh queries triggered at cursor position
                    }
                }
            }).await;
        });
        
       
        
        
        
        // Query signal values when cursor movement stops (ENABLED - using domain signals)
        Task::start(async {
            let was_moving = Mutable::new(false);
            
            // Use domain signals instead of direct domain access
            let movement_signal = map_ref! {
                let left = crate::visualizer::timeline::timeline_actor::is_cursor_moving_left_signal(),
                let right = crate::visualizer::timeline::timeline_actor::is_cursor_moving_right_signal() =>
                *left || *right
            };
            
            movement_signal.for_each(move |is_moving| {
                let was_moving = was_moving.clone();
                async move {
                if is_moving {
                    // Movement started - just track state, don't query
                    was_moving.set(true);
                } else if was_moving.get() {
                    // Movement just stopped - use unified caching logic with built-in range checking
                    crate::views::trigger_signal_value_queries();
                    was_moving.set(false);
                }
                }
            }).await;
        });
        
        // Direct cursor position handler (ENABLED - using domain signals)
        Task::start(async {
            let last_position = Mutable::new(0.0);
            
            // Use domain signal instead of direct domain access - combined approach
            let movement_and_position_signal = map_ref! {
                let cursor_pos = crate::visualizer::timeline::timeline_actor::cursor_position_seconds_signal(),
                let left = crate::visualizer::timeline::timeline_actor::is_cursor_moving_left_signal(),
                let right = crate::visualizer::timeline::timeline_actor::is_cursor_moving_right_signal() =>
                (*cursor_pos, *left || *right)
            };
            
            movement_and_position_signal.for_each(move |(cursor_pos, is_moving)| {
                let last_position = last_position.clone();
                async move {
                // Only query for direct position changes (not during Q/E movement)
                if !is_moving && (cursor_pos - last_position.get()).abs() > 0.001 {
                    // Use the unified caching logic with built-in range checking
                    crate::views::trigger_signal_value_queries();
                }
                
                last_position.set(cursor_pos);
                }
            }).await;
        });
    });
}

// Helper functions for optimized variable value updates

/// Check if cursor is within the currently visible timeline range
pub fn is_cursor_in_visible_range(cursor_time: f64) -> bool {
    match current_viewport() {
        Some(viewport) => {
            let cursor_ns = crate::visualizer::timeline::time_types::TimeNs::from_nanos((cursor_time * 1_000_000_000.0) as u64);
            viewport.contains(cursor_ns)
        }
        None => false // If viewport not initialized, cursor is not in visible range
    }
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
    
    // Watch for directory expansions in the file picker dialog (for backend loading)
    Task::start(async {
        let config = crate::config::app_config();
        
        config.file_picker_expanded_directories.signal_cloned().for_each(|expanded_set| async move {
            let expanded_hash: std::collections::HashSet<String> = expanded_set.iter().cloned().collect();
            crate::views::monitor_directory_expansions(expanded_hash);
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
    
    // Config initialization complete - loaded with await in main
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


fn drag_overlay() -> impl Element {
    // Full-screen transparent overlay for consistent drag handling
    // Similar to Load Files dialog overlay but transparent and for drag operations
    El::new()
        .s(Background::new().color("rgba(0, 0, 0, 0)")) // Completely transparent
        .s(Width::fill())
        .s(Height::fill())
        .s(Align::center())
        .update_raw_el(|raw_el| {
            raw_el
                .style("position", "fixed")
                .style("top", "0")
                .style("left", "0") 
                .style("z-index", "999") // High z-index to capture events
                .style("pointer-events", "auto") // Ensure it captures mouse events
        })
        .on_pointer_move_event({
            let last_x = Mutable::new(0.0f32);
            let last_y = Mutable::new(0.0f32);
            let is_first_move = Mutable::new(true);
            
            move |event| {
                let current_x = event.x() as f32;
                let current_y = event.y() as f32;
                
                if is_first_move.get() {
                    // First move - just store position, don't send delta
                    last_x.set_neq(current_x);
                    last_y.set_neq(current_y);
                    is_first_move.set_neq(false);
                } else {
                    // Calculate deltas from absolute coordinates
                    let delta_x = current_x - last_x.get();
                    let delta_y = current_y - last_y.get();
                    
                    // Send absolute position to new dragging system
                    process_drag_movement((current_x, current_y));
                    
                    // Update last position for next delta calculation
                    last_x.set_neq(current_x);
                    last_y.set_neq(current_y);
                    
                }
            }
        })
        .on_pointer_up(|| {
            crate::visualizer::interaction::dragging::end_drag();
        })
        .on_pointer_leave(|| {
            crate::visualizer::interaction::dragging::end_drag();
        })
}

fn main_layout() -> impl Element {
    let is_any_divider_dragging_1 = is_any_divider_dragging();
    let is_any_divider_dragging_2 = is_any_divider_dragging();

    El::new()
        .s(Height::screen())
        .s(Width::fill())
        // TEST 3: Remove root container scrollbars
        .text_content_selecting_signal(
            is_any_divider_dragging_1.map(|is_dragging| {
                if is_dragging {
                    TextContentSelecting::none()
                } else {
                    TextContentSelecting::auto()
                }
            })
        )
        .s(Cursor::with_signal(
            active_divider_type_signal().map(|active_divider| {
                match active_divider {
                    Some(DividerType::FilesPanelSecondary) => Some(CursorIcon::RowResize), // Vertical resize for height
                    Some(DividerType::FilesPanelMain) => Some(CursorIcon::ColumnResize), // Horizontal resize for width
                    Some(DividerType::VariablesNameColumn) => Some(CursorIcon::ColumnResize), // Horizontal resize for column width
                    Some(DividerType::VariablesValueColumn) => Some(CursorIcon::ColumnResize), // Horizontal resize for column width
                    None => None, // No dragging - default cursor
                }
            })
        ))
        .on_pointer_up(|| {
            crate::visualizer::interaction::dragging::end_drag();
        })
        .on_pointer_leave(|| {
            crate::visualizer::interaction::dragging::end_drag();
        })
        .update_raw_el(move |raw_el| {
            raw_el.global_event_handler(move |event: zoon::events::KeyDown| {
                // CRITICAL DEBUG: Check if search input focus is blocking keyboard events
                let search_focused = state::VARIABLES_SEARCH_INPUT_FOCUSED.get();
                // Keyboard event captured
                
                // Skip timeline controls if typing in search input
                if search_focused {
                    // Keyboard blocked: search input has focus
                    return;
                }
                
                match event.key().as_str() {
                    "Shift" => {
                        // Track Shift key state
                        crate::visualizer::state::timeline_state::IS_SHIFT_PRESSED.set_neq(true);
                    },
                    "w" | "W" => {
                        // W key: zoom in pressed
                        
                        // Zoom in using WaveformTimeline domain
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        waveform_timeline.zoom_in_pressed_relay.send(());
                        waveform_timeline.redraw_requested_relay.send(()); // ✅ Trigger rerender like Z key
                        
                        // Zoom in completed
                        
                        // ✅ FIXED: Removed legacy canvas call that was changing viewport range
                        // Legacy function removed: crate::visualizer::canvas::waveform_canvas::start_smooth_zoom_in();
                    },
                    "s" | "S" => {
                        // S key: zoom out pressed
                        
                        // Zoom out using WaveformTimeline domain
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        waveform_timeline.zoom_out_pressed_relay.send(());
                        waveform_timeline.redraw_requested_relay.send(()); // ✅ Trigger rerender like Z key
                        
                        // Zoom out completed
                        
                        // ✅ FIXED: Removed legacy canvas call that was changing viewport range  
                        // Legacy function removed: crate::visualizer::canvas::waveform_canvas::start_smooth_zoom_out();
                    },
                    "a" | "A" => {
                        // Pan left using WaveformTimeline domain
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        waveform_timeline.pan_left_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::visualizer::canvas::waveform_canvas::start_smooth_pan_left();
                    },
                    "d" | "D" => {
                        // Pan right using WaveformTimeline domain
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        waveform_timeline.pan_right_pressed_relay.send(());
                        
                        // Legacy function call for backward compatibility (will be removed)
                        crate::visualizer::canvas::waveform_canvas::start_smooth_pan_right();
                    },
                    "q" | "Q" => {
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        if crate::visualizer::state::timeline_state::IS_SHIFT_PRESSED.get() {
                            // Shift+Q: Jump to previous transition using WaveformTimeline domain
                            waveform_timeline.jump_to_previous_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::visualizer::canvas::waveform_canvas::jump_to_previous_transition();
                        } else {
                            // Q: Cursor left using WaveformTimeline domain
                            waveform_timeline.left_key_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::visualizer::canvas::waveform_canvas::start_smooth_cursor_left();
                        }
                    },
                    "e" | "E" => {
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        if crate::visualizer::state::timeline_state::IS_SHIFT_PRESSED.get() {
                            // Shift+E: Jump to next transition using WaveformTimeline domain
                            waveform_timeline.jump_to_next_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::visualizer::canvas::waveform_canvas::jump_to_next_transition();
                        } else {
                            // E: Cursor right using WaveformTimeline domain
                            waveform_timeline.right_key_pressed_relay.send(());
                            
                            // Legacy function call for backward compatibility (will be removed)
                            crate::visualizer::canvas::waveform_canvas::start_smooth_cursor_right();
                        }
                    },
                    "r" | "R" => {
                        // R key: reset zoom pressed
                        
                        // R: Reset zoom using WaveformTimeline domain only
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        waveform_timeline.reset_zoom_pressed_relay.send(());
                        
                        // Reset zoom completed
                    },
                    "z" | "Z" => {
                        // Z: Move zoom center to 0 (keep cursor position unchanged)
                        let waveform_timeline = crate::visualizer::timeline::timeline_actor_domain();
                        
                        // ✅ CORRECT FIX: Reset ONLY zoom center to 0, do NOT move cursor
                        // Z key: zoom center reset to 0
                        waveform_timeline.zoom_center_reset_to_zero_relay.send(());
                        
                        // Trigger timeline rerender to refresh the display
                        waveform_timeline.redraw_requested_relay.send(());
                        // Zoom center reset completed
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
                        crate::visualizer::state::timeline_state::IS_SHIFT_PRESSED.set_neq(false);
                    },
                    "w" | "W" => {
                        // ✅ FIXED: No legacy zoom animation to stop - domain actor handles zoom instantly
                        // Removed: crate::visualizer::canvas::waveform_canvas::stop_smooth_zoom_in();
                    },
                    "s" | "S" => {
                        // ✅ FIXED: No legacy zoom animation to stop - domain actor handles zoom instantly
                        // Removed: crate::visualizer::canvas::waveform_canvas::stop_smooth_zoom_out();
                    },
                    "a" | "A" => {
                        // Stop smooth pan left
                        crate::visualizer::canvas::waveform_canvas::stop_smooth_pan_left();
                    },
                    "d" | "D" => {
                        // Stop smooth pan right
                        crate::visualizer::canvas::waveform_canvas::stop_smooth_pan_right();
                    },
                    "q" | "Q" => {
                        // Always stop smooth cursor when Q is released 
                        // (Shift+Q is instantaneous, normal Q is continuous)
                        crate::visualizer::canvas::waveform_canvas::stop_smooth_cursor_left();
                    },
                    "e" | "E" => {
                        // Always stop smooth cursor when E is released
                        // (Shift+E is instantaneous, normal E is continuous) 
                        crate::visualizer::canvas::waveform_canvas::stop_smooth_cursor_right();
                    },
                    _ => {} // Ignore other keys
                }
            })
        })
        .child(layout_with_drag_overlay())
}

// Layout with conditional drag overlay
fn layout_with_drag_overlay() -> impl Element {
    let is_any_divider_dragging = is_any_divider_dragging();

    // Use Stack to layer the overlay over the main layout
    Stack::new()
        .s(Height::fill())
        .s(Width::fill())
        .layer(docked_layout_wrapper()) // Base layer: main layout
        .layer_signal(is_any_divider_dragging.map(|is_dragging| {
            if is_dragging {
                Some(drag_overlay())
            } else {
                None
            }
        })) // Overlay layer: drag overlay when dragging
}

// Wrapper function that switches between docked and undocked layouts
fn docked_layout_wrapper() -> impl Element {
    El::new()
        .s(Height::screen())
        .s(Width::fill())
        // TEST 3: Remove root container scrollbars
        .child_signal(crate::config::app_config().dock_mode_actor.signal().map(|dock_mode| {
            let is_docked = matches!(dock_mode, shared::DockMode::Bottom);
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
                                    .s(Height::exact_signal(files_panel_height_signal().map(|h| h as u32)))
                                    .s(Width::fill())
                                    .item(
                                        El::new()
                                            .s(Width::exact_signal(files_panel_width_signal().map(|w| w as u32)))
                                            .s(Height::fill())
                                            .child(files_panel_with_height())
                                    )
                                    .item(views::files_panel_vertical_divider())
                                    .item(
                                        El::new()
                                            .s(Width::fill())
                                            .s(Height::fill())
                                            .child(variables_panel_with_fill())
                                    )
                            )
                            .item(views::files_panel_horizontal_divider())
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
                                    .s(Width::exact_signal(files_panel_width_signal().map(|w| w as u32)))
                                    .s(Height::fill())
                                    .child(
                                        Column::new()
                                            .s(Height::fill())
                                            .item(files_panel_with_height())
                                            .item(views::files_panel_horizontal_divider())
                                            .item(variables_panel_with_fill())
                                    )
                            )
                            .item(views::files_panel_vertical_divider())
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

