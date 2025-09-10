use crate::dataflow::*;
use zoon::*;
use futures::{select, stream::StreamExt};

#[derive(Clone, Debug)]
pub struct AnimationController {
    pub animation_state: Actor<AnimationState>,
    
    pub pan_left_requested_relay: Relay,
    pub pan_right_requested_relay: Relay,
    pub pan_left_stopped_relay: Relay,
    pub pan_right_stopped_relay: Relay,
    
    pub cursor_left_requested_relay: Relay,
    pub cursor_right_requested_relay: Relay,
    pub cursor_left_stopped_relay: Relay,
    pub cursor_right_stopped_relay: Relay,
    
    pub animation_tick_relay: Relay<AnimationFrame>,
}

#[derive(Clone, Debug)]
struct AnimationState {
    pub panning_left: bool,
    pub panning_right: bool,
    pub cursor_moving_left: bool,
    pub cursor_moving_right: bool,
    pub frame_count: u32,
    pub zoom_level: Option<f64>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            panning_left: false,
            panning_right: false,
            cursor_moving_left: false,
            cursor_moving_right: false,
            frame_count: 0,
            zoom_level: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationFrame {
    pub frame_number: u32,
    pub delta_time_ms: f32,
    pub active_animations: Vec<AnimationType>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationType {
    PanLeft,
    PanRight,
    CursorLeft,
    CursorRight,
}

impl AnimationController {
    pub async fn new(
        waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    ) -> Self {
        let (pan_left_requested_relay, mut pan_left_requested_stream) = relay();
        let (pan_right_requested_relay, mut pan_right_requested_stream) = relay();
        let (pan_left_stopped_relay, mut pan_left_stopped_stream) = relay();
        let (pan_right_stopped_relay, mut pan_right_stopped_stream) = relay();
        
        let (cursor_left_requested_relay, mut cursor_left_requested_stream) = relay();
        let (cursor_right_requested_relay, mut cursor_right_requested_stream) = relay();
        let (cursor_left_stopped_relay, mut cursor_left_stopped_stream) = relay();
        let (cursor_right_stopped_relay, mut cursor_right_stopped_stream) = relay();
        
        let (animation_tick_relay, mut animation_tick_stream) = relay::<AnimationFrame>();
        
        let animation_state = Actor::new(AnimationState::default(), async move |state| {
            loop {
                select! {
                    result = pan_left_requested_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.panning_left = true;
                            drop(current_state);
                        }
                    }
                    result = pan_right_requested_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.panning_right = true;
                            drop(current_state);
                        }
                    }
                    result = pan_left_stopped_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.panning_left = false;
                            drop(current_state);
                        }
                    }
                    result = pan_right_stopped_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.panning_right = false;
                            drop(current_state);
                        }
                    }
                    result = cursor_left_requested_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.cursor_moving_left = true;
                            drop(current_state);
                        }
                    }
                    result = cursor_right_requested_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.cursor_moving_right = true;
                            drop(current_state);
                        }
                    }
                    result = cursor_left_stopped_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.cursor_moving_left = false;
                            drop(current_state);
                        }
                    }
                    result = cursor_right_stopped_stream.next() => {
                        if let Some(()) = result {
                            let mut current_state = state.lock_mut();
                            current_state.cursor_moving_right = false;
                            drop(current_state);
                        }
                    }
                    result = animation_tick_stream.next() => {
                        if let Some(frame) = result {
                            let mut current_state = state.lock_mut();
                            current_state.frame_count = frame.frame_number;
                            drop(current_state);
                        }
                    }
                }
            }
        });
        
        Self {
            animation_state,
            pan_left_requested_relay,
            pan_right_requested_relay,
            pan_left_stopped_relay,
            pan_right_stopped_relay,
            cursor_left_requested_relay,
            cursor_right_requested_relay,
            cursor_left_stopped_relay,
            cursor_right_stopped_relay,
            animation_tick_relay,
        }
    }
    
    
}

