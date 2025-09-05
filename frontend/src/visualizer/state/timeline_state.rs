use zoon::*;
use std::collections::HashMap;
use crate::visualizer::timeline::time_types::{TimeNs, TimelineCache};

// ===== MIGRATED FROM STATE.RS: Timeline-related global state =====

// MIGRATED: Timeline cache → use unified_timeline_cache_signal() from waveform_timeline
pub static UNIFIED_TIMELINE_CACHE: Lazy<Mutable<TimelineCache>> = Lazy::new(|| Mutable::new(TimelineCache::new()));

// MIGRATED: Cursor initialization → use startup_cursor_position_set_signal() from waveform_timeline
pub static STARTUP_CURSOR_POSITION_SET: Lazy<Mutable<bool>> = lazy::default();

// MIGRATED: Zoom control → use is_zooming_in_signal() / is_zooming_out_signal() from waveform_timeline
pub static IS_ZOOMING_IN: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Pan control → use is_panning_left_signal() / is_panning_right_signal() from waveform_timeline
pub static IS_PANNING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_PANNING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Cursor movement → use is_cursor_moving_left/right_signal() from waveform_timeline
pub static IS_CURSOR_MOVING_LEFT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
pub static IS_CURSOR_MOVING_RIGHT: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Shift key → use is_shift_pressed_signal() from waveform_timeline
pub static IS_SHIFT_PRESSED: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// MIGRATED: Mouse tracking → use mouse_x_position_signal() / mouse_time_ns_signal() from waveform_timeline
pub static MOUSE_TIME_NS: Lazy<Mutable<TimeNs>> = Lazy::new(|| Mutable::new(TimeNs::ZERO));

// MIGRATED: Zoom center → use zoom_center_ns_signal() from waveform_timeline

// MIGRATED: Signal values → use signal_values_signal() from waveform_timeline
pub static SIGNAL_VALUES: Lazy<Mutable<HashMap<String, crate::visualizer::formatting::signal_values::SignalValue>>> = lazy::default();

// MIGRATED: Variable formats → use selected_variable_formats_signal() from waveform_timeline
pub static SELECTED_VARIABLE_FORMATS: Lazy<Mutable<HashMap<String, shared::VarFormat>>> = lazy::default();