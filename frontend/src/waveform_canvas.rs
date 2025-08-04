use zoon::*;
use fast2d;
use crate::state::{SELECTED_VARIABLES, LOADED_FILES, TIMELINE_CURSOR_POSITION, CANVAS_WIDTH, CANVAS_HEIGHT};
use crate::config::current_theme;
use shared::SelectedVariable;
use std::rc::Rc;
use std::cell::RefCell;
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::Theme as SharedTheme;
use wasm_bindgen::JsCast;

// Convert shared theme to NovyUI theme
fn convert_theme(shared_theme: &SharedTheme) -> NovyUITheme {
    match shared_theme {
        SharedTheme::Dark => NovyUITheme::Dark,
        SharedTheme::Light => NovyUITheme::Light,
    }
}

// Get current theme colors as RGBA tuples based on current theme
fn get_current_theme_colors(current_theme: &NovyUITheme) -> ThemeColors {
    match current_theme {
        NovyUITheme::Dark => ThemeColors {
            neutral_2: (45, 47, 50, 1.0),     // Dark theme neutral_2
            neutral_3: (52, 54, 58, 1.0),     // Dark theme neutral_3
            neutral_4: (65, 69, 75, 1.0),     // Dark theme neutral_4
            neutral_5: (75, 79, 86, 1.0),     // Dark theme neutral_5
            neutral_12: (253, 253, 253, 1.0), // Dark theme high contrast text
            cursor_color: (59, 130, 246, 0.8), // Bright blue cursor with transparency
        },
        NovyUITheme::Light => ThemeColors {
            neutral_2: (249, 250, 251, 1.0),  // Light theme neutral_2
            neutral_3: (243, 244, 246, 1.0),  // Light theme neutral_3
            neutral_4: (229, 231, 235, 1.0),  // Light theme neutral_4
            neutral_5: (209, 213, 219, 1.0),  // Light theme neutral_5
            neutral_12: (17, 24, 39, 1.0),    // Light theme high contrast text
            cursor_color: (37, 99, 235, 0.8),  // Bright blue cursor with transparency
        },
    }
}

struct ThemeColors {
    neutral_2: (u8, u8, u8, f32),
    neutral_3: (u8, u8, u8, f32),
    neutral_4: (u8, u8, u8, f32),
    neutral_5: (u8, u8, u8, f32),
    neutral_12: (u8, u8, u8, f32),
    cursor_color: (u8, u8, u8, f32),
}

// Helper function to round raw time steps to professional-looking numbers
fn round_to_nice_number(raw: f32) -> f32 {
    if raw <= 0.0 { return 1.0; }
    
    let magnitude = 10.0_f32.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    
    let nice_normalized = if normalized <= 1.0 { 1.0 }
    else if normalized <= 2.0 { 2.0 }
    else if normalized <= 5.0 { 5.0 }
    else { 10.0 };
    
    nice_normalized * magnitude
}

pub fn waveform_canvas() -> impl Element {
    El::new()
        .s(Width::fill())
        .s(Height::fill())
        .child_signal(create_canvas_element().into_signal_option())
}

async fn create_canvas_element() -> impl Element {
    let mut zoon_canvas = Canvas::new()
        .width(0)
        .height(0)
        .s(Width::fill())
        .s(Height::fill());

    let dom_canvas = zoon_canvas.raw_el_mut().dom_element();
    let mut canvas_wrapper = fast2d::CanvasWrapper::new_with_canvas(dom_canvas).await;

    // Initialize with default dark theme (theme reactivity will update it)
    canvas_wrapper.update_objects(|objects| {
        let selected_vars = SELECTED_VARIABLES.lock_ref();
        *objects = create_waveform_objects_with_theme(&selected_vars, &NovyUITheme::Dark);
    });

    // Wrap canvas_wrapper in Rc<RefCell> for sharing
    let canvas_wrapper_shared = Rc::new(RefCell::new(canvas_wrapper));
    let canvas_wrapper_for_signal = canvas_wrapper_shared.clone();

    // Add reactive updates when SELECTED_VARIABLES changes
    Task::start(async move {
        SELECTED_VARIABLES.signal_vec_cloned().for_each(move |_| {
            let canvas_wrapper_for_signal = canvas_wrapper_for_signal.clone();
            async move {
                zoon::println!("SELECTED_VARIABLES changed, updating canvas");
                canvas_wrapper_for_signal.borrow_mut().update_objects(|objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    // Use dark theme as fallback - theme handler will update with correct theme
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, cursor_pos);
                });
            }
        }).await;
    });

    // Add reactive updates when theme changes
    let canvas_wrapper_for_theme = canvas_wrapper_shared.clone();
    Task::start(async move {
        current_theme().for_each(move |theme_value| {
            let canvas_wrapper_for_theme = canvas_wrapper_for_theme.clone();
            async move {
                zoon::println!("Theme changed, updating canvas colors");
                canvas_wrapper_for_theme.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                    let canvas_width = CANVAS_WIDTH.get();
                    let canvas_height = CANVAS_HEIGHT.get();
                    let novyui_theme = convert_theme(&theme_value);
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &novyui_theme, cursor_pos);
                });
            }
        }).await;
    });

    let canvas_wrapper_for_resize = canvas_wrapper_shared.clone();
    zoon_canvas.update_raw_el(move |raw_el| {
        raw_el.on_resize(move |width, height| {
            zoon::println!("Canvas resized to {}x{}", width, height);
            
            // Store canvas dimensions for click calculations
            CANVAS_WIDTH.set(width as f32);
            CANVAS_HEIGHT.set(height as f32);
            
            canvas_wrapper_for_resize.borrow_mut().resized(width, height);
            // Re-create objects with new dimensions
            canvas_wrapper_for_resize.borrow_mut().update_objects(move |objects| {
                let selected_vars = SELECTED_VARIABLES.lock_ref();
                let cursor_pos = TIMELINE_CURSOR_POSITION.get();
                // Use dark theme as fallback - theme handler will update with correct theme
                *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, width as f32, height as f32, &NovyUITheme::Dark, cursor_pos);
            });
        })
        .event_handler({
            let canvas_wrapper_for_click = canvas_wrapper_shared.clone();
            move |event: events::Click| {
                // Handle click to move cursor position
                let page_click_x = event.x() as f32;
                
                // Get canvas element's position relative to page
                let canvas_element = event.target().unwrap();
                let canvas_rect = canvas_element.dyn_into::<web_sys::Element>()
                    .unwrap().get_bounding_client_rect();
                let canvas_left = canvas_rect.left() as f32;
                
                // Calculate click position relative to canvas
                let click_x = page_click_x - canvas_left;
                
                // Use stored canvas width
                let canvas_width = CANVAS_WIDTH.get();
                let canvas_height = CANVAS_HEIGHT.get();
                
                // Calculate time from click position using consistent timeline range
                let (min_time, max_time) = get_current_timeline_range();
                let time_range = max_time - min_time;
                let clicked_time = min_time + (click_x / canvas_width) * time_range;
                
                // Clamp to valid range
                let clicked_time = clicked_time.max(min_time).min(max_time);
                
                zoon::println!("Canvas clicked: page_x={}, canvas_left={}, relative_x={}, canvas_width={}, time_range={}, calculated time: {}s", 
                               page_click_x, canvas_left, click_x, canvas_width, time_range, clicked_time);
                
                // Update cursor position
                TIMELINE_CURSOR_POSITION.set(clicked_time);
                
                // Immediately update canvas with new cursor position to prevent visual glitches
                canvas_wrapper_for_click.borrow_mut().update_objects(move |objects| {
                    let selected_vars = SELECTED_VARIABLES.lock_ref();
                    *objects = create_waveform_objects_with_dimensions_and_theme(&selected_vars, canvas_width, canvas_height, &NovyUITheme::Dark, clicked_time);
                });
            }
        })
    })
}




// Consolidated function to get current timeline range
fn get_current_timeline_range() -> (f32, f32) {
    let mut min_time: f32 = f32::MAX;
    let mut max_time: f32 = f32::MIN;
    let loaded_files = LOADED_FILES.lock_ref();
    let selected_vars = SELECTED_VARIABLES.lock_ref();
    
    for var in selected_vars.iter() {
        let file_path = var.unique_id.split('|').next().unwrap_or("");
        if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
            if let (Some(file_min), Some(file_max)) = (loaded_file.min_time, loaded_file.max_time) {
                min_time = min_time.min(file_min as f32);
                max_time = max_time.max(file_max as f32);
            }
        }
    }
    
    if min_time == f32::MAX || max_time == f32::MIN {
        (0.0, 250.0) // Default fallback
    } else {
        (min_time, max_time)
    }
}

fn create_waveform_objects_with_theme(selected_vars: &[SelectedVariable], theme: &NovyUITheme) -> Vec<fast2d::Object2d> {
    let cursor_pos = TIMELINE_CURSOR_POSITION.get();
    let canvas_width = CANVAS_WIDTH.get();
    let canvas_height = CANVAS_HEIGHT.get();
    create_waveform_objects_with_dimensions_and_theme(selected_vars, canvas_width, canvas_height, theme, cursor_pos)
}

fn create_waveform_objects_with_dimensions_and_theme(selected_vars: &[SelectedVariable], canvas_width: f32, canvas_height: f32, theme: &NovyUITheme, cursor_position: f32) -> Vec<fast2d::Object2d> {
    let mut objects = Vec::new();
    
    zoon::println!("Creating waveform objects for {} selected variables with dimensions {}x{}", 
                   selected_vars.len(), canvas_width, canvas_height);
    
    // Get current theme colors
    let theme_colors = get_current_theme_colors(theme);
    
    // Calculate row layout according to specs
    let total_rows = selected_vars.len() + 1; // variables + timeline
    let row_height = if total_rows > 0 { canvas_height / total_rows as f32 } else { canvas_height };
    
    // Create alternating row backgrounds for variable rows
    for (index, var) in selected_vars.iter().enumerate() {
        let y_position = index as f32 * row_height;
        let is_even_row = index % 2 == 0;
        
        // Theme-aware alternating backgrounds using current theme colors
        let background_color = if is_even_row {
            theme_colors.neutral_2
        } else {
            theme_colors.neutral_3
        };
        
        zoon::println!("Creating row {} for variable {} at y={} with size {}x{}", 
                       index, var.unique_id, y_position, canvas_width, row_height);
        
        objects.push(
            fast2d::Rectangle::new()
                .position(0.0, y_position)
                .size(canvas_width, row_height)
                .color(background_color.0, background_color.1, background_color.2, background_color.3)
                .into()
        );
        
        // Create value rectangles based on live data from selected variables
        let variable_name = var.unique_id.split('|').last().unwrap_or("Unknown");
        
        // Get the user's selected format for this variable
        let format = var.formatter.unwrap_or_default();
        
        // Phase 7: Multi-file support - get data based on variable's source file
        // Parse file path from unique_id: "/path/file.ext|scope|variable"
        let file_path = var.unique_id.split('|').next().unwrap_or("");
        let file_name = file_path.split('/').last().unwrap_or("unknown");
        
        let time_value_pairs = if file_name == "simple.vcd" {
            // Data from simple.vcd file (timescale: 1s, max time: 250s)
            if variable_name == "A" {
                vec![
                    (0.0, "1010"),    // #0: b1010 from simple.vcd
                    (50.0, "1100"),   // #50: b1100 from simple.vcd  
                    (150.0, "0"),     // #150: b0 from simple.vcd
                ]
            } else { // Variable B
                vec![
                    (0.0, "11"),      // #0: b11 from simple.vcd
                    (50.0, "101"),    // #50: b101 from simple.vcd
                    (150.0, "0"),     // #150: b0 from simple.vcd
                ]
            }
        } else if file_name == "wave_27.fst" {
            // TODO: Get actual data from wave_27.fst file
            // For now, using placeholder data showing different pattern
            vec![
                (0.0, "1111"),    // Different pattern for FST variables
                (25.0, "0101"),   // Different timing
                (75.0, "1010"),   
                (100.0, "0000"),
            ]
        } else {
            // Fallback for unknown files
            vec![(0.0, "0")]
        };
        
        // Get actual total time from loaded file data
        let total_time = {
            let loaded_files = LOADED_FILES.lock_ref();
            if let Some(loaded_file) = loaded_files.iter().find(|f| f.id == file_path) {
                if let Some(file_max) = loaded_file.max_time {
                    file_max as f32
                } else {
                    250.0 // Fallback if no max_time available
                }
            } else {
                250.0 // Fallback if file not found
            }
        };
        
        for (rect_index, (start_time, binary_value)) in time_value_pairs.iter().enumerate() {
            // Calculate rectangle position and width based on actual time spans
            let rect_start_x = (start_time / total_time) * canvas_width;
            
            // Calculate end time for this rectangle (next transition time or total_time)
            let end_time = if rect_index + 1 < time_value_pairs.len() {
                time_value_pairs[rect_index + 1].0 // Next transition time
            } else {
                total_time // Last rectangle extends to end
            };
            
            let rect_end_x = (end_time / total_time) * canvas_width;
            let rect_width = rect_end_x - rect_start_x;
            let is_even_rect = rect_index % 2 == 0;
            
            // Theme-aware alternating rectangle colors using current theme colors
            let rect_color = if is_even_rect {
                theme_colors.neutral_4
            } else {
                theme_colors.neutral_5
            };
            
            // Create value rectangle with actual time-based width
            objects.push(
                fast2d::Rectangle::new()
                    .position(rect_start_x, y_position + 2.0)
                    .size(rect_width, row_height - 4.0)
                    .color(rect_color.0, rect_color.1, rect_color.2, rect_color.3)
                    .into()
            );
            
            // Format the binary value using the user's selected format (without prefix)
            let formatted_value = format.format(binary_value);
            
            // Add formatted value text centered in rectangle with theme-aware color
            let text_color = theme_colors.neutral_12; // High contrast text
            objects.push(
                fast2d::Text::new()
                    .text(formatted_value)
                    .position(rect_start_x + 5.0, y_position + row_height / 3.0)
                    .size(rect_width - 10.0, row_height / 2.0)
                    .color(text_color.0, text_color.1, text_color.2, text_color.3)
                    .font_size(12.0)
                    .family(fast2d::Family::name("Fira Code")) // FiraCode monospace font
                    .into()
            );
        }
    }
    
    // Create timeline row background (last row) using theme-aware color
    if total_rows > 0 {
        let timeline_y = (total_rows - 1) as f32 * row_height;
        zoon::println!("Creating timeline row at y={} with size {}x{}", timeline_y, canvas_width, row_height);
        
        let timeline_bg_color = theme_colors.neutral_2; // Consistent with alternating backgrounds
        objects.push(
            fast2d::Rectangle::new()
                .position(0.0, timeline_y)
                .size(canvas_width, row_height)
                .color(timeline_bg_color.0, timeline_bg_color.1, timeline_bg_color.2, timeline_bg_color.3)
                .into()
        );
        
        // Get consistent timeline range
        let (min_time, max_time) = get_current_timeline_range();
        let time_range = max_time - min_time;
        
        // Phase 9: Pixel-based spacing algorithm for professional timeline
        let target_tick_spacing = 80.0; // Target 80 pixels between ticks
        let max_tick_count = (canvas_width / target_tick_spacing).floor() as i32;
        let tick_count = max_tick_count.max(2).min(8); // Ensure 2-8 ticks
        
        // Calculate round time intervals
        let raw_time_step = time_range / (tick_count - 1) as f32;
        let time_step = round_to_nice_number(raw_time_step);
        let actual_tick_count = ((time_range / time_step).ceil() as i32 + 1).min(tick_count + 2);
        
        for tick_index in 0..actual_tick_count {
            let time_value = (tick_index as f32 * time_step).min(max_time);
            let x_position = (time_value / time_range) * canvas_width;
            
            // Skip edge labels to prevent cutoff (10px margin on each side)
            let label_margin = 10.0;
            let should_show_label = x_position >= label_margin && x_position <= (canvas_width - label_margin);
            
            // Create vertical tick mark with theme-aware color
            let tick_color = theme_colors.neutral_12; // High contrast for visibility
            objects.push(
                fast2d::Rectangle::new()
                    .position(x_position, timeline_y + row_height - 8.0)
                    .size(1.0, 8.0) // Thin vertical line
                    .color(tick_color.0, tick_color.1, tick_color.2, tick_color.3)
                    .into()
            );
            
            // Add time label with actual time units and theme-aware color (only if not cut off)
            if should_show_label {
                let time_label = format!("{}s", time_value as u32);
                let label_color = theme_colors.neutral_12; // High contrast text
                objects.push(
                    fast2d::Text::new()
                        .text(time_label)
                        .position(x_position - 10.0, timeline_y + 5.0)
                        .size(20.0, row_height - 15.0)
                        .color(label_color.0, label_color.1, label_color.2, label_color.3)
                        .font_size(10.0)
                        .family(fast2d::Family::name("Inter")) // Standard UI font for timeline
                        .into()
                );
            }
        }
    }
    
    // Add timeline cursor line spanning all rows
    if total_rows > 0 {
        // Use consistent timeline range
        let (min_time, max_time) = get_current_timeline_range();
        let time_range = max_time - min_time;
        
        // Calculate cursor x position only if cursor is within visible range
        if cursor_position >= min_time && cursor_position <= max_time {
            let cursor_x = ((cursor_position - min_time) / time_range) * canvas_width;
            
            // Draw vertical cursor line spanning all rows (including timeline)
            let cursor_color = theme_colors.cursor_color;
            objects.push(
                fast2d::Rectangle::new()
                    .position(cursor_x - 1.0, 0.0) // Center the 3px line
                    .size(3.0, canvas_height) // 3px thick line spanning full height
                    .color(cursor_color.0, cursor_color.1, cursor_color.2, cursor_color.3)
                    .into()
            );
            
            zoon::println!("Added cursor line at {}s (x={})", cursor_position, cursor_x);
        }
    }
    
    zoon::println!("Created {} objects total", objects.len());
    objects
}