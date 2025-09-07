//! Web platform implementation using MoonZoon Connection
//! 
//! Uses existing SSE/WebSocket connection for frontend-backend communication.

use shared::UpMsg;
use crate::platform::Platform;

/// Web platform implementation using Actor+Relay compatible ConnectionAdapter
pub struct WebPlatform {
    connection_adapter: crate::connection::ConnectionAdapter,
}

impl WebPlatform {
    pub fn new(connection_adapter: crate::connection::ConnectionAdapter) -> Self {
        Self { connection_adapter }
    }
}

impl Platform for WebPlatform {
    
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        // TODO: This needs to be converted to instance method
        // For now, placeholder to fix compilation
        Err("Platform abstraction needs instance method conversion".to_string())
    }
    
    async fn request_response<T>(msg: UpMsg) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        // TODO: This also needs to be converted to instance method
        // For now, placeholder to fix compilation
        let _ = msg;
        Err("Platform abstraction needs instance method conversion".to_string())
    }
}