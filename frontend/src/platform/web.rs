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

    // Lightweight watchdog: when the window regains focus, re-request LoadConfig once.
    // This fixes the common case of a backend hot-reload while the app tab stays open.
    if let Some(window) = web_sys::window() {
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            let has_conn = (&*CONNECTION).get_cloned().is_some();
            if !has_conn { return; }
            // Fire and forget; platform send already has a small retry loop for LoadConfig
            zoon::Task::start(async move {
                let _ = WebPlatform::send_message(UpMsg::LoadConfig).await;
            });
        }) as Box<dyn FnMut()>);
        let _ = window.add_event_listener_with_callback("focus", closure.as_ref().unchecked_ref());
        // Leak the closure for the lifetime of the app; event listener lives with the page
        closure.forget();
    }
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
pub fn server_is_ready() -> bool {
    SERVER_READY.get()
}

/// Public helper: wait until the backend handler looks ready.
pub async fn wait_until_handler_ready() -> bool {
    wait_until_server_ready().await
}

impl Platform for WebPlatform {
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        match (&*CONNECTION).get_cloned() {
            Some(connection) => {
                // Minimal local robustness: retry LoadConfig a few times on transient
                // network errors (e.g., dev server just swapped) without extra flags.
                if matches!(msg, UpMsg::LoadConfig) {
                    let mut attempt: u8 = 0;
                    loop {
                        attempt = attempt.saturating_add(1);
                        let result = connection.send_up_msg(UpMsg::LoadConfig).await;
                        if result.is_ok() { return Ok(()); }
                        if attempt >= 12 { return Err("LoadConfig failed".into()); }
                        zoon::Timer::sleep(300).await;
                    }
                }

                let is_critical = !matches!(
                    msg,
                    UpMsg::SaveConfig(_) | UpMsg::UpdateWorkspaceHistory(_) | UpMsg::FrontendTrace { .. }
                );
                match connection.send_up_msg(msg).await {
                    Ok(_cor_id) => Ok(()),
                    Err(e) => {
                        if is_critical && !LOADCONFIG_INFLIGHT.get() {
                            LOADCONFIG_INFLIGHT.set(true);
                            enqueue_critical(UpMsg::LoadConfig);
                            ensure_flusher();
                        }
                        Err(format!("{:?}", e))
                    }
                }
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
async fn wait_until_server_ready() -> bool {
    // Probe the actual API handler via a tiny POST so the exact route+method is verified.
    // Treat any non-network status except 404 as ready (400/405 are fine).
    // We DO NOT mark ready based on fallback GETs; we only use them to avoid extra POSTs,
    // but readiness flips true strictly when the handler path responds.
    let mut delay_ms: u32 = 250; // exponential backoff up to ~1500ms
    let mut elapsed_ms: u32 = 0;
    const MAX_ELAPSED: u32 = 6000;

    while elapsed_ms <= MAX_ELAPSED {
        if let Some(window) = web_sys::window() {
            let init = web_sys::RequestInit::new();
            init.set_method("POST");
            init.set_mode(web_sys::RequestMode::SameOrigin);
            init.set_body(&JsValue::from_str("{}"));

            // Add JSON content-type so the handler returns 400/405 when present
            let headers = web_sys::Headers::new().unwrap();
            let _ = headers.append("Content-Type", "application/json");
            init.set_headers(&headers);

            let mut fallback_needed = true;
            if let Ok(request) = web_sys::Request::new_with_str_and_init("/_api/up_msg_handler", &init) {
                let promise = window.fetch_with_request(&request);
                match JsFuture::from(promise).await {
                    Ok(resp_value) => {
                        if let Ok(resp) = resp_value.dyn_into::<Response>() {
                            let status = resp.status();
                            if status != 404 && status != 0 {
                                SERVER_ALIVE.set(true);
                                SERVER_READY.set(true);
                                return true;
                            }
                            // Only fall back when we confirmed a 404 / 0
                            fallback_needed = status == 404 || status == 0;
                        }
                    }
                    Err(_) => { /* fall back below */ }
                }
            }

            if fallback_needed {
                // Fallback 1: GET API root
                let mut get_init = web_sys::RequestInit::new();
                get_init.set_method("GET");
                get_init.set_mode(web_sys::RequestMode::SameOrigin);
                if let Ok(request) = web_sys::Request::new_with_str_and_init("/_api/", &get_init) {
                    let promise = window.fetch_with_request(&request);
                    if JsFuture::from(promise).await.is_ok() {
                        SERVER_ALIVE.set(true);
                    }
                }

                // Fallback 2: GET a static asset
                if let Ok(request) = web_sys::Request::new_with_str_and_init("/_api/public/content.css", &get_init) {
                    let promise = window.fetch_with_request(&request);
                    if JsFuture::from(promise).await.is_ok() {
                        SERVER_ALIVE.set(true);
                    }
                }
            }
        }

        Timer::sleep(delay_ms).await;
        elapsed_ms = elapsed_ms.saturating_add(delay_ms);
        // Exponential backoff with cap ~1500ms
        delay_ms = (delay_ms.saturating_mul(2)).min(1500);
    }

    SERVER_READY.set(false);
    false
}

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
