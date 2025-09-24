use crate::dataflow::*;
use fast2d::{CanvasWrapper as Fast2DCanvas, Family, Object2d, Rectangle, Text};
use futures::{select, stream::StreamExt};
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::{SignalTransition, SignalValue, VarFormat};
use zoon::*;

#[derive(Debug, Clone, Copy)]
enum TimeUnit {
    Nanosecond,
    Microsecond,
    Millisecond,
    Second,
}

impl TimeUnit {
    fn suffix(self) -> &'static str {
        match self {
            TimeUnit::Nanosecond => "ns",
            TimeUnit::Microsecond => "us",
            TimeUnit::Millisecond => "ms",
            TimeUnit::Second => "s",
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
pub struct VariableRenderSnapshot {
    pub unique_id: String,
    pub formatter: VarFormat,
    pub transitions: Vec<SignalTransition>,
    pub cursor_value: Option<SignalValue>,
}

#[derive(Clone, Debug)]
pub struct RenderingParameters {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub viewport_start_ns: u64,
    pub viewport_end_ns: u64,
    pub cursor_position_ns: Option<u64>,
    pub zoom_center_ns: Option<u64>,
    pub theme: NovyUITheme,
    pub variables: Vec<VariableRenderSnapshot>,
}

pub struct WaveformRenderer {
    pub rendering_state: Actor<RenderingState>,
    pub render_requested_relay: Relay<RenderingParameters>,
    pub render_completed_relay: Relay<RenderResult>,

    canvas: Option<Fast2DCanvas>,
}

impl Clone for WaveformRenderer {
    fn clone(&self) -> Self {
        Self {
            rendering_state: self.rendering_state.clone(),
            render_requested_relay: self.render_requested_relay.clone(),
            render_completed_relay: self.render_completed_relay.clone(),
            canvas: None,
        }
    }
}

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

    pub fn set_theme(&mut self, _theme: NovyUITheme) {}

    pub fn set_dimensions(&mut self, width: f32, height: f32) {
        if let Some(canvas) = &mut self.canvas {
            let width = width.max(1.0) as u32;
            let height = height.max(1.0) as u32;
            canvas.resized(width, height);
        }
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
        if params.viewport_end_ns <= params.viewport_start_ns {
            return Vec::new();
        }

        let theme_colors = Self::get_theme_colors(params.theme);
        let mut objects = Vec::new();

        Self::add_waveforms(&mut objects, params, &theme_colors);
        Self::add_cursor_lines(&mut objects, params, &theme_colors);
        Self::add_timeline_row(&mut objects, params, &theme_colors);

        objects
    }

    fn add_waveforms(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if params.variables.is_empty() {
            return;
        }

        let total_rows = params.variables.len() + 1;
        let available_height = params.canvas_height as f32;
        let row_height = available_height / total_rows.max(1) as f32;

        for (index, variable) in params.variables.iter().enumerate() {
            let row_top = index as f32 * row_height;
            let row_color = if index % 2 == 0 {
                theme_colors.neutral_2
            } else {
                theme_colors.neutral_3
            };

            objects.push(
                Rectangle::new()
                    .position(0.0, row_top)
                    .size(params.canvas_width as f32, row_height)
                    .color(row_color.0, row_color.1, row_color.2, row_color.3)
                    .into(),
            );

            Self::add_signal_segments(objects, variable, row_top, row_height, params, theme_colors);

            if index < params.variables.len() - 1 {
                let separator_y =
                    ((index + 1) as f32 * row_height).min(params.canvas_height as f32);
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
    }

    fn add_signal_segments(
        objects: &mut Vec<Object2d>,
        variable: &VariableRenderSnapshot,
        row_top: f32,
        row_height: f32,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if params.viewport_end_ns <= params.viewport_start_ns {
            return;
        }

        let range_ns = params.viewport_end_ns - params.viewport_start_ns;
        let width = params.canvas_width as f64;
        let start_ns = params.viewport_start_ns;
        let end_ns = params.viewport_end_ns;

        for (index, transition) in variable.transitions.iter().enumerate() {
            let mut segment_start = transition.time_ns;
            if segment_start >= end_ns {
                break;
            }
            let next_time = if index + 1 < variable.transitions.len() {
                variable.transitions[index + 1].time_ns
            } else {
                end_ns
            };
            let mut segment_end = next_time;

            if segment_end <= start_ns {
                continue;
            }
            if segment_start < start_ns {
                segment_start = start_ns;
            }
            if segment_end > end_ns {
                segment_end = end_ns;
            }
            if segment_end <= segment_start {
                continue;
            }

            let start_ratio = (segment_start - start_ns) as f64 / range_ns as f64;
            let end_ratio = (segment_end - start_ns) as f64 / range_ns as f64;
            let rect_start_x = (start_ratio * width).max(0.0);
            let rect_end_x = (end_ratio * width).max(rect_start_x + 1.0);
            let rect_width = (rect_end_x - rect_start_x).max(1.0);

            let color = Self::color_for_value(&transition.value, index, theme_colors);
            objects.push(
                Rectangle::new()
                    .position(rect_start_x as f32, row_top + 2.0)
                    .size(rect_width as f32, row_height - 4.0)
                    .color(color.0, color.1, color.2, color.3)
                    .into(),
            );

            if rect_width as f32 > 18.0 && row_height > 14.0 {
                let text_color = theme_colors.neutral_12;
                let text = Self::truncate_value_text(&transition.value, rect_width as usize / 7);
                objects.push(
                    Text::new()
                        .text(text)
                        .position(rect_start_x as f32 + 4.0, row_top + row_height / 3.0)
                        .size(rect_width as f32 - 8.0, row_height / 2.0)
                        .color(text_color.0, text_color.1, text_color.2, text_color.3)
                        .font_size(11.0)
                        .family(Family::name("Fira Code"))
                        .into(),
                );
            }
        }
    }

    fn color_for_value(value: &str, index: usize, theme_colors: &ThemeColors) -> (u8, u8, u8, f32) {
        let normalized = value.trim().to_ascii_uppercase();
        match normalized.as_str() {
            "Z" => theme_colors.cursor_color,
            "X" | "U" => (220, 80, 80, 0.9),
            "N/A" => theme_colors.neutral_3,
            _ => {
                if index % 2 == 0 {
                    theme_colors.value_color_1
                } else {
                    theme_colors.value_color_2
                }
            }
        }
    }

    fn truncate_value_text(value: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }
        let char_count = value.chars().count();
        if char_count <= max_chars {
            return value.to_string();
        }
        if max_chars <= 3 {
            value.chars().take(max_chars).collect()
        } else {
            let mut truncated: String = value.chars().take(max_chars - 3).collect();
            truncated.push_str("...");
            truncated
        }
    }

    fn add_cursor_lines(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if params.viewport_end_ns <= params.viewport_start_ns {
            return;
        }
        let range_ns = (params.viewport_end_ns - params.viewport_start_ns) as f64;

        if let Some(cursor_ns) = params.cursor_position_ns {
            if (params.viewport_start_ns..=params.viewport_end_ns).contains(&cursor_ns) {
                let ratio = (cursor_ns - params.viewport_start_ns) as f64 / range_ns;
                let x = (ratio * params.canvas_width as f64) as f32;
                objects.push(
                    Rectangle::new()
                        .position(x - 1.0, 0.0)
                        .size(3.0, params.canvas_height as f32)
                        .color(255, 165, 0, 1.0)
                        .into(),
                );
            }
        }

        if let Some(center_ns) = params.zoom_center_ns {
            if (params.viewport_start_ns..=params.viewport_end_ns).contains(&center_ns) {
                let ratio = (center_ns - params.viewport_start_ns) as f64 / range_ns;
                let x = (ratio * params.canvas_width as f64) as f32;
                let color = theme_colors.cursor_color;
                objects.push(
                    Rectangle::new()
                        .position(x - 1.0, 0.0)
                        .size(3.0, params.canvas_height as f32)
                        .color(color.0, color.1, color.2, color.3)
                        .into(),
                );
            }
        }
    }

    fn add_timeline_row(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if params.viewport_end_ns <= params.viewport_start_ns {
            return;
        }

        let total_rows = params.variables.len() + 1;
        let available_height = (params.canvas_height as f32 - 5.0).max(1.0);
        let row_height = available_height / total_rows.max(1) as f32;
        let timeline_y = (total_rows - 1) as f32 * row_height;

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

        let start_s = params.viewport_start_ns as f64 / 1_000_000_000.0;
        let end_s = params.viewport_end_ns as f64 / 1_000_000_000.0;
        let time_range_s = (end_s - start_s).max(1e-9);
        let time_range_ns = (params.viewport_end_ns - params.viewport_start_ns) as f64;

        let target_tick_spacing = 80.0;
        let desired_tick_count =
            (params.canvas_width as f64 / target_tick_spacing).clamp(2.0, 12.0);
        let raw_step_s = time_range_s / desired_tick_count.max(1.0);
        let step_s = Self::round_to_nice_number(raw_step_s);

        let first_tick_s = (start_s / step_s).ceil() * step_s;
        let mut tick_s = first_tick_s;
        while tick_s < end_s {
            let tick_ns = (tick_s * 1_000_000_000.0).round() as u64;
            let ratio = (tick_ns - params.viewport_start_ns) as f64 / time_range_ns;
            let x = (ratio * params.canvas_width as f64) as f32;

            objects.push(
                Rectangle::new()
                    .position(x, timeline_y)
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
                    .position(x, 0.0)
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
            if (x as f64) >= label_margin
                && (x as f64) <= (params.canvas_width as f64 - label_margin)
            {
                let label = Self::format_time_ns(tick_ns);
                objects.push(
                    Text::new()
                        .text(label)
                        .position(x - 10.0, timeline_y + 15.0)
                        .size(50.0, row_height - 15.0)
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

            tick_s += step_s;
        }

        let start_label = Self::format_time_ns(params.viewport_start_ns);
        objects.push(
            Text::new()
                .text(start_label)
                .position(5.0, timeline_y + 15.0)
                .size(60.0, row_height - 15.0)
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

        let end_label = Self::format_time_ns(params.viewport_end_ns);
        let label_width = (end_label.len() as f32 * 7.0).max(40.0);
        objects.push(
            Text::new()
                .text(end_label)
                .position(
                    params.canvas_width as f32 - label_width - 5.0,
                    timeline_y + 15.0,
                )
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

    fn format_time_ns(ns: u64) -> String {
        if ns >= 1_000_000_000 {
            format!("{:.1}s", ns as f64 / 1_000_000_000.0)
        } else if ns >= 1_000_000 {
            format!("{:.1}ms", ns as f64 / 1_000_000.0)
        } else if ns >= 1_000 {
            format!("{:.1}us", ns as f64 / 1_000.0)
        } else {
            format!("{}ns", ns)
        }
    }

    fn round_to_nice_number(value: f64) -> f64 {
        if value <= 0.0 {
            return 1.0;
        }

        let magnitude = 10_f64.powf(value.log10().floor());
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

    fn get_current_time_ms() -> f32 {
        (js_sys::Date::now()) as f32
    }
}
