use crate::dataflow::*;
use fast2d::{CanvasWrapper as Fast2DCanvas, Family, Object2d, Rectangle, Text};
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use zoon::*;
use futures::{select, stream::StreamExt};
use shared::{SelectedVariable, SignalValue};

#[derive(Debug, Clone, Copy)]
enum TimeUnit {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
}

impl TimeUnit {
    fn suffix(&self) -> &'static str {
        match self {
            TimeUnit::Nanosecond => "ns",
            TimeUnit::Microsecond => "μs",
            TimeUnit::Millisecond => "ms",
            TimeUnit::Second => "s",
        }
    }

    fn scale_factor(&self) -> f32 {
        match self {
            TimeUnit::Nanosecond => 1e9,
            TimeUnit::Microsecond => 1e6,
            TimeUnit::Millisecond => 1e3,
            TimeUnit::Second => 1.0,
        }
    }
}

#[derive(Clone, Debug)]
struct ThemeColors {
    neutral_2: (u8, u8, u8, f32),
    neutral_3: (u8, u8, u8, f32),
    neutral_12: (u8, u8, u8, f32),
    grid_color: (u8, u8, u8, f32),
    separator_color: (u8, u8, u8, f32),
    cursor_color: (u8, u8, u8, f32),
    value_color_1: (u8, u8, u8, f32),
    value_color_2: (u8, u8, u8, f32),
}

#[derive(Clone, Debug)]
pub struct RenderingParameters {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub viewport_start: f64,
    pub viewport_end: f64,
    pub cursor_position: Option<f64>,
    pub zoom_center_position: Option<f64>,
    pub theme: NovyUITheme,
    pub selected_variables: Vec<SelectedVariable>,
}

pub struct WaveformRenderer {
    pub rendering_state: Actor<RenderingState>,
    pub render_requested_relay: Relay<RenderingParameters>,
    pub render_completed_relay: Relay<RenderResult>,
    
    canvas: Option<Fast2DCanvas>,
}

impl Clone for WaveformRenderer {
    fn clone(&self) -> Self {
        // Canvas cannot be cloned (GPU resource), so we create a new instance without canvas
        // The canvas will be set later via set_canvas method
        Self {
            rendering_state: self.rendering_state.clone(),
            render_requested_relay: self.render_requested_relay.clone(),
            render_completed_relay: self.render_completed_relay.clone(),
            canvas: None,
        }
    }
}

// SAFETY: WaveformRenderer is only used on the main thread where GPU resources are accessed
// The Fast2DCanvas contains raw pointers but is only used in single-threaded WASM context
unsafe impl Send for WaveformRenderer {}
unsafe impl Sync for WaveformRenderer {}

#[derive(Clone, Debug)]
struct RenderingState {
    pub last_render_params: Option<RenderingParameters>,
    pub render_count: u32,
}

impl Default for RenderingState {
    fn default() -> Self {
        Self {
            last_render_params: None,
            render_count: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderResult {
    pub render_count: u32,
    pub objects_rendered: usize,
    pub rendering_time_ms: f32,
}

impl WaveformRenderer {
    pub async fn new() -> Self {
        let (render_requested_relay, mut render_requested_stream) = relay();
        let (render_completed_relay, _render_completed_stream) = relay();
        
        let render_completed_relay_for_actor = render_completed_relay.clone();
        let rendering_state = Actor::new(RenderingState::default(), async move |state| {
            loop {
                select! {
                    params_result = render_requested_stream.next() => {
                        if let Some(params) = params_result {
                            let start_time = Self::get_current_time_ms();
                            
                            let objects = Self::build_render_objects(&params);
                            let render_time = Self::get_current_time_ms() - start_time;
                            
                            let mut current_state = state.lock_mut();
                            current_state.render_count += 1;
                            current_state.last_render_params = Some(params);
                            let render_count = current_state.render_count;
                            drop(current_state);
                            
                            render_completed_relay_for_actor.send(RenderResult {
                                render_count,
                                objects_rendered: objects.len(),
                                rendering_time_ms: render_time,
                            });
                        }
                    }
                }
            }
        });
        
        Self {
            rendering_state,
            render_requested_relay,
            render_completed_relay,
            canvas: None,
        }
    }
    
    pub fn set_canvas(&mut self, canvas: Fast2DCanvas) {
        self.canvas = Some(canvas);
    }
    
    pub fn has_canvas(&self) -> bool {
        self.canvas.is_some()
    }
    
    /// Set the theme for the renderer (placeholder for future theme handling)
    pub fn set_theme(&mut self, _theme: NovyUITheme) {
        // Theme will be passed through RenderingParameters when rendering
        // This method exists for API compatibility
    }
    
    /// Set the canvas dimensions (placeholder for future dimension handling)
    pub fn set_dimensions(&mut self, _width: f32, _height: f32) {
        // Dimensions will be passed through RenderingParameters when rendering
        // This method exists for API compatibility
    }
    
    pub fn render_frame(&mut self, params: RenderingParameters) -> bool {
        if let Some(canvas) = &mut self.canvas {
            let objects = Self::build_render_objects(&params);
            
            canvas.update_objects(|canvas_objects| {
                canvas_objects.clear();
                canvas_objects.extend(objects);
            });
            
            self.render_requested_relay.send(params);
            true
        } else {
            false
        }
    }
    
    fn build_render_objects(params: &RenderingParameters) -> Vec<Object2d> {
        if params.canvas_width == 0 || params.canvas_height == 0 {
            return Vec::new();
        }
        
        let mut objects = Vec::new();
        let theme_colors = Self::get_theme_colors(params.theme);
        
        Self::add_waveforms(&mut objects, params, &theme_colors);
        Self::add_cursor_lines(&mut objects, params, &theme_colors);
        
        objects
    }
    
    fn add_waveforms(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        let variables = &params.selected_variables;
        if variables.is_empty() {
            return;
        }
        
        let time_range = params.viewport_end - params.viewport_start;
        if time_range <= 0.0 {
            return;
        }
        
        let total_rows = variables.len() + 1;
        let row_height = (params.canvas_height as f32 - 5.0) / total_rows as f32;
        
        for (index, variable) in variables.iter().enumerate() {
            let y_position = index as f32 * row_height;
            let is_even_row = index % 2 == 0;
            
            let background_color = if is_even_row {
                theme_colors.neutral_2
            } else {
                theme_colors.neutral_3
            };
            
            objects.push(
                Rectangle::new()
                    .position(0.0, y_position)
                    .size(params.canvas_width as f32, row_height)
                    .color(
                        background_color.0,
                        background_color.1,
                        background_color.2,
                        background_color.3,
                    )
                    .into(),
            );
            
            Self::add_signal_blocks(
                objects,
                variable,
                y_position,
                row_height,
                params,
                theme_colors,
            );
            
            if index < variables.len() - 1 {
                let separator_y = (index + 1) as f32 * row_height;
                objects.push(
                    Rectangle::new()
                        .position(0.0, separator_y - 0.5)
                        .size(params.canvas_width as f32, 1.0)
                        .color(
                            theme_colors.separator_color.0,
                            theme_colors.separator_color.1,
                            theme_colors.separator_color.2,
                            theme_colors.separator_color.3,
                        )
                        .into(),
                );
            }
        }
        
        Self::add_timeline_row(objects, params, theme_colors, row_height, total_rows);
    }
    
    fn add_signal_blocks(
        objects: &mut Vec<Object2d>,
        variable: &SelectedVariable,
        y_position: f32,
        row_height: f32,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        let transitions = Self::get_variable_transitions(variable, params);
        let time_range = params.viewport_end - params.viewport_start;
        
        for (rect_index, (start_time, signal_value)) in transitions.iter().enumerate() {
            let end_time = if rect_index + 1 < transitions.len() {
                transitions[rect_index + 1].0.min(params.viewport_end as f32)
            } else {
                params.viewport_end as f32
            };
            
            if end_time <= params.viewport_start as f32 || *start_time >= params.viewport_end as f32 {
                continue;
            }
            
            let visible_start = start_time.max(params.viewport_start as f32);
            let visible_end = end_time.min(params.viewport_end as f32);
            
            let time_to_pixel_ratio = params.canvas_width as f64 / time_range;
            let rect_start_x = (visible_start as f64 - params.viewport_start) * time_to_pixel_ratio;
            let rect_end_x = (visible_end as f64 - params.viewport_start) * time_to_pixel_ratio;
            let raw_rect_width = rect_end_x - rect_start_x;
            
            if raw_rect_width < 2.0 {
                if rect_start_x >= -10.0 && rect_start_x <= params.canvas_width as f64 + 10.0 {
                    let line_x = rect_start_x.max(0.0).min(params.canvas_width as f64 - 1.0);
                    objects.push(
                        Rectangle::new()
                            .position(line_x as f32, y_position)
                            .size(1.0, row_height)
                            .color(
                                theme_colors.cursor_color.0,
                                theme_colors.cursor_color.1,
                                theme_colors.cursor_color.2,
                                theme_colors.cursor_color.3,
                            )
                            .into(),
                    );
                }
                continue;
            }
            
            let rect_width = raw_rect_width.max(1.0);
            let rect_start_x = rect_start_x.max(0.0).min(params.canvas_width as f64 - rect_width);
            
            if rect_width <= 0.0 || rect_start_x >= params.canvas_width as f64 {
                continue;
            }
            
            let rect_color = match signal_value {
                SignalValue::Present(_) => {
                    let is_even_rect = rect_index % 2 == 0;
                    if is_even_rect {
                        theme_colors.value_color_1
                    } else {
                        theme_colors.value_color_2
                    }
                }
                SignalValue::Missing => theme_colors.neutral_2,
                SignalValue::Loading => theme_colors.neutral_3,
            };
            
            objects.push(
                Rectangle::new()
                    .position(rect_start_x as f32, y_position + 2.0)
                    .size(rect_width as f32, row_height - 4.0)
                    .color(rect_color.0, rect_color.1, rect_color.2, rect_color.3)
                    .into(),
            );
            
            let (formatted_value, text_color) = match signal_value {
                SignalValue::Present(binary_value) => {
                    (binary_value.clone(), theme_colors.neutral_12)
                }
                SignalValue::Missing => ("N/A".to_string(), theme_colors.neutral_3),
                SignalValue::Loading => ("Loading...".to_string(), theme_colors.neutral_3),
            };
            
            let text_padding = 5.0;
            let text_width = (rect_width - (text_padding * 2.0)).max(0.0);
            let text_height = (row_height / 2.0).max(8.0);
            
            if text_width >= 10.0 && text_height >= 8.0 {
                objects.push(
                    Text::new()
                        .text(formatted_value)
                        .position(
                            (rect_start_x + text_padding as f64) as f32,
                            (y_position + row_height / 3.0) as f32,
                        )
                        .size(text_width as f32, text_height as f32)
                        .color(text_color.0, text_color.1, text_color.2, text_color.3)
                        .font_size(11.0)
                        .family(Family::name("Fira Code"))
                        .into(),
                );
            }
        }
    }
    
    fn add_cursor_lines(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        _theme_colors: &ThemeColors,
    ) {
        let time_range = params.viewport_end - params.viewport_start;
        if time_range <= 0.0 {
            return;
        }
        
        if let Some(cursor_pos) = params.cursor_position {
            if cursor_pos >= params.viewport_start && cursor_pos <= params.viewport_end {
                let cursor_x = ((cursor_pos - params.viewport_start) / time_range) * params.canvas_width as f64;
                objects.push(
                    Rectangle::new()
                        .position(cursor_x as f32 - 1.0, 0.0)
                        .size(3.0, params.canvas_height as f32)
                        .color(255, 165, 0, 1.0)
                        .into(),
                );
            }
        }
        
        if let Some(zoom_center) = params.zoom_center_position {
            if zoom_center >= params.viewport_start && zoom_center <= params.viewport_end {
                let zoom_center_x = ((zoom_center - params.viewport_start) / time_range) * params.canvas_width as f64;
                objects.push(
                    Rectangle::new()
                        .position(zoom_center_x as f32 - 1.0, 0.0)
                        .size(3.0, params.canvas_height as f32)
                        .color(37, 99, 235, 0.9)
                        .into(),
                );
            }
        }
    }
    
    fn add_timeline_row(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
        row_height: f32,
        total_rows: usize,
    ) {
        let timeline_y = (total_rows - 1) as f32 * row_height;
        let time_range = params.viewport_end - params.viewport_start;
        
        objects.push(
            Rectangle::new()
                .position(0.0, timeline_y)
                .size(params.canvas_width as f32, row_height)
                .color(
                    theme_colors.neutral_2.0,
                    theme_colors.neutral_2.1,
                    theme_colors.neutral_2.2,
                    1.0,
                )
                .into(),
        );
        
        let target_tick_spacing = 60.0;
        let max_tick_count = (params.canvas_width as f32 / target_tick_spacing).floor() as i32;
        let tick_count = max_tick_count.max(2).min(10);
        
        let raw_time_step = time_range / (tick_count - 1) as f64;
        let time_step = Self::round_to_nice_number(raw_time_step as f32) as f64;
        
        let first_tick = (params.viewport_start / time_step).ceil() * time_step;
        let last_tick = params.viewport_end;
        let actual_tick_count = ((last_tick - first_tick) / time_step).ceil() as i32 + 1;
        
        let time_unit = Self::get_time_unit_for_range(params.viewport_start, params.viewport_end);
        
        for tick_index in 0..actual_tick_count {
            let time_value = first_tick + (tick_index as f64 * time_step);
            let time_value = time_value.min(params.viewport_end);
            let x_position = ((time_value - params.viewport_start) / time_range) * params.canvas_width as f64;
            
            objects.push(
                Rectangle::new()
                    .position(x_position as f32, timeline_y)
                    .size(1.0, 8.0)
                    .color(
                        theme_colors.neutral_12.0,
                        theme_colors.neutral_12.1,
                        theme_colors.neutral_12.2,
                        theme_colors.neutral_12.3,
                    )
                    .into(),
            );
            
            objects.push(
                Rectangle::new()
                    .position(x_position as f32, 0.0)
                    .size(1.0, timeline_y)
                    .color(
                        theme_colors.grid_color.0,
                        theme_colors.grid_color.1,
                        theme_colors.grid_color.2,
                        theme_colors.grid_color.3,
                    )
                    .into(),
            );
            
            let label_margin = 35.0;
            if x_position >= label_margin && x_position <= (params.canvas_width as f64 - label_margin) {
                let time_label = Self::format_time_with_unit(time_value as f32, time_unit);
                let is_near_right_edge = x_position > (params.canvas_width as f64 - 60.0);
                
                if !is_near_right_edge {
                    objects.push(
                        Text::new()
                            .text(time_label)
                            .position(x_position as f32 - 10.0, timeline_y + 15.0)
                            .size(20.0, row_height - 15.0)
                            .color(
                                theme_colors.neutral_12.0,
                                theme_colors.neutral_12.1,
                                theme_colors.neutral_12.2,
                                theme_colors.neutral_12.3,
                            )
                            .font_size(11.0)
                            .family(Family::name("Inter"))
                            .into(),
                    );
                }
            }
        }
        
        let label_y = timeline_y + 15.0;
        
        let start_label = Self::format_time_with_unit(params.viewport_start as f32, time_unit);
        objects.push(
            Text::new()
                .text(start_label)
                .position(5.0, label_y)
                .size(30.0, row_height - 15.0)
                .color(
                    theme_colors.neutral_12.0,
                    theme_colors.neutral_12.1,
                    theme_colors.neutral_12.2,
                    theme_colors.neutral_12.3,
                )
                .font_size(11.0)
                .family(Family::name("Inter"))
                .into(),
        );
        
        let end_label = Self::format_time_with_unit(params.viewport_end as f32, time_unit);
        let label_width = (end_label.len() as f32 * 7.0).max(30.0);
        objects.push(
            Text::new()
                .text(end_label)
                .position(params.canvas_width as f32 - label_width - 5.0, label_y)
                .size(label_width, row_height - 15.0)
                .color(
                    theme_colors.neutral_12.0,
                    theme_colors.neutral_12.1,
                    theme_colors.neutral_12.2,
                    theme_colors.neutral_12.3,
                )
                .font_size(11.0)
                .family(Family::name("Inter"))
                .into(),
        );
    }
    
    fn get_variable_transitions(
        _variable: &SelectedVariable,
        _params: &RenderingParameters,
    ) -> Vec<(f32, SignalValue)> {
        Vec::new()
    }
    
    fn get_theme_colors(theme: NovyUITheme) -> ThemeColors {
        match theme {
            NovyUITheme::Dark => ThemeColors {
                neutral_2: (45, 47, 50, 1.0),
                neutral_3: (52, 54, 58, 1.0),
                neutral_12: (253, 253, 253, 1.0),
                grid_color: (64, 64, 64, 0.4),
                separator_color: (80, 80, 80, 0.6),
                cursor_color: (59, 130, 246, 0.8),
                value_color_1: (65, 69, 75, 1.0),
                value_color_2: (75, 79, 86, 1.0),
            },
            NovyUITheme::Light => ThemeColors {
                neutral_2: (249, 250, 251, 1.0),
                neutral_3: (243, 244, 246, 1.0),
                neutral_12: (17, 24, 39, 1.0),
                grid_color: (148, 163, 184, 0.4),
                separator_color: (100, 116, 139, 0.6),
                cursor_color: (37, 99, 235, 0.8),
                value_color_1: (229, 231, 235, 1.0),
                value_color_2: (209, 213, 219, 1.0),
            },
        }
    }
    
    fn get_time_unit_for_range(min_time: f64, max_time: f64) -> TimeUnit {
        let range = max_time - min_time;
        if range < 1e-6 {
            TimeUnit::Nanosecond
        } else if range < 1e-3 {
            TimeUnit::Microsecond
        } else if range < 1.0 {
            TimeUnit::Millisecond
        } else {
            TimeUnit::Second
        }
    }
    
    fn format_time_with_unit(time_seconds: f32, unit: TimeUnit) -> String {
        let scaled_value = time_seconds * unit.scale_factor();
        match unit {
            TimeUnit::Nanosecond => {
                format!("{}{}", scaled_value.round() as i32, unit.suffix())
            }
            TimeUnit::Microsecond => {
                format!("{}{}", scaled_value.round() as i32, unit.suffix())
            }
            _ => {
                format!("{}{}", scaled_value.round() as i32, unit.suffix())
            }
        }
    }
    
    fn round_to_nice_number(value: f32) -> f32 {
        if value <= 0.0 {
            return 1.0;
        }
        
        let magnitude = 10_f32.powf(value.log10().floor());
        let normalized = value / magnitude;
        
        let nice_normalized = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };
        
        nice_normalized * magnitude
    }
    
    fn get_current_time_ms() -> f32 {
        (js_sys::Date::now()) as f32
    }
}