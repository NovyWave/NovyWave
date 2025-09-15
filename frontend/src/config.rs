use crate::dataflow::{Actor, Relay, relay};
use crate::platform::{CurrentPlatform, Platform};
use crate::visualizer::timeline::TimeNs;
use futures::{StreamExt, FutureExt, select, stream::FusedStream};
use serde::{Deserialize, Serialize};
use shared::{UpMsg, DownMsg};
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

/// File picker domain with proper Actor+Relay architecture
#[derive(Clone)]
pub struct FilePickerDomain {
    pub expanded_directories_actor: Actor<indexmap::IndexSet<String>>,
    pub scroll_position_actor: Actor<i32>,
    pub directory_cache_actor: Actor<std::collections::HashMap<String, Vec<shared::FileSystemItem>>>,
    pub directory_errors_actor: Actor<std::collections::HashMap<String, String>>,
    pub directory_loading_actor: Actor<std::collections::HashSet<String>>,
    pub backend_sender_actor: Actor<()>,

    // Event-source relays for UI interactions
    pub directory_expanded_relay: Relay<String>,
    pub directory_collapsed_relay: Relay<String>,
    pub scroll_position_changed_relay: Relay<i32>,
    pub config_save_requested_relay: Relay,

    // Internal relays for async operations (replaces zoon::Task)
    pub directory_load_requested_relay: Relay<String>,
}

impl FilePickerDomain {
    pub async fn new(
        initial_expanded: indexmap::IndexSet<String>,
        initial_scroll: i32,
        config_save_relay: Relay,
        connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        connection_message_actor: &crate::app::ConnectionMessageActor,
    ) -> Self {
        let (directory_expanded_relay, mut directory_expanded_stream) = relay::<String>();
        let (directory_collapsed_relay, mut directory_collapsed_stream) = relay::<String>();
        let (scroll_position_changed_relay, mut scroll_position_changed_stream) = relay::<i32>();
        let (directory_load_requested_relay, mut directory_load_requested_stream) = relay::<String>();

        // ✅ ACTOR+RELAY: Subscribe to ConnectionMessageActor relays instead of internal ones
        let mut directory_contents_received_stream = connection_message_actor.directory_contents_relay.subscribe();
        let mut directory_error_received_stream = connection_message_actor.directory_error_relay.subscribe();
        let config_save_requested_relay = config_save_relay;

        let expanded_directories_actor = Actor::new(initial_expanded, {
            let save_relay = config_save_requested_relay.clone();
            async move |state| {
                let mut expanded_stream = directory_expanded_stream.fuse();
                let mut collapsed_stream = directory_collapsed_stream.fuse();

                loop {
                    futures::select! {
                        dir = expanded_stream.next() => {
                            if let Some(dir) = dir {
                                let mut current = state.get_cloned();
                                if current.insert(dir) {
                                    state.set_neq(current);
                                    save_relay.send(()); // Trigger config save
                                }
                            }
                        }
                        dir = collapsed_stream.next() => {
                            if let Some(dir) = dir {
                                let mut current = state.get_cloned();
                                if current.shift_remove(&dir) {
                                    state.set_neq(current);
                                    save_relay.send(()); // Trigger config save
                                }
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let scroll_position_actor = Actor::new(initial_scroll, {
            let save_relay = config_save_requested_relay.clone();
            async move |state| {
                let mut position_stream = scroll_position_changed_stream.fuse();

                loop {
                    futures::select! {
                        position = position_stream.next() => {
                            if let Some(position) = position {
                                state.set_neq(position);
                                save_relay.send(()); // Trigger config save
                            }
                        }
                        complete => break,
                    }
                }
            }
        });


        zoon::println!("🏗️ DIRECTORY_CACHE_ACTOR: Creating new cache Actor instance");
        let directory_cache_actor = Actor::new(std::collections::HashMap::new(), {
            async move |state| {
                loop {
                    futures::select! {
                        contents = directory_contents_received_stream.next() => {
                            if let Some((path, items)) = contents {
                                zoon::println!("📦 DIRECTORY_CACHE_ACTOR: Received {} items for path: {}", items.len(), path);

                                // Use set_neq with proper change detection - this MUST trigger signals
                                let mut cache = state.get_cloned();
                                cache.insert(path.clone(), items);
                                let cache_size = cache.len();

                                state.set_neq(cache);
                                zoon::println!("📦 DIRECTORY_CACHE_ACTOR: Called set_neq(), cache now contains {} paths", cache_size);
                                zoon::println!("📦 DIRECTORY_CACHE_ACTOR: Signal should fire to update TreeView");
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        let directory_errors_actor = Actor::new(std::collections::HashMap::new(), async move |state| {
            loop {
                futures::select! {
                    error = directory_error_received_stream.next() => {
                        if let Some((path, error_message)) = error {
                            let mut errors = state.get_cloned();
                            errors.insert(path, error_message);
                            state.set_neq(errors);
                        }
                    }
                    complete => break,
                }
            }
        });

        // ✅ ACTOR+RELAY FIX: Add directory loading Actor to handle load requests
        // Since connection can't be used directly in Actor (Send trait issues), we use a different approach:
        // The Actor tracks loading requests and the UI layer polls for pending requests
        let directory_loading_actor = Actor::new(std::collections::HashSet::<String>::new(), {
            async move |state| {
                loop {
                    futures::select! {
                        requested_path = directory_load_requested_stream.next() => {
                            if let Some(path) = requested_path {
                                zoon::println!("📁 DIRECTORY_LOADING_ACTOR: Queuing load request for: {}", path);
                                let mut pending_requests = state.get_cloned();
                                pending_requests.insert(path);
                                state.set_neq(pending_requests);
                            }
                        }
                        complete => break,
                    }
                }
            }
        });

        // ✅ ACTOR+RELAY PATTERN: Backend request sender using nested Actor pattern
        let backend_sender_actor = {
            let connection_clone = connection.clone();
            let directory_cache_for_sender = directory_cache_actor.clone();
            let directory_loading_for_sender = directory_loading_actor.clone();

            Actor::new((), async move |_state| {
                let mut directory_loading_stream = directory_loading_for_sender.signal().to_stream().fuse();

                loop {
                    if let Some(pending_requests) = directory_loading_stream.next().await {
                        zoon::println!("📤 BACKEND_SENDER: Processing {} pending requests", pending_requests.len());

                        // Check cache to avoid sending requests for directories that already have data
                        let current_cache = directory_cache_for_sender.signal().to_stream().next().await.unwrap_or_default();

                        for request_path in pending_requests.iter() {
                            if !current_cache.contains_key(request_path) {
                                zoon::println!("📤 BACKEND_SENDER: Sending directory request for: {}", request_path);
                                let path_for_request = request_path.clone();

                                // Connection requires async handling within Actor context
                                connection_clone.send_up_msg(shared::UpMsg::BrowseDirectory(path_for_request)).await.unwrap_throw();
                            } else {
                                zoon::println!("📤 BACKEND_SENDER: Skipping request for {} (already in cache)", request_path);
                            }
                        }
                    } else {
                        // Stream ended
                        break;
                    }
                }
            })
        };

        Self {
            expanded_directories_actor,
            scroll_position_actor,
            directory_cache_actor,
            directory_errors_actor,
            directory_loading_actor,
            backend_sender_actor,
            directory_expanded_relay,
            directory_collapsed_relay,
            scroll_position_changed_relay,
            config_save_requested_relay,
            directory_load_requested_relay,
        }
    }

    pub fn expanded_directories_signal(&self) -> impl Signal<Item = indexmap::IndexSet<String>> {
        self.expanded_directories_actor.signal()
    }

    pub fn scroll_position_signal(&self) -> impl Signal<Item = i32> {
        self.scroll_position_actor.signal()
    }

    pub fn directory_cache_signal(&self) -> impl Signal<Item = std::collections::HashMap<String, Vec<shared::FileSystemItem>>> + use<> {
        self.directory_cache_actor.signal()
    }

    pub fn directory_errors_signal(&self) -> impl Signal<Item = std::collections::HashMap<String, String>> {
        self.directory_errors_actor.signal()
    }

    pub fn pending_directory_loads_signal(&self) -> impl Signal<Item = std::collections::HashSet<String>> {
        self.directory_loading_actor.signal()
    }
}

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

    // File picker domain with proper Actor+Relay architecture
    pub file_picker_domain: FilePickerDomain,

    pub loaded_selected_variables: Vec<shared::SelectedVariable>,

    pub theme_button_clicked_relay: Relay,
    pub dock_mode_button_clicked_relay: Relay,
    pub theme_changed_relay: Relay<SharedTheme>,
    pub dock_mode_changed_relay: Relay<DockMode>,
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
    _expanded_sync_actor: Actor<()>,
    _scroll_sync_actor: Actor<()>,
    _clipboard_actor: Actor<()>,
    _save_trigger_actor: Actor<()>,
    _config_save_debouncer_actor: Actor<()>,
    _config_loaded_actor: Actor<()>,
}

impl AppConfig {
    async fn load_config_from_backend() -> Result<SharedAppConfig, String> {
        // Platform layer fallback - using defaults until proper backend config loading
        zoon::println!("⚙️ CONFIG: Using default configuration");
        Ok(SharedAppConfig::default())
    }

    pub async fn new(
        connection: std::sync::Arc<SendWrapper<Connection<shared::UpMsg, shared::DownMsg>>>,
        connection_message_actor: crate::app::ConnectionMessageActor,
    ) -> Self {
        zoon::println!("🚀 CONFIG: Starting AppConfig::new() initialization");
        let config = Self::load_config_from_backend()
            .await
            .unwrap_or_else(|_error| SharedAppConfig::default());

        let (theme_button_clicked_relay, mut theme_button_clicked_stream) = relay();
        let (dock_mode_button_clicked_relay, mut dock_mode_button_clicked_stream) = relay();
        let (theme_changed_relay, mut theme_changed_stream) = relay();
        let (dock_mode_changed_relay, mut dock_mode_changed_stream) = relay();
        let (variables_filter_changed_relay, variables_filter_changed_stream) = relay();
        let (session_state_changed_relay, session_state_changed_stream) = relay::<SessionState>();
        let session_state_changed_stream_for_config_saver = session_state_changed_relay.subscribe();
        let session_state_changed_stream_for_session_actor = session_state_changed_relay.subscribe();
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

                            // Config save handled by Task-based ConfigSaver
                            zoon::println!("💾 THEME_ACTOR: Theme button clicked, Task-based config saver will handle save");
                        }
                    }
                    direct_change = theme_changed_stream.next() => {
                        if let Some(new_theme) = direct_change {
                            current_theme = new_theme;
                            state.set(new_theme);

                            let novyui_theme = match new_theme {
                                SharedTheme::Light => theme::Theme::Light,
                                SharedTheme::Dark => theme::Theme::Dark,
                            };
                            theme::set_theme(novyui_theme);
                        }
                    }
                    complete => break,
                }
            }
        });

        zoon::println!("✅ CONFIG: Theme actor created successfully");

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
                    direct_change = dock_mode_changed_stream.next() => {
                        if let Some(new_mode) = direct_change {
                            current_dock_mode = new_mode;
                            state.set(new_mode);
                        }
                    }
                    complete => break,
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
                let mut session_stream = session_state_changed_stream_for_session_actor;
                let mut variables_filter_stream = variables_filter_changed_stream;

                loop {
                    select! {
                        session_change = session_stream.next() => {
                            if let Some(new_session) = session_change {
                                zoon::println!("🗃️ SESSION_ACTOR: Received session update with {} expanded directories",
                                    new_session.file_picker_expanded_directories.len());
                                for dir in &new_session.file_picker_expanded_directories {
                                    zoon::println!("🗃️ SESSION_ACTOR: Expanded dir: {}", dir);
                                }
                                state.set_neq(new_session);
                                zoon::println!("🗃️ SESSION_ACTOR: Session state updated successfully");
                            }
                        }
                        filter_change = variables_filter_stream.next() => {
                            if let Some(new_filter) = filter_change {
                                zoon::println!("🔍 SESSION_ACTOR: Received filter update: {}", new_filter);
                                state.update_mut(|session| {
                                    session.variables_search_filter = new_filter;
                                });
                            }
                        }
                        complete => break,
                    }
                }
            }
        );

        let toast_dismiss_ms_actor = Actor::new(config.ui.toast_dismiss_ms as u32, async move |_state| {
            loop {
                Task::next_macro_tick().await;
            }
        });

        // Clone actors for config_saver_actor closure
        let theme_actor_clone = theme_actor.clone();
        let dock_mode_actor_clone = dock_mode_actor.clone();
        let session_actor_clone = session_state_actor.clone();
        let toast_actor_clone = toast_dismiss_ms_actor.clone();

        // FIX: Connect ConfigSaver to button click relays for immediate save trigger
        zoon::println!("🔧 CONFIG_SAVER: Setting up button-click-based config saving");

        // Clone the relay for struct return since it will be moved into Tasks
        let config_save_requested_relay_for_struct = config_save_requested_relay.clone();

        // Use Actor to handle config save triggering (eliminates zoon::Task)
        let save_trigger_actor = Actor::new((), {
            let theme_button_stream = theme_button_clicked_relay.subscribe();
            let dock_button_stream = dock_mode_button_clicked_relay.subscribe();
            let config_save_relay = config_save_requested_relay.clone();
            async move |_state| {
                let mut theme_stream = theme_button_stream;
                let mut dock_stream = dock_button_stream;

                loop {
                    select! {
                        result = theme_stream.next() => {
                            match result {
                                Some(_) => {
                                    zoon::println!("💾 CONFIG_SAVER: Theme button clicked, triggering save");
                                    config_save_relay.send(());
                                }
                                None => break, // Stream ended
                            }
                        }
                        result = dock_stream.next() => {
                            match result {
                                Some(_) => {
                                    zoon::println!("💾 CONFIG_SAVER: Dock mode button clicked, triggering save");
                                    config_save_relay.send(());
                                }
                                None => break, // Stream ended
                            }
                        }
                    }
                }
            }
        });

        // Create FilePickerDomain with proper Actor+Relay architecture
        let initial_expanded_set = {
            let mut expanded_set = indexmap::IndexSet::new();
            zoon::println!("🔍 CONFIG_LOAD: Loading {} expanded directories from config", config.workspace.load_files_expanded_directories.len());
            for dir in &config.workspace.load_files_expanded_directories {
                zoon::println!("🔍 CONFIG_LOAD: Restoring expanded directory: {}", dir);
                expanded_set.insert(dir.clone());
            }
            zoon::println!("🔍 CONFIG_LOAD: Final expanded set has {} directories", expanded_set.len());
            expanded_set
        };

        let file_picker_domain = FilePickerDomain::new(
            initial_expanded_set.clone(),
            config.workspace.load_files_scroll_position,
            config_save_requested_relay.clone(),
            connection,
            &connection_message_actor,
        ).await;

        // File picker changes now trigger config save through FilePickerDomain
        // Use nested Actor pattern for debouncing instead of Task::start
        let config_save_debouncer_actor = {
            let theme_actor_clone = theme_actor.clone();
            let dock_mode_actor_clone = dock_mode_actor.clone();
            let session_actor_clone = session_state_actor.clone();
            let toast_actor_clone = toast_dismiss_ms_actor.clone();
            let file_picker_domain_clone = file_picker_domain.clone();

            Actor::new((), async move |_state| {
                let mut config_save_requested_stream = config_save_requested_stream.fuse();

                loop {
                    select! {
                        result = config_save_requested_stream.next() => {
                            if let Some(()) = result {
                                // Debounce loop - wait for quiet period, cancelling if new request arrives
                                loop {
                                    select! {
                                        // New save request cancels timer
                                        result = config_save_requested_stream.next() => {
                                            if let Some(()) = result {
                                                continue; // Restart timer
                                            }
                                        }
                                        // Timer completes - do the save
                                        _ = zoon::Timer::sleep(300).fuse() => {
                                            let theme = theme_actor_clone.signal().to_stream().next().await.unwrap_or_default();
                                            let dock_mode = dock_mode_actor_clone.signal().to_stream().next().await.unwrap_or_default();
                                            let session = session_actor_clone.signal().to_stream().next().await.unwrap_or_default();
                                            let toast_dismiss_ms = toast_actor_clone.signal().to_stream().next().await.unwrap_or_default();

                                            // Get file picker data from domain instead of session
                                            let expanded_directories: Vec<String> = file_picker_domain_clone.expanded_directories_signal().to_stream().next().await.unwrap_or_default().into_iter().collect();
                                            let scroll_position = file_picker_domain_clone.scroll_position_signal().to_stream().next().await.unwrap_or_default();

                                            let shared_config = shared::AppConfig {
                                                app: shared::AppSection::default(),
                                                workspace: shared::WorkspaceSection {
                                                    opened_files: session.opened_files,
                                                    docked_bottom_dimensions: shared::DockedBottomDimensions {
                                                        files_and_scopes_panel_width: 470.0,
                                                        files_and_scopes_panel_height: 375.0,
                                                        selected_variables_panel_name_column_width: Some(338.0),
                                                        selected_variables_panel_value_column_width: Some(247.0),
                                                    },
                                                    docked_right_dimensions: shared::DockedRightDimensions {
                                                        files_and_scopes_panel_width: 528.0,
                                                        files_and_scopes_panel_height: 278.0,
                                                        selected_variables_panel_name_column_width: Some(177.0),
                                                        selected_variables_panel_value_column_width: Some(201.0),
                                                    },
                                                    dock_mode,
                                                    expanded_scopes: Vec::new(), // Reserved for Files & Scopes panel scope expansion
                                                    load_files_expanded_directories: expanded_directories,
                                                    selected_scope_id: None,
                                                    load_files_scroll_position: scroll_position,
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

                                        if let Err(e) = CurrentPlatform::send_message(UpMsg::SaveConfig(shared_config)).await {
                                            zoon::println!("❌ CONFIG_SAVER: Failed to save config: {:?}", e);
                                        } else {
                                            zoon::println!("💾 CONFIG_SAVER: Theme/dock config saved successfully");
                                        }
                                        break; // Back to outer loop
                                    }
                                }
                            }
                        }
                    }
                    }
                }
            })
        };

        // Complex bridge pattern removed - using direct FilePickerDomain events

        let scroll_sync_actor = Actor::new((), {
            let scroll_position_sync = file_picker_domain.scroll_position_actor.clone();
            let session_scroll_sync = session_state_actor.clone();
            let session_scroll_changed_relay = session_state_changed_relay.clone();

            async move |_state| {
                let mut scroll_stream = scroll_position_sync.signal().to_stream();

                while let Some(scroll_position) = scroll_stream.next().await {
                    // Create updated session with new scroll position
                    let current_session = session_scroll_sync.signal().to_stream().next().await.unwrap_or_default();
                    let updated_session = SessionState {
                        file_picker_scroll_position: scroll_position,
                        ..current_session
                    };
                    zoon::println!("💾 CONFIG_SYNC: Sending session update with scroll position {}", scroll_position);
                    session_scroll_changed_relay.send(updated_session);
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

        // ✅ ACTOR+RELAY: Subscribe to config_loaded_relay from ConnectionMessageActor
        let config_loaded_actor = {
            let config_loaded_stream = connection_message_actor.config_loaded_relay.subscribe();
            let theme_relay = theme_changed_relay.clone();
            let dock_relay = dock_mode_changed_relay.clone();
            let file_picker_domain_clone = file_picker_domain.clone();

            Actor::new((), async move |_state| {
                let mut config_stream = config_loaded_stream;

                while let Some(loaded_config) = config_stream.next().await {
                    zoon::println!("🔄 CONFIG_LOADED_ACTOR: Received config from ConnectionMessageActor");

                    // Update theme using proper relay
                    theme_relay.send(loaded_config.ui.theme);
                    zoon::println!("🎨 CONFIG_LOADED_ACTOR: Sent theme change to {:?}", loaded_config.ui.theme);

                    // Update dock mode using proper relay
                    dock_relay.send(loaded_config.workspace.dock_mode);
                    zoon::println!("📍 CONFIG_LOADED_ACTOR: Sent dock mode change to {:?}", loaded_config.workspace.dock_mode);

                    // Update expanded directories using FilePickerDomain
                    zoon::println!("📁 CONFIG_LOADED_ACTOR: Loading {} expanded directories", loaded_config.workspace.load_files_expanded_directories.len());
                    for dir in &loaded_config.workspace.load_files_expanded_directories {
                        zoon::println!("📁 CONFIG_LOADED_ACTOR: Expanding directory: {}", dir);
                        file_picker_domain_clone.directory_expanded_relay.send(dir.clone());
                    }

                    // Update scroll position using FilePickerDomain relay
                    file_picker_domain_clone.scroll_position_changed_relay.send(loaded_config.workspace.load_files_scroll_position);

                    zoon::println!("✅ CONFIG_LOADED_ACTOR: Successfully processed config via Actor+Relay");
                }
            })
        };

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

            file_picker_domain,

            loaded_selected_variables: config.workspace.selected_variables.clone(),

            theme_button_clicked_relay,
            dock_mode_button_clicked_relay,
            theme_changed_relay,
            dock_mode_changed_relay,
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
            config_save_requested_relay: config_save_requested_relay_for_struct,

            clipboard_copy_requested_relay,

            error_display,
            _expanded_sync_actor: Actor::new((), async move |_| { loop { Task::next_macro_tick().await; } }),
            _scroll_sync_actor: scroll_sync_actor,
            _clipboard_actor: clipboard_actor,
            _save_trigger_actor: save_trigger_actor,
            _config_save_debouncer_actor: config_save_debouncer_actor,
            _config_loaded_actor: config_loaded_actor,
        }
    }

    /// Update config from loaded backend data
    pub fn update_from_loaded_config(&self, loaded_config: shared::AppConfig) {
        zoon::println!("🔄 CONFIG: Updating from loaded backend config");

        // Update theme using proper relay (not direct state access)
        self.theme_changed_relay.send(loaded_config.ui.theme);
        zoon::println!("🎨 CONFIG: Sent theme change to {:?}", loaded_config.ui.theme);

        // Update dock mode using proper relay (not direct state access)
        self.dock_mode_changed_relay.send(loaded_config.workspace.dock_mode);
        zoon::println!("📍 CONFIG: Sent dock mode change to {:?}", loaded_config.workspace.dock_mode);

        // Update expanded directories using FilePickerDomain
        let mut expanded_set = indexmap::IndexSet::new();
        zoon::println!("📁 CONFIG: Loading {} expanded directories", loaded_config.workspace.load_files_expanded_directories.len());
        for dir in &loaded_config.workspace.load_files_expanded_directories {
            zoon::println!("📁 CONFIG: Adding expanded directory: {}", dir);
            expanded_set.insert(dir.clone());
        }
        // Use FilePickerDomain relays to update expanded directories
        for dir in &expanded_set {
            self.file_picker_domain.directory_expanded_relay.send(dir.clone());
        }

        // Update scroll position using FilePickerDomain relay
        self.file_picker_domain.scroll_position_changed_relay.send(loaded_config.workspace.load_files_scroll_position);

        zoon::println!("✅ CONFIG: Successfully updated all config from backend");
    }


}