// Canvas transitions module - functionality migrated to timeline_actor
//
// This module previously handled signal transition caching and request batching.
// All functionality has been migrated to proper Actor+Relay architecture in:
// - frontend/src/visualizer/timeline/timeline_actor.rs