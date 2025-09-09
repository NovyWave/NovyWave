use shared::{FileState, TrackedFile};
use zoon::*;

/// Get timeline range from loaded tracked files
pub fn get_timeline_range_from_files(tracked_files: &Vec<TrackedFile>) -> Option<(f64, f64)> {
    let loaded_files: Vec<_> = tracked_files
        .iter()
        .filter_map(|tracked_file| match &tracked_file.state {
            FileState::Loaded(waveform_file) => Some(waveform_file),
            _ => None,
        })
        .collect();

    if loaded_files.is_empty() {
        return None;
    }

    let mut min_time: f64 = f64::MAX;
    let mut max_time: f64 = f64::MIN;

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

/// Create a signal for timeline range based on tracked files and selected variables
pub fn timeline_range_signal(
    tracked_files: &crate::tracked_files::TrackedFiles,
    selected_variables: &crate::selected_variables::SelectedVariables,
) -> impl Signal<Item = Option<(f64, f64)>> {
    let files_count_signal = tracked_files.file_count_signal();
    let variables_signal = selected_variables.variables_vec_actor.signal();
    
    map_ref! {
        let files_count = files_count_signal,
        let _selected_vars = variables_signal,
        let tracked_files_data = tracked_files.files_vec_signal.signal_cloned() => {
            if *files_count == 0 {
                None
            } else {
                // Get actual timeline range from tracked files data
                get_timeline_range_from_files(&tracked_files_data)
            }
        }
    }
    .dedupe_cloned()
}

/// Format time value with appropriate units (ns, μs, ms, s)
pub fn format_time(time: f64) -> String {
    if !time.is_finite() || time <= 0.0 {
        "0s".to_string()
    } else if time < 1e-6 {
        let ns_val = time * 1e9;
        if ns_val.fract() == 0.0 {
            format!("{}ns", ns_val as i64)
        } else {
            format!("{:.1}ns", ns_val)
        }
    } else if time < 1e-3 {
        let us_val = time * 1e6;
        if us_val.fract() == 0.0 {
            format!("{}μs", us_val as i64)
        } else {
            format!("{:.1}μs", us_val)
        }
    } else if time < 1.0 {
        let ms_val = time * 1e3;
        if ms_val.fract() == 0.0 {
            format!("{}ms", ms_val as i64)
        } else {
            format!("{:.1}ms", ms_val)
        }
    } else {
        if time.fract() == 0.0 {
            format!("{}s", time as i64)
        } else {
            format!("{:.1}s", time)
        }
    }
}