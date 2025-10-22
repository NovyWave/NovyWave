//! Web platform implementation - cache cleared v6
use crate::platform::Platform;
use shared::{DownMsg, UpMsg};
use std::sync::Arc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;
use std::collections::VecDeque;
use zoon::{Mutable, SendWrapper, Timer};

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
// Prevent multiple concurrent LoadConfig attempts during dev-server swaps
static LOADCONFIG_INFLIGHT: std::sync::LazyLock<Mutable<bool>> =
    std::sync::LazyLock::new(|| Mutable::new(false));
// Timestamp (ms since epoch) of the last DownMsg observed, used to detect long gaps
static LAST_DOWNMSG_MS: std::sync::LazyLock<Mutable<i64>> =
    std::sync::LazyLock::new(|| Mutable::new(0));

// Small queue for critical UpMsgs while the server isn't ready
static PENDING_QUEUE: std::sync::LazyLock<Mutable<VecDeque<UpMsg>>> =
    std::sync::LazyLock::new(|| Mutable::new(VecDeque::with_capacity(8)));

// Simple flag to avoid spawning multiple flushers
static FLUSHING: std::sync::LazyLock<Mutable<bool>> =
    std::sync::LazyLock::new(|| Mutable::new(false));

pub struct WebPlatform;

/// Initialize the platform with a connection
pub fn set_platform_connection(connection: Arc<SendWrapper<zoon::Connection<UpMsg, DownMsg>>>) {
    (&*CONNECTION).set(Some(connection));
    zoon::println!("🌐 PLATFORM: Connection initialized for WebPlatform");
}

/// Expose a read-only signal indicating whether the backend API appears ready
pub fn server_ready_signal() -> impl zoon::Signal<Item = bool> { SERVER_READY.signal() }

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
pub fn server_is_ready() -> bool { SERVER_ALIVE.get() }

/// Public helper: wait until the backend handler looks ready.
pub async fn wait_until_handler_ready() -> bool {
    wait_until_server_ready().await
}

impl Platform for WebPlatform {
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        match (&*CONNECTION).get_cloned() {
            Some(connection) => {
                if matches!(msg, UpMsg::LoadConfig) {
                    let _ = connection.send_up_msg(UpMsg::LoadConfig).await;
                    return Ok(());
                }

                // Suppress non-critical posts until we see the first DownMsg
                // to avoid ERR_EMPTY_RESPONSE during handler startup.
                let is_critical = matches!(msg, UpMsg::LoadConfig | UpMsg::SelectWorkspace { .. } | UpMsg::LoadWaveformFile(_));
                if !SERVER_ALIVE.get() && !is_critical {
                    return Ok(());
                }

                connection
                    .send_up_msg(msg)
                    .await
                    .map(|_| ())
                    .map_err(|e| format!("{:?}", e))
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

#[allow(dead_code)]
async fn wait_until_server_ready() -> bool { true }

#[allow(dead_code)]
fn enqueue_critical(msg: UpMsg) {
    // Keep the queue small and ensure LoadConfig is prioritized by inserting at the front
    let mut q = PENDING_QUEUE.lock_mut();
    if q.len() >= 8 {
        // Drop oldest non-LoadConfig to make space
        if let Some(pos) = q.iter().position(|m| !matches!(m, UpMsg::LoadConfig)) {
            q.remove(pos);
        } else {
            // All are LoadConfig; keep only one
            q.clear();
        }
    }
    if matches!(msg, UpMsg::LoadConfig) {
        // Avoid duplicate LoadConfig in queue
        if q.iter().any(|m| matches!(m, UpMsg::LoadConfig)) {
            return;
        }
        q.push_front(msg);
    } else {
        q.push_back(msg);
    }
}

#[allow(dead_code)]
fn ensure_flusher() {
    if FLUSHING.get() {
        return;
    }
    FLUSHING.set(true);
    zoon::Task::start(async move {
        // First, try to verify handler readiness.
        let ready = wait_until_server_ready().await;

        if ready {
            // Flush all queued messages with small spacing
            while let Some(next_msg) = { PENDING_QUEUE.lock_mut().pop_front() } {
                if let Some(connection) = (&*CONNECTION).get_cloned() {
                    let is_load_config = matches!(next_msg, UpMsg::LoadConfig);
                    let _ = connection.send_up_msg(next_msg).await;
                    if is_load_config { LOADCONFIG_INFLIGHT.set(false); }
                }
                Timer::sleep(200).await; // avoid stampede
            }
            FLUSHING.set(false);
            return;
        }

        // If the server appears alive but the handler isn't mounted yet,
        // attempt a single guarded LoadConfig send (no flood).
        if SERVER_ALIVE.get() {
            if let Some(load_index) = PENDING_QUEUE
                .lock_mut()
                .iter()
                .position(|m| matches!(m, UpMsg::LoadConfig))
            {
                let msg = PENDING_QUEUE.lock_mut().remove(load_index).unwrap();
                if let Some(connection) = (&*CONNECTION).get_cloned() {
                    let result = connection.send_up_msg(msg).await;
                    match result {
                        Ok(_) => {
                            SERVER_READY.set(true);
                            LOADCONFIG_INFLIGHT.set(false);
                            // Re-arm to flush any remaining messages
                            FLUSHING.set(false);
                            ensure_flusher();
                            return;
                        }
                        Err(_) => {
                            // Requeue and back off
                            enqueue_critical(UpMsg::LoadConfig);
                            LOADCONFIG_INFLIGHT.set(false);
                            Timer::sleep(600).await;
                        }
                    }
                }
            }
        }

        FLUSHING.set(false);
    });
}
