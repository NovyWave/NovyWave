use crate::dataflow::Actor;
use fast2d::{CanvasWrapper as Fast2DCanvas, Family, Object2d, Rectangle, Text};
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::{SignalTransition, SignalValue, VarFormat};

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
    row_even_bg: (u8, u8, u8, f32),
    row_odd_bg: (u8, u8, u8, f32),
    timeline_row_bg: (u8, u8, u8, f32),
    neutral_12: (u8, u8, u8, f32),
    grid_color: (u8, u8, u8, f32),
    separator_color: (u8, u8, u8, f32),
    cursor_color: (u8, u8, u8, f32),
    segment_divider_color: (u8, u8, u8, f32),
    value_low_color: (u8, u8, u8, f32),
    value_high_color: (u8, u8, u8, f32),
    value_bus_color: (u8, u8, u8, f32),
    state_high_impedance: (u8, u8, u8, f32),
    state_unknown: (u8, u8, u8, f32),
    state_uninitialized: (u8, u8, u8, f32),
    segment_alt_multiplier: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SignalState {
    Regular,
    HighImpedance,
    Unknown,
    Uninitialized,
    Missing,
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
    rendering_state: Actor<RenderingState>,
    canvas: Option<Fast2DCanvas>,
}

#[derive(Clone, Debug)]
struct RenderingState {
    pub last_render_params: Option<RenderingParameters>,
    pub render_count: u32,
    pub last_result: Option<RenderResult>,
}

impl Default for RenderingState {
    fn default() -> Self {
        Self {
            last_render_params: None,
            render_count: 0,
            last_result: None,
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
        let rendering_state = Actor::new(RenderingState::default(), async move |_state| {});

        Self {
            rendering_state,
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
            let start_time = Self::get_current_time_ms();
            let objects = Self::build_render_objects(&params);
            let objects_rendered = objects.len();

            canvas.update_objects(|canvas_objects| {
                canvas_objects.clear();
                canvas_objects.extend(objects);
            });

            let render_time = Self::get_current_time_ms() - start_time;
            let mut state = self.rendering_state.state.lock_mut();
            state.render_count = state.render_count.saturating_add(1);
            let render_count = state.render_count;
            state.last_render_params = Some(params.clone());
            state.last_result = Some(RenderResult {
                render_count,
                objects_rendered,
                rendering_time_ms: render_time,
            });
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
        Self::add_timeline_row(&mut objects, params, &theme_colors);
        Self::add_cursor_lines(&mut objects, params, &theme_colors);

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
                theme_colors.row_even_bg
            } else {
                theme_colors.row_odd_bg
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

            let state = Self::classify_signal_state(&transition.value);
            if state == SignalState::Missing {
                continue;
            }

            let formatted_value =
                SignalValue::Present(transition.value.clone()).get_formatted(&variable.formatter);

            let (rect_top, rect_height, base_color) = match state {
                SignalState::HighImpedance => {
                    let height = (row_height - 4.0).max(2.0) * 0.55;
                    let top = row_top + (row_height - height) / 2.0;
                    (top, height, theme_colors.state_high_impedance)
                }
                SignalState::Unknown => {
                    (row_top + 2.0, row_height - 4.0, theme_colors.state_unknown)
                }
                SignalState::Uninitialized => (
                    row_top + 2.0,
                    row_height - 4.0,
                    theme_colors.state_uninitialized,
                ),
                SignalState::Regular => (
                    row_top + 2.0,
                    row_height - 4.0,
                    Self::regular_value_color(&transition.value, variable.formatter, theme_colors),
                ),
                SignalState::Missing => unreachable!(),
            };

            let color = if index % 2 == 0 {
                base_color
            } else {
                Self::tint_color(base_color, theme_colors.segment_alt_multiplier)
            };

            objects.push(
                Rectangle::new()
                    .position(rect_start_x as f32, rect_top)
                    .size(rect_width as f32, rect_height.max(1.5))
                    .color(color.0, color.1, color.2, color.3)
                    .into(),
            );

            if rect_start_x > 0.5 {
                objects.push(
                    Rectangle::new()
                        .position(rect_start_x as f32, rect_top)
                        .size(1.0, rect_height.max(1.5))
                        .color(
                            theme_colors.segment_divider_color.0,
                            theme_colors.segment_divider_color.1,
                            theme_colors.segment_divider_color.2,
                            theme_colors.segment_divider_color.3,
                        )
                        .into(),
                );
            }

            if rect_width as f32 > 18.0 && row_height > 14.0 {
                let text_color = theme_colors.neutral_12;
                let text = Self::truncate_value_text(&formatted_value, rect_width as usize / 7);
                let text_top = rect_top + rect_height / 2.0 - 6.0;
                objects.push(
                    Text::new()
                        .text(text)
                        .position(rect_start_x as f32 + 4.0, text_top.max(row_top + 2.0))
                        .size(rect_width as f32 - 8.0, rect_height.max(12.0))
                        .color(text_color.0, text_color.1, text_color.2, text_color.3)
                        .font_size(13.0)
                        .family(Family::name("Fira Code"))
                        .into(),
                );
            }
        }
    }

    fn classify_signal_state(value: &str) -> SignalState {
        let normalized = value.trim().to_ascii_uppercase();
        match normalized.as_str() {
            "Z" => SignalState::HighImpedance,
            "X" => SignalState::Unknown,
            "U" => SignalState::Uninitialized,
            "N/A" | "NA" => SignalState::Missing,
            _ => SignalState::Regular,
        }
    }

    fn tint_color(color: (u8, u8, u8, f32), multiplier: f32) -> (u8, u8, u8, f32) {
        let (r, g, b, a) = color;
        let clamp = |component: u8| -> u8 {
            let scaled = (component as f32) * multiplier;
            scaled.clamp(0.0, 255.0).round() as u8
        };

        (clamp(r), clamp(g), clamp(b), a)
    }

    fn regular_value_color(
        value: &str,
        formatter: VarFormat,
        theme_colors: &ThemeColors,
    ) -> (u8, u8, u8, f32) {
        let normalized = value.trim();
        if normalized.is_empty() {
            return theme_colors.value_bus_color;
        }

        match formatter {
            VarFormat::Binary | VarFormat::BinaryWithGroups => {
                if normalized.len() == 1 && (normalized == "0" || normalized == "1") {
                    if normalized == "1" {
                        theme_colors.value_high_color
                    } else {
                        theme_colors.value_low_color
                    }
                } else {
                    theme_colors.value_bus_color
                }
            }
            VarFormat::Hexadecimal
            | VarFormat::Octal
            | VarFormat::Signed
            | VarFormat::Unsigned
            | VarFormat::ASCII => theme_colors.value_bus_color,
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
                let dash_height = 6.0;
                let gap_height = 4.0;
                let mut y = 0.0;
                let color = (67, 217, 115, 0.95);
                while y < params.canvas_height as f32 {
                    let remaining = params.canvas_height as f32 - y;
                    let segment_height = remaining.min(dash_height);
                    objects.push(
                        Rectangle::new()
                            .position(x - 1.0, y)
                            .size(2.0, segment_height)
                            .color(color.0, color.1, color.2, color.3)
                            .into(),
                    );
                    y += dash_height + gap_height;
                }
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
        let available_height = params.canvas_height as f32;
        let row_height = available_height / total_rows.max(1) as f32;
        let timeline_y = (total_rows - 1) as f32 * row_height;
        let timeline_height = (params.canvas_height as f32 - timeline_y).max(1.0);

        objects.push(
            Rectangle::new()
                .position(0.0, timeline_y)
                .size(params.canvas_width as f32, timeline_height)
                .color(
                    theme_colors.timeline_row_bg.0,
                    theme_colors.timeline_row_bg.1,
                    theme_colors.timeline_row_bg.2,
                    theme_colors.timeline_row_bg.3,
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

        let mut ticks: Vec<(f32, Option<String>)> = Vec::new();

        ticks.push((0.0, Some(Self::format_time_ns(params.viewport_start_ns))));

        let first_tick_s = (start_s / step_s).ceil() * step_s;
        let mut tick_s = first_tick_s;
        while tick_s < end_s {
            let tick_ns = (tick_s * 1_000_000_000.0).round() as u64;
            let ratio = (tick_ns - params.viewport_start_ns) as f64 / time_range_ns;
            let x = (ratio * params.canvas_width as f64) as f32;

            if x > 0.0 && x < params.canvas_width as f32 {
                ticks.push((x, Some(Self::format_time_ns(tick_ns))));
            }

            tick_s += step_s;
        }

        ticks.push((
            params.canvas_width as f32,
            Some(Self::format_time_ns(params.viewport_end_ns)),
        ));

        let mut last_label_right = -f32::INFINITY;
        let minimum_label_gap = 56.0;

        for (x, label) in &ticks {
            objects.push(
                Rectangle::new()
                    .position(*x, timeline_y)
                    .size(1.0, 8.0)
                    .color(
                        theme_colors.neutral_12.0,
                        theme_colors.neutral_12.1,
                        theme_colors.neutral_12.2,
                        theme_colors.neutral_12.3,
                    )
                    .into(),
            );
            if timeline_y > 0.0 {
                objects.push(
                    Rectangle::new()
                        .position(*x, 0.0)
                        .size(1.0, timeline_y)
                        .color(
                            theme_colors.grid_color.0,
                            theme_colors.grid_color.1,
                            theme_colors.grid_color.2,
                            theme_colors.grid_color.3,
                        )
                        .into(),
                );
            }

            if let Some(text) = label {
                let approx_width = (text.len() as f32 * 6.5).max(35.0);
                let left_edge = x - approx_width / 2.0;
                if left_edge > last_label_right + minimum_label_gap {
                    objects.push(
                        Text::new()
                            .text(text.clone())
                            .position(left_edge, timeline_y + 15.0)
                            .size(approx_width, timeline_height - 15.0)
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
                    last_label_right = left_edge + approx_width;
                }
            }
        }
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
                row_even_bg: (6, 9, 14, 1.0),
                row_odd_bg: (12, 15, 22, 1.0),
                timeline_row_bg: (5, 11, 22, 1.0),
                neutral_12: (253, 253, 253, 1.0),
                grid_color: (36, 50, 72, 0.35),
                separator_color: (42, 48, 58, 0.6),
                cursor_color: (59, 130, 246, 0.8),
                segment_divider_color: (3, 4, 9, 1.0),
                value_low_color: (18, 50, 140, 1.0),
                value_high_color: (16, 96, 72, 1.0),
                value_bus_color: (44, 58, 150, 1.0),
                state_high_impedance: (234, 179, 8, 0.85),
                state_unknown: (220, 38, 38, 0.9),
                state_uninitialized: (220, 38, 38, 0.65),
                segment_alt_multiplier: 0.45,
            },
            NovyUITheme::Light => ThemeColors {
                row_even_bg: (248, 250, 255, 1.0),
                row_odd_bg: (240, 246, 255, 1.0),
                timeline_row_bg: (234, 246, 255, 1.0),
                neutral_12: (17, 24, 39, 1.0),
                grid_color: (158, 173, 194, 0.35),
                separator_color: (135, 148, 170, 0.6),
                cursor_color: (37, 99, 235, 0.8),
                segment_divider_color: (206, 212, 224, 1.0),
                value_low_color: (110, 148, 255, 1.0),
                value_high_color: (54, 200, 160, 1.0),
                value_bus_color: (152, 176, 255, 1.0),
                state_high_impedance: (202, 138, 4, 0.9),
                state_unknown: (220, 38, 38, 0.85),
                state_uninitialized: (220, 38, 38, 0.6),
                segment_alt_multiplier: 1.1,
            },
        }
    }

    fn get_current_time_ms() -> f32 {
        (js_sys::Date::now()) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::WaveformRenderer;

    #[test]
    fn rounds_small_values_to_friendly_steps() {
        assert!((WaveformRenderer::round_to_nice_number(0.12) - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn rounds_medium_values_to_two() {
        assert!((WaveformRenderer::round_to_nice_number(1.1) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rounds_large_values_to_ten() {
        assert!((WaveformRenderer::round_to_nice_number(6.5) - 10.0).abs() < f64::EPSILON);
    }
}
