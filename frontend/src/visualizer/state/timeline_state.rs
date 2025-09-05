// ✅ REMOVED: All imports cleaned up as global mutables eliminated
// All timeline state is now managed by Actor+Relay architecture in timeline_actor.rs

// ===== MIGRATED FROM STATE.RS: Timeline-related global state =====

// ✅ REMOVED: UNIFIED_TIMELINE_CACHE migrated to waveform_timeline domain unified_timeline_cache_signal()

// ✅ MIGRATED: Cursor initialization → use cursor_initialized_signal() from waveform_timeline domain

// ✅ REMOVED: Zoom control globals → migrated to internal Actor state in timeline_actor.rs
// Use is_zooming_in_signal() / is_zooming_out_signal() from waveform_timeline domain

// ✅ REMOVED: Pan control globals → migrated to internal Actor state in timeline_actor.rs  
// Use is_panning_left_signal() / is_panning_right_signal() from waveform_timeline domain

// ✅ REMOVED: Cursor movement globals → migrated to internal Actor state in timeline_actor.rs
// Use is_cursor_moving_left/right_signal() from waveform_timeline domain

// ✅ REMOVED: Shift key global → migrated to internal Actor state in timeline_actor.rs
// Use is_shift_pressed_signal() from waveform_timeline domain

// ✅ REMOVED: MOUSE_TIME_NS migrated to waveform_timeline domain mouse_time_ns_signal()

// MIGRATED: Zoom center → use zoom_center_ns_signal() from waveform_timeline

// ✅ REMOVED: SIGNAL_VALUES migrated to waveform_timeline domain signal_values_signal()

// ✅ REMOVED: Variable formats global → migrated to internal Actor state in timeline_actor.rs
// Use selected_variable_formats_signal() from waveform_timeline domain