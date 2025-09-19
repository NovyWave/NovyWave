//! Cursor animation control system for smooth timeline cursor movement
//!
//! Manages all cursor animation state including position, target, active state, and direction
//! for coordinated smooth cursor movement animations in the timeline.

use crate::dataflow::{Actor, Relay, relay};
use futures::{StreamExt, select};
use zoon::*;

// Import time domain
use super::time_domain::TimeNs;

/// Cursor animation controller with Actor+Relay architecture
#[derive(Clone, Debug)]
pub struct CursorAnimationController {
    /// Cursor animation state actors
    pub cursor_animation_position: Actor<f64>, // Current position in seconds (high precision)
    pub cursor_animation_target: Actor<f64>, // Target position in seconds
    pub cursor_animation_active: Actor<bool>, // Animation active flag
    pub cursor_animation_direction: Actor<i8>, // -1 for left, 1 for right, 0 for stopped

    /// Animation state relays
    pub cursor_moving_left_started_relay: Relay<()>,
    pub cursor_moving_left_stopped_relay: Relay<()>,
    pub cursor_moving_right_started_relay: Relay<()>,
    pub cursor_moving_right_stopped_relay: Relay<()>,
}

impl CursorAnimationController {
    pub async fn new(
        cursor_position: Actor<TimeNs>,
        cursor_moving_left: Actor<bool>,
        cursor_moving_right: Actor<bool>,
    ) -> Self {
        // Create animation state relays
        let (cursor_moving_left_started_relay, cursor_moving_left_started_stream) = relay::<()>();
        let (cursor_moving_left_stopped_relay, cursor_moving_left_stopped_stream) = relay::<()>();
        let (cursor_moving_right_started_relay, cursor_moving_right_started_stream) = relay::<()>();
        let (cursor_moving_right_stopped_relay, cursor_moving_right_stopped_stream) = relay::<()>();

        // Cursor animation actors for smooth cursor movement
        let cursor_animation_position = Actor::new(0.0f64, {
            let cursor_position_for_animation = cursor_position.clone();
            async move |position_handle| {
                // Track actual cursor position changes for animation
                let mut cursor_stream = cursor_position_for_animation.signal().to_stream().fuse();

                loop {
                    select! {
                        cursor_update = cursor_stream.next() => {
                            match cursor_update {
                                Some(new_cursor) => {
                                    // Update animation position to match actual cursor
                                    position_handle.set(new_cursor.display_seconds());
                                }
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let cursor_animation_target = Actor::new(0.0f64, {
            let cursor_moving_left_for_target = cursor_moving_left.clone();
            let cursor_moving_right_for_target = cursor_moving_right.clone();
            let cursor_position_for_target = cursor_position.clone();
            async move |target_handle| {
                let mut cursor_stream = cursor_position_for_target.signal().to_stream().fuse();
                let mut moving_left_stream =
                    cursor_moving_left_for_target.signal().to_stream().fuse();
                let mut moving_right_stream =
                    cursor_moving_right_for_target.signal().to_stream().fuse();

                loop {
                    select! {
                        cursor_update = cursor_stream.next() => {
                            match cursor_update {
                                Some(new_cursor) => {
                                    // Set animation target to current cursor position
                                    target_handle.set(new_cursor.display_seconds());
                                }
                                None => break,
                            }
                        }
                        moving_left = moving_left_stream.next() => {
                            match moving_left {
                                Some(is_moving) if is_moving => {
                                    // Set target to left direction when moving left
                                    let current = target_handle.get();
                                    target_handle.set(current - 1.0); // Move 1 second left
                                }
                                _ => {}
                            }
                        }
                        moving_right = moving_right_stream.next() => {
                            match moving_right {
                                Some(is_moving) if is_moving => {
                                    // Set target to right direction when moving right
                                    let current = target_handle.get();
                                    target_handle.set(current + 1.0); // Move 1 second right
                                }
                                _ => {}
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let cursor_animation_active = Actor::new(false, {
            let cursor_moving_left_for_active = cursor_moving_left.clone();
            let cursor_moving_right_for_active = cursor_moving_right.clone();
            async move |active_handle| {
                let mut moving_left_stream =
                    cursor_moving_left_for_active.signal().to_stream().fuse();
                let mut moving_right_stream =
                    cursor_moving_right_for_active.signal().to_stream().fuse();

                loop {
                    select! {
                        moving_left = moving_left_stream.next() => {
                            match moving_left {
                                Some(is_moving) => active_handle.set(is_moving),
                                None => break,
                            }
                        }
                        moving_right = moving_right_stream.next() => {
                            match moving_right {
                                Some(is_moving) => active_handle.set(is_moving),
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let cursor_animation_direction = Actor::new(0i8, {
            let cursor_moving_left_for_direction = cursor_moving_left.clone();
            let cursor_moving_right_for_direction = cursor_moving_right.clone();
            async move |direction_handle| {
                let mut moving_left_stream =
                    cursor_moving_left_for_direction.signal().to_stream().fuse();
                let mut moving_right_stream = cursor_moving_right_for_direction
                    .signal()
                    .to_stream()
                    .fuse();

                loop {
                    select! {
                        moving_left = moving_left_stream.next() => {
                            match moving_left {
                                Some(true) => direction_handle.set(-1i8), // Moving left
                                Some(false) => direction_handle.set(0i8), // Stopped
                                None => break,
                            }
                        }
                        moving_right = moving_right_stream.next() => {
                            match moving_right {
                                Some(true) => direction_handle.set(1i8), // Moving right
                                Some(false) => direction_handle.set(0i8), // Stopped
                                None => break,
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        Self {
            cursor_animation_position,
            cursor_animation_target,
            cursor_animation_active,
            cursor_animation_direction,
            cursor_moving_left_started_relay,
            cursor_moving_left_stopped_relay,
            cursor_moving_right_started_relay,
            cursor_moving_right_stopped_relay,
        }
    }
}
