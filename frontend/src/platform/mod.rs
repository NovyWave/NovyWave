//! Platform abstraction layer for NovyWave
//! 
//! Provides unified interface for communication between frontend and backend
//! across different deployment modes (web vs desktop).

use shared::{UpMsg, DownMsg};

/// Platform abstraction trait for frontend-backend communication
pub trait Platform {
    /// Check if the platform is available in current environment
    fn is_available() -> bool;
    
    /// Send a message to the backend
    async fn send_message(msg: UpMsg) -> Result<(), String>;
    
    /// Initialize message handler for receiving backend messages
    fn init_message_handler(handler: fn(DownMsg));
}

// Conditional compilation based on NOVYWAVE_PLATFORM
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
pub mod web;
#[cfg(NOVYWAVE_PLATFORM = "WEB")]
pub use web::WebPlatform as CurrentPlatform;

#[cfg(NOVYWAVE_PLATFORM = "TAURI")]
pub mod tauri;
#[cfg(NOVYWAVE_PLATFORM = "TAURI")]
pub use tauri::TauriPlatform as CurrentPlatform;

// Fallback to web platform if no platform specified
#[cfg(not(any(NOVYWAVE_PLATFORM = "WEB", NOVYWAVE_PLATFORM = "TAURI")))]
pub mod web;
#[cfg(not(any(NOVYWAVE_PLATFORM = "WEB", NOVYWAVE_PLATFORM = "TAURI")))]
pub use web::WebPlatform as CurrentPlatform;