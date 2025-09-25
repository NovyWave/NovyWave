//! Time domain for timeline time representations and calculations
//!
//! Complete domain containing all time-related types and their operations:
//! TimeNs, DurationNs, NsPerPixel, Viewport, TimelineCoordinates

use std::fmt;
use std::ops::{Add, Sub};

// Time conversion constants
pub const NS_PER_SECOND: f64 = 1_000_000_000.0;
pub const NS_PER_MILLISECOND: f64 = 1_000_000.0;
pub const NS_PER_MICROSECOND: f64 = 1_000.0;

pub const DEFAULT_TIMELINE_RANGE_NS: u64 = 1_000_000_000;
pub const MAX_ZOOM_NS_PER_PIXEL: u64 = 10_000_000_000;
pub const MIN_ZOOM_NS_PER_PIXEL: u64 = 1_000;
pub const MIN_CURSOR_STEP_NS: u64 = 1_000_000;
pub const MAX_CURSOR_STEP_NS: u64 = 1_000_000_000;
pub const MS_DISPLAY_THRESHOLD_NS: u64 = 100_000;
pub const US_DISPLAY_THRESHOLD_NS: u64 = 1_000;

/// Represents a point in time as nanoseconds since the start of a waveform file.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TimeNs(pub u64);

impl TimeNs {
    pub const ZERO: TimeNs = TimeNs(0);

    pub fn from_nanos(nanos: u64) -> Self {
        TimeNs(nanos)
    }

    pub fn from_external_seconds(seconds: f64) -> Self {
        TimeNs((seconds * NS_PER_SECOND) as u64)
    }

    pub fn nanos(self) -> u64 {
        self.0
    }

    pub fn display_seconds(self) -> f64 {
        self.0 as f64 / NS_PER_SECOND
    }

    pub fn display_millis(self) -> f64 {
        self.0 as f64 / NS_PER_MILLISECOND
    }

    pub fn display_micros(self) -> f64 {
        self.0 as f64 / NS_PER_MICROSECOND
    }

    pub fn duration_since(self, earlier: TimeNs) -> DurationNs {
        DurationNs(self.0.saturating_sub(earlier.0))
    }

    pub fn add_duration(self, duration: DurationNs) -> TimeNs {
        TimeNs(self.0.saturating_add(duration.0))
    }

    pub fn sub_duration(self, duration: DurationNs) -> TimeNs {
        TimeNs(self.0.saturating_sub(duration.0))
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationNs(pub u64);

impl DurationNs {
    pub fn from_nanos(nanos: u64) -> Self {
        DurationNs(nanos)
    }

    pub fn from_external_seconds(seconds: f64) -> Self {
        DurationNs((seconds * NS_PER_SECOND) as u64)
    }

    pub fn nanos(self) -> u64 {
        self.0
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NsPerPixel(pub u64);

impl NsPerPixel {
    pub const MAX_ZOOM_IN: NsPerPixel = NsPerPixel(1);
    pub const MEDIUM_ZOOM: NsPerPixel = NsPerPixel(MIN_CURSOR_STEP_NS);

    pub fn nanos(self) -> u64 {
        self.0
    }

    pub fn zoom_in_smooth(self, factor: f64) -> Self {
        let new_ns_per_pixel = ((self.0 as f64) * (1.0 - factor.clamp(0.0, 0.9))).max(1.0) as u64;
        NsPerPixel(new_ns_per_pixel.max(1))
    }

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
        NsPerPixel::MEDIUM_ZOOM
    }
}

#[cfg(test)]
mod tests {
    use super::NsPerPixel;

    #[test]
    fn displays_nanoseconds_per_pixel() {
        assert_eq!(NsPerPixel(1).to_string(), "1ns/px");
    }

    #[test]
    fn displays_microseconds_per_pixel() {
        assert_eq!(NsPerPixel(50_000).to_string(), "50.0μs/px");
    }

    #[test]
    fn displays_seconds_per_pixel() {
        assert_eq!(NsPerPixel(2_000_000_000).to_string(), "2.0s/px");
    }
}

/// Represents a viewport (visible time range) in the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub start: TimeNs,
    pub end: TimeNs,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            start: TimeNs::ZERO,
            end: TimeNs::ZERO,
        }
    }
}

impl Viewport {
    pub fn new(start: TimeNs, end: TimeNs) -> Self {
        Viewport {
            start: start.min(end),
            end: start.max(end),
        }
    }

    pub fn duration(self) -> DurationNs {
        self.end.duration_since(self.start)
    }

    pub fn contains(self, time: TimeNs) -> bool {
        time >= self.start && time <= self.end
    }

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

/// Timeline coordinate system for all timeline operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineCoordinates {
    pub cursor_ns: TimeNs,
    pub viewport_start_ns: TimeNs,
    pub ns_per_pixel: NsPerPixel,
    pub canvas_width_pixels: u32,
}

impl TimelineCoordinates {
    pub fn new(
        cursor_ns: TimeNs,
        viewport_start_ns: TimeNs,
        ns_per_pixel: NsPerPixel,
        canvas_width_pixels: u32,
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
            canvas_width_pixels: 640,
        }
    }
}
