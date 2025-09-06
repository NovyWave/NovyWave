// All timeline state is now managed by Actor+Relay architecture in timeline_actor.rs

// ===== MIGRATED FROM STATE.RS: Timeline-related global state =====



// Use is_zooming_in_signal() / is_zooming_out_signal() from waveform_timeline domain

// Use is_panning_left_signal() / is_panning_right_signal() from waveform_timeline domain

// Use is_cursor_moving_left/right_signal() from waveform_timeline domain

// Use is_shift_pressed_signal() from waveform_timeline domain


// MIGRATED: Zoom center → use zoom_center_ns_signal() from waveform_timeline


// Use selected_variable_formats_signal() from waveform_timeline domain