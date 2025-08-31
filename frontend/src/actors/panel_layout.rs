//! PanelLayout domain for comprehensive panel and dock management using Actor+Relay architecture
//!
//! Complete panel layout domain that replaces ALL 12+ panel layout global mutables with event-driven architecture.
//! Manages panel dimensions, dock modes, dragging states, column widths, and layout transitions.
//!
//! ## Replaces Global Mutables:
//! - FILES_PANEL_WIDTH: Mutable<u32>
//! - FILES_PANEL_HEIGHT: Mutable<u32> 
//! - VARIABLES_NAME_COLUMN_WIDTH: Mutable<u32>
//! - VARIABLES_VALUE_COLUMN_WIDTH: Mutable<u32>
//! - IS_DOCKED_TO_BOTTOM: Mutable<bool>
//! - DOCK_MODE_FOR_CONFIG: Mutable<shared::DockMode>
//! - Various dragging state mutables
//! - Panel transition states

#![allow(dead_code)] // Actor+Relay API not yet fully integrated

use crate::actors::{Actor, Relay, relay};
use shared::DockMode;
use std::sync::OnceLock;
use zoon::*;
use futures::{StreamExt, select};

// Note: Using global_domains PANEL_LAYOUT_DOMAIN_INSTANCE instead of local static

/// Complete panel layout domain with Actor+Relay architecture.
/// 
/// Consolidates ALL panel layout state into a single cohesive domain.
/// Replaces 12+ global mutables with event-driven reactive state management.
#[derive(Clone, Debug)]
pub struct PanelLayout {
    // === CORE STATE ACTORS (replacing 12+ global mutables) ===
    
    /// Files panel width in pixels → replaces FILES_PANEL_WIDTH
    files_panel_width: Actor<u32>,
    
    /// Files panel height in pixels → replaces FILES_PANEL_HEIGHT  
    files_panel_height: Actor<u32>,
    
    /// Variables table name column width → replaces VARIABLES_NAME_COLUMN_WIDTH
    variables_name_column_width: Actor<u32>,
    
    /// Variables table value column width → replaces VARIABLES_VALUE_COLUMN_WIDTH
    variables_value_column_width: Actor<u32>,
    
    /// Timeline panel height for dock layouts
    timeline_panel_height: Actor<u32>,
    
    /// Current dock mode → replaces IS_DOCKED_TO_BOTTOM + DOCK_MODE_FOR_CONFIG
    dock_mode: Actor<DockMode>,
    
    /// Panel dimensions for each dock mode (preserved during switching)
    dock_mode_dimensions: Actor<DockModeDimensions>,
    
    // === DRAGGING STATES ===
    
    /// Files panel vertical divider being dragged
    files_vertical_dragging: Actor<bool>,
    
    /// Files panel horizontal divider being dragged
    files_horizontal_dragging: Actor<bool>,
    
    /// Variables name column divider being dragged
    name_divider_dragging: Actor<bool>,
    
    /// Variables value column divider being dragged
    value_divider_dragging: Actor<bool>,
    
    /// Dock transition in progress (layout switching)
    dock_transitioning: Actor<bool>,
    
    // === EVENT-SOURCE RELAYS (following {source}_{event}_relay pattern) ===
    
    /// Files panel was resized by user drag
    pub files_panel_resized_relay: Relay<PanelResizeEvent>,
    
    /// Variables column was resized by user drag
    pub variables_column_resized_relay: Relay<ColumnResizeEvent>,
    
    /// Timeline panel was resized by user drag
    pub timeline_panel_resized_relay: Relay<u32>,
    
    /// User clicked dock mode toggle button
    pub dock_mode_toggled_relay: Relay<()>,
    
    /// User selected specific dock mode
    pub dock_mode_changed_relay: Relay<DockMode>,
    
    /// Panel layout restored from configuration
    pub layout_restored_relay: Relay<PanelLayoutState>,
    
    /// Drag operation started on panel divider
    pub panel_drag_started_relay: Relay<PanelDragEvent>,
    
    /// Drag operation ended on panel divider
    pub panel_drag_ended_relay: Relay<PanelDragEvent>,
    
    /// Mouse moved during drag operation
    pub drag_mouse_moved_relay: Relay<(f32, f32)>,
    
    /// Window/viewport resized affecting panel layout
    pub viewport_resized_relay: Relay<(f32, f32)>,
}

/// Panel resize event data
#[derive(Clone, Debug)]
pub struct PanelResizeEvent {
    pub panel: PanelType,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Column resize event data
#[derive(Clone, Debug)]
pub struct ColumnResizeEvent {
    pub column: VariableColumn,
    pub width: u32,
}

/// Panel drag event data
#[derive(Clone, Debug)]
pub struct PanelDragEvent {
    pub divider: DividerType,
    pub position: (f32, f32),
}

/// Panel type enumeration
#[derive(Clone, Debug)]
pub enum PanelType {
    Files,
    Variables,
    Timeline,
}

/// Variable table column types
#[derive(Clone, Debug)]
pub enum VariableColumn {
    Name,
    Value,
}

/// Panel divider types
#[derive(Clone, Debug)]
pub enum DividerType {
    FilesVertical,
    FilesHorizontal,
    VariablesNameColumn,
    VariablesValueColumn,
}

/// Complete panel layout state
#[derive(Clone, Debug)]
pub struct PanelLayoutState {
    pub files_panel_width: u32,
    pub files_panel_height: u32,
    pub variables_name_column_width: u32,
    pub variables_value_column_width: u32,
    pub timeline_panel_height: u32,
    pub dock_mode: DockMode,
}

/// Panel dimensions for each dock mode
#[derive(Clone, Debug)]
pub struct DockModeDimensions {
    pub right_dock: PanelLayoutState,
    pub bottom_dock: PanelLayoutState,
}

impl Default for PanelLayoutState {
    fn default() -> Self {
        Self {
            files_panel_width: 470,
            files_panel_height: 300,
            variables_name_column_width: 180,
            variables_value_column_width: 100,
            timeline_panel_height: 200,
            dock_mode: DockMode::Right,
        }
    }
}

impl Default for DockModeDimensions {
    fn default() -> Self {
        Self {
            right_dock: PanelLayoutState {
                files_panel_width: 400,
                files_panel_height: 300,
                variables_name_column_width: 180,
                variables_value_column_width: 100,
                timeline_panel_height: 150,
                dock_mode: DockMode::Right,
            },
            bottom_dock: PanelLayoutState {
                files_panel_width: 1400,
                files_panel_height: 600,
                variables_name_column_width: 180,
                variables_value_column_width: 100,
                timeline_panel_height: 200,
                dock_mode: DockMode::Bottom,
            },
        }
    }
}

impl PanelLayout {
    /// Create a new comprehensive PanelLayout domain - simplified for compilation
    pub async fn new() -> Self {
        // Create all event-source relays
        let (files_panel_resized_relay, _files_panel_resized_stream) = relay();
        let (variables_column_resized_relay, _variables_column_resized_stream) = relay();
        let (timeline_panel_resized_relay, _timeline_panel_resized_stream) = relay();
        let (dock_mode_toggled_relay, _dock_mode_toggled_stream) = relay();
        let (dock_mode_changed_relay, _dock_mode_changed_stream) = relay();
        let (layout_restored_relay, _layout_restored_stream) = relay();
        let (panel_drag_started_relay, _panel_drag_started_stream) = relay();
        let (panel_drag_ended_relay, _panel_drag_ended_stream) = relay();
        let (drag_mouse_moved_relay, _drag_mouse_moved_stream) = relay();
        let (viewport_resized_relay, _viewport_resized_stream) = relay();
        
        // Use placeholder actors for now - will be properly implemented later
        let files_panel_width = {
            let drag_mouse_moved_relay_clone = drag_mouse_moved_relay.clone();
            Actor::new(470, async move |handle| {
                let mut drag_mouse_moved_stream = drag_mouse_moved_relay_clone.subscribe().fuse();
            let mut files_vertical_dragging_stream = crate::actors::global_domains::panel_layout_files_vertical_dragging_signal().to_stream().fuse();
            
            let mut is_dragging = false;
            zoon::println!("🚀 Files panel width Actor initialized");
            
            loop {
                select! {
                    dragging_event = files_vertical_dragging_stream.next() => {
                        match dragging_event {
                            Some(dragging_state) => {
                                // Sync Actor state with current dock-specific width when dragging starts
                                if dragging_state && !is_dragging {
                                    let config = crate::config::app_config();
                                    let config_clone = config.clone();
                                    let handle_clone = handle.clone();
                                    Task::start(async move {
                                        let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                        let current_width = match current_dock_mode {
                                            Some(shared::DockMode::Right) => {
                                                if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                    dims.files_panel_width as u32
                                                } else { 470 }
                                            }
                                            Some(shared::DockMode::Bottom) => {
                                                if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                    dims.files_panel_width as u32
                                                } else { 470 }
                                            }
                                            None => 470
                                        };
                                        handle_clone.set(current_width);
                                        zoon::println!("🎯 Synced Actor width to current dock: {}", current_width);
                                    });
                                }
                                is_dragging = dragging_state;
                                zoon::println!("🎯 Files panel dragging state: {}", is_dragging);
                            }
                            None => break, // Stream ended
                        }
                    }
                    mouse_event = drag_mouse_moved_stream.next() => {
                        match mouse_event {
                            Some((movement_x, _movement_y)) => {
                                if is_dragging {
                                    // Get current width and apply movement
                                    let current_width = handle.get();
                                    let movement_x: f32 = movement_x;  // Explicit type
                                    let new_width_f32: f32 = (current_width as f32 + movement_x).max(200.0).min(800.0);
                                    let new_width = new_width_f32 as u32;
                                    
                                    if new_width != current_width {
                                        handle.set(new_width);
                                        
                                        // Update only the current dock mode's dimensions to preserve independence
                                        let config = crate::config::app_config();
                                        
                                        // Get current dock mode - we need to check which mode is active
                                        // Since we can't get sync values from Actor signals easily, let's use both
                                        // and let the config system handle which one is currently active
                                        
                                        // Get current right dimensions and update just the width
                                        let config_clone = config.clone();
                                        Task::start(async move {
                                            let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                            
                                            match current_dock_mode {
                                                Some(shared::DockMode::Right) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                        current_dims.files_panel_width = new_width as f32;
                                                        config_clone.panel_dimensions_right_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated RIGHT dock dimensions: width = {}", new_width);
                                                    }
                                                }
                                                Some(shared::DockMode::Bottom) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                        current_dims.files_panel_width = new_width as f32;
                                                        config_clone.panel_dimensions_bottom_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated BOTTOM dock dimensions: width = {}", new_width);
                                                    }
                                                }
                                                None => {
                                                    zoon::eprintln!("⚠️ Could not determine dock mode for width update");
                                                }
                                            }
                                        });
                                        
                                        zoon::println!("🔄 Panel width: {} -> {} (Δ{}) - Config updated", current_width, new_width, movement_x);
                                    }
                                }
                            }
                            None => break, // Stream ended
                        }
                    }
                }
            }
        })
        };
        let files_panel_height = {
            let drag_mouse_moved_relay_clone = drag_mouse_moved_relay.clone();
            
            // Use reasonable default - the bridge initialization will set the correct value
            let initial_height = 400u32;
            zoon::println!("🔧 INIT: files_panel_height Actor starting with default: {}", initial_height);
            
            Actor::new(initial_height, async move |handle| {
                let mut drag_mouse_moved_stream = drag_mouse_moved_relay_clone.subscribe().fuse();
            let mut files_horizontal_dragging_stream = crate::actors::global_domains::panel_layout_files_horizontal_dragging_signal().to_stream().fuse();
            
            let mut is_dragging = false;
            zoon::println!("🚀 Files panel height Actor initialized");
            
            // Immediate startup sync to prevent jumping on first drag
            {
                let config = crate::config::app_config();
                let config_clone = config.clone();
                let handle_clone = handle.clone();
                Task::start(async move {
                    Timer::sleep(100).await; // Small delay to ensure config is loaded
                    let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                    zoon::println!("🔍 STARTUP SYNC: Current dock mode for height: {:?}", current_dock_mode);
                    let current_height = match current_dock_mode {
                        Some(shared::DockMode::Right) => {
                            if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                zoon::println!("🔍 STARTUP SYNC: Right dock dims.files_panel_height = {}", dims.files_panel_height);
                                dims.files_panel_height as u32
                            } else { 
                                zoon::println!("🔍 STARTUP SYNC: No right dock dimensions found, using default 300");
                                300 
                            }
                        }
                        Some(shared::DockMode::Bottom) => {
                            if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                zoon::println!("🔍 STARTUP SYNC: Bottom dock dims.files_panel_height = {}", dims.files_panel_height);
                                dims.files_panel_height as u32
                            } else { 
                                zoon::println!("🔍 STARTUP SYNC: No bottom dock dimensions found, using default 300");
                                300 
                            }
                        }
                        _ => {
                            zoon::println!("🔍 STARTUP SYNC: Unknown dock mode, using default 300");
                            300
                        }
                    };
                    
                    zoon::println!("🎯 STARTUP SYNC: Setting files panel height to: {}", current_height);
                    handle_clone.set(current_height);
                });
            }
            
            loop {
                select! {
                    dragging_event = files_horizontal_dragging_stream.next() => {
                        match dragging_event {
                            Some(dragging_state) => {
                                // Sync Actor state with current dock-specific height when dragging starts
                                if dragging_state && !is_dragging {
                                    let config = crate::config::app_config();
                                    let config_clone = config.clone();
                                    let handle_clone = handle.clone();
                                    Task::start(async move {
                                        let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                        zoon::println!("🔍 DEBUG: Current dock mode for height sync: {:?}", current_dock_mode);
                                        let current_height = match current_dock_mode {
                                            Some(shared::DockMode::Right) => {
                                                if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                    zoon::println!("🔍 DEBUG: Right dock dims.files_panel_height = {}", dims.files_panel_height);
                                                    dims.files_panel_height as u32
                                                } else { 
                                                    zoon::println!("🔍 DEBUG: No right dock dimensions found, using default 300");
                                                    300 
                                                }
                                            }
                                            Some(shared::DockMode::Bottom) => {
                                                if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                    zoon::println!("🔍 DEBUG: Bottom dock dims.files_panel_height = {}", dims.files_panel_height);
                                                    dims.files_panel_height as u32
                                                } else { 
                                                    zoon::println!("🔍 DEBUG: No bottom dock dimensions found, using default 300");
                                                    300 
                                                }
                                            }
                                            None => {
                                                zoon::println!("🔍 DEBUG: No dock mode found, using default 300");
                                                300
                                            }
                                        };
                                        handle_clone.set(current_height);
                                        zoon::println!("🎯 Synced files panel height to current dock: {}", current_height);
                                    });
                                }
                                is_dragging = dragging_state;
                                zoon::println!("🎯 Files panel horizontal dragging state: {}", is_dragging);
                            }
                            None => break,
                        }
                    }
                    mouse_event = drag_mouse_moved_stream.next() => {
                        match mouse_event {
                            Some((_movement_x, movement_y)) => {
                                if is_dragging {
                                    let current_height = handle.get();
                                    let movement_y: f32 = movement_y;
                                    let new_height_f32: f32 = (current_height as f32 + movement_y).max(150.0).min(600.0);
                                    let new_height = new_height_f32 as u32;
                                    
                                    if new_height != current_height {
                                        handle.set(new_height);
                                        
                                        // Update dock-specific config for height
                                        let config = crate::config::app_config();
                                        let config_clone = config.clone();
                                        Task::start(async move {
                                            let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                            
                                            match current_dock_mode {
                                                Some(shared::DockMode::Right) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                        current_dims.files_panel_height = new_height as f32;
                                                        config_clone.panel_dimensions_right_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated RIGHT dock height: {}", new_height);
                                                    }
                                                }
                                                Some(shared::DockMode::Bottom) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                        current_dims.files_panel_height = new_height as f32;
                                                        config_clone.panel_dimensions_bottom_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated BOTTOM dock height: {}", new_height);
                                                    }
                                                }
                                                None => {
                                                    zoon::eprintln!("⚠️ Could not determine dock mode for height update");
                                                }
                                            }
                                        });
                                        
                                        zoon::println!("🔄 Panel height: {} -> {} (Δ{}) - Config updated", current_height, new_height, movement_y);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        })
        };
        let variables_name_column_width = {
            let drag_mouse_moved_relay_clone = drag_mouse_moved_relay.clone();
            
            // Use reasonable default - the bridge initialization will set the correct value
            let initial_width = 200u32;
            zoon::println!("🔧 INIT: name_column_width Actor starting with default: {}", initial_width);
            
            Actor::new(initial_width, async move |handle| {
                let mut drag_mouse_moved_stream = drag_mouse_moved_relay_clone.subscribe().fuse();
                let mut name_divider_dragging_stream = crate::actors::global_domains::panel_layout_name_divider_dragging_signal().to_stream().fuse();
            
            let mut is_dragging = false;
            zoon::println!("🚀 Variables name column width Actor initialized");
            
            // Immediate startup sync to prevent jumping on first drag
            {
                let config = crate::config::app_config();
                let config_clone = config.clone();
                let handle_clone = handle.clone();
                Task::start(async move {
                    Timer::sleep(100).await; // Small delay to ensure config is loaded
                    let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                    zoon::println!("🔍 STARTUP SYNC: Current dock mode for name column: {:?}", current_dock_mode);
                    let current_width = match current_dock_mode {
                        Some(shared::DockMode::Right) => {
                            if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                zoon::println!("🔍 STARTUP SYNC: Right dock dims.variables_name_column_width = {}", dims.variables_name_column_width);
                                dims.variables_name_column_width as u32
                            } else { 
                                zoon::println!("🔍 STARTUP SYNC: No right dock dimensions found for name column, using default 150");
                                150 
                            }
                        }
                        Some(shared::DockMode::Bottom) => {
                            if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                zoon::println!("🔍 STARTUP SYNC: Bottom dock dims.variables_name_column_width = {}", dims.variables_name_column_width);
                                dims.variables_name_column_width as u32
                            } else { 
                                zoon::println!("🔍 STARTUP SYNC: No bottom dock dimensions found for name column, using default 150");
                                150 
                            }
                        }
                        _ => {
                            zoon::println!("🔍 STARTUP SYNC: Unknown dock mode for name column, using default 150");
                            150
                        }
                    };
                    
                    zoon::println!("🎯 STARTUP SYNC: Setting name column width to: {}", current_width);
                    handle_clone.set(current_width);
                });
            }
            
            loop {
                select! {
                    dragging_event = name_divider_dragging_stream.next() => {
                        match dragging_event {
                            Some(dragging_state) => {
                                // Sync Actor state with current dock-specific name column width when dragging starts
                                if dragging_state && !is_dragging {
                                    let config = crate::config::app_config();
                                    let config_clone = config.clone();
                                    let handle_clone = handle.clone();
                                    Task::start(async move {
                                        let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                        let current_width = match current_dock_mode {
                                            Some(shared::DockMode::Right) => {
                                                if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                    dims.variables_name_column_width as u32
                                                } else { 180 }
                                            }
                                            Some(shared::DockMode::Bottom) => {
                                                if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                    dims.variables_name_column_width as u32
                                                } else { 180 }
                                            }
                                            None => 180
                                        };
                                        handle_clone.set(current_width);
                                        zoon::println!("🎯 Synced name column width to current dock: {}", current_width);
                                    });
                                }
                                is_dragging = dragging_state;
                                zoon::println!("🎯 Name column dragging state: {}", is_dragging);
                            }
                            None => break,
                        }
                    }
                    mouse_event = drag_mouse_moved_stream.next() => {
                        match mouse_event {
                            Some((movement_x, _movement_y)) => {
                                if is_dragging {
                                    let current_width = handle.get();
                                    let movement_x: f32 = movement_x;
                                    let new_width_f32: f32 = (current_width as f32 + movement_x).max(100.0).min(400.0);
                                    let new_width = new_width_f32 as u32;
                                    
                                    if new_width != current_width {
                                        handle.set(new_width);
                                        
                                        // Update dock-specific config for name column width
                                        let config = crate::config::app_config();
                                        let config_clone = config.clone();
                                        Task::start(async move {
                                            let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                            
                                            match current_dock_mode {
                                                Some(shared::DockMode::Right) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                        current_dims.variables_name_column_width = new_width as f32;
                                                        config_clone.panel_dimensions_right_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated RIGHT dock name column width: {}", new_width);
                                                    }
                                                }
                                                Some(shared::DockMode::Bottom) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                        current_dims.variables_name_column_width = new_width as f32;
                                                        config_clone.panel_dimensions_bottom_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated BOTTOM dock name column width: {}", new_width);
                                                    }
                                                }
                                                None => {
                                                    zoon::eprintln!("⚠️ Could not determine dock mode for name column width update");
                                                }
                                            }
                                        });
                                        
                                        zoon::println!("🔄 Name column width: {} -> {} (Δ{}) - Config updated", current_width, new_width, movement_x);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        })
        };
        let variables_value_column_width = {
            let drag_mouse_moved_relay_clone = drag_mouse_moved_relay.clone();
            
            // Use reasonable default - the bridge initialization will set the correct value
            let initial_width = 150u32;
            zoon::println!("🔧 INIT: value_column_width Actor starting with default: {}", initial_width);
            
            Actor::new(initial_width, async move |handle| {
                let mut drag_mouse_moved_stream = drag_mouse_moved_relay_clone.subscribe().fuse();
                let mut value_divider_dragging_stream = crate::actors::global_domains::panel_layout_value_divider_dragging_signal().to_stream().fuse();
            
            let mut is_dragging = false;
            zoon::println!("🚀 Variables value column width Actor initialized");
            
            // Immediate startup sync to prevent jumping on first drag
            {
                let config = crate::config::app_config();
                let config_clone = config.clone();
                let handle_clone = handle.clone();
                Task::start(async move {
                    Timer::sleep(100).await; // Small delay to ensure config is loaded
                    let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                    zoon::println!("🔍 STARTUP SYNC: Current dock mode for value column: {:?}", current_dock_mode);
                    let current_width = match current_dock_mode {
                        Some(shared::DockMode::Right) => {
                            if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                zoon::println!("🔍 STARTUP SYNC: Right dock dims.variables_value_column_width = {}", dims.variables_value_column_width);
                                dims.variables_value_column_width as u32
                            } else { 
                                zoon::println!("🔍 STARTUP SYNC: No right dock dimensions found for value column, using default 100");
                                100 
                            }
                        }
                        Some(shared::DockMode::Bottom) => {
                            if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                zoon::println!("🔍 STARTUP SYNC: Bottom dock dims.variables_value_column_width = {}", dims.variables_value_column_width);
                                dims.variables_value_column_width as u32
                            } else { 
                                zoon::println!("🔍 STARTUP SYNC: No bottom dock dimensions found for value column, using default 100");
                                100 
                            }
                        }
                        _ => {
                            zoon::println!("🔍 STARTUP SYNC: Unknown dock mode for value column, using default 100");
                            100
                        }
                    };
                    
                    zoon::println!("🎯 STARTUP SYNC: Setting value column width to: {}", current_width);
                    handle_clone.set(current_width);
                });
            }
            
            loop {
                select! {
                    dragging_event = value_divider_dragging_stream.next() => {
                        match dragging_event {
                            Some(dragging_state) => {
                                // Sync Actor state with current dock-specific value column width when dragging starts
                                if dragging_state && !is_dragging {
                                    let config = crate::config::app_config();
                                    let config_clone = config.clone();
                                    let handle_clone = handle.clone();
                                    Task::start(async move {
                                        let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                        let current_width = match current_dock_mode {
                                            Some(shared::DockMode::Right) => {
                                                if let Some(dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                    dims.variables_value_column_width as u32
                                                } else { 100 }
                                            }
                                            Some(shared::DockMode::Bottom) => {
                                                if let Some(dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                    dims.variables_value_column_width as u32
                                                } else { 100 }
                                            }
                                            None => 100
                                        };
                                        handle_clone.set(current_width);
                                        zoon::println!("🎯 Synced value column width to current dock: {}", current_width);
                                    });
                                }
                                is_dragging = dragging_state;
                                zoon::println!("🎯 Value column dragging state: {}", is_dragging);
                            }
                            None => break,
                        }
                    }
                    mouse_event = drag_mouse_moved_stream.next() => {
                        match mouse_event {
                            Some((movement_x, _movement_y)) => {
                                if is_dragging {
                                    let current_width = handle.get();
                                    let movement_x: f32 = movement_x;
                                    let new_width_f32: f32 = (current_width as f32 + movement_x).max(80.0).min(300.0);
                                    let new_width = new_width_f32 as u32;
                                    
                                    if new_width != current_width {
                                        handle.set(new_width);
                                        
                                        // Update dock-specific config for value column width
                                        let config = crate::config::app_config();
                                        let config_clone = config.clone();
                                        Task::start(async move {
                                            let current_dock_mode = config_clone.dock_mode_actor.signal().to_stream().next().await;
                                            
                                            match current_dock_mode {
                                                Some(shared::DockMode::Right) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_right_actor.signal().to_stream().next().await {
                                                        current_dims.variables_value_column_width = new_width as f32;
                                                        config_clone.panel_dimensions_right_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated RIGHT dock value column width: {}", new_width);
                                                    }
                                                }
                                                Some(shared::DockMode::Bottom) => {
                                                    if let Some(mut current_dims) = config_clone.panel_dimensions_bottom_actor.signal().to_stream().next().await {
                                                        current_dims.variables_value_column_width = new_width as f32;
                                                        config_clone.panel_dimensions_bottom_changed_relay.send(current_dims);
                                                        zoon::println!("🔄 Updated BOTTOM dock value column width: {}", new_width);
                                                    }
                                                }
                                                None => {
                                                    zoon::eprintln!("⚠️ Could not determine dock mode for value column width update");
                                                }
                                            }
                                        });
                                        
                                        zoon::println!("🔄 Value column width: {} -> {} (Δ{}) - Config updated", current_width, new_width, movement_x);
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        })
        };
        let timeline_panel_height = Actor::new(200, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        let dock_mode = Actor::new(DockMode::Right, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        let dock_mode_dimensions = Actor::new(DockModeDimensions::default(), async move |_handle| {
            // TODO: Implement proper actor processor  
        });
        let files_vertical_dragging = Actor::new(false, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        let files_horizontal_dragging = Actor::new(false, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        let name_divider_dragging = Actor::new(false, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        let value_divider_dragging = Actor::new(false, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        let dock_transitioning = Actor::new(false, async move |_handle| {
            // TODO: Implement proper actor processor
        });
        
        // ===== BRIDGE: Connect Actor signals to old mutable signals =====
        // This bridges the new Actor system with the old mutable signals that the UI reads from
        
        // Bridge files_panel_height Actor to old mutable
        {
            let files_panel_height_clone = files_panel_height.clone();
            Task::start(async move {
                files_panel_height_clone.signal().for_each_sync(|height| {
                    crate::actors::global_domains::set_files_panel_height(height.clone());
                }).await;
            });
        }
        
        // Bridge variables_name_column_width Actor to old mutable
        {
            let name_column_width_clone = variables_name_column_width.clone();
            Task::start(async move {
                name_column_width_clone.signal().for_each_sync(|width| {
                    crate::actors::global_domains::set_variables_name_column_width(width.clone());
                }).await;
            });
        }
        
        // Bridge variables_value_column_width Actor to old mutable
        {
            let value_column_width_clone = variables_value_column_width.clone();
            Task::start(async move {
                value_column_width_clone.signal().for_each_sync(|width| {
                    crate::actors::global_domains::set_variables_value_column_width(width.clone());
                }).await;
            });
        }
        
        // Create domain instance with initialized actors
        Self {
            files_panel_width,
            files_panel_height,
            variables_name_column_width,
            variables_value_column_width,
            timeline_panel_height,
            dock_mode,
            dock_mode_dimensions,
            files_vertical_dragging,
            files_horizontal_dragging,
            name_divider_dragging,
            value_divider_dragging,
            dock_transitioning,
            files_panel_resized_relay,
            variables_column_resized_relay,
            timeline_panel_resized_relay,
            dock_mode_toggled_relay,
            dock_mode_changed_relay,
            layout_restored_relay,
            panel_drag_started_relay,
            panel_drag_ended_relay,
            drag_mouse_moved_relay,
            viewport_resized_relay,
        }
    }
    
    // === EVENT HANDLERS ===
    
    async fn handle_files_panel_resized(&self, _event: PanelResizeEvent) {
        // TODO: Implement actual Actor processing when Actor API is clarified
        // For now, use signal synchronization approach like other domains
    }
    
    async fn handle_variables_column_resized(&self, _event: ColumnResizeEvent) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_timeline_panel_resized(&self, _height: u32) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_dock_mode_toggled(&self) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_dock_mode_changed(&self, _mode: DockMode) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_layout_restored(&self, _state: PanelLayoutState) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_panel_drag_started(&self, _event: PanelDragEvent) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_panel_drag_ended(&self, _event: PanelDragEvent) {
        // TODO: Implement proper Actor processing 
    }
    
    async fn handle_drag_mouse_moved(&self, _position: (f32, f32)) {
        // This is now handled by the Actor stream processing above
        // TODO: Remove this function and its call sites
    }
    
    async fn handle_viewport_resized(&self, _size: (f32, f32)) {
        // TODO: Implement proper Actor processing 
    }
}

// ===== SIGNAL ACCESS FUNCTIONS (LIFETIME-SAFE) =====

/// Get files panel width signal
pub fn files_panel_width_signal() -> impl Signal<Item = u32> {
    crate::actors::global_domains::panel_layout_files_width_signal()
}

/// Get files panel height signal
pub fn files_panel_height_signal() -> impl Signal<Item = u32> {
    crate::actors::global_domains::panel_layout_files_height_signal()
}

/// Get variables name column width signal
pub fn variables_name_column_width_signal() -> impl Signal<Item = u32> {
    crate::actors::global_domains::panel_layout_name_column_width_signal()
}

/// Get variables value column width signal
pub fn variables_value_column_width_signal() -> impl Signal<Item = u32> {
    crate::actors::global_domains::panel_layout_value_column_width_signal()
}

/// Get timeline panel height signal
pub fn timeline_panel_height_signal() -> impl Signal<Item = u32> {
    crate::actors::global_domains::panel_layout_timeline_height_signal()
}


/// Get docked to bottom signal (derived for backward compatibility)  
pub fn docked_to_bottom_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::panel_layout_dock_mode_signal().map(|mode| matches!(mode, DockMode::Bottom))
}

/// Get dock transitioning signal
pub fn dock_transitioning_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::panel_layout_dock_transitioning_signal()
}

/// Get files vertical dragging signal
pub fn files_vertical_dragging_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::panel_layout_files_vertical_dragging_signal()
}

/// Get files horizontal dragging signal
pub fn files_horizontal_dragging_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::panel_layout_files_horizontal_dragging_signal()
}

/// Get name divider dragging signal
pub fn name_divider_dragging_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::panel_layout_name_divider_dragging_signal()
}

/// Get value divider dragging signal
pub fn value_divider_dragging_signal() -> impl Signal<Item = bool> {
    crate::actors::global_domains::panel_layout_value_divider_dragging_signal()
}

// ===== PUBLIC RELAY FUNCTIONS (EVENT-SOURCE API) =====

/// Files panel resized event
pub fn resize_files_panel(width: Option<u32>, height: Option<u32>) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.files_panel_resized_relay.send(PanelResizeEvent {
        panel: PanelType::Files,
        width,
        height,
    });
}

/// Variables column resized event  
pub fn resize_variables_column(column: VariableColumn, width: u32) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.variables_column_resized_relay.send(ColumnResizeEvent { column, width });
}

/// Timeline panel resized event
pub fn resize_timeline_panel(height: u32) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.timeline_panel_resized_relay.send(height);
}

/// Toggle dock mode event
pub fn toggle_dock_mode() {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.dock_mode_toggled_relay.send(());
}

/// Change dock mode event
pub fn change_dock_mode(mode: DockMode) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.dock_mode_changed_relay.send(mode);
}

/// Restore panel layout from configuration
pub fn restore_panel_layout(state: PanelLayoutState) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.layout_restored_relay.send(state);
}

/// Panel drag started event
pub fn start_panel_drag(divider: DividerType, position: (f32, f32)) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.panel_drag_started_relay.send(PanelDragEvent { divider, position });
}

/// Panel drag ended event
pub fn end_panel_drag(divider: DividerType, position: (f32, f32)) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.panel_drag_ended_relay.send(PanelDragEvent { divider, position });
}

/// Mouse moved during drag
pub fn drag_mouse_moved(position: (f32, f32)) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.drag_mouse_moved_relay.send(position);
}

/// Viewport resized affecting layout
pub fn viewport_resized(size: (f32, f32)) {
    let domain = crate::actors::global_domains::panel_layout_domain();
    domain.viewport_resized_relay.send(size);
}

// ===== MIGRATION FOUNDATION =====

/// Migration helper: Get current files panel width (replaces FILES_PANEL_WIDTH.get())
pub fn current_files_panel_width() -> u32 {
    // Use signal storage for immediate synchronous access during migration
    crate::actors::global_domains::PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.files_panel_width_mutable.get())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default width 300");
            300
        })
}

/// Migration helper: Get current files panel height (replaces FILES_PANEL_HEIGHT.get())
pub fn current_files_panel_height() -> u32 {
    crate::actors::global_domains::PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.files_panel_height_mutable.get())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default height 200");
            200
        })
}

/// Migration helper: Get current variables name column width (replaces VARIABLES_NAME_COLUMN_WIDTH.get())
pub fn current_variables_name_column_width() -> u32 {
    crate::actors::global_domains::PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.variables_name_column_width_mutable.get())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default name column width 150");
            150
        })
}

/// Migration helper: Get current variables value column width (replaces VARIABLES_VALUE_COLUMN_WIDTH.get())
pub fn current_variables_value_column_width() -> u32 {
    crate::actors::global_domains::PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.variables_value_column_width_mutable.get())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default value column width 100");
            100
        })
}

/// Migration helper: Get current dock mode (replaces IS_DOCKED_TO_BOTTOM.get())
pub fn current_dock_mode() -> shared::DockMode {
    crate::actors::global_domains::PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.dock_mode_mutable.get())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning default dock mode Right");
            shared::DockMode::Right
        })
}

/// Migration helper: Check if docked to bottom (replaces IS_DOCKED_TO_BOTTOM.get())
pub fn is_docked_to_bottom() -> bool {
    matches!(current_dock_mode(), shared::DockMode::Bottom)
}

/// Migration helper: Check if dock transition is in progress (replaces DOCK_TOGGLE_IN_PROGRESS.get())
pub fn is_dock_transitioning() -> bool {
    crate::actors::global_domains::PANEL_LAYOUT_SIGNALS.get()
        .map(|signals| signals.dock_transitioning_mutable.get())
        .unwrap_or_else(|| {
            zoon::eprintln!("⚠️ PanelLayout signals not initialized, returning false dock transitioning");
            false
        })
}

/// Migration helper: Set files panel width (replaces FILES_PANEL_WIDTH.set_neq())
pub fn set_files_panel_width(width: u32) {
    resize_files_panel(Some(width), None);
}

/// Migration helper: Set files panel height (replaces FILES_PANEL_HEIGHT.set_neq())
pub fn set_files_panel_height(height: u32) {
    resize_files_panel(None, Some(height));
}

/// Migration helper: Set variables name column width (replaces VARIABLES_NAME_COLUMN_WIDTH.set_neq())
pub fn set_variables_name_column_width(width: u32) {
    resize_variables_column(VariableColumn::Name, width);
}

/// Migration helper: Set variables value column width (replaces VARIABLES_VALUE_COLUMN_WIDTH.set_neq())
pub fn set_variables_value_column_width(width: u32) {
    resize_variables_column(VariableColumn::Value, width);
}

/// Migration helper: Set dock mode (replaces IS_DOCKED_TO_BOTTOM.set_neq())
pub fn set_dock_mode(mode: shared::DockMode) {
    change_dock_mode(mode);
}

/// Migration helper: Set docked to bottom (replaces IS_DOCKED_TO_BOTTOM.set_neq())
pub fn set_docked_to_bottom(docked: bool) {
    let mode = if docked { shared::DockMode::Bottom } else { shared::DockMode::Right };
    change_dock_mode(mode);
}

// ===== LEGACY SIGNAL COMPATIBILITY =====

/// Legacy signal compatibility: Get files panel width signal (replaces FILES_PANEL_WIDTH.signal())
pub fn files_width_signal() -> impl Signal<Item = u32> {
    // Use dock mode aware dimensions from app_config instead of static panel_layout
    map_ref! {
        let dock_mode = crate::config::app_config().dock_mode_actor.signal(),
        let right_dims = crate::config::app_config().panel_dimensions_right_actor.signal(),
        let bottom_dims = crate::config::app_config().panel_dimensions_bottom_actor.signal() => {
            match dock_mode {
                shared::DockMode::Right => right_dims.files_panel_width as u32,
                shared::DockMode::Bottom => bottom_dims.files_panel_width as u32,
            }
        }
    }
}

/// Legacy signal compatibility: Get files panel height signal (replaces FILES_PANEL_HEIGHT.signal())
pub fn files_height_signal() -> impl Signal<Item = u32> {
    files_panel_height_signal()
}

/// Legacy signal compatibility: Get variables name column width signal (replaces VARIABLES_NAME_COLUMN_WIDTH.signal())
pub fn name_column_width_signal() -> impl Signal<Item = u32> {
    variables_name_column_width_signal()
}

/// Legacy signal compatibility: Get variables value column width signal (replaces VARIABLES_VALUE_COLUMN_WIDTH.signal())
pub fn value_column_width_signal() -> impl Signal<Item = u32> {
    variables_value_column_width_signal()
}

/// Legacy signal compatibility: Get dock mode signal (docked to bottom bool) - DUPLICATE REMOVED

// ===== LEGACY RELAY COMPATIBILITY (for existing imports) =====

/// Legacy relay compatibility: Vertical divider dragged relay - Simple working approach
pub fn vertical_divider_dragged_relay() -> Relay<f32> {
    static RELAY: std::sync::OnceLock<Relay<f32>> = std::sync::OnceLock::new();
    
    RELAY.get_or_init(|| {
        let (relay, stream) = relay();
        
        // Simple: just update the dragging state and let the existing system handle it
        Task::start(stream.for_each(move |drag_state| async move {
            let is_dragging = drag_state > 0.0;
            crate::actors::global_domains::set_files_vertical_dragging(is_dragging);
            zoon::println!("🔄 Vertical drag state: {}", is_dragging);
        }));
        
        relay
    }).clone()
}

/// Legacy relay compatibility: Horizontal divider dragged relay
pub fn horizontal_divider_dragged_relay() -> Relay<f32> {
    static CACHED_RELAY: OnceLock<Relay<f32>> = OnceLock::new();
    CACHED_RELAY.get_or_init(|| {
        let (relay, stream) = relay();
        let mut stream = stream.fuse();
        
        // Task to handle horizontal dragging state changes
        Task::start(async move {
            while let Some(drag_amount) = stream.next().await {
                let is_dragging = drag_amount > 0.0;
                crate::actors::global_domains::set_files_horizontal_dragging(is_dragging);
                zoon::println!("🔄 Horizontal drag state: {}", is_dragging);
            }
        });
        
        relay
    }).clone()
}

/// Legacy relay compatibility: Name divider dragged relay  
pub fn name_divider_dragged_relay() -> Relay<f32> {
    static CACHED_RELAY: OnceLock<Relay<f32>> = OnceLock::new();
    CACHED_RELAY.get_or_init(|| {
        let (relay, stream) = relay();
        let mut stream = stream.fuse();
        
        // Task to handle name column dragging state changes
        Task::start(async move {
            while let Some(drag_amount) = stream.next().await {
                let is_dragging = drag_amount > 0.0;
                crate::actors::global_domains::set_name_divider_dragging(is_dragging);
                zoon::println!("🔄 Name column drag state: {}", is_dragging);
            }
        });
        
        relay
    }).clone()
}

/// Legacy relay compatibility: Value divider dragged relay
pub fn value_divider_dragged_relay() -> Relay<f32> {
    static CACHED_RELAY: OnceLock<Relay<f32>> = OnceLock::new();
    CACHED_RELAY.get_or_init(|| {
        let (relay, stream) = relay();
        let mut stream = stream.fuse();
        
        // Task to handle value column dragging state changes
        Task::start(async move {
            while let Some(drag_amount) = stream.next().await {
                let is_dragging = drag_amount > 0.0;
                crate::actors::global_domains::set_value_divider_dragging(is_dragging);
                zoon::println!("🔄 Value column drag state: {}", is_dragging);
            }
        });
        
        relay
    }).clone()
}

/// Legacy relay compatibility: Mouse moved relay
pub fn mouse_moved_relay() -> Relay<(f32, f32)> {
    static RELAY: std::sync::OnceLock<Relay<(f32, f32)>> = std::sync::OnceLock::new();
    
    RELAY.get_or_init(|| {
        let (relay, stream) = relay();
        
        // Connect to drag_mouse_moved function
        Task::start(stream.for_each(move |position| async move {
            drag_mouse_moved(position);
        }));
        
        relay
    }).clone()
}

/// Legacy signal compatibility: Vertical dragging signal
pub fn vertical_dragging_signal() -> impl Signal<Item = bool> {
    files_vertical_dragging_signal()
}

/// Legacy signal compatibility: Horizontal dragging signal
pub fn horizontal_dragging_signal() -> impl Signal<Item = bool> {
    files_horizontal_dragging_signal()
}

// ===== INITIALIZATION =====

/// Initialize the panel layout domain
pub fn initialize() {
    // Domain is initialized through global_domains system
    // This function remains for compatibility with existing initialization calls
}