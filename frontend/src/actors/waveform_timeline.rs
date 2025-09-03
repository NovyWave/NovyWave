//! WaveformTimeline domain for timeline management using Actor+Relay architecture
//!
//! Consolidated timeline management domain to replace global mutables with event-driven architecture.
//! Manages cursor position, viewport ranges, zoom levels, and cached waveform data.

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::actors::{Actor, ActorMap, Relay, relay};
use crate::actors::global_domains::waveform_timeline_domain;
use crate::time_types::{TimeNs, Viewport, NsPerPixel, TimelineCoordinates, TimelineCache};
use shared::{SignalTransition, SignalValue, WaveformFile, VarFormat};
use zoon::{MutableExt, SignalVecExt, SignalExt};
use futures::{StreamExt, select};
use std::collections::{BTreeMap, HashMap};
use crate::format_utils;

/// Domain-driven timeline management with Actor+Relay architecture.
/// 
/// Replaces ALL 25 timeline-related global mutables with cohesive event-driven state management.
/// Handles cursor position, viewport, zoom/pan, mouse tracking, canvas state, and signal caching.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct WaveformTimeline {
    // === CORE TIMELINE STATE (15 mutables from state.rs) ===
    /// Current cursor position in nanoseconds
    cursor_position: Actor<TimeNs>,
    
    /// Timeline viewport (visible time range)
    viewport: Actor<Viewport>,
    
    /// Timeline resolution (nanoseconds per pixel)
    ns_per_pixel: Actor<NsPerPixel>,
    
    /// Unified timeline coordinates for integer-based calculations
    coordinates: Actor<TimelineCoordinates>,
    
    /// Unified timeline cache - replaces 4 separate cache systems
    cache: Actor<TimelineCache>,
    
    /// Track if cursor position was set during startup
    cursor_initialized: Actor<bool>,
    
    /// Smooth zoom control flags
    zooming_in: Actor<bool>,
    zooming_out: Actor<bool>,
    
    /// Smooth pan control flags  
    panning_left: Actor<bool>,
    panning_right: Actor<bool>,
    
    /// Smooth cursor movement control flags
    cursor_moving_left: Actor<bool>,
    cursor_moving_right: Actor<bool>,
    
    /// Shift key state tracking for modifier combinations
    shift_pressed: Actor<bool>,
    
    /// Mouse position tracking for zoom center
    mouse_x: Actor<f32>,
    mouse_time: Actor<TimeNs>,
    
    // === ZOOM/PAN STATE (5 mutables from state.rs) ===
    /// Zoom center position (in nanoseconds)
    zoom_center: Actor<TimeNs>,
    
    /// Canvas dimensions for coordinate calculations
    canvas_width: Actor<f32>,
    canvas_height: Actor<f32>,
    
    /// Current signal values at cursor position
    signal_values: ActorMap<String, format_utils::SignalValue>,
    signal_values_hashmap_signal: zoon::Mutable<HashMap<String, format_utils::SignalValue>>,  // Dedicated signal for efficient HashMap access
    
    /// Format selections for selected variables
    variable_formats: ActorMap<String, VarFormat>,
    variable_formats_hashmap_signal: zoon::Mutable<HashMap<String, VarFormat>>,  // Dedicated signal for efficient HashMap access
    
    // === CANVAS STATE (5 mutables from waveform_canvas.rs) ===
    /// Track pending backend requests
    has_pending_request: Actor<bool>,
    
    /// Processed canvas cache for rendering optimization
    canvas_cache: ActorMap<String, Vec<(f32, SignalValue)>>,
    canvas_cache_hashmap_signal: zoon::Mutable<HashMap<String, Vec<(f32, SignalValue)>>>,  // Dedicated signal for efficient HashMap access
    
    /// Force redraw counter for invalidation
    force_redraw: Actor<u32>,
    
    /// Last redraw time for performance tracking
    last_redraw_time: Actor<f64>,
    
    /// Last canvas update timestamp
    last_canvas_update: Actor<u64>,
    
    /// Timeline statistics and metadata
    timeline_stats: Actor<TimelineStats>,
    
    // === USER TIMELINE INTERACTION EVENTS ===
    /// User clicked on timeline canvas at specific time
    pub cursor_clicked_relay: Relay<TimeNs>,
    
    /// User moved cursor to specific time
    pub cursor_moved_relay: Relay<TimeNs>,
    
    /// User started zoom in gesture
    pub zoom_in_started_relay: Relay<TimeNs>,
    
    /// User started zoom out gesture
    pub zoom_out_started_relay: Relay<TimeNs>,
    
    /// User started panning left
    pub pan_left_started_relay: Relay<()>,
    
    /// User started panning right
    pub pan_right_started_relay: Relay<()>,
    
    /// User moved mouse over canvas (position and time)
    pub mouse_moved_relay: Relay<(f32, TimeNs)>,
    
    /// Canvas dimensions changed (resize)
    pub canvas_resized_relay: Relay<(f32, f32)>,
    
    /// Force redraw requested
    pub redraw_requested_relay: Relay<()>,
    
    /// Signal values updated from backend
    pub signal_values_updated_relay: Relay<HashMap<String, format_utils::SignalValue>>,
    
    // === KEYBOARD NAVIGATION EVENTS ===
    /// User pressed left arrow key
    pub left_key_pressed_relay: Relay<()>,
    
    /// User pressed right arrow key
    pub right_key_pressed_relay: Relay<()>,
    
    /// User pressed zoom in key
    pub zoom_in_pressed_relay: Relay<()>,
    
    /// User pressed zoom out key
    pub zoom_out_pressed_relay: Relay<()>,
    
    /// User pressed pan left key
    pub pan_left_pressed_relay: Relay<()>,
    
    /// User pressed pan right key  
    pub pan_right_pressed_relay: Relay<()>,
    
    /// User pressed jump to previous transition key
    pub jump_to_previous_pressed_relay: Relay<()>,
    
    /// User pressed jump to next transition key
    pub jump_to_next_pressed_relay: Relay<()>,
    
    /// User pressed reset zoom key
    pub reset_zoom_pressed_relay: Relay<()>,
    
    /// User pressed reset zoom center key
    pub reset_zoom_center_pressed_relay: Relay<()>,
    
    /// User clicked fit all button
    pub fit_all_clicked_relay: Relay<()>,
    
    // === SYSTEM TIMELINE EVENTS ===
    /// Waveform data loaded from file
    pub data_loaded_relay: Relay<(String, WaveformFile)>,
    
    /// Signal transitions cached for rendering
    pub transitions_cached_relay: Relay<(String, Vec<SignalTransition>)>,
    
    /// Cursor values updated from cached data
    pub cursor_values_updated_relay: Relay<BTreeMap<String, SignalValue>>,
    
    /// Timeline range calculated from loaded data
    pub timeline_bounds_calculated_relay: Relay<(f64, f64)>,
    
    /// Viewport changed due to resize or user action
    pub viewport_changed_relay: Relay<(f64, f64)>,
    
    /// Ns per pixel changed for zoom display synchronization
    pub ns_per_pixel_changed_relay: Relay<NsPerPixel>,
}

/// Timeline statistics and metadata
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct TimelineStats {
    pub total_signals: usize,
    pub cached_transitions: usize,
    pub min_time: f64,
    pub max_time: f64,
    pub time_range: f64,
}

#[allow(dead_code)]
impl WaveformTimeline {
    /// Create a new WaveformTimeline domain with comprehensive event processors
    /// 
    /// Migrates ALL 25 global mutables to Actor+Relay architecture:
    /// - 15 core timeline mutables from state.rs
    /// - 5 zoom/pan mutables from state.rs  
    /// - 5 canvas mutables from waveform_canvas.rs
    pub async fn new() -> Self {
        // Create relays for comprehensive user interactions
        let (cursor_clicked_relay, cursor_clicked_stream) = relay::<TimeNs>();
        let (cursor_moved_relay, cursor_moved_stream) = relay::<TimeNs>();
        let (zoom_in_started_relay, zoom_in_started_stream) = relay::<TimeNs>();
        let (zoom_out_started_relay, zoom_out_started_stream) = relay::<TimeNs>();
        let (pan_left_started_relay, pan_left_started_stream) = relay::<()>();
        let (pan_right_started_relay, pan_right_started_stream) = relay::<()>();
        let (mouse_moved_relay, mouse_moved_stream) = relay::<(f32, TimeNs)>();
        let (canvas_resized_relay, canvas_resized_stream) = relay::<(f32, f32)>();
        let (redraw_requested_relay, redraw_requested_stream) = relay::<()>();
        let (signal_values_updated_relay, signal_values_updated_stream) = relay::<HashMap<String, format_utils::SignalValue>>();
        
        // Create relays for keyboard navigation
        let (left_key_pressed_relay, left_key_pressed_stream) = relay::<()>();
        let (right_key_pressed_relay, right_key_pressed_stream) = relay::<()>();
        let (zoom_in_pressed_relay, zoom_in_pressed_stream) = relay::<()>();
        let (zoom_out_pressed_relay, zoom_out_pressed_stream) = relay::<()>();
        let (pan_left_pressed_relay, _pan_left_pressed_stream) = relay::<()>();
        let (pan_right_pressed_relay, _pan_right_pressed_stream) = relay::<()>();
        let (jump_to_previous_pressed_relay, _jump_to_previous_pressed_stream) = relay::<()>();
        let (jump_to_next_pressed_relay, _jump_to_next_pressed_stream) = relay::<()>();
        let (reset_zoom_pressed_relay, reset_zoom_pressed_stream) = relay::<()>();
        let (reset_zoom_center_pressed_relay, reset_zoom_center_pressed_stream) = relay::<()>();
        let (fit_all_clicked_relay, fit_all_clicked_stream) = relay::<()>();
        
        // Create relays for system events
        let (data_loaded_relay, _data_loaded_stream) = relay::<(String, WaveformFile)>();
        let (transitions_cached_relay, _transitions_cached_stream) = relay::<(String, Vec<SignalTransition>)>();
        let (cursor_values_updated_relay, _cursor_values_updated_stream) = relay::<BTreeMap<String, SignalValue>>();
        let (timeline_bounds_calculated_relay, _timeline_bounds_calculated_stream) = relay::<(f64, f64)>();
        let (viewport_changed_relay, _viewport_changed_stream) = relay::<(f64, f64)>();
        let (ns_per_pixel_changed_relay, mut ns_per_pixel_changed_stream) = relay::<NsPerPixel>();
        
        // Create cursor position actor with comprehensive event handling
        let cursor_position = Actor::new(TimeNs::ZERO, async move |cursor_handle| {
            let mut cursor_clicked = cursor_clicked_stream;
            let mut cursor_moved = cursor_moved_stream;
            let mut left_key_pressed = left_key_pressed_stream;
            let mut right_key_pressed = right_key_pressed_stream;
            
            loop {
                select! {
                    event = cursor_clicked.next() => {
                        match event {
                            Some(time_ns) => cursor_handle.set(time_ns),
                            None => break,
                        }
                    }
                    event = cursor_moved.next() => {
                        match event {
                            Some(time_ns) => cursor_handle.set(time_ns),
                            None => break,
                        }
                    }
                    event = left_key_pressed.next() => {
                        match event {
                            Some(()) => cursor_handle.update_mut(|current| {
                                // Calculate adaptive step size based on visible time range
                                let step_size = calculate_adaptive_cursor_step();
                                let new_time = current.nanos().saturating_sub(step_size);
                                *current = TimeNs::from_nanos(new_time);
                            }),
                            None => break,
                        }
                    }
                    event = right_key_pressed.next() => {
                        match event {
                            Some(()) => cursor_handle.update_mut(|current| {
                                // Calculate adaptive step size based on visible time range
                                let step_size = calculate_adaptive_cursor_step();
                                let old_time = current.nanos();
                                let new_time = old_time.saturating_add(step_size);
                                *current = TimeNs::from_nanos(new_time);
                                zoon::println!("🎯 CURSOR: Right key pressed - moved from {}ns to {}ns (step: {}ns)", old_time, new_time, step_size);
                            }),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        // Create viewport actor with comprehensive event handling
        let viewport = Actor::new(
            Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(250.0)), 
            async move |viewport_handle| {
                let mut viewport_changed = _viewport_changed_stream;
                let mut timeline_bounds_calculated = _timeline_bounds_calculated_stream;
                let mut fit_all_clicked = fit_all_clicked_stream;
                
                loop {
                    select! {
                        event = viewport_changed.next() => {
                            match event {
                                Some((start, end)) => {
                                    let new_viewport = Viewport::new(
                                        TimeNs::from_external_seconds(start),
                                        TimeNs::from_external_seconds(end)
                                    );
                                    viewport_handle.set(new_viewport);
                                }
                                None => break,
                            }
                        }
                        event = timeline_bounds_calculated.next() => {
                            match event {
                                Some((min_time, max_time)) => {
                                    let new_viewport = Viewport::new(
                                        TimeNs::from_external_seconds(min_time),
                                        TimeNs::from_external_seconds(max_time)
                                    );
                                    viewport_handle.set(new_viewport);
                                }
                                None => break,
                            }
                        }
                        event = fit_all_clicked.next() => {
                            match event {
                                Some(()) => {
                                    // Will be updated by timeline bounds calculation
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        );
        
        // Create ns_per_pixel actor with zoom event handling
        let ns_per_pixel = Actor::new(NsPerPixel::MEDIUM_ZOOM, {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            let viewport_for_ns_per_pixel = viewport.clone();
            async move |ns_per_pixel_handle| {
                let mut zoom_in_started = zoom_in_started_stream;
                let mut zoom_out_started = zoom_out_started_stream;
                let mut zoom_in_pressed = zoom_in_pressed_stream;
                let mut zoom_out_pressed = zoom_out_pressed_stream;
                let mut reset_zoom_pressed = reset_zoom_pressed_stream;
                let mut reset_zoom_center_pressed = reset_zoom_center_pressed_stream;
                let mut ns_per_pixel_changed = ns_per_pixel_changed_stream;
                let mut canvas_resized = canvas_resized_relay_clone.subscribe();
            
            loop {
                select! {
                    event = zoom_in_started.next() => {
                        match event {
                            Some(_center_time) => {
                                let current = ns_per_pixel_handle.get();
                                ns_per_pixel_handle.set_neq(current.zoom_in_smooth(0.3));
                            }
                            None => break,
                        }
                    }
                    event = zoom_out_started.next() => {
                        match event {
                            Some(_center_time) => {
                                let current = ns_per_pixel_handle.get();
                                ns_per_pixel_handle.set_neq(current.zoom_out_smooth(0.3));
                            }
                            None => break,
                        }
                    }
                    event = zoom_in_pressed.next() => {
                        match event {
                            Some(()) => {
                                let current = ns_per_pixel_handle.get();
                                ns_per_pixel_handle.set_neq(current.zoom_in_smooth(0.3));
                            }
                            None => break,
                        }
                    }
                    event = zoom_out_pressed.next() => {
                        match event {
                            Some(()) => {
                                let current = ns_per_pixel_handle.get();
                                ns_per_pixel_handle.set_neq(current.zoom_out_smooth(0.3));
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom_pressed.next() => {
                        match event {
                            Some(()) => {
                                // MANDATORY DEBUG: Verify Actor is running and receiving events
                                zoon::println!("🚨🚨🚨 WaveformTimeline Actor IS RUNNING: reset_zoom_pressed event received!");
                                
                                // CRITICAL DEBUG: Track R key calculation cycling
                                use std::sync::atomic::{AtomicU32, Ordering};
                                static R_KEY_COUNTER: AtomicU32 = AtomicU32::new(0);
                                let r_count = R_KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
                                
                                // ITERATION 7: Debug logging (removed SystemTime - not available in WASM)
                                zoon::println!("🚨 R KEY DEBUG #{}: Starting contextual zoom calculation", r_count);
                                
                                // Debug current Actor state before calculation
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                let current_viewport = crate::actors::waveform_timeline::current_viewport();
                                let current_coords = crate::actors::waveform_timeline::current_coordinates();
                                
                                // ITERATION 4: Additional Actor state consistency checks (using public signals since handles not in scope)
                                // Note: We can only access ns_per_pixel_handle directly within this Actor
                                
                                zoon::println!("🚨 BEFORE CALCULATION:");
                                zoon::println!("   Current ns_per_pixel: {} ({:.3}ms/px)", current_ns_per_pixel, current_ns_per_pixel.nanos() as f64 / 1_000_000.0);
                                zoon::println!("   Current viewport: {:.3}s to {:.3}s", current_viewport.start.display_seconds(), current_viewport.end.display_seconds());
                                zoon::println!("   Coords cache: viewport={:.3}s-{:.3}s, canvas_width={}px, ns_per_pixel={}", 
                                    current_coords.viewport_start_ns.display_seconds(),
                                    (current_coords.viewport_start_ns.nanos() + (current_coords.canvas_width_pixels as u64 * current_coords.ns_per_pixel.nanos())) as f64 / 1_000_000_000.0,
                                    current_coords.canvas_width_pixels,
                                    current_coords.ns_per_pixel);
                                    
                                // ITERATION 4: Check Actor state consistency between calls (limited to available state)
                                static mut PREVIOUS_STATE: Option<(u64, u64, u64, u32)> = None;
                                let current_state = (
                                    current_ns_per_pixel.nanos(),
                                    current_viewport.start.nanos(),
                                    current_viewport.end.nanos(),
                                    current_coords.canvas_width_pixels
                                );
                                unsafe {
                                    if let Some(prev) = PREVIOUS_STATE {
                                        if current_state != prev {
                                            zoon::println!("🚨 ACTOR STATE CHANGED:");
                                            if current_state.0 != prev.0 { zoon::println!("   ns_per_pixel: {} -> {} ({:.3}ms/px -> {:.3}ms/px)", prev.0, current_state.0, prev.0 as f64 / 1_000_000.0, current_state.0 as f64 / 1_000_000.0); }
                                            if current_state.1 != prev.1 { zoon::println!("   viewport_start: {} -> {} ({:.6}s -> {:.6}s)", prev.1, current_state.1, prev.1 as f64 / 1_000_000_000.0, current_state.1 as f64 / 1_000_000_000.0); }
                                            if current_state.2 != prev.2 { zoon::println!("   viewport_end: {} -> {} ({:.6}s -> {:.6}s)", prev.2, current_state.2, prev.2 as f64 / 1_000_000_000.0, current_state.2 as f64 / 1_000_000_000.0); }
                                            if current_state.3 != prev.3 { zoon::println!("   canvas_width: {}px -> {}px", prev.3, current_state.3); }
                                        } else {
                                            zoon::println!("🚨 ACTOR STATE IDENTICAL to previous call");
                                        }
                                    } else {
                                        zoon::println!("🚨 FIRST R KEY PRESS - saving initial state");
                                    }
                                    PREVIOUS_STATE = Some(current_state);
                                }
                                
                                // ITERATION 7: Debug checkpoint 1 - Before canvas width calculation
                                
                                // ITERATION 9 FIX: Use stable canvas width to prevent cycling
                                // Problem: current_canvas_width() returns inconsistent values between R key presses
                                // Solution: Cache canvas width and only update on significant changes
                                static STABLE_CANVAS_WIDTH: std::sync::OnceLock<std::sync::Mutex<u32>> = std::sync::OnceLock::new();
                                
                                let canvas_width = {
                                    let raw_width = crate::actors::waveform_timeline::current_canvas_width() as u32;
                                    let mutex = STABLE_CANVAS_WIDTH.get_or_init(|| std::sync::Mutex::new(raw_width));
                                    let mut cached_width = mutex.lock().unwrap();
                                    
                                    // Only update cached width if it's significantly different (not minor fluctuations)
                                    if *cached_width == 0 || (raw_width as i32 - *cached_width as i32).abs() > 50 {
                                        zoon::println!("🔧 ITERATION 9 FIX: Updating stable canvas width {} → {} (significant change >50px)", 
                                            *cached_width, raw_width);
                                        *cached_width = raw_width;
                                    } else if raw_width != *cached_width {
                                        zoon::println!("🔧 ITERATION 9 FIX: Ignoring canvas width fluctuation {} → {} (using stable {}px)", 
                                            *cached_width, raw_width, *cached_width);
                                    }
                                    
                                    zoon::println!("🔧 CANVAS WIDTH: Using stable {}px for R key calculation [raw: {}px]", 
                                        *cached_width, raw_width);
                                    
                                    *cached_width
                                };
                                
                                if let Some((min_time, max_time)) = crate::waveform_canvas::get_maximum_timeline_range() {
                                    // ITERATION 7: Debug checkpoint 2 - After timeline range calculation
                                    
                                    let time_range_ns = ((max_time - min_time) * 1_000_000_000.0) as u64;
                                    let contextual_zoom = NsPerPixel(time_range_ns / canvas_width as u64);
                                    
                                    zoon::println!("🚨 ITERATION 7: Timeline range calculated");
                                    
                                    // ITERATION 6: Enhanced clamping logic debug
                                    let min_zoom = NsPerPixel(1_000); // 1μs/px (very zoomed in)
                                    let max_zoom = NsPerPixel(10_000_000_000); // 10s/px (very zoomed out)
                                    
                                    zoon::println!("🚨 ITERATION 6 - CLAMPING DEBUG:");
                                    zoon::println!("   Min zoom bound: {} ({:.3}ms/px)", min_zoom, min_zoom.nanos() as f64 / 1_000_000.0);
                                    zoon::println!("   Max zoom bound: {} ({:.3}ms/px)", max_zoom, max_zoom.nanos() as f64 / 1_000_000.0);
                                    zoon::println!("   Contextual zoom before clamp: {} ({:.3}ms/px)", contextual_zoom, contextual_zoom.nanos() as f64 / 1_000_000.0);
                                    
                                    let raw_clamp = contextual_zoom.nanos().clamp(min_zoom.nanos(), max_zoom.nanos());
                                    let clamped_zoom = NsPerPixel(raw_clamp);
                                    
                                    // ITERATION 6: Track clamping behavior
                                    let was_clamped = raw_clamp != contextual_zoom.nanos();
                                    zoon::println!("🚨 CLAMPING RESULT: {} -> {} (was_clamped: {})", 
                                        contextual_zoom.nanos(), raw_clamp, was_clamped);
                                    if was_clamped {
                                        if raw_clamp == min_zoom.nanos() {
                                            zoon::println!("   CLAMPED TO MIN ZOOM (1μs/px)");
                                        } else if raw_clamp == max_zoom.nanos() {
                                            zoon::println!("   CLAMPED TO MAX ZOOM (10s/px)");
                                        } else {
                                            zoon::println!("   ERROR: Unexpected clamping result");
                                        }
                                    } else {
                                        zoon::println!("   NO CLAMPING APPLIED - contextual zoom within bounds");
                                    }
                                    
                                    zoon::println!("🚨 CALCULATION RESULTS:");
                                    zoon::println!("   Timeline range: {:.6}s to {:.6}s (span: {:.6}s)", min_time, max_time, max_time - min_time);
                                    zoon::println!("   Time range in ns: {} ns", time_range_ns);
                                    zoon::println!("   Canvas width used: {}px (STABLE)", canvas_width);
                                    zoon::println!("   Raw calculation: {} ns / {} px = {} ns/px", time_range_ns, canvas_width, time_range_ns / canvas_width as u64);
                                    zoon::println!("   Contextual zoom: {} ({:.3}ms/px)", contextual_zoom, contextual_zoom.nanos() as f64 / 1_000_000.0);
                                    zoon::println!("   Final zoom: {} ({:.3}ms/px) (clamped)", clamped_zoom, clamped_zoom.nanos() as f64 / 1_000_000.0);
                                    
                                    // ITERATION 9 FIX VALIDATION: Track if stable canvas width prevents cycling
                                    static PREVIOUS_ZOOM_RESULT: std::sync::OnceLock<std::sync::Mutex<Option<f64>>> = std::sync::OnceLock::new();
                                    let zoom_ms_per_pixel = clamped_zoom.nanos() as f64 / 1_000_000.0;
                                    let mutex = PREVIOUS_ZOOM_RESULT.get_or_init(|| std::sync::Mutex::new(None));
                                    let mut previous_zoom = mutex.lock().unwrap();
                                    
                                    if let Some(prev_zoom) = *previous_zoom {
                                        if (zoom_ms_per_pixel - prev_zoom).abs() < 0.1 {
                                            zoon::println!("✅ ITERATION 9 SUCCESS: Stable zoom result {:.1}ms/px matches previous {:.1}ms/px", 
                                                zoom_ms_per_pixel, prev_zoom);
                                        } else {
                                            zoon::println!("⚠️ ITERATION 9 NOTE: Zoom changed {:.1}ms/px → {:.1}ms/px (different timeline data?)", 
                                                prev_zoom, zoom_ms_per_pixel);
                                        }
                                    } else {
                                        zoon::println!("🔧 ITERATION 9: First R key press, zoom = {:.1}ms/px (baseline established)", zoom_ms_per_pixel);
                                    }
                                    *previous_zoom = Some(zoom_ms_per_pixel);
                                    
                                    ns_per_pixel_handle.set(clamped_zoom);
                                    
                                    // ITERATION 7: Final debug checkpoint
                                    
                                    zoon::println!("🚨 ITERATION 7 - FINAL: R key calculation completed successfully");
                                        
                                    // ITERATION 8: COMPREHENSIVE SUMMARY per R key press
                                    zoon::println!("═══════════════════════════════════════════════════════");
                                    zoon::println!("🔥 R KEY #{} SUMMARY:", r_count);
                                    zoon::println!("   INPUT DATA:");
                                    zoon::println!("     Timeline: {:.6}s to {:.6}s (span: {:.6}s)", min_time, max_time, max_time - min_time);
                                    zoon::println!("     Canvas: {}px wide", canvas_width);
                                    zoon::println!("   CALCULATION: {} ns ÷ {} px = {} ns/px", time_range_ns, canvas_width, time_range_ns / canvas_width as u64);
                                    zoon::println!("   RESULT: {} ({:.3}ms/px) [clamped: {}]", clamped_zoom, clamped_zoom.nanos() as f64 / 1_000_000.0, was_clamped);
                                    zoon::println!("═══════════════════════════════════════════════════════");
                                } else {
                                    zoon::println!("🚨 R KEY: No timeline range available, using MEDIUM_ZOOM fallback");
                                    ns_per_pixel_handle.set(NsPerPixel::MEDIUM_ZOOM);
                                }
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom_center_pressed.next() => {
                        match event {
                            Some(()) => {
                                zoon::println!("🔧 Z KEY: Reset zoom center pressed (ns_per_pixel Actor)");
                                // Note: Zoom center reset is handled by cursor_position Actor
                                // This is just for logging/debugging
                            }
                            None => break,
                        }
                    }
                    event = ns_per_pixel_changed.next() => {
                        match event {
                            Some(new_ns_per_pixel) => {
                                zoon::println!("🎯 NS_PER_PIXEL_CHANGED: Direct update to {}", new_ns_per_pixel);
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                            }
                            None => break,
                        }
                    }
                    event = canvas_resized.next() => {
                        match event {
                            Some((new_width, _height)) => {
                                // ✅ PROPER FIX: Get current viewport range from viewport actor signal
                                if let Some(current_viewport) = viewport_for_ns_per_pixel.signal().to_stream().next().await {
                                    let viewport_range_ns = current_viewport.end.nanos() - current_viewport.start.nanos();
                                    let corrected_ns_per_pixel = NsPerPixel(viewport_range_ns / new_width as u64);
                                    
                                    zoon::println!("🔍 VIEWPORT DEBUG: Canvas resize detected");
                                    zoon::println!("   Canvas width: {} px", new_width);
                                    zoon::println!("   Viewport start: {} ns ({:.6}s)", current_viewport.start.nanos(), current_viewport.start.display_seconds());
                                    zoon::println!("   Viewport end: {} ns ({:.6}s)", current_viewport.end.nanos(), current_viewport.end.display_seconds());
                                    zoon::println!("   Viewport range: {} ns ({:.6}s)", viewport_range_ns, viewport_range_ns as f64 / 1_000_000_000.0);
                                    zoon::println!("   Calculated zoom: {} ({:.3}ms/px)", corrected_ns_per_pixel, corrected_ns_per_pixel.nanos() as f64 / 1_000_000.0);
                                    
                                    zoon::println!("📏 CANVAS RESIZED: {} px width → recalculating zoom to {} (using actual viewport {:.3}s-{:.3}s)", 
                                        new_width, corrected_ns_per_pixel, 
                                        current_viewport.start.display_seconds(),
                                        current_viewport.end.display_seconds());
                                    
                                    // Only update if the calculation produces a different value
                                    let current_ns_per_pixel = ns_per_pixel_handle.get();
                                    if corrected_ns_per_pixel != current_ns_per_pixel {
                                        ns_per_pixel_handle.set_neq(corrected_ns_per_pixel);
                                        zoon::println!("✅ Updated ns_per_pixel from {} to {} using viewport range {}ns", 
                                            current_ns_per_pixel, corrected_ns_per_pixel, viewport_range_ns);
                                    } else {
                                        zoon::println!("⏸️ No change needed - zoom already correct for viewport range {}ns", viewport_range_ns);
                                    }
                                } else {
                                    zoon::println!("⚠️ Could not get current viewport for canvas resize calculation");
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        // Create coordinates actor for unified timeline coordinate system
        let coordinates = Actor::new(
            TimelineCoordinates::new(
                TimeNs::ZERO,               // cursor_ns
                TimeNs::ZERO,               // viewport_start_ns  
                NsPerPixel::MEDIUM_ZOOM,    // ns_per_pixel
                800                         // canvas_width_pixels (initial)
            ),
            async move |_coords_handle| {
                // Coordinates are updated by other actors
                loop {
                    futures::future::pending::<()>().await;
                }
            }
        );
        
        // Create unified timeline cache actor
        let cache = Actor::new(TimelineCache::new(), async move |_cache_handle| {
            // Cache is updated by other actors
            loop {
                futures::future::pending::<()>().await;
            }
        });
        
        // Create all control flag actors
        let cursor_initialized = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let zooming_in = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let zooming_out = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let panning_left = Actor::new(false, async move |panning_handle| {
            let mut pan_left_started = pan_left_started_stream;
            
            loop {
                select! {
                    event = pan_left_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // TODO: Stop panning after duration or on key release
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let panning_right = Actor::new(false, async move |panning_handle| {
            let mut pan_right_started = pan_right_started_stream;
            
            loop {
                select! {
                    event = pan_right_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // TODO: Stop panning after duration or on key release
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let cursor_moving_left = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let cursor_moving_right = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let shift_pressed = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Mouse tracking actors
        let mouse_x = Actor::new(0.0_f32, async move |mouse_x_handle| {
            let mut mouse_moved = mouse_moved_stream;
            
            loop {
                select! {
                    event = mouse_moved.next() => {
                        match event {
                            Some((x_pos, _time)) => mouse_x_handle.set(x_pos),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let mouse_time = Actor::new(TimeNs::ZERO, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Zoom center actor
        let zoom_center = Actor::new(TimeNs::ZERO, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Canvas dimension actors
        let canvas_width = Actor::new(800.0_f32, {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            async move |width_handle| {
                let mut canvas_resized = canvas_resized_relay_clone.subscribe();
            
            loop {
                select! {
                    event = canvas_resized.next() => {
                        match event {
                            Some((width, _height)) => {
                                width_handle.set(width);
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        let canvas_height = Actor::new(400.0_f32, {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            async move |height_handle| {
                let mut canvas_resized_stream = canvas_resized_relay_clone.subscribe();
                
                loop {
                    select! {
                        event = canvas_resized_stream.next() => {
                            match event {
                                Some((_width, height)) => height_handle.set(height),
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });
        
        // Create dedicated signals for efficient access (no conversion antipatterns)
        let signal_values_hashmap_signal = zoon::Mutable::new(HashMap::new());
        let variable_formats_hashmap_signal = zoon::Mutable::new(HashMap::new());
        let canvas_cache_hashmap_signal = zoon::Mutable::new(HashMap::new());
        
        // Signal values ActorMap
        let signal_values = ActorMap::new(BTreeMap::new(), {
            let signal_values_sync = signal_values_hashmap_signal.clone();
            async move |values_handle| {
            let mut signal_values_updated = signal_values_updated_stream;
            
            loop {
                select! {
                    event = signal_values_updated.next() => {
                        match event {
                            Some(updated_values) => {
                                for (signal_id, value) in updated_values {
                                    values_handle.lock_mut().insert_cloned(signal_id, value);
                                }
                                
                                // Sync dedicated HashMap signal after ActorMap change
                                {
                                    let current_map: HashMap<String, format_utils::SignalValue> = values_handle.lock_ref().iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect();
                                    signal_values_sync.set(current_map);
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        // Variable formats ActorMap
        let variable_formats = ActorMap::new(BTreeMap::new(), async move |_formats_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Canvas state actors
        let has_pending_request = Actor::new(false, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let canvas_cache = ActorMap::new(BTreeMap::new(), async move |_cache_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let force_redraw = Actor::new(0_u32, async move |redraw_handle| {
            let mut redraw_requested = redraw_requested_stream;
            
            loop {
                select! {
                    event = redraw_requested.next() => {
                        match event {
                            Some(()) => redraw_handle.update_mut(|counter| *counter += 1),
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        let last_redraw_time = Actor::new(0.0_f64, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        let last_canvas_update = Actor::new(0_u64, async move |_handle| {
            loop { futures::future::pending::<()>().await; }
        });
        
        // Timeline stats actor
        let timeline_stats = Actor::new(TimelineStats::default(), async move |_stats_handle| {
            loop {
                futures::future::pending::<()>().await;
            }
        });
        
        Self {
            // Core timeline state
            cursor_position,
            viewport,
            ns_per_pixel,
            coordinates,
            cache,
            cursor_initialized,
            
            // Control flags
            zooming_in,
            zooming_out,
            panning_left,
            panning_right,
            cursor_moving_left,
            cursor_moving_right,
            shift_pressed,
            
            // Mouse tracking
            mouse_x,
            mouse_time,
            
            // Zoom/pan state
            zoom_center,
            canvas_width,
            canvas_height,
            signal_values,
            signal_values_hashmap_signal,
            variable_formats,
            variable_formats_hashmap_signal,
            
            // Canvas state
            has_pending_request,
            canvas_cache,
            canvas_cache_hashmap_signal,
            force_redraw,
            last_redraw_time,
            last_canvas_update,
            
            timeline_stats,
            
            // User interaction relays
            cursor_clicked_relay,
            cursor_moved_relay,
            zoom_in_started_relay,
            zoom_out_started_relay,
            pan_left_started_relay,
            pan_right_started_relay,
            mouse_moved_relay,
            canvas_resized_relay,
            redraw_requested_relay,
            signal_values_updated_relay,
            
            // Keyboard navigation relays
            left_key_pressed_relay,
            right_key_pressed_relay,
            zoom_in_pressed_relay,
            zoom_out_pressed_relay,
            pan_left_pressed_relay,
            pan_right_pressed_relay,
            jump_to_previous_pressed_relay,
            jump_to_next_pressed_relay,
            reset_zoom_pressed_relay,
            reset_zoom_center_pressed_relay,
            fit_all_clicked_relay,
            
            // System event relays
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
            ns_per_pixel_changed_relay,
        }
    }
    
    // === REACTIVE SIGNAL ACCESS ===
    
    /// Get reactive signal for cursor position (TimeNs)
    pub fn cursor_position_signal(&self) -> impl zoon::Signal<Item = TimeNs> {
        self.cursor_position.signal()
    }
    
    /// Get reactive signal for cursor position in seconds (for compatibility)
    pub fn cursor_position_seconds_signal(&self) -> impl zoon::Signal<Item = f64> {
        use zoon::SignalExt;
        self.cursor_position.signal().map(|time_ns| time_ns.display_seconds())
    }
    
    /// Get reactive signal for viewport (visible time range)
    pub fn viewport_signal(&self) -> impl zoon::Signal<Item = Viewport> {
        self.viewport.signal()
    }
    
    /// Get reactive signal for nanoseconds per pixel (zoom resolution)
    pub fn ns_per_pixel_signal(&self) -> impl zoon::Signal<Item = NsPerPixel> {
        self.ns_per_pixel.signal()
    }
    
    /// Get reactive signal for timeline coordinates computed from current state
    pub fn coordinates_signal(&self) -> impl zoon::Signal<Item = TimelineCoordinates> {
        zoon::map_ref! {
            let cursor_pos = self.cursor_position.signal(),
            let viewport = self.viewport.signal(),
            let ns_per_pixel = self.ns_per_pixel.signal(),
            let canvas_width = self.canvas_width.signal() =>
            TimelineCoordinates::new(
                *cursor_pos,
                viewport.start,         // Use actual viewport start, not TimeNs::ZERO
                *ns_per_pixel,
                *canvas_width as u32
            )
        }
    }
    
    /// Get reactive signal for unified timeline cache
    pub fn cache_signal(&self) -> impl zoon::Signal<Item = TimelineCache> {
        self.cache.signal()
    }
    
    /// Get reactive signal for cursor initialization status
    pub fn cursor_initialized_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.cursor_initialized.signal()
    }
    
    // === CONTROL FLAG SIGNALS ===
    
    /// Get reactive signal for zoom in status
    pub fn zooming_in_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.zooming_in.signal()
    }
    
    /// Get reactive signal for zoom out status
    pub fn zooming_out_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.zooming_out.signal()
    }
    
    /// Get reactive signal for panning left status
    pub fn panning_left_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.panning_left.signal()
    }
    
    /// Get reactive signal for panning right status
    pub fn panning_right_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.panning_right.signal()
    }
    
    /// Get reactive signal for cursor moving left status
    pub fn cursor_moving_left_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.cursor_moving_left.signal()
    }
    
    /// Get reactive signal for cursor moving right status
    pub fn cursor_moving_right_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.cursor_moving_right.signal()
    }
    
    /// Get reactive signal for shift key pressed status
    pub fn shift_pressed_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.shift_pressed.signal()
    }
    
    // === MOUSE TRACKING SIGNALS ===
    
    /// Get reactive signal for mouse X position
    pub fn mouse_x_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.mouse_x.signal()
    }
    
    /// Get reactive signal for mouse time position
    pub fn mouse_time_signal(&self) -> impl zoon::Signal<Item = TimeNs> {
        self.mouse_time.signal()
    }
    
    // === ZOOM/PAN SIGNALS ===
    
    /// Get reactive signal for zoom center
    pub fn zoom_center_signal(&self) -> impl zoon::Signal<Item = TimeNs> {
        self.zoom_center.signal()
    }
    
    /// Get reactive signal for canvas width
    pub fn canvas_width_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.canvas_width.signal()
    }
    
    /// Get reactive signal for canvas height
    pub fn canvas_height_signal(&self) -> impl zoon::Signal<Item = f32> {
        self.canvas_height.signal()
    }
    
    /// Get reactive signal for all signal values
    pub fn signal_values_signal(&self) -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
        // ✅ FIXED: Use dedicated HashMap signal instead of conversion antipattern
        self.signal_values_hashmap_signal.signal_cloned()
    }
    
    /// Get reactive signal for variable formats
    pub fn variable_formats_signal(&self) -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
        // ✅ FIXED: Use dedicated HashMap signal instead of conversion antipattern
        self.variable_formats_hashmap_signal.signal_cloned()
    }
    
    // === CANVAS STATE SIGNALS ===
    
    /// Get reactive signal for pending request status
    pub fn has_pending_request_signal(&self) -> impl zoon::Signal<Item = bool> {
        self.has_pending_request.signal()
    }
    
    /// Get reactive signal for canvas cache
    pub fn canvas_cache_signal(&self) -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
        // ✅ FIXED: Use dedicated HashMap signal instead of conversion antipattern
        self.canvas_cache_hashmap_signal.signal_cloned()
    }
    
    /// Get reactive signal for force redraw counter
    pub fn force_redraw_signal(&self) -> impl zoon::Signal<Item = u32> {
        self.force_redraw.signal()
    }
    
    /// Get reactive signal for last redraw time
    pub fn last_redraw_time_signal(&self) -> impl zoon::Signal<Item = f64> {
        self.last_redraw_time.signal()
    }
    
    /// Get reactive signal for last canvas update
    pub fn last_canvas_update_signal(&self) -> impl zoon::Signal<Item = u64> {
        self.last_canvas_update.signal()
    }
    
    /// Get reactive signal for timeline statistics
    pub fn timeline_stats_signal(&self) -> impl zoon::Signal<Item = TimelineStats> {
        self.timeline_stats.signal()
    }
    
    /// Get signal for specific signal value
    pub fn signal_value_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<format_utils::SignalValue>> {
        self.signal_values.value_signal(signal_id)
    }
    
    /// Get signal for specific variable format
    pub fn variable_format_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<VarFormat>> {
        self.variable_formats.value_signal(signal_id)
    }
    
    /// Get signal for specific canvas cache entry
    pub fn canvas_cache_for_id(&self, signal_id: String) -> impl zoon::Signal<Item = Option<Vec<(f32, SignalValue)>>> {
        self.canvas_cache.value_signal(signal_id)
    }
    
    /// Check if cursor is within visible range (combined signal)
    pub fn is_cursor_visible_signal(&self) -> impl zoon::Signal<Item = bool> {
        zoon::map_ref! {
            let cursor_pos = self.cursor_position.signal(),
            let viewport = self.viewport.signal() =>
            viewport.contains(*cursor_pos)
        }
    }
    
    /// Get time duration per pixel at current zoom (combined signal)
    pub fn time_per_pixel_signal(&self) -> impl zoon::Signal<Item = f64> {
        zoon::map_ref! {
            let viewport = self.viewport.signal(),
            let canvas_width = self.canvas_width.signal() => {
                let start_seconds = viewport.start.display_seconds();
                let end_seconds = viewport.end.display_seconds();
                (end_seconds - start_seconds) / *canvas_width as f64
            }
        }
    }
    
    /// Get current timeline bounds (convenience method)
    pub fn timeline_bounds_signal(&self) -> impl zoon::Signal<Item = (TimeNs, TimeNs)> {
        self.viewport.signal().map(|viewport| (viewport.start, viewport.end))
    }
    
    /// Get canvas dimensions as combined signal
    pub fn canvas_dimensions_signal(&self) -> impl zoon::Signal<Item = (f32, f32)> {
        zoon::map_ref! {
            let width = self.canvas_width.signal(),
            let height = self.canvas_height.signal() =>
            (*width, *height)
        }
    }
    
    /// Create a dummy instance for relay access during initialization
    /// This prevents panics when timeline functions are called before domain initialization
    pub fn new_dummy_for_initialization() -> Self {
        // unused shared import removed
        
        // Create dummy relays that can be cloned but won't process events meaningfully
        let cursor_clicked_relay = Relay::new();
        let cursor_moved_relay = Relay::new();
        let zoom_in_started_relay = Relay::new();
        let zoom_out_started_relay = Relay::new();
        let pan_left_started_relay = Relay::new();
        let pan_right_started_relay = Relay::new();
        let mouse_moved_relay = Relay::new();
        let canvas_resized_relay = Relay::new();
        let redraw_requested_relay = Relay::new();
        let signal_values_updated_relay = Relay::new();
        let left_key_pressed_relay = Relay::new();
        let right_key_pressed_relay = Relay::new();
        let zoom_in_pressed_relay = Relay::new();
        let zoom_out_pressed_relay = Relay::new();
        let pan_left_pressed_relay = Relay::new();
        let pan_right_pressed_relay = Relay::new();
        let jump_to_previous_pressed_relay = Relay::new();
        let jump_to_next_pressed_relay = Relay::new();
        let reset_zoom_pressed_relay = Relay::new();
        let reset_zoom_center_pressed_relay = Relay::new();
        let fit_all_clicked_relay = Relay::new();
        let data_loaded_relay = Relay::new();
        let transitions_cached_relay = Relay::new();
        let cursor_values_updated_relay = Relay::new();
        let timeline_bounds_calculated_relay = Relay::new();
        let viewport_changed_relay = Relay::new();
        let ns_per_pixel_changed_relay = Relay::new();

        Self {
            // Create dummy actors with default values
            cursor_position: Actor::new(TimeNs::ZERO, async |_| { loop { futures::future::pending::<()>().await; } }),
            viewport: Actor::new(Viewport::new(TimeNs::ZERO, TimeNs::from_nanos(1_000_000_000)), async |_| { loop { futures::future::pending::<()>().await; } }),
            ns_per_pixel: Actor::new(NsPerPixel::default(), async |_| { loop { futures::future::pending::<()>().await; } }),
            coordinates: Actor::new(TimelineCoordinates::default(), async |_| { loop { futures::future::pending::<()>().await; } }),
            cache: Actor::new(TimelineCache::default(), async |_| { loop { futures::future::pending::<()>().await; } }),
            cursor_initialized: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            zooming_in: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            zooming_out: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            panning_left: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            panning_right: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            cursor_moving_left: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            cursor_moving_right: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            shift_pressed: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            mouse_x: Actor::new(0.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            mouse_time: Actor::new(TimeNs::ZERO, async |_| { loop { futures::future::pending::<()>().await; } }),
            zoom_center: Actor::new(TimeNs::ZERO, async |_| { loop { futures::future::pending::<()>().await; } }),
            canvas_width: Actor::new(800.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            canvas_height: Actor::new(400.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            signal_values: ActorMap::new(BTreeMap::new(), async |_| { loop { futures::future::pending::<()>().await; } }),
            variable_formats: ActorMap::new(BTreeMap::new(), async |_| { loop { futures::future::pending::<()>().await; } }),
            has_pending_request: Actor::new(false, async |_| { loop { futures::future::pending::<()>().await; } }),
            canvas_cache: ActorMap::new(BTreeMap::new(), async |_| { loop { futures::future::pending::<()>().await; } }),
            force_redraw: Actor::new(0, async |_| { loop { futures::future::pending::<()>().await; } }),
            last_redraw_time: Actor::new(0.0, async |_| { loop { futures::future::pending::<()>().await; } }),
            last_canvas_update: Actor::new(0, async |_| { loop { futures::future::pending::<()>().await; } }),
            timeline_stats: Actor::new(TimelineStats::default(), async |_| { loop { futures::future::pending::<()>().await; } }),
            
            // Use the dummy relays
            cursor_clicked_relay,
            cursor_moved_relay,
            zoom_in_started_relay,
            zoom_out_started_relay,
            pan_left_started_relay,
            pan_right_started_relay,
            mouse_moved_relay,
            canvas_resized_relay,
            redraw_requested_relay,
            signal_values_updated_relay,
            left_key_pressed_relay,
            right_key_pressed_relay,
            zoom_in_pressed_relay,
            zoom_out_pressed_relay,
            pan_left_pressed_relay,
            pan_right_pressed_relay,
            jump_to_previous_pressed_relay,
            jump_to_next_pressed_relay,
            reset_zoom_pressed_relay,
            reset_zoom_center_pressed_relay,
            fit_all_clicked_relay,
            data_loaded_relay,
            transitions_cached_relay,
            cursor_values_updated_relay,
            timeline_bounds_calculated_relay,
            viewport_changed_relay,
            ns_per_pixel_changed_relay,
            
            // HashMap-backed signals for reactive access (dummy values for static approach)
            canvas_cache_hashmap_signal: zoon::Mutable::new(HashMap::new()),
            signal_values_hashmap_signal: zoon::Mutable::new(HashMap::new()),
            variable_formats_hashmap_signal: zoon::Mutable::new(HashMap::new()),
        }
    }

    // === STATIC SIGNAL ACCESSORS FOR GLOBAL FUNCTIONS ===
    
    /// Static version of variable_formats_signal for global access → CONNECTED TO DOMAIN SIGNAL
    pub fn variable_formats_signal_static() -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
        use std::sync::OnceLock;
        static VARIABLE_FORMATS_SIGNAL: OnceLock<zoon::Mutable<HashMap<String, VarFormat>>> = OnceLock::new();
        
        let signal = VARIABLE_FORMATS_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(HashMap::new());
            
            // Connect to real domain signal if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.variable_formats_hashmap_signal.signal_cloned().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal_cloned()
    }
    
    /// Static version of has_pending_request_signal for global access
    pub fn has_pending_request_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.has_pending_request.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false pending request signal");
                use std::sync::OnceLock;
                static FALLBACK_PENDING_REQUEST: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_PENDING_REQUEST.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
    
    /// Static version of canvas_cache_signal for global access → CONNECTED TO DOMAIN SIGNAL
    pub fn canvas_cache_signal_static() -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
        use std::sync::OnceLock;
        static CANVAS_CACHE_SIGNAL: OnceLock<zoon::Mutable<HashMap<String, Vec<(f32, SignalValue)>>>> = OnceLock::new();
        
        let signal = CANVAS_CACHE_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(HashMap::new());
            
            // Connect to real domain signal if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.canvas_cache_hashmap_signal.signal_cloned().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal_cloned()
    }
    
    /// Static version of force_redraw_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn force_redraw_signal_static() -> impl zoon::Signal<Item = u32> {
        use std::sync::OnceLock;
        static FORCE_REDRAW_SIGNAL: OnceLock<zoon::Mutable<u32>> = OnceLock::new();
        
        let signal = FORCE_REDRAW_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.force_redraw.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of last_redraw_time_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn last_redraw_time_signal_static() -> impl zoon::Signal<Item = f64> {
        use std::sync::OnceLock;
        static LAST_REDRAW_TIME_SIGNAL: OnceLock<zoon::Mutable<f64>> = OnceLock::new();
        
        let signal = LAST_REDRAW_TIME_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(0.0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.last_redraw_time.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of last_canvas_update_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn last_canvas_update_signal_static() -> impl zoon::Signal<Item = u64> {
        use std::sync::OnceLock;
        static LAST_CANVAS_UPDATE_SIGNAL: OnceLock<zoon::Mutable<u64>> = OnceLock::new();
        
        let signal = LAST_CANVAS_UPDATE_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.last_canvas_update.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of mouse_x_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn mouse_x_signal_static() -> impl zoon::Signal<Item = f32> {
        use std::sync::OnceLock;
        static MOUSE_X_SIGNAL: OnceLock<zoon::Mutable<f32>> = OnceLock::new();
        
        let signal = MOUSE_X_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(0.0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.mouse_x.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of mouse_time_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn mouse_time_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        use std::sync::OnceLock;
        static MOUSE_TIME_SIGNAL: OnceLock<zoon::Mutable<TimeNs>> = OnceLock::new();
        
        let signal = MOUSE_TIME_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(TimeNs::ZERO);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.mouse_time.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of zoom_center_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn zoom_center_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        use std::sync::OnceLock;
        static ZOOM_CENTER_SIGNAL: OnceLock<zoon::Mutable<TimeNs>> = OnceLock::new();
        
        let signal = ZOOM_CENTER_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(TimeNs::ZERO);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.zoom_center.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of canvas_width_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn canvas_width_signal_static() -> impl zoon::Signal<Item = f32> {
        use std::sync::OnceLock;
        static CANVAS_WIDTH_SIGNAL: OnceLock<zoon::Mutable<f32>> = OnceLock::new();
        
        let signal = CANVAS_WIDTH_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(800.0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.canvas_width.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of canvas_height_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn canvas_height_signal_static() -> impl zoon::Signal<Item = f32> {
        use std::sync::OnceLock;
        static CANVAS_HEIGHT_SIGNAL: OnceLock<zoon::Mutable<f32>> = OnceLock::new();
        
        let signal = CANVAS_HEIGHT_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(400.0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.canvas_height.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of signal_values_signal for global access → CONNECTED TO DOMAIN SIGNAL
    pub fn signal_values_signal_static() -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
        use std::sync::OnceLock;
        static SIGNAL_VALUES_SIGNAL: OnceLock<zoon::Mutable<HashMap<String, format_utils::SignalValue>>> = OnceLock::new();
        
        let signal = SIGNAL_VALUES_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(HashMap::new());
            
            // Connect to real domain signal if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.signal_values_hashmap_signal.signal_cloned().for_each(move |value| {
                    mutable_clone.set(value);  // Use set() since SignalValue doesn't implement PartialEq
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal_cloned()
    }

    // === MORE STATIC SIGNAL ACCESSORS ===
    
    /// Static version of cursor_position_signal for global access
    pub fn cursor_position_signal_static() -> impl zoon::Signal<Item = TimeNs> {
        use std::sync::OnceLock;
        static CURSOR_POSITION_SIGNAL: OnceLock<zoon::Mutable<TimeNs>> = OnceLock::new();
        
        let signal = CURSOR_POSITION_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(TimeNs::ZERO);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.cursor_position.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of cursor_position_seconds_signal for global access  
    pub fn cursor_position_seconds_signal_static() -> impl zoon::Signal<Item = f64> {
        use std::sync::OnceLock;
        static CURSOR_SECONDS_SIGNAL: OnceLock<zoon::Mutable<f64>> = OnceLock::new();
        
        let signal = CURSOR_SECONDS_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(0.0);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.cursor_position.signal().for_each(move |value| {
                    mutable_clone.set_neq(value.display_seconds());
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of viewport_signal for global access
    /// Returns None until actual VCD data is loaded - no fallbacks!
    pub fn viewport_signal_static() -> impl zoon::Signal<Item = Option<Viewport>> {
        use std::sync::OnceLock;
        static VIEWPORT_SIGNAL: OnceLock<zoon::Mutable<Option<Viewport>>> = OnceLock::new();
        
        let signal = VIEWPORT_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(None); // ✅ No default - None until real VCD data is available
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.viewport.signal().for_each(move |value| {
                    mutable_clone.set_neq(Some(value)); // Wrap in Some() since we return Option<Viewport>
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of ns_per_pixel_signal for global access
    pub fn ns_per_pixel_signal_static() -> impl zoon::Signal<Item = NsPerPixel> {
        use std::sync::OnceLock;
        static NS_PER_PIXEL_SIGNAL: OnceLock<zoon::Mutable<NsPerPixel>> = OnceLock::new();
        
        let signal = NS_PER_PIXEL_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(NsPerPixel::default());
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.ns_per_pixel.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of coordinates_signal for global access
    pub fn coordinates_signal_static() -> impl zoon::Signal<Item = TimelineCoordinates> {
        use std::sync::OnceLock;
        static COORDINATES_SIGNAL: OnceLock<zoon::Mutable<TimelineCoordinates>> = OnceLock::new();
        
        let signal = COORDINATES_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(TimelineCoordinates::default());
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.coordinates_signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of cache_signal for global access
    pub fn cache_signal_static() -> impl zoon::Signal<Item = ()> {
        use std::sync::OnceLock;
        static CACHE_SIGNAL: OnceLock<zoon::Mutable<()>> = OnceLock::new();
        
        let signal = CACHE_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(());
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.cache.signal().for_each(move |_| {
                    mutable_clone.set_neq(());
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of cursor_initialized_signal for global access
    pub fn cursor_initialized_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.cursor_initialized.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false cursor initialized signal");
                use std::sync::OnceLock;
                static FALLBACK_CURSOR_INIT: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_CURSOR_INIT.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
    
    // === CONTROL FLAGS STATIC SIGNALS ===
    
    /// Static version of zooming_in_signal for global access
    pub fn zooming_in_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.zooming_in.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false zooming in signal");
                use std::sync::OnceLock;
                static FALLBACK_ZOOMING_IN: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_ZOOMING_IN.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
    
    /// Static version of zooming_out_signal for global access
    pub fn zooming_out_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.zooming_out.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false zooming out signal");
                use std::sync::OnceLock;
                static FALLBACK_ZOOMING_OUT: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_ZOOMING_OUT.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
    
    /// Static version of panning_left_signal for global access
    pub fn panning_left_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.panning_left.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false panning left signal");
                use std::sync::OnceLock;
                static FALLBACK_PANNING_LEFT: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_PANNING_LEFT.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
    
    /// Static version of panning_right_signal for global access
    pub fn panning_right_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.panning_right.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false panning right signal");
                use std::sync::OnceLock;
                static FALLBACK_PANNING_RIGHT: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_PANNING_RIGHT.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
    
    /// Static version of cursor_moving_left_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn cursor_moving_left_signal_static() -> impl zoon::Signal<Item = bool> {
        use std::sync::OnceLock;
        static CURSOR_MOVING_LEFT_SIGNAL: OnceLock<zoon::Mutable<bool>> = OnceLock::new();
        
        let signal = CURSOR_MOVING_LEFT_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(false);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.cursor_moving_left.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of cursor_moving_right_signal for global access → CONNECTED TO DOMAIN ACTOR
    pub fn cursor_moving_right_signal_static() -> impl zoon::Signal<Item = bool> {
        use std::sync::OnceLock;
        static CURSOR_MOVING_RIGHT_SIGNAL: OnceLock<zoon::Mutable<bool>> = OnceLock::new();
        
        let signal = CURSOR_MOVING_RIGHT_SIGNAL.get_or_init(|| {
            let mutable = zoon::Mutable::new(false);
            
            // Connect to real domain actor if available
            if let Some(timeline) = WAVEFORM_TIMELINE_INSTANCE.get() {
                let mutable_clone = mutable.clone();
                zoon::Task::start(timeline.cursor_moving_right.signal().for_each(move |value| {
                    mutable_clone.set_neq(value);
                    async {}
                }));
            }
            
            mutable
        });
        signal.signal()
    }
    
    /// Static version of shift_pressed_signal for global access
    pub fn shift_pressed_signal_static() -> impl zoon::Signal<Item = bool> {
        WAVEFORM_TIMELINE_INSTANCE.get()
            .map(|timeline| timeline.shift_pressed.signal())
            .unwrap_or_else(|| {
                zoon::eprintln!("⚠️ WaveformTimeline not initialized, returning false shift pressed signal");
                use std::sync::OnceLock;
                static FALLBACK_SHIFT_PRESSED: OnceLock<Actor<bool>> = OnceLock::new();
                FALLBACK_SHIFT_PRESSED.get_or_init(|| Actor::new(false, |_| async { loop { futures::future::pending::<()>().await; } })).signal()
            })
    }
}

// === EVENT HANDLER IMPLEMENTATIONS ===

#[allow(dead_code)]
impl WaveformTimeline {
    /// Process loaded waveform data and cache transitions
    fn process_waveform_data(
        transitions_handle: &zoon::MutableBTreeMap<String, Vec<SignalTransition>>,
        file_id: String,
        waveform_data: WaveformFile
    ) {
        // Process each scope and signal in the waveform data
        for scope_data in &waveform_data.scopes {
            for signal_data in &scope_data.variables {
                let signal_id = format!("{}|{}|{}", file_id, scope_data.name, signal_data.name);
                
                // For now, create empty transitions - actual loading will happen via different mechanism
                let transitions: Vec<SignalTransition> = Vec::new();
                
                transitions_handle.lock_mut().insert_cloned(signal_id, transitions);
            }
        }
    }
    
    /// Update cursor values based on current position and cached transitions
    fn update_cursor_values_from_cache(
        cursor_position: f64,
        transitions: &BTreeMap<String, Vec<SignalTransition>>,
        values_handle: &zoon::MutableBTreeMap<String, SignalValue>
    ) {
        for (signal_id, signal_transitions) in transitions {
            // Find the most recent transition at or before cursor position
            let mut current_value = None;
            let cursor_ns = cursor_position * 1_000_000_000.0; // Convert seconds to ns
            
            for transition in signal_transitions.iter() {
                if transition.time_ns as f64 <= cursor_ns {
                    current_value = Some(SignalValue::Present(transition.value.clone()));
                } else {
                    break;
                }
            }
            
            if let Some(value) = current_value {
                values_handle.lock_mut().insert_cloned(signal_id.clone(), value);
            } else {
                values_handle.lock_mut().insert_cloned(signal_id.clone(), SignalValue::Missing);
            }
        }
    }
    
    /// Calculate timeline bounds from all loaded data
    fn calculate_timeline_bounds(transitions: &BTreeMap<String, Vec<SignalTransition>>) -> (f64, f64) {
        let mut min_time = f64::MAX;
        let mut max_time = f64::MIN;
        
        for signal_transitions in transitions.values() {
            if let Some(first) = signal_transitions.first() {
                min_time = min_time.min(first.time_ns as f64 / 1_000_000_000.0);
            }
            if let Some(last) = signal_transitions.last() {
                max_time = max_time.max(last.time_ns as f64 / 1_000_000_000.0);
            }
        }
        
        if min_time == f64::MAX || max_time == f64::MIN {
            (0.0, 1.0) // Default range if no data
        } else {
            (min_time, max_time)
        }
    }
}

// === CONVENIENCE FUNCTIONS FOR UI INTEGRATION ===

// ===== GLOBAL WAVEFORM TIMELINE INSTANCE =====

/// Global WaveformTimeline instance
static WAVEFORM_TIMELINE_INSTANCE: std::sync::OnceLock<WaveformTimeline> = std::sync::OnceLock::new();

/// Initialize the WaveformTimeline domain (call once on app startup)
/// Replaces all 25 global mutables with unified Actor+Relay architecture
pub async fn initialize_waveform_timeline() -> WaveformTimeline {
    let waveform_timeline = WaveformTimeline::new().await;
    if let Err(_) = WAVEFORM_TIMELINE_INSTANCE.set(waveform_timeline.clone()) {
        zoon::eprintln!("⚠️ WaveformTimeline already initialized - ignoring duplicate initialization");
    }
    waveform_timeline
}

/// Get the global WaveformTimeline instance
fn get_waveform_timeline() -> WaveformTimeline {
    // Use the global domains instance (which is properly initialized by initialize_all_domains())
    crate::actors::global_domains::waveform_timeline_domain()
}

// ===== CONVENIENCE FUNCTIONS FOR GLOBAL ACCESS =====

// === CORE TIMELINE STATE ACCESS ===

/// Get cursor position signal (replaces _TIMELINE_CURSOR_NS.signal())
pub fn cursor_position_signal() -> impl zoon::Signal<Item = TimeNs> {
    crate::actors::global_domains::waveform_timeline_domain().cursor_position.signal()
}

/// Get cursor position in seconds signal (compatibility)
pub fn cursor_position_seconds_signal() -> impl zoon::Signal<Item = f64> {
    crate::actors::global_domains::waveform_timeline_domain().cursor_position.signal()
        .map(|time_ns| time_ns.display_seconds())
}

/// Get viewport signal (replaces _TIMELINE_VIEWPORT.signal())
pub fn viewport_signal() -> impl zoon::Signal<Item = Viewport> {
    crate::actors::global_domains::waveform_timeline_domain().viewport.signal()
}

/// Get ns per pixel signal (replaces _TIMELINE_NS_PER_PIXEL.signal())
pub fn ns_per_pixel_signal() -> impl zoon::Signal<Item = NsPerPixel> {
    crate::actors::global_domains::waveform_timeline_domain().ns_per_pixel.signal().map(|value| {
        zoon::println!("📊 UI NS_PER_PIXEL SIGNAL: Actor emitting {}", value);
        value
    })
}

/// Get coordinates signal (replaces _TIMELINE_COORDINATES.signal())
pub fn coordinates_signal() -> impl zoon::Signal<Item = TimelineCoordinates> {
    WaveformTimeline::coordinates_signal_static()
}

/// Get unified cache signal (replaces UNIFIED_TIMELINE_CACHE.signal())
pub fn unified_timeline_cache_signal() -> impl zoon::Signal<Item = ()> {
    WaveformTimeline::cache_signal_static()
}

/// Get cursor initialized signal (replaces STARTUP_CURSOR_POSITION_SET.signal())
pub fn startup_cursor_position_set_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::cursor_initialized_signal_static()
}

// === CONTROL FLAG SIGNALS ===

/// Get zooming in signal (replaces IS_ZOOMING_IN.signal())
pub fn is_zooming_in_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::zooming_in_signal_static()
}

/// Get zooming out signal (replaces IS_ZOOMING_OUT.signal())
pub fn is_zooming_out_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::zooming_out_signal_static()
}

/// Get panning left signal (replaces IS_PANNING_LEFT.signal())
pub fn is_panning_left_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::panning_left_signal_static()
}

/// Get panning right signal (replaces IS_PANNING_RIGHT.signal())
pub fn is_panning_right_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::panning_right_signal_static()
}

/// Get cursor moving left signal (replaces IS_CURSOR_MOVING_LEFT.signal())
pub fn is_cursor_moving_left_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::cursor_moving_left_signal_static()
}

/// Get cursor moving right signal (replaces IS_CURSOR_MOVING_RIGHT.signal())
pub fn is_cursor_moving_right_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::cursor_moving_right_signal_static()
}

/// Get shift pressed signal (replaces IS_SHIFT_PRESSED.signal())
pub fn is_shift_pressed_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::shift_pressed_signal_static()
}

// === MOUSE TRACKING SIGNALS ===

/// Get mouse X position signal (replaces MOUSE_X_POSITION.signal())
pub fn mouse_x_position_signal() -> impl zoon::Signal<Item = f32> {
    WaveformTimeline::mouse_x_signal_static()
}

/// Get mouse time signal (replaces MOUSE_TIME_NS.signal())
pub fn mouse_time_ns_signal() -> impl zoon::Signal<Item = TimeNs> {
    WaveformTimeline::mouse_time_signal_static()
}

// === ZOOM/PAN SIGNALS ===

/// Get zoom center signal (replaces ZOOM_CENTER_NS.signal())
pub fn zoom_center_ns_signal() -> impl zoon::Signal<Item = TimeNs> {
    WaveformTimeline::zoom_center_signal_static()
}

/// Get canvas width signal (replaces CANVAS_WIDTH.signal())
pub fn canvas_width_signal() -> impl zoon::Signal<Item = f32> {
    WaveformTimeline::canvas_width_signal_static()
}

/// Get canvas height signal (replaces CANVAS_HEIGHT.signal())
pub fn canvas_height_signal() -> impl zoon::Signal<Item = f32> {
    WaveformTimeline::canvas_height_signal_static()
}

/// Get signal values signal (replaces SIGNAL_VALUES.signal())
pub fn signal_values_signal() -> impl zoon::Signal<Item = HashMap<String, format_utils::SignalValue>> {
    WaveformTimeline::signal_values_signal_static()
}

/// Get variable formats signal (replaces SELECTED_VARIABLE_FORMATS.signal())
pub fn selected_variable_formats_signal() -> impl zoon::Signal<Item = HashMap<String, VarFormat>> {
    WaveformTimeline::variable_formats_signal_static()
}

// === CANVAS STATE SIGNALS ===

/// Get pending request signal (replaces _HAS_PENDING_REQUEST.signal())
pub fn has_pending_request_signal() -> impl zoon::Signal<Item = bool> {
    WaveformTimeline::has_pending_request_signal_static()
}

/// Get canvas cache signal (replaces PROCESSED_CANVAS_CACHE.signal())
pub fn processed_canvas_cache_signal() -> impl zoon::Signal<Item = HashMap<String, Vec<(f32, SignalValue)>>> {
    WaveformTimeline::canvas_cache_signal_static()
}

/// Get force redraw signal (replaces FORCE_REDRAW.signal())
pub fn force_redraw_signal() -> impl zoon::Signal<Item = u32> {
    WaveformTimeline::force_redraw_signal_static()
}

/// Get last redraw time signal (replaces LAST_REDRAW_TIME.signal())
pub fn last_redraw_time_signal() -> impl zoon::Signal<Item = f64> {
    WaveformTimeline::last_redraw_time_signal_static()
}

/// Get last canvas update signal (replaces LAST_CANVAS_UPDATE.signal())
pub fn last_canvas_update_signal() -> impl zoon::Signal<Item = u64> {
    WaveformTimeline::last_canvas_update_signal_static()
}

// ===== EVENT RELAY FUNCTIONS FOR UI INTEGRATION =====

// === USER INTERACTION EVENTS ===

/// User clicked on timeline at specific time (replaces direct cursor position setting)
pub fn cursor_clicked_relay() -> Relay<TimeNs> {
    get_waveform_timeline().cursor_clicked_relay.clone()
}

/// User moved cursor to specific time
pub fn cursor_moved_relay() -> Relay<TimeNs> {
    get_waveform_timeline().cursor_moved_relay.clone()
}

/// User started zoom in gesture at specific time
pub fn zoom_in_started_relay() -> Relay<TimeNs> {
    get_waveform_timeline().zoom_in_started_relay.clone()
}

/// User started zoom out gesture at specific time
pub fn zoom_out_started_relay() -> Relay<TimeNs> {
    get_waveform_timeline().zoom_out_started_relay.clone()
}

/// User started panning left
pub fn pan_left_started_relay() -> Relay<()> {
    get_waveform_timeline().pan_left_started_relay.clone()
}

/// User started panning right
pub fn pan_right_started_relay() -> Relay<()> {
    get_waveform_timeline().pan_right_started_relay.clone()
}

/// User moved mouse over canvas (position and time)
pub fn mouse_moved_relay() -> Relay<(f32, TimeNs)> {
    get_waveform_timeline().mouse_moved_relay.clone()
}

/// Canvas dimensions changed (resize)
pub fn canvas_resized_relay() -> Relay<(f32, f32)> {
    get_waveform_timeline().canvas_resized_relay.clone()
}

/// Force redraw requested
pub fn redraw_requested_relay() -> Relay<()> {
    get_waveform_timeline().redraw_requested_relay.clone()
}

/// Signal values updated from backend
pub fn signal_values_updated_relay() -> Relay<HashMap<String, format_utils::SignalValue>> {
    get_waveform_timeline().signal_values_updated_relay.clone()
}

// === KEYBOARD NAVIGATION EVENTS ===

/// User pressed left arrow key
pub fn left_key_pressed_relay() -> Relay<()> {
    get_waveform_timeline().left_key_pressed_relay.clone()
}

/// User pressed right arrow key
pub fn right_key_pressed_relay() -> Relay<()> {
    get_waveform_timeline().right_key_pressed_relay.clone()
}

/// User pressed zoom in key
pub fn zoom_in_pressed_relay() -> Relay<()> {
    get_waveform_timeline().zoom_in_pressed_relay.clone()
}

/// User pressed zoom out key
pub fn zoom_out_pressed_relay() -> Relay<()> {
    get_waveform_timeline().zoom_out_pressed_relay.clone()
}

// === SYSTEM EVENTS ===

/// Waveform data loaded from file
pub fn data_loaded_relay() -> Relay<(String, WaveformFile)> {
    get_waveform_timeline().data_loaded_relay.clone()
}

/// Signal transitions cached for rendering
pub fn transitions_cached_relay() -> Relay<(String, Vec<SignalTransition>)> {
    get_waveform_timeline().transitions_cached_relay.clone()
}

/// Viewport changed due to resize or user action
pub fn viewport_changed_relay() -> Relay<(f64, f64)> {
    get_waveform_timeline().viewport_changed_relay.clone()
}

/// Timeline bounds calculated from loaded data
pub fn timeline_bounds_calculated_relay() -> Relay<(f64, f64)> {
    get_waveform_timeline().timeline_bounds_calculated_relay.clone()
}

// === SYNCHRONOUS ACCESS FUNCTIONS (Cache Current Values Pattern) ===

/// Get current cursor position synchronously (replacement for bridge function)
pub fn current_cursor_position() -> TimeNs {
    // Use cached value pattern - the Timeline Actor caches current values
    *static_cache_cursor().get_or_init(|| std::sync::Mutex::new(TimeNs::ZERO)).lock().unwrap()
}

/// Get current cursor position in seconds synchronously
pub fn current_cursor_position_seconds() -> f64 {
    current_cursor_position().display_seconds()
}

/// Get current viewport synchronously (replacement for bridge function)
pub fn current_viewport() -> Viewport {
    // ✅ CRITICAL FIX: Use larger default range to prevent microsecond zoom display
    // 250s range matches the actual waveform data and ensures proper zoom ratio calculation
    *static_cache_viewport().get_or_init(|| std::sync::Mutex::new(
        Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(250.0))
    )).lock().unwrap()
}

/// Get current ns_per_pixel synchronously (replacement for bridge function)
pub fn current_ns_per_pixel() -> NsPerPixel {
    *static_cache_ns_per_pixel().get_or_init(|| std::sync::Mutex::new(NsPerPixel::MEDIUM_ZOOM)).lock().unwrap()
}

/// Get current timeline coordinates synchronously (replacement for bridge function)
pub fn current_coordinates() -> TimelineCoordinates {
    // ✅ COORDINATE TRANSFORMATION FIX: Use actual timeline state instead of hardcoded values
    let cursor_ns = current_cursor_position();
    let viewport = current_viewport();
    let ns_per_pixel = current_ns_per_pixel();
    let canvas_width = current_canvas_width() as u32;
    
    TimelineCoordinates::new(
        cursor_ns,
        viewport.start,        // Use actual viewport start, not hardcoded ZERO
        ns_per_pixel,         // Use actual zoom level, not hardcoded MEDIUM_ZOOM
        canvas_width          // Use actual canvas width, not hardcoded 800
    )
}

/// Get current canvas width synchronously (replacement for bridge function)
pub fn current_canvas_width() -> f32 {
    // Use cached value pattern - cache is updated by initialize_value_caching() from real signals
    *static_cache_width().get_or_init(|| std::sync::Mutex::new(800.0)).lock().unwrap()
}

/// Get current canvas height synchronously (replacement for bridge function)
pub fn current_canvas_height() -> f32 {
    // Use cached value pattern - cache is updated by initialize_value_caching() from real signals
    *static_cache_height().get_or_init(|| std::sync::Mutex::new(400.0)).lock().unwrap()
}

/// Set cursor position through domain event (replacement for bridge function)
pub fn set_cursor_position(time_ns: TimeNs) {
    cursor_moved_relay().send(time_ns);
}

/// Set cursor position from f64 seconds (convenience function)
pub fn set_cursor_position_seconds(seconds: f64) {
    let time_ns = TimeNs::from_external_seconds(seconds);
    cursor_moved_relay().send(time_ns);
}

/// Set cursor position if changed (replacement for bridge function)
pub fn set_cursor_position_if_changed(time_ns: TimeNs) {
    let current_ns = current_cursor_position();
    
    // Only emit event if value actually changed
    if current_ns != time_ns {
        cursor_moved_relay().send(time_ns);
    }
}

/// Set viewport if changed (replacement for bridge function)
pub fn set_viewport_if_changed(new_viewport: Viewport) {
    let current_viewport = current_viewport();
    
    // DEBUG: Track all viewport change attempts to catch the 1s corruption
    zoon::println!("🔍 SET_VIEWPORT_IF_CHANGED: Attempt to change viewport");
    zoon::println!("   Current: {:.6}s to {:.6}s (span: {:.6}s)", 
        current_viewport.start.display_seconds(), 
        current_viewport.end.display_seconds(),
        current_viewport.duration().display_seconds());
    zoon::println!("   New:     {:.6}s to {:.6}s (span: {:.6}s)", 
        new_viewport.start.display_seconds(), 
        new_viewport.end.display_seconds(),
        new_viewport.duration().display_seconds());
    
    if new_viewport.duration().display_seconds() <= 1.1 {
        zoon::println!("🚨 VIEWPORT CORRUPTION BLOCKED: Attempted to set {}s viewport - violates NO FALLBACKS rule!", 
            new_viewport.duration().display_seconds());
        zoon::println!("   ✅ PRESERVING existing viewport: {:.6}s to {:.6}s", 
            current_viewport.start.display_seconds(), current_viewport.end.display_seconds());
        return; // Block the corruption, preserve current viewport
    }
    
    // Only emit event if value actually changed
    if current_viewport != new_viewport {
        zoon::println!("   Status: CHANGING viewport from {:.6}s-{:.6}s to {:.6}s-{:.6}s", 
            current_viewport.start.display_seconds(), current_viewport.end.display_seconds(),
            new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
        viewport_changed_relay().send(viewport_tuple);
    } else {
        zoon::println!("   Status: No change needed");
    }
}

/// Set ns_per_pixel if changed (replacement for bridge function)
pub fn set_ns_per_pixel_if_changed(new_ns_per_pixel: NsPerPixel) {
    let current_ns_per_pixel = current_ns_per_pixel();
    
    // Only emit event if value actually changed
    if current_ns_per_pixel != new_ns_per_pixel {
        zoon::println!("🔄 SET_NS_PER_PIXEL: Updating from {} to {}", current_ns_per_pixel, new_ns_per_pixel);
        
        // CRITICAL FIX: Use proper Actor+Relay pattern - send ns_per_pixel update event
        let domain = waveform_timeline_domain();
        domain.ns_per_pixel_changed_relay.send(new_ns_per_pixel);
        zoon::println!("✅ DOMAIN UPDATE: Sent ns_per_pixel update event: {}", new_ns_per_pixel);
        
        // Update cached value for synchronous access  
        let cached_ns_per_pixel = static_cache_ns_per_pixel();
        *cached_ns_per_pixel.get_or_init(|| std::sync::Mutex::new(NsPerPixel::MEDIUM_ZOOM)).lock().unwrap() = new_ns_per_pixel;
        
        let zoom_center = current_cursor_position();
        zoom_in_started_relay().send(zoom_center);
    }
}

/// Set canvas dimensions through domain event (replacement for bridge function)
pub fn set_canvas_dimensions(width: f32, height: f32) {
    canvas_resized_relay().send((width, height));
}

/// Initialize value caching for synchronous access (Cache Current Values pattern)
pub fn initialize_value_caching() {
    use zoon::{Task, SignalExt};
    
    // Cache cursor position as it flows through signals
    Task::start(async move {
        cursor_position_seconds_signal()
            .for_each(move |cursor_position| {
                // Cache the current value for synchronous access
                let cursor_ns = TimeNs::from_external_seconds(cursor_position);
                let cached_cursor = static_cache_cursor();
                *cached_cursor.get_or_init(|| std::sync::Mutex::new(TimeNs::ZERO)).lock().unwrap() = cursor_ns;
                
                async {}
            })
            .await;
    });
    
    // Cache viewport as it flows through signals
    Task::start(async move {
        viewport_signal()
            .for_each(move |viewport| {
                // Cache the current value for synchronous access
                let cached_viewport = static_cache_viewport();
                *cached_viewport.get_or_init(|| std::sync::Mutex::new(Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(250.0)))).lock().unwrap() = viewport;
                
                async {}
            })
            .await;
    });
    
    // Cache ns_per_pixel as it flows through signals
    Task::start(async move {
        ns_per_pixel_signal()
            .for_each(move |ns_per_pixel| {
                // Cache the current value for synchronous access
                let cached_ns_per_pixel = static_cache_ns_per_pixel();
                *cached_ns_per_pixel.get_or_init(|| std::sync::Mutex::new(NsPerPixel::MEDIUM_ZOOM)).lock().unwrap() = ns_per_pixel;
                
                async {}
            })
            .await;
    });
    
    // Cache coordinates as they flow through signals
    Task::start(async move {
        coordinates_signal()
            .for_each(move |coordinates| {
                // Cache the current value for synchronous access
                let cached_coordinates = static_cache_coordinates();
                *cached_coordinates.get_or_init(|| std::sync::Mutex::new(TimelineCoordinates::new(TimeNs::ZERO, TimeNs::ZERO, NsPerPixel::MEDIUM_ZOOM, 800))).lock().unwrap() = coordinates;
                
                async {}
            })
            .await;
    });
    
    // Cache canvas dimensions as they flow through signals
    Task::start(async move {
        // Connect directly to the real domain Actor signal from global_domains
        let timeline = crate::actors::global_domains::waveform_timeline_domain();
        timeline.canvas_width.signal()
            .for_each(move |width| {
                // Cache the current value for synchronous access
                let cached_width = static_cache_width();
                *cached_width.get_or_init(|| std::sync::Mutex::new(800.0)).lock().unwrap() = width;
                
                async {}
            })
            .await;
    });
    
    Task::start(async move {
        // Connect directly to the real domain Actor signal from global_domains
        let timeline = crate::actors::global_domains::waveform_timeline_domain();
        timeline.canvas_height.signal()
            .for_each(move |height| {
                // Cache the current value for synchronous access
                let cached_height = static_cache_height();
                *cached_height.get_or_init(|| std::sync::Mutex::new(400.0)).lock().unwrap() = height;
                
                async {}
            })
            .await;
    });
}

// Helper functions to get the static cache instances
fn static_cache_cursor() -> &'static std::sync::OnceLock<std::sync::Mutex<TimeNs>> {
    static CACHED_CURSOR: std::sync::OnceLock<std::sync::Mutex<TimeNs>> = std::sync::OnceLock::new();
    &CACHED_CURSOR
}

fn static_cache_viewport() -> &'static std::sync::OnceLock<std::sync::Mutex<Viewport>> {
    static CACHED_VIEWPORT: std::sync::OnceLock<std::sync::Mutex<Viewport>> = std::sync::OnceLock::new();
    &CACHED_VIEWPORT
}

fn static_cache_ns_per_pixel() -> &'static std::sync::OnceLock<std::sync::Mutex<NsPerPixel>> {
    static CACHED_NS_PER_PIXEL: std::sync::OnceLock<std::sync::Mutex<NsPerPixel>> = std::sync::OnceLock::new();
    &CACHED_NS_PER_PIXEL
}

fn static_cache_coordinates() -> &'static std::sync::OnceLock<std::sync::Mutex<TimelineCoordinates>> {
    static CACHED_COORDINATES: std::sync::OnceLock<std::sync::Mutex<TimelineCoordinates>> = std::sync::OnceLock::new();
    &CACHED_COORDINATES
}

fn static_cache_width() -> &'static std::sync::OnceLock<std::sync::Mutex<f32>> {
    static CACHED_WIDTH: std::sync::OnceLock<std::sync::Mutex<f32>> = std::sync::OnceLock::new();
    &CACHED_WIDTH
}

fn static_cache_height() -> &'static std::sync::OnceLock<std::sync::Mutex<f32>> {
    static CACHED_HEIGHT: std::sync::OnceLock<std::sync::Mutex<f32>> = std::sync::OnceLock::new();
    &CACHED_HEIGHT
}

/// Calculate adaptive step size for cursor movement (Q/E keys)
/// Returns step size in nanoseconds based on visible time range
fn calculate_adaptive_cursor_step() -> u64 {
    let viewport = current_viewport();
    let visible_range_ns = viewport.end.nanos() - viewport.start.nanos();
    
    // Step size should be approximately 1% of visible range, with reasonable bounds
    let base_step = visible_range_ns / 100; // 1% of visible range
    
    // Apply bounds to keep step size reasonable
    let min_step = 1_000_000; // 1ms minimum
    let max_step = 1_000_000_000; // 1s maximum
    
    base_step.clamp(min_step, max_step)
}