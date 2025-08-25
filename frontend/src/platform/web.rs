//! Web platform implementation using MoonZoon Connection
//! 
//! Uses existing SSE/WebSocket connection for frontend-backend communication.

use shared::UpMsg;
use crate::platform::Platform;

/// Web platform implementation using MoonZoon's Connection API
pub struct WebPlatform;

impl Platform for WebPlatform {
    
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        // Use the MoonZoon CONNECTION directly to avoid infinite recursion
        crate::connection::CONNECTION.send_up_msg(msg).await
            .map(|_| ()) // Convert CorId to ()
            .map_err(|e| format!("SSE connection failed: {:?}", e))
    }
}