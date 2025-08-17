//! Web platform implementation using MoonZoon Connection
//! 
//! Uses existing SSE/WebSocket connection for frontend-backend communication.

use shared::{UpMsg, DownMsg};
use crate::platform::Platform;

/// Web platform implementation using MoonZoon's Connection API
pub struct WebPlatform;

impl Platform for WebPlatform {
    fn is_available() -> bool {
        // Always available in web context
        true
    }
    
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        // Use the MoonZoon CONNECTION directly to avoid infinite recursion
        crate::connection::CONNECTION.send_up_msg(msg).await
            .map(|_| ()) // Convert CorId to ()
            .map_err(|e| format!("SSE connection failed: {:?}", e))
    }
    
    fn init_message_handler(_handler: fn(DownMsg)) {
        // MoonZoon connection is already initialized in connection.rs
        // The handler is already set up in the CONNECTION static
        // No additional setup needed for web platform
    }
}