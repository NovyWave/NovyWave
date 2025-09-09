//! Zoom control system for timeline viewport scaling and navigation
//!
//! Manages all zoom-related functionality including zoom in/out, reset zoom, viewport calculations,
//! and ns_per_pixel resolution management for timeline navigation.

use crate::dataflow::{Actor, Relay, relay};
use zoon::*;
use futures::{StreamExt, select};

// Import time domain
use super::time_domain::{TimeNs, NsPerPixel, Viewport};
use super::maximum_timeline_range::MaximumTimelineRange;

// Canvas dimension constants
const DEFAULT_CANVAS_WIDTH: f32 = 800.0;

/// Zoom controller with Actor+Relay architecture
#[derive(Clone, Debug)]
pub struct ZoomController {
    /// Timeline resolution (nanoseconds per pixel)
    pub ns_per_pixel: Actor<NsPerPixel>,
    
    /// Zoom center position (in nanoseconds)
    pub zoom_center: Actor<TimeNs>,
    
    /// User zoom interaction relays
    pub zoom_in_started_relay: Relay<TimeNs>,
    pub zoom_out_started_relay: Relay<TimeNs>,
    pub zoom_in_pressed_relay: Relay<()>,
    pub zoom_out_pressed_relay: Relay<()>,
    pub reset_zoom_pressed_relay: Relay<()>,
    pub reset_zoom_center_pressed_relay: Relay<()>,
    
    /// Zoom center control relays
    pub zoom_center_follow_mouse_relay: Relay<TimeNs>,
    
    /// System zoom relays
    pub ns_per_pixel_changed_relay: Relay<NsPerPixel>,
    pub viewport_changed_relay: Relay<(f64, f64)>,
    pub fit_all_clicked_relay: Relay<()>,
}

impl ZoomController {
    /// Access to zoom_center Actor for external usage
    pub fn zoom_center(&self) -> &Actor<TimeNs> {
        &self.zoom_center
    }
    
    pub async fn new(
        maximum_timeline_range: MaximumTimelineRange,
        viewport: Actor<Viewport>,
        cursor_center_at_viewport_relay: Relay<()>,
        zoom_center_reset_to_zero_relay: Relay<()>,
        canvas_resized_relay: Relay<(f32, f32)>,
    ) -> Self {
        // Create zoom interaction relays
        let (zoom_in_started_relay, zoom_in_started_stream) = relay::<TimeNs>();
        let (zoom_out_started_relay, zoom_out_started_stream) = relay::<TimeNs>();
        let (zoom_in_pressed_relay, zoom_in_pressed_stream) = relay::<()>();
        let (zoom_out_pressed_relay, zoom_out_pressed_stream) = relay::<()>();
        let (reset_zoom_pressed_relay, reset_zoom_pressed_stream) = relay::<()>();
        let (reset_zoom_center_pressed_relay, reset_zoom_center_pressed_stream) = relay::<()>();
        
        // Create system zoom relays
        let (ns_per_pixel_changed_relay, ns_per_pixel_changed_stream) = relay::<NsPerPixel>();
        let (viewport_changed_relay, viewport_changed_stream) = relay::<(f64, f64)>();
        let (fit_all_clicked_relay, fit_all_clicked_stream) = relay::<()>();
        
        // Create zoom center control relays
        let (zoom_center_follow_mouse_relay, zoom_center_follow_mouse_stream) = relay::<TimeNs>();
        
        // Create ns_per_pixel actor with zoom event handling
        let ns_per_pixel = Actor::new(NsPerPixel::default(), {
            let canvas_resized_relay_clone = canvas_resized_relay.clone();
            let viewport_for_ns_per_pixel = viewport.clone();
            let cursor_center_at_viewport_relay_clone = cursor_center_at_viewport_relay.clone();
            let zoom_center_reset_to_zero_relay_clone = zoom_center_reset_to_zero_relay.clone();
            let viewport_changed_relay_clone = viewport_changed_relay.clone();
            let fit_all_clicked_relay_clone = fit_all_clicked_relay.clone();
            async move |ns_per_pixel_handle| {
                let mut zoom_in_started = zoom_in_started_stream;
                let mut zoom_out_started = zoom_out_started_stream;
                let mut zoom_in_pressed = zoom_in_pressed_stream;
                let mut zoom_out_pressed = zoom_out_pressed_stream;
                
                // Cache current values pattern for timeline range
                let mut cached_timeline_range: Option<(f64, f64)> = None;
                let mut timeline_range_stream = maximum_timeline_range.range.signal().to_stream().fuse();
                let mut reset_zoom_pressed = reset_zoom_pressed_stream;
                let mut reset_zoom_center_pressed = reset_zoom_center_pressed_stream;
                let mut ns_per_pixel_changed = ns_per_pixel_changed_stream;
                let mut canvas_resized = canvas_resized_relay_clone.subscribe();
            
            loop {
                select! {
                    // Update cached timeline range
                    range = timeline_range_stream.next() => {
                        if let Some(new_range) = range {
                            cached_timeline_range = new_range;
                        }
                    }
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
                                // Zoom in operation starting
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                let new_ns_per_pixel = current_ns_per_pixel.zoom_in_smooth(0.3);
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                                
                                let _current_viewport = viewport_for_ns_per_pixel.signal().to_stream().next().await.unwrap_or_else(|| {
                                    super::time_domain::Viewport::default() // Default viewport when no data available
                                });
                                
                                // Get actual canvas width from current viewport dimensions
                                let canvas_width = DEFAULT_CANVAS_WIDTH; // Will be replaced by actual width
                                if canvas_width <= 0.0 {
                                    continue; // Timeline not initialized yet, skip this frame
                                }
                                
                                let center_time = TimeNs::ZERO; // Will be replaced by actual zoom center
                                
                                // Calculate new viewport range based on new zoom level and ACTUAL canvas width
                                let half_range_ns = (new_ns_per_pixel.nanos() * canvas_width as u64) / 2;
                                let new_start = TimeNs::from_nanos(center_time.nanos().saturating_sub(half_range_ns));
                                let new_end = TimeNs::from_nanos(center_time.nanos() + half_range_ns);
                                let new_viewport = Viewport::new(new_start, new_end);
                                
                                // Use the viewport changed relay to update viewport
                                let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
                                viewport_changed_relay_clone.send(viewport_tuple);
                            }
                            None => break,
                        }
                    }
                    event = zoom_out_pressed.next() => {
                        match event {
                            Some(()) => {
                                // Zoom out operation starting
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                let new_ns_per_pixel = current_ns_per_pixel.zoom_out_smooth(0.3);
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                                
                                let _current_viewport = viewport_for_ns_per_pixel.signal().to_stream().next().await.unwrap_or_else(|| {
                                    super::time_domain::Viewport::default() // Default viewport when no data available
                                });
                                // Get actual canvas width from current viewport dimensions
                                let canvas_width = DEFAULT_CANVAS_WIDTH; // Will be replaced by actual width
                                if canvas_width <= 0.0 {
                                    continue; // Timeline not initialized yet, skip this frame
                                }
                                
                                let center_time = TimeNs::ZERO; // Will be replaced by actual zoom center
                                
                                // Calculate new viewport range based on new zoom level
                                let half_range_ns = (new_ns_per_pixel.nanos() * canvas_width as u64) / 2;
                                let new_start = TimeNs::from_nanos(center_time.nanos().saturating_sub(half_range_ns));
                                let new_end = TimeNs::from_nanos(center_time.nanos() + half_range_ns);
                                let new_viewport = Viewport::new(new_start, new_end);
                                
                                // Use the viewport changed relay to update viewport
                                let viewport_tuple = (new_viewport.start.display_seconds(), new_viewport.end.display_seconds());
                                viewport_changed_relay_clone.send(viewport_tuple);
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom_pressed.next() => {
                        match event {
                            Some(()) => {
                                let current_ns_per_pixel = ns_per_pixel_handle.get();
                                if current_ns_per_pixel.nanos() == 0 {
                                    continue;
                                }
                                
                                let canvas_width = DEFAULT_CANVAS_WIDTH as u32;
                                
                                if let Some((min_time, max_time)) = cached_timeline_range {
                                    let time_range_ns = ((max_time - min_time) * super::time_domain::NS_PER_SECOND) as u64;
                                    let contextual_zoom = NsPerPixel((time_range_ns + canvas_width as u64 / 2) / canvas_width as u64);
                                    
                                    // Calculate zoom constraints
                                    let min_zoom = NsPerPixel(super::time_domain::MIN_ZOOM_NS_PER_PIXEL); // 1μs/px (very zoomed in)
                                    let max_zoom = NsPerPixel(super::time_domain::MAX_ZOOM_NS_PER_PIXEL); // 10s/px (very zoomed out)
                                    
                                    let raw_clamp = contextual_zoom.nanos().clamp(min_zoom.nanos(), max_zoom.nanos());
                                    let clamped_zoom = NsPerPixel(raw_clamp);
                                    
                                    ns_per_pixel_handle.set(clamped_zoom);
                                    
                                    fit_all_clicked_relay_clone.send(());
                                    cursor_center_at_viewport_relay_clone.send(());
                                    zoom_center_reset_to_zero_relay_clone.send(());
                                } else {
                                    // No timeline range available - skip reset operation
                                    continue;
                                }
                            }
                            None => break,
                        }
                    }
                    event = reset_zoom_center_pressed.next() => {
                        match event {
                            Some(()) => {
                                zoom_center_reset_to_zero_relay_clone.send(());
                            }
                            None => break,
                        }
                    }
                    event = ns_per_pixel_changed.next() => {
                        match event {
                            Some(new_ns_per_pixel) => {
                                ns_per_pixel_handle.set_neq(new_ns_per_pixel);
                            }
                            None => break,
                        }
                    }
                    event = canvas_resized.next() => {
                        match event {
                            Some((new_width, _height)) => {
                                if let Some(current_viewport) = viewport_for_ns_per_pixel.signal().to_stream().next().await {
                                    let viewport_range_ns = current_viewport.end.nanos() - current_viewport.start.nanos();
                                    let corrected_ns_per_pixel = NsPerPixel((viewport_range_ns + new_width as u64 / 2) / new_width as u64);
                                    
                                    let current_ns_per_pixel = ns_per_pixel_handle.get();
                                    if corrected_ns_per_pixel != current_ns_per_pixel {
                                        ns_per_pixel_handle.set_neq(corrected_ns_per_pixel);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        }});
        
        // Create zoom center actor with reset and mouse follow functionality
        let zoom_center = Actor::new(TimeNs::ZERO, async move |zoom_center_handle| {
            let mut zoom_center_reset_to_zero = zoom_center_reset_to_zero_relay.subscribe();
            let mut zoom_center_follow_mouse = zoom_center_follow_mouse_stream;
            
            loop {
                select! {
                    event = zoom_center_reset_to_zero.next() => {
                        match event {
                            Some(()) => {
                                zoom_center_handle.set(TimeNs::ZERO);
                            },
                            None => break,
                        }
                    }
                    event = zoom_center_follow_mouse.next() => {
                        match event {
                            Some(time_ns) => {
                                zoom_center_handle.set(time_ns);
                            },
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });
        
        Self {
            ns_per_pixel,
            zoom_center,
            zoom_in_started_relay,
            zoom_out_started_relay,
            zoom_in_pressed_relay,
            zoom_out_pressed_relay,
            reset_zoom_pressed_relay,
            reset_zoom_center_pressed_relay,
            zoom_center_follow_mouse_relay,
            ns_per_pixel_changed_relay,
            viewport_changed_relay,
            fit_all_clicked_relay,
        }
    }
}