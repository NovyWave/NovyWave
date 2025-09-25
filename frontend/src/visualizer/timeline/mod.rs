//! Timeline domain entry point.
//!
//! Exposes the core timeline actor together with time-domain utilities and
//! supporting range computations.

pub mod maximum_timeline_range;
pub mod time_domain;
pub mod timeline_actor;

pub use maximum_timeline_range::MaximumTimelineRange;
pub use time_domain::{DurationNs, TimeNs, TimePerPixel, TimelineCoordinates, Viewport};
pub use timeline_actor::{TimelineRenderState, TimelineVariableSeries, WaveformTimeline};
