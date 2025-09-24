//! Reactive panel dragging system using pure Actor+Relay architecture
//!
//! Data flows: Mouse Events → Dragging Actor → Config Updates → UI Signals

use crate::dataflow::{Actor, Relay, relay};
use futures::{StreamExt, select};
use shared::DockMode;
use zoon::*;

// Panel sizing constraints (keep layout usable while permitting generous resizing)
const MIN_FILES_PANEL_WIDTH_RIGHT: f32 = 240.0;
const MAX_FILES_PANEL_WIDTH_RIGHT: f32 = 1200.0;
const MIN_FILES_PANEL_WIDTH_BOTTOM: f32 = 240.0;
const MAX_FILES_PANEL_WIDTH_BOTTOM: f32 = 1600.0;

const MIN_FILES_PANEL_HEIGHT_RIGHT: f32 = 220.0;
const MIN_FILES_PANEL_HEIGHT_BOTTOM: f32 = 220.0;
const MAX_FILES_PANEL_HEIGHT: f32 = 900.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DividerType {
    FilesPanelMain,
    FilesPanelSecondary,
    VariablesNameColumn,
    VariablesValueColumn,
}

/// Reactive dragging system - pure Actor+Relay architecture
#[derive(Clone)]
pub struct DraggingSystem {
    // State managed by Actor
    drag_state_actor: Actor<DragState>,

    // Event relays for mouse interactions
    pub drag_started_relay: Relay<(DividerType, (f32, f32))>,
    pub drag_moved_relay: Relay<(f32, f32)>,
    pub drag_ended_relay: Relay,
}

#[derive(Clone, Debug, Default)]
struct DragState {
    active_divider: Option<DividerType>,
    drag_start_position: (f32, f32),
    initial_value: f32,
}

impl DraggingSystem {
    pub async fn new(app_config: crate::config::AppConfig) -> Self {
        let (drag_started_relay, mut drag_started_stream) = relay::<(DividerType, (f32, f32))>();
        let (drag_moved_relay, mut drag_moved_stream) = relay::<(f32, f32)>();
        let (drag_ended_relay, mut drag_ended_stream) = relay::<()>();

        let drag_state_actor = Actor::new(DragState::default(), async move |state_handle| {
            // Cache current values pattern - ONLY in Actor loops
            let mut cached_dock_mode = DockMode::Right;
            let mut cached_dimensions = (300.0, 300.0, 300.0, 300.0); // width_right, width_bottom, height_right, height_bottom
            let mut cached_column_widths = (190.0, 220.0); // name, value

            // Signal streams for cached values
            let mut dock_mode_stream = app_config.dock_mode_actor.signal().to_stream().fuse();
            let mut width_right_stream = app_config
                .files_panel_width_right_actor
                .signal()
                .to_stream()
                .fuse();
            let mut width_bottom_stream = app_config
                .files_panel_width_bottom_actor
                .signal()
                .to_stream()
                .fuse();
            let mut height_right_stream = app_config
                .files_panel_height_right_actor
                .signal()
                .to_stream()
                .fuse();
            let mut height_bottom_stream = app_config
                .files_panel_height_bottom_actor
                .signal()
                .to_stream()
                .fuse();
            let mut name_width_stream = app_config
                .variables_name_column_width_actor
                .signal()
                .to_stream()
                .fuse();
            let mut value_width_stream = app_config
                .variables_value_column_width_actor
                .signal()
                .to_stream()
                .fuse();

            loop {
                select! {
                    // Update cached config values
                    dock_mode = dock_mode_stream.next() => {
                        match dock_mode {
                            Some(dock_mode) => cached_dock_mode = dock_mode,
                            None => break, // Stream closed
                        }
                    }
                    width = width_right_stream.next() => {
                        match width {
                            Some(width) => cached_dimensions.0 = width,
                            None => break, // Stream closed
                        }
                    }
                    width = width_bottom_stream.next() => {
                        match width {
                            Some(width) => cached_dimensions.1 = width,
                            None => break, // Stream closed
                        }
                    }
                    height = height_right_stream.next() => {
                        match height {
                            Some(height) => cached_dimensions.2 = height,
                            None => break, // Stream closed
                        }
                    }
                    height = height_bottom_stream.next() => {
                        match height {
                            Some(height) => cached_dimensions.3 = height,
                            None => break, // Stream closed
                        }
                    }
                    width = name_width_stream.next() => {
                        match width {
                            Some(width) => cached_column_widths.0 = width,
                            None => break, // Stream closed
                        }
                    }
                    width = value_width_stream.next() => {
                        match width {
                            Some(width) => cached_column_widths.1 = width,
                            None => break, // Stream closed
                        }
                    }

                    // Process drag events with cached values
                    drag_event = drag_started_stream.next() => {
                        match drag_event {
                            Some((divider_type, start_pos)) => {
                        let initial_value = match divider_type {
                            DividerType::FilesPanelMain => match cached_dock_mode {
                                DockMode::Right => cached_dimensions.0,
                                DockMode::Bottom => cached_dimensions.1,
                            },
                            DividerType::FilesPanelSecondary => match cached_dock_mode {
                                DockMode::Right => cached_dimensions.2,
                                DockMode::Bottom => cached_dimensions.3,
                            },
                            DividerType::VariablesNameColumn => cached_column_widths.0,
                            DividerType::VariablesValueColumn => cached_column_widths.1,
                        };

                        state_handle.set(DragState {
                            active_divider: Some(divider_type),
                            drag_start_position: start_pos,
                            initial_value,
                        });
                            }
                            None => break, // Stream closed
                        }
                    }

                    drag_moved = drag_moved_stream.next() => {
                        match drag_moved {
                            Some(current_position) => {
                        // Use Actor's own cached state - no external state queries
                        let mut current_drag_state = state_handle.lock_mut();

                        if let Some(divider_type) = current_drag_state.active_divider {
                            let (delta, new_value) = match divider_type {
                                DividerType::FilesPanelMain => {
                                    let delta_x =
                                        current_position.0 - current_drag_state.drag_start_position.0;
                                    let unclamped_width = current_drag_state.initial_value + delta_x;
                                    let (min_width, max_width) = match cached_dock_mode {
                                        DockMode::Right => (
                                            MIN_FILES_PANEL_WIDTH_RIGHT,
                                            MAX_FILES_PANEL_WIDTH_RIGHT,
                                        ),
                                        DockMode::Bottom => (
                                            MIN_FILES_PANEL_WIDTH_BOTTOM,
                                            MAX_FILES_PANEL_WIDTH_BOTTOM,
                                        ),
                                    };
                                    let new_width = unclamped_width.clamp(min_width, max_width);
                                    (delta_x, new_width)
                                }
                                DividerType::FilesPanelSecondary => {
                                    let delta_y =
                                        current_position.1 - current_drag_state.drag_start_position.1;
                                    let unclamped_height = current_drag_state.initial_value + delta_y;
                                    let min_height = match cached_dock_mode {
                                        DockMode::Right => MIN_FILES_PANEL_HEIGHT_RIGHT,
                                        DockMode::Bottom => MIN_FILES_PANEL_HEIGHT_BOTTOM,
                                    };
                                    let new_height = unclamped_height.clamp(min_height, MAX_FILES_PANEL_HEIGHT);
                                    (delta_y, new_height)
                                }
                                DividerType::VariablesNameColumn => {
                                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                                    let new_width = (current_drag_state.initial_value + delta_x).clamp(100.0, 400.0);
                                    (delta_x, new_width)
                                }
                                DividerType::VariablesValueColumn => {
                                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                                    let new_width = (current_drag_state.initial_value + delta_x).clamp(100.0, 400.0);
                                    (delta_x, new_width)
                                }
                            };

                            if delta.abs() > 1.0 {
                                // Emit config updates via relays
                                match (divider_type, cached_dock_mode) {
                                    (DividerType::FilesPanelMain, DockMode::Right) => {
                                        app_config.files_width_right_changed_relay.send(new_value);
                                    }
                                    (DividerType::FilesPanelMain, DockMode::Bottom) => {
                                        app_config.files_width_bottom_changed_relay.send(new_value);
                                    }
                                    (DividerType::FilesPanelSecondary, DockMode::Right) => {
                                        app_config.files_height_right_changed_relay.send(new_value);
                                    }
                                    (DividerType::FilesPanelSecondary, DockMode::Bottom) => {
                                        app_config.files_height_bottom_changed_relay.send(new_value);
                                    }
                                    (DividerType::VariablesNameColumn, _) => {
                                        app_config.name_column_width_changed_relay.send(new_value);
                                    }
                                    (DividerType::VariablesValueColumn, _) => {
                                        app_config.value_column_width_changed_relay.send(new_value);
                                    }
                                }

                                app_config.config_save_requested_relay.send(());
                            }
                        }
                            }
                            None => break, // Stream closed
                        }
                    }

                    drag_ended = drag_ended_stream.next() => {
                        match drag_ended {
                            Some(()) => {
                                state_handle.set(DragState::default());
                            }
                            None => break, // Stream closed
                        }
                    }
                }
            }
        });

        Self {
            drag_state_actor,
            drag_started_relay,
            drag_moved_relay,
            drag_ended_relay,
        }
    }

    /// Check if any divider is currently being dragged
    pub fn is_any_divider_dragging(&self) -> impl Signal<Item = bool> + 'static {
        self.drag_state_actor
            .signal()
            .map(|state| state.active_divider.is_some())
    }

    /// Check if a specific divider type is being dragged
    pub fn is_divider_dragging(
        &self,
        divider_type: DividerType,
    ) -> impl Signal<Item = bool> + 'static {
        self.drag_state_actor.signal().map(move |state| {
            matches!(state.active_divider, Some(ref active_type) if *active_type == divider_type)
        })
    }

    /// Get the currently active divider type
    pub fn active_divider_type_signal(&self) -> impl Signal<Item = Option<DividerType>> + 'static {
        self.drag_state_actor
            .signal()
            .map(|state| state.active_divider)
    }

    pub fn active_overlay_divider_signal(
        &self,
    ) -> impl Signal<Item = Option<DividerType>> + 'static {
        self.drag_state_actor
            .signal()
            .map(|state| state.active_divider)
    }
}

// === SIGNAL COMPOSITION FUNCTIONS ===
// These provide composed signals that UI components can bind to

/// Get files panel height signal for current dock mode
pub fn files_panel_height_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config.dock_mode_actor.signal(),
        let right_height = app_config.files_panel_height_right_actor.signal(),
        let bottom_height = app_config.files_panel_height_bottom_actor.signal() => {
            match dock_mode {
                DockMode::Right => *right_height,
                DockMode::Bottom => *bottom_height,
            }
        }
    }
}

/// Get files panel width signal for current dock mode
pub fn files_panel_width_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config.dock_mode_actor.signal(),
        let right_width = app_config.files_panel_width_right_actor.signal(),
        let bottom_width = app_config.files_panel_width_bottom_actor.signal() => {
            match dock_mode {
                DockMode::Right => *right_width,
                DockMode::Bottom => *bottom_width,
            }
        }
    }
}

/// Get variables name column width signal
pub fn variables_name_column_width_signal(
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = f32> {
    app_config.variables_name_column_width_actor.signal()
}

/// Get variables value column width signal
pub fn variables_value_column_width_signal(
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = f32> {
    app_config.variables_value_column_width_actor.signal()
}

// === EVENT FUNCTIONS ===
// Simple relay emission functions for UI to call

/// Start dragging a divider
pub fn start_drag(system: &DraggingSystem, divider_type: DividerType, start_position: (f32, f32)) {
    system
        .drag_started_relay
        .send((divider_type, start_position));
}

/// Process mouse movement during drag
pub fn process_drag_movement(system: &DraggingSystem, current_position: (f32, f32)) {
    system.drag_moved_relay.send(current_position);
}

/// Stop dragging
pub fn end_drag(system: &DraggingSystem) {
    system.drag_ended_relay.send(());
}

/// Check if any divider is currently being dragged (legacy compatibility)
pub fn is_any_divider_dragging(system: &DraggingSystem) -> impl Signal<Item = bool> {
    system.is_any_divider_dragging()
}

/// Check if a specific divider type is being dragged (legacy compatibility)
pub fn is_divider_dragging(
    system: &DraggingSystem,
    divider_type: DividerType,
) -> impl Signal<Item = bool> {
    system.is_divider_dragging(divider_type)
}
