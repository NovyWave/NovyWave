//! Platform abstraction layer for NovyWave
//!
//! Provides unified interface for communication between frontend and backend
//! across different deployment modes (web vs desktop).

use shared::UpMsg;

/// Platform abstraction trait for frontend-backend communication
pub trait Platform {
    /// Send a message to the backend
    async fn send_message(msg: UpMsg) -> Result<(), String>;

    /// Send a request and wait for response
    #[allow(dead_code)]
    async fn request_response<T>(msg: UpMsg) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned;
}

// Conditional compilation based on NOVYWAVE_PLATFORM
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
pub mod web;
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
pub use web::{WebPlatform as CurrentPlatform, set_platform_connection, server_ready_signal, notify_server_alive};

#[cfg(NOVYWAVE_PLATFORM = "TAURI")]
pub mod tauri;
#[cfg(NOVYWAVE_PLATFORM = "TAURI")]
pub use tauri::{TauriPlatform as CurrentPlatform, server_ready_signal, notify_server_alive};

// Fallback to web platform if no platform specified
#[cfg(not(any(NOVYWAVE_PLATFORM = "WEB", NOVYWAVE_PLATFORM = "TAURI")))]
pub mod web;
#[cfg(not(any(NOVYWAVE_PLATFORM = "WEB", NOVYWAVE_PLATFORM = "TAURI")))]
pub use web::{WebPlatform as CurrentPlatform, set_platform_connection, server_ready_signal, notify_server_alive};
