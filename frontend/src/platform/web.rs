//! Web platform implementation - cache cleared v6
use crate::platform::Platform;
use shared::{DownMsg, UpMsg};
use std::sync::Arc;
use zoon::{Mutable, SendWrapper};

// Global connection holder for platform layer using zoon patterns
static CONNECTION: std::sync::LazyLock<
    Mutable<Option<Arc<SendWrapper<zoon::Connection<UpMsg, DownMsg>>>>>,
> = std::sync::LazyLock::new(|| Mutable::new(None));

// Track server readiness to avoid spamming failed POSTs during dev-server restarts
static SERVER_READY: std::sync::LazyLock<Mutable<bool>> =
    std::sync::LazyLock::new(|| Mutable::new(false));
// Track generic server aliveness (/_api/ responds) even if handler isn't mounted yet
static SERVER_ALIVE: std::sync::LazyLock<Mutable<bool>> =
    std::sync::LazyLock::new(|| Mutable::new(false));
// Timestamp (ms since epoch) of the last DownMsg observed, used to detect long gaps
static LAST_DOWNMSG_MS: std::sync::LazyLock<Mutable<i64>> =
    std::sync::LazyLock::new(|| Mutable::new(0));

pub struct WebPlatform;

/// Initialize the platform with a connection
pub fn set_platform_connection(connection: Arc<SendWrapper<zoon::Connection<UpMsg, DownMsg>>>) {
    (&*CONNECTION).set(Some(connection));
    // Debug-only: connection initialization notice (silenced by default)
}

/// Expose a read-only signal indicating whether the backend API appears ready
pub fn server_ready_signal() -> impl zoon::Signal<Item = bool> {
    SERVER_READY.signal()
}

/// Mark the server as alive/ready when any DownMsg is received
pub fn notify_server_alive() {
    SERVER_ALIVE.set(true);
    SERVER_READY.set(true);
    // Update last seen DownMsg timestamp
    if let Some(perf) = web_sys::window().and_then(|w| w.performance()) {
        let now = perf.now() as i64;
        LAST_DOWNMSG_MS.set(now);
    }
}

/// Cheap readiness check for places that must avoid non-critical posts during boot
pub fn server_is_ready() -> bool {
    SERVER_ALIVE.get()
}

/// Request update download - no-op on web platform (updates only work on desktop)
pub fn request_update_download() {
    zoon::println!("platform(web): request_update_download - no-op (desktop only)");
}

/// Request app restart to complete update - no-op on web platform
pub fn request_app_restart() {
    zoon::println!("platform(web): request_app_restart - no-op (desktop only)");
}

/// Set up update event listeners - no-op on web platform (updates only work on desktop)
pub fn setup_update_event_listeners(_error_display: crate::error_display::ErrorDisplay) {
    zoon::println!("platform(web): setup_update_event_listeners - no-op (desktop only)");
}

/// Get the application version - returns compile-time version on web
pub async fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// removed explicit wait_until_handler_ready; rely on DownMsg to flip readiness

impl Platform for WebPlatform {
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        zoon::println!("platform(web): send_message {:?}", msg);
        match (&*CONNECTION).get_cloned() {
            Some(connection) => {
                if matches!(msg, UpMsg::LoadConfig) {
                    if let Err(e) = connection.send_up_msg(UpMsg::LoadConfig).await {
                        zoon::println!("ERROR: Failed to send LoadConfig: {:?}", e);
                    }
                    zoon::println!("platform(web): sent LoadConfig");
                    return Ok(());
                }

                // Suppress non-critical posts until we see the first DownMsg
                // to avoid ERR_EMPTY_RESPONSE during handler startup.
                let is_critical = matches!(
                    msg,
                    UpMsg::LoadConfig | UpMsg::SelectWorkspace { .. } | UpMsg::LoadWaveformFile(_)
                );
                if !SERVER_ALIVE.get() && !is_critical {
                    zoon::println!("platform(web): suppressing {:?}, server not alive", msg);
                    return Ok(());
                }

                let res = connection
                    .send_up_msg(msg)
                    .await
                    .map(|_| ())
                    .map_err(|e| format!("{:?}", e));
                if res.is_err() {
                    zoon::println!("platform(web): send_up_msg error {:?}", res);
                }
                res
            }
            None => Err("No platform connection available".to_string()),
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
