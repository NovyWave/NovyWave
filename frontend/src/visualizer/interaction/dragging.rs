//! Simplified panel dragging system
//!
//! Direct config-based dragging that eliminates jumping and supports dock-specific persistence.
//! Replaces the complex Actor+Relay+Bridge system with direct config updates.

use zoon::*;
use futures::StreamExt;
use shared::DockMode;

// === DRAGGING STATE ===

/// Global dragging state for all dividers using Atom (local UI state pattern)
/// ✅ FIXED: Replaced OnceLock with Atom for local UI state management
static DRAGGING_STATE: std::sync::LazyLock<DraggerState> = std::sync::LazyLock::new(DraggerState::default);

#[derive(Clone)]
pub struct DraggerState {
    /// Which divider is currently being dragged
    pub active_divider: Mutable<Option<DividerType>>,
    /// Start position for calculating deltas
    pub drag_start_position: Mutable<(f32, f32)>,
    /// Initial dimension value when drag started (prevents jumping)  
    pub initial_value: Mutable<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DividerType {
    /// Files & Scopes panel width (vertical drag in Right dock, horizontal drag in Bottom dock)
    FilesPanelMain,
    /// Files & Scopes panel height (horizontal drag in Right dock, vertical drag in Bottom dock)
    FilesPanelSecondary,
    /// Variables table name column width
    VariablesNameColumn,
    /// Variables table value column width
    VariablesValueColumn,
}

impl Default for DraggerState {
    fn default() -> Self {
        Self {
            active_divider: Mutable::new(None),
            drag_start_position: Mutable::new((0.0, 0.0)),
            initial_value: Mutable::new(0.0),
        }
    }
}

fn dragging_state() -> &'static DraggerState {
    &DRAGGING_STATE
}

// === DRAGGING LOGIC ===

/// Start dragging a divider
pub fn start_drag(divider_type: DividerType, _start_position: (f32, f32), app_config: &crate::config::AppConfig) {
    let state = dragging_state();
    
    // Set dragging state - get initial value from reactive signal
    state.active_divider.set_neq(Some(divider_type.clone()));
    // Don't set drag_start_position yet - wait for first mouse move event
    state.drag_start_position.set_neq((0.0, 0.0));
    
    // Use Task to get initial value from signals asynchronously
    let state_clone = state.clone();
    let divider_clone = divider_type.clone();
    let app_config_clone = app_config.clone();
    Task::start(async move {
        // Get initial value from the corresponding signal
        let initial_value = match &divider_clone {
            DividerType::FilesPanelMain => files_panel_width_signal(app_config_clone.clone()).to_stream().next().await.unwrap_or(crate::config::DEFAULT_PANEL_WIDTH),
            DividerType::FilesPanelSecondary => files_panel_height_signal(app_config_clone.clone()).to_stream().next().await.unwrap_or(crate::config::DEFAULT_PANEL_HEIGHT),
            DividerType::VariablesNameColumn => variables_name_column_width_signal(app_config_clone.clone()).to_stream().next().await.unwrap_or(crate::config::DEFAULT_NAME_COLUMN_WIDTH),
            DividerType::VariablesValueColumn => variables_value_column_width_signal(app_config_clone.clone()).to_stream().next().await.unwrap_or(crate::config::DEFAULT_VALUE_COLUMN_WIDTH),
        };
        
        state_clone.initial_value.set_neq(initial_value);
        
    });
}

/// Process mouse movement during drag
pub fn process_drag_movement(current_position: (f32, f32), app_config: &crate::config::AppConfig) {
    let state = dragging_state();
    
    if let Some(divider_type) = state.active_divider.get_cloned() {
        let start_pos = state.drag_start_position.get();
        let initial_value = state.initial_value.get();
        
        // If this is the first mouse move, capture the starting position
        if start_pos == (0.0, 0.0) {
            state.drag_start_position.set_neq(current_position);
            return;
        }
        
        // Calculate delta based on divider type - dock mode handled in update function
        let (delta, new_value) = match &divider_type {
            DividerType::FilesPanelMain => {
                // Files panel width - horizontal movement
                let delta_x = current_position.0 - start_pos.0;
                let new_width = (initial_value + delta_x).clamp(
                    crate::config::MIN_FILES_PANEL_WIDTH,
                    crate::config::MAX_FILES_PANEL_WIDTH
                );
                (delta_x, new_width)
            }
            DividerType::FilesPanelSecondary => {
                // Files panel height - vertical movement
                let delta_y = current_position.1 - start_pos.1;
                let new_height = (initial_value + delta_y).clamp(
                    crate::config::MIN_PANEL_HEIGHT,
                    crate::config::MAX_PANEL_HEIGHT
                );
                (delta_y, new_height)
            }
            DividerType::VariablesNameColumn => {
                // Name column width - horizontal movement
                let delta_x = current_position.0 - start_pos.0;
                let new_width = (initial_value + delta_x).clamp(
                    crate::config::MIN_COLUMN_WIDTH,
                    crate::config::MAX_COLUMN_WIDTH
                );
                (delta_x, new_width)
            }
            DividerType::VariablesValueColumn => {
                // Value column width - horizontal movement
                let delta_x = current_position.0 - start_pos.0;
                let new_width = (initial_value + delta_x).clamp(
                    crate::config::MIN_COLUMN_WIDTH,
                    crate::config::MAX_COLUMN_WIDTH
                );
                (delta_x, new_width)
            }
        };
        
        // Only update if value actually changed (avoids spam)
        if delta.abs() > 1.0 {
            update_config_dimension(&divider_type, new_value, app_config);
            
        }
    }
}

/// Stop dragging
pub fn end_drag() {
    let state = dragging_state();
    
    if let Some(_divider_type) = state.active_divider.get_cloned() {
        // Divider was active, now ending drag
    }
    
    state.active_divider.set_neq(None);
}

/// Update the appropriate config dimension
fn update_config_dimension(divider_type: &DividerType, new_value: f32, app_config: &crate::config::AppConfig) {
    let config = app_config.clone();
    let divider_type = divider_type.clone();
    
    Task::start(async move {
        let dock_mode = config.dock_mode_actor.signal().to_stream().next().await.unwrap_or(DockMode::Right);
        
        match (&divider_type, dock_mode) {
            (DividerType::FilesPanelMain, DockMode::Right) => {
                config.files_width_right_changed_relay.send(new_value);
            }
            (DividerType::FilesPanelMain, DockMode::Bottom) => {
                config.files_width_bottom_changed_relay.send(new_value);
            }
            (DividerType::FilesPanelSecondary, DockMode::Right) => {
                config.files_height_right_changed_relay.send(new_value);
            }
            (DividerType::FilesPanelSecondary, DockMode::Bottom) => {
                config.files_height_bottom_changed_relay.send(new_value);
            }
            (DividerType::VariablesNameColumn, _) => {
                config.name_column_width_changed_relay.send(new_value);
            }
            (DividerType::VariablesValueColumn, _) => {
                config.value_column_width_changed_relay.send(new_value);
            }
        }
    });
}

// === DRAGGING SIGNALS ===

/// Check if any divider is currently being dragged
pub fn is_any_divider_dragging() -> impl Signal<Item = bool> {
    dragging_state().active_divider.signal_ref(|divider| divider.is_some())
}

/// Check if a specific divider type is being dragged
pub fn is_divider_dragging(divider_type: DividerType) -> impl Signal<Item = bool> {
    dragging_state().active_divider.signal_ref(move |active| {
        matches!(active, Some(active_type) if *active_type == divider_type)
    })
}

/// Get the currently active divider type
pub fn active_divider_type_signal() -> impl Signal<Item = Option<DividerType>> {
    dragging_state().active_divider.signal_cloned()
}

// === PANEL DIMENSION SIGNALS ===

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

/// Get variables name column width signal for current dock mode
pub fn variables_name_column_width_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    app_config.variables_name_column_width_actor.signal()
}

/// Get variables value column width signal for current dock mode
pub fn variables_value_column_width_signal(app_config: crate::config::AppConfig) -> impl Signal<Item = f32> {
    app_config.variables_value_column_width_actor.signal()
}