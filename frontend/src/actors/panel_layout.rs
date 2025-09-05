//! Simplified panel layout signals - replaces complex Actor+Relay bridge system
//!
//! This provides compatibility functions that delegate to the working dragging system
//! and config system directly. The complex Actor+bridge system has been removed.

use zoon::*;
use shared::DockMode;

// === SIGNAL ACCESS FUNCTIONS ===

/// Get files panel width signal
pub fn files_panel_width_signal() -> impl Signal<Item = u32> {
    // Use the new dragging system signal that works correctly
    crate::visualizer::interaction::dragging::files_panel_width_signal().map(|w| w as u32)
}

/// Get files panel height signal  
pub fn files_panel_height_signal() -> impl Signal<Item = u32> {
    // Use the new dragging system signal that works correctly
    crate::visualizer::interaction::dragging::files_panel_height_signal().map(|h| h as u32)
}

/// Get variables name column width signal
pub fn variables_name_column_width_signal() -> impl Signal<Item = u32> {
    // Use the new dragging system signal that works correctly
    crate::visualizer::interaction::dragging::variables_name_column_width_signal().map(|w| w as u32)
}

/// Get variables value column width signal
pub fn variables_value_column_width_signal() -> impl Signal<Item = u32> {
    // Use the new dragging system signal that works correctly
    crate::visualizer::interaction::dragging::variables_value_column_width_signal().map(|w| w as u32)
}

/// Get timeline panel height signal
pub fn timeline_panel_height_signal() -> impl Signal<Item = u32> {
    // Return a default timeline height - timeline not yet implemented
    zoon::always(200u32)
}

/// Get docked to bottom signal (derived for backward compatibility)  
pub fn docked_to_bottom_signal() -> impl Signal<Item = bool> {
    // Use config system directly
    crate::config::app_config().dock_mode_actor.signal().map(|mode| matches!(mode, DockMode::Bottom))
}

/// Get dock transitioning signal
pub fn dock_transitioning_signal() -> impl Signal<Item = bool> {
    // Return false - dock transitions are instant now
    zoon::always(false)
}

/// Get files vertical dragging signal
pub fn files_vertical_dragging_signal() -> impl Signal<Item = bool> {
    // Use dragging system signal
    crate::visualizer::interaction::dragging::is_divider_dragging(crate::visualizer::interaction::dragging::DividerType::FilesPanelMain)
}

/// Get files horizontal dragging signal
pub fn files_horizontal_dragging_signal() -> impl Signal<Item = bool> {
    // Use dragging system signal
    crate::visualizer::interaction::dragging::is_divider_dragging(crate::visualizer::interaction::dragging::DividerType::FilesPanelSecondary)
}

/// Get name divider dragging signal
pub fn name_divider_dragging_signal() -> impl Signal<Item = bool> {
    // Use dragging system signal
    crate::visualizer::interaction::dragging::is_divider_dragging(crate::visualizer::interaction::dragging::DividerType::VariablesNameColumn)
}

/// Get value divider dragging signal
pub fn value_divider_dragging_signal() -> impl Signal<Item = bool> {
    // Use dragging system signal
    crate::visualizer::interaction::dragging::is_divider_dragging(crate::visualizer::interaction::dragging::DividerType::VariablesValueColumn)
}

// === COMPATIBILITY FUNCTIONS (NO-OP) ===

/// Initialize function for compatibility - no longer needed
pub fn initialize() {
    // Panel layout now handled by dragging.rs system - no initialization needed
}

// === DEPRECATED BRIDGE FUNCTIONS (NO-OP) ===
// These functions are no longer needed but kept for compilation compatibility

/// Migration helper: Get current files panel height - DEPRECATED
pub fn current_files_panel_height() -> u32 {
    // Return reasonable default - actual values come from config system now
    300
}

/// Migration helper: Get current variables name column width - DEPRECATED  
pub fn current_variables_name_column_width() -> u32 {
    // Return reasonable default - actual values come from config system now
    180
}

/// Migration helper: Get current variables value column width - DEPRECATED
pub fn current_variables_value_column_width() -> u32 {
    // Return reasonable default - actual values come from config system now
    100
}

/// Migration helper: Set files panel height - DEPRECATED (NO-OP)
pub fn set_files_panel_height(_height: u32) {
    // No-op - dragging system handles updates directly through config
}

/// Migration helper: Set variables name column width - DEPRECATED (NO-OP)  
pub fn set_variables_name_column_width(_width: u32) {
    // No-op - dragging system handles updates directly through config
}

/// Migration helper: Set variables value column width - DEPRECATED (NO-OP)
pub fn set_variables_value_column_width(_width: u32) {
    // No-op - dragging system handles updates directly through config
}

// === LEGACY COMPATIBILITY ===

/// Legacy signal compatibility: Get files panel width signal (replaces FILES_PANEL_WIDTH.signal())
pub fn files_width_signal() -> impl Signal<Item = u32> {
    files_panel_width_signal()
}

/// Legacy signal compatibility: Get files panel height signal (replaces FILES_PANEL_HEIGHT.signal())
pub fn files_height_signal() -> impl Signal<Item = u32> {
    files_panel_height_signal()
}

/// Legacy signal compatibility: Get variables name column width signal
pub fn name_column_width_signal() -> impl Signal<Item = u32> {
    variables_name_column_width_signal()
}

/// Legacy signal compatibility: Get variables value column width signal
pub fn value_column_width_signal() -> impl Signal<Item = u32> {
    variables_value_column_width_signal()
}

/// Legacy signal compatibility: Vertical dragging signal
pub fn vertical_dragging_signal() -> impl Signal<Item = bool> {
    files_vertical_dragging_signal()
}

/// Legacy signal compatibility: Horizontal dragging signal
pub fn horizontal_dragging_signal() -> impl Signal<Item = bool> {
    files_horizontal_dragging_signal()
}