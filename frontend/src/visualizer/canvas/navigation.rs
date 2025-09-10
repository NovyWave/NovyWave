use crate::dataflow::*;
use zoon::*;
use futures::{select, stream::StreamExt};
use shared::SignalTransition;

#[derive(Clone, Debug, PartialEq)]
pub enum NavigationType {
    PreviousTransition,
    NextTransition,
}

#[derive(Clone, Debug)]
pub struct NavigationResult {
    pub target_time_seconds: f64,
    pub navigation_type: NavigationType,
    pub wrapped: bool,
}

#[derive(Clone)]
pub struct NavigationController {
    pub debounce_state: Actor<NavigationDebounceState>,
    pub previous_transition_requested_relay: Relay,
    pub next_transition_requested_relay: Relay,
    pub navigation_completed_relay: Relay<NavigationResult>,
}

#[derive(Clone, Debug)]
struct NavigationDebounceState {
    last_navigation_time_ns: u64,
    debounce_period_ms: u64,
}

impl Default for NavigationDebounceState {
    fn default() -> Self {
        Self {
            last_navigation_time_ns: 0,
            debounce_period_ms: 100,
        }
    }
}

impl NavigationController {
    pub async fn new(
        timeline_cache: crate::visualizer::timeline::timeline_cache::TimelineCacheController,
        cursor_position: Actor<crate::visualizer::timeline::time_domain::TimeNs>,
        selected_variables: crate::selected_variables::SelectedVariables,
    ) -> Self {
        let (previous_transition_requested_relay, mut previous_transition_stream) = relay();
        let (next_transition_requested_relay, mut next_transition_stream) = relay();
        let (navigation_completed_relay, _navigation_completed_stream) = relay();

        let debounce_state = Actor::new(NavigationDebounceState::default(), {
            let navigation_completed_relay_for_actor = navigation_completed_relay.clone();
            async move |state| {
                let mut cached_cursor_position: Option<f64> = None;
                let mut cached_transitions: Vec<f64> = Vec::new();

                let mut cursor_stream = cursor_position.signal().to_stream().fuse();
                let mut variables_stream = selected_variables.variables_vec_actor.signal().to_stream().fuse();

            loop {
                select! {
                    cursor_update = cursor_stream.next() => {
                        if let Some(cursor_ns) = cursor_update {
                            cached_cursor_position = Some(cursor_ns.display_seconds());
                        }
                    }
                    variables_update = variables_stream.next() => {
                        if let Some(variables) = variables_update {
                            let cache = timeline_cache.cache.signal().to_stream().next().await;
                            if let Some(cache_data) = cache {
                                cached_transitions = Self::extract_transitions_from_cache(&cache_data, &variables);
                            }
                        }
                    }
                    previous_request = previous_transition_stream.next() => {
                        if let Some(()) = previous_request {
                            if let Some(result) = Self::process_navigation_with_state_handle(
                                &mut state.lock_mut(),
                                NavigationType::PreviousTransition,
                                cached_cursor_position,
                                &cached_transitions,
                            ) {
                                navigation_completed_relay_for_actor.send(result);
                            }
                        }
                    }
                    next_request = next_transition_stream.next() => {
                        if let Some(()) = next_request {
                            if let Some(result) = Self::process_navigation_with_state_handle(
                                &mut state.lock_mut(),
                                NavigationType::NextTransition,
                                cached_cursor_position,
                                &cached_transitions,
                            ) {
                                navigation_completed_relay_for_actor.send(result);
                            }
                        }
                    }
                }
            }
        }
        });

        Self {
            debounce_state,
            previous_transition_requested_relay,
            next_transition_requested_relay,
            navigation_completed_relay,
        }
    }

    fn extract_transitions_from_cache(
        cache: &crate::visualizer::timeline::timeline_cache::TimelineCache,
        variables: &[shared::SelectedVariable],
    ) -> Vec<f64> {
        let mut all_transitions = Vec::new();

        for variable in variables {
            let signal_id = match (variable.file_path(), variable.scope_path(), variable.variable_name()) {
                (Some(file_path), Some(scope_path), Some(variable_name)) => {
                    format!("{}|{}|{}", file_path, scope_path, variable_name)
                },
                _ => continue, // Skip variables with incomplete path information
            };

            if let Some(transitions) = cache.raw_transitions.get(&signal_id) {
                for transition in transitions {
                    all_transitions.push(transition.time_ns as f64 / 1_000_000_000.0);
                }
            }
        }

        all_transitions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all_transitions.dedup_by(|a, b| (*a - *b).abs() < 1e-15);
        all_transitions
    }


    fn find_previous_transition(current_cursor: f64, transitions: &[f64]) -> Option<(f64, bool)> {
        const F64_PRECISION_TOLERANCE: f64 = 1e-15;

        let mut previous_transition: Option<f64> = None;

        for &transition_time in transitions.iter() {
            if transition_time < current_cursor - F64_PRECISION_TOLERANCE {
                previous_transition = Some(transition_time);
            } else {
                break;
            }
        }

        if let Some(prev_time) = previous_transition {
            Some((prev_time, false))
        } else if !transitions.is_empty() {
            let last_transition = transitions[transitions.len() - 1];
            Some((last_transition, true))
        } else {
            None
        }
    }

    fn find_next_transition(current_cursor: f64, transitions: &[f64]) -> Option<(f64, bool)> {
        const F64_PRECISION_TOLERANCE: f64 = 1e-15;

        let next_transition = transitions
            .iter()
            .find(|&&transition_time| {
                transition_time > current_cursor + F64_PRECISION_TOLERANCE
            })
            .copied();

        if let Some(next_time) = next_transition {
            Some((next_time, false))
        } else if !transitions.is_empty() {
            let first_transition = transitions[0];
            Some((first_transition, true))
        } else {
            None
        }
    }

    // Helper method that works with state handle directly (for use inside Actor closures)
    fn process_navigation_with_state_handle(
        state_handle: &mut impl std::ops::DerefMut<Target = NavigationDebounceState>,
        navigation_type: NavigationType,
        current_cursor: Option<f64>,
        transitions: &[f64],
    ) -> Option<NavigationResult> {
        let current_time = (zoon::performance().now() * 1_000_000.0) as u64;

        let time_since_last = current_time - state_handle.last_navigation_time_ns;
        let debounce_threshold = state_handle.debounce_period_ms * 1_000_000;

        if time_since_last < debounce_threshold {
            return None;
        }

        state_handle.last_navigation_time_ns = current_time;

        let cursor_pos = current_cursor?;
        if transitions.is_empty() {
            return None;
        }

        let (target_time, wrapped) = match navigation_type {
            NavigationType::PreviousTransition => {
                Self::find_previous_transition(cursor_pos, transitions)
            }
            NavigationType::NextTransition => {
                Self::find_next_transition(cursor_pos, transitions)
            }
        }?;

        Some(NavigationResult {
            target_time_seconds: target_time,
            navigation_type,
            wrapped,
        })
    }

    pub fn request_previous_transition(&self) {
        self.previous_transition_requested_relay.send(());
    }

    pub fn request_next_transition(&self) {
        self.next_transition_requested_relay.send(());
    }

    pub fn navigation_completed_signal(&self) -> impl Stream<Item = NavigationResult> {
        self.navigation_completed_relay.subscribe()
    }
}