use crate::config::{AppConfig, TimeRange, TimelineState};
use crate::connection::ConnectionAdapter;
use crate::dataflow::{Actor, Relay, relay};
use crate::selected_variables::SelectedVariables;
use crate::visualizer::timeline::maximum_timeline_range::MaximumTimelineRange;
use crate::visualizer::timeline::time_domain::{NsPerPixel, TimeNs, Viewport};
use futures::{FutureExt, StreamExt, select};
use gloo_timers::callback::Timeout;
use shared::{
    SignalTransition, SignalValue, UnifiedSignalData, UnifiedSignalRequest, UpMsg, VarFormat,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use zoon::*;

#[derive(Clone, Debug)]
pub struct TimelineVariableSeries {
    pub unique_id: String,
    pub formatter: VarFormat,
    pub transitions: Vec<SignalTransition>,
    pub total_transitions: usize,
    pub cursor_value: Option<SignalValue>,
}

impl TimelineVariableSeries {
    pub fn empty(unique_id: String, formatter: VarFormat) -> Self {
        Self {
            unique_id,
            formatter,
            transitions: Vec::new(),
            total_transitions: 0,
            cursor_value: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineRenderState {
    pub viewport_start: TimeNs,
    pub viewport_end: TimeNs,
    pub cursor: TimeNs,
    pub zoom_center: TimeNs,
    pub canvas_width_px: u32,
    pub canvas_height_px: u32,
    pub ns_per_pixel: NsPerPixel,
    pub variables: Vec<TimelineVariableSeries>,
}

impl Default for TimelineRenderState {
    fn default() -> Self {
        Self {
            viewport_start: TimeNs::ZERO,
            viewport_end: TimeNs::from_nanos(1_000_000_000),
            cursor: TimeNs::ZERO,
            zoom_center: TimeNs::ZERO,
            canvas_width_px: 1,
            canvas_height_px: 1,
            ns_per_pixel: NsPerPixel(1_000_000),
            variables: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct TimelineBounds {
    start: TimeNs,
    end: TimeNs,
}

impl Default for TimelineBounds {
    fn default() -> Self {
        Self {
            start: TimeNs::ZERO,
            end: TimeNs::ZERO,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RequestContext {
    latest_request_id: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct VariableSeriesData {
    transitions: Vec<SignalTransition>,
    total_transitions: usize,
}

/// Primary timeline actor coordinating cursor, viewport, zoom and data requests.
#[derive(Clone)]
pub struct WaveformTimeline {
    cursor: Actor<TimeNs>,
    viewport: Actor<Viewport>,
    zoom_center: Actor<TimeNs>,
    canvas_width: Actor<f32>,
    canvas_height: Actor<f32>,
    shift_active: Actor<bool>,
    render_state: Actor<TimelineRenderState>,
    series_map: Actor<BTreeMap<String, VariableSeriesData>>,
    cursor_values: Actor<BTreeMap<String, SignalValue>>,
    request_state: Actor<RequestContext>,

    selected_variables: SelectedVariables,
    maximum_range: MaximumTimelineRange,
    connection: ConnectionAdapter,
    app_config: AppConfig,
    request_counter: Arc<AtomicU64>,
    bounds_state: Mutable<Option<TimelineBounds>>,
    request_debounce: Rc<RefCell<Option<Timeout>>>,
    config_debounce: Rc<RefCell<Option<Timeout>>>,
    viewport_initialized: Mutable<bool>,

    pub left_key_pressed_relay: Relay<()>,
    pub right_key_pressed_relay: Relay<()>,
    pub zoom_in_pressed_relay: Relay<()>,
    pub zoom_out_pressed_relay: Relay<()>,
    pub pan_left_pressed_relay: Relay<()>,
    pub pan_right_pressed_relay: Relay<()>,
    pub jump_to_previous_pressed_relay: Relay<()>,
    pub jump_to_next_pressed_relay: Relay<()>,
    pub reset_zoom_center_pressed_relay: Relay<()>,
    pub reset_zoom_pressed_relay: Relay<()>,
    pub shift_key_pressed_relay: Relay<()>,
    pub shift_key_released_relay: Relay<()>,
    pub canvas_resized_relay: Relay<(f32, f32)>,
    pub cursor_clicked_relay: Relay<TimeNs>,
    pub zoom_center_follow_mouse_relay: Relay<Option<TimeNs>>,
    pub variable_format_updated_relay: Relay<(String, VarFormat)>,
}

impl WaveformTimeline {
    pub async fn new(
        selected_variables: SelectedVariables,
        maximum_range: MaximumTimelineRange,
        connection: ConnectionAdapter,
        app_config: AppConfig,
    ) -> Self {
        let cursor = Actor::new(TimeNs::ZERO, |_state| async move {});
        let viewport = Actor::new(
            Viewport::new(TimeNs::ZERO, TimeNs::from_nanos(1_000_000_000)),
            |_state| async move {},
        );
        let zoom_center = Actor::new(TimeNs::ZERO, |_state| async move {});
        let canvas_width = Actor::new(800.0, |_state| async move {});
        let canvas_height = Actor::new(400.0, |_state| async move {});
        let shift_active = Actor::new(false, |_state| async move {});
        let render_state = Actor::new(TimelineRenderState::default(), |_state| async move {});
        let series_map = Actor::new(BTreeMap::new(), |_state| async move {});
        let cursor_values = Actor::new(BTreeMap::new(), |_state| async move {});
        let request_state = Actor::new(RequestContext::default(), |_state| async move {});

        let (left_key_pressed_relay, left_key_stream) = relay::<()>();
        let (right_key_pressed_relay, right_key_stream) = relay::<()>();
        let (zoom_in_pressed_relay, zoom_in_stream) = relay::<()>();
        let (zoom_out_pressed_relay, zoom_out_stream) = relay::<()>();
        let (pan_left_pressed_relay, pan_left_stream) = relay::<()>();
        let (pan_right_pressed_relay, pan_right_stream) = relay::<()>();
        let (jump_to_previous_pressed_relay, jump_prev_stream) = relay::<()>();
        let (jump_to_next_pressed_relay, jump_next_stream) = relay::<()>();
        let (reset_zoom_center_pressed_relay, reset_center_stream) = relay::<()>();
        let (reset_zoom_pressed_relay, reset_zoom_stream) = relay::<()>();
        let (shift_key_pressed_relay, shift_pressed_stream) = relay::<()>();
        let (shift_key_released_relay, shift_released_stream) = relay::<()>();
        let (canvas_resized_relay, canvas_resized_stream) = relay::<(f32, f32)>();
        let (cursor_clicked_relay, cursor_clicked_stream) = relay::<TimeNs>();
        let (zoom_center_follow_mouse_relay, zoom_center_follow_stream) = relay::<Option<TimeNs>>();
        let (variable_format_updated_relay, format_updated_stream) = relay::<(String, VarFormat)>();

        let bounds_state = Mutable::new(None);
        let request_debounce = Rc::new(RefCell::new(None));
        let config_debounce = Rc::new(RefCell::new(None));
        let viewport_initialized = Mutable::new(false);

        let timeline = Self {
            cursor,
            viewport,
            zoom_center,
            canvas_width,
            canvas_height,
            shift_active,
            render_state,
            series_map,
            cursor_values,
            request_state,
            selected_variables,
            maximum_range,
            connection,
            app_config,
            request_counter: Arc::new(AtomicU64::new(1)),
            bounds_state,
            request_debounce,
            config_debounce,
            viewport_initialized,
            left_key_pressed_relay,
            right_key_pressed_relay,
            zoom_in_pressed_relay,
            zoom_out_pressed_relay,
            pan_left_pressed_relay,
            pan_right_pressed_relay,
            jump_to_previous_pressed_relay,
            jump_to_next_pressed_relay,
            reset_zoom_center_pressed_relay,
            reset_zoom_pressed_relay,
            shift_key_pressed_relay,
            shift_key_released_relay,
            canvas_resized_relay,
            cursor_clicked_relay,
            zoom_center_follow_mouse_relay,
            variable_format_updated_relay,
        };

        timeline.initialize_from_config().await;
        timeline.spawn_shift_tracking(shift_pressed_stream, shift_released_stream);
        timeline.spawn_canvas_resize_handler(canvas_resized_stream);
        timeline.spawn_cursor_navigation(
            left_key_stream,
            right_key_stream,
            cursor_clicked_stream,
            jump_prev_stream,
            jump_next_stream,
        );
        timeline.spawn_zoom_and_pan_handlers(
            zoom_in_stream,
            zoom_out_stream,
            pan_left_stream,
            pan_right_stream,
            reset_zoom_stream,
            reset_center_stream,
            zoom_center_follow_stream,
        );
        timeline.spawn_variable_format_handler(format_updated_stream);
        timeline.spawn_selected_variables_listener();
        timeline.spawn_bounds_listener();
        timeline.spawn_request_triggers();

        timeline.schedule_request();

        timeline
    }

    pub fn render_state_actor(&self) -> Actor<TimelineRenderState> {
        self.render_state.clone()
    }

    pub fn cursor_actor(&self) -> Actor<TimeNs> {
        self.cursor.clone()
    }

    pub fn viewport_actor(&self) -> Actor<Viewport> {
        self.viewport.clone()
    }

    pub fn zoom_center_actor(&self) -> Actor<TimeNs> {
        self.zoom_center.clone()
    }

    pub fn canvas_width_actor(&self) -> Actor<f32> {
        self.canvas_width.clone()
    }

    pub fn cursor_values_actor(&self) -> Actor<BTreeMap<String, SignalValue>> {
        self.cursor_values.clone()
    }

    fn bounds(&self) -> Option<TimelineBounds> {
        self.bounds_state.get_cloned()
    }

    fn viewport_duration_ns(&self) -> u64 {
        self.viewport.state.get_cloned().duration().nanos().max(1)
    }

    fn clamp_to_bounds(&self, time: TimeNs) -> TimeNs {
        if let Some(bounds) = self.bounds() {
            let clamped = time.nanos().clamp(bounds.start.nanos(), bounds.end.nanos());
            TimeNs::from_nanos(clamped)
        } else {
            time
        }
    }

    fn set_cursor_clamped(&self, time: TimeNs) {
        let clamped = self.clamp_to_bounds(time);
        self.cursor.state.set_neq(clamped);
        self.ensure_cursor_within_viewport();
        self.update_render_state();
        self.schedule_request();
        self.schedule_config_save();
    }

    fn move_cursor_left(&self) {
        let faster = self.shift_active.state.get_cloned();
        let step = self.cursor_step_ns(faster);
        let current = self.cursor.state.get_cloned().nanos();
        let new_time = current.saturating_sub(step);
        self.set_cursor_clamped(TimeNs::from_nanos(new_time));
    }

    fn move_cursor_right(&self) {
        let faster = self.shift_active.state.get_cloned();
        let step = self.cursor_step_ns(faster);
        let current = self.cursor.state.get_cloned().nanos();
        let new_time = current.saturating_add(step);
        self.set_cursor_clamped(TimeNs::from_nanos(new_time));
    }

    fn cursor_step_ns(&self, faster: bool) -> u64 {
        let duration = self.viewport_duration_ns();
        let width = self.canvas_width.state.get_cloned().max(1.0) as u64;
        let mut step = (duration / width.max(1)).max(1_000);
        if faster {
            step = step.saturating_mul(10);
        }
        step
    }

    fn jump_to_previous_transition(&self) {
        let times = self.collect_sorted_transition_times();
        if times.is_empty() {
            return;
        }
        let cursor_ns = self.cursor.state.get_cloned().nanos();
        if let Some(prev) = times.iter().rev().find(|&&t| t < cursor_ns).copied() {
            self.set_cursor_clamped(TimeNs::from_nanos(prev));
        }
    }

    fn jump_to_next_transition(&self) {
        let times = self.collect_sorted_transition_times();
        if times.is_empty() {
            return;
        }
        let cursor_ns = self.cursor.state.get_cloned().nanos();
        if let Some(next) = times.iter().find(|&&t| t > cursor_ns).copied() {
            self.set_cursor_clamped(TimeNs::from_nanos(next));
        }
    }

    fn collect_sorted_transition_times(&self) -> Vec<u64> {
        let map = self.series_map.state.lock_ref();
        let mut times = Vec::new();
        for series in map.values() {
            for transition in &series.transitions {
                times.push(transition.time_ns);
            }
        }
        drop(map);
        times.sort_unstable();
        times.dedup();
        times
    }

    fn zoom_in(&self, faster: bool) {
        let width = self.canvas_width.state.get_cloned().max(1.0) as u64;
        if width == 0 {
            return;
        }
        let viewport = self.viewport.state.get_cloned();
        let current_duration = viewport.duration().nanos();
        if current_duration <= width {
            return;
        }
        let factor = if faster { 0.4 } else { 0.7 };
        let mut new_duration = ((current_duration as f64) * factor).round() as u64;
        new_duration = new_duration.max(width);
        self.set_viewport_with_duration(self.zoom_center.state.get_cloned(), new_duration);
    }

    fn zoom_out(&self, faster: bool) {
        let width = self.canvas_width.state.get_cloned().max(1.0) as u64;
        let viewport = self.viewport.state.get_cloned();
        let current_duration = viewport.duration().nanos();
        let factor = if faster { 1.8 } else { 1.3 };
        let mut new_duration = ((current_duration as f64) * factor).round() as u64;
        new_duration = new_duration.max(width);
        if let Some(bounds) = self.bounds() {
            let max_duration = bounds.end.duration_since(bounds.start).nanos();
            new_duration = new_duration.min(max_duration.max(width));
        }
        self.set_viewport_with_duration(self.zoom_center.state.get_cloned(), new_duration);
    }

    fn pan_left(&self, faster: bool) {
        let viewport = self.viewport.state.get_cloned();
        let duration = viewport.duration().nanos();
        let mut step = (duration / 5).max(1);
        if faster {
            step = step.saturating_mul(3);
        }
        let new_start = viewport.start.nanos().saturating_sub(step);
        let new_end = viewport.end.nanos().saturating_sub(step);
        self.set_viewport_clamped(
            TimeNs::from_nanos(new_start),
            TimeNs::from_nanos(new_end.max(new_start + 1)),
        );
    }

    fn pan_right(&self, faster: bool) {
        let viewport = self.viewport.state.get_cloned();
        let duration = viewport.duration().nanos();
        let mut step = (duration / 5).max(1);
        if faster {
            step = step.saturating_mul(3);
        }
        let new_start = viewport.start.nanos().saturating_add(step);
        let new_end = viewport.end.nanos().saturating_add(step);
        self.set_viewport_clamped(
            TimeNs::from_nanos(new_start),
            TimeNs::from_nanos(new_end.max(new_start + 1)),
        );
    }

    fn reset_zoom(&self) {
        if let Some(bounds) = self.bounds() {
            self.set_viewport_clamped(bounds.start, bounds.end);
            let start = bounds.start.nanos();
            let end = bounds.end.nanos();
            if end > start {
                let midpoint = start.saturating_add((end - start) / 2);
                self.cursor.state.set_neq(TimeNs::from_nanos(midpoint));
            } else {
                self.cursor.state.set_neq(bounds.start);
            }
        } else {
            self.viewport.state.set(Viewport::new(
                TimeNs::ZERO,
                TimeNs::from_nanos(1_000_000_000),
            ));
            self.cursor.state.set_neq(TimeNs::from_nanos(500_000_000));
        }
        self.set_zoom_center(TimeNs::ZERO);
        self.update_render_state();
        self.schedule_request();
        self.schedule_config_save();
    }

    fn reset_zoom_center(&self) {
        self.set_zoom_center(TimeNs::ZERO);
    }

    fn set_zoom_center(&self, time: TimeNs) {
        let clamped = self.clamp_to_bounds(time);
        self.zoom_center.state.set_neq(clamped);
        self.update_render_state();
    }

    fn set_viewport_with_duration(&self, center: TimeNs, duration_ns: u64) {
        let half = duration_ns / 2;
        let mut start_ns = center.nanos().saturating_sub(half);
        let mut end_ns = start_ns.saturating_add(duration_ns);

        if let Some(bounds) = self.bounds() {
            if end_ns > bounds.end.nanos() {
                let diff = end_ns - bounds.end.nanos();
                start_ns = start_ns.saturating_sub(diff);
                end_ns = bounds.end.nanos();
            }
            if start_ns < bounds.start.nanos() {
                let diff = bounds.start.nanos() - start_ns;
                start_ns = bounds.start.nanos();
                end_ns = start_ns.saturating_add(duration_ns).min(bounds.end.nanos());
            }
        }

        self.set_viewport_clamped(
            TimeNs::from_nanos(start_ns),
            TimeNs::from_nanos(end_ns.max(start_ns + 1)),
        );
    }

    fn set_viewport_clamped(&self, start: TimeNs, end: TimeNs) {
        let mut clamped_start = start;
        let mut clamped_end = end;
        if let Some(bounds) = self.bounds() {
            if clamped_start < bounds.start {
                let shift = bounds.start.nanos() - clamped_start.nanos();
                clamped_start = bounds.start;
                clamped_end = TimeNs::from_nanos(clamped_end.nanos().saturating_add(shift));
            }
            if clamped_end > bounds.end {
                let shift = clamped_end.nanos() - bounds.end.nanos();
                clamped_end = bounds.end;
                clamped_start = TimeNs::from_nanos(clamped_start.nanos().saturating_sub(shift));
            }
        }

        if clamped_end <= clamped_start {
            clamped_end = TimeNs::from_nanos(clamped_start.nanos() + 1);
        }

        self.viewport
            .state
            .set(Viewport::new(clamped_start, clamped_end));
        self.ensure_cursor_within_viewport();
        self.update_render_state();
        self.schedule_request();
        self.schedule_config_save();
    }

    fn ensure_viewport_within_bounds(&self) {
        if let Some(bounds) = self.bounds() {
            let viewport = self.viewport.state.get_cloned();
            let start = viewport.start.nanos();
            let end = viewport.end.nanos();
            let bounds_start = bounds.start.nanos();
            let bounds_end = bounds.end.nanos();

            if start < bounds_start || end > bounds_end {
                let clamped_start = start.clamp(bounds_start, bounds_end);
                let clamped_end = end.clamp(bounds_start, bounds_end);
                if clamped_end > clamped_start {
                    self.viewport.state.set(Viewport::new(
                        TimeNs::from_nanos(clamped_start),
                        TimeNs::from_nanos(clamped_end),
                    ));
                }
            }
        }
        self.ensure_cursor_within_viewport();
        self.update_render_state();
        self.schedule_config_save();
    }

    fn ensure_cursor_within_viewport(&self) {
        let viewport = self.viewport.state.get_cloned();
        let cursor = self.cursor.state.get_cloned();
        if cursor < viewport.start {
            self.cursor.state.set_neq(viewport.start);
        } else if cursor > viewport.end {
            self.cursor.state.set_neq(viewport.end);
        }
    }

    fn on_variable_format_updated(&self, unique_id: String, _format: VarFormat) {
        self.cursor_values
            .state
            .lock_mut()
            .insert(unique_id, SignalValue::Loading);
        self.update_render_state();
        self.schedule_request();
    }

    fn on_selected_variables_updated(&self, variables: Vec<shared::SelectedVariable>) {
        let desired: HashSet<_> = variables.iter().map(|var| var.unique_id.clone()).collect();

        {
            let mut map = self.series_map.state.lock_mut();
            map.retain(|key, _| desired.contains(key));
        }

        {
            let mut values_map = self.cursor_values.state.lock_mut();
            values_map.retain(|key, _| desired.contains(key));
            for variable in &variables {
                values_map
                    .entry(variable.unique_id.clone())
                    .or_insert(SignalValue::Loading);
            }
        }

        self.update_render_state();
        self.schedule_request();
        self.schedule_config_save();
    }

    fn determine_formatter(&self, unique_id: &str) -> VarFormat {
        let snapshot = self
            .selected_variables
            .variables_vec_actor
            .state
            .get_cloned();
        snapshot
            .into_iter()
            .find(|var| var.unique_id == unique_id)
            .and_then(|var| var.formatter)
            .unwrap_or(VarFormat::Hexadecimal)
    }

    fn schedule_request(&self) {
        if let Some(mut timer) = self.request_debounce.borrow_mut().take() {
            timer.cancel();
        }

        let debounce_slot = self.request_debounce.clone();
        let timeline = self.clone();
        let timeout = Timeout::new(1_000, move || {
            *debounce_slot.borrow_mut() = None;
            timeline.send_request();
        });
        *self.request_debounce.borrow_mut() = Some(timeout);
    }

    fn schedule_config_save(&self) {
        if let Some(mut timer) = self.config_debounce.borrow_mut().take() {
            timer.cancel();
        }

        let debounce_slot = self.config_debounce.clone();
        let timeline = self.clone();
        let timeout = Timeout::new(1_000, move || {
            *debounce_slot.borrow_mut() = None;
            timeline.sync_state_to_config();
        });
        *self.config_debounce.borrow_mut() = Some(timeout);
    }

    fn send_request(&self) {
        let variables = self
            .selected_variables
            .variables_vec_actor
            .state
            .get_cloned();

        if variables.is_empty() {
            self.series_map.state.lock_mut().clear();
            self.cursor_values.state.lock_mut().clear();
            self.update_render_state();
            return;
        }

        let viewport = self.viewport.state.get_cloned();
        let start_ns = viewport.start.nanos();
        let end_ns = viewport.end.nanos();
        if end_ns <= start_ns {
            return;
        }

        let width_px = self.canvas_width.state.get_cloned().max(1.0) as u32;
        if width_px == 0 {
            return;
        }

        let max_transitions = (width_px as usize).saturating_mul(4).max(1);

        let mut requests = Vec::with_capacity(variables.len());

        for variable in &variables {
            if let Some((file_path, scope_path, variable_name)) = variable.parse_unique_id() {
                let formatter = variable.formatter.unwrap_or(VarFormat::Hexadecimal);
                requests.push(UnifiedSignalRequest {
                    file_path,
                    scope_path,
                    variable_name,
                    time_range_ns: Some((start_ns, end_ns)),
                    max_transitions: Some(max_transitions),
                    format: formatter,
                });
            }
        }

        if requests.is_empty() {
            return;
        }

        {
            let mut values_map = self.cursor_values.state.lock_mut();
            for variable in &variables {
                values_map.insert(variable.unique_id.clone(), SignalValue::Loading);
            }
        }
        self.update_render_state();

        let cursor_ns = self.cursor.state.get_cloned().nanos();
        let request_id = format!(
            "timeline-{}",
            self.request_counter.fetch_add(1, Ordering::SeqCst)
        );
        self.request_state.state.set(RequestContext {
            latest_request_id: Some(request_id.clone()),
        });

        let connection = self.connection.clone();
        Task::start(async move {
            connection
                .send_up_msg(UpMsg::UnifiedSignalQuery {
                    signal_requests: requests,
                    cursor_time_ns: Some(cursor_ns),
                    request_id,
                })
                .await;
        });
    }

    pub fn apply_unified_signal_response(
        &self,
        request_id: &str,
        signal_data: Vec<UnifiedSignalData>,
        cursor_values: BTreeMap<String, SignalValue>,
    ) {
        let current_request = self.request_state.state.get_cloned();
        if current_request
            .latest_request_id
            .as_deref()
            .map(|id| id != request_id)
            .unwrap_or(true)
        {
            return;
        }

        {
            let mut map = self.series_map.state.lock_mut();
            for data in signal_data {
                map.insert(
                    data.unique_id.clone(),
                    VariableSeriesData {
                        transitions: data.transitions,
                        total_transitions: data.total_transitions,
                    },
                );
            }
        }

        {
            let mut values_map = self.cursor_values.state.lock_mut();
            for (unique_id, value) in cursor_values {
                values_map.insert(unique_id, value);
            }
        }

        self.update_render_state();
    }

    fn sync_state_to_config(&self) {
        let viewport = self.viewport.state.get_cloned();
        let cursor = self.cursor.state.get_cloned();
        let ns_per_pixel = self.render_state.state.get_cloned().ns_per_pixel;

        if viewport.end <= viewport.start {
            return;
        }

        let timeline_state = TimelineState {
            cursor_position: Some(cursor),
            visible_range: Some(TimeRange {
                start: viewport.start,
                end: viewport.end,
            }),
            zoom_level: Some(ns_per_pixel.nanos() as f64),
        };

        self.app_config
            .timeline_state_changed_relay
            .send(timeline_state);
    }

    pub fn handle_unified_signal_error(&self, request_id: &str, error: &str) {
        let current_request = self.request_state.state.get_cloned();
        if current_request
            .latest_request_id
            .as_deref()
            .map(|id| id == request_id)
            .unwrap_or(false)
        {
            zoon::println!("Unified signal request failed: {}", error);
        }
    }

    fn update_render_state(&self) {
        let viewport = self.viewport.state.get_cloned();
        let cursor = self.cursor.state.get_cloned();
        let zoom_center = self.zoom_center.state.get_cloned();
        let width = self.canvas_width.state.get_cloned().max(1.0) as u32;
        let height = self.canvas_height.state.get_cloned().max(1.0) as u32;
        let duration = viewport.duration().nanos();
        let ns_per_pixel = if width == 0 {
            NsPerPixel(1)
        } else {
            NsPerPixel((duration / width.max(1) as u64).max(1))
        };

        let variables_snapshot = self
            .selected_variables
            .variables_vec_actor
            .state
            .get_cloned();

        let series_guard = self.series_map.state.lock_ref();
        let values_guard = self.cursor_values.state.lock_ref();
        let mut variables = Vec::with_capacity(variables_snapshot.len());
        for variable in variables_snapshot {
            let formatter = variable.formatter.unwrap_or(VarFormat::Hexadecimal);
            let series_data = series_guard.get(&variable.unique_id);
            let cursor_value = values_guard.get(&variable.unique_id).cloned();
            match series_data {
                Some(series) => {
                    variables.push(TimelineVariableSeries {
                        unique_id: variable.unique_id.clone(),
                        formatter,
                        transitions: series.transitions.clone(),
                        total_transitions: series.total_transitions,
                        cursor_value,
                    });
                }
                None => {
                    let mut series =
                        TimelineVariableSeries::empty(variable.unique_id.clone(), formatter);
                    series.cursor_value = cursor_value;
                    variables.push(series);
                }
            }
        }
        drop(series_guard);
        drop(values_guard);

        self.render_state.state.set(TimelineRenderState {
            viewport_start: viewport.start,
            viewport_end: viewport.end,
            cursor,
            zoom_center,
            canvas_width_px: width,
            canvas_height_px: height,
            ns_per_pixel,
            variables,
        });
    }

    pub fn apply_cursor_values<I>(&self, values: I)
    where
        I: IntoIterator<Item = (String, SignalValue)>,
    {
        {
            let mut map = self.cursor_values.state.lock_mut();
            for (key, value) in values {
                map.insert(key, value);
            }
        }
        self.update_render_state();
    }

    async fn initialize_from_config(&self) {
        let stored_state = self
            .app_config
            .timeline_state_actor
            .signal()
            .to_stream()
            .next()
            .await
            .unwrap_or_default();

        if let Some(range) = stored_state.visible_range {
            self.viewport
                .state
                .set(Viewport::new(range.start, range.end));
            self.zoom_center.state.set_neq(range.start);
            self.viewport_initialized.set(true);
        } else {
            self.viewport_initialized.set(false);
        }

        if let Some(cursor) = stored_state.cursor_position {
            self.cursor.state.set_neq(cursor);
        } else {
            let viewport = self.viewport.state.get_cloned();
            self.cursor.state.set_neq(viewport.center());
        }

        self.update_render_state();
        self.schedule_config_save();
    }

    fn spawn_shift_tracking(
        &self,
        shift_pressed_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        shift_released_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
    ) {
        let shift_actor = self.shift_active.clone();
        Task::start(async move {
            let mut pressed = shift_pressed_stream.fuse();
            let mut released = shift_released_stream.fuse();

            loop {
                select! {
                    event = pressed.next() => {
                        match event {
                            Some(()) => shift_actor.state.set_neq(true),
                            None => break,
                        }
                    }
                    event = released.next() => {
                        match event {
                            Some(()) => shift_actor.state.set_neq(false),
                            None => break,
                        }
                    }
                }
            }
        });
    }

    fn spawn_canvas_resize_handler(
        &self,
        canvas_resized_stream: impl futures::Stream<Item = (f32, f32)> + Unpin + 'static,
    ) {
        let timeline = self.clone();
        Task::start(async move {
            let mut stream = canvas_resized_stream.fuse();
            while let Some((width, height)) = stream.next().await {
                timeline.canvas_width.state.set_neq(width.max(1.0));
                timeline.canvas_height.state.set_neq(height.max(1.0));
                timeline.update_render_state();
                timeline.schedule_request();
            }
        });
    }

    fn spawn_cursor_navigation(
        &self,
        left_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        right_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        cursor_clicked_stream: impl futures::Stream<Item = TimeNs> + Unpin + 'static,
        jump_prev_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        jump_next_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
    ) {
        let timeline = self.clone();
        Task::start(async move {
            let mut left = left_stream.fuse();
            let mut right = right_stream.fuse();
            let mut clicked = cursor_clicked_stream.fuse();
            let mut jump_prev = jump_prev_stream.fuse();
            let mut jump_next = jump_next_stream.fuse();

            loop {
                select! {
                    event = left.next() => {
                        match event {
                            Some(()) => timeline.move_cursor_left(),
                            None => break,
                        }
                    }
                    event = right.next() => {
                        match event {
                            Some(()) => timeline.move_cursor_right(),
                            None => break,
                        }
                    }
                    event = clicked.next() => {
                        match event {
                            Some(time) => timeline.set_cursor_clamped(time),
                            None => break,
                        }
                    }
                    event = jump_prev.next() => {
                        match event {
                            Some(()) => timeline.jump_to_previous_transition(),
                            None => break,
                        }
                    }
                    event = jump_next.next() => {
                        match event {
                            Some(()) => timeline.jump_to_next_transition(),
                            None => break,
                        }
                    }
                }
            }
        });
    }

    fn spawn_zoom_and_pan_handlers(
        &self,
        zoom_in_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        zoom_out_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        pan_left_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        pan_right_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        reset_zoom_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        reset_center_stream: impl futures::Stream<Item = ()> + Unpin + 'static,
        zoom_center_follow_stream: impl futures::Stream<Item = Option<TimeNs>> + Unpin + 'static,
    ) {
        let timeline = self.clone();
        Task::start(async move {
            let mut zoom_in = zoom_in_stream.fuse();
            let mut zoom_out = zoom_out_stream.fuse();
            let mut pan_left = pan_left_stream.fuse();
            let mut pan_right = pan_right_stream.fuse();
            let mut reset_zoom = reset_zoom_stream.fuse();
            let mut reset_center = reset_center_stream.fuse();
            let mut follow = zoom_center_follow_stream.fuse();

            loop {
                select! {
                    event = zoom_in.next() => {
                        match event {
                            Some(()) => {
                                let faster = timeline.shift_active.state.get_cloned();
                                timeline.zoom_in(faster);
                            }
                            None => break,
                        }
                    }
                    event = zoom_out.next() => {
                        match event {
                            Some(()) => {
                                let faster = timeline.shift_active.state.get_cloned();
                                timeline.zoom_out(faster);
                            }
                            None => break,
                        }
                    }
                    event = pan_left.next() => {
                        match event {
                            Some(()) => {
                                let faster = timeline.shift_active.state.get_cloned();
                                timeline.pan_left(faster);
                            }
                            None => break,
                        }
                    }
                    event = pan_right.next() => {
                        match event {
                            Some(()) => {
                                let faster = timeline.shift_active.state.get_cloned();
                                timeline.pan_right(faster);
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom.next() => {
                        match event {
                            Some(()) => timeline.reset_zoom(),
                            None => break,
                        }
                    }
                    event = reset_center.next() => {
                        match event {
                            Some(()) => timeline.reset_zoom_center(),
                            None => break,
                        }
                    }
                    event = follow.next() => {
                        match event {
                            Some(Some(time)) => timeline.set_zoom_center(time),
                            Some(None) => {},
                            None => break,
                        }
                    }
                }
            }
        });
    }

    fn spawn_variable_format_handler(
        &self,
        format_stream: impl futures::Stream<Item = (String, VarFormat)> + Unpin + 'static,
    ) {
        let timeline = self.clone();
        Task::start(async move {
            let mut stream = format_stream.fuse();
            while let Some((unique_id, format)) = stream.next().await {
                timeline.on_variable_format_updated(unique_id, format);
            }
        });
    }

    fn spawn_selected_variables_listener(&self) {
        let timeline = self.clone();
        Task::start(async move {
            let mut stream = timeline
                .selected_variables
                .variables_vec_actor
                .signal()
                .to_stream()
                .fuse();

            while let Some(variables) = stream.next().await {
                timeline.on_selected_variables_updated(variables);
            }
        });
    }

    fn spawn_bounds_listener(&self) {
        let timeline = self.clone();
        Task::start(async move {
            let mut stream = timeline.maximum_range.range.signal().to_stream().fuse();

            while let Some(maybe_range) = stream.next().await {
                if let Some((start, end)) = maybe_range {
                    let bounds = TimelineBounds { start, end };
                    timeline.bounds_state.set(Some(bounds.clone()));

                    if !timeline.viewport_initialized.get() {
                        timeline
                            .viewport
                            .state
                            .set(Viewport::new(bounds.start, bounds.end));

                        let start_ns = bounds.start.nanos();
                        let end_ns = bounds.end.nanos();
                        let midpoint = if end_ns > start_ns {
                            start_ns.saturating_add((end_ns - start_ns) / 2)
                        } else {
                            start_ns
                        };

                        timeline.cursor.state.set_neq(TimeNs::from_nanos(midpoint));
                        timeline.zoom_center.state.set_neq(bounds.start);
                        timeline.viewport_initialized.set(true);
                        timeline.update_render_state();
                        timeline.schedule_request();
                        timeline.schedule_config_save();
                    } else {
                        timeline.ensure_viewport_within_bounds();
                    }
                } else {
                    timeline.bounds_state.set(None);
                    timeline.viewport.state.set(Viewport::new(
                        TimeNs::ZERO,
                        TimeNs::from_nanos(1_000_000_000),
                    ));
                    timeline.update_render_state();
                    timeline.schedule_request();
                    timeline.schedule_config_save();
                    timeline.viewport_initialized.set(false);
                }
            }
        });
    }

    fn spawn_request_triggers(&self) {
        let timeline = self.clone();
        Task::start(async move {
            let mut cursor_stream = timeline.cursor.signal().to_stream().fuse();
            while cursor_stream.next().await.is_some() {
                timeline.update_render_state();
                timeline.schedule_request();
            }
        });

        let timeline = self.clone();
        Task::start(async move {
            let mut viewport_stream = timeline.viewport.signal().to_stream().fuse();
            while viewport_stream.next().await.is_some() {
                timeline.ensure_viewport_within_bounds();
                timeline.schedule_request();
            }
        });

        let timeline = self.clone();
        Task::start(async move {
            let mut width_stream = timeline.canvas_width.signal().to_stream().fuse();
            while width_stream.next().await.is_some() {
                timeline.update_render_state();
                timeline.schedule_request();
            }
        });
    }
}
