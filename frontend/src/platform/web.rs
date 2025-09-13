//! Web platform implementation - cache cleared v6
use shared::{UpMsg, DownMsg};
use crate::platform::Platform;
use std::sync::Arc;
use zoon::{Mutable,SendWrapper};

// Global connection holder for platform layer using zoon patterns
static CONNECTION: std::sync::LazyLock<Mutable<Option<Arc<SendWrapper<zoon::Connection<UpMsg, DownMsg>>>>>> = std::sync::LazyLock::new(|| Mutable::new(None));

pub struct WebPlatform;

/// Initialize the platform with a connection
pub fn set_platform_connection(connection: Arc<SendWrapper<zoon::Connection<UpMsg, DownMsg>>>) {
    (&*CONNECTION).set(Some(connection));
    zoon::println!("🌐 PLATFORM: Connection initialized for WebPlatform");
}

impl Platform for WebPlatform {
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        if let Some(connection) = (&*CONNECTION).get_cloned() {
            zoon::println!("🌐 PLATFORM: Sending message through connection");
            // Connection send is async, need to await
            if let Err(e) = connection.send_up_msg(msg).await {
                Err(format!("Failed to send message: {:?}", e))
            } else {
                Ok(())
            }
        } else {
            Err("No platform connection available".to_string())
        }
    }

    async fn request_response<T>(_msg: UpMsg) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        // For now, we don't need request_response pattern
        Err("Request-response not implemented in WebPlatform".to_string())
    }
}