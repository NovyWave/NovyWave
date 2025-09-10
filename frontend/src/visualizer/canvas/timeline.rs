use crate::visualizer::timeline::timeline_actor::NsPerPixel;
// use crate::visualizer::timeline::timeline_actor::{current_ns_per_pixel, current_viewport}; // Functions do not exist
use crate::dataflow::*;
use futures::{select, stream::StreamExt, FutureExt};
use std::collections::HashSet;
use zoon::{SignalExt, Signal};

#[derive(Clone)]
pub struct TimelineContext {
    pub tracked_files: crate::tracked_files::TrackedFiles,
    pub selected_variables: crate::selected_variables::SelectedVariables,
    pub waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
}

impl TimelineContext {
    pub fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
        waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    ) -> Self {
        Self {
            tracked_files,
            selected_variables,
            waveform_timeline,
        }
    }
}


pub fn get_min_valid_range_ns(canvas_width: u32) -> u64 {
    NsPerPixel::MAX_ZOOM_IN.nanos() * canvas_width as u64
}


impl TimelineContext {
    pub async fn compute_maximum_timeline_range(&self) -> Option<(f64, f64)> {
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<shared::WaveformFile> = tracked_files
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        let selected_file_paths = self.get_selected_variable_file_paths().await;

        let mut min_time: f64 = f64::MAX;
        let mut max_time: f64 = f64::MIN;
        let mut has_valid_files = false;

        if selected_file_paths.is_empty() {
            let file_range_result = self.compute_full_file_range();
            if let Some((file_min, file_max)) = file_range_result {
                if file_min < file_max && file_min.is_finite() && file_max.is_finite() {
                    return Some((file_min, file_max));
                }
            }
            return None;
        } else {
            for file in loaded_files.iter() {
                let file_matches = selected_file_paths.iter().any(|path| {
                    let matches = file.id == *path;
                    matches
                });

                if file_matches {
                    if let (Some(file_min), Some(file_max)) = (
                        file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                        file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                    ) {
                        min_time = min_time.min(file_min);
                        max_time = max_time.max(file_max);
                        has_valid_files = true;
                    }
                }
            }
        }

        if !has_valid_files || min_time == max_time {
            return None;
        }

        if !min_time.is_finite() || !max_time.is_finite() {
            return None;
        }

        let file_range = max_time - min_time;
        let canvas_width = self.waveform_timeline.canvas_width_signal().to_stream().next().await.unwrap_or(0.0);
        if file_range < get_min_valid_range_ns(canvas_width as u32) as f64 / 1_000_000_000.0 {
            let expanded_end = min_time + get_min_valid_range_ns(canvas_width as u32) as f64 / 1_000_000_000.0;
            if expanded_end.is_finite() {
                return Some((min_time, expanded_end));
            } else {
                return None;
            }
        }
        
        let result = (min_time, max_time);
        Some(result)
    }

    pub async fn get_selected_variable_file_paths(&self) -> HashSet<String> {
        let selected_vars = self.selected_variables.variables_vec_actor.signal().to_stream().next().await.unwrap_or_default();
        selected_vars
            .iter()
            .filter_map(|var| var.file_path())
            .collect()
    }

    pub fn compute_full_file_range(&self) -> Option<(f64, f64)> {
        let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
        let loaded_files: Vec<shared::WaveformFile> = tracked_files
            .iter()
            .filter_map(|tracked_file| match &tracked_file.state {
                shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
                _ => None,
            })
            .collect();

        let mut file_candidates: Vec<_> = loaded_files
            .iter()
            .filter_map(|file| {
                if let (Some(file_min), Some(file_max)) = (
                    file.min_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                    file.max_time_ns.map(|ns| ns as f64 / 1_000_000_000.0),
                ) {
                    if file_min.is_finite() && file_max.is_finite() && file_min < file_max {
                        let span = file_max - file_min;
                        Some((file, file_min, file_max, span))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if file_candidates.is_empty() {
            return None;
        }

        file_candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((_file, file_min, file_max, _span)) = file_candidates.first() {
            let min_time = *file_min;
            let max_time = *file_max;
            
            if min_time < max_time {
                let time_range = max_time - min_time;
                let buffer = time_range * 0.2;
                let expanded_min = (min_time - buffer).max(0.0);
                let expanded_max = max_time + buffer;
                
                validate_timeline_range(expanded_min, expanded_max)
            } else {
                None
            }
        } else {
            None
        }
    }

    // TODO: Replace with signal-based implementation - no direct state access allowed
    // pub fn compute_selected_variables_file_range(&self) -> Option<(f64, f64)> {
    //     let selected_variables = self.selected_variables.variables_vec_actor.signal().to_stream().next().await;
    //     let tracked_files = self.tracked_files.files_vec_signal.get_cloned();
    //     let loaded_files: Vec<shared::WaveformFile> = tracked_files
    //         .iter()
    //         .filter_map(|tracked_file| match &tracked_file.state {
    //             shared::FileState::Loaded(waveform_file) => Some(waveform_file.clone()),
    //             _ => None,
    //         })
    //         .collect();
    //     // ... rest of function requires async signal-based implementation
    // }

    pub async fn compute_current_timeline_range(&self) -> Option<(f64, f64)> {
        let current_viewport = self.waveform_timeline.viewport_signal().to_stream().next().await.unwrap_or_default();
        
        let range_start = current_viewport.start.display_seconds();
        let range_end = current_viewport.end.display_seconds();

        if range_end > range_start && range_start >= 0.0 && range_start.is_finite() && range_end.is_finite() {
            let canvas_width = self.waveform_timeline.canvas_width_signal().to_stream().next().await.unwrap_or(0.0) as u32;
            let min_zoom_range = get_min_valid_range_ns(canvas_width) as f64 / 1_000_000_000.0;
            let current_range = range_end - range_start;

            if current_range >= min_zoom_range {
                return Some((range_start, range_end));
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub struct MaximumTimelineRange {
    pub range: Actor<Option<(f64, f64)>>,
    pub range_updated_relay: Relay<Option<(f64, f64)>>,
}

impl MaximumTimelineRange {
    pub async fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
        waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    ) -> Self {
        let (range_updated_relay, mut range_updated_stream) = relay();
        
        let range = Actor::new(None, async move |state| {
            loop {
                select! {
                    new_range = range_updated_stream.next() => {
                        if let Some(new_range) = new_range {
                            state.set(new_range);
                        } else {
                            break; // Stream closed
                        }
                    }
                }
            }
        });
        
        let timeline_context = TimelineContext::new(tracked_files.clone(), selected_variables.clone(), waveform_timeline.clone());
        let range_relay = range_updated_relay.clone();
        
        let range_computation_actor = Actor::new(false, async move |state_handle| {
            let mut files_dropped_stream = tracked_files.files_dropped_relay.subscribe();
            let mut file_loaded_stream = tracked_files.file_load_completed_relay.subscribe();
            let mut variables_stream = selected_variables.variable_clicked_relay.subscribe();
            let mut selection_cleared_stream = selected_variables.selection_cleared_relay.subscribe();
            
            loop {
                select! {
                    _ = files_dropped_stream.next() => {
                        if !state_handle.get() {
                            let new_range = timeline_context.compute_maximum_timeline_range().await;
                            range_relay.send(new_range);
                            state_handle.set(true);
                        }
                    }
                    _ = file_loaded_stream.next() => {
                        let new_range = timeline_context.compute_maximum_timeline_range().await;
                        range_relay.send(new_range);
                    }
                    _ = variables_stream.next() => {
                        let new_range = timeline_context.compute_maximum_timeline_range().await;
                        range_relay.send(new_range);
                    }
                    _ = selection_cleared_stream.next() => {
                        let new_range = timeline_context.compute_maximum_timeline_range().await;
                        range_relay.send(new_range);
                    }
                }
            }
        });
        
        Self { range, range_updated_relay }
    }
    
    pub fn range_signal(&self) -> impl Signal<Item = Option<(f64, f64)>> {
        self.range.signal()
    }
}

#[derive(Clone, Debug)]
pub struct CurrentTimelineRange {
    pub range: Actor<Option<(f64, f64)>>,
    pub range_updated_relay: Relay<Option<(f64, f64)>>,
}

impl CurrentTimelineRange {
    pub async fn new(
        tracked_files: crate::tracked_files::TrackedFiles,
        selected_variables: crate::selected_variables::SelectedVariables,
        waveform_timeline: crate::visualizer::timeline::timeline_actor::WaveformTimeline,
    ) -> Self {
        let (range_updated_relay, mut range_updated_stream) = relay();
        
        let range = Actor::new(None, async move |state| {
            loop {
                select! {
                    new_range = range_updated_stream.next() => {
                        if let Some(new_range) = new_range {
                            state.set(new_range);
                        } else {
                            break; // Stream closed
                        }
                    }
                }
            }
        });
        
        let timeline_context = TimelineContext::new(
            tracked_files, 
            selected_variables, 
            waveform_timeline.clone()
        );
        let range_relay = range_updated_relay.clone();
        
        let range_computation_actor = Actor::new(false, async move |state_handle| {
            let mut viewport_stream = waveform_timeline.viewport_signal().to_stream().fuse();
            
            loop {
                select! {
                    _ = viewport_stream.next() => {
                        let new_range = timeline_context.compute_current_timeline_range().await;
                        range_relay.send(new_range);
                    }
                }
            }
        });
        
        Self { range, range_updated_relay }
    }
    
    pub fn range_signal(&self) -> impl Signal<Item = Option<(f64, f64)>> {
        self.range.signal()
    }
}

pub fn validate_timeline_range(start: f64, end: f64) -> Option<(f64, f64)> {
    if !start.is_finite() || !end.is_finite() || start >= end {
        None
    } else {
        Some((start, end))
    }
}



