//! MaximumTimelineRange standalone derived state actor
//!
//! Centralized computation of timeline range from multiple files to eliminate
//! scattered computation and provide single source of truth for timeline bounds.

use super::time_domain::TimePs;
use shared::FileState;
use std::collections::HashSet;
use std::sync::Arc;
use zoon::{map_ref, Mutable, SignalExt, SignalVecExt, Task, TaskHandle};

/// Minimal data extracted from TrackedFile for range computation
#[derive(Clone, PartialEq)]
struct FileRangeData {
    path: String,
    time_range: Option<(u64, u64)>,
}

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

        let files_signal = tracked_files.files.signal_vec_cloned()
            .map(|file| FileRangeData {
                path: file.path.clone(),
                time_range: match &file.state {
                    FileState::Loaded(wf) => match (wf.min_time_ns, wf.max_time_ns) {
                        (Some(min), Some(max)) => Some((min, max)),
                        _ => None,
                    },
                    _ => None,
                },
            })
            .to_signal_cloned();

        let vars_signal = selected_variables.variables_vec_actor.signal_cloned()
            .map(|vars| vars.iter().filter_map(|v| v.file_path()).collect::<HashSet<_>>());

        let _range_task = Arc::new(Task::start_droppable(
            map_ref! {
                let file_ranges = files_signal,
                let file_filter = vars_signal
                    =>
                Self::compute_range(&file_ranges, &file_filter)
            }
            .for_each(move |computed| {
                range_clone.set_neq(computed);
                async {}
            }),
        ));

        Self { range, _range_task }
    }

    /// Pure function to compute maximum range from extracted data
    fn compute_range(
        file_ranges: &[FileRangeData],
        file_filter: &HashSet<String>,
    ) -> Option<(TimePs, TimePs)> {
        if file_filter.is_empty() {
            return None;
        }

        let mut min_time: Option<TimePs> = None;
        let mut max_time: Option<TimePs> = None;

        for file_data in file_ranges {
            if !file_filter.contains(&file_data.path) {
                continue;
            }

            if let Some((start_ns, end_ns)) = file_data.time_range {
                let start = TimePs::from_nanos(start_ns);
                min_time = Some(match min_time {
                    Some(current) => current.min(start),
                    None => start,
                });

                let end = TimePs::from_nanos(end_ns);
                max_time = Some(match max_time {
                    Some(current) => current.max(end),
                    None => end,
                });
            }
        }

        match (min_time, max_time) {
            (Some(start), Some(end)) if end > start => Some((start, end)),
            _ => None,
        }
    }
}
