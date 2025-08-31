//! Simplified panel dragging system
//!
//! Direct config-based dragging that eliminates jumping and supports dock-specific persistence.
//! Replaces the complex Actor+Relay+Bridge system with direct config updates.

use zoon::*;
use futures::StreamExt;
use crate::config::app_config;
use shared::DockMode;

// === DRAGGING STATE ===

/// Global dragging state for all dividers
static DRAGGING_STATE: std::sync::OnceLock<DraggerState> = std::sync::OnceLock::new();

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
    DRAGGING_STATE.get_or_init(DraggerState::default)
}

// === DRAGGING LOGIC ===

/// Start dragging a divider
pub fn start_drag(divider_type: DividerType, _start_position: (f32, f32)) {
    let state = dragging_state();
    
    // Set dragging state - get initial value from reactive signal
    state.active_divider.set_neq(Some(divider_type.clone()));
    // Don't set drag_start_position yet - wait for first mouse move event
    state.drag_start_position.set_neq((0.0, 0.0));
    
    // Use Task to get initial value from signals asynchronously
    let state_clone = state.clone();
    let divider_clone = divider_type.clone();
    Task::start(async move {
        // Get initial value from the corresponding signal
        let initial_value = match &divider_clone {
            DividerType::FilesPanelMain => files_panel_width_signal().to_stream().next().await.unwrap_or(470.0),
            DividerType::FilesPanelSecondary => files_panel_height_signal().to_stream().next().await.unwrap_or(300.0),
            DividerType::VariablesNameColumn => variables_name_column_width_signal().to_stream().next().await.unwrap_or(180.0),
            DividerType::VariablesValueColumn => variables_value_column_width_signal().to_stream().next().await.unwrap_or(100.0),
        };
        
        state_clone.initial_value.set_neq(initial_value);
        
        zoon::println!("🎯 Started dragging {:?} from initial value {}", 
            divider_clone, initial_value);
    });
}

/// Process mouse movement during drag
pub fn process_drag_movement(current_position: (f32, f32)) {
    let state = dragging_state();
    
    if let Some(divider_type) = state.active_divider.get_cloned() {
        let start_pos = state.drag_start_position.get();
        let initial_value = state.initial_value.get();
        
        // If this is the first mouse move, capture the starting position
        if start_pos == (0.0, 0.0) {
            state.drag_start_position.set_neq(current_position);
            zoon::println!("🎯 First mouse move - capturing start position: {:?}", current_position);
            return;
        }
        
        // Calculate delta based on divider type - dock mode handled in update function
        let (delta, new_value) = match &divider_type {
            DividerType::FilesPanelMain => {
                // Files panel width - horizontal movement
                let delta_x = current_position.0 - start_pos.0;
                let new_width = (initial_value + delta_x).clamp(200.0, 1200.0);
                (delta_x, new_width)
            }
            DividerType::FilesPanelSecondary => {
                // Files panel height - vertical movement
                let delta_y = current_position.1 - start_pos.1;
                let new_height = (initial_value + delta_y).clamp(150.0, 800.0);
                (delta_y, new_height)
            }
            DividerType::VariablesNameColumn => {
                // Name column width - horizontal movement
                let delta_x = current_position.0 - start_pos.0;
                let new_width = (initial_value + delta_x).clamp(100.0, 400.0);
                (delta_x, new_width)
            }
            DividerType::VariablesValueColumn => {
                // Value column width - horizontal movement
                let delta_x = current_position.0 - start_pos.0;
                let new_width = (initial_value + delta_x).clamp(80.0, 300.0);
                (delta_x, new_width)
            }
        };
        
        // Only update if value actually changed (avoids spam)
        if delta.abs() > 1.0 {
            update_config_dimension(&divider_type, new_value);
            
            zoon::println!("🔄 Dragging {:?}: {} -> {} (Δ{:.1})", 
                divider_type, initial_value, new_value, delta);
        }
    }
}

/// Stop dragging
pub fn end_drag() {
    let state = dragging_state();
    
    if let Some(divider_type) = state.active_divider.get_cloned() {
        zoon::println!("🏁 Finished dragging {:?}", divider_type);
    }
    
    state.active_divider.set_neq(None);
}

/// Update the appropriate config dimension
fn update_config_dimension(divider_type: &DividerType, new_value: f32) {
    let config = app_config();
    let divider_type = divider_type.clone();
    
    // Get dock mode and update the appropriate dimensions asynchronously
    Task::start(async move {
        let dock_mode = config.dock_mode_actor.signal().to_stream().next().await.unwrap_or(DockMode::Right);
        
        match (&divider_type, dock_mode) {
            (DividerType::FilesPanelMain, DockMode::Right) => {
                let mut dims = config.panel_dimensions_right_actor.signal().to_stream().next().await.unwrap();
                dims.files_panel_width = new_value;
                config.panel_dimensions_right_changed_relay.send(dims);
            }
            (DividerType::FilesPanelMain, DockMode::Bottom) => {
                let mut dims = config.panel_dimensions_bottom_actor.signal().to_stream().next().await.unwrap();
                dims.files_panel_width = new_value;
                config.panel_dimensions_bottom_changed_relay.send(dims);
            }
            (DividerType::FilesPanelSecondary, DockMode::Right) => {
                let mut dims = config.panel_dimensions_right_actor.signal().to_stream().next().await.unwrap();
                dims.files_panel_height = new_value;
                config.panel_dimensions_right_changed_relay.send(dims);
            }
            (DividerType::FilesPanelSecondary, DockMode::Bottom) => {
                let mut dims = config.panel_dimensions_bottom_actor.signal().to_stream().next().await.unwrap();
                dims.files_panel_height = new_value;
                config.panel_dimensions_bottom_changed_relay.send(dims);
            }
            (DividerType::VariablesNameColumn, DockMode::Right) => {
                let mut dims = config.panel_dimensions_right_actor.signal().to_stream().next().await.unwrap();
                dims.variables_name_column_width = new_value;
                config.panel_dimensions_right_changed_relay.send(dims);
            }
            (DividerType::VariablesNameColumn, DockMode::Bottom) => {
                let mut dims = config.panel_dimensions_bottom_actor.signal().to_stream().next().await.unwrap();
                dims.variables_name_column_width = new_value;
                config.panel_dimensions_bottom_changed_relay.send(dims);
            }
            (DividerType::VariablesValueColumn, DockMode::Right) => {
                let mut dims = config.panel_dimensions_right_actor.signal().to_stream().next().await.unwrap();
                dims.variables_value_column_width = new_value;
                config.panel_dimensions_right_changed_relay.send(dims);
            }
            (DividerType::VariablesValueColumn, DockMode::Bottom) => {
                let mut dims = config.panel_dimensions_bottom_actor.signal().to_stream().next().await.unwrap();
                dims.variables_value_column_width = new_value;
                config.panel_dimensions_bottom_changed_relay.send(dims);
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

// === PANEL DIMENSION SIGNALS ===

/// Get files panel width signal for current dock mode
pub fn files_panel_width_signal() -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config().dock_mode_actor.signal(),
        let right_dims = app_config().panel_dimensions_right_actor.signal(),
        let bottom_dims = app_config().panel_dimensions_bottom_actor.signal() => {
            match dock_mode {
                DockMode::Right => right_dims.files_panel_width,
                DockMode::Bottom => bottom_dims.files_panel_width,
            }
        }
    }
}

/// Get files panel height signal for current dock mode
pub fn files_panel_height_signal() -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config().dock_mode_actor.signal(),
        let right_dims = app_config().panel_dimensions_right_actor.signal(),
        let bottom_dims = app_config().panel_dimensions_bottom_actor.signal() => {
            match dock_mode {
                DockMode::Right => right_dims.files_panel_height,
                DockMode::Bottom => bottom_dims.files_panel_height,
            }
        }
    }
}

/// Get variables name column width signal for current dock mode
pub fn variables_name_column_width_signal() -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config().dock_mode_actor.signal(),
        let right_dims = app_config().panel_dimensions_right_actor.signal(),
        let bottom_dims = app_config().panel_dimensions_bottom_actor.signal() => {
            match dock_mode {
                DockMode::Right => right_dims.variables_name_column_width,
                DockMode::Bottom => bottom_dims.variables_name_column_width,
            }
        }
    }
}

/// Get variables value column width signal for current dock mode
pub fn variables_value_column_width_signal() -> impl Signal<Item = f32> {
    map_ref! {
        let dock_mode = app_config().dock_mode_actor.signal(),
        let right_dims = app_config().panel_dimensions_right_actor.signal(),
        let bottom_dims = app_config().panel_dimensions_bottom_actor.signal() => {
            match dock_mode {
                DockMode::Right => right_dims.variables_value_column_width,
                DockMode::Bottom => bottom_dims.variables_value_column_width,
            }
        }
    }
}