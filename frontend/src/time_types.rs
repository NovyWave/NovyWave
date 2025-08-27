use std::fmt;
use std::ops::{Add, Sub};

/// Represents a point in time as nanoseconds since the start of a waveform file.
/// 
/// Uses u64 internally to provide:
/// - 1 nanosecond resolution
/// - ~584 years maximum duration (u64::MAX / 1_000_000_000 / 60 / 60 / 24 / 365)
/// - No floating point precision issues
/// - Fast integer arithmetic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeNs(pub u64);

impl TimeNs {
    pub const ZERO: TimeNs = TimeNs(0);
    
    /// Create a new TimeNs from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        TimeNs(nanos)
    }
    
    /// Create a new TimeNs from seconds (converts to nanoseconds)
    pub fn from_seconds(seconds: f64) -> Self {
        TimeNs((seconds * 1_000_000_000.0) as u64)
    }
    
    /// Get nanoseconds value
    pub fn nanos(self) -> u64 {
        self.0
    }
    
    /// Convert to seconds (for display purposes only)
    pub fn to_seconds(self) -> f64 {
        self.0 as f64 / 1_000_000_000.0
    }
    
    /// Convert to milliseconds (for display purposes only)
    pub fn to_millis(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }
    
    /// Convert to microseconds (for display purposes only)
    pub fn to_micros(self) -> f64 {
        self.0 as f64 / 1_000.0
    }
    
    /// Safely subtract two time points, returning a duration
    pub fn duration_since(self, earlier: TimeNs) -> DurationNs {
        DurationNs(self.0.saturating_sub(earlier.0))
    }
    
    /// Safely add a duration to this time point
    pub fn add_duration(self, duration: DurationNs) -> TimeNs {
        TimeNs(self.0.saturating_add(duration.0))
    }
    
    /// Safely subtract a duration from this time point
    pub fn sub_duration(self, duration: DurationNs) -> TimeNs {
        TimeNs(self.0.saturating_sub(duration.0))
    }
}

impl fmt::Display for TimeNs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.to_seconds();
        if seconds >= 1.0 {
            write!(f, "{:.3}s", seconds)
        } else if seconds >= 0.001 {
            write!(f, "{:.3}ms", self.to_millis())
        } else if seconds >= 0.000001 {
            write!(f, "{:.3}μs", self.to_micros())
        } else {
            write!(f, "{}ns", self.0)
        }
    }
}

/// Represents a duration in nanoseconds.
/// 
/// Used for time ranges, zoom calculations, and temporal arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationNs(pub u64);

impl DurationNs {
    pub const ZERO: DurationNs = DurationNs(0);
    
    /// Create a new DurationNs from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        DurationNs(nanos)
    }
    
    /// Create a new DurationNs from seconds (converts to nanoseconds)
    pub fn from_seconds(seconds: f64) -> Self {
        DurationNs((seconds * 1_000_000_000.0) as u64)
    }
    
    /// Get nanoseconds value
    pub fn nanos(self) -> u64 {
        self.0
    }
    
    /// Convert to seconds (for display purposes only)
    pub fn to_seconds(self) -> f64 {
        self.0 as f64 / 1_000_000_000.0
    }
    
    /// Divide duration by a factor (for zoom calculations)
    pub fn div_f64(self, divisor: f64) -> DurationNs {
        DurationNs((self.0 as f64 / divisor).round() as u64)
    }
    
    /// Multiply duration by a factor (for zoom calculations)
    pub fn mul_f64(self, multiplier: f64) -> DurationNs {
        DurationNs((self.0 as f64 * multiplier).round() as u64)
    }
}

impl fmt::Display for DurationNs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.to_seconds();
        if seconds >= 1.0 {
            write!(f, "{:.3}s", seconds)
        } else if seconds >= 0.001 {
            write!(f, "{:.3}ms", self.0 as f64 / 1_000_000.0)
        } else if seconds >= 0.000001 {
            write!(f, "{:.3}μs", self.0 as f64 / 1_000.0)
        } else {
            write!(f, "{}ns", self.0)
        }
    }
}

impl Add for DurationNs {
    type Output = DurationNs;
    
    fn add(self, rhs: DurationNs) -> DurationNs {
        DurationNs(self.0.saturating_add(rhs.0))
    }
}

impl Sub for DurationNs {
    type Output = DurationNs;
    
    fn sub(self, rhs: DurationNs) -> DurationNs {
        DurationNs(self.0.saturating_sub(rhs.0))
    }
}

/// Represents zoom level as a percentage.
/// 
/// Examples:
/// - 100 = 1x (normal zoom)
/// - 200 = 2x (zoomed in 2x)
/// - 50 = 0.5x (zoomed out to show more)
/// - 1000 = 10x (highly zoomed in)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZoomLevel(pub u32);

impl ZoomLevel {
    pub const NORMAL: ZoomLevel = ZoomLevel(100);
    pub const MIN: ZoomLevel = ZoomLevel(100);    // Maximum zoom out (1x = full file)
    pub const MAX: ZoomLevel = ZoomLevel(1_000_000); // Maximum zoom in
    
    /// Create a new zoom level from percentage
    pub fn from_percent(percent: u32) -> Self {
        ZoomLevel(percent.clamp(Self::MIN.0, Self::MAX.0))
    }
    
    /// Create a new zoom level from floating point factor
    pub fn from_factor(factor: f32) -> Self {
        let percent = (factor * 100.0) as u32;
        Self::from_percent(percent)
    }
    
    /// Get percentage value
    pub fn percent(self) -> u32 {
        self.0
    }
    
    /// Get floating point factor (for calculations)
    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }
    
    /// Check if this zoom level would show more detail than another
    pub fn is_more_detailed_than(self, other: ZoomLevel) -> bool {
        self.0 > other.0
    }
    
    /// Increase zoom level by a factor
    pub fn zoom_in(self, factor: f32) -> Self {
        let new_percent = (self.0 as f32 * factor) as u32;
        Self::from_percent(new_percent)
    }
    
    /// Decrease zoom level by a factor
    pub fn zoom_out(self, factor: f32) -> Self {
        let new_percent = (self.0 as f32 / factor) as u32;
        Self::from_percent(new_percent)
    }
}

impl fmt::Display for ZoomLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= 1000 {
            write!(f, "{}x", self.0 / 100)
        } else if self.0 >= 100 {
            write!(f, "{:.1}x", self.0 as f32 / 100.0)
        } else {
            write!(f, "{:.2}x", self.0 as f32 / 100.0)
        }
    }
}

/// Represents a viewport (visible time range) in the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub start: TimeNs,
    pub end: TimeNs,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(start: TimeNs, end: TimeNs) -> Self {
        Viewport { 
            start: start.min(end), 
            end: start.max(end) 
        }
    }
    
    /// Get the duration of this viewport
    pub fn duration(self) -> DurationNs {
        self.end.duration_since(self.start)
    }
    
    /// Check if a time point is within this viewport
    pub fn contains(self, time: TimeNs) -> bool {
        time >= self.start && time <= self.end
    }
    
    /// Get the center time of this viewport
    pub fn center(self) -> TimeNs {
        let duration = self.duration();
        self.start.add_duration(DurationNs(duration.0 / 2))
    }
    
    /// Create a new viewport centered on a time point with given duration
    pub fn centered_on(center: TimeNs, duration: DurationNs) -> Self {
        let half_duration = DurationNs(duration.0 / 2);
        Viewport::new(
            center.sub_duration(half_duration),
            center.add_duration(half_duration)
        )
    }
    
    /// Zoom this viewport to a new zoom level, centered on a point
    pub fn zoom_to(self, zoom_level: ZoomLevel, center: TimeNs) -> Self {
        let current_duration = self.duration();
        let new_duration = current_duration.div_f64(zoom_level.factor() as f64);
        Viewport::centered_on(center, new_duration)
    }
    
    /// Pan this viewport by a duration (positive = pan right, negative = pan left)
    pub fn pan(self, offset: DurationNs) -> Self {
        Viewport::new(
            self.start.add_duration(offset),
            self.end.add_duration(offset)
        )
    }
    
    /// Expand this viewport by adding buffer on both sides (for caching)
    pub fn with_buffer(self, buffer_percent: f32) -> Self {
        let duration = self.duration();
        let buffer = duration.mul_f64(buffer_percent as f64);
        Viewport::new(
            self.start.sub_duration(buffer),
            self.end.add_duration(buffer)
        )
    }
}

impl fmt::Display for Viewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} → {}", self.start, self.end)
    }
}

/// Coordinate conversion utilities for timeline rendering
pub mod coordinates {
    use super::*;
    
    /// Convert mouse X coordinate to timeline time with safe integer arithmetic
    #[allow(dead_code)]
    pub fn mouse_to_time_ns(mouse_x: f32, canvas_width: f32, viewport: Viewport) -> TimeNs {
        if canvas_width <= 0.0 {
            return viewport.start;
        }
        
        let viewport_duration = viewport.duration();
        let normalized_x = (mouse_x / canvas_width).clamp(0.0, 1.0);
        let offset_ns = (viewport_duration.nanos() as f64 * normalized_x as f64) as u64;
        viewport.start.add_duration(DurationNs(offset_ns))
    }
    
    /// Convert timeline time to pixel X coordinate (for rendering)
    #[allow(dead_code)]
    pub fn time_to_pixel(time_ns: TimeNs, canvas_width: f32, viewport: Viewport) -> f32 {
        let viewport_duration = viewport.duration();
        if viewport_duration.nanos() == 0 {
            return 0.0;
        }
        
        let time_offset = time_ns.duration_since(viewport.start);
        (time_offset.nanos() as f64 / viewport_duration.nanos() as f64 * canvas_width as f64) as f32
    }
    
    /// Get viewport from old floating point range (for migration compatibility)
    #[allow(dead_code)]
    pub fn viewport_from_f64_range(start: f64, end: f64) -> Viewport {
        Viewport::new(
            TimeNs::from_seconds(start.max(0.0)),
            TimeNs::from_seconds(end.max(start))
        )
    }
    
    /// Convert viewport to floating point range (for rendering compatibility)
    #[allow(dead_code)]
    pub fn viewport_to_f64_range(viewport: Viewport) -> (f64, f64) {
        (viewport.start.to_seconds(), viewport.end.to_seconds())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_ns_creation() {
        let time1 = TimeNs::from_seconds(1.5);
        assert_eq!(time1.nanos(), 1_500_000_000);
        
        let time2 = TimeNs::from_nanos(2_000_000_000);
        assert_eq!(time2.to_seconds(), 2.0);
    }
    
    #[test]
    fn test_duration_arithmetic() {
        let time1 = TimeNs::from_seconds(1.0);
        let time2 = TimeNs::from_seconds(3.0);
        
        let duration = time2.duration_since(time1);
        assert_eq!(duration.to_seconds(), 2.0);
    }
    
    #[test]
    fn test_zoom_level() {
        let zoom = ZoomLevel::from_factor(2.0);
        assert_eq!(zoom.percent(), 200);
        assert_eq!(zoom.factor(), 2.0);
    }
    
    #[test]
    fn test_viewport_operations() {
        let viewport = Viewport::new(
            TimeNs::from_seconds(1.0),
            TimeNs::from_seconds(3.0)
        );
        
        assert_eq!(viewport.duration().to_seconds(), 2.0);
        assert!(viewport.contains(TimeNs::from_seconds(2.0)));
        assert!(!viewport.contains(TimeNs::from_seconds(4.0)));
        
        let center = viewport.center();
        assert_eq!(center.to_seconds(), 2.0);
    }
    
    #[test]
    fn test_saturating_arithmetic() {
        let time = TimeNs::ZERO;
        let duration = DurationNs::from_seconds(1.0);
        
        // Should not underflow
        let result = time.sub_duration(duration);
        assert_eq!(result, TimeNs::ZERO);
    }
    
    #[test]
    fn test_coordinate_conversion() {
        use super::coordinates::*;
        
        let viewport = Viewport::new(
            TimeNs::from_seconds(0.0),
            TimeNs::from_seconds(10.0)
        );
        
        // Test mouse to time conversion
        let time_ns = mouse_to_time_ns(50.0, 100.0, viewport); // 50% across canvas
        assert_eq!(time_ns.to_seconds(), 5.0); // Should be 5 seconds
        
        // Test time to pixel conversion
        let pixel_x = time_to_pixel(TimeNs::from_seconds(2.5), 100.0, viewport);
        assert_eq!(pixel_x, 25.0); // 25% of 100 pixels = 25 pixels
    }
    
    #[test]
    fn test_viewport_conversion() {
        use super::coordinates::*;
        
        let viewport = viewport_from_f64_range(1.5, 3.5);
        let (start, end) = viewport_to_f64_range(viewport);
        
        assert_eq!(start, 1.5);
        assert_eq!(end, 3.5);
    }
}

// ===== UNIFIED TIMELINE CACHE =====

/// Unified cache for all timeline data - replaces 4 separate cache systems
/// 
/// This combines:
/// - Viewport data (decimated transitions for rendering)
/// - Cursor values (point-in-time signal values)
/// - Raw transition data (full precision for calculations)
/// - Request deduplication and performance statistics
#[derive(Clone, Debug)]
pub struct TimelineCache {
    /// Viewport data for rendering - decimated transitions within current viewport
    pub viewport_data: std::collections::HashMap<String, ViewportSignalData>,
    
    /// Cursor values at current timeline position
    pub cursor_values: std::collections::HashMap<String, shared::SignalValue>,
    
    /// Full raw transition data for precise calculations (indexed by signal_id)
    pub raw_transitions: std::collections::HashMap<String, Vec<shared::SignalTransition>>,
    
    /// Request tracking for deduplication
    pub active_requests: std::collections::HashMap<String, CacheRequestState>,
    
    /// Cache metadata and performance statistics
    pub metadata: CacheMetadata,
}

/// Signal data optimized for viewport rendering
#[derive(Clone, Debug)]
pub struct ViewportSignalData {
    /// Decimated transitions for current viewport (optimized for rendering)
    pub transitions: Vec<shared::SignalTransition>,
    /// Viewport this data covers
    pub viewport: Viewport,
    /// When this data was last updated
    pub last_updated_ns: TimeNs,
    /// Total number of transitions in source data (before decimation)
    pub total_source_transitions: usize,
}

/// Request state for cache deduplication
#[derive(Clone, Debug)]
pub struct CacheRequestState {
    pub request_id: String,
    pub requested_signals: Vec<String>,
    pub cursor_time: Option<TimeNs>,
    pub viewport: Option<Viewport>,
    pub timestamp_ns: TimeNs,
    pub request_type: CacheRequestType,
}

/// Types of requests to the cache system
#[derive(Clone, Debug, PartialEq)]
pub enum CacheRequestType {
    /// Request viewport data (decimated for rendering)
    ViewportData,
    /// Request cursor values (point-in-time)
    CursorValues,
    /// Request raw transition data (full precision)
    RawTransitions,
}

/// Cache performance and validity metadata
#[derive(Clone, Debug)]
pub struct CacheMetadata {
    /// Current viewport covered by cache
    pub current_viewport: Viewport,
    /// Current cursor position
    pub current_cursor: TimeNs,
    /// Cache hit/miss statistics
    pub statistics: shared::SignalStatistics,
    /// When cache was last invalidated
    pub last_invalidation_ns: TimeNs,
    /// Cache validity flags
    pub validity: CacheValidity,
}

/// Cache validity tracking
#[derive(Clone, Debug)]
pub struct CacheValidity {
    /// Is viewport data valid for current view?
    pub viewport_valid: bool,
    /// Are cursor values valid for current position?
    pub cursor_valid: bool,
    /// Are raw transitions complete and up-to-date?
    pub raw_transitions_valid: bool,
}

impl TimelineCache {
    /// Create a new empty timeline cache
    pub fn new() -> Self {
        TimelineCache {
            viewport_data: std::collections::HashMap::new(),
            cursor_values: std::collections::HashMap::new(),
            raw_transitions: std::collections::HashMap::new(),
            active_requests: std::collections::HashMap::new(),
            metadata: CacheMetadata {
                current_viewport: Viewport::new(TimeNs::ZERO, TimeNs::from_seconds(100.0)),
                current_cursor: TimeNs::ZERO,
                statistics: shared::SignalStatistics {
                    total_signals: 0,
                    cached_signals: 0,
                    query_time_ms: 0,
                    cache_hit_ratio: 0.0,
                },
                last_invalidation_ns: TimeNs::ZERO,
                validity: CacheValidity {
                    viewport_valid: false,
                    cursor_valid: false,
                    raw_transitions_valid: false,
                },
            },
        }
    }
    
    /// Invalidate cache when viewport changes significantly
    pub fn invalidate_viewport(&mut self, new_viewport: Viewport) {
        // Check if viewport changed significantly (>20% difference)
        let current_duration = self.metadata.current_viewport.duration();
        let new_duration = new_viewport.duration();
        let viewport_changed = if current_duration.nanos() > 0 {
            let duration_ratio = (new_duration.nanos() as f64) / (current_duration.nanos() as f64);
            duration_ratio < 0.8 || duration_ratio > 1.2
        } else {
            true // First time or invalid current viewport
        };
        
        if viewport_changed || !new_viewport.contains(self.metadata.current_viewport.center()) {
            self.viewport_data.clear();
            self.metadata.validity.viewport_valid = false;
            self.metadata.current_viewport = new_viewport;
            self.metadata.last_invalidation_ns = TimeNs::from_seconds(js_sys::Date::now() / 1000.0);
        }
    }
    
    /// Invalidate cursor cache when cursor moves significantly
    pub fn invalidate_cursor(&mut self, new_cursor: TimeNs) {
        // Invalidate if cursor moved outside current viewport or >1% of viewport duration
        let viewport_duration = self.metadata.current_viewport.duration();
        let cursor_threshold = DurationNs::from_nanos(viewport_duration.nanos() / 100); // 1% of viewport
        
        let cursor_changed = self.metadata.current_cursor.duration_since(new_cursor) > cursor_threshold ||
                           new_cursor.duration_since(self.metadata.current_cursor) > cursor_threshold;
        
        if cursor_changed || !self.metadata.current_viewport.contains(new_cursor) {
            self.cursor_values.clear();
            self.metadata.validity.cursor_valid = false;
            self.metadata.current_cursor = new_cursor;
        }
    }
    
    /// Get viewport signal data for rendering
    pub fn get_viewport_data(&self, signal_id: &str) -> Option<&ViewportSignalData> {
        self.viewport_data.get(signal_id)
    }
    
    /// Get cursor value at current timeline position
    pub fn get_cursor_value(&self, signal_id: &str) -> Option<&shared::SignalValue> {
        self.cursor_values.get(signal_id)
    }
    
    /// Get raw transition data for calculations
    pub fn get_raw_transitions(&self, signal_id: &str) -> Option<&Vec<shared::SignalTransition>> {
        self.raw_transitions.get(signal_id)
    }
    
    /// Check if request would be duplicate
    pub fn is_duplicate_request(&self, signal_ids: &[String], request_type: CacheRequestType) -> bool {
        let now = TimeNs::from_seconds(js_sys::Date::now() / 1000.0);
        let dedup_threshold = DurationNs::from_seconds(0.5); // 500ms deduplication window
        
        self.active_requests.values().any(|request| {
            request.request_type == request_type &&
            now.duration_since(request.timestamp_ns) < dedup_threshold &&
            signal_ids.iter().any(|id| request.requested_signals.contains(id))
        })
    }
}

impl Default for TimelineCache {
    fn default() -> Self {
        Self::new()
    }
}