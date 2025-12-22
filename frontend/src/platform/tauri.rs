//! Tauri platform implementation
//!
//! Desktop build reuses the same MoonZoon Connection as the web target.
//! The embedded backend is started by Tauri (see `src-tauri/src/lib.rs`)
//! and the JS shim in `main.rs` rewrites fetch/EventSource to hit
//! `http://127.0.0.1:8080/_api/...`, so the standard web platform logic works.

use crate::platform::Platform;
use crate::platform::web;
use shared::UpMsg;
use zoon::*;

/// Tauri platform simply delegates to the web implementation.
pub struct TauriPlatform;

impl Platform for TauriPlatform {
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        web::WebPlatform::send_message(msg).await
    }

    async fn request_response<T>(msg: UpMsg) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned,
    {
        web::WebPlatform::request_response(msg).await
    }
}

/// Desktop build talks to a local backend; consider it ready immediately.
pub fn server_ready_signal() -> impl Signal<Item = bool> {
    zoon::always(true)
}

pub fn notify_server_alive() {
    // no-op; always ready
}

pub fn server_is_ready() -> bool {
    true
}

/// Capture the connection so platform calls can forward UpMsgs.
pub fn set_platform_connection(
    connection: std::sync::Arc<zoon::SendWrapper<zoon::Connection<UpMsg, shared::DownMsg>>>,
) {
    web::set_platform_connection(connection);
}
