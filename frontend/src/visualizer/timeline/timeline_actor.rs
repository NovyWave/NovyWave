use crate::config::{AppConfig, TimeRange, TimelineState};
use crate::connection::ConnectionAdapter;
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::maximum_timeline_range::MaximumTimelineRange;
use crate::visualizer::timeline::time_domain::{
    FS_PER_PS, MIN_CURSOR_STEP_NS, PS_PER_NS, TimePerPixel, TimePs, Viewport,
};
use futures::{StreamExt, select};
use gloo_timers::callback::Timeout;
use js_sys::Date;
use shared::{
    SignalTransition, SignalValue, UnifiedSignalData, UnifiedSignalRequest, UpMsg, VarFormat,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use zoon::*;
use zoon::TaskHandle;

const REQUEST_DEBOUNCE_MS: u32 = 75;
const CURSOR_LOADING_DELAY_MS: u32 = 500;
const CONFIG_SAVE_DEBOUNCE_MS: u32 = 1_000;
const ZOOM_CENTER_MIN_INTERVAL_MS: f64 = 16.0;
const MIN_DURATION_PS: u64 = 1;
const MIN_TIME_PER_PIXEL_FS: u64 = 200;
const CURSOR_STEP_RATIO: f64 = 0.04;
const CURSOR_FAST_MULTIPLIER: u64 = 4;
const CACHE_HIT_THRESHOLD: f64 = 0.8;
const CACHE_MAX_SEGMENTS_PER_VARIABLE: usize = 2;

#[derive(Clone, Debug)]
pub struct TimelineVariableSeries {
    pub unique_id: String,
    pub formatter: VarFormat,
    pub transitions: Arc<Vec<SignalTransition>>,
    pub total_transitions: usize,
    pub cursor_value: Option<SignalValue>,
}

impl TimelineVariableSeries {
    pub fn empty(unique_id: String, formatter: VarFormat) -> Self {
        Self {
            unique_id,
            formatter,
            transitions: Arc::new(Vec::new()),
            total_transitions: 0,
            cursor_value: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineRenderState {
    pub viewport_start: TimePs,
    pub viewport_end: TimePs,
    pub cursor: TimePs,
    pub zoom_center: TimePs,
    pub canvas_width_px: u32,
    pub canvas_height_px: u32,
    pub time_per_pixel: TimePerPixel,
    pub variables: Vec<TimelineVariableSeries>,
}

impl Default for TimelineRenderState {
    fn default() -> Self {
        Self {
            viewport_start: TimePs::ZERO,
            viewport_end: TimePs::from_nanos(1_000_000_000),
            cursor: TimePs::ZERO,
            zoom_center: TimePs::ZERO,
            canvas_width_px: 1,
            canvas_height_px: 1,
            time_per_pixel: TimePerPixel::from_picoseconds(MIN_CURSOR_STEP_NS * PS_PER_NS),
            variables: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineTooltipData {
    pub variable_label: String,
    pub variable_unique_id: String,
    pub time: TimePs,
    pub time_label: String,
    pub value_label: String,
    pub raw_value: SignalValue,
    pub educational_message: Option<String>,
    pub screen_x: f32,
    pub screen_y: f32,
    pub vertical_alignment: TooltipVerticalAlignment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipVerticalAlignment {
    Above,
    Below,
}

#[derive(Clone, Debug)]
pub struct TimelinePointerHover {
    pub normalized_x: f64,
    pub normalized_y: f64,
}

#[derive(Clone, Debug)]
struct PointerHoverSnapshot {
    normalized_x: f64,
    normalized_y: f64,
}

#[derive(Clone, Debug)]
struct TimelineBounds {
    start: TimePs,
    end: TimePs,
}

impl Default for TimelineBounds {
    fn default() -> Self {
        Self {
            start: TimePs::ZERO,
            end: TimePs::ZERO,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RequestContext {
    latest_request_id: Option<String>,
    latest_request_started_ms: Option<f64>,
    latest_request_windows: BTreeMap<String, RequestedWindow>,
}

#[derive(Clone, Debug, Default)]
struct VariableSeriesData {
    transitions: Arc<Vec<SignalTransition>>,
    total_transitions: usize,
}

#[derive(Clone, Debug)]
struct RequestedWindow {
    range_ns: (u64, u64),
    lod_bucket: u64,
}

#[derive(Clone, Debug, Default)]
pub struct TimelineDebugMetrics {
    pub last_request_duration_ms: Option<f64>,
    pub last_render_duration_ms: Option<f64>,
    pub last_cache_hit: Option<bool>,
    pub last_cache_coverage: Option<f64>,
}

#[derive(Clone, Debug)]
struct TimelineCacheEntry {
    lod_bucket: u64,
    range_ns: (u64, u64),
    transitions: Arc<Vec<SignalTransition>>,
    total_transitions: usize,
}

impl TimelineCacheEntry {
    fn coverage_ratio(&self, range_ns: (u64, u64)) -> f64 {
        let requested = range_ns.1.saturating_sub(range_ns.0);
        if requested == 0 {
            return 0.0;
        }
        let overlap_start = self.range_ns.0.max(range_ns.0);
        let overlap_end = self.range_ns.1.min(range_ns.1);
        if overlap_end <= overlap_start {
            return 0.0;
        }
        let overlap = overlap_end - overlap_start;
        overlap as f64 / requested as f64
    }
}

#[derive(Clone, Debug, Default)]
struct TimelineWindowCache {
    entries: BTreeMap<String, VecDeque<TimelineCacheEntry>>,
}

impl TimelineWindowCache {
    fn best_entry(
        &self,
        unique_id: &str,
        lod_bucket: u64,
        range_ns: (u64, u64),
    ) -> Option<(TimelineCacheEntry, f64)> {
        let slots = self.entries.get(unique_id)?;
        let mut best: Option<(TimelineCacheEntry, f64)> = None;
        for entry in slots {
            if entry.lod_bucket != lod_bucket {
                continue;
            }
            let coverage = entry.coverage_ratio(range_ns);
            if coverage >= CACHE_HIT_THRESHOLD {
                match &mut best {
                    Some((_, best_cov)) if coverage <= *best_cov => {}
                    _ => best = Some((entry.clone(), coverage)),
                }
            }
        }
        best
    }

    fn retain_variables(&mut self, desired: &HashSet<String>) {
        self.entries.retain(|key, _| desired.contains(key));
    }

    fn invalidate_ids(&mut self, unique_ids: &[String]) {
        for unique_id in unique_ids {
            self.entries.remove(unique_id);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

fn ranges_overlap(a: (u64, u64), b: (u64, u64)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

fn range_contains(container: (u64, u64), inner: (u64, u64)) -> bool {
    container.0 <= inner.0 && container.1 >= inner.1
}

fn merge_signal_transitions(
    existing: &[SignalTransition],
    new_data: &[SignalTransition],
) -> Vec<SignalTransition> {
    let mut merged = Vec::with_capacity(existing.len() + new_data.len());
    let mut i = 0;
    let mut j = 0;

    while i < existing.len() && j < new_data.len() {
        if existing[i].time_ns <= new_data[j].time_ns {
            push_transition(&mut merged, &existing[i]);
            i += 1;
        } else {
            push_transition(&mut merged, &new_data[j]);
            j += 1;
        }
    }

    while i < existing.len() {
        push_transition(&mut merged, &existing[i]);
        i += 1;
    }

    while j < new_data.len() {
        push_transition(&mut merged, &new_data[j]);
        j += 1;
    }

    merged
}

fn push_transition(target: &mut Vec<SignalTransition>, transition: &SignalTransition) {
    if let Some(last) = target.last() {
        if last.time_ns == transition.time_ns {
            if last.value == transition.value {
                return;
            } else {
                target.pop();
            }
        }
    }
    target.push(transition.clone());
}

fn ensure_leading_transition(
    transitions: &mut Vec<SignalTransition>,
    range_start_ns: u64,
    previous: Option<&[SignalTransition]>,
) {
    if transitions.is_empty() {
        return;
    }

    if transitions.first().map(|t| t.time_ns).unwrap_or(u64::MAX) <= range_start_ns {
        return;
    }

    if let Some(prev_list) = previous {
        if let Some(previous_transition) = prev_list
            .iter()
            .rev()
            .find(|transition| transition.time_ns <= range_start_ns)
        {
            let mut synthetic = previous_transition.clone();
            synthetic.time_ns = range_start_ns;
            transitions.insert(0, synthetic);
            return;
        }
    }

    let mut synthetic = transitions.first().cloned().unwrap();
    synthetic.time_ns = range_start_ns;
    transitions.insert(0, synthetic);
}

/// Primary timeline state coordinating cursor, viewport, zoom and data requests.
#[derive(Clone)]
pub struct WaveformTimeline {
    cursor: Mutable<TimePs>,
    viewport: Mutable<Viewport>,
    zoom_center: Mutable<TimePs>,
    canvas_width: Mutable<f32>,
    canvas_height: Mutable<f32>,
    pub shift_active: Mutable<bool>,
    render_state: Mutable<TimelineRenderState>,
    series_map: Mutable<BTreeMap<String, VariableSeriesData>>,
    cursor_values: Mutable<BTreeMap<String, SignalValue>>,
    tooltip_state: Mutable<Option<TimelineTooltipData>>,
    request_state: Mutable<RequestContext>,
    window_cache: Mutable<TimelineWindowCache>,
    cursor_loading_timers: Rc<RefCell<BTreeMap<String, Timeout>>>,
    debug_metrics: Mutable<TimelineDebugMetrics>,
    debug_overlay_enabled: Mutable<bool>,
    tooltip_enabled: Mutable<bool>,
    reload_in_progress: Rc<RefCell<HashSet<String>>>,
    reload_viewport_snapshot: Rc<RefCell<Option<(Viewport, TimePs)>>>,
    reload_restore_pending: Rc<Cell<bool>>,

    selected_variables: SelectedVariables,
    maximum_range: MaximumTimelineRange,
    connection: ConnectionAdapter,
    app_config: AppConfig,
    request_counter: Arc<AtomicU64>,
    bounds_state: Mutable<Option<TimelineBounds>>,
    request_debounce: Rc<RefCell<Option<Timeout>>>,
    config_debounce: Rc<RefCell<Option<Timeout>>>,
    viewport_initialized: Mutable<bool>,
    restoring_from_config: Rc<Cell<bool>>,
    pointer_hover_snapshot: Mutable<Option<PointerHoverSnapshot>>,
    zoom_center_pending: Rc<RefCell<Option<TimePs>>>,
    zoom_center_timer: Rc<RefCell<Option<Timeout>>>,
    zoom_center_last_update_ms: Rc<RefCell<f64>>,
    zoom_center_anchor_ratio: Rc<RefCell<Option<f64>>>,
    config_restored: Mutable<bool>,
    _listener_handles: Vec<Arc<TaskHandle>>,
}

impl WaveformTimeline {
    fn expand_range_with_margin(&self, range: (u64, u64), margin: u64) -> (u64, u64) {
        let (start, end) = range;
        let mut expanded_start = start.saturating_sub(margin);
        let mut expanded_end = end.saturating_add(margin);
        if let Some(bounds) = self.bounds() {
            expanded_start = expanded_start.max(bounds.start.nanos());
            expanded_end = expanded_end.min(bounds.end.nanos());
        }
        if expanded_end <= expanded_start {
            expanded_end = expanded_start.saturating_add(1);
        }
        (expanded_start, expanded_end)
    }

    fn expand_left_range(&self, range: (u64, u64), margin: u64) -> (u64, u64) {
        let (start, end) = range;
        let mut expanded_start = start.saturating_sub(margin);
        let mut expanded_end = end;
        if let Some(bounds) = self.bounds() {
            expanded_start = expanded_start.max(bounds.start.nanos());
            expanded_end = expanded_end.min(bounds.end.nanos());
        }
        if expanded_end <= expanded_start {
            expanded_end = expanded_start.saturating_add(1);
        }
        (expanded_start, expanded_end)
    }

    fn expand_right_range(&self, range: (u64, u64), margin: u64) -> (u64, u64) {
        let (start, end) = range;
        let mut expanded_start = start;
        let mut expanded_end = end.saturating_add(margin);
        if let Some(bounds) = self.bounds() {
            expanded_start = expanded_start.max(bounds.start.nanos());
            expanded_end = expanded_end.min(bounds.end.nanos());
        }
        if expanded_end <= expanded_start {
            expanded_end = expanded_start.saturating_add(1);
        }
        (expanded_start, expanded_end)
    }

    pub async fn new(
        selected_variables: SelectedVariables,
        tracked_files: TrackedFiles,
        maximum_range: MaximumTimelineRange,
        connection: ConnectionAdapter,
        app_config: AppConfig,
    ) -> Self {
        let cursor = Mutable::new(TimePs::ZERO);
        let viewport = Mutable::new(Viewport::new(TimePs::ZERO, TimePs::from_nanos(1_000_000_000)));
        let zoom_center = Mutable::new(TimePs::ZERO);
        let canvas_width = Mutable::new(800.0);
        let canvas_height = Mutable::new(400.0);
        let shift_active = Mutable::new(false);
        let render_state = Mutable::new(TimelineRenderState::default());
        let series_map = Mutable::new(BTreeMap::new());
        let cursor_values = Mutable::new(BTreeMap::new());
        let tooltip_state = Mutable::new(None);
        let request_state = Mutable::new(RequestContext::default());
        let window_cache = Mutable::new(TimelineWindowCache::default());
        let debug_metrics = Mutable::new(TimelineDebugMetrics::default());
        let debug_overlay_enabled = Mutable::new(false);
        let tooltip_enabled = Mutable::new(true);
        let cursor_loading_timers = Rc::new(RefCell::new(BTreeMap::new()));
        let zoom_center_pending = Rc::new(RefCell::new(None));
        let zoom_center_timer = Rc::new(RefCell::new(None));
        let zoom_center_last_update_ms = Rc::new(RefCell::new(Date::now()));
        let zoom_center_anchor_ratio = Rc::new(RefCell::new(None));
        let reload_in_progress = Rc::new(RefCell::new(HashSet::new()));
        let reload_viewport_snapshot = Rc::new(RefCell::new(None));
        let reload_restore_pending = Rc::new(Cell::new(false));

        let bounds_state = Mutable::new(None);
        let request_debounce = Rc::new(RefCell::new(None));
        let config_debounce = Rc::new(RefCell::new(None));
        let viewport_initialized = Mutable::new(false);
        let pointer_hover_snapshot = Mutable::new(None);
        let restoring_from_config = Rc::new(Cell::new(false));
        let config_restored = Mutable::new(false);

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
            tooltip_state: tooltip_state.clone(),
            request_state,
            window_cache,
            cursor_loading_timers: cursor_loading_timers.clone(),
            debug_metrics,
            debug_overlay_enabled,
            tooltip_enabled,
            reload_in_progress: reload_in_progress.clone(),
            reload_viewport_snapshot: reload_viewport_snapshot.clone(),
            reload_restore_pending: reload_restore_pending.clone(),
            selected_variables,
            maximum_range,
            connection,
            app_config,
            request_counter: Arc::new(AtomicU64::new(1)),
            bounds_state,
            request_debounce,
            config_debounce,
            viewport_initialized,
            restoring_from_config: restoring_from_config.clone(),
            pointer_hover_snapshot: pointer_hover_snapshot.clone(),
            config_restored: config_restored.clone(),
            zoom_center_pending: zoom_center_pending.clone(),
            zoom_center_timer: zoom_center_timer.clone(),
            zoom_center_last_update_ms: zoom_center_last_update_ms.clone(),
            zoom_center_anchor_ratio: zoom_center_anchor_ratio.clone(),
            _listener_handles: Vec::new(),
        };

        timeline.initialize_from_config().await;

        let mut handles = Vec::new();
        handles.push(timeline.spawn_config_restore_listener());
        handles.push(timeline.spawn_selected_variables_listener());
        handles.push(timeline.spawn_bounds_listener());
        let tracked_files_for_reload = tracked_files.clone();
        handles.push(timeline.spawn_file_reload_listener(tracked_files_for_reload));
        handles.push(timeline.spawn_file_reload_completion_listener(tracked_files));
        let request_handles = timeline.spawn_request_triggers();
        handles.extend(request_handles);

        let mut timeline = timeline;
        timeline._listener_handles = handles;

        timeline.schedule_request();

        timeline
    }

    pub fn render_state_actor(&self) -> Mutable<TimelineRenderState> {
        self.render_state.clone()
    }

    pub fn cursor_actor(&self) -> Mutable<TimePs> {
        self.cursor.clone()
    }

    pub fn viewport_actor(&self) -> Mutable<Viewport> {
        self.viewport.clone()
    }

    pub fn zoom_center_actor(&self) -> Mutable<TimePs> {
        self.zoom_center.clone()
    }

    pub fn canvas_width_actor(&self) -> Mutable<f32> {
        self.canvas_width.clone()
    }

    pub fn cursor_values_actor(&self) -> Mutable<BTreeMap<String, SignalValue>> {
        let current_count = self.cursor_values.lock_ref().len();
        zoon::println!("[TIMELINE] cursor_values_actor() called - current map has {} entries", current_count);
        self.cursor_values.clone()
    }

    pub fn tooltip_actor(&self) -> Mutable<Option<TimelineTooltipData>> {
        self.tooltip_state.clone()
    }

    pub fn tooltip_visibility_handle(&self) -> Mutable<bool> {
        self.tooltip_enabled.clone()
    }

    pub fn debug_metrics_actor(&self) -> Mutable<TimelineDebugMetrics> {
        self.debug_metrics.clone()
    }

    pub fn debug_overlay_atom(&self) -> Mutable<bool> {
        self.debug_overlay_enabled.clone()
    }

    pub fn record_render_duration(&self, duration_ms: f64) {
        if duration_ms > 80.0 {
            zoon::println!(
                "⚠️ Timeline render took {:.1}ms (threshold 80ms)",
                duration_ms
            );
        }
        self.debug_metrics
            .update_mut(|metrics| metrics.last_render_duration_ms = Some(duration_ms));
    }

    fn record_request_duration(&self, duration_ms: f64) {
        if duration_ms > 80.0 {
            zoon::println!(
                "⚠️ Timeline request completed in {:.1}ms (threshold 80ms)",
                duration_ms
            );
        }
        self.debug_metrics
            .update_mut(|metrics| metrics.last_request_duration_ms = Some(duration_ms));
    }

    fn record_cache_usage(&self, cache_hits: usize, total_variables: usize, best_coverage: f64) {
        self.debug_metrics.update_mut(|metrics| {
            if total_variables == 0 {
                metrics.last_cache_hit = None;
                metrics.last_cache_coverage = None;
            } else {
                metrics.last_cache_hit = Some(cache_hits > 0);
                metrics.last_cache_coverage = if cache_hits > 0 {
                    Some(best_coverage)
                } else {
                    None
                };
            }
        });
    }

    fn lod_bucket_for(time_per_pixel: TimePerPixel) -> u64 {
        let mut bucket = 1u64;
        let target = time_per_pixel.picoseconds().max(1);
        while bucket < target {
            match bucket.checked_mul(2) {
                Some(next) => bucket = next,
                None => return bucket,
            }
        }
        bucket
    }

    fn bounds(&self) -> Option<TimelineBounds> {
        self.bounds_state.get_cloned()
    }

    fn viewport_duration_ps(&self) -> u64 {
        self.viewport
            .get_cloned()
            .duration()
            .picoseconds()
            .max(1)
    }

    fn clamp_to_bounds(&self, time: TimePs) -> TimePs {
        if let Some(bounds) = self.bounds() {
            let clamped = time
                .picoseconds()
                .clamp(bounds.start.picoseconds(), bounds.end.picoseconds());
            TimePs::from_picoseconds(clamped)
        } else {
            time
        }
    }

    fn refresh_cursor_values_from_series(&self) -> bool {
        let cursor_ns = self.cursor.get_cloned().nanos();

        let snapshot: Vec<(String, Arc<Vec<SignalTransition>>)> = {
            let map_ref = self.series_map.lock_ref();
            map_ref
                .iter()
                .map(|(unique_id, data)| (unique_id.clone(), Arc::clone(&data.transitions)))
                .collect()
        };

        if snapshot.is_empty() {
            return false;
        }

        let updates: Vec<(String, SignalValue)> = snapshot
            .into_iter()
            .map(|(unique_id, transitions_arc)| {
                let value =
                    Self::cursor_value_from_transitions(transitions_arc.as_slice(), cursor_ns);
                (unique_id, value)
            })
            .collect();

        let mut changed = false;
        {
            let mut values_map = self.cursor_values.lock_mut();
            for (unique_id, value) in updates {
                let needs_update = match values_map.get(&unique_id) {
                    Some(existing) if *existing == value => false,
                    _ => true,
                };
                if needs_update {
                    values_map.insert(unique_id, value);
                    changed = true;
                }
            }
        }

        changed
    }

    fn cursor_value_from_transitions(
        transitions: &[SignalTransition],
        cursor_ns: u64,
    ) -> SignalValue {
        if transitions.is_empty() {
            return SignalValue::Missing;
        }

        match transitions.binary_search_by(|transition| transition.time_ns.cmp(&cursor_ns)) {
            Ok(idx) => SignalValue::present(transitions[idx].value.clone()),
            Err(0) => SignalValue::Missing,
            Err(idx) => {
                let prev = &transitions[idx - 1];
                SignalValue::present(prev.value.clone())
            }
        }
    }

    pub fn set_cursor_clamped(&self, time: TimePs) {
        let clamped = self.clamp_to_bounds(time);
        self.cursor.set_neq(clamped);
        self.ensure_cursor_within_viewport();
        self.refresh_cursor_values_from_series();
        self.update_render_state();
        self.schedule_request();
        self.schedule_config_save();
    }

    pub fn move_cursor_left(&self) {
        let faster = self.shift_active.get_cloned();
        let step = self.cursor_step_ps(faster);
        let current = self.cursor.get_cloned().picoseconds();
        let new_time = current.saturating_sub(step);
        self.set_cursor_clamped(TimePs::from_picoseconds(new_time));
    }

    pub fn move_cursor_right(&self) {
        let faster = self.shift_active.get_cloned();
        let step = self.cursor_step_ps(faster);
        let current = self.cursor.get_cloned().picoseconds();
        let new_time = current.saturating_add(step);
        self.set_cursor_clamped(TimePs::from_picoseconds(new_time));
    }

    fn cursor_step_ps(&self, faster: bool) -> u64 {
        let duration = self.viewport_duration_ps();
        let base_step = ((duration as f64) * CURSOR_STEP_RATIO).round() as u64;
        let mut step = base_step.max(1);
        if faster {
            step = step.saturating_mul(CURSOR_FAST_MULTIPLIER);
        }
        step
    }

    pub fn jump_to_previous_transition(&self) {
        let times = self.collect_sorted_transition_times();
        if times.is_empty() {
            return;
        }
        let cursor_ns = self.cursor.get_cloned().nanos();
        if let Some(prev) = times.iter().rev().find(|&&t| t < cursor_ns).copied() {
            self.set_cursor_clamped(TimePs::from_nanos(prev));
        }
    }

    pub fn jump_to_next_transition(&self) {
        let times = self.collect_sorted_transition_times();
        if times.is_empty() {
            return;
        }
        let cursor_ns = self.cursor.get_cloned().nanos();
        if let Some(next) = times.iter().find(|&&t| t > cursor_ns).copied() {
            self.set_cursor_clamped(TimePs::from_nanos(next));
        }
    }

    fn collect_sorted_transition_times(&self) -> Vec<u64> {
        let map = self.series_map.lock_ref();
        let mut times = Vec::new();
        for series in map.values() {
            for transition in series.transitions.iter() {
                times.push(transition.time_ns);
            }
        }
        drop(map);
        times.sort_unstable();
        times.dedup();
        times
    }

    pub fn zoom_in(&self, faster: bool) {
        let viewport = self.viewport.get_cloned();
        let current_duration = viewport.duration().picoseconds();
        let min_duration = self.min_duration_ps();
        if current_duration <= min_duration {
            return;
        }
        let center = self.resolve_zoom_center();
        let (numerator, denominator) = if faster {
            (2.0f64, 5.0f64)
        } else {
            (7.0f64, 10.0f64)
        };
        let mut new_duration =
            ((current_duration as f64) * (numerator / denominator)).floor() as u64;
        if new_duration < min_duration {
            new_duration = min_duration;
        }
        if new_duration >= current_duration {
            new_duration = current_duration.saturating_sub(1).max(min_duration);
        }
        self.set_viewport_with_duration(center, new_duration);
    }

    pub fn zoom_out(&self, faster: bool) {
        let viewport = self.viewport.get_cloned();
        let current_duration = viewport.duration().picoseconds();
        let center = self.resolve_zoom_center();
        let (numerator, denominator) = if faster {
            (9.0f64, 5.0f64)
        } else {
            (13.0f64, 10.0f64)
        };
        let mut new_duration =
            ((current_duration as f64) * (numerator / denominator)).ceil() as u64;
        let min_duration = self.min_duration_ps();
        if new_duration <= current_duration {
            new_duration = current_duration.saturating_add(1).max(min_duration);
        }
        if let Some(bounds) = self.bounds() {
            let max_duration = bounds.end.duration_since(bounds.start).picoseconds();
            new_duration = new_duration.min(max_duration);
        }
        self.set_viewport_with_duration(center, new_duration);
    }

    pub fn pan_left(&self, faster: bool) {
        let viewport = self.viewport.get_cloned();
        let duration = viewport.duration().picoseconds();
        let mut step = (duration / 5).max(1);
        if faster {
            step = step.saturating_mul(3);
        }
        let start_ps = viewport.start.picoseconds();
        let available_left = match self.bounds() {
            Some(bounds) => start_ps.saturating_sub(bounds.start.picoseconds()),
            None => start_ps,
        };
        let shift = step.min(available_left);
        if shift == 0 {
            return;
        }
        let new_start = start_ps.saturating_sub(shift);
        let new_end = viewport.end.picoseconds().saturating_sub(shift);
        self.set_viewport_clamped(
            TimePs::from_picoseconds(new_start),
            TimePs::from_picoseconds(new_end.max(new_start + 1)),
        );
    }

    pub fn pan_right(&self, faster: bool) {
        let viewport = self.viewport.get_cloned();
        let duration = viewport.duration().picoseconds();
        let mut step = (duration / 5).max(1);
        if faster {
            step = step.saturating_mul(3);
        }
        let end_ps = viewport.end.picoseconds();
        let available_right = match self.bounds() {
            Some(bounds) => bounds.end.picoseconds().saturating_sub(end_ps),
            None => u64::MAX.saturating_sub(end_ps),
        };
        let shift = step.min(available_right);
        if shift == 0 {
            return;
        }
        let new_start = viewport.start.picoseconds().saturating_add(shift);
        let new_end = end_ps.saturating_add(shift);
        self.set_viewport_clamped(
            TimePs::from_picoseconds(new_start),
            TimePs::from_picoseconds(new_end.max(new_start + 1)),
        );
    }

    pub fn reset_zoom(&self) {
        if let Some(bounds) = self.bounds() {
            self.set_viewport_clamped(bounds.start, bounds.end);
            let start = bounds.start.picoseconds();
            let end = bounds.end.picoseconds();
            if end > start {
                let midpoint = start.saturating_add((end - start) / 2);
                self.cursor
                    .set_neq(TimePs::from_picoseconds(midpoint));
            } else {
                self.cursor.set_neq(bounds.start);
            }
        } else {
            self.viewport.set(Viewport::new(
                TimePs::ZERO,
                TimePs::from_nanos(1_000_000_000),
            ));
            self.cursor.set_neq(TimePs::from_nanos(500_000_000));
        }
        self.clear_zoom_anchor_ratio();
        self.set_zoom_center(TimePs::ZERO);
        self.update_render_state();
        self.schedule_config_save();
    }

    pub fn reset_zoom_center(&self) {
        self.clear_zoom_anchor_ratio();
        self.set_zoom_center(TimePs::ZERO);
    }

    pub fn set_shift_active(&self, active: bool) {
        self.shift_active.set_neq(active);
    }

    pub fn set_canvas_dimensions(&self, width: f32, height: f32) {
        self.canvas_width.set_neq(width);
        self.canvas_height.set_neq(height);
        self.update_render_state();
    }

    pub fn set_zoom_center_follow(&self, time: Option<TimePs>) {
        match time {
            Some(t) => {
                self.update_zoom_anchor_ratio(t);
                *self.zoom_center_pending.borrow_mut() = Some(t);
                let last_update = *self.zoom_center_last_update_ms.borrow();
                let now = Date::now();
                let elapsed = now - last_update;
                if elapsed >= ZOOM_CENTER_MIN_INTERVAL_MS {
                    self.zoom_center_pending.borrow_mut().take();
                    self.set_zoom_center(t);
                } else {
                    let delay = (ZOOM_CENTER_MIN_INTERVAL_MS - elapsed).ceil() as u32;
                    self.schedule_zoom_center_update(delay);
                }
            }
            None => {
                self.clear_zoom_anchor_ratio();
                self.zoom_center_pending.borrow_mut().take();
                if let Some(timer) = self.zoom_center_timer.borrow_mut().take() {
                    timer.cancel();
                }
            }
        }
    }

    pub fn set_pointer_hover(&self, event: Option<TimelinePointerHover>) {
        match event {
            Some(pointer) => {
                let snapshot = PointerHoverSnapshot {
                    normalized_x: pointer.normalized_x.clamp(0.0, 1.0),
                    normalized_y: pointer.normalized_y.clamp(0.0, 1.0),
                };
                self.pointer_hover_snapshot.set(Some(snapshot));
                self.refresh_tooltip();
            }
            None => {
                self.pointer_hover_snapshot.set(None);
                self.tooltip_state.set_neq(None);
            }
        }
    }

    pub fn toggle_tooltip(&self) {
        let new_value = !self.tooltip_enabled.get_cloned();
        self.tooltip_enabled.set_neq(new_value);
        if new_value {
            self.refresh_tooltip();
        } else {
            self.tooltip_state.set_neq(None);
        }
        self.schedule_config_save();
    }

    fn set_zoom_center(&self, time: TimePs) {
        let clamped = self.clamp_to_bounds(time);
        if let Some(timer) = self.zoom_center_timer.borrow_mut().take() {
            timer.cancel();
        }
        self.zoom_center_pending.borrow_mut().take();
        if self.zoom_center.get_cloned() == clamped {
            *self.zoom_center_last_update_ms.borrow_mut() = Date::now();
            return;
        }
        self.zoom_center.set_neq(clamped);
        *self.zoom_center_last_update_ms.borrow_mut() = Date::now();
        self.update_zoom_center_only(clamped);
        self.schedule_config_save();
    }

    fn update_zoom_anchor_ratio(&self, anchor_time: TimePs) {
        let viewport = self.viewport.get_cloned();
        let duration_ps = viewport.duration().picoseconds();
        let anchor_ps = anchor_time.picoseconds();
        let start_ps = viewport.start.picoseconds();

        let ratio = if duration_ps == 0 {
            0.5
        } else {
            let offset = anchor_ps.saturating_sub(start_ps) as f64;
            let duration = duration_ps as f64;
            let mut ratio = offset / duration;
            if !ratio.is_finite() {
                ratio = 0.5;
            }
            if ratio < 0.0 {
                ratio = 0.0;
            }
            if ratio > 1.0 {
                ratio = 1.0;
            }
            ratio
        };

        *self.zoom_center_anchor_ratio.borrow_mut() = Some(ratio);
    }

    fn current_zoom_anchor_ratio(&self) -> Option<f64> {
        self.zoom_center_anchor_ratio.borrow().clone()
    }

    fn clear_zoom_anchor_ratio(&self) {
        self.zoom_center_anchor_ratio.borrow_mut().take();
    }

    fn resolve_zoom_center(&self) -> TimePs {
        let pending_time = {
            let mut pending = self.zoom_center_pending.borrow_mut();
            pending.take()
        };
        if let Some(time) = pending_time {
            if let Some(timer) = self.zoom_center_timer.borrow_mut().take() {
                timer.cancel();
            }
            self.set_zoom_center(time);
        }
        self.zoom_center.get_cloned()
    }

    fn set_viewport_with_duration(&self, center: TimePs, duration_ps: u64) {
        let min_duration = self.min_duration_ps();
        let target_duration = duration_ps.max(min_duration);
        let center_ps = center.picoseconds();

        let anchor_ratio = self.current_zoom_anchor_ratio();

        let viewport = self.viewport.get_cloned();
        let current_start = viewport.start.picoseconds();
        let current_end = viewport.end.picoseconds();
        let current_duration = viewport.duration().picoseconds().max(1);

        let offset_from_start = if center_ps <= current_start {
            0
        } else if center_ps >= current_end {
            current_duration
        } else {
            center_ps - current_start
        };
        let offset_in_new = if current_duration == 0 {
            target_duration.saturating_div(2)
        } else {
            let numerator = (offset_from_start as u128) * (target_duration as u128);
            let denominator = current_duration as u128;
            let rounded = (numerator + (denominator / 2)) / denominator;
            rounded.min(target_duration as u128) as u64
        };

        let mut start_ns = center_ps.saturating_sub(offset_in_new);
        let mut end_ns = start_ns.saturating_add(target_duration);

        if let Some(bounds) = self.bounds() {
            let bounds_start = bounds.start.picoseconds();
            let bounds_end = bounds.end.picoseconds();
            if start_ns < bounds_start {
                let diff = bounds_start - start_ns;
                start_ns = bounds_start;
                end_ns = end_ns.saturating_add(diff);
            }
            if end_ns > bounds_end {
                let diff = end_ns - bounds_end;
                end_ns = bounds_end;
                start_ns = start_ns.saturating_sub(diff);
            }
        }

        if end_ns <= start_ns {
            end_ns = start_ns.saturating_add(1);
        }

        self.set_viewport_clamped(
            TimePs::from_picoseconds(start_ns),
            TimePs::from_picoseconds(end_ns),
        );

        if let Some(mut ratio) = anchor_ratio {
            if ratio.is_finite() {
                if ratio < 0.0 {
                    ratio = 0.0;
                }
                if ratio > 1.0 {
                    ratio = 1.0;
                }

                let updated_viewport = self.viewport.get_cloned();
                let updated_duration = updated_viewport.duration().picoseconds();

                if updated_duration > 0 {
                    // Preserve the visual anchor by re-centering at the stored pixel ratio after any clamping.
                    let start_ps = updated_viewport.start.picoseconds();
                    let offset = ((updated_duration as f64) * ratio).round() as u64;
                    let capped_offset = offset.min(updated_duration);
                    let new_center_ps = start_ps.saturating_add(capped_offset);
                    let new_center = TimePs::from_picoseconds(new_center_ps);
                    if new_center != self.zoom_center.get_cloned() {
                        self.set_zoom_center(new_center);
                    }
                }
            }
        }
    }

    fn min_duration_ps(&self) -> u64 {
        let width = self.canvas_width.get_cloned().max(1.0) as u32;
        Self::min_duration_ps_for_width(width).max(MIN_DURATION_PS)
    }

    fn min_duration_ps_for_width(width_px: u32) -> u64 {
        let width = width_px.max(1) as u128;
        let min_duration_fs = width * MIN_TIME_PER_PIXEL_FS as u128;
        let divisor = FS_PER_PS as u128;
        let min_duration_ps = (min_duration_fs + (divisor - 1)) / divisor;
        min_duration_ps.max(1) as u64
    }

    fn set_viewport_clamped(&self, start: TimePs, end: TimePs) {
        let mut clamped_start = start;
        let mut clamped_end = end;
        if let Some(bounds) = self.bounds() {
            if clamped_start < bounds.start {
                let shift = bounds.start.picoseconds() - clamped_start.picoseconds();
                clamped_start = bounds.start;
                clamped_end =
                    TimePs::from_picoseconds(clamped_end.picoseconds().saturating_add(shift));
            }
            if clamped_end > bounds.end {
                let shift = clamped_end.picoseconds() - bounds.end.picoseconds();
                clamped_end = bounds.end;
                clamped_start =
                    TimePs::from_picoseconds(clamped_start.picoseconds().saturating_sub(shift));
            }
        }

        if clamped_end <= clamped_start {
            clamped_end = TimePs::from_picoseconds(clamped_start.picoseconds() + 1);
        }

        self.viewport
            .set(Viewport::new(clamped_start, clamped_end));
        self.ensure_cursor_within_viewport();
        self.refresh_cursor_values_from_series();
        self.update_render_state();
        self.schedule_config_save();
    }

    fn ensure_viewport_within_bounds(&self) {
        if let Some(bounds) = self.bounds() {
            let viewport = self.viewport.get_cloned();
            let start = viewport.start.picoseconds();
            let end = viewport.end.picoseconds();
            let bounds_start = bounds.start.picoseconds();
            let bounds_end = bounds.end.picoseconds();

            if start < bounds_start || end > bounds_end {
                let mut clamped_start = start.clamp(bounds_start, bounds_end);
                let mut clamped_end = end.clamp(bounds_start, bounds_end);

                if clamped_end <= clamped_start {
                    let bounds_span = bounds_end.saturating_sub(bounds_start).max(1);
                    clamped_start = bounds_start;
                    clamped_end = bounds_start.saturating_add(bounds_span);
                }

                self.viewport.set_neq(Viewport::new(
                    TimePs::from_picoseconds(clamped_start),
                    TimePs::from_picoseconds(clamped_end),
                ));
            }
        }
        self.ensure_cursor_within_viewport();
        self.refresh_cursor_values_from_series();
        self.update_render_state();
        self.schedule_config_save();
    }

    fn ensure_cursor_within_viewport(&self) {
        let viewport = self.viewport.get_cloned();
        let cursor = self.cursor.get_cloned();
        if cursor < viewport.start {
            self.cursor.set_neq(viewport.start);
        } else if cursor > viewport.end {
            self.cursor.set_neq(viewport.end);
        }
    }

    pub fn on_variable_format_updated(&self, unique_id: String, _format: VarFormat) {
        self.cancel_cursor_loading_indicator(&unique_id);
        self.update_render_state();
        self.schedule_request();
    }

    fn on_selected_variables_updated(&self, variables: Vec<shared::SelectedVariable>) {
        let desired: HashSet<_> = variables.iter().map(|var| var.unique_id.clone()).collect();

        {
            let mut map = self.series_map.lock_mut();
            map.retain(|key, _| desired.contains(key));
        }

        {
            let mut values_map = self.cursor_values.lock_mut();
            values_map.retain(|key, _| desired.contains(key));
            for variable in &variables {
                values_map
                    .entry(variable.unique_id.clone())
                    .or_insert(SignalValue::Loading);
            }
        }

        self.prune_cursor_loading_timers(&desired);

        {
            let mut cache = self.window_cache.lock_mut();
            cache.retain_variables(&desired);
        }

        self.update_render_state();
        self.schedule_request();
        self.schedule_config_save();
    }

    fn schedule_request(&self) {
        {
            let mut slot = self.request_debounce.borrow_mut();
            if let Some(timer) = slot.take() {
                timer.cancel();
            }
        }

        let last_started = self
            .request_state
            .get_cloned()
            .latest_request_started_ms;
        let now = Date::now();
        let min_interval_ms = REQUEST_DEBOUNCE_MS as f64;

        if let Some(last_started_ms) = last_started {
            let elapsed = now - last_started_ms;
            if elapsed >= min_interval_ms {
                self.send_request();
                return;
            }

            let remaining = (min_interval_ms - elapsed).ceil().max(1.0) as u32;
            let debounce_slot = self.request_debounce.clone();
            let timeline = self.clone();
            let timeout = Timeout::new(remaining, move || {
                *debounce_slot.borrow_mut() = None;
                timeline.send_request();
            });
            *self.request_debounce.borrow_mut() = Some(timeout);
        } else {
            self.send_request();
        }
    }

    fn schedule_config_save(&self) {
        if self.restoring_from_config.get() {
            return;
        }
        if let Some(timer) = self.config_debounce.borrow_mut().take() {
            timer.cancel();
        }

        let debounce_slot = self.config_debounce.clone();
        let timeline = self.clone();
        let timeout = Timeout::new(CONFIG_SAVE_DEBOUNCE_MS, move || {
            *debounce_slot.borrow_mut() = None;
            timeline.sync_state_to_config();
        });
        *self.config_debounce.borrow_mut() = Some(timeout);
    }

    fn schedule_zoom_center_update(&self, delay_ms: u32) {
        {
            let mut timer_ref = self.zoom_center_timer.borrow_mut();
            if let Some(existing) = timer_ref.take() {
                existing.cancel();
            }
        }

        let timeline = self.clone();
        let pending = self.zoom_center_pending.clone();
        let timer_slot = self.zoom_center_timer.clone();

        let timeout = Timeout::new(delay_ms.max(1), move || {
            *timer_slot.borrow_mut() = None;
            let maybe_time = {
                let mut pending_ref = pending.borrow_mut();
                pending_ref.take()
            };
            if let Some(time) = maybe_time {
                timeline.set_zoom_center(time);
            }
        });

        *self.zoom_center_timer.borrow_mut() = Some(timeout);
    }

    fn schedule_cursor_loading_indicator(&self, unique_id: String) {
        {
            let mut timers_ref = self.cursor_loading_timers.borrow_mut();
            if let Some(existing) = timers_ref.remove(&unique_id) {
                existing.cancel();
            }
        }

        let timeline = self.clone();
        let timers = self.cursor_loading_timers.clone();
        let unique_id_for_closure = unique_id.clone();
        let timeout = Timeout::new(CURSOR_LOADING_DELAY_MS, move || {
            let mut should_update = false;
            {
                let mut values_map = timeline.cursor_values.lock_mut();
                let needs_update = match values_map.get(&unique_id_for_closure) {
                    Some(SignalValue::Present(_)) => false,
                    Some(SignalValue::Loading) => false,
                    _ => true,
                };
                if needs_update {
                    values_map.insert(unique_id_for_closure.clone(), SignalValue::Loading);
                    should_update = true;
                }
            }

            if should_update {
                timeline.update_render_state();
            }

            timers.borrow_mut().remove(&unique_id_for_closure);
        });

        self.cursor_loading_timers
            .borrow_mut()
            .insert(unique_id, timeout);
    }

    fn cancel_cursor_loading_indicator(&self, unique_id: &str) {
        if let Some(timer) = self.cursor_loading_timers.borrow_mut().remove(unique_id) {
            timer.cancel();
        }
    }

    fn cancel_all_cursor_loading_indicators(&self) {
        let keys: Vec<String> = {
            let timers = self.cursor_loading_timers.borrow();
            timers.keys().cloned().collect()
        };

        for key in keys {
            self.cancel_cursor_loading_indicator(&key);
        }
    }

    fn prune_cursor_loading_timers(&self, desired: &HashSet<String>) {
        let keys_to_remove: Vec<String> = {
            let timers = self.cursor_loading_timers.borrow();
            timers
                .keys()
                .filter(|key| !desired.contains(*key))
                .cloned()
                .collect()
        };

        for key in keys_to_remove {
            self.cancel_cursor_loading_indicator(&key);
        }
    }

    fn send_request(&self) {
        let variables = self
            .selected_variables
            .variables_vec_actor
            .get_cloned();

        zoon::println!("[TIMELINE] send_request: {} variables", variables.len());

        if variables.is_empty() {
            self.series_map.lock_mut().clear();
            self.cursor_values.lock_mut().clear();
            self.window_cache.lock_mut().clear();
            self.cancel_all_cursor_loading_indicators();
            self.update_render_state();
            return;
        }

        let viewport = self.viewport.get_cloned();
        let start_ps = viewport.start.picoseconds();
        let end_ps = viewport.end.picoseconds();
        zoon::println!("[TIMELINE] viewport: start_ps={} end_ps={}", start_ps, end_ps);
        if end_ps <= start_ps {
            zoon::println!("[TIMELINE] SKIPPING: viewport invalid (end <= start)");
            return;
        }

        let start_ns = start_ps / PS_PER_NS;
        let mut end_ns = if end_ps == 0 {
            0
        } else {
            (end_ps + PS_PER_NS - 1) / PS_PER_NS
        };
        if end_ns <= start_ns {
            end_ns = start_ns.saturating_add(1);
        }

        let width_px = self.canvas_width.get_cloned().max(1.0) as u32;
        if width_px == 0 {
            return;
        }

        let max_transitions = (width_px as usize).saturating_mul(4).max(1);
        let request_range = (start_ns, end_ns);
        let margin = (end_ns - start_ns).saturating_div(4).max(1);
        let expanded_range = self.expand_range_with_margin(request_range, margin);
        let duration_ps = end_ps.saturating_sub(start_ps).max(1);
        let time_per_pixel = TimePerPixel::from_duration_and_width(duration_ps, width_px);
        let lod_bucket = Self::lod_bucket_for(time_per_pixel);

        struct VariablePlan {
            unique_id: String,
            formatter: VarFormat,
            request_parts: Option<(String, String, String)>,
            cached_entry: Option<TimelineCacheEntry>,
            request_range_override: Option<(u64, u64)>,
            needs_request: bool,
        }

        let mut plans = Vec::with_capacity(variables.len());
        let mut cache_hits = 0usize;
        let mut best_coverage = 0.0_f64;

        {
            let cache_guard = self.window_cache.lock_ref();
            for variable in &variables {
                let unique_id = variable.unique_id.clone();
                let formatter = variable.formatter.unwrap_or(VarFormat::Hexadecimal);
                let request_parts = variable.parse_unique_id();

                let mut plan = VariablePlan {
                    unique_id,
                    formatter,
                    request_parts,
                    cached_entry: None,
                    request_range_override: None,
                    needs_request: true,
                };

                if let Some((entry, coverage)) =
                    cache_guard.best_entry(&plan.unique_id, lod_bucket, request_range)
                {
                    cache_hits += 1;
                    best_coverage = best_coverage.max(coverage);
                    let missing_left = request_range.0 < entry.range_ns.0;
                    let missing_right = request_range.1 > entry.range_ns.1;

                    plan.needs_request = missing_left || missing_right;
                    if missing_left && !missing_right {
                        let missing_range = (request_range.0, entry.range_ns.0);
                        if missing_range.1 > missing_range.0 {
                            let fetch_range = self.expand_left_range(missing_range, margin);
                            plan.request_range_override = Some(fetch_range);
                        } else {
                            plan.needs_request = false;
                        }
                    } else if missing_right && !missing_left {
                        let missing_range = (entry.range_ns.1, request_range.1);
                        if missing_range.1 > missing_range.0 {
                            let fetch_range = self.expand_right_range(missing_range, margin);
                            plan.request_range_override = Some(fetch_range);
                        } else {
                            plan.needs_request = false;
                        }
                    }
                    plan.cached_entry = Some(entry);
                }

                plans.push(plan);
            }
        }

        self.record_cache_usage(cache_hits, plans.len(), best_coverage);

        {
            let mut map = self.series_map.lock_mut();
            for plan in &plans {
                if let Some(entry) = &plan.cached_entry {
                    map.insert(
                        plan.unique_id.clone(),
                        VariableSeriesData {
                            transitions: Arc::clone(&entry.transitions),
                            total_transitions: entry.total_transitions,
                        },
                    );
                }
            }
        }

        self.update_render_state();

        let mut requests = Vec::new();
        let mut request_windows = BTreeMap::new();
        let mut loading_candidates = Vec::new();
        for plan in &plans {
            if plan.needs_request {
                let range_to_request = plan.request_range_override.unwrap_or(expanded_range);
                if let Some((file_path, scope_path, variable_name)) = &plan.request_parts {
                    if plan.cached_entry.is_none() {
                        loading_candidates.push(plan.unique_id.clone());
                    }
                    request_windows.insert(
                        plan.unique_id.clone(),
                        RequestedWindow {
                            range_ns: range_to_request,
                            lod_bucket,
                        },
                    );
                    requests.push(UnifiedSignalRequest {
                        file_path: file_path.clone(),
                        scope_path: scope_path.clone(),
                        variable_name: variable_name.clone(),
                        time_range_ns: Some(range_to_request),
                        max_transitions: Some(max_transitions),
                        format: plan.formatter,
                    });
                }
            }
        }

        if requests.is_empty() {
            zoon::println!("[TIMELINE] SKIPPING: no requests to send");
            return;
        }

        zoon::println!("[TIMELINE] Sending {} requests:", requests.len());
        for req in &requests {
            zoon::println!("[TIMELINE]   - {}|{}|{}", req.file_path, req.scope_path, req.variable_name);
        }

        for unique_id in loading_candidates {
            self.schedule_cursor_loading_indicator(unique_id);
        }

        let cursor_ns = self.cursor.get_cloned().nanos();

        let request_id = format!(
            "timeline-{}",
            self.request_counter.fetch_add(1, Ordering::SeqCst)
        );
        let mut context = self.request_state.get_cloned();
        context.latest_request_id = Some(request_id.clone());
        context.latest_request_started_ms = Some(Date::now());
        context.latest_request_windows = request_windows;
        self.request_state.set(context);

        zoon::println!("[TIMELINE] Sending request_id={}", request_id);

        let connection = self.connection.clone();
        Task::start_droppable(async move {
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
        zoon::println!("[TIMELINE] apply_unified_signal_response: request_id={} signal_data={} cursor_values={}",
            request_id, signal_data.len(), cursor_values.len());
        for (k, v) in &cursor_values {
            zoon::println!("[TIMELINE]   cursor: {} = {:?}", k, v);
        }

        let mut current_request = self.request_state.get_cloned();
        if current_request
            .latest_request_id
            .as_deref()
            .map(|id| id != request_id)
            .unwrap_or(true)
        {
            zoon::println!("[TIMELINE] SKIPPING response: request_id mismatch (expected {:?}, got {})",
                current_request.latest_request_id, request_id);
            return;
        }

        let mut request_windows = std::mem::take(&mut current_request.latest_request_windows);

        if let Some(started_ms) = current_request.latest_request_started_ms.take() {
            let duration_ms = Date::now() - started_ms;
            self.record_request_duration(duration_ms);
        }
        self.request_state.set(current_request.clone());

        let mut cache = self.window_cache.lock_mut();
        let mut series_map = self.series_map.lock_mut();

        for UnifiedSignalData {
            unique_id,
            transitions,
            total_transitions: _,
            actual_time_range_ns,
            ..
        } in signal_data
        {
            let requested_window = request_windows.remove(&unique_id);
            let existing_series = series_map
                .get(&unique_id)
                .map(|series| series.transitions.clone());

            let mut transitions_vec = transitions;

            let mut merged_range = actual_time_range_ns
                .or_else(|| requested_window.as_ref().map(|window| window.range_ns))
                .or_else(|| {
                    let start = transitions_vec.first()?.time_ns;
                    let end = transitions_vec.last()?.time_ns;
                    Some((start, end))
                })
                .unwrap_or((0, 0));

            let mut cache_slot_action: Option<u64> = None;

            if let Some(window) = &requested_window {
                let lod_bucket = window.lod_bucket;
                let slots = cache
                    .entries
                    .entry(unique_id.clone())
                    .or_insert_with(VecDeque::new);

                if let Some(position) = slots.iter().position(|entry| {
                    entry.lod_bucket == lod_bucket && ranges_overlap(entry.range_ns, merged_range)
                }) {
                    let existing_entry = slots.remove(position).unwrap();
                    transitions_vec = merge_signal_transitions(
                        existing_entry.transitions.as_ref(),
                        transitions_vec.as_slice(),
                    );
                    merged_range = (
                        existing_entry.range_ns.0.min(merged_range.0),
                        existing_entry.range_ns.1.max(merged_range.1),
                    );
                    slots.retain(|entry| {
                        !(entry.lod_bucket == lod_bucket
                            && range_contains(merged_range, entry.range_ns))
                    });
                }

                cache_slot_action = Some(lod_bucket);
            }

            let leading_start_ns = requested_window
                .as_ref()
                .map(|window| window.range_ns.0)
                .or_else(|| actual_time_range_ns.map(|range| range.0))
                .unwrap_or(merged_range.0);

            let previous_slice = existing_series.as_ref().map(|arc| arc.as_slice());
            ensure_leading_transition(&mut transitions_vec, leading_start_ns, previous_slice);
            merged_range.0 = merged_range.0.min(leading_start_ns);

            if let Some(existing_arc) = &existing_series {
                let existing_slice = existing_arc.as_ref();
                if !existing_slice.is_empty() {
                    transitions_vec =
                        merge_signal_transitions(existing_slice, transitions_vec.as_slice());
                    if let Some(first) = existing_slice.first() {
                        merged_range.0 = merged_range.0.min(first.time_ns);
                    }
                    if let Some(last) = existing_slice.last() {
                        merged_range.1 = merged_range.1.max(last.time_ns);
                    }
                }
            }

            let transitions_arc = Arc::new(transitions_vec);
            let transition_count = transitions_arc.len();

            if let Some(lod_bucket) = cache_slot_action {
                let slots = cache
                    .entries
                    .entry(unique_id.clone())
                    .or_insert_with(VecDeque::new);
                slots.push_front(TimelineCacheEntry {
                    lod_bucket,
                    range_ns: merged_range,
                    transitions: Arc::clone(&transitions_arc),
                    total_transitions: transition_count,
                });
                while slots.len() > CACHE_MAX_SEGMENTS_PER_VARIABLE {
                    slots.pop_back();
                }
            }

            series_map.insert(
                unique_id,
                VariableSeriesData {
                    transitions: transitions_arc,
                    total_transitions: transition_count,
                },
            );
        }

        drop(series_map);
        drop(cache);

        {
            let mut values_map = self.cursor_values.lock_mut();
            for (unique_id, value) in cursor_values {
                values_map.insert(unique_id.clone(), value);
                self.cancel_cursor_loading_indicator(&unique_id);
            }
            zoon::println!("[TIMELINE] After cursor_values insertion, map has {} entries", values_map.len());
        }

        self.update_render_state();
        self.refresh_cursor_values_from_series();

        if self.reload_in_progress.borrow().is_empty() && self.reload_restore_pending.get() {
            if let Some((viewport, cursor)) = self.reload_viewport_snapshot.borrow_mut().take() {
                self.viewport.set(viewport);
                self.cursor.set(cursor);
                self.update_render_state();
            }
            self.reload_restore_pending.set(false);
        }

        current_request.latest_request_windows = request_windows;
        self.request_state.set(current_request);
        zoon::println!("[TIMELINE] apply_unified_signal_response: COMPLETED for request_id");
    }

    fn sync_state_to_config(&self) {
        let viewport = self.viewport.get_cloned();
        let cursor = self.cursor.get_cloned();
        let zoom_center = self.zoom_center.get_cloned();
        let tooltip_enabled = self.tooltip_enabled.get_cloned();

        if viewport.end <= viewport.start {
            return;
        }

        let timeline_state = TimelineState {
            cursor_position: Some(cursor),
            visible_range: Some(TimeRange {
                start: viewport.start,
                end: viewport.end,
            }),
            zoom_center: Some(zoom_center),
            tooltip_enabled,
        };

        self.app_config.update_timeline_state(timeline_state);
    }

    pub fn handle_unified_signal_error(&self, request_id: &str, error: &str) {
        let mut current_request = self.request_state.get_cloned();
        if current_request
            .latest_request_id
            .as_deref()
            .map(|id| id == request_id)
            .unwrap_or(false)
        {
            zoon::println!("Unified signal request failed: {}", error);
            if let Some(started_ms) = current_request.latest_request_started_ms.take() {
                let duration_ms = Date::now() - started_ms;
                self.record_request_duration(duration_ms);
            }
            let pending: Vec<String> = current_request
                .latest_request_windows
                .keys()
                .cloned()
                .collect();

            {
                let mut values_map = self.cursor_values.lock_mut();
                for unique_id in &pending {
                    let is_loading = values_map
                        .get(unique_id)
                        .map(|value| matches!(value, SignalValue::Loading))
                        .unwrap_or(true);
                    if is_loading {
                        values_map.insert(unique_id.clone(), SignalValue::Missing);
                    }
                }
            }

            for unique_id in &pending {
                self.cancel_cursor_loading_indicator(unique_id);
            }
            current_request.latest_request_windows.clear();
            self.request_state.set(current_request);
            if !pending.is_empty() {
                self.update_render_state();
            }
        }
    }

    fn update_render_state(&self) {
        let viewport = self.viewport.get_cloned();
        let cursor = self.cursor.get_cloned();
        let zoom_center = self.zoom_center.get_cloned();
        let width = self.canvas_width.get_cloned().max(1.0) as u32;
        let height = self.canvas_height.get_cloned().max(1.0) as u32;
        let duration_ps = viewport.duration().picoseconds();
        let time_per_pixel = TimePerPixel::from_duration_and_width(duration_ps, width);

        let variables_snapshot = self
            .selected_variables
            .variables_vec_actor
            .get_cloned();

        let series_guard = self.series_map.lock_ref();
        let values_guard = self.cursor_values.lock_ref();
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
                        transitions: Arc::clone(&series.transitions),
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

        self.render_state.set(TimelineRenderState {
            viewport_start: viewport.start,
            viewport_end: viewport.end,
            cursor,
            zoom_center,
            canvas_width_px: width,
            canvas_height_px: height,
            time_per_pixel,
            variables,
        });

        self.refresh_tooltip();
    }

    fn update_zoom_center_only(&self, zoom_center: TimePs) {
        self.render_state.update_mut(|state| {
            if state.zoom_center != zoom_center {
                state.zoom_center = zoom_center;
            }
        });
    }

    pub fn apply_cursor_values<I>(&self, values: I)
    where
        I: IntoIterator<Item = (String, SignalValue)>,
    {
        {
            let mut map = self.cursor_values.lock_mut();
            for (key, value) in values {
                map.insert(key.clone(), value);
                self.cancel_cursor_loading_indicator(&key);
            }
        }
        self.update_render_state();
    }

    fn refresh_tooltip(&self) {
        if !self.tooltip_enabled.get_cloned() {
            self.tooltip_state.set_neq(None);
            return;
        }

        let snapshot = match self.pointer_hover_snapshot.get_cloned() {
            Some(snapshot) => snapshot,
            None => {
                self.tooltip_state.set_neq(None);
                return;
            }
        };

        let render_state = self.render_state.get_cloned();
        if render_state.variables.is_empty() {
            self.tooltip_state.set_neq(None);
            return;
        }

        let total_rows = (render_state.variables.len() + 1) as f64;
        let row_index = (snapshot.normalized_y * total_rows).floor() as usize;
        if row_index >= render_state.variables.len() {
            self.tooltip_state.set_neq(None);
            return;
        }

        let series = &render_state.variables[row_index];
        let start_ps = render_state.viewport_start.picoseconds();
        let end_ps = render_state.viewport_end.picoseconds();
        let duration_ps = end_ps.saturating_sub(start_ps);
        let normalized_x = snapshot.normalized_x.clamp(0.0, 1.0);
        let offset_ps = if duration_ps == 0 {
            0
        } else {
            ((duration_ps as f64) * normalized_x).round() as u64
        };
        let target_time = TimePs::from_picoseconds(start_ps.saturating_add(offset_ps));
        let target_ns = target_time.picoseconds() / PS_PER_NS;

        let value = Self::cursor_value_from_transitions(series.transitions.as_ref(), target_ns);
        let formatted_value = value.get_formatted(&series.formatter);
        let variable_label = Self::tooltip_label_from_unique_id(&series.unique_id);
        let time_label = format!("{}", target_time);

        let canvas_width = render_state.canvas_width_px.max(1) as f32;
        let canvas_height = render_state.canvas_height_px.max(1) as f32;

        let mut screen_x = (normalized_x as f32) * canvas_width;
        let mut screen_y = (snapshot.normalized_y as f32) * canvas_height;
        const TOOLTIP_MARGIN: f32 = 12.0;
        if canvas_width > TOOLTIP_MARGIN {
            screen_x = screen_x.clamp(TOOLTIP_MARGIN, canvas_width - TOOLTIP_MARGIN);
        }
        if canvas_height > TOOLTIP_MARGIN {
            screen_y = screen_y.clamp(TOOLTIP_MARGIN, canvas_height - TOOLTIP_MARGIN);
        }

        let vertical_alignment = if snapshot.normalized_y < 0.2 {
            TooltipVerticalAlignment::Below
        } else {
            TooltipVerticalAlignment::Above
        };

        let educational_message =
            Self::tooltip_special_state_message(&value).map(|message| message.to_string());

        let tooltip = TimelineTooltipData {
            variable_label,
            variable_unique_id: series.unique_id.clone(),
            time: target_time,
            time_label,
            value_label: formatted_value,
            raw_value: value.clone(),
            educational_message,
            screen_x,
            screen_y,
            vertical_alignment,
        };

        self.tooltip_state.set_neq(Some(tooltip));
    }

    fn tooltip_label_from_unique_id(unique_id: &str) -> String {
        let mut parts = unique_id.splitn(3, '|');
        let file_part = parts.next();
        let scope_part = parts.next();
        let name_part = parts.next();

        match (file_part, scope_part, name_part) {
            (_, Some(scope), Some(name)) if !scope.is_empty() => {
                format!("{} :: {}", scope, name)
            }
            (_, _, Some(name)) => name.to_string(),
            _ => unique_id.to_string(),
        }
    }

    fn tooltip_special_state_message(value: &SignalValue) -> Option<&'static str> {
        match value {
            SignalValue::Present(raw) => match raw.trim().to_ascii_uppercase().as_str() {
                "Z" => Some(
                    "High-Impedance (Z)\nSignal is disconnected or floating.\nCommon in tri-state buses and disabled outputs.",
                ),
                "X" => Some(
                    "Unknown (X)\nSignal value cannot be determined.\nOften caused by timing violations or uninitialized logic.",
                ),
                "U" => Some(
                    "Uninitialized (U)\nSignal has not been assigned a value.\nTypically seen during power-up or before reset.",
                ),
                _ => None,
            },
            _ => None,
        }
    }

    async fn initialize_from_config(&self) {
        let initial_state = self
            .app_config
            .timeline_state
            .signal_cloned()
            .to_stream()
            .next()
            .await
            .unwrap_or_default();

        self.apply_config_state(&initial_state, true);
    }

    fn spawn_config_restore_listener(&self) -> Arc<TaskHandle> {
        let timeline = self.clone();
        let receiver = timeline
            .app_config
            .timeline_restore_receiver
            .borrow_mut()
            .take();
        Arc::new(Task::start_droppable(async move {
            if let Some(receiver) = receiver {
                let mut stream = receiver.fuse();
                while let Some(state) = stream.next().await {
                    timeline.apply_config_state(&state, false);
                }
            }
        }))
    }

    fn apply_config_state(&self, state: &TimelineState, is_initial: bool) {
        self.restoring_from_config.set(true);

        let previous_tooltip_enabled = self.tooltip_enabled.get_cloned();

        let sanitized_range = state
            .visible_range
            .as_ref()
            .map(|range| self.sanitize_config_range(range));

        let mut viewport_changed = false;
        if let Some((start, end)) = sanitized_range {
            let current_viewport = self.viewport.get_cloned();
            if current_viewport.start != start || current_viewport.end != end {
                self.viewport.set(Viewport::new(start, end));
                viewport_changed = true;
            }
            self.viewport_initialized.set(true);
        } else if is_initial {
            self.viewport_initialized.set(false);
        }

        let mut cursor_changed = false;
        if let Some((start, end)) = sanitized_range {
            let target = state.cursor_position.unwrap_or(start);
            let clamped = Self::clamp_time_to_range(target, start, end);
            if self.cursor.get_cloned() != clamped {
                self.cursor.set_neq(clamped);
                cursor_changed = true;
            }
        } else if let Some(cursor) = state.cursor_position {
            let clamped = self.clamp_to_bounds(cursor);
            if self.cursor.get_cloned() != clamped {
                self.cursor.set_neq(clamped);
                cursor_changed = true;
            }
        } else if is_initial {
            let center = self.viewport.get_cloned().center();
            if self.cursor.get_cloned() != center {
                self.cursor.set_neq(center);
                cursor_changed = true;
            }
        }

        let mut zoom_target = state
            .zoom_center
            .or_else(|| state.visible_range.map(|range| range.start))
            .unwrap_or_else(|| self.viewport.get_cloned().start);

        if let Some((start, end)) = sanitized_range {
            zoom_target = Self::clamp_time_to_range(zoom_target, start, end);
        } else {
            zoom_target = self.clamp_to_bounds(zoom_target);
        }

        let mut zoom_changed = false;
        if self.zoom_center.get_cloned() != zoom_target {
            self.zoom_center.set_neq(zoom_target);
            zoom_changed = true;
        }
        *self.zoom_center_last_update_ms.borrow_mut() = Date::now();
        self.update_zoom_center_only(zoom_target);

        if self.tooltip_enabled.get_cloned() != state.tooltip_enabled {
            self.tooltip_enabled.set_neq(state.tooltip_enabled);
        }

        self.restoring_from_config.set(false);
        if !is_initial {
            self.config_restored.set_neq(true);
        }

        if viewport_changed {
            self.ensure_cursor_within_viewport();
        }
        if viewport_changed || cursor_changed {
            self.refresh_cursor_values_from_series();
        }
        if viewport_changed || cursor_changed || zoom_changed {
            self.update_render_state();
        }
        if viewport_changed {
            self.schedule_request();
        } else if cursor_changed {
            self.schedule_request();
        }

        if previous_tooltip_enabled != state.tooltip_enabled {
            if state.tooltip_enabled {
                self.refresh_tooltip();
            } else {
                self.tooltip_state.set_neq(None);
            }
        }
    }

    fn sanitize_config_range(&self, range: &TimeRange) -> (TimePs, TimePs) {
        let mut start_ps = range.start.picoseconds();
        let mut end_ps = range.end.picoseconds();

        if let Some(bounds) = self.bounds() {
            let bounds_start = bounds.start.picoseconds();
            let bounds_end = bounds.end.picoseconds();
            if bounds_end > bounds_start {
                start_ps = start_ps.clamp(bounds_start, bounds_end);
                end_ps = end_ps.clamp(bounds_start, bounds_end);
            } else {
                start_ps = bounds_start;
                end_ps = bounds_start.saturating_add(1);
            }
        }

        if end_ps <= start_ps {
            if let Some(bounds) = self.bounds() {
                let bounds_start = bounds.start.picoseconds();
                let bounds_end = bounds.end.picoseconds();
                if bounds_end > bounds_start {
                    if start_ps >= bounds_end {
                        start_ps = bounds_end.saturating_sub(1);
                        end_ps = bounds_end;
                    } else {
                        end_ps = (start_ps + 1).min(bounds_end);
                        if end_ps <= start_ps {
                            end_ps = bounds_end.min(start_ps.saturating_add(1));
                        }
                    }
                } else {
                    end_ps = start_ps.saturating_add(1);
                }
            } else {
                end_ps = start_ps.saturating_add(1);
            }
        }

        (
            TimePs::from_picoseconds(start_ps),
            TimePs::from_picoseconds(end_ps),
        )
    }

    fn clamp_time_to_range(time: TimePs, start: TimePs, end: TimePs) -> TimePs {
        if time < start {
            start
        } else if time > end {
            end
        } else {
            time
        }
    }

    fn handle_file_reload_requested(&self, file_path: &str) {
        let mut set = self.reload_in_progress.borrow_mut();
        if set.is_empty() {
            let snapshot = (
                self.viewport.get_cloned(),
                self.cursor.get_cloned(),
            );
            *self.reload_viewport_snapshot.borrow_mut() = Some(snapshot);
        }
        set.insert(file_path.to_string());
        drop(set);
        self.reload_restore_pending.set(false);

        {
            let mut series_map = self.series_map.lock_mut();
            let prefix = format!("{}|", file_path);
            series_map.retain(|key, _| !key.starts_with(&prefix));
        }

        let variables_snapshot = self
            .selected_variables
            .variables_vec_actor
            .get_cloned();

        let affected_ids: HashSet<String> = variables_snapshot
            .iter()
            .filter_map(|var| {
                var.file_path().and_then(|path| {
                    if path == file_path {
                        Some(var.unique_id.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        if affected_ids.is_empty() {
            return;
        }

        let affected_list: Vec<String> = affected_ids.iter().cloned().collect();

        {
            let mut cursor_values = self.cursor_values.lock_mut();
            for unique_id in &affected_list {
                cursor_values.insert(unique_id.clone(), SignalValue::Loading);
            }
        }

        {
            let mut window_cache = self.window_cache.lock_mut();
            window_cache.invalidate_ids(&affected_list);
        }

        for unique_id in &affected_list {
            self.cancel_cursor_loading_indicator(unique_id);
            self.schedule_cursor_loading_indicator(unique_id.clone());
        }

        self.update_render_state();
        self.schedule_request();
    }

    fn handle_file_reload_completed(&self, file_path: &str) {
        let mut set = self.reload_in_progress.borrow_mut();
        set.remove(file_path);
        let pending = !set.is_empty();
        drop(set);

        if !pending {
            if self.reload_viewport_snapshot.borrow().is_some() {
                self.reload_restore_pending.set(true);
            }
            self.schedule_request();
        }
    }

    fn has_active_reload(&self) -> bool {
        !self.reload_in_progress.borrow().is_empty()
    }

    fn spawn_selected_variables_listener(&self) -> Arc<TaskHandle> {
        let timeline = self.clone();
        Arc::new(Task::start_droppable(async move {
            let mut stream = timeline
                .selected_variables
                .variables_vec_actor
                .signal_cloned()
                .to_stream()
                .fuse();

            while let Some(variables) = stream.next().await {
                timeline.on_selected_variables_updated(variables);
            }
        }))
    }

    fn spawn_file_reload_listener(&self, tracked_files: TrackedFiles) -> Arc<TaskHandle> {
        let timeline = self.clone();
        Arc::new(Task::start_droppable(async move {
            let mut reload_stream = tracked_files.file_reload_started_signal().to_stream().fuse();

            while let Some(maybe_path) = reload_stream.next().await {
                if let Some(path) = maybe_path {
                    timeline.handle_file_reload_requested(&path);
                }
            }
        }))
    }

    fn spawn_file_reload_completion_listener(&self, tracked_files: TrackedFiles) -> Arc<TaskHandle> {
        let timeline = self.clone();
        Arc::new(Task::start_droppable(async move {
            let mut stream = tracked_files.file_reload_completed_signal().to_stream().fuse();

            while let Some(maybe_file_id) = stream.next().await {
                if let Some(file_id) = maybe_file_id {
                    timeline.handle_file_reload_completed(&file_id);
                }
            }
        }))
    }

    fn spawn_bounds_listener(&self) -> Arc<TaskHandle> {
        let timeline = self.clone();
        Arc::new(Task::start_droppable(async move {
            let mut stream = timeline.maximum_range.range.signal().to_stream().fuse();

            while let Some(maybe_range) = stream.next().await {
                if let Some((start, end)) = maybe_range {
                    if timeline.has_active_reload() || timeline.reload_restore_pending.get() {
                        continue;
                    }
                    let bounds = TimelineBounds { start, end };
                    timeline.bounds_state.set(Some(bounds.clone()));

                    if !timeline.viewport_initialized.get() {
                        timeline
                            .viewport
                            .set(Viewport::new(bounds.start, bounds.end));

                        let start_ps = bounds.start.picoseconds();
                        let end_ps = bounds.end.picoseconds();
                        let midpoint_ps = if end_ps > start_ps {
                            start_ps.saturating_add((end_ps - start_ps) / 2)
                        } else {
                            start_ps
                        };

                        timeline
                            .cursor
                            .set_neq(TimePs::from_picoseconds(midpoint_ps));
                        timeline.zoom_center.set_neq(bounds.start);
                        timeline.viewport_initialized.set(true);
                        timeline.update_render_state();
                        timeline.schedule_request();
                        timeline.schedule_config_save();
                    } else {
                        timeline.ensure_viewport_within_bounds();
                    }
                } else {
                    if timeline.has_active_reload() || timeline.reload_restore_pending.get() {
                        continue;
                    }
                    timeline.bounds_state.set(None);

                    let has_variables = !timeline
                        .selected_variables
                        .variables_vec_actor
                        .get_cloned()
                        .is_empty();

                    if timeline.config_restored.get_cloned() && has_variables {
                        continue;
                    }

                    timeline.viewport.set(Viewport::new(
                        TimePs::ZERO,
                        TimePs::from_nanos(1_000_000_000),
                    ));
                    timeline.update_render_state();
                    timeline.schedule_request();
                    timeline.schedule_config_save();
                    timeline.viewport_initialized.set(false);
                    if !has_variables {
                        timeline.config_restored.set_neq(false);
                    }
                }
            }
        }))
    }

    fn spawn_request_triggers(&self) -> Vec<Arc<TaskHandle>> {
        let timeline = self.clone();
        let viewport_handle = Arc::new(Task::start_droppable(async move {
            let mut viewport_stream = timeline.viewport.signal_cloned().to_stream().fuse();
            while viewport_stream.next().await.is_some() {
                timeline.ensure_viewport_within_bounds();
                timeline.schedule_request();
                timeline.schedule_config_save();
            }
        }));

        let timeline = self.clone();
        let width_handle = Arc::new(Task::start_droppable(async move {
            let mut width_stream = timeline.canvas_width.signal_cloned().to_stream().fuse();
            while width_stream.next().await.is_some() {
                timeline.update_render_state();
                timeline.schedule_request();
                timeline.schedule_config_save();
            }
        }));

        vec![viewport_handle, width_handle]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_cursor_values_actor_signal_propagation() {
        let cursor_values: Mutable<BTreeMap<String, SignalValue>> = Mutable::new(BTreeMap::new());

        let unique_id = "/path/to/file.vcd|scope|clk".to_string();

        {
            let mut map = cursor_values.lock_mut();
            map.insert(unique_id.clone(), SignalValue::Present("1".to_string()));
        }

        let map = cursor_values.lock_ref();
        let value = map.get(&unique_id).cloned();

        assert_eq!(
            value,
            Some(SignalValue::Present("1".to_string())),
            "Cursor value should be retrievable after insertion via lock_mut()"
        );
    }

    #[test]
    fn test_cursor_values_map_lookup_format() {
        let mut map: BTreeMap<String, SignalValue> = BTreeMap::new();

        let unique_id_inserted =
            "/home/user/file.vcd|counter_tb|clk".to_string();
        let unique_id_queried =
            "/home/user/file.vcd|counter_tb|clk".to_string();

        map.insert(unique_id_inserted, SignalValue::Present("0".to_string()));

        let result = map.get(&unique_id_queried).cloned();
        assert_eq!(
            result,
            Some(SignalValue::Present("0".to_string())),
            "Map lookup should find value when key format matches exactly"
        );
    }

    #[test]
    fn test_cursor_values_signal_cloned_map_access() {
        let cursor_values: Mutable<BTreeMap<String, SignalValue>> = Mutable::new(BTreeMap::new());

        let unique_id = "/path/to/file.vcd|scope|var".to_string();
        let unique_id_for_lookup = unique_id.clone();

        {
            let mut map = cursor_values.lock_mut();
            map.insert(unique_id.clone(), SignalValue::Present("test_value".to_string()));
        }

        let map_snapshot = cursor_values.get_cloned();
        let value_from_snapshot = map_snapshot.get(&unique_id_for_lookup).cloned();

        assert_eq!(
            value_from_snapshot,
            Some(SignalValue::Present("test_value".to_string())),
            "Value should be accessible from cloned map snapshot"
        );
    }

    #[test]
    fn test_cursor_values_none_for_missing_key() {
        let cursor_values: Mutable<BTreeMap<String, SignalValue>> = Mutable::new(BTreeMap::new());

        {
            let mut map = cursor_values.lock_mut();
            map.insert(
                "/file.vcd|scope|existing_var".to_string(),
                SignalValue::Present("1".to_string()),
            );
        }

        let map_snapshot = cursor_values.get_cloned();
        let value = map_snapshot.get("/file.vcd|scope|missing_var").cloned();

        assert_eq!(
            value, None,
            "Missing key should return None, not panic or return wrong value"
        );
    }

    #[test]
    fn test_cursor_values_overwrite() {
        let cursor_values: Mutable<BTreeMap<String, SignalValue>> = Mutable::new(BTreeMap::new());

        let unique_id = "/file.vcd|scope|var".to_string();

        {
            let mut map = cursor_values.lock_mut();
            map.insert(unique_id.clone(), SignalValue::Loading);
        }

        {
            let map = cursor_values.lock_ref();
            assert_eq!(
                map.get(&unique_id).cloned(),
                Some(SignalValue::Loading),
                "Initial value should be Loading"
            );
        }

        {
            let mut map = cursor_values.lock_mut();
            map.insert(unique_id.clone(), SignalValue::Present("42".to_string()));
        }

        {
            let map = cursor_values.lock_ref();
            assert_eq!(
                map.get(&unique_id).cloned(),
                Some(SignalValue::Present("42".to_string())),
                "Value should be overwritten from Loading to Present"
            );
        }
    }
}
