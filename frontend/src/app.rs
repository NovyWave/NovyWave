//! NovyWaveApp - Self-contained Actor+Relay Architecture

use futures::select;
use zoon::*;
use zoon::events::KeyDown;
use moonzoon_novyui::{ButtonVariant, ButtonSize, IconName};
use std::sync::OnceLock;

use crate::config::AppConfig;
use crate::dataflow::atom::Atom;
use crate::dataflow::{Relay, relay};
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::WaveformTimeline;
use shared::{DownMsg, UpMsg};

/// Global message router for DirectoryContents/DirectoryError routing
pub struct MessageRouter {
    file_picker_domain: crate::config::FilePickerDomain,
}

impl MessageRouter {
    pub fn new(file_picker_domain: crate::config::FilePickerDomain) -> Self {
        Self { file_picker_domain }
    }

    pub fn route_directory_contents(&self, path: String, items: Vec<shared::FileSystemItem>) {
        zoon::println!("📨 ROUTER: Routing DirectoryContents to FilePickerDomain");
        self.file_picker_domain.directory_contents_received_relay.send((path, items));
    }

    pub fn route_directory_error(&self, path: String, error: String) {
        zoon::println!("📨 ROUTER: Routing DirectoryError to FilePickerDomain");
        self.file_picker_domain.directory_error_received_relay.send((path, error));
    }
}

static MESSAGE_ROUTER: OnceLock<MessageRouter> = OnceLock::new();

/// Global config store for ConfigLoaded message routing
static CONFIG_STORE: OnceLock<crate::config::AppConfig> = OnceLock::new();

// Import from extracted modules
use crate::action_buttons::load_files_button_with_progress;
use crate::file_management::files_panel;
use crate::file_picker::file_paths_dialog;

/// Self-contained NovyWave application
pub struct NovyWaveApp {
    /// File tracking and management domain
    pub tracked_files: TrackedFiles,

    /// Variable selection and scope management domain
    pub selected_variables: SelectedVariables,

    /// Timeline, cursor, and waveform visualization domain
    pub waveform_timeline: WaveformTimeline,

    /// Waveform canvas rendering and state domain
    pub waveform_canvas: crate::visualizer::canvas::waveform_canvas::WaveformCanvas,

    /// Application configuration (theme, panels, etc.)
    pub config: AppConfig,
    
    /// Panel dragging and resizing system
    pub dragging_system: crate::dragging::DraggingSystem,

    /// Backend communication connection (Arc for cloning)
    pub connection: std::sync::Arc<SendWrapper<Connection<UpMsg, DownMsg>>>,

    // === UI STATE (Atom pattern for local UI concerns) ===
    /// File picker dialog visibility
    pub file_dialog_visible: Atom<bool>,

    /// Current search filter text in various panels
    pub search_filter: Atom<String>,

    /// Loading/error states for UI feedback
    pub app_loading: Atom<bool>,
    pub error_message: Atom<Option<String>>,


    // === EVENT-SOURCE RELAYS (app-level events) ===
    /// App initialization completed
    pub app_initialized_relay: Relay<()>,

    /// File dialog open/close requested
    pub file_dialog_toggle_requested_relay: Relay<()>,

    /// Global error occurred (for toast notifications)
    pub error_occurred_relay: Relay<String>,

    /// App shutdown requested
    pub shutdown_requested_relay: Relay<()>,
}

// Remove Default implementation - use new() method instead

impl NovyWaveApp {
    /// Create new NovyWaveApp with proper async initialization
    ///
    /// This replaces the complex global domain initialization from main.rs
    pub async fn new() -> Self {
        zoon::println!("🔥 APP: Starting NovyWaveApp::new() initialization");

        // Load fonts first
        Self::load_and_register_fonts().await;

        let tracked_files = TrackedFiles::new().await;
        let selected_variables = SelectedVariables::new().await;

        // Create MaximumTimelineRange standalone actor for centralized range computation
        let maximum_timeline_range = crate::visualizer::timeline::MaximumTimelineRange::new(
            tracked_files.clone(),
            selected_variables.clone(),
        ).await;

        let waveform_timeline = WaveformTimeline::new(maximum_timeline_range).await;

        // Initialize waveform canvas rendering domain
        let waveform_canvas = crate::visualizer::canvas::waveform_canvas::WaveformCanvas::new().await;

        // ✅ FIX: Create main connection FIRST before any config initialization
        let connection = Self::create_connection_with_domain_integration(
            &tracked_files,
        )
        .await;

        // Initialize platform layer with the connection
        let connection_arc = std::sync::Arc::new(connection);
        crate::platform::set_platform_connection(connection_arc.clone());

        // Create main config with proper connection
        let config = AppConfig::new(connection_arc.clone()).await;

        // Initialize message router for DirectoryContents routing
        let message_router = MessageRouter::new(config.file_picker_domain.clone());
        if MESSAGE_ROUTER.set(message_router).is_err() {
            zoon::println!("❌ APP: Failed to set MESSAGE_ROUTER - already initialized");
        } else {
            zoon::println!("✅ APP: MESSAGE_ROUTER initialized successfully");
        }

        // Initialize config store for ConfigLoaded message routing
        if CONFIG_STORE.set(config.clone()).is_err() {
            zoon::println!("❌ APP: Failed to set CONFIG_STORE - already initialized");
        } else {
            zoon::println!("✅ APP: CONFIG_STORE initialized successfully");
        }

        // Initialize dragging system after config is ready
        let dragging_system = crate::dragging::DraggingSystem::new(config.clone()).await;

        // Request config loading through platform layer
        zoon::println!("🔄 APP: Requesting config load through platform layer");
        if let Err(e) = <crate::platform::CurrentPlatform as crate::platform::Platform>::send_message(shared::UpMsg::LoadConfig).await {
            zoon::println!("❌ APP: Failed to request config load: {:?}", e);
        }

        let (shutdown_requested_relay, mut shutdown_requested_stream) = relay();
        let (app_initialized_relay, _app_initialized_stream) = relay();
        let (file_dialog_toggle_requested_relay, _file_dialog_toggle_requested_stream) = relay();
        let (error_occurred_relay, _error_occurred_stream) = relay();

        let file_dialog_visible = Atom::new(false);
        let search_filter = Atom::new(String::new());
        let app_loading = Atom::new(false); // Initialization complete
        let error_message = Atom::new(None);

        Self::setup_app_coordination(
            &selected_variables,
            &config,
        )
        .await;

        NovyWaveApp {
            tracked_files,
            selected_variables,
            waveform_timeline,
            waveform_canvas,
            config,
            dragging_system,
            connection: connection_arc,
            file_dialog_visible,
            search_filter,
            app_loading,
            error_message,
            app_initialized_relay,
            file_dialog_toggle_requested_relay,
            error_occurred_relay,
            shutdown_requested_relay,
        }
    }

    /// Load and register fonts (from main.rs)
    async fn load_and_register_fonts() {
        use zoon::futures_util::future::try_join_all;

        let fonts = try_join_all([
            fast2d::fetch_file("/_api/public/fonts/FiraCode-Regular.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Regular.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Bold.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-Italic.ttf"),
            fast2d::fetch_file("/_api/public/fonts/Inter-BoldItalic.ttf"),
        ])
        .await
        .unwrap_throw();

        fast2d::register_fonts(fonts).unwrap_throw();
    }

    /// Create connection with TrackedFiles integration (DirectoryContents will be handled later)
    async fn create_connection_with_domain_integration(
        tracked_files: &TrackedFiles,
    ) -> SendWrapper<Connection<UpMsg, DownMsg>> {

        let tf_relay = tracked_files.file_load_completed_relay.clone();

        let connection = Connection::new(move |down_msg, _| {
            // Handle backend messages with direct domain access
            zoon::println!("🔍 APP: Received message: {}", match &down_msg {
                DownMsg::DirectoryContents { path, items } => format!("DirectoryContents(path={}, items={})", path, items.len()),
                DownMsg::DirectoryError { path, error } => format!("DirectoryError(path={}, error={})", path, error),
                DownMsg::ConfigLoaded(_) => "ConfigLoaded".to_string(),
                DownMsg::ConfigSaved => "ConfigSaved".to_string(),
                DownMsg::FileLoaded { file_id, .. } => format!("FileLoaded({})", file_id),
                _ => format!("Other({:?})", std::mem::discriminant(&down_msg))
            });
            match down_msg {
                DownMsg::FileLoaded { file_id, hierarchy } => {
                    // Send to TrackedFiles domain directly
                    if let Some(loaded_file) = hierarchy.files.first() {
                        tf_relay.send((file_id, shared::FileState::Loaded(loaded_file.clone())));
                    }
                }
                DownMsg::ParsingStarted {
                    file_id,
                    filename: _,
                } => {
                    tf_relay.send((
                        file_id,
                        shared::FileState::Loading(shared::LoadingStatus::Parsing),
                    ));
                }
                DownMsg::DirectoryContents { path, items } => {
                    zoon::println!("📥 APP: Received DirectoryContents for {} with {} items", path, items.len());
                    zoon::println!("📥 APP: Items: {:?}", items.iter().map(|i| &i.name).collect::<Vec<_>>());
                    // Route to FilePickerDomain through global router (will be set after config creation)
                    if let Some(router) = MESSAGE_ROUTER.get() {
                        router.route_directory_contents(path, items);
                    } else {
                        zoon::println!("📨 APP: DirectoryContents received but router not initialized yet");
                    }
                }
                DownMsg::DirectoryError { path, error } => {
                    zoon::println!("❌ APP: DirectoryError for {}: {}", path, error);
                    // Route to FilePickerDomain through global router (will be set after config creation)
                    if let Some(router) = MESSAGE_ROUTER.get() {
                        router.route_directory_error(path, error);
                    } else {
                        zoon::println!("📨 APP: DirectoryError received but router not initialized yet");
                    }
                }
                DownMsg::ConfigLoaded(loaded_config) => {
                    zoon::println!("🎉 APP: Config loaded from backend");
                    // Route to AppConfig through global config store (will be set after config creation)
                    if let Some(app_config) = CONFIG_STORE.get() {
                        zoon::println!("🔄 APP: Calling update_from_loaded_config with backend data");
                        app_config.update_from_loaded_config(loaded_config);
                    } else {
                        zoon::println!("📨 APP: ConfigLoaded received but config store not initialized yet");
                    }
                }
                // ... other message handling
                _ => {
                    zoon::println!("🔍 APP: Unhandled message: {:?}", std::mem::discriminant(&down_msg));
                }
            }
        });
        SendWrapper::new(connection)
    }

    /// Setup app-level coordination
    async fn setup_app_coordination(
        selected_variables: &SelectedVariables,
        config: &AppConfig,
    ) {

        // Restore selected variables from config
        if !config.loaded_selected_variables.is_empty() {
            selected_variables
                .variables_restored_relay
                .send(config.loaded_selected_variables.clone());
        }
    }

    /// Root UI element
    pub fn root(&self) -> impl Element {
        Stack::new()
            .s(Height::screen())
            .s(Width::fill())
            .s(
                Background::new().color_signal(self.config.theme_actor.signal().map(|theme| {
                    match theme {
                        shared::Theme::Light => "rgb(255, 255, 255)",
                        shared::Theme::Dark => "rgb(13, 13, 13)",
                    }
                })),
            )
            .s(Font::new().family([
                FontFamily::new("Inter"),
                FontFamily::new("system-ui"),
                FontFamily::new("Segoe UI"),
                FontFamily::new("Arial"),
                FontFamily::SansSerif,
            ]))
            .update_raw_el({
                let theme_relay = self.config.theme_button_clicked_relay.clone();
                let dock_relay = self.config.dock_mode_button_clicked_relay.clone();

                // Timeline navigation relays
                let timeline = self.waveform_timeline.clone();
                let left_key_pressed = timeline.left_key_pressed_relay.clone();
                let right_key_pressed = timeline.right_key_pressed_relay.clone();
                let zoom_in_pressed = timeline.zoom_in_pressed_relay.clone();
                let zoom_out_pressed = timeline.zoom_out_pressed_relay.clone();
                let pan_left_pressed = timeline.pan_left_pressed_relay.clone();
                let pan_right_pressed = timeline.pan_right_pressed_relay.clone();
                let jump_to_previous = timeline.jump_to_previous_pressed_relay.clone();
                let jump_to_next = timeline.jump_to_next_pressed_relay.clone();
                let reset_zoom_center = timeline.reset_zoom_center_pressed_relay.clone();
                let reset_zoom = timeline.reset_zoom_pressed_relay.clone();
                let shift_key_pressed = timeline.shift_key_pressed_relay.clone();
                let shift_key_released = timeline.shift_key_released_relay.clone();

                move |raw_el| {
                    raw_el.global_event_handler(move |event: KeyDown| {
                        // Check if the active element is an input/textarea to disable shortcuts
                        // This prevents conflicts when user is typing in input fields
                        let should_handle_shortcuts = if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                if let Some(active_element) = document.active_element() {
                                    let tag_name = active_element.tag_name().to_lowercase();
                                    // Disable shortcuts when input fields are focused
                                    !matches!(tag_name.as_str(), "input" | "textarea")
                                } else {
                                    true // No active element, allow shortcuts
                                }
                            } else {
                                true // No document, allow shortcuts
                            }
                        } else {
                            true // No window, allow shortcuts
                        };

                        if !should_handle_shortcuts {
                            return;
                        }

                        // Handle Shift key tracking
                        if event.shift_key() {
                            shift_key_pressed.send(());
                        }

                        // Handle keyboard shortcuts
                        if event.ctrl_key() {
                            match event.key().as_str() {
                                "t" | "T" => {
                                    event.prevent_default();
                                    theme_relay.send(());
                                }
                                "d" | "D" => {
                                    event.prevent_default();
                                    dock_relay.send(());
                                }
                                _ => {}
                            }
                        } else {
                            // Timeline navigation shortcuts (without Ctrl)
                            match event.key().as_str() {
                                // Cursor Movement
                                "q" | "Q" => {
                                    event.prevent_default();
                                    if event.shift_key() {
                                        jump_to_previous.send(());
                                    } else {
                                        left_key_pressed.send(());
                                    }
                                }
                                "e" | "E" => {
                                    event.prevent_default();
                                    if event.shift_key() {
                                        jump_to_next.send(());
                                    } else {
                                        right_key_pressed.send(());
                                    }
                                }

                                // Viewport Panning
                                "a" | "A" => {
                                    event.prevent_default();
                                    pan_left_pressed.send(());
                                    // TODO: Add Shift+A support for faster panning when shift handling is implemented
                                }
                                "d" | "D" => {
                                    event.prevent_default();
                                    pan_right_pressed.send(());
                                    // TODO: Add Shift+D support for faster panning when shift handling is implemented
                                }

                                // Zoom Controls
                                "w" | "W" => {
                                    event.prevent_default();
                                    zoom_in_pressed.send(());
                                    // TODO: Add Shift+W support for faster zoom when shift handling is implemented
                                }
                                "s" | "S" => {
                                    event.prevent_default();
                                    zoom_out_pressed.send(());
                                    // TODO: Add Shift+S support for faster zoom when shift handling is implemented
                                }

                                // Reset Controls
                                "z" | "Z" => {
                                    event.prevent_default();
                                    reset_zoom_center.send(());
                                }
                                "r" | "R" => {
                                    event.prevent_default();
                                    reset_zoom.send(());
                                }

                                _ => {}
                            }
                        }
                    })
                }
            })
            .layer(self.main_layout())
            .layer_signal(self.file_dialog_visible.signal().map_true({
                let tracked_files = self.tracked_files.clone();
                let selected_variables = self.selected_variables.clone();
                let config = self.config.clone();
                let file_dialog_visible = self.file_dialog_visible.clone();
                let connection = self.connection.clone();
                move || {
                    file_paths_dialog(
                        tracked_files.clone(),
                        selected_variables.clone(),
                        config.clone(),
                        file_dialog_visible.clone(),
                        crate::connection::ConnectionAdapter::from_arc(connection.clone())
                    )
                }
            }))
            .layer(self.toast_notifications_container())
    }

    /// Main layout
    fn main_layout(&self) -> impl Element {
        crate::main_layout(
            &self.tracked_files,
            &self.selected_variables,
            &self.waveform_timeline,
            &self.config,
            &self.dragging_system,
            &self.waveform_canvas,
            &self.file_dialog_visible,
        )
    }



    fn toast_notifications_container(&self) -> impl Element {
        crate::error_ui::toast_notifications_container(self.config.clone())
    }

}
