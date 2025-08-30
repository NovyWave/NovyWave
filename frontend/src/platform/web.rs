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
    
    async fn request_response<T>(msg: UpMsg) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        // Use proper Zoon Connection exchange_msgs for request-response pattern
        match crate::connection::CONNECTION.exchange_msgs(msg).await {
            Ok((down_msg, _cor_id)) => {
                match down_msg {
                    shared::DownMsg::ConfigLoaded(config) => {
                        // Try to deserialize the config as type T
                        serde_json::from_str(&serde_json::to_string(&config).unwrap())
                            .map_err(|e| format!("Failed to deserialize response: {e}"))
                    },
                    other => Err(format!("Unexpected response: {other:?}")),
                }
            },
            Err(error) => Err(format!("Failed to exchange message: {error:?}")),
        }
    }
}