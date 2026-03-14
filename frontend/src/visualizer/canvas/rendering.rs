use crate::visualizer::timeline::time_domain::{PS_PER_MS, PS_PER_NS, PS_PER_SECOND, PS_PER_US};
use fast2d::{CanvasWrapper as Fast2DCanvas, Family, Line, Object2d, Rectangle, Text};
use moonzoon_novyui::tokens::theme::Theme as NovyUITheme;
use shared::{AnalogLimits, SignalTransition, SignalValue, VarFormat};
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;
use zoon::Mutable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimeLabelUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Picoseconds,
}

#[derive(Clone, Debug, PartialEq)]
enum PixelValue {
    Single(Rc<String>),
    Mixed,
}

#[derive(Clone, Debug)]
struct ThemeColors {
    row_even_bg: (u8, u8, u8, f32),
    row_odd_bg: (u8, u8, u8, f32),
    timeline_row_bg: (u8, u8, u8, f32),
    neutral_12: (u8, u8, u8, f32),
    grid_color: (u8, u8, u8, f32),
    separator_color: (u8, u8, u8, f32),
    #[allow(dead_code)]
    cursor_color: (u8, u8, u8, f32),
    segment_divider_color: (u8, u8, u8, f32),
    value_low_color: (u8, u8, u8, f32),
    value_high_color: (u8, u8, u8, f32),
    value_bus_color: (u8, u8, u8, f32),
    state_high_impedance: (u8, u8, u8, f32),
    state_unknown: (u8, u8, u8, f32),
    state_uninitialized: (u8, u8, u8, f32),
    segment_alt_multiplier: f32,
    value_analog_color: (u8, u8, u8, f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SignalState {
    Regular,
    HighImpedance,
    Unknown,
    Uninitialized,
    Missing,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasRenderDebug {
    pub had_canvas: bool,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub viewport_start_ps: u64,
    pub viewport_end_ps: u64,
    pub rows_len: usize,
    pub markers_len: usize,
    pub static_changed: bool,
    pub static_count: usize,
    pub overlay_count: usize,
    pub total_objects: usize,
    pub static_skip_reason: Option<&'static str>,
}

thread_local! {
    static CANVAS_RENDER_DEBUG: RefCell<CanvasRenderDebug> =
        RefCell::new(CanvasRenderDebug::default());
}

pub fn canvas_render_debug_snapshot() -> CanvasRenderDebug {
    CANVAS_RENDER_DEBUG.with(|debug| debug.borrow().clone())
}

#[derive(Clone, Debug)]
pub struct VariableRenderSnapshot {
    pub unique_id: String,
    pub formatter: VarFormat,
    pub transitions: Arc<Vec<SignalTransition>>,
    #[allow(dead_code)]
    pub cursor_value: Option<SignalValue>,
    pub actual_time_range_ns: Option<(u64, u64)>,
    pub signal_type: Option<String>,
    pub row_height: u32,
    pub analog_limits: Option<AnalogLimits>,
}

#[derive(Clone, Debug)]
pub enum RenderRowSnapshot {
    GroupHeader { name: String, row_height: u32 },
    Variable(VariableRenderSnapshot),
}

#[derive(Clone, Debug)]
pub struct MarkerRenderData {
    pub time_ps: u64,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct RenderingParameters {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub viewport_start_ps: u64,
    pub viewport_end_ps: u64,
    pub cursor_position_ps: Option<u64>,
    pub zoom_center_ps: Option<u64>,
    pub theme: NovyUITheme,
    pub rows: Vec<RenderRowSnapshot>,
    pub markers: Vec<MarkerRenderData>,
}

fn row_metrics(
    rows: &[RenderRowSnapshot],
) -> Vec<crate::selected_variables_layout::SelectedVariablesRowMetric> {
    rows.iter()
        .map(|row| match row {
            RenderRowSnapshot::GroupHeader { .. } => {
                crate::selected_variables_layout::SelectedVariablesRowMetric::group_header()
            }
            RenderRowSnapshot::Variable(variable) => {
                crate::selected_variables_layout::SelectedVariablesRowMetric::variable(
                    variable.row_height,
                )
            }
        })
        .collect()
}

pub struct WaveformRenderer {
    rendering_state: Mutable<RenderingState>,
    canvas: Option<Fast2DCanvas>,
}

#[derive(Clone, Debug)]
struct RenderingState {
    pub last_render_params: Option<RenderingParameters>,
    pub render_count: u32,
    pub last_result: Option<RenderResult>,
    pub static_cache: Option<StaticCacheInfo>,
}

impl Default for RenderingState {
    fn default() -> Self {
        Self {
            last_render_params: None,
            render_count: 0,
            last_result: None,
            static_cache: None,
        }
    }
}

#[derive(Clone, Debug)]
struct StaticCacheInfo {
    key: StaticRenderKey,
    static_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StaticRenderKey {
    canvas_width: u32,
    canvas_height: u32,
    viewport_start_ps: u64,
    viewport_end_ps: u64,
    theme_key: u8,
    variables_signature: u64,
    revision: u8,
}

const STATIC_CACHE_REVISION: u8 = 1;

impl StaticRenderKey {
    fn from_params(params: &RenderingParameters) -> Self {
        Self {
            canvas_width: params.canvas_width,
            canvas_height: params.canvas_height,
            viewport_start_ps: params.viewport_start_ps,
            viewport_end_ps: params.viewport_end_ps,
            theme_key: Self::theme_key(params.theme),
            variables_signature: Self::rows_signature(&params.rows),
            revision: STATIC_CACHE_REVISION,
        }
    }

    fn rows_signature(rows: &[RenderRowSnapshot]) -> u64 {
        let mut hasher = DefaultHasher::new();
        rows.len().hash(&mut hasher);
        for row in rows {
            match row {
                RenderRowSnapshot::GroupHeader { name, row_height } => {
                    name.hash(&mut hasher);
                    row_height.hash(&mut hasher);
                }
                RenderRowSnapshot::Variable(variable) => {
                    variable.unique_id.hash(&mut hasher);
                    variable.formatter.hash(&mut hasher);
                    variable.row_height.hash(&mut hasher);
                    variable.signal_type.hash(&mut hasher);
                    if let Some(limits) = &variable.analog_limits {
                        limits.auto.hash(&mut hasher);
                        limits.min.to_bits().hash(&mut hasher);
                        limits.max.to_bits().hash(&mut hasher);
                    }
                    let ptr = Arc::as_ptr(&variable.transitions) as usize;
                    ptr.hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }

    fn theme_key(theme: NovyUITheme) -> u8 {
        match theme {
            NovyUITheme::Dark => 0,
            NovyUITheme::Light => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderResult {
    #[allow(dead_code)]
    pub render_count: u32,
    #[allow(dead_code)]
    pub objects_rendered: usize,
    #[allow(dead_code)]
    pub rendering_time_ms: f32,
}

impl WaveformRenderer {
    pub fn new() -> Self {
        let rendering_state = Mutable::new(RenderingState::default());

        Self {
            rendering_state,
            canvas: None,
        }
    }

    pub fn set_canvas(&mut self, canvas: Fast2DCanvas) {
        self.canvas = Some(canvas);
        self.rendering_state.lock_mut().static_cache = None;
    }

    #[allow(dead_code)]
    pub fn has_canvas(&self) -> bool {
        self.canvas.is_some()
    }

    pub fn set_theme(&mut self, _theme: NovyUITheme) {}

    pub fn set_dimensions(&mut self, width: f32, height: f32) {
        if let Some(canvas) = &mut self.canvas {
            let width = width.max(1.0) as u32;
            let height = height.max(1.0) as u32;
            canvas.resized(width, height);
            self.rendering_state.lock_mut().static_cache = None;
        }
    }

    pub fn render_frame(&mut self, params: RenderingParameters) -> Option<f32> {
        if let Some(canvas) = &mut self.canvas {
            let start_time = Self::get_current_time_ms();
            let theme_colors = Self::get_theme_colors(params.theme);
            let static_skip_reason = Self::static_skip_reason(&params);
            let overlay_objects = Self::build_overlay_objects(&params, &theme_colors);

            let mut state = self.rendering_state.lock_mut();
            let static_key = StaticRenderKey::from_params(&params);
            let mut static_count = state
                .static_cache
                .as_ref()
                .map(|cache| cache.static_count)
                .unwrap_or(0);
            let static_changed = state
                .static_cache
                .as_ref()
                .map(|cache| cache.key != static_key)
                .unwrap_or(true);

            let mut static_objects = if static_changed {
                let objects = Self::build_static_objects(&params, &theme_colors);
                static_count = objects.len();
                Some(objects)
            } else {
                None
            };

            let static_count_local = static_count;
            let overlay_for_update = overlay_objects.clone();
            canvas.update_objects(move |canvas_objects| {
                if let Some(mut new_static) = static_objects.take() {
                    canvas_objects.clear();
                    canvas_objects.append(&mut new_static);
                } else {
                    canvas_objects.truncate(static_count_local);
                }
                canvas_objects.extend(overlay_for_update.iter().cloned());
            });

            state.static_cache = Some(StaticCacheInfo {
                key: static_key,
                static_count,
            });

            state.render_count = state.render_count.saturating_add(1);
            let render_count = state.render_count;
            state.last_render_params = Some(params.clone());
            let render_time = Self::get_current_time_ms() - start_time;
            let objects_rendered = static_count + overlay_objects.len();
            state.last_result = Some(RenderResult {
                render_count,
                objects_rendered,
                rendering_time_ms: render_time,
            });
            CANVAS_RENDER_DEBUG.with(|debug| {
                *debug.borrow_mut() = CanvasRenderDebug {
                    had_canvas: true,
                    canvas_width: params.canvas_width,
                    canvas_height: params.canvas_height,
                    viewport_start_ps: params.viewport_start_ps,
                    viewport_end_ps: params.viewport_end_ps,
                    rows_len: params.rows.len(),
                    markers_len: params.markers.len(),
                    static_changed,
                    static_count,
                    overlay_count: overlay_objects.len(),
                    total_objects: objects_rendered,
                    static_skip_reason,
                };
            });
            if render_time > 80.0 {
                zoon::println!(
                    "⚠️ Waveform render took {:.1}ms (threshold 80ms)",
                    render_time
                );
            }
            Some(render_time)
        } else {
            CANVAS_RENDER_DEBUG.with(|debug| {
                *debug.borrow_mut() = CanvasRenderDebug {
                    had_canvas: false,
                    canvas_width: params.canvas_width,
                    canvas_height: params.canvas_height,
                    viewport_start_ps: params.viewport_start_ps,
                    viewport_end_ps: params.viewport_end_ps,
                    rows_len: params.rows.len(),
                    markers_len: params.markers.len(),
                    static_changed: false,
                    static_count: 0,
                    overlay_count: 0,
                    total_objects: 0,
                    static_skip_reason: Some("missing_canvas"),
                };
            });
            None
        }
    }

    fn static_skip_reason(params: &RenderingParameters) -> Option<&'static str> {
        if params.canvas_width == 0 || params.canvas_height == 0 {
            return Some("zero_canvas");
        }
        if params.viewport_end_ps <= params.viewport_start_ps {
            return Some("invalid_viewport");
        }
        None
    }

    fn build_static_objects(
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) -> Vec<Object2d> {
        if params.canvas_width == 0 || params.canvas_height == 0 {
            return Vec::new();
        }
        if params.viewport_end_ps <= params.viewport_start_ps {
            return Vec::new();
        }

        let mut objects = Vec::new();
        Self::add_waveforms(&mut objects, params, theme_colors);
        Self::add_timeline_row(&mut objects, params, theme_colors);
        objects
    }

    fn build_overlay_objects(
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) -> Vec<Object2d> {
        let mut objects = Vec::new();
        if params.canvas_width == 0 || params.canvas_height == 0 {
            return objects;
        }
        if params.viewport_end_ps <= params.viewport_start_ps {
            return objects;
        }
        Self::add_cursor_lines(&mut objects, params, theme_colors);
        Self::add_marker_lines(&mut objects, params);
        objects
    }

    fn compute_row_layout(
        params: &RenderingParameters,
    ) -> Vec<crate::selected_variables_layout::SelectedVariablesRowSpan> {
        let metrics = row_metrics(&params.rows);
        crate::selected_variables_layout::compute_row_spans(&metrics)
    }

    fn add_waveforms(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if params.rows.is_empty() {
            return;
        }

        let layout = Self::compute_row_layout(params);

        for (index, row) in params.rows.iter().enumerate() {
            let row_top = layout[index].top_px;
            let row_height = layout[index].height_px;
            let divider_height = layout[index].divider_height_after_px;
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

            match row {
                RenderRowSnapshot::GroupHeader { name, .. } => {
                    objects.push(
                        Text::new()
                            .text(name.clone())
                            .position(10.0, row_top + (row_height / 2.0) - 6.0)
                            .size(
                                (params.canvas_width as f32 - 20.0).max(20.0),
                                row_height.max(12.0),
                            )
                            .color(
                                theme_colors.neutral_12.0,
                                theme_colors.neutral_12.1,
                                theme_colors.neutral_12.2,
                                0.62,
                            )
                            .font_size(12.0)
                            .family(Family::name("Inter"))
                            .into(),
                    );
                }
                RenderRowSnapshot::Variable(variable) => {
                    if Self::is_analog_signal(variable) {
                        Self::add_analog_signal(
                            objects,
                            variable,
                            row_top,
                            row_height,
                            params,
                            theme_colors,
                        );
                    } else {
                        Self::add_signal_segments(
                            objects,
                            variable,
                            row_top,
                            row_height,
                            params,
                            theme_colors,
                        );
                    }
                }
            }

            if divider_height > 0.0 {
                let separator_y = (row_top + row_height + (divider_height / 2.0))
                    .min(params.canvas_height as f32);
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
        if params.viewport_end_ps <= params.viewport_start_ps {
            return;
        }

        let range_ps = params.viewport_end_ps - params.viewport_start_ps;
        if range_ps == 0 {
            return;
        }

        let width_px = params.canvas_width as usize;
        if width_px == 0 {
            return;
        }

        let start_ps = params.viewport_start_ps;
        let end_ps = params.viewport_end_ps;
        let ps_per_pixel = range_ps as f64 / params.canvas_width.max(1) as f64;
        let mut pixel_states: Vec<Option<PixelValue>> = vec![None; width_px];
        let transition_values: Vec<Rc<String>> = variable
            .transitions
            .iter()
            .map(|t| Rc::new(t.value.clone()))
            .collect();

        for (index, transition) in variable.transitions.iter().enumerate() {
            let mut segment_start = transition.time_ns.saturating_mul(PS_PER_NS);
            if segment_start >= end_ps {
                break;
            }
            let next_time = if index + 1 < variable.transitions.len() {
                variable.transitions[index + 1]
                    .time_ns
                    .saturating_mul(PS_PER_NS)
            } else {
                end_ps
            };
            let mut segment_end = next_time;

            if segment_end <= start_ps {
                continue;
            }
            if segment_start < start_ps {
                segment_start = start_ps;
            }
            if segment_end > end_ps {
                segment_end = end_ps;
            }
            if segment_end <= segment_start {
                continue;
            }

            let start_px = ((segment_start - start_ps) as f64 / ps_per_pixel).floor() as isize;
            let end_px = ((segment_end - start_ps) as f64 / ps_per_pixel).ceil() as isize;
            let value_rc = transition_values[index].clone();

            for px in start_px..end_px {
                if px < 0 || px as usize >= width_px {
                    continue;
                }
                let entry = &mut pixel_states[px as usize];
                match entry {
                    None => {
                        *entry = Some(PixelValue::Single(value_rc.clone()));
                    }
                    Some(PixelValue::Single(existing)) => {
                        if existing.as_ref() != value_rc.as_ref() {
                            *entry = Some(PixelValue::Mixed);
                        }
                    }
                    Some(PixelValue::Mixed) => {}
                }
            }
        }

        let mut run_start = 0usize;
        let mut absolute_segment_index = 0usize;
        let mut current_state = if width_px > 0 {
            pixel_states[0].clone()
        } else {
            None
        };

        for idx in 1..=width_px {
            let state = if idx < width_px {
                pixel_states[idx].clone()
            } else {
                None
            };

            if !Self::pixel_state_equal(current_state.as_ref(), state.as_ref()) {
                if let Some(pixel_state) = current_state.clone() {
                    Self::draw_pixel_run(
                        objects,
                        pixel_state.clone(),
                        run_start,
                        idx,
                        row_top,
                        row_height,
                        theme_colors,
                        absolute_segment_index,
                        variable.formatter,
                    );
                    if !matches!(pixel_state, PixelValue::Mixed) {
                        absolute_segment_index += 1;
                    }
                }
                run_start = idx;
                current_state = state;
            }
        }
    }

    fn pixel_state_equal(a: Option<&PixelValue>, b: Option<&PixelValue>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(PixelValue::Mixed), Some(PixelValue::Mixed)) => true,
            (Some(PixelValue::Single(av)), Some(PixelValue::Single(bv))) => {
                Rc::ptr_eq(av, bv) || av.as_ref() == bv.as_ref()
            }
            _ => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_pixel_run(
        objects: &mut Vec<Object2d>,
        pixel_state: PixelValue,
        start_px: usize,
        end_px: usize,
        row_top: f32,
        row_height: f32,
        theme_colors: &ThemeColors,
        segment_index: usize,
        formatter: VarFormat,
    ) {
        if end_px <= start_px {
            return;
        }

        let rect_start_x = start_px as f32;
        let rect_width = (end_px - start_px) as f32;
        if rect_width <= 0.0 {
            return;
        }

        match pixel_state {
            PixelValue::Mixed => {
                let rect_top = row_top + 2.0;
                let rect_height = row_height - 4.0;
                let highlight = (226, 119, 40, 0.58);
                objects.push(
                    Rectangle::new()
                        .position(rect_start_x, rect_top)
                        .size(rect_width, rect_height.max(1.5))
                        .color(highlight.0, highlight.1, highlight.2, highlight.3)
                        .into(),
                );

                if rect_start_x > 0.5 {
                    objects.push(
                        Rectangle::new()
                            .position(rect_start_x, rect_top)
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
            }
            PixelValue::Single(value) => {
                let value_str = value.as_ref();
                let state = Self::classify_signal_state(value_str);
                if state == SignalState::Missing {
                    return;
                }

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
                        Self::regular_value_color(value_str, formatter, theme_colors),
                    ),
                    SignalState::Missing => unreachable!(),
                };

                let color = if segment_index % 2 == 0 {
                    base_color
                } else {
                    Self::tint_color(base_color, theme_colors.segment_alt_multiplier)
                };

                objects.push(
                    Rectangle::new()
                        .position(rect_start_x, rect_top)
                        .size(rect_width, rect_height.max(1.5))
                        .color(color.0, color.1, color.2, color.3)
                        .into(),
                );

                if rect_start_x > 0.5 {
                    objects.push(
                        Rectangle::new()
                            .position(rect_start_x, rect_top)
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

                if rect_width > 18.0 && row_height > 14.0 {
                    let text_color = theme_colors.neutral_12;
                    let formatted_value =
                        SignalValue::Present(value_str.clone()).get_formatted(&formatter);
                    let text = Self::truncate_value_text(&formatted_value, rect_width as usize / 7);
                    let text_top = rect_top + rect_height / 2.0 - 6.0;
                    objects.push(
                        Text::new()
                            .text(text)
                            .position(rect_start_x + 4.0, text_top.max(row_top + 2.0))
                            .size(rect_width - 8.0, rect_height.max(12.0))
                            .color(text_color.0, text_color.1, text_color.2, text_color.3)
                            .font_size(13.0)
                            .family(Family::name("Fira Code"))
                            .into(),
                    );
                }
            }
        }
    }

    fn is_analog_signal(variable: &VariableRenderSnapshot) -> bool {
        variable.signal_type.as_deref() == Some("Real")
    }

    fn parse_analog_value(value_str: &str) -> Option<f64> {
        let v: f64 = value_str.trim().parse().ok()?;
        if v.is_nan() || v.is_infinite() {
            None
        } else {
            Some(v)
        }
    }

    fn compute_analog_range(
        transitions: &[SignalTransition],
        actual_time_range_ns: Option<(u64, u64)>,
        viewport_start_ps: u64,
        viewport_end_ps: u64,
        analog_limits: Option<&AnalogLimits>,
    ) -> Option<(f64, f64)> {
        if let Some(limits) = analog_limits {
            if !limits.auto
                && limits.min.is_finite()
                && limits.max.is_finite()
                && limits.min < limits.max
            {
                return Some((limits.min, limits.max));
            }
        }

        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut found = false;
        let actual_end_ps =
            actual_time_range_ns.map(|(_, end_ns)| end_ns.saturating_mul(PS_PER_NS));
        for (index, t) in transitions.iter().enumerate() {
            let start_ps = t.time_ns.saturating_mul(PS_PER_NS);
            let end_ps = transitions
                .get(index + 1)
                .map(|next| next.time_ns.saturating_mul(PS_PER_NS))
                .or(actual_end_ps)
                .unwrap_or(viewport_end_ps);

            if end_ps <= viewport_start_ps {
                continue;
            }
            if start_ps >= viewport_end_ps {
                break;
            }
            if let Some(v) = Self::parse_analog_value(&t.value) {
                if v < min {
                    min = v;
                }
                if v > max {
                    max = v;
                }
                found = true;
            }
        }
        if found { Some((min, max)) } else { None }
    }

    fn add_analog_signal(
        objects: &mut Vec<Object2d>,
        variable: &VariableRenderSnapshot,
        row_top: f32,
        row_height: f32,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if Self::analog_visible_span_width_px(
            &variable.transitions,
            variable.actual_time_range_ns,
            params.viewport_start_ps,
            params.viewport_end_ps,
            params.canvas_width,
        )
        .is_some_and(|width| width < 2.0)
        {
            Self::add_analog_zoom_hint(objects, row_top, row_height, params, theme_colors);
            return;
        }

        let range = match Self::compute_analog_range(
            &variable.transitions,
            variable.actual_time_range_ns,
            params.viewport_start_ps,
            params.viewport_end_ps,
            variable.analog_limits.as_ref(),
        ) {
            Some(r) => r,
            None => return,
        };
        let (min_val, max_val) = range;
        let value_range = max_val - min_val;
        let margin = 4.0f32;
        let draw_top = row_top + margin;
        let draw_height = (row_height - 2.0 * margin).max(1.0);

        let start_ps = params.viewport_start_ps;
        let end_ps = params.viewport_end_ps;
        let range_ps = (end_ps - start_ps) as f64;
        if range_ps <= 0.0 {
            return;
        }

        let value_to_y = |val: f64| -> f32 {
            if value_range.abs() < 1e-30 {
                draw_top + draw_height / 2.0
            } else {
                let normalized = (val - min_val) / value_range;
                draw_top + draw_height * (1.0 - normalized as f32)
            }
        };

        let time_to_x = |time_ns: u64| -> f32 {
            let time_ps = time_ns.saturating_mul(PS_PER_NS);
            let ratio = (time_ps.saturating_sub(start_ps)) as f64 / range_ps;
            (ratio * params.canvas_width as f64) as f32
        };
        let time_ps_to_x = |time_ps: u64| -> f32 {
            let ratio = (time_ps.saturating_sub(start_ps)) as f64 / range_ps;
            (ratio * params.canvas_width as f64) as f32
        };
        let actual_end_ps = variable
            .actual_time_range_ns
            .map(|(_, end_ns)| end_ns.saturating_mul(PS_PER_NS));

        let mut points: Vec<(f32, f32)> = Vec::new();

        for (i, transition) in variable.transitions.iter().enumerate() {
            let time_ps = transition.time_ns.saturating_mul(PS_PER_NS);
            let val = match Self::parse_analog_value(&transition.value) {
                Some(v) => v,
                None => continue,
            };

            let next_time_ps = if i + 1 < variable.transitions.len() {
                variable.transitions[i + 1]
                    .time_ns
                    .saturating_mul(PS_PER_NS)
            } else {
                actual_end_ps.unwrap_or(end_ps)
            };

            if next_time_ps <= start_ps {
                continue;
            }
            if time_ps >= end_ps {
                break;
            }

            let y = value_to_y(val);

            if points.is_empty() {
                let x_start = if time_ps < start_ps {
                    0.0
                } else {
                    time_to_x(transition.time_ns)
                };
                points.push((x_start, y));
            } else {
                let x = time_to_x(transition.time_ns);
                points.push((x, points.last().unwrap().1));
                points.push((x, y));
            }

            let x_end = if next_time_ps > end_ps {
                params.canvas_width as f32
            } else {
                time_ps_to_x(next_time_ps)
            };
            points.push((x_end, y));
        }

        if points.len() >= 2 {
            let color = theme_colors.value_analog_color;
            objects.push(
                Line::new()
                    .points(&points)
                    .width(1.5)
                    .color(color.0, color.1, color.2, color.3)
                    .into(),
            );
        }
    }

    fn analog_visible_span_width_px(
        transitions: &[SignalTransition],
        actual_time_range_ns: Option<(u64, u64)>,
        viewport_start_ps: u64,
        viewport_end_ps: u64,
        canvas_width_px: u32,
    ) -> Option<f64> {
        if transitions.is_empty() || viewport_end_ps <= viewport_start_ps || canvas_width_px == 0 {
            return None;
        }

        let mut visible_start_ps = u64::MAX;
        let mut visible_end_ps = 0_u64;
        let mut found_segment = false;
        let actual_end_ps =
            actual_time_range_ns.map(|(_, end_ns)| end_ns.saturating_mul(PS_PER_NS));

        for (index, transition) in transitions.iter().enumerate() {
            let segment_start_ps = transition.time_ns.saturating_mul(PS_PER_NS);
            let segment_end_ps = transitions
                .get(index + 1)
                .map(|next| next.time_ns.saturating_mul(PS_PER_NS))
                .or(actual_end_ps)
                .unwrap_or(viewport_end_ps);

            if segment_end_ps <= viewport_start_ps {
                continue;
            }
            if segment_start_ps >= viewport_end_ps {
                break;
            }

            let clamped_start = segment_start_ps.max(viewport_start_ps);
            let clamped_end = segment_end_ps.min(viewport_end_ps);
            if clamped_end <= clamped_start {
                continue;
            }

            visible_start_ps = visible_start_ps.min(clamped_start);
            visible_end_ps = visible_end_ps.max(clamped_end);
            found_segment = true;
        }

        if !found_segment || visible_end_ps <= visible_start_ps {
            return None;
        }

        let viewport_span_ps = (viewport_end_ps - viewport_start_ps) as f64;
        let visible_span_ps = (visible_end_ps - visible_start_ps) as f64;
        Some((visible_span_ps / viewport_span_ps) * canvas_width_px as f64)
    }

    fn add_analog_zoom_hint(
        objects: &mut Vec<Object2d>,
        row_top: f32,
        row_height: f32,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        objects.push(
            Text::new()
                .text("Zoom in to inspect analog waveform")
                .position(10.0, row_top + (row_height / 2.0) - 5.0)
                .size(
                    (params.canvas_width as f32 - 20.0).max(20.0),
                    row_height.max(12.0),
                )
                .color(
                    theme_colors.neutral_12.0,
                    theme_colors.neutral_12.1,
                    theme_colors.neutral_12.2,
                    0.5,
                )
                .font_size(10.0)
                .family(Family::name("Inter"))
                .into(),
        );
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

        if normalized.len() == 1 {
            if normalized == "1" {
                return theme_colors.value_high_color;
            }
            if normalized == "0" {
                return theme_colors.value_low_color;
            }
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
        if params.viewport_end_ps <= params.viewport_start_ps {
            return;
        }
        let range_ps = (params.viewport_end_ps - params.viewport_start_ps) as f64;

        if let Some(cursor_ps) = params.cursor_position_ps {
            if (params.viewport_start_ps..=params.viewport_end_ps).contains(&cursor_ps) {
                let ratio = (cursor_ps - params.viewport_start_ps) as f64 / range_ps;
                let x = (ratio * params.canvas_width as f64) as f32;
                let (r, g, b, a) = theme_colors.cursor_color;
                objects.push(
                    Rectangle::new()
                        .position(x - 1.0, 0.0)
                        .size(3.0, params.canvas_height as f32)
                        .color(r, g, b, a)
                        .into(),
                );
            }
        }

        if let Some(center_ps) = params.zoom_center_ps {
            if (params.viewport_start_ps..=params.viewport_end_ps).contains(&center_ps) {
                let ratio = (center_ps - params.viewport_start_ps) as f64 / range_ps;
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

    fn add_marker_lines(objects: &mut Vec<Object2d>, params: &RenderingParameters) {
        if params.viewport_end_ps <= params.viewport_start_ps || params.markers.is_empty() {
            return;
        }
        let range_ps = (params.viewport_end_ps - params.viewport_start_ps) as f64;
        let marker_color = (0u8, 220u8, 220u8, 0.85f32);
        let canvas_height = params.canvas_height as f32;
        let layout = Self::compute_row_layout(params);
        let timeline_y = layout
            .last()
            .map(|span| span.top_px + span.height_px + span.divider_height_after_px)
            .unwrap_or(0.0);
        let timeline_height = (canvas_height - timeline_y).max(1.0);
        let mut markers = params.markers.clone();
        markers.sort_by_key(|marker| marker.time_ps);
        let lane_height = 14.0f32;
        let lane_gap = 2.0f32;
        let lane_capacity =
            (((timeline_height + lane_gap) / (lane_height + lane_gap)).floor() as usize).max(1);
        let lane_count = lane_capacity.min(3);
        let mut lane_end_x = vec![0.0f32; lane_count];

        for marker in &markers {
            if !(params.viewport_start_ps..=params.viewport_end_ps).contains(&marker.time_ps) {
                continue;
            }
            let ratio = (marker.time_ps - params.viewport_start_ps) as f64 / range_ps;
            let x = (ratio * params.canvas_width as f64) as f32;

            objects.push(
                Rectangle::new()
                    .position(x - 0.5, 0.0)
                    .size(1.0, canvas_height)
                    .color(
                        marker_color.0,
                        marker_color.1,
                        marker_color.2,
                        marker_color.3,
                    )
                    .into(),
            );

            let label_width = (marker.name.chars().count() as f32 * 6.5 + 16.0).clamp(36.0, 180.0);
            let label_x = (x + 4.0).min((params.canvas_width as f32 - label_width - 2.0).max(0.0));
            if let Some((lane_index, lane_end)) = lane_end_x
                .iter_mut()
                .enumerate()
                .find(|(_, lane_end)| label_x >= **lane_end)
            {
                let label_y = (canvas_height - lane_height - 2.0)
                    - lane_index as f32 * (lane_height + lane_gap);
                if label_y >= timeline_y && label_y + lane_height <= canvas_height {
                    objects.push(
                        Text::new()
                            .text(marker.name.clone())
                            .position(label_x, label_y)
                            .size(label_width, lane_height)
                            .color(marker_color.0, marker_color.1, marker_color.2, 1.0)
                            .font_size(11.0)
                            .family(Family::name("Inter"))
                            .into(),
                    );
                    *lane_end = label_x + label_width + 6.0;
                }
            }
        }
    }

    fn add_timeline_row(
        objects: &mut Vec<Object2d>,
        params: &RenderingParameters,
        theme_colors: &ThemeColors,
    ) {
        if params.viewport_end_ps <= params.viewport_start_ps {
            return;
        }

        let layout = Self::compute_row_layout(params);
        let timeline_y = layout
            .last()
            .map(|span| span.top_px + span.height_px + span.divider_height_after_px)
            .unwrap_or(0.0);
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

        let start_s = params.viewport_start_ps as f64 / PS_PER_SECOND as f64;
        let end_s = params.viewport_end_ps as f64 / PS_PER_SECOND as f64;
        let time_range_s = (end_s - start_s).max(1e-12);
        let time_range_ps = (params.viewport_end_ps - params.viewport_start_ps) as f64;

        let target_tick_spacing = 80.0;
        let desired_tick_count =
            (params.canvas_width as f64 / target_tick_spacing).clamp(2.0, 12.0);
        let raw_step_s = time_range_s / desired_tick_count.max(1.0);
        let step_s = Self::round_to_nice_number(raw_step_s);
        let step_ps = step_s * PS_PER_SECOND as f64;
        let label_unit = Self::select_time_unit(step_ps, time_range_ps);

        let mut ticks: Vec<(f32, Option<String>)> = Vec::new();

        ticks.push((
            0.0,
            Some(Self::format_time_label(
                params.viewport_start_ps,
                label_unit,
            )),
        ));

        let first_tick_s = (start_s / step_s).ceil() * step_s;
        let mut tick_s = first_tick_s;
        while tick_s < end_s {
            let tick_ps = (tick_s * PS_PER_SECOND as f64).round() as u64;
            let ratio = (tick_ps - params.viewport_start_ps) as f64 / time_range_ps;
            let x = (ratio * params.canvas_width as f64) as f32;

            if x > 0.0 && x < params.canvas_width as f32 {
                ticks.push((x, Some(Self::format_time_label(tick_ps, label_unit))));
            }

            tick_s += step_s;
        }

        ticks.push((
            params.canvas_width as f32,
            Some(Self::format_time_label(params.viewport_end_ps, label_unit)),
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

    fn select_time_unit(step_ps: f64, range_ps: f64) -> TimeLabelUnit {
        let candidates = [
            TimeLabelUnit::Seconds,
            TimeLabelUnit::Milliseconds,
            TimeLabelUnit::Microseconds,
            TimeLabelUnit::Nanoseconds,
            TimeLabelUnit::Picoseconds,
        ];

        for unit in candidates {
            let unit_scale = unit.base_ps();
            let range_value = range_ps / unit_scale;
            let step_value = step_ps / unit_scale;
            if range_value >= 1.0 && step_value >= 0.1 {
                return unit;
            }
        }

        TimeLabelUnit::Picoseconds
    }

    fn format_time_label(ps: u64, unit: TimeLabelUnit) -> String {
        let value = ps as f64 / unit.base_ps();
        let mut formatted = Self::format_axis_number(value);
        formatted.push_str(unit.suffix());
        formatted
    }

    fn format_axis_number(value: f64) -> String {
        let mut s = if value.abs() >= 100.0 {
            format!("{:.0}", value.round())
        } else if value.abs() >= 10.0 {
            format!("{:.1}", value)
        } else if value.abs() >= 1.0 {
            format!("{:.2}", value)
        } else {
            format!("{:.3}", value)
        };

        if let Some(pos) = s.find('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.len() > pos && s.ends_with('.') {
                s.pop();
            }
        }

        s
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
                value_analog_color: (40, 170, 200, 0.95),
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
                value_analog_color: (20, 140, 180, 0.95),
            },
        }
    }

    fn get_current_time_ms() -> f32 {
        (js_sys::Date::now()) as f32
    }
}

impl TimeLabelUnit {
    fn base_ps(self) -> f64 {
        match self {
            TimeLabelUnit::Seconds => PS_PER_SECOND as f64,
            TimeLabelUnit::Milliseconds => PS_PER_MS as f64,
            TimeLabelUnit::Microseconds => PS_PER_US as f64,
            TimeLabelUnit::Nanoseconds => PS_PER_NS as f64,
            TimeLabelUnit::Picoseconds => 1.0,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            TimeLabelUnit::Seconds => "s",
            TimeLabelUnit::Milliseconds => "ms",
            TimeLabelUnit::Microseconds => "us",
            TimeLabelUnit::Nanoseconds => "ns",
            TimeLabelUnit::Picoseconds => "ps",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WaveformRenderer;
    use shared::{AnalogLimits, SignalTransition};

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

    #[test]
    fn manual_analog_limits_override_visible_window_range() {
        let transitions = vec![
            SignalTransition::new(0, "1.0".to_string()),
            SignalTransition::new(10, "3.0".to_string()),
        ];

        let range = WaveformRenderer::compute_analog_range(
            &transitions,
            None,
            0,
            20_000,
            Some(&AnalogLimits::manual(-5.0, 5.0)),
        );

        assert_eq!(range, Some((-5.0, 5.0)));
    }

    #[test]
    fn auto_analog_range_uses_only_visible_window() {
        let transitions = vec![
            SignalTransition::new(0, "1.0".to_string()),
            SignalTransition::new(10, "10.0".to_string()),
            SignalTransition::new(20, "100.0".to_string()),
        ];

        let range = WaveformRenderer::compute_analog_range(
            &transitions,
            None,
            10_000,
            20_000,
            Some(&AnalogLimits::auto()),
        );

        assert_eq!(range, Some((10.0, 10.0)));
    }

    #[test]
    fn analog_visible_span_width_tracks_compressed_segments() {
        let transitions = vec![
            SignalTransition::new(0, "0.0".to_string()),
            SignalTransition::new(1, "1.0".to_string()),
        ];

        let visible_width = WaveformRenderer::analog_visible_span_width_px(
            &transitions,
            Some((0, 1)),
            0,
            1_000_000,
            100,
        );

        assert!(visible_width.is_some());
        assert!(visible_width.unwrap() < 2.0);
    }
}
