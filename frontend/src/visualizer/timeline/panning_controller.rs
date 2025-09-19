//! Panning control system for smooth timeline viewport movement
//!
//! Manages left and right panning state for coordinated viewport panning animations
//! in the timeline, consolidating duplicate panning logic.

use crate::dataflow::{Actor, Relay, relay};
use futures::{StreamExt, select};
use zoon::*;

/// Panning controller with Actor+Relay architecture
#[derive(Clone, Debug)]
pub struct PanningController {
    /// Smooth pan control flags  
    pub panning_left: Actor<bool>,
    pub panning_right: Actor<bool>,

    /// User panning relays
    pub pan_left_started_relay: Relay<()>,
    pub pan_right_started_relay: Relay<()>,

    /// Animation state relays  
    pub panning_left_started_relay: Relay<()>,
    pub panning_left_stopped_relay: Relay<()>,
    pub panning_right_started_relay: Relay<()>,
    pub panning_right_stopped_relay: Relay<()>,
}

impl PanningController {
    pub async fn new() -> Self {
        // Create user panning relays
        let (pan_left_started_relay, pan_left_started_stream) = relay::<()>();
        let (pan_right_started_relay, pan_right_started_stream) = relay::<()>();

        // Create animation state relays
        let (panning_left_started_relay, panning_left_started_stream) = relay::<()>();
        let (panning_left_stopped_relay, panning_left_stopped_stream) = relay::<()>();
        let (panning_right_started_relay, panning_right_started_stream) = relay::<()>();
        let (panning_right_stopped_relay, panning_right_stopped_stream) = relay::<()>();

        let panning_left = Actor::new(false, async move |panning_handle| {
            let mut pan_left_started = pan_left_started_stream;
            let mut panning_started = panning_left_started_stream;
            let mut panning_stopped = panning_left_stopped_stream;

            loop {
                select! {
                    event = pan_left_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // Panning will stop on key release or timeout
                            }
                            None => break,
                        }
                    }
                    event = panning_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                            }
                            None => break,
                        }
                    }
                    event = panning_stopped.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(false);
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
            let mut panning_started = panning_right_started_stream;
            let mut panning_stopped = panning_right_stopped_stream;

            loop {
                select! {
                    event = pan_right_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                                // Panning will stop on key release or timeout
                            }
                            None => break,
                        }
                    }
                    event = panning_started.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(true);
                            }
                            None => break,
                        }
                    }
                    event = panning_stopped.next() => {
                        match event {
                            Some(()) => {
                                panning_handle.set(false);
                            }
                            None => break,
                        }
                    }
                    complete => break,
                }
            }
        });

        Self {
            panning_left,
            panning_right,
            pan_left_started_relay,
            pan_right_started_relay,
            panning_left_started_relay,
            panning_left_stopped_relay,
            panning_right_started_relay,
            panning_right_stopped_relay,
        }
    }
}
