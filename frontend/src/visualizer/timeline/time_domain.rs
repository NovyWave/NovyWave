//! Time domain for timeline time representations and calculations
//!
//! Complete domain containing all time-related types and their operations:
//! TimePs, DurationPs, TimePerPixel, Viewport, TimelineCoordinates

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::{Add, Sub};

// Time conversion constants expressed in picoseconds
pub const PS_PER_NS: u64 = 1_000;
pub const PS_PER_US: u64 = 1_000_000;
pub const PS_PER_MS: u64 = 1_000_000_000;
pub const PS_PER_SECOND: u64 = 1_000_000_000_000;
pub const FS_PER_PS: u64 = 1_000;
pub const AS_PER_PS: u64 = 1_000_000;

pub const MIN_CURSOR_STEP_NS: u64 = 1_000_000;

const PS_PER_SECOND_F64: f64 = PS_PER_SECOND as f64;
const PS_PER_MS_F64: f64 = PS_PER_MS as f64;
const PS_PER_US_F64: f64 = PS_PER_US as f64;
const PS_PER_NS_F64: f64 = PS_PER_NS as f64;
const FS_PER_PS_F64: f64 = FS_PER_PS as f64;
const AS_PER_PS_F64: f64 = AS_PER_PS as f64;

/// Represents a point in time as picoseconds since the start of a waveform file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimePs(pub u64);

impl TimePs {
    pub const ZERO: TimePs = TimePs(0);

    pub fn from_nanos(nanos: u64) -> Self {
        TimePs(nanos.saturating_mul(PS_PER_NS))
    }

    pub fn from_picoseconds(picoseconds: u64) -> Self {
        TimePs(picoseconds)
    }

    pub fn from_external_seconds(seconds: f64) -> Self {
        if !seconds.is_finite() {
            return TimePs(u64::MAX);
        }
        let ps = (seconds * PS_PER_SECOND_F64).round();
        if ps.is_nan() {
            return TimePs(0);
        }
        let clamped = ps.max(0.0).min(u64::MAX as f64);
        TimePs(clamped as u64)
    }

    pub fn nanos(self) -> u64 {
        self.0 / PS_PER_NS
    }

    pub fn picoseconds(self) -> u64 {
        self.0
    }

    pub fn display_seconds(self) -> f64 {
        self.0 as f64 / PS_PER_SECOND_F64
    }

    pub fn display_millis(self) -> f64 {
        self.0 as f64 / PS_PER_MS_F64
    }

    pub fn display_micros(self) -> f64 {
        self.0 as f64 / PS_PER_US_F64
    }

    pub fn display_nanos(self) -> f64 {
        self.0 as f64 / PS_PER_NS_F64
    }

    pub fn duration_since(self, earlier: TimePs) -> DurationPs {
        DurationPs(self.0.saturating_sub(earlier.0))
    }

    pub fn add_duration(self, duration: DurationPs) -> TimePs {
        TimePs(self.0.saturating_add(duration.0))
    }

    pub fn sub_duration(self, duration: DurationPs) -> TimePs {
        TimePs(self.0.saturating_sub(duration.0))
    }
}

impl fmt::Display for TimePs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.display_seconds();
        if seconds >= 1.0 {
            write!(f, "{:.3}s", seconds)
        } else if seconds >= 0.001 {
            write!(f, "{:.3}ms", self.display_millis())
        } else if seconds >= 0.000001 {
            write!(f, "{:.3}μs", self.display_micros())
        } else if seconds >= 0.000000001 {
            write!(f, "{:.3}ns", self.display_nanos())
        } else {
            write!(f, "{}ps", self.0)
        }
    }
}

impl Serialize for TimePs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.nanos())
    }
}

impl<'de> Deserialize<'de> for TimePs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos = u64::deserialize(deserializer)?;
        Ok(TimePs::from_nanos(nanos))
    }
}

/// Represents a duration in picoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DurationPs(pub u64);

impl DurationPs {
    pub fn from_nanos(nanos: u64) -> Self {
        DurationPs(nanos.saturating_mul(PS_PER_NS))
    }

    pub fn from_picoseconds(picoseconds: u64) -> Self {
        DurationPs(picoseconds)
    }

    pub fn from_external_seconds(seconds: f64) -> Self {
        if !seconds.is_finite() {
            return DurationPs(u64::MAX);
        }
        let ps = (seconds * PS_PER_SECOND_F64).round();
        if ps.is_nan() {
            return DurationPs(0);
        }
        let clamped = ps.max(0.0).min(u64::MAX as f64);
        DurationPs(clamped as u64)
    }

    pub fn nanos(self) -> u64 {
        self.0 / PS_PER_NS
    }

    pub fn picoseconds(self) -> u64 {
        self.0
    }

    pub fn display_seconds(self) -> f64 {
        self.0 as f64 / PS_PER_SECOND_F64
    }
}

impl fmt::Display for DurationPs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.display_seconds();
        if seconds >= 1.0 {
            write!(f, "{:.3}s", seconds)
        } else if seconds >= 0.001 {
            write!(f, "{:.3}ms", self.0 as f64 / PS_PER_MS_F64)
        } else if seconds >= 0.000001 {
            write!(f, "{:.3}μs", self.0 as f64 / PS_PER_US_F64)
        } else if seconds >= 0.000000001 {
            write!(f, "{:.3}ns", self.0 as f64 / PS_PER_NS_F64)
        } else {
            write!(f, "{}ps", self.0)
        }
    }
}

impl Add for DurationPs {
    type Output = DurationPs;

    fn add(self, rhs: DurationPs) -> DurationPs {
        DurationPs(self.0.saturating_add(rhs.0))
    }
}

impl Sub for DurationPs {
    type Output = DurationPs;

    fn sub(self, rhs: DurationPs) -> DurationPs {
        DurationPs(self.0.saturating_sub(rhs.0))
    }
}

/// Represents timeline resolution as picoseconds per pixel.
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

    pub fn from_duration_and_width(duration_ps: u64, width_px: u32) -> Self {
        let duration = duration_ps.max(1) as u128;
        let width = width_px.max(1) as u128;
        let ps_per_pixel = (duration / width).max(1) as u64;
        Self::from_picoseconds(ps_per_pixel)
    }

    fn format_axis_value(value: f64) -> String {
        let abs = value.abs();
        let mut formatted = if abs >= 100.0 {
            format!("{:.0}", value.round())
        } else if abs >= 10.0 {
            format!("{:.1}", value)
        } else if abs >= 1.0 {
            format!("{:.2}", value)
        } else if abs >= 0.1 {
            format!("{:.3}", value)
        } else if abs >= 0.01 {
            format!("{:.4}", value)
        } else if abs >= 0.001 {
            format!("{:.5}", value)
        } else if abs >= 0.0001 {
            format!("{:.6}", value)
        } else {
            return format!("{:.2e}", value);
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

    pub fn formatted_from_duration_and_width(duration_ps: u64, width_px: u32) -> String {
        if width_px == 0 {
            return "0ps/px".to_string();
        }
        let ps_per_pixel = duration_ps as f64 / width_px.max(1) as f64;
        Self::formatted_from_ps_per_pixel(ps_per_pixel)
    }

    fn formatted_from_ps_per_pixel(ps_per_pixel: f64) -> String {
        if !ps_per_pixel.is_finite() || ps_per_pixel <= 0.0 {
            return "0ps/px".to_string();
        }

        if ps_per_pixel >= PS_PER_SECOND_F64 {
            let value = ps_per_pixel / PS_PER_SECOND_F64;
            return format!("{}s/px", Self::format_axis_value(value));
        }
        if ps_per_pixel >= PS_PER_MS_F64 {
            let value = ps_per_pixel / PS_PER_MS_F64;
            return format!("{}ms/px", Self::format_axis_value(value));
        }
        if ps_per_pixel >= PS_PER_US_F64 {
            let value = ps_per_pixel / PS_PER_US_F64;
            return format!("{}us/px", Self::format_axis_value(value));
        }
        if ps_per_pixel >= PS_PER_NS_F64 {
            let value = ps_per_pixel / PS_PER_NS_F64;
            return format!("{}ns/px", Self::format_axis_value(value));
        }
        if ps_per_pixel >= 1.0 {
            return format!("{}ps/px", Self::format_axis_value(ps_per_pixel));
        }

        let fs_per_pixel = ps_per_pixel * FS_PER_PS_F64;
        if fs_per_pixel >= 1.0 {
            return format!("{}fs/px", Self::format_axis_value(fs_per_pixel));
        }

        let as_per_pixel = ps_per_pixel * AS_PER_PS_F64;
        format!("{}as/px", Self::format_axis_value(as_per_pixel))
    }
}

impl fmt::Display for TimePerPixel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ps = self.picoseconds;
        if ps >= PS_PER_SECOND {
            if ps % PS_PER_SECOND == 0 {
                write!(f, "{}s/px", ps / PS_PER_SECOND)
            } else {
                let value = ps as f64 / PS_PER_SECOND_F64;
                write!(f, "{}s/px", Self::format_axis_value(value))
            }
        } else if ps >= PS_PER_MS {
            if ps % PS_PER_MS == 0 {
                write!(f, "{}ms/px", ps / PS_PER_MS)
            } else {
                let value = ps as f64 / PS_PER_MS_F64;
                write!(f, "{}ms/px", Self::format_axis_value(value))
            }
        } else if ps >= PS_PER_US {
            if ps % PS_PER_US == 0 {
                write!(f, "{}us/px", ps / PS_PER_US)
            } else {
                let value = ps as f64 / PS_PER_US_F64;
                write!(f, "{}us/px", Self::format_axis_value(value))
            }
        } else if ps >= PS_PER_NS {
            if ps % PS_PER_NS == 0 {
                write!(f, "{}ns/px", ps / PS_PER_NS)
            } else {
                let value = ps as f64 / PS_PER_NS_F64;
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
    pub start: TimePs,
    pub end: TimePs,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            start: TimePs::ZERO,
            end: TimePs::ZERO,
        }
    }
}

impl Viewport {
    pub fn new(start: TimePs, end: TimePs) -> Self {
        Viewport {
            start: start.min(end),
            end: start.max(end),
        }
    }

    pub fn duration(self) -> DurationPs {
        self.end.duration_since(self.start)
    }

    pub fn contains(self, time: TimePs) -> bool {
        time >= self.start && time <= self.end
    }

    pub fn center(self) -> TimePs {
        let duration = self.duration();
        self.start
            .add_duration(DurationPs::from_picoseconds(duration.picoseconds() / 2))
    }
}

impl fmt::Display for Viewport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} → {}", self.start, self.end)
    }
}
