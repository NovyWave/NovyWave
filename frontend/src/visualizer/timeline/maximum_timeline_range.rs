//! MaximumTimelineRange standalone derived state actor
//!
//! Centralized computation of timeline range from multiple files to eliminate
//! scattered computation and provide single source of truth for timeline bounds.

use crate::dataflow::Actor;
use super::time_domain::TimeNs;
use shared::{FileState, WaveformFile};
use futures::{StreamExt, select};
use zoon::Signal;


/// Maximum Timeline Range actor - stores computed timeline range for efficient access
#[derive(Clone, Debug)]
pub struct MaximumTimelineRange {
    pub range: Actor<Option<(f64, f64)>>,
}

impl MaximumTimelineRange {
    pub async fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
    ) -> Self {
        let tracked_files_for_actor = tracked_files.clone();
        let selected_variables_for_actor = selected_variables.clone();
        
        let range = Actor::new(None, async move |state| {
            let mut files_change_stream = tracked_files_for_actor.files_vec_signal.signal().to_stream();
            
            // Set initial range
            let initial_range = Self::compute_maximum_range(
                &tracked_files_for_actor, 
                &selected_variables_for_actor
            );
            state.set(initial_range);
            
            // Update range when files change
            while let Some(_files) = files_change_stream.next().await {
                let range_result = Self::compute_maximum_range(
                    &tracked_files_for_actor, 
                    &selected_variables_for_actor
                );
                state.set(range_result);
            }
        });
        
        Self { range }
    }

    /// Compute maximum timeline range - extracted from zoon::Task logic
    fn compute_maximum_range(
        tracked_files: &crate::tracked_files::TrackedFiles,
        _selected_variables: &crate::selected_variables::SelectedVariables,
    ) -> Option<(f64, f64)> {
        // Inline get_maximum_timeline_range logic to avoid circular dependency
        let tracked_files_vec = tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<WaveformFile> = tracked_files_vec
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        if loaded_files.is_empty() {
            return None;
        }

        let mut min_time: f64 = f64::MAX;
        let mut max_time: f64 = f64::MIN;

        // Get min/max time from all loaded files
        for waveform_file in &loaded_files {
            if let Some(start_time_ns) = waveform_file.min_time_ns {
                let start_time_seconds = start_time_ns as f64 / 1_000_000_000.0;
                min_time = min_time.min(start_time_seconds);
            }
            if let Some(end_time_ns) = waveform_file.max_time_ns {
                let end_time_seconds = end_time_ns as f64 / 1_000_000_000.0;
                max_time = max_time.max(end_time_seconds);
            }
        }

        if min_time != f64::MAX && max_time != f64::MIN && min_time < max_time {
            Some((min_time, max_time))
        } else {
            None
        }
    }
    
}