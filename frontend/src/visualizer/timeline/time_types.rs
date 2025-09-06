use std::fmt;
use std::ops::{Add, Sub};

// ===== TIME CONVERSION CONSTANTS =====
// Centralized constants to replace hardcoded magic numbers throughout timeline system

/// Nanoseconds per second conversion factor
pub const NS_PER_SECOND: f64 = 1_000_000_000.0;

/// Nanoseconds per millisecond conversion factor  
pub const NS_PER_MILLISECOND: f64 = 1_000_000.0;

/// Nanoseconds per microsecond conversion factor
pub const NS_PER_MICROSECOND: f64 = 1_000.0;


/// Default timeline range in nanoseconds (1 second)
pub const DEFAULT_TIMELINE_RANGE_NS: u64 = 1_000_000_000;

/// Maximum zoom level (10 seconds per pixel) in nanoseconds per pixel
pub const MAX_ZOOM_NS_PER_PIXEL: u64 = 10_000_000_000;

/// Minimum zoom level (1 microsecond per pixel) in nanoseconds per pixel
pub const MIN_ZOOM_NS_PER_PIXEL: u64 = 1_000;

/// Minimum cursor movement step (1 millisecond) in nanoseconds
pub const MIN_CURSOR_STEP_NS: u64 = 1_000_000;

/// Maximum cursor movement step (1 second) in nanoseconds  
pub const MAX_CURSOR_STEP_NS: u64 = 1_000_000_000;

/// Display threshold for milliseconds in formatting (100 microseconds)
pub const MS_DISPLAY_THRESHOLD_NS: u64 = 100_000;

/// Display threshold for microseconds in formatting (1 microsecond in nanoseconds)  
pub const US_DISPLAY_THRESHOLD_NS: u64 = 1_000;

/// Represents a point in time as nanoseconds since the start of a waveform file.
/// 
/// Uses u64 internally to provide:
/// - 1 nanosecond resolution
/// - ~584 years maximum duration (u64::MAX / 1_000_000_000 / 60 / 60 / 24 / 365)
/// - No floating point precision issues
/// - Fast integer arithmetic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct TimeNs(pub u64);

impl TimeNs {
    pub const ZERO: TimeNs = TimeNs(0);
    
    /// Create a new TimeNs from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        TimeNs(nanos)
    }
    
    /// Create TimeNs from seconds (for converting external f64 values only)
    /// This should only be used when converting from external sources like JS timestamps,
    /// animation positions, or file metadata - NOT for API boundaries
    pub fn from_external_seconds(seconds: f64) -> Self {
        TimeNs((seconds * NS_PER_SECOND) as u64)
    }
    
    
    /// Get nanoseconds value
    pub fn nanos(self) -> u64 {
        self.0
    }
    
    /// Convert to seconds for display purposes only
    /// This is only for human-readable output, NOT for API boundaries
    pub fn display_seconds(self) -> f64 {
        self.0 as f64 / NS_PER_SECOND
    }
    
    /// Convert to milliseconds for display purposes only
    pub fn display_millis(self) -> f64 {
        self.0 as f64 / NS_PER_MILLISECOND
    }
    
    /// Convert to microseconds for display purposes only
    pub fn display_micros(self) -> f64 {
        self.0 as f64 / NS_PER_MICROSECOND
    }
    
    /// Safely subtract two time points, returning a duration
    pub fn duration_since(self, earlier: TimeNs) -> DurationNs {
        DurationNs(self.0.saturating_sub(earlier.0))
    }
    
    /// Safely add a duration to this time point
    pub fn add_duration(self, duration: DurationNs) -> TimeNs {
        TimeNs(self.0.saturating_add(duration.0))
    }
    
}

impl fmt::Display for TimeNs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.display_seconds();
        if seconds >= 1.0 {
            write!(f, "{:.3}s", seconds)
        } else if seconds >= 0.001 {
            write!(f, "{:.3}ms", self.display_millis())
        } else if seconds >= 0.000001 {
            write!(f, "{:.3}μs", self.display_micros())
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
    /// Create a new DurationNs from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        DurationNs(nanos)
    }
    
    /// Create DurationNs from seconds (for converting external f64 values only)
    /// This should only be used when converting from external sources like JS timestamps,
    /// animation durations, or thresholds - NOT for API boundaries
    pub fn from_external_seconds(seconds: f64) -> Self {
        DurationNs((seconds * NS_PER_SECOND) as u64)
    }
    
    
    /// Get nanoseconds value
    pub fn nanos(self) -> u64 {
        self.0
    }
    
    /// Convert to seconds for display purposes only
    /// This is only for human-readable output, NOT for API boundaries
    pub fn display_seconds(self) -> f64 {
        self.0 as f64 / NS_PER_SECOND
    }
    
}

impl fmt::Display for DurationNs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.display_seconds();
        if seconds >= 1.0 {
            write!(f, "{:.3}s", seconds)
        } else if seconds >= 0.001 {
            write!(f, "{:.3}ms", self.0 as f64 / NS_PER_MILLISECOND)
        } else if seconds >= 0.000001 {
            write!(f, "{:.3}μs", self.0 as f64 / NS_PER_MICROSECOND)
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

/// Represents timeline resolution as nanoseconds per pixel.
/// 
/// This is the industry-standard approach used by:
/// - Oscilloscopes (time per division)
/// - Professional DAWs (samples per pixel) 
/// - Google Maps (units per tile/pixel)
///
/// Examples:
/// - NsPerPixel(1) = 1 nanosecond per pixel (maximum zoom in)
/// - NsPerPixel(1_000) = 1 microsecond per pixel (high zoom)
/// - NsPerPixel(1_000_000) = 1 millisecond per pixel (medium zoom)
/// - NsPerPixel(1_000_000_000) = 1 second per pixel (zoomed out)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NsPerPixel(pub u64);

impl NsPerPixel {
    pub const MAX_ZOOM_IN: NsPerPixel = NsPerPixel(1);      // 1 ns/pixel (finest resolution)
    pub const MEDIUM_ZOOM: NsPerPixel = NsPerPixel(MIN_CURSOR_STEP_NS); // 1 ms/pixel  
    
    
    /// Get nanoseconds per pixel value
    pub fn nanos(self) -> u64 {
        self.0
    }
    
    
    
    
    /// Zoom in by reducing nanoseconds per pixel (smooth zoom)
    pub fn zoom_in_smooth(self, factor: f64) -> Self {
        let new_ns_per_pixel = ((self.0 as f64) * (1.0 - factor.clamp(0.0, 0.9))).max(1.0) as u64;
        NsPerPixel(new_ns_per_pixel.max(1))
    }
    
    /// Zoom out by increasing nanoseconds per pixel (smooth zoom)  
    pub fn zoom_out_smooth(self, factor: f64) -> Self {
        let new_ns_per_pixel = ((self.0 as f64) * (1.0 + factor.clamp(0.0, 10.0))) as u64;
        NsPerPixel(new_ns_per_pixel)
    }
    
    
}

impl fmt::Display for NsPerPixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 >= DEFAULT_TIMELINE_RANGE_NS {
            write!(f, "{:.1}s/px", self.0 as f64 / NS_PER_SECOND)
        } else if self.0 >= MS_DISPLAY_THRESHOLD_NS {
            // Show ms/px for values >= 100 μs (0.1 ms) - more readable for longer timescales
            write!(f, "{:.1}ms/px", self.0 as f64 / NS_PER_MILLISECOND)
        } else if self.0 >= US_DISPLAY_THRESHOLD_NS {
            write!(f, "{:.1}μs/px", self.0 as f64 / NS_PER_MICROSECOND)
        } else {
            write!(f, "{}ns/px", self.0)
        }
    }
}

impl Default for NsPerPixel {
    fn default() -> Self {
        NsPerPixel::MEDIUM_ZOOM // 1 ms/pixel - reasonable default zoom
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
    
    
    
}

impl fmt::Display for Viewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} → {}", self.start, self.end)
    }
}

// ✅ NO DEFAULT IMPLEMENTATION - Only create viewport when we have actual VCD file data
// This prevents any fallback rendering until real timeline data is available

/// Coordinate conversion utilities for timeline rendering - PURE INTEGER VERSION

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_ns_creation() {
        let time1 = TimeNs::from_external_seconds(1.5);
        assert_eq!(time1.nanos(), (1.5 * NS_PER_SECOND) as u64);
        
        let time2 = TimeNs::from_nanos((2.0 * NS_PER_SECOND) as u64);
        assert_eq!(time2.display_seconds(), 2.0);
    }
    
    #[test]
    fn test_duration_arithmetic() {
        let time1 = TimeNs::from_external_seconds(1.0);
        let time2 = TimeNs::from_external_seconds(3.0);
        
        let duration = time2.duration_since(time1);
        assert_eq!(duration.display_seconds(), 2.0);
    }
    
    #[test]
    fn test_ns_per_pixel() {
        let resolution = NsPerPixel::from_nanos(1000);
        assert_eq!(resolution.nanos(), 1000);
        
        // Test nanoseconds value access
        assert_eq!(resolution.nanos() * 100, 100_000); // 100 pixels * 1000 ns/pixel
        
        // Test zoom in/out
        let zoomed_in = resolution.zoom_in_smooth(0.1); // 10% zoom in
        assert!(zoomed_in.is_more_detailed_than(resolution));
        
        let zoomed_out = resolution.zoom_out_smooth(0.1); // 10% zoom out  
        assert!(resolution.is_more_detailed_than(zoomed_out));
    }
    
    #[test]
    fn test_viewport_operations() {
        let viewport = Viewport::new(
            TimeNs::from_external_seconds(1.0),
            TimeNs::from_external_seconds(3.0)
        );
        
        assert_eq!(viewport.duration().display_seconds(), 2.0);
        assert!(viewport.contains(TimeNs::from_external_seconds(2.0)));
        assert!(!viewport.contains(TimeNs::from_external_seconds(4.0)));
        
        let center = viewport.center();
        assert_eq!(center.display_seconds(), 2.0);
    }
    
    #[test]
    fn test_saturating_arithmetic() {
        let time = TimeNs::ZERO;
        let duration = DurationNs::from_external_seconds(1.0);
        
        // Should not underflow
        let result = time.sub_duration(duration);
        assert_eq!(result, TimeNs::ZERO);
    }
    
    #[test]
    fn test_coordinate_conversion() {
        use super::coordinates::*;
        
        let viewport = Viewport::new(
            TimeNs::from_external_seconds(0.0),
            TimeNs::from_external_seconds(10.0)
        );
        
        // Test new pure integer coordinate conversion
        let ns_per_pixel = NsPerPixel::from_viewport(viewport, 100); // 100 pixels wide
        
        // Test mouse to time conversion (pure integer)
        let time_ns = mouse_to_time_ns(50, ns_per_pixel, viewport.start); // 50 pixels from start
        let expected_offset = 50 * ns_per_pixel.nanos();
        assert_eq!(time_ns.nanos(), expected_offset);
        
        // Test time to pixel conversion (pure integer)
        let pixel_x = time_to_pixel(TimeNs::from_external_seconds(2.5), ns_per_pixel, viewport.start);
        assert!(pixel_x.is_some()); // Should be valid conversion
    }
    
    #[test]
    fn test_viewport_conversion() {
        use super::coordinates::*;
        
        // Create viewport directly since viewport_from_f64_range doesn't exist
        let viewport = Viewport { start: TimeNs::from_external_seconds(1.5), end: TimeNs::from_external_seconds(3.5) };
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
}

/// Request state for cache deduplication
#[derive(Clone, Debug)]
pub struct CacheRequestState {
    pub requested_signals: Vec<String>,
    pub _viewport: Option<Viewport>,
    pub timestamp_ns: TimeNs,
    pub request_type: CacheRequestType,
}

/// Types of requests to the cache system
#[derive(Clone, Debug, PartialEq)]
pub enum CacheRequestType {
    /// Request cursor values (point-in-time)
    CursorValues,
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
                // 🎯 NON-INTERFERING DEFAULT: 10 seconds - large enough that fallback detection (<5s) won't trigger,
                // but won't override real file data ranges
                current_viewport: Viewport::new(TimeNs::ZERO, TimeNs::from_external_seconds(10.0)),
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
            self.metadata.last_invalidation_ns = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
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
        let now = TimeNs::from_external_seconds(js_sys::Date::now() / 1000.0);
        let dedup_threshold = DurationNs::from_external_seconds(0.5); // 500ms deduplication window
        
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

// ===== TIMELINE COORDINATES - PURE INTEGER SYSTEM =====

/// Universal coordinate system for all timeline operations using pure integer arithmetic
/// 
/// This replaces all floating-point timeline calculations with precise integer operations.
/// Used for mouse interactions, panning, zooming, and rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineCoordinates {
    pub cursor_ns: TimeNs,              // Current cursor position in nanoseconds
    pub viewport_start_ns: TimeNs,      // Viewport start time in nanoseconds  
    pub ns_per_pixel: NsPerPixel,       // Timeline resolution (replaces zoom level)
    pub canvas_width_pixels: u32,       // Canvas width in pixels
}

impl TimelineCoordinates {
    /// Create new timeline coordinates
    pub fn new(
        cursor_ns: TimeNs,
        viewport_start_ns: TimeNs, 
        ns_per_pixel: NsPerPixel,
        canvas_width_pixels: u32
    ) -> Self {
        TimelineCoordinates {
            cursor_ns,
            viewport_start_ns,
            ns_per_pixel,
            canvas_width_pixels,
        }
    }
    
    
    
}

impl Default for TimelineCoordinates {
    fn default() -> Self {
        Self {
            cursor_ns: TimeNs::ZERO,
            viewport_start_ns: TimeNs::ZERO,
            ns_per_pixel: NsPerPixel::default(),
            canvas_width_pixels: 640, // Reasonable default width (updated to actual when canvas loads)
        }
    }
}