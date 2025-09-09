//! NovyWaveApp - Self-contained Actor+Relay Architecture

use futures::select;
use zoon::*;
use moonzoon_novyui::{ButtonVariant, ButtonSize, IconName};

use crate::config::AppConfig;
use crate::dataflow::atom::Atom;
use crate::dataflow::{Relay, relay};
use crate::selected_variables::SelectedVariables;
use crate::tracked_files::TrackedFiles;
use crate::visualizer::timeline::timeline_actor::WaveformTimeline;
use shared::{DownMsg, UpMsg};

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

    /// Application configuration (theme, panels, etc.)
    pub config: AppConfig,

    /// Backend communication connection (Arc for cloning)
    pub connection: std::sync::Arc<Connection<UpMsg, DownMsg>>,

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
        // Load fonts first
        Self::load_and_register_fonts().await;

        // Initialize configuration
        let config = AppConfig::new().await;

        let tracked_files = TrackedFiles::new().await;
        let selected_variables = SelectedVariables::new().await;
        
        // Create MaximumTimelineRange standalone actor for centralized range computation
        let maximum_timeline_range = crate::visualizer::timeline::timeline_actor::MaximumTimelineRange::new(
            tracked_files.clone(),
            selected_variables.clone(),
        ).await;
        
        let waveform_timeline = WaveformTimeline::new(maximum_timeline_range).await;

        let connection = Self::create_connection_with_domain_integration(
            &tracked_files,
        )
        .await;

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
            config,
            connection: std::sync::Arc::new(connection),
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

    /// Create connection with proper domain integration
    async fn create_connection_with_domain_integration(
        tracked_files: &TrackedFiles,
    ) -> Connection<UpMsg, DownMsg> {

        let tf_relay = tracked_files.file_load_completed_relay.clone();

        Connection::new(move |down_msg, _| {
            // Handle backend messages with direct domain access
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
                // ... other message handling
                _ => {}
            }
        })
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
            .layer(self.main_layout())
            .layer_signal(self.file_dialog_visible.signal().map_true({
                let tracked_files = self.tracked_files.clone();
                let selected_variables = self.selected_variables.clone();
                let config = self.config.clone();
                let file_dialog_visible = self.file_dialog_visible.clone();
                move || {
                    file_paths_dialog(
                        tracked_files.clone(),
                        selected_variables.clone(), 
                        config.clone(),
                        file_dialog_visible.clone()
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
        )
    }

    /// Files panel with integrated load button
    pub fn files_panel(&self) -> impl Element {
        files_panel(
            self.tracked_files.clone(),
            self.selected_variables.clone(),
            load_files_button_with_progress(
                self.tracked_files.clone(),
                ButtonVariant::Outline,
                ButtonSize::Small,
                Some(IconName::Folder),
                self.file_dialog_visible.clone()
            )
        )
    }


    fn toast_notifications_container(&self) -> impl Element {
        crate::error_ui::toast_notifications_container(self.config.clone())
    }
}
