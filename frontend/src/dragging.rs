//! Reactive panel dragging system using direct Mutable pattern
//!
//! Data flows: Mouse Events → Direct Methods → Config Updates → UI Signals

use shared::DockMode;
use zoon::*;

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

#[derive(Clone, Copy, Debug, Default)]
struct DragState {
    active_divider: Option<DividerType>,
    drag_start_position: (f32, f32),
    initial_value: f32,
}

#[derive(Clone)]
pub struct DraggingSystem {
    drag_state: Mutable<DragState>,
    app_config: crate::config::AppConfig,
}

impl DraggingSystem {
    pub fn new(app_config: crate::config::AppConfig) -> Self {
        Self {
            drag_state: Mutable::new(DragState::default()),
            app_config,
        }
    }

    pub fn start_drag(&self, divider_type: DividerType, start_position: (f32, f32)) {
        let dock_mode = self.app_config.dock_mode.get_cloned();

        let initial_value = match divider_type {
            DividerType::FilesPanelMain => match dock_mode {
                DockMode::Right => self.app_config.files_panel_width_right.get_cloned(),
                DockMode::Bottom => self.app_config.files_panel_width_bottom.get_cloned(),
            },
            DividerType::FilesPanelSecondary => match dock_mode {
                DockMode::Right => self.app_config.files_panel_height_right.get_cloned(),
                DockMode::Bottom => self.app_config.files_panel_height_bottom.get_cloned(),
            },
            DividerType::VariablesNameColumn => self.app_config.variables_name_column_width.get_cloned(),
            DividerType::VariablesValueColumn => self.app_config.variables_value_column_width.get_cloned(),
        };

        self.drag_state.set(DragState {
            active_divider: Some(divider_type),
            drag_start_position: start_position,
            initial_value,
        });
    }

    pub fn process_drag_movement(&self, current_position: (f32, f32)) {
        let current_drag_state = self.drag_state.get();

        if let Some(divider_type) = current_drag_state.active_divider {
            let dock_mode = self.app_config.dock_mode.get_cloned();

            let (delta, new_value) = match divider_type {
                DividerType::FilesPanelMain => {
                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                    let unclamped_width = current_drag_state.initial_value + delta_x;
                    let (min_width, max_width) = match dock_mode {
                        DockMode::Right => (MIN_FILES_PANEL_WIDTH_RIGHT, MAX_FILES_PANEL_WIDTH_RIGHT),
                        DockMode::Bottom => (MIN_FILES_PANEL_WIDTH_BOTTOM, MAX_FILES_PANEL_WIDTH_BOTTOM),
                    };
                    (delta_x, unclamped_width.clamp(min_width, max_width))
                }
                DividerType::FilesPanelSecondary => {
                    let delta_y = current_position.1 - current_drag_state.drag_start_position.1;
                    let unclamped_height = current_drag_state.initial_value + delta_y;
                    let min_height = match dock_mode {
                        DockMode::Right => MIN_FILES_PANEL_HEIGHT_RIGHT,
                        DockMode::Bottom => MIN_FILES_PANEL_HEIGHT_BOTTOM,
                    };
                    (delta_y, unclamped_height.clamp(min_height, MAX_FILES_PANEL_HEIGHT))
                }
                DividerType::VariablesNameColumn => {
                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                    (delta_x, (current_drag_state.initial_value + delta_x).clamp(100.0, 400.0))
                }
                DividerType::VariablesValueColumn => {
                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                    (delta_x, (current_drag_state.initial_value + delta_x).clamp(100.0, 400.0))
                }
            };

            if delta.abs() > 1.0 {
                match (divider_type, dock_mode) {
                    (DividerType::FilesPanelMain, DockMode::Right) => {
                        self.app_config.files_panel_width_right.set_neq(new_value);
                    }
                    (DividerType::FilesPanelMain, DockMode::Bottom) => {
                        self.app_config.files_panel_width_bottom.set_neq(new_value);
                    }
                    (DividerType::FilesPanelSecondary, DockMode::Right) => {
                        self.app_config.files_panel_height_right.set_neq(new_value);
                    }
                    (DividerType::FilesPanelSecondary, DockMode::Bottom) => {
                        self.app_config.files_panel_height_bottom.set_neq(new_value);
                    }
                    (DividerType::VariablesNameColumn, _) => {
                        self.app_config.variables_name_column_width.set_neq(new_value);
                    }
                    (DividerType::VariablesValueColumn, _) => {
                        self.app_config.variables_value_column_width.set_neq(new_value);
                    }
                }
                self.app_config.request_config_save();
            }
        }
    }

    pub fn end_drag(&self) {
        self.drag_state.set(DragState::default());
    }

    pub fn is_any_divider_dragging(&self) -> impl Signal<Item = bool> + 'static {
        self.drag_state.signal().map(|state| state.active_divider.is_some())
    }

    pub fn is_divider_dragging(&self, divider_type: DividerType) -> impl Signal<Item = bool> + 'static {
        self.drag_state.signal().map(move |state| {
            matches!(state.active_divider, Some(ref active_type) if *active_type == divider_type)
        })
    }

    pub fn active_divider_type_signal(&self) -> impl Signal<Item = Option<DividerType>> + 'static {
        self.drag_state.signal().map(|state| state.active_divider)
    }

    pub fn active_overlay_divider_signal(&self) -> impl Signal<Item = Option<DividerType>> + 'static {
        self.drag_state.signal().map(|state| state.active_divider)
    }
}

pub fn files_panel_height_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config.dock_mode.signal_cloned(),
        let right_height = app_config.files_panel_height_right.signal(),
        let bottom_height = app_config.files_panel_height_bottom.signal() => {
            match dock_mode {
                DockMode::Right => *right_height,
                DockMode::Bottom => *bottom_height,
            }
        }
    }
}

pub fn files_panel_width_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config.dock_mode.signal_cloned(),
        let right_width = app_config.files_panel_width_right.signal(),
        let bottom_width = app_config.files_panel_width_bottom.signal() => {
            match dock_mode {
                DockMode::Right => *right_width,
                DockMode::Bottom => *bottom_width,
            }
        }
    }
}

pub fn variables_name_column_width_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    app_config.variables_name_column_width.signal()
}

pub fn variables_value_column_width_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    app_config.variables_value_column_width.signal()
}

pub fn start_drag(system: &DraggingSystem, divider_type: DividerType, start_position: (f32, f32)) {
    system.start_drag(divider_type, start_position);
}

pub fn process_drag_movement(system: &DraggingSystem, current_position: (f32, f32)) {
    system.process_drag_movement(current_position);
}

pub fn end_drag(system: &DraggingSystem) {
    system.end_drag();
}
