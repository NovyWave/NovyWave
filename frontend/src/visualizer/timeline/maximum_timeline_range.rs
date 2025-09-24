//! MaximumTimelineRange standalone derived state actor
//!
//! Centralized computation of timeline range from multiple files to eliminate
//! scattered computation and provide single source of truth for timeline bounds.

use super::time_domain::TimeNs;
use crate::dataflow::Actor;
use futures::{StreamExt, select};
use shared::{FileState, SelectedVariable, TrackedFile};
use std::collections::HashSet;
use zoon::{SignalExt, SignalVecExt};

/// Maximum Timeline Range actor - stores computed timeline range for efficient access
#[derive(Clone, Debug)]
pub struct MaximumTimelineRange {
    pub range: Actor<Option<(TimeNs, TimeNs)>>,
}

impl MaximumTimelineRange {
    pub async fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
    ) -> Self {
        let tracked_files_clone = tracked_files.clone();
        let selected_variables_clone = selected_variables.clone();

        let range = Actor::new(None, async move |state| {
            let mut files_stream = tracked_files_clone
                .files
                .signal_vec()
                .to_signal_cloned()
                .to_stream()
                .fuse();
            let mut selection_stream = selected_variables_clone
                .variables_vec_actor
                .signal()
                .to_stream()
                .fuse();

            let mut latest_files: Vec<TrackedFile> = Vec::new();
            let mut latest_selection: Vec<SelectedVariable> = Vec::new();

            loop {
                select! {
                    files = files_stream.next() => {
                        match files {
                            Some(files) => {
                                latest_files = files;
                                let range = Self::compute_range(&latest_files, &latest_selection);
                                state.set(range);
                            }
                            None => break,
                        }
                    }
                    selection = selection_stream.next() => {
                        match selection {
                            Some(selection) => {
                                latest_selection = selection;
                                let range = Self::compute_range(&latest_files, &latest_selection);
                                state.set(range);
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Self { range }
    }

    /// Pure function to compute maximum range from file vector
    fn compute_range(
        tracked_files: &[TrackedFile],
        selected_variables: &[SelectedVariable],
    ) -> Option<(TimeNs, TimeNs)> {
        let file_filter: HashSet<String> = selected_variables
            .iter()
            .filter_map(|var| var.file_path())
            .collect();

        if file_filter.is_empty() {
            return None;
        }

        let mut min_time: Option<TimeNs> = None;
        let mut max_time: Option<TimeNs> = None;

        for tracked_file in tracked_files {
            if !file_filter.contains(&tracked_file.path) {
                continue;
            }

            if let FileState::Loaded(waveform_file) = &tracked_file.state {
                if let Some(start_ns) = waveform_file.min_time_ns {
                    let start = TimeNs::from_nanos(start_ns);
                    min_time = Some(match min_time {
                        Some(current) => current.min(start),
                        None => start,
                    });
                }
                if let Some(end_ns) = waveform_file.max_time_ns {
                    let end = TimeNs::from_nanos(end_ns);
                    max_time = Some(match max_time {
                        Some(current) => current.max(end),
                        None => end,
                    });
                }
            }
        }

        match (min_time, max_time) {
            (Some(start), Some(end)) if end > start => Some((start, end)),
            _ => None,
        }
    }
}
