//! MaximumTimelineRange standalone derived state actor
//!
//! Centralized computation of timeline range from multiple files to eliminate
//! scattered computation and provide single source of truth for timeline bounds.

use super::time_domain::TimePs;
use futures::{StreamExt, select};
#[cfg(debug_assertions)]
use shared::FileFormat;
use shared::{FileState, SelectedVariable, TrackedFile};
use std::collections::HashSet;
use std::sync::Arc;
use zoon::{Mutable, SignalExt, SignalVecExt, Task, TaskHandle};

/// Maximum Timeline Range actor - stores computed timeline range for efficient access
#[derive(Clone)]
pub struct MaximumTimelineRange {
    pub range: Mutable<Option<(TimePs, TimePs)>>,
    _range_task: Arc<TaskHandle>,
}

impl MaximumTimelineRange {
    pub fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
    ) -> Self {
        let range = Mutable::new(None);
        let range_clone = range.clone();

        let _range_task = Arc::new(Task::start_droppable(async move {
            let mut files_stream = tracked_files
                .files
                .signal_vec_cloned()
                .to_signal_cloned()
                .to_stream()
                .fuse();
            let mut selection_stream = selected_variables
                .variables_vec_actor
                .signal_cloned()
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
                                let computed_range = Self::compute_range(&latest_files, &latest_selection);
                                range_clone.set_neq(computed_range);
                            }
                            None => break,
                        }
                    }
                    selection = selection_stream.next() => {
                        match selection {
                            Some(selection) => {
                                latest_selection = selection;
                                let computed_range = Self::compute_range(&latest_files, &latest_selection);
                                range_clone.set_neq(computed_range);
                            }
                            None => break,
                        }
                    }
                }
            }
        }));

        Self { range, _range_task }
    }

    /// Pure function to compute maximum range from file vector
    fn compute_range(
        tracked_files: &[TrackedFile],
        selected_variables: &[SelectedVariable],
    ) -> Option<(TimePs, TimePs)> {
        let file_filter: HashSet<String> = selected_variables
            .iter()
            .filter_map(|var| var.file_path())
            .collect();

        if file_filter.is_empty() {
            return None;
        }

        let mut min_time: Option<TimePs> = None;
        let mut max_time: Option<TimePs> = None;

        for tracked_file in tracked_files {
            if !file_filter.contains(&tracked_file.path) {
                continue;
            }

            if let FileState::Loaded(waveform_file) = &tracked_file.state {
                #[cfg(debug_assertions)]
                {
                    if let (Some(start_ns), Some(end_ns)) =
                        (waveform_file.min_time_ns, waveform_file.max_time_ns)
                    {
                        if end_ns <= start_ns {
                            zoon::println!(
                                "⚠️ Timeline bounds for '{}' are non-positive ({}..{})",
                                tracked_file.filename,
                                start_ns,
                                end_ns
                            );
                        } else {
                            let span_ns = end_ns - start_ns;
                            const SUSPECT_SPAN_THRESHOLD_NS: u64 = 1_000_000_000_000; // ~1000 seconds
                            if span_ns >= SUSPECT_SPAN_THRESHOLD_NS
                                && matches!(waveform_file.format, FileFormat::FST)
                            {
                                zoon::println!(
                                    "⚠️ FST '{}' spans {:.1}s; check timescale inference",
                                    tracked_file.filename,
                                    span_ns as f64 / 1_000_000_000.0
                                );
                            }
                        }
                    }
                }

                if let Some(start_ns) = waveform_file.min_time_ns {
                    let start = TimePs::from_nanos(start_ns);
                    min_time = Some(match min_time {
                        Some(current) => current.min(start),
                        None => start,
                    });
                }
                if let Some(end_ns) = waveform_file.max_time_ns {
                    let end = TimePs::from_nanos(end_ns);
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
