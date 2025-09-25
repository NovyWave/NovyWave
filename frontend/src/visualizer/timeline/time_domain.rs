//! Time domain for timeline time representations and calculations
//!
//! Complete domain containing all time-related types and their operations:
//! TimeNs, DurationNs, TimePerPixel, Viewport, TimelineCoordinates

use std::fmt;
use std::ops::{Add, Sub};

// Time conversion constants
pub const NS_PER_SECOND: f64 = 1_000_000_000.0;
pub const NS_PER_MILLISECOND: f64 = 1_000_000.0;
pub const NS_PER_MICROSECOND: f64 = 1_000.0;

pub const DEFAULT_TIMELINE_RANGE_NS: u64 = 1_000_000_000;
pub const PS_PER_NS: u64 = 1_000;
pub const PS_PER_US: u64 = 1_000_000;
pub const PS_PER_MS: u64 = 1_000_000_000;
pub const PS_PER_SECOND: u64 = 1_000_000_000_000;
pub const MIN_CURSOR_STEP_NS: u64 = 1_000_000;
pub const MAX_CURSOR_STEP_NS: u64 = 1_000_000_000;

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
pub struct TimePerPixel {
    picoseconds: u64,
}

impl TimePerPixel {
    pub const MAX_ZOOM_IN: TimePerPixel = TimePerPixel { picoseconds: 1 };
    pub const MEDIUM_ZOOM: TimePerPixel = TimePerPixel {
        picoseconds: MIN_CURSOR_STEP_NS * PS_PER_NS,
    };

    pub fn picoseconds(self) -> u64 {
        self.picoseconds
    }

    pub fn from_picoseconds(ps: u64) -> Self {
        Self {
            picoseconds: ps.max(1),
        }
    }

    pub fn from_duration_and_width(duration_ns: u64, width_px: u32) -> Self {
        let duration_ps = (duration_ns as u128) * (PS_PER_NS as u128);
        let width = width_px.max(1) as u128;
        let ps_per_pixel = (duration_ps / width).max(1) as u64;
        Self::from_picoseconds(ps_per_pixel)
    }

    fn format_axis_value(value: f64) -> String {
        let mut formatted = if value.abs() >= 100.0 {
            format!("{:.0}", value.round())
        } else if value.abs() >= 10.0 {
            format!("{:.1}", value)
        } else if value.abs() >= 1.0 {
            format!("{:.2}", value)
        } else {
            format!("{:.3}", value)
        };

        if let Some(pos) = formatted.find('.') {
            while formatted.ends_with('0') {
                formatted.pop();
            }
            if formatted.len() > pos && formatted.ends_with('.') {
                formatted.pop();
            }
        }

        formatted
    }
}

impl fmt::Display for TimePerPixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ps = self.picoseconds;
        if ps >= PS_PER_SECOND {
            if ps % PS_PER_SECOND == 0 {
                write!(f, "{}s/px", ps / PS_PER_SECOND)
            } else {
                let value = ps as f64 / PS_PER_SECOND as f64;
                write!(f, "{}s/px", Self::format_axis_value(value))
            }
        } else if ps >= PS_PER_MS {
            if ps % PS_PER_MS == 0 {
                write!(f, "{}ms/px", ps / PS_PER_MS)
            } else {
                let value = ps as f64 / PS_PER_MS as f64;
                write!(f, "{}ms/px", Self::format_axis_value(value))
            }
        } else if ps >= PS_PER_US {
            if ps % PS_PER_US == 0 {
                write!(f, "{}us/px", ps / PS_PER_US)
            } else {
                let value = ps as f64 / PS_PER_US as f64;
                write!(f, "{}us/px", Self::format_axis_value(value))
            }
        } else if ps >= PS_PER_NS {
            if ps % PS_PER_NS == 0 {
                write!(f, "{}ns/px", ps / PS_PER_NS)
            } else {
                let value = ps as f64 / PS_PER_NS as f64;
                write!(f, "{}ns/px", Self::format_axis_value(value))
            }
        } else {
            write!(f, "{}ps/px", ps)
        }
    }
}

impl Default for TimePerPixel {
    fn default() -> Self {
        TimePerPixel::MEDIUM_ZOOM
    }
}

#[cfg(test)]
mod tests {
    use super::{PS_PER_NS, TimePerPixel};

    #[test]
    fn displays_nanoseconds_per_pixel() {
        assert_eq!(
            TimePerPixel::from_picoseconds(PS_PER_NS).to_string(),
            "1ns/px"
        );
    }

    #[test]
    fn displays_microseconds_per_pixel() {
        assert_eq!(
            TimePerPixel::from_picoseconds(50_000 * PS_PER_NS).to_string(),
            "50us/px"
        );
    }

    #[test]
    fn displays_seconds_per_pixel() {
        assert_eq!(
            TimePerPixel::from_picoseconds(2_000_000_000 * PS_PER_NS).to_string(),
            "2s/px"
        );
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
    pub time_per_pixel: TimePerPixel,
    pub canvas_width_pixels: u32,
}

impl TimelineCoordinates {
    pub fn new(
        cursor_ns: TimeNs,
        viewport_start_ns: TimeNs,
        time_per_pixel: TimePerPixel,
        canvas_width_pixels: u32,
    ) -> Self {
        TimelineCoordinates {
            cursor_ns,
            viewport_start_ns,
            time_per_pixel,
            canvas_width_pixels,
        }
    }
}

impl Default for TimelineCoordinates {
    fn default() -> Self {
        Self {
            cursor_ns: TimeNs::ZERO,
            viewport_start_ns: TimeNs::ZERO,
            time_per_pixel: TimePerPixel::default(),
            canvas_width_pixels: 640,
        }
    }
}
