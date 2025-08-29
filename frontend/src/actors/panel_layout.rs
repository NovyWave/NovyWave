#![allow(dead_code)] // Transitional implementation - full Actor+Relay migration pending

use zoon::*;
use crate::actors::{Relay, relay};

/// Panel Layout Domain - Simple Mutable-based state for compilation compatibility

// Files panel dimensions
static FILES_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(470));
static FILES_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(300));
static VERTICAL_DRAGGING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static HORIZONTAL_DRAGGING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Variables table column dimensions  
static NAME_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(180));
static VALUE_COLUMN_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(100));
static NAME_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));
static VALUE_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// Dock mode and transitioning
static DOCKED_TO_BOTTOM: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(true));
static DOCK_TRANSITIONING: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(false));

// ===== SIGNAL ACCESS FUNCTIONS =====

/// Get files panel width signal
pub fn files_width_signal() -> impl Signal<Item = u32> {
    FILES_WIDTH.signal()
}

/// Get files panel height signal
pub fn files_height_signal() -> impl Signal<Item = u32> {
    FILES_HEIGHT.signal()
}

/// Get vertical dragging state signal
pub fn vertical_dragging_signal() -> impl Signal<Item = bool> {
    VERTICAL_DRAGGING.signal()
}

/// Get horizontal dragging state signal  
pub fn horizontal_dragging_signal() -> impl Signal<Item = bool> {
    HORIZONTAL_DRAGGING.signal()
}

/// Get name column width signal
pub fn name_column_width_signal() -> impl Signal<Item = u32> {
    NAME_COLUMN_WIDTH.signal()
}

/// Get value column width signal
pub fn value_column_width_signal() -> impl Signal<Item = u32> {
    VALUE_COLUMN_WIDTH.signal()
}

/// Get name divider dragging state signal
pub fn name_divider_dragging_signal() -> impl Signal<Item = bool> {
    NAME_DIVIDER_DRAGGING.signal()
}

/// Get value divider dragging state signal
pub fn value_divider_dragging_signal() -> impl Signal<Item = bool> {
    VALUE_DIVIDER_DRAGGING.signal()
}

/// Get dock mode signal (docked to bottom)
pub fn docked_to_bottom_signal() -> impl Signal<Item = bool> {
    DOCKED_TO_BOTTOM.signal()
}

/// Get dock transitioning state signal
pub fn dock_transitioning_signal() -> impl Signal<Item = bool> {
    DOCK_TRANSITIONING.signal()
}

// ===== RELAY FUNCTIONS =====

/// Files width changed relay
pub fn files_width_changed_relay() -> Relay<u32> {
    let (relay, _stream) = relay();
    relay
}

/// Files height changed relay
pub fn files_height_changed_relay() -> Relay<u32> {
    let (relay, _stream) = relay();
    relay
}

/// Vertical divider dragged relay
pub fn vertical_divider_dragged_relay() -> Relay<f32> {
    let (relay, _stream) = relay();
    relay
}

/// Horizontal divider dragged relay
pub fn horizontal_divider_dragged_relay() -> Relay<f32> {
    let (relay, _stream) = relay();
    relay
}

/// Name divider dragged relay
pub fn name_divider_dragged_relay() -> Relay<f32> {
    let (relay, _stream) = relay();
    relay
}

/// Value divider dragged relay
pub fn value_divider_dragged_relay() -> Relay<f32> {
    let (relay, _stream) = relay();
    relay
}

/// Dock mode changed relay
pub fn dock_mode_changed_relay() -> Relay<bool> {
    let (relay, _stream) = relay();
    relay
}

/// Dock transition started relay
pub fn dock_transition_started_relay() -> Relay<()> {
    let (relay, _stream) = relay();
    relay
}

/// Dock transition ended relay
pub fn dock_transition_ended_relay() -> Relay<()> {
    let (relay, _stream) = relay();
    relay
}

/// Mouse moved relay
pub fn mouse_moved_relay() -> Relay<(f32, f32)> {
    let (relay, _stream) = relay();
    relay
}

// ===== HELPER FUNCTIONS =====

/// Set files width (convenience function - for now just a placeholder)
pub fn set_files_width(_width: u32) {
    // Placeholder implementation
}

/// Set files height (convenience function - for now just a placeholder)
pub fn set_files_height(_height: u32) {
    // Placeholder implementation
}

/// Set dock mode (convenience function - for now just a placeholder)
pub fn set_dock_mode(_docked_to_bottom: bool) {
    // Placeholder implementation
}

// ===== INITIALIZATION =====

/// Initialize the panel layout domain
pub fn initialize() {
    // Domain is automatically initialized when first accessed via Lazy
    // Static mutables are initialized on first access
}