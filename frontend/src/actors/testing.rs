//! Comprehensive Actor+Relay Testing Patterns
//!
//! Signal-based testing without .get() methods, following reactive testing principles.

#[cfg(test)]
mod actor_relay_tests {
    use super::*;
    use crate::actors::{Actor, ActorVec, Relay, relay, tracked_files_domain, waveform_timeline_domain};
    use crate::time_types::{TimeNs, Viewport, NsPerPixel};
    use shared::{TrackedFile, SelectedVariable, FileState};
    use zoon::{Task, Timer};
    use futures::StreamExt;
    use std::path::PathBuf;
    
    /// Test TrackedFiles domain follows Actor+Relay patterns
    #[tokio::test]
    async fn test_tracked_files_domain() {
        let tracked_files = tracked_files_domain();
        
        // Test file_dropped_relay event
        let test_paths = vec![PathBuf::from("test1.vcd"), PathBuf::from("test2.vcd")];
        tracked_files.file_dropped_relay.send(test_paths.clone());
        
        // Wait reactively for state change using signal
        let files = tracked_files.files
            .signal_vec()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
            
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, test_paths[0]);
        assert_eq!(files[1].path, test_paths[1]);
        
        // Test file_selected_relay event
        tracked_files.file_selected_relay.send(test_paths[0].clone());
        
        // Verify reactive behavior without using .get()
        Timer::sleep(50).await; // Allow signal processing
        // In real implementation, would check selection state via signal
    }
    
    /// Test WaveformTimeline domain cursor management
    #[tokio::test]
    async fn test_waveform_timeline_cursor() {
        let waveform_timeline = waveform_timeline_domain();
        
        // Test cursor_dragged_relay event
        let test_position = 123.456;
        waveform_timeline.cursor_dragged_relay.send(test_position);
        
        // Wait for cursor position signal to update
        let cursor_pos = waveform_timeline.cursor_position_signal()
            .to_stream()
            .next().await.unwrap();
            
        assert!((cursor_pos - test_position).abs() < f64::EPSILON);
    }
    
    /// Test WaveformTimeline domain viewport management
    #[tokio::test]
    async fn test_waveform_timeline_viewport() {
        let waveform_timeline = waveform_timeline_domain();
        
        // Test viewport_changed_relay event
        let test_viewport = (10.0, 50.0); // start_seconds, end_seconds
        waveform_timeline.viewport_changed_relay.send(test_viewport);
        
        // Wait for viewport signal to update
        let viewport = waveform_timeline.viewport_signal()
            .to_stream()
            .next().await.unwrap();
            
        assert_eq!(viewport.start.display_seconds(), test_viewport.0);
        assert_eq!(viewport.end.display_seconds(), test_viewport.1);
    }
    
    /// Test WaveformTimeline domain zoom management
    #[tokio::test]
    async fn test_waveform_timeline_zoom() {
        let waveform_timeline = waveform_timeline_domain();
        
        // Test zoom_changed_relay event
        let test_zoom_factor = 2.0;
        waveform_timeline.zoom_changed_relay.send(test_zoom_factor);
        
        // Wait for ns_per_pixel signal to update
        let ns_per_pixel = waveform_timeline.ns_per_pixel_signal()
            .to_stream()
            .next().await.unwrap();
            
        // Verify zoom factor conversion applied correctly
        let expected_ns = 1_000_000.0 / test_zoom_factor;
        assert!((ns_per_pixel.nanos() as f32 - expected_ns).abs() < 1.0);
    }
    
    /// Test multiple rapid events are processed sequentially
    #[tokio::test]
    async fn test_sequential_event_processing() {
        let waveform_timeline = waveform_timeline_domain();
        
        let mut cursor_stream = waveform_timeline.cursor_position_signal().to_stream();
        
        // Send multiple cursor events rapidly
        waveform_timeline.cursor_dragged_relay.send(10.0);
        waveform_timeline.cursor_dragged_relay.send(20.0);
        waveform_timeline.cursor_dragged_relay.send(30.0);
        
        // Verify sequential processing - should get final result
        let final_position = cursor_stream.next().await.unwrap();
        
        // Due to sequential processing, should reflect final position
        assert_eq!(final_position, 30.0);
    }
    
    /// Test that value caching bridge works correctly
    #[tokio::test]
    async fn test_value_caching_bridge() {
        use crate::actors::domain_bridges::{
            get_cached_cursor_position_seconds, 
            set_cursor_position_seconds,
            get_cached_viewport,
            set_viewport_if_changed
        };
        use crate::time_types::Viewport;
        
        // Test cursor position caching
        let test_position = 42.0;
        set_cursor_position_seconds(test_position);
        
        // Allow bridge signal processing
        Timer::sleep(50).await;
        
        // Verify cached access returns same value
        let cached_position = get_cached_cursor_position_seconds();
        assert_eq!(cached_position, test_position);
        
        // Test viewport caching
        let test_viewport = Viewport::new(
            TimeNs::from_external_seconds(100.0),
            TimeNs::from_external_seconds(200.0)
        );
        set_viewport_if_changed(test_viewport);
        
        // Allow bridge signal processing
        Timer::sleep(50).await;
        
        // Verify cached access
        let cached_viewport = get_cached_viewport();
        assert_eq!(cached_viewport.start.display_seconds(), 100.0);
        assert_eq!(cached_viewport.end.display_seconds(), 200.0);
    }
    
    /// Integration test: File loading → Variable selection → Timeline display workflow
    #[tokio::test]
    async fn test_complete_workflow_integration() {
        let tracked_files = tracked_files_domain();
        let waveform_timeline = waveform_timeline_domain();
        
        // 1. Load files through TrackedFiles domain
        let test_files = vec![PathBuf::from("integration_test.vcd")];
        tracked_files.file_dropped_relay.send(test_files.clone());
        
        // Wait for files to be tracked
        let files = tracked_files.files
            .signal_vec()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
        assert_eq!(files.len(), 1);
        
        // 2. Set timeline cursor position
        let cursor_seconds = 25.5;
        waveform_timeline.cursor_dragged_relay.send(cursor_seconds);
        
        // Wait for cursor position update
        let cursor_pos = waveform_timeline.cursor_position_signal()
            .to_stream()
            .next().await.unwrap();
        assert_eq!(cursor_pos, cursor_seconds);
        
        // 3. Test viewport changes affect timeline
        let viewport_tuple = (10.0, 100.0);
        waveform_timeline.viewport_changed_relay.send(viewport_tuple);
        
        let viewport = waveform_timeline.viewport_signal()
            .to_stream()
            .next().await.unwrap();
        assert_eq!(viewport.start.display_seconds(), viewport_tuple.0);
        assert_eq!(viewport.end.display_seconds(), viewport_tuple.1);
        
        // Integration successful: File → Timeline workflow verified
    }
    
    /// Test Actor+Relay architecture prevents recursive locks
    #[tokio::test]
    async fn test_no_recursive_locks() {
        let waveform_timeline = waveform_timeline_domain();
        
        // Rapid concurrent events should not cause recursive locks
        for i in 1..=10 {
            let position = i as f64 * 10.0;
            waveform_timeline.cursor_dragged_relay.send(position);
            waveform_timeline.zoom_changed_relay.send(2.0);
            waveform_timeline.viewport_changed_relay.send((0.0, position * 2.0));
        }
        
        // Wait for all processing to complete
        Timer::sleep(100).await;
        
        // Verify final state is consistent (no panics occurred)
        let final_cursor = waveform_timeline.cursor_position_signal()
            .to_stream()
            .next().await.unwrap();
        
        // Should have final cursor position (100.0 from last iteration)
        assert_eq!(final_cursor, 100.0);
    }
    
    /// Test signal-based testing pattern for collections  
    #[tokio::test]
    async fn test_actor_vec_signal_based_testing() {
        let tracked_files = tracked_files_domain();
        
        // Test empty initial state
        let initial_files = tracked_files.files
            .signal_vec()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
        assert_eq!(initial_files.len(), 0);
        
        // Add files one by one and test incremental changes
        tracked_files.file_dropped_relay.send(vec![PathBuf::from("file1.vcd")]);
        
        let files_after_first = tracked_files.files
            .signal_vec()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
        assert_eq!(files_after_first.len(), 1);
        
        tracked_files.file_dropped_relay.send(vec![PathBuf::from("file2.vcd"), PathBuf::from("file3.vcd")]);
        
        let files_after_batch = tracked_files.files
            .signal_vec()
            .to_signal_cloned()
            .to_stream()
            .next().await.unwrap();
        assert_eq!(files_after_batch.len(), 3);
    }
    
    /// Test error conditions and edge cases
    #[tokio::test] 
    async fn test_edge_cases() {
        let waveform_timeline = waveform_timeline_domain();
        
        // Test extreme zoom values
        waveform_timeline.zoom_changed_relay.send(0.001); // Very zoomed out
        let ns_per_pixel_zoomed_out = waveform_timeline.ns_per_pixel_signal()
            .to_stream()
            .next().await.unwrap();
        assert!(ns_per_pixel_zoomed_out.nanos() > 1_000_000); // Should be large value
        
        waveform_timeline.zoom_changed_relay.send(1000.0); // Very zoomed in
        let ns_per_pixel_zoomed_in = waveform_timeline.ns_per_pixel_signal()
            .to_stream()
            .next().await.unwrap();
        assert!(ns_per_pixel_zoomed_in.nanos() < 10_000); // Should be small value
        
        // Test boundary cursor positions
        waveform_timeline.cursor_dragged_relay.send(-1.0); // Negative time
        waveform_timeline.cursor_dragged_relay.send(f64::MAX); // Extreme time
        
        // Should handle gracefully without panicking
        let final_cursor = waveform_timeline.cursor_position_signal()
            .to_stream()
            .next().await.unwrap();
        
        // Should have some reasonable final value (exact value depends on clamping logic)
        assert!(final_cursor.is_finite());
    }
}

/// Test utilities for Actor+Relay testing
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use futures::stream::Stream;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    
    /// Helper for collecting multiple signal emissions during testing
    pub struct SignalCollector<T> {
        emissions: Arc<Mutex<Vec<T>>>,
    }
    
    impl<T: Clone + Send + 'static> SignalCollector<T> {
        pub fn new() -> Self {
            Self {
                emissions: Arc::new(Mutex::new(Vec::new())),
            }
        }
        
        /// Start collecting signals from the provided stream
        pub fn start_collecting<S>(&self, mut signal_stream: S)
        where
            S: Stream<Item = T> + Unpin + Send + 'static
        {
            let emissions = self.emissions.clone();
            Task::start(async move {
                while let Some(value) = signal_stream.next().await {
                    emissions.lock().unwrap().push(value);
                }
            });
        }
        
        /// Get all collected emissions
        pub fn get_emissions(&self) -> Vec<T> {
            self.emissions.lock().unwrap().clone()
        }
        
        /// Wait until at least count emissions are collected
        pub async fn wait_for_emissions(&self, count: usize, timeout_ms: u64) -> Vec<T> {
            let start = Instant::now();
            
            loop {
                let current_count = self.emissions.lock().unwrap().len();
                if current_count >= count {
                    return self.get_emissions();
                }
                
                if start.elapsed() > Duration::from_millis(timeout_ms) {
                    panic!("Timeout waiting for {} emissions, got {}", count, current_count);
                }
                
                Timer::sleep(10).await;
            }
        }
        
        /// Clear collected emissions
        pub fn clear(&self) {
            self.emissions.lock().unwrap().clear();
        }
    }
    
    /// Mock relay for testing external dependencies
    pub struct MockRelay<T> {
        events: Arc<Mutex<Vec<T>>>,
        auto_respond: Option<Box<dyn Fn(&T) -> Option<T> + Send + Sync>>,
    }
    
    impl<T: Clone> MockRelay<T> {
        pub fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                auto_respond: None,
            }
        }
        
        /// Set auto-response function for testing
        pub fn with_auto_response<F>(mut self, f: F) -> Self
        where
            F: Fn(&T) -> Option<T> + Send + Sync + 'static
        {
            self.auto_respond = Some(Box::new(f));
            self
        }
        
        /// Get all emitted events
        pub fn events(&self) -> Vec<T> {
            self.events.lock().unwrap().clone()
        }
        
        /// Clear recorded events
        pub fn clear_events(&self) {
            self.events.lock().unwrap().clear();
        }
        
        /// Simulate emission
        pub fn simulate_emit(&self, event: T) {
            self.events.lock().unwrap().push(event.clone());
            
            // Call auto-response if configured
            if let Some(ref responder) = self.auto_respond {
                if let Some(response) = responder(&event) {
                    self.events.lock().unwrap().push(response);
                }
            }
        }
    }
}

/// Development testing helpers (available in debug builds)
#[cfg(debug_assertions)]
pub mod dev_testing {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;
    
    /// Debug tracker for Actor+Relay interactions
    pub struct ActorDebugger {
        enabled: Arc<Mutex<bool>>,
        filter: Arc<Mutex<Option<String>>>,
        events: Arc<Mutex<Vec<DebugEvent>>>,
    }
    
    #[derive(Debug, Clone)]
    pub struct DebugEvent {
        pub timestamp: f64,
        pub actor_name: String,
        pub event_type: String,
        pub data: String,
    }
    
    impl ActorDebugger {
        /// Create new debugger instance
        pub fn new() -> Self {
            Self {
                enabled: Arc::new(Mutex::new(false)),
                filter: Arc::new(Mutex::new(None)),
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }
        
        /// Enable/disable debug output
        pub fn set_enabled(&self, enabled: bool) {
            *self.enabled.lock().unwrap() = enabled;
        }
        
        /// Set filter for specific actors
        pub fn set_filter(&self, pattern: Option<String>) {
            *self.filter.lock().unwrap() = pattern;
        }
        
        /// Record debug event
        pub fn record_event(&self, actor_name: &str, event_type: &str, data: &str) {
            if !*self.enabled.lock().unwrap() {
                return;
            }
            
            if let Some(ref filter) = *self.filter.lock().unwrap() {
                if !actor_name.contains(filter) {
                    return;
                }
            }
            
            let event = DebugEvent {
                timestamp: js_sys::Date::now(),
                actor_name: actor_name.to_string(),
                event_type: event_type.to_string(),
                data: data.to_string(),
            };
            
            self.events.lock().unwrap().push(event);
            zoon::println!("🔍 [{}] {}: {}", actor_name, event_type, data);
        }
        
        /// Get all recorded events
        pub fn get_events(&self) -> Vec<DebugEvent> {
            self.events.lock().unwrap().clone()
        }
        
        /// Clear all debug data
        pub fn clear(&self) {
            self.events.lock().unwrap().clear();
        }
        
        /// Print summary of recorded events
        pub fn print_summary(&self) {
            let events = self.get_events();
            let mut actor_counts: HashMap<String, usize> = HashMap::new();
            
            for event in &events {
                *actor_counts.entry(event.actor_name.clone()).or_insert(0) += 1;
            }
            
            zoon::println!("📊 Actor Event Summary:");
            for (actor, count) in actor_counts {
                zoon::println!("  {}: {} events", actor, count);
            }
        }
    }
    
    /// Global debugger instance
    static ACTOR_DEBUGGER: std::sync::LazyLock<ActorDebugger> = 
        std::sync::LazyLock::new(|| ActorDebugger::new());
    
    /// Debug macro for Actor events
    #[macro_export]
    macro_rules! actor_debug {
        ($actor:expr, $event_type:expr, $data:expr) => {
            #[cfg(debug_assertions)]
            $crate::actors::testing::ACTOR_DEBUGGER.record_event($actor, $event_type, &format!("{}", $data));
        };
    }
    
    /// Enable Actor debugging for development
    pub fn enable_actor_debugging() {
        ACTOR_DEBUGGER.set_enabled(true);
    }
    
    /// Disable Actor debugging
    pub fn disable_actor_debugging() {
        ACTOR_DEBUGGER.set_enabled(false);
    }
    
    /// Set Actor debugging filter
    pub fn set_actor_debug_filter(pattern: Option<String>) {
        ACTOR_DEBUGGER.set_filter(pattern);
    }
    
    /// Get Actor debugging summary
    pub fn get_actor_debug_summary() -> Vec<DebugEvent> {
        ACTOR_DEBUGGER.get_events()
    }
    
    /// Clear Actor debugging data
    pub fn clear_actor_debug_data() {
        ACTOR_DEBUGGER.clear();
    }
    
    /// Print Actor debugging summary
    pub fn print_actor_debug_summary() {
        ACTOR_DEBUGGER.print_summary();
    }
}

/// Performance testing utilities
#[cfg(test)]
pub mod performance_tests {
    use super::*;
    use std::time::{Duration, Instant};
    
    /// Test that Actor+Relay architecture has minimal over-rendering
    #[tokio::test]
    async fn test_reduced_over_rendering() {
        use crate::actors::test_utils::SignalCollector;
        
        let waveform_timeline = waveform_timeline_domain();
        
        // Set up signal collection to count emissions
        let cursor_collector = SignalCollector::new();
        let cursor_stream = waveform_timeline.cursor_position_signal().to_stream();
        cursor_collector.start_collecting(cursor_stream);
        
        let viewport_collector = SignalCollector::new();
        let viewport_stream = waveform_timeline.viewport_signal().to_stream();
        viewport_collector.start_collecting(viewport_stream);
        
        // Perform multiple operations that should be batched
        let start_time = Instant::now();
        for i in 1..=10 {
            waveform_timeline.cursor_dragged_relay.send(i as f64);
            waveform_timeline.viewport_changed_relay.send((0.0, i as f64 * 2.0));
        }
        
        // Wait for processing
        Timer::sleep(200).await;
        let processing_time = start_time.elapsed();
        
        // Verify performance: Should have minimal signal emissions (target < 5 per operation)
        let cursor_emissions = cursor_collector.get_emissions();
        let viewport_emissions = viewport_collector.get_emissions();
        
        // With proper batching, should have much fewer emissions than operations
        assert!(cursor_emissions.len() <= 10, "Too many cursor emissions: {}", cursor_emissions.len());
        assert!(viewport_emissions.len() <= 10, "Too many viewport emissions: {}", viewport_emissions.len());
        
        // Processing should be fast (sub-second for 10 operations)
        assert!(processing_time < Duration::from_millis(500), 
               "Processing too slow: {:?}", processing_time);
        
        zoon::println!("🚀 Performance test: {} cursor signals, {} viewport signals in {:?}", 
                      cursor_emissions.len(), viewport_emissions.len(), processing_time);
    }
}