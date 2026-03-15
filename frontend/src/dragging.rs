//! Reactive panel dragging system using direct Mutable pattern
//!
//! Data flows: Mouse Events → Direct Methods → Config Updates → UI Signals

use shared::DockMode;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use zoon::*;

const MIN_FILES_PANEL_WIDTH_RIGHT: f32 = 240.0;
const MAX_FILES_PANEL_WIDTH_RIGHT: f32 = 1200.0;
const MIN_FILES_PANEL_WIDTH_BOTTOM: f32 = 240.0;
const MAX_FILES_PANEL_WIDTH_BOTTOM: f32 = 1600.0;

const MIN_FILES_PANEL_HEIGHT_RIGHT: f32 = 220.0;
const MIN_FILES_PANEL_HEIGHT_BOTTOM: f32 = 220.0;
const MAX_FILES_PANEL_HEIGHT: f32 = 900.0;

#[derive(Clone, Debug, PartialEq)]
pub enum DividerType {
    FilesPanelMain,
    FilesPanelSecondary,
    VariablesNameColumn,
    VariablesValueColumn,
    SignalRowDivider { unique_id: String },
}

#[derive(Clone, Debug, Default)]
struct DragState {
    active_divider: Option<DividerType>,
    drag_start_position: (f32, f32),
    initial_value: f32,
    has_logged_first_move: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraggingDebugMetrics {
    pub row_drag_update_count: u64,
    pub divider_drag_update_count: u64,
    pub applied_divider_resize_count: u64,
    pub applied_row_resize_count: u64,
}

#[derive(Clone)]
pub struct DraggingSystem {
    drag_state: Mutable<DragState>,
    app_config: crate::config::AppConfig,
    selected_variables: crate::selected_variables::SelectedVariables,
    pending_divider_resize: Mutable<Option<(DividerType, f32)>>,
    divider_resize_frame_scheduled: Mutable<bool>,
    debug_metrics: Mutable<DraggingDebugMetrics>,
}

impl DraggingSystem {
    pub fn new(
        app_config: crate::config::AppConfig,
        selected_variables: crate::selected_variables::SelectedVariables,
    ) -> Self {
        let system = Self {
            drag_state: Mutable::new(DragState::default()),
            app_config,
            selected_variables,
            pending_divider_resize: Mutable::new(None),
            divider_resize_frame_scheduled: Mutable::new(false),
            debug_metrics: Mutable::new(DraggingDebugMetrics::default()),
        };
        system.install_global_drag_event_listeners();
        system
    }

    fn install_global_drag_event_listeners(&self) {
        let Some(window) = web_sys::window() else {
            return;
        };

        let move_system = self.clone();
        let move_closure = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
            if move_system.drag_state.lock_ref().active_divider.is_none() {
                return;
            }
            move_system.process_drag_movement((event.client_x() as f32, event.client_y() as f32));
        }) as Box<dyn FnMut(_)>);
        let _ = window
            .add_event_listener_with_callback("pointermove", move_closure.as_ref().unchecked_ref());
        move_closure.forget();

        let up_system = self.clone();
        let up_closure = Closure::wrap(Box::new(move |_event: web_sys::PointerEvent| {
            if up_system.drag_state.lock_ref().active_divider.is_none() {
                return;
            }
            up_system.end_drag();
        }) as Box<dyn FnMut(_)>);
        let _ = window
            .add_event_listener_with_callback("pointerup", up_closure.as_ref().unchecked_ref());
        up_closure.forget();

        let cancel_system = self.clone();
        let cancel_closure = Closure::wrap(Box::new(move |_event: web_sys::PointerEvent| {
            let active_divider = cancel_system.drag_state.lock_ref().active_divider.clone();
            if let Some(active_divider) = active_divider {
                log_drag(format!(
                    "cancel divider={} source=window",
                    divider_label(&active_divider)
                ));
                cancel_system.end_drag();
            }
        }) as Box<dyn FnMut(_)>);
        let _ = window.add_event_listener_with_callback(
            "pointercancel",
            cancel_closure.as_ref().unchecked_ref(),
        );
        cancel_closure.forget();
    }

    fn apply_divider_resize_value(&self, divider_type: &DividerType, value: f32) {
        let dock_mode = self.app_config.dock_mode.get_cloned();
        match (divider_type, dock_mode) {
            (DividerType::FilesPanelMain, DockMode::Right) => {
                self.app_config.files_panel_width_right.set_neq(value);
            }
            (DividerType::FilesPanelMain, DockMode::Bottom) => {
                self.app_config.files_panel_width_bottom.set_neq(value);
            }
            (DividerType::FilesPanelSecondary, DockMode::Right) => {
                self.app_config.files_panel_height_right.set_neq(value);
            }
            (DividerType::FilesPanelSecondary, DockMode::Bottom) => {
                self.app_config.files_panel_height_bottom.set_neq(value);
            }
            (DividerType::VariablesNameColumn, _) => {
                self.app_config.variables_name_column_width.set_neq(value);
            }
            (DividerType::VariablesValueColumn, _) => {
                self.app_config.variables_value_column_width.set_neq(value);
            }
            (DividerType::SignalRowDivider { .. }, _) => {}
        }
        self.debug_metrics.update_mut(|metrics| {
            metrics.applied_divider_resize_count =
                metrics.applied_divider_resize_count.saturating_add(1);
        });
    }

    fn apply_pending_divider_resize(&self) {
        let Some((divider_type, value)) = self.pending_divider_resize.take() else {
            self.divider_resize_frame_scheduled.set(false);
            return;
        };
        self.divider_resize_frame_scheduled.set(false);
        self.apply_divider_resize_value(&divider_type, value);
    }

    fn schedule_divider_resize_frame(&self, divider_type: DividerType, value: f32) {
        self.pending_divider_resize.set(Some((divider_type, value)));

        if self.divider_resize_frame_scheduled.get_cloned() {
            return;
        }

        self.divider_resize_frame_scheduled.set(true);
        let system = self.clone();
        let callback = Closure::once(move || {
            system.apply_pending_divider_resize();
        });

        if let Some(window) = web_sys::window() {
            if window
                .request_animation_frame(callback.as_ref().unchecked_ref())
                .is_ok()
            {
                callback.forget();
                return;
            }
        }

        self.apply_pending_divider_resize();
    }

    fn apply_live_row_resize(&self, unique_id: &str, row_height: u32) {
        self.selected_variables
            .set_live_row_height_without_total_height(unique_id, row_height);
        self.debug_metrics.update_mut(|metrics| {
            metrics.applied_row_resize_count = metrics.applied_row_resize_count.saturating_add(1);
        });
    }

    pub fn start_drag(&self, divider_type: DividerType, start_position: (f32, f32)) {
        let dock_mode = self.app_config.dock_mode.get_cloned();

        self.app_config.set_divider_drag_in_progress(true);

        if matches!(divider_type, DividerType::SignalRowDivider { .. }) {
            self.app_config.set_row_resize_in_progress(true);
        }

        let initial_value = match &divider_type {
            DividerType::FilesPanelMain => match dock_mode {
                DockMode::Right => self.app_config.files_panel_width_right.get_cloned(),
                DockMode::Bottom => self.app_config.files_panel_width_bottom.get_cloned(),
            },
            DividerType::FilesPanelSecondary => match dock_mode {
                DockMode::Right => self.app_config.files_panel_height_right.get_cloned(),
                DockMode::Bottom => self.app_config.files_panel_height_bottom.get_cloned(),
            },
            DividerType::VariablesNameColumn => {
                self.app_config.variables_name_column_width.get_cloned()
            }
            DividerType::VariablesValueColumn => {
                self.app_config.variables_value_column_width.get_cloned()
            }
            DividerType::SignalRowDivider { unique_id } => {
                self.selected_variables.live_row_height(unique_id) as f32
            }
        };

        self.drag_state.set(DragState {
            active_divider: Some(divider_type),
            drag_start_position: start_position,
            initial_value,
            has_logged_first_move: false,
        });
        log_drag(format!(
            "start divider={} x={} y={}",
            divider_label(self.drag_state.lock_ref().active_divider.as_ref().unwrap()),
            start_position.0,
            start_position.1
        ));
    }

    pub fn process_drag_movement(&self, current_position: (f32, f32)) {
        let current_drag_state = self.drag_state.get_cloned();

        if let Some(ref divider_type) = current_drag_state.active_divider {
            let dock_mode = self.app_config.dock_mode.get_cloned();

            let (delta, new_value) = match divider_type {
                DividerType::FilesPanelMain => {
                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                    let unclamped_width = current_drag_state.initial_value + delta_x;
                    let (min_width, max_width) = match dock_mode {
                        DockMode::Right => {
                            (MIN_FILES_PANEL_WIDTH_RIGHT, MAX_FILES_PANEL_WIDTH_RIGHT)
                        }
                        DockMode::Bottom => {
                            (MIN_FILES_PANEL_WIDTH_BOTTOM, MAX_FILES_PANEL_WIDTH_BOTTOM)
                        }
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
                    (
                        delta_y,
                        unclamped_height.clamp(min_height, MAX_FILES_PANEL_HEIGHT),
                    )
                }
                DividerType::VariablesNameColumn => {
                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                    (
                        delta_x,
                        (current_drag_state.initial_value + delta_x).clamp(100.0, 400.0),
                    )
                }
                DividerType::VariablesValueColumn => {
                    let delta_x = current_position.0 - current_drag_state.drag_start_position.0;
                    (
                        delta_x,
                        (current_drag_state.initial_value + delta_x).clamp(100.0, 400.0),
                    )
                }
                DividerType::SignalRowDivider { .. } => {
                    let delta_y = current_position.1 - current_drag_state.drag_start_position.1;
                    (
                        delta_y,
                        (current_drag_state.initial_value + delta_y).clamp(20.0, 300.0),
                    )
                }
            };

            if delta.abs() > 1.0 {
                if !current_drag_state.has_logged_first_move {
                    self.drag_state.update_mut(|state| {
                        state.has_logged_first_move = true;
                    });
                    log_drag(format!(
                        "move divider={} x={} y={}",
                        divider_label(divider_type),
                        current_position.0,
                        current_position.1
                    ));
                }
                match (divider_type, dock_mode) {
                    (DividerType::FilesPanelMain, DockMode::Right) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.divider_drag_update_count =
                                metrics.divider_drag_update_count.saturating_add(1);
                        });
                        self.schedule_divider_resize_frame(divider_type.clone(), new_value);
                    }
                    (DividerType::FilesPanelMain, DockMode::Bottom) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.divider_drag_update_count =
                                metrics.divider_drag_update_count.saturating_add(1);
                        });
                        self.schedule_divider_resize_frame(divider_type.clone(), new_value);
                    }
                    (DividerType::FilesPanelSecondary, DockMode::Right) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.divider_drag_update_count =
                                metrics.divider_drag_update_count.saturating_add(1);
                        });
                        self.schedule_divider_resize_frame(divider_type.clone(), new_value);
                    }
                    (DividerType::FilesPanelSecondary, DockMode::Bottom) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.divider_drag_update_count =
                                metrics.divider_drag_update_count.saturating_add(1);
                        });
                        self.schedule_divider_resize_frame(divider_type.clone(), new_value);
                    }
                    (DividerType::VariablesNameColumn, _) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.divider_drag_update_count =
                                metrics.divider_drag_update_count.saturating_add(1);
                        });
                        self.apply_divider_resize_value(divider_type, new_value);
                    }
                    (DividerType::VariablesValueColumn, _) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.divider_drag_update_count =
                                metrics.divider_drag_update_count.saturating_add(1);
                        });
                        self.apply_divider_resize_value(divider_type, new_value);
                    }
                    (DividerType::SignalRowDivider { unique_id }, _) => {
                        self.debug_metrics.update_mut(|metrics| {
                            metrics.row_drag_update_count =
                                metrics.row_drag_update_count.saturating_add(1);
                        });
                        self.apply_live_row_resize(unique_id, new_value as u32);
                    }
                }
            }
        }
    }

    pub fn end_drag(&self) {
        let active_divider = self.drag_state.get_cloned().active_divider;
        if let Some(ref divider) = active_divider {
            log_drag(format!("end divider={}", divider_label(divider)));
        }
        self.apply_pending_divider_resize();
        match active_divider {
            Some(DividerType::SignalRowDivider { unique_id }) => {
                let final_row_height = self.selected_variables.live_row_height(&unique_id);

                self.app_config.set_row_resize_in_progress(false);
                self.selected_variables
                    .set_live_row_height(&unique_id, final_row_height);
                self.selected_variables.commit_live_row_height(&unique_id);
            }
            Some(DividerType::VariablesNameColumn | DividerType::VariablesValueColumn) => {
                self.app_config
                    .sync_live_selected_variables_widths_to_current_dock();
                self.app_config.request_save();
            }
            Some(DividerType::FilesPanelMain | DividerType::FilesPanelSecondary) => {
                self.app_config.request_save();
            }
            None => {}
        }
        self.app_config.set_divider_drag_in_progress(false);
        self.divider_resize_frame_scheduled.set(false);
        self.drag_state.set(DragState::default());
    }

    pub fn is_any_divider_dragging(&self) -> impl Signal<Item = bool> + 'static {
        self.drag_state
            .signal_cloned()
            .map(|state| state.active_divider.is_some())
    }

    pub fn is_divider_dragging(
        &self,
        divider_type: DividerType,
    ) -> impl Signal<Item = bool> + 'static {
        self.drag_state.signal_cloned().map(move |state| {
            matches!(state.active_divider, Some(ref active_type) if *active_type == divider_type)
        })
    }

    pub fn active_divider_type_signal(&self) -> impl Signal<Item = Option<DividerType>> + 'static {
        self.drag_state
            .signal_cloned()
            .map(|state| state.active_divider)
    }

    pub fn active_overlay_divider_signal(
        &self,
    ) -> impl Signal<Item = Option<DividerType>> + 'static {
        self.drag_state
            .signal_cloned()
            .map(|state| state.active_divider)
    }

    pub fn debug_metrics_actor(&self) -> Mutable<DraggingDebugMetrics> {
        self.debug_metrics.clone()
    }

    pub fn reset_debug_metrics(&self) {
        self.debug_metrics.set(DraggingDebugMetrics::default());
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

pub fn variables_name_column_width_signal(
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = f32> {
    app_config.variables_name_column_width.signal()
}

pub fn variables_value_column_width_signal(
    app_config: crate::config::AppConfig,
) -> impl Signal<Item = f32> {
    app_config.variables_value_column_width.signal()
}

pub fn start_drag(system: &DraggingSystem, divider_type: DividerType, start_position: (f32, f32)) {
    system.start_drag(divider_type, start_position);
}

pub fn capture_pointer(raw_pointer_down: &events_extra::PointerDown, label: &str) {
    let pointer_id = raw_pointer_down.pointer_id();
    let Some(target) = raw_pointer_down.dyn_target::<web_sys::Element>() else {
        log_drag(format!(
            "capture-miss label={label} pointer_id={pointer_id}"
        ));
        return;
    };

    match target.set_pointer_capture(pointer_id) {
        Ok(()) => {
            let testid = target.get_attribute("data-testid").unwrap_or_default();
            log_drag(format!(
                "capture-ok label={label} pointer_id={pointer_id} testid={testid}"
            ));
        }
        Err(error) => {
            log_drag(format!(
                "capture-fail label={label} pointer_id={pointer_id} error={error:?}"
            ));
        }
    }
}

fn divider_label(divider_type: &DividerType) -> String {
    match divider_type {
        DividerType::FilesPanelMain => "files_panel_main".to_owned(),
        DividerType::FilesPanelSecondary => "files_panel_secondary".to_owned(),
        DividerType::VariablesNameColumn => "variables_name_column".to_owned(),
        DividerType::VariablesValueColumn => "variables_value_column".to_owned(),
        DividerType::SignalRowDivider { unique_id } => format!("signal_row:{unique_id}"),
    }
}

#[cfg(debug_assertions)]
fn log_drag(message: String) {
    zoon::println!("[DRAG] {message}");
}

#[cfg(not(debug_assertions))]
fn log_drag(_message: String) {}
