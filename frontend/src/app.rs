//! NovyWaveApp - Self-contained Actor+Relay Architecture

use futures::{select, StreamExt};
use zoon::*;
use zoon::events::KeyDown;
use moonzoon_novyui::{ButtonVariant, ButtonSize, IconName};
use std::sync::OnceLock;

use crate::config::AppConfig;
use crate::dataflow::atom::Atom;
use crate::dataflow::{Relay, relay, Actor};
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::WaveformTimeline;
use shared::{DownMsg, UpMsg};


/// Actor+Relay replacement for global message routing
/// Transforms DownMsg stream into domain-specific relay streams
#[derive(Clone)]
pub struct ConnectionMessageActor {
    // Message-specific relays that domains can subscribe to
    pub config_loaded_relay: Relay<shared::AppConfig>,
    pub config_saved_relay: Relay<()>,
    pub directory_contents_relay: Relay<(String, Vec<shared::FileSystemItem>)>,
    pub directory_error_relay: Relay<(String, String)>,
    pub file_loaded_relay: Relay<(String, shared::FileState)>,
    pub parsing_started_relay: Relay<(String, String)>,

    // Actor handles message processing
    _message_actor: Actor<()>,
}

impl ConnectionMessageActor {
    /// Create ConnectionMessageActor with DownMsg stream from connection
    pub async fn new(mut down_msg_stream: impl futures::stream::Stream<Item = DownMsg> + Unpin + 'static) -> Self {
        zoon::println!("🏗️ CONNECTION_MSG_ACTOR: Starting new() initialization");

        // Create all message-specific relays
        let (config_loaded_relay, _) = relay();
        let (config_saved_relay, _) = relay();
        let (directory_contents_relay, _) = relay();
        let (directory_error_relay, _) = relay();
        let (file_loaded_relay, _) = relay();
        let (parsing_started_relay, _) = relay();

        zoon::println!("✅ CONNECTION_MSG_ACTOR: All relays created successfully");

        // Clone relays for use in Actor closure
        let config_loaded_relay_clone = config_loaded_relay.clone();
        let config_saved_relay_clone = config_saved_relay.clone();
        let directory_contents_relay_clone = directory_contents_relay.clone();
        let directory_error_relay_clone = directory_error_relay.clone();
        let file_loaded_relay_clone = file_loaded_relay.clone();
        let parsing_started_relay_clone = parsing_started_relay.clone();

        // Actor processes DownMsg stream and routes to appropriate relays
        zoon::println!("🚀 CONNECTION_MSG_ACTOR: Creating Actor with stream processing loop");
        let message_actor = Actor::new((), async move |_state| {
            zoon::println!("🔄 CONNECTION_MSG_ACTOR: Actor started, entering message loop");
            zoon::println!("🔄 CONNECTION_MSG_ACTOR: Stream moved into Actor successfully");

            let mut stream = down_msg_stream;
            let mut loop_counter = 0;
            loop {
                loop_counter += 1;
                zoon::println!("🔄 CONNECTION_MSG_ACTOR: Starting loop iteration {}", loop_counter);
                zoon::println!("⏳ CONNECTION_MSG_ACTOR: Waiting for next message...");

                use futures::StreamExt;
                match stream.next().await {
                        Some(down_msg) => {
                            zoon::println!("📨 CONNECTION_MSG_ACTOR: Received message in Actor loop");
                            // Route each message type to its specific relay
                            match down_msg {
                                DownMsg::ConfigLoaded(config) => {
                                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Routing ConfigLoaded");
                                    config_loaded_relay_clone.send(config);
                                }
                                DownMsg::ConfigSaved => {
                                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Routing ConfigSaved");
                                    config_saved_relay_clone.send(());
                                }
                                DownMsg::DirectoryContents { path, items } => {
                                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Routing DirectoryContents for path='{}' with {} items", path, items.len());
                                    zoon::println!("📊 CONNECTION_MSG_ACTOR: Items list for '{}': {:?}", path, items.iter().map(|item| format!("{}({})", item.name, if item.is_directory { "dir" } else { "file" })).collect::<Vec<_>>());
                                    let path_for_relay = path.clone();
                                    directory_contents_relay_clone.send((path, items));
                                    zoon::println!("✅ CONNECTION_MSG_ACTOR: Successfully sent DirectoryContents via relay for '{}'", path_for_relay);
                                }
                                DownMsg::DirectoryError { path, error } => {
                                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Routing DirectoryError for {}", path);
                                    directory_error_relay_clone.send((path, error));
                                }
                                DownMsg::FileLoaded { file_id, hierarchy } => {
                                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Routing FileLoaded for {}", file_id);
                                    if let Some(loaded_file) = hierarchy.files.first() {
                                        file_loaded_relay_clone.send((file_id, shared::FileState::Loaded(loaded_file.clone())));
                                    }
                                }
                                DownMsg::ParsingStarted { file_id, filename } => {
                                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Routing ParsingStarted for {}", file_id);
                                    parsing_started_relay_clone.send((file_id, filename));
                                }
                                _ => {
                                    // Other message types can be added as needed
                                    zoon::println!("🔍 CONNECTION_MSG_ACTOR: Unhandled message type: {:?}", std::mem::discriminant(&down_msg));
                                }
                            }
                            zoon::println!("✅ CONNECTION_MSG_ACTOR: Finished processing message, continuing loop");
                        }
                        None => {
                            zoon::println!("💔 CONNECTION_MSG_ACTOR: down_msg_stream ended, no more messages");
                            zoon::println!("💔 CONNECTION_MSG_ACTOR: This means the message channel was closed!");
                            break;
                        }
                    }
                    zoon::println!("🔄 CONNECTION_MSG_ACTOR: Completed loop iteration {}", loop_counter);
                }
                zoon::println!("⛔ CONNECTION_MSG_ACTOR: Actor loop ended");
        });

        zoon::println!("✅ CONNECTION_MSG_ACTOR: Actor created, returning instance");
        Self {
            config_loaded_relay,
            config_saved_relay,
            directory_contents_relay,
            directory_error_relay,
            file_loaded_relay,
            parsing_started_relay,
            _message_actor: message_actor,
        }
    }
}


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

    /// Message routing actor (keeps relays alive)
    pub connection_message_actor: ConnectionMessageActor,

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

        // ✅ ACTOR+RELAY: Use working connection with message actor integration
        let (connection, connection_message_actor) = Self::create_connection_with_message_actor(tracked_files.clone()).await;

        // Initialize platform layer with the working connection
        let connection_arc = std::sync::Arc::new(connection);
        crate::platform::set_platform_connection(connection_arc.clone());

        // Create main config with proper connection and message routing
        zoon::println!("🎯 APP: About to call AppConfig::new()");
        let config = AppConfig::new(connection_arc.clone(), connection_message_actor.clone(), tracked_files.clone()).await;
        zoon::println!("🎯 APP: AppConfig::new() completed");

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
            connection_message_actor,
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

    /// Create connection with ConnectionMessageActor integration
    /// Returns both connection and message actor for proper Actor+Relay architecture
    async fn create_connection_with_message_actor(
        tracked_files: TrackedFiles,
    ) -> (SendWrapper<Connection<UpMsg, DownMsg>>, ConnectionMessageActor) {
        use futures::channel::mpsc;

        let (down_msg_sender, down_msg_receiver) = mpsc::unbounded::<DownMsg>();
        let tf_relay = tracked_files.file_load_completed_relay.clone();

        // Create ConnectionMessageActor with the message stream
        // ✅ FIX: Move receiver into closure to prevent reference capture after Send bounds removal
        let connection_message_actor = ConnectionMessageActor::new(down_msg_receiver).await;

        // Create connection that sends to the stream
        let connection = Connection::new({
            let sender = down_msg_sender; // Move sender explicitly before closure
            move |down_msg, _| {
            // Log the received message
            zoon::println!("🔍 APP: Received message: {}", match &down_msg {
                DownMsg::DirectoryContents { path, items } => format!("DirectoryContents(path={}, items={})", path, items.len()),
                DownMsg::DirectoryError { path, error } => format!("DirectoryError(path={}, error={})", path, error),
                DownMsg::ConfigLoaded(_) => "ConfigLoaded".to_string(),
                DownMsg::ConfigSaved => "ConfigSaved".to_string(),
                DownMsg::FileLoaded { file_id, .. } => format!("FileLoaded({})", file_id),
                _ => format!("Other({:?})", std::mem::discriminant(&down_msg))
            });

            // Handle TrackedFiles messages directly (not routed through ConnectionMessageActor)
            match &down_msg {
                DownMsg::FileLoaded { file_id, hierarchy } => {
                    if let Some(loaded_file) = hierarchy.files.first() {
                        tf_relay.send((file_id.clone(), shared::FileState::Loaded(loaded_file.clone())));
                    }
                }
                DownMsg::ParsingStarted { file_id, .. } => {
                    tf_relay.send((
                        file_id.clone(),
                        shared::FileState::Loading(shared::LoadingStatus::Parsing),
                    ));
                }
                _ => {
                    // All other messages go to ConnectionMessageActor for routing
                }
            }

            // Send all messages to ConnectionMessageActor for domain routing
            if let Err(send_error) = sender.unbounded_send(down_msg) {
                zoon::println!("❌ APP: Failed to send message to ConnectionMessageActor: {:?}", send_error);
                zoon::println!("❌ APP: Channel is disconnected - receiver may have been dropped");
            }
        }
        });

        (SendWrapper::new(connection), connection_message_actor)
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
