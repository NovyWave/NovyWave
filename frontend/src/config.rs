use crate::dataflow::{Actor, Relay, relay};
use crate::platform::{CurrentPlatform, Platform};
use crate::visualizer::timeline::TimeNs;
use futures::{StreamExt, select};
use serde::{Deserialize, Serialize};
use shared::UpMsg;
use shared::{self, AppConfig as SharedAppConfig, DockMode, Theme as SharedTheme};
use std::sync::Arc;
use zoon::*;
use moonzoon_novyui::tokens::theme;
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeRange {
    pub start: TimeNs,
    pub end: TimeNs,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionState {
    pub opened_files: Vec<String>,
    pub variables_search_filter: String,
    pub file_picker_scroll_position: i32,
    pub file_picker_expanded_directories: Vec<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            variables_search_filter: String::new(),
            file_picker_scroll_position: 0,
            file_picker_expanded_directories: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TimelineState {
    pub cursor_position: TimeNs,
    pub visible_range: TimeRange,
    pub zoom_level: f64,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            cursor_position: TimeNs::ZERO,
            visible_range: TimeRange {
                start: TimeNs::ZERO,
                end: TimeNs::from_nanos(10_000_000_000),
            },
            zoom_level: 1.0,
        }
    }
}

pub const DEFAULT_PANEL_WIDTH: f32 = 300.0;
pub const DEFAULT_PANEL_HEIGHT: f32 = 300.0;
pub const DEFAULT_TIMELINE_HEIGHT: f32 = 200.0;
pub const DEFAULT_NAME_COLUMN_WIDTH: f32 = 190.0;
pub const DEFAULT_VALUE_COLUMN_WIDTH: f32 = 220.0;

pub const MIN_PANEL_HEIGHT: f32 = 150.0;
pub const MAX_PANEL_HEIGHT: f32 = 530.0;
pub const MIN_COLUMN_WIDTH: f32 = 100.0;
pub const MAX_COLUMN_WIDTH: f32 = 400.0;
pub const MIN_FILES_PANEL_WIDTH: f32 = 200.0;
pub const MAX_FILES_PANEL_WIDTH: f32 = 600.0;

#[derive(Clone)]
pub struct AppConfig {
    pub theme_actor: Actor<SharedTheme>,
    pub dock_mode_actor: Actor<DockMode>,
    
    pub files_panel_width_right_actor: Actor<f32>,
    pub files_panel_height_right_actor: Actor<f32>,
    pub files_panel_width_bottom_actor: Actor<f32>,
    pub files_panel_height_bottom_actor: Actor<f32>,
    pub variables_panel_width_actor: Actor<f32>,
    pub timeline_panel_height_actor: Actor<f32>,
    pub variables_name_column_width_actor: Actor<f32>,
    pub variables_value_column_width_actor: Actor<f32>,
    
    pub session_state_actor: Actor<SessionState>,
    pub toast_dismiss_ms_actor: Actor<u32>,

    pub file_picker_expanded_directories: Mutable<indexmap::IndexSet<String>>,
    pub file_picker_scroll_position: Mutable<i32>,

    pub loaded_selected_variables: Vec<shared::SelectedVariable>,

    pub theme_button_clicked_relay: Relay,
    pub dock_mode_button_clicked_relay: Relay,
    pub variables_filter_changed_relay: Relay<String>,
    pub files_width_right_changed_relay: Relay<f32>,
    pub files_height_right_changed_relay: Relay<f32>,
    pub files_width_bottom_changed_relay: Relay<f32>,
    pub files_height_bottom_changed_relay: Relay<f32>,
    pub variables_width_changed_relay: Relay<f32>,
    pub timeline_height_changed_relay: Relay<f32>,
    pub name_column_width_changed_relay: Relay<f32>,
    pub value_column_width_changed_relay: Relay<f32>,
    pub session_state_changed_relay: Relay<SessionState>,

    pub config_save_requested_relay: Relay,
    
    pub clipboard_copy_requested_relay: Relay<String>,
    
    pub error_display: crate::error_display::ErrorDisplay,
    
    _config_saver_actor: Actor<()>,
    _expanded_sync_actor: Actor<()>,
    _scroll_sync_actor: Actor<()>,
    _clipboard_actor: Actor<()>,
}

impl AppConfig {
    async fn load_config_from_backend() -> Result<SharedAppConfig, String> {
        crate::platform::CurrentPlatform::request_response(UpMsg::LoadConfig).await
    }

    pub async fn new() -> Self {
        let config = Self::load_config_from_backend()
            .await
            .unwrap_or_else(|_error| SharedAppConfig::default());

        let (theme_button_clicked_relay, mut theme_button_clicked_stream) = relay();
        let (dock_mode_button_clicked_relay, mut dock_mode_button_clicked_stream) = relay();
        let (variables_filter_changed_relay, variables_filter_changed_stream) = relay();
        let (session_state_changed_relay, session_state_changed_stream) = relay();
        let (config_save_requested_relay, mut config_save_requested_stream) = relay();
        
        let (clipboard_copy_requested_relay, mut clipboard_copy_requested_stream) = relay::<String>();
        
        let (files_width_right_changed_relay, mut files_width_right_changed_stream) = relay();
        let (files_height_right_changed_relay, mut files_height_right_changed_stream) = relay();
        let (files_width_bottom_changed_relay, mut files_width_bottom_changed_stream) = relay();
        let (files_height_bottom_changed_relay, mut files_height_bottom_changed_stream) = relay();
        let (variables_width_changed_relay, mut variables_width_changed_stream) = relay();
        let (timeline_height_changed_relay, mut timeline_height_changed_stream) = relay();
        let (name_column_width_changed_relay, mut name_column_width_changed_stream) = relay();
        let (value_column_width_changed_relay, mut value_column_width_changed_stream) = relay();

        let theme_actor = Actor::new(config.ui.theme, async move |state| {
            let mut current_theme = config.ui.theme;

            let initial_novyui_theme = match current_theme {
                SharedTheme::Light => theme::Theme::Light,
                SharedTheme::Dark => theme::Theme::Dark,
            };
            theme::init_theme(Some(initial_novyui_theme), None);

            loop {
                select! {
                    button_click = theme_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            let new_theme = match current_theme {
                                SharedTheme::Light => SharedTheme::Dark,
                                SharedTheme::Dark => SharedTheme::Light,
                            };
                            current_theme = new_theme;
                            state.set(new_theme);

                            let novyui_theme = match new_theme {
                                SharedTheme::Light => theme::Theme::Light,
                                SharedTheme::Dark => theme::Theme::Dark,
                            };
                            theme::set_theme(novyui_theme);
                        }
                    }
                }
            }
        });

        let dock_mode_actor = Actor::new(config.workspace.dock_mode.clone(), async move |state| {
            let mut current_dock_mode = config.workspace.dock_mode.clone();

            loop {
                select! {
                    button_click = dock_mode_button_clicked_stream.next() => {
                        if let Some(()) = button_click {
                            let new_mode = match current_dock_mode {
                                DockMode::Right => DockMode::Bottom,
                                DockMode::Bottom => DockMode::Right,
                            };
                            current_dock_mode = new_mode;
                            state.set(new_mode);
                        }
                    }
                }
            }
        });

        let files_panel_width_right_actor = Actor::new(
            config.workspace.docked_right_dimensions.files_and_scopes_panel_width as f32,
            async move |state| {
                loop {
                    select! {
                        new_width = files_width_right_changed_stream.next() => {
                            if let Some(width) = new_width {
                                state.set_neq(width);
                            }
                        }
                    }
                }
            }
        );

        let files_panel_height_right_actor = Actor::new(
            config.workspace.docked_right_dimensions.files_and_scopes_panel_height as f32,
            async move |state| {
                loop {
                    select! {
                        new_height = files_height_right_changed_stream.next() => {
                            if let Some(height) = new_height {
                                state.set_neq(height);
                            }
                        }
                    }
                }
            }
        );

        let files_panel_width_bottom_actor = Actor::new(
            config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width as f32,
            async move |state| {
                loop {
                    select! {
                        new_width = files_width_bottom_changed_stream.next() => {
                            if let Some(width) = new_width {
                                state.set_neq(width);
                            }
                        }
                    }
                }
            }
        );

        let files_panel_height_bottom_actor = Actor::new(
            config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height as f32,
            async move |state| {
                loop {
                    select! {
                        new_height = files_height_bottom_changed_stream.next() => {
                            if let Some(height) = new_height {
                                state.set_neq(height);
                            }
                        }
                    }
                }
            }
        );

        let variables_panel_width_actor = Actor::new(DEFAULT_PANEL_WIDTH, async move |state| {
            loop {
                select! {
                    new_width = variables_width_changed_stream.next() => {
                        if let Some(width) = new_width {
                            state.set_neq(width);
                        }
                    }
                }
            }
        });

        let timeline_panel_height_actor = Actor::new(DEFAULT_TIMELINE_HEIGHT, async move |state| {
            loop {
                select! {
                    new_height = timeline_height_changed_stream.next() => {
                        if let Some(height) = new_height {
                            state.set_neq(height);
                        }
                    }
                }
            }
        });

        let variables_name_column_width_actor = Actor::new(
            config.workspace.docked_right_dimensions.selected_variables_panel_name_column_width
                .unwrap_or(DEFAULT_NAME_COLUMN_WIDTH as f64) as f32,
            async move |state| {
                loop {
                    select! {
                        new_width = name_column_width_changed_stream.next() => {
                            if let Some(width) = new_width {
                                state.set_neq(width);
                            }
                        }
                    }
                }
            }
        );

        let variables_value_column_width_actor = Actor::new(
            config.workspace.docked_right_dimensions.selected_variables_panel_value_column_width
                .unwrap_or(DEFAULT_VALUE_COLUMN_WIDTH as f64) as f32,
            async move |state| {
                loop {
                    select! {
                        new_width = value_column_width_changed_stream.next() => {
                            if let Some(width) = new_width {
                                state.set_neq(width);
                            }
                        }
                    }
                }
            }
        );

        let session_state_actor = Actor::new(
            SessionState {
                opened_files: config.workspace.opened_files,
                variables_search_filter: config.workspace.variables_search_filter,
                file_picker_scroll_position: config.workspace.load_files_scroll_position,
                file_picker_expanded_directories: config.workspace.load_files_expanded_directories.clone(),
            },
            async move |state| {
                let mut session_stream = session_state_changed_stream;
                let mut variables_filter_stream = variables_filter_changed_stream;

                loop {
                    select! {
                        session_change = session_stream.next() => {
                            if let Some(new_session) = session_change {
                                state.set_neq(new_session);
                            }
                        }
                        filter_change = variables_filter_stream.next() => {
                            if let Some(new_filter) = filter_change {
                                state.update_mut(|session| {
                                    session.variables_search_filter = new_filter;
                                });
                            }
                        }
                    }
                }
            },
        );

        let toast_dismiss_ms_actor = Actor::new(config.ui.toast_dismiss_ms as u32, async move |_state| {
            loop {
                Task::next_macro_tick().await;
            }
        });

        let config_saver_actor = Actor::new((), {
            let theme_actor = theme_actor.clone();
            let dock_mode_actor = dock_mode_actor.clone();
            let files_width_right_actor = files_panel_width_right_actor.clone();
            let files_height_right_actor = files_panel_height_right_actor.clone();
            let files_width_bottom_actor = files_panel_width_bottom_actor.clone();
            let files_height_bottom_actor = files_panel_height_bottom_actor.clone();
            let variables_width_actor = variables_panel_width_actor.clone();
            let timeline_height_actor = timeline_panel_height_actor.clone();
            let name_width_actor = variables_name_column_width_actor.clone();
            let value_width_actor = variables_value_column_width_actor.clone();
            let session_actor = session_state_actor.clone();
            let toast_actor = toast_dismiss_ms_actor.clone();
            
            async move |_state| {
                let debounce_task = Arc::new(std::sync::Mutex::new(None::<TaskHandle>));

                loop {
                    select! {
                        save_request = config_save_requested_stream.next() => {
                            if let Some(()) = save_request {
                                let debounce_task = debounce_task.clone();
                                let theme_actor = theme_actor.clone();
                                let dock_mode_actor = dock_mode_actor.clone();
                                let files_width_right_actor = files_width_right_actor.clone();
                                let files_height_right_actor = files_height_right_actor.clone();
                                let files_width_bottom_actor = files_width_bottom_actor.clone();
                                let files_height_bottom_actor = files_height_bottom_actor.clone();
                                let variables_width_actor = variables_width_actor.clone();
                                let timeline_height_actor = timeline_height_actor.clone();
                                let name_width_actor = name_width_actor.clone();
                                let value_width_actor = value_width_actor.clone();
                                let session_actor = session_actor.clone();
                                let toast_actor = toast_actor.clone();

                                *debounce_task.lock().unwrap() = None;

                                let handle = Task::start_droppable(async move {
                                    Timer::sleep(1000).await;

                                    let theme = theme_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let dock_mode = dock_mode_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let files_width_right = files_width_right_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let files_height_right = files_height_right_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let files_width_bottom = files_width_bottom_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let files_height_bottom = files_height_bottom_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let variables_width = variables_width_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let timeline_height = timeline_height_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let name_width = name_width_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let value_width = value_width_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let session = session_actor.signal().to_stream().next().await.unwrap_or_default();
                                    let toast_dismiss_ms = toast_actor.signal().to_stream().next().await.unwrap_or_default();

                                    let shared_config = shared::AppConfig {
                                        app: shared::AppSection::default(),
                                        workspace: shared::WorkspaceSection {
                                            opened_files: session.opened_files,
                                            docked_bottom_dimensions: shared::DockedBottomDimensions {
                                                files_and_scopes_panel_width: files_width_bottom as f64,
                                                files_and_scopes_panel_height: files_height_bottom as f64,
                                                selected_variables_panel_name_column_width: Some(name_width as f64),
                                                selected_variables_panel_value_column_width: Some(value_width as f64),
                                            },
                                            docked_right_dimensions: shared::DockedRightDimensions {
                                                files_and_scopes_panel_width: files_width_right as f64,
                                                files_and_scopes_panel_height: files_height_right as f64,
                                                selected_variables_panel_name_column_width: Some(name_width as f64),
                                                selected_variables_panel_value_column_width: Some(value_width as f64),
                                            },
                                            dock_mode,
                                            expanded_scopes: session.file_picker_expanded_directories.clone(),
                                            load_files_expanded_directories: session.file_picker_expanded_directories,
                                            selected_scope_id: None,
                                            load_files_scroll_position: session.file_picker_scroll_position,
                                            variables_search_filter: session.variables_search_filter,
                                            selected_variables: Vec::new(),
                                            timeline_cursor_position_ns: 0,
                                            timeline_visible_range_start_ns: None,
                                            timeline_visible_range_end_ns: None,
                                            timeline_zoom_level: 1.0,
                                        },
                                        ui: shared::UiSection {
                                            theme,
                                            toast_dismiss_ms: toast_dismiss_ms as u64,
                                        },
                                    };

                                    let _ = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await;
                                });

                                *debounce_task.lock().unwrap() = Some(handle);
                            }
                        }
                    }
                }
            }
        });

        let file_picker_expanded_directories = {
            let mut expanded_set = indexmap::IndexSet::new();
            for dir in &config.workspace.load_files_expanded_directories {
                expanded_set.insert(dir.clone());
            }
            Mutable::new(expanded_set)
        };

        let file_picker_scroll_position = Mutable::new(config.workspace.load_files_scroll_position);

        let expanded_sync_actor = Actor::new((), {
            let file_picker_sync = file_picker_expanded_directories.clone();
            let session_sync = session_state_actor.clone();
            let session_changed_relay = session_state_changed_relay.clone();
            
            async move |_state| {
                let mut expanded_stream = file_picker_sync.signal_cloned().to_stream();
                
                while let Some(expanded_set) = expanded_stream.next().await {
                    if let Some(mut session_state) = session_sync.signal().to_stream().next().await {
                        session_state.file_picker_expanded_directories = 
                            expanded_set.iter().cloned().collect();
                        session_changed_relay.send(session_state);
                    }
                }
            }
        });

        let scroll_sync_actor = Actor::new((), {
            let scroll_position_sync = file_picker_scroll_position.clone();
            let session_scroll_sync = session_state_actor.clone();
            let session_scroll_changed_relay = session_state_changed_relay.clone();
            
            async move |_state| {
                let mut scroll_stream = scroll_position_sync.signal().to_stream();
                
                while let Some(scroll_position) = scroll_stream.next().await {
                    if let Some(mut session_state) = session_scroll_sync.signal().to_stream().next().await {
                        session_state.file_picker_scroll_position = scroll_position;
                        session_scroll_changed_relay.send(session_state);
                    }
                }
            }
        });

        let clipboard_actor = Actor::new((), async move |_state| {
            while let Some(text) = clipboard_copy_requested_stream.next().await {
                // Use spawn_local within Actor to handle WASM clipboard operations
                spawn_local(async move {
                    if let Some(window) = web_sys::window() {
                        let navigator = window.navigator();
                        
                        #[cfg(web_sys_unstable_apis)]
                        {
                            let clipboard = navigator.clipboard();
                            match JsFuture::from(clipboard.write_text(&text)).await {
                                Ok(_) => {
                                    // Clipboard copy successful
                                }
                                Err(e) => {
                                    zoon::println!("Clipboard error: {:?}", e);
                                }
                            }
                        }
                        
                        #[cfg(not(web_sys_unstable_apis))]
                        {
                            zoon::println!("Clipboard error: Clipboard API requires unstable APIs flag");
                        }
                    }
                });
            }
        });

        let error_display = crate::error_display::ErrorDisplay::new().await;

        Self {
            theme_actor,
            dock_mode_actor,
            files_panel_width_right_actor,
            files_panel_height_right_actor,
            files_panel_width_bottom_actor,
            files_panel_height_bottom_actor,
            variables_panel_width_actor,
            timeline_panel_height_actor,
            variables_name_column_width_actor,
            variables_value_column_width_actor,
            session_state_actor,
            toast_dismiss_ms_actor,

            file_picker_expanded_directories,
            file_picker_scroll_position,

            loaded_selected_variables: config.workspace.selected_variables.clone(),

            theme_button_clicked_relay,
            dock_mode_button_clicked_relay,
            variables_filter_changed_relay,
            files_width_right_changed_relay,
            files_height_right_changed_relay,
            files_width_bottom_changed_relay,
            files_height_bottom_changed_relay,
            variables_width_changed_relay,
            timeline_height_changed_relay,
            name_column_width_changed_relay,
            value_column_width_changed_relay,
            session_state_changed_relay,
            config_save_requested_relay,
            
            clipboard_copy_requested_relay,
            
            error_display,

            _config_saver_actor: config_saver_actor,
            _expanded_sync_actor: expanded_sync_actor,
            _scroll_sync_actor: scroll_sync_actor,
            _clipboard_actor: clipboard_actor,
        }
    }
}