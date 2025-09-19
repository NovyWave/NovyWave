//! MaximumTimelineRange standalone derived state actor
//!
//! Centralized computation of timeline range from multiple files to eliminate
//! scattered computation and provide single source of truth for timeline bounds.

use super::time_domain::TimeNs;
use crate::dataflow::Actor;
use futures::{StreamExt, select};
use shared::{FileState, TrackedFile, WaveformFile};
use zoon::{Signal, SignalExt, SignalVecExt};

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
        let range = Actor::new(None, async move |state| {
            let mut files_stream = tracked_files
                .files
                .signal_vec()
                .to_signal_cloned()
                .to_stream();

            // Wait for files and compute range reactively
            while let Some(files) = files_stream.next().await {
                let range = Self::compute_range_from_files(&files);
                state.set(range);
            }
        });

        Self { range }
    }

    /// Pure function to compute maximum range from file vector
    fn compute_range_from_files(tracked_files: &[TrackedFile]) -> Option<(f64, f64)> {
        let loaded_files: Vec<WaveformFile> = tracked_files
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
