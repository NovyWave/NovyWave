//! Tauri platform implementation
//!
//! Desktop build reuses the same MoonZoon Connection as the web target.
//! The embedded backend is started by Tauri (see `src-tauri/src/lib.rs`)
//! and the JS shim in `main.rs` rewrites fetch/EventSource to hit
//! `http://127.0.0.1:8082/_api/...`, so the standard web platform logic works.

use crate::platform::Platform;
use crate::platform::web;
use shared::UpMsg;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use zoon::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    fn tauri_invoke(cmd: &str);

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    fn tauri_invoke_with_args(cmd: &str, args: JsValue);

    /// Tauri event listener - listen(event, handler) returns Promise<UnlistenFn>
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    fn tauri_listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> js_sys::Promise;

    /// Get app version from Tauri
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "app"], js_name = getVersion)]
    fn tauri_get_version() -> js_sys::Promise;
}

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

/// Request update download via Tauri command
pub fn request_update_download() {
    if !tauri_api_available() {
        return;
    }
    tauri_invoke("request_update_download");
}

/// Request app restart to complete update via Tauri command
pub fn request_app_restart() {
    if !tauri_api_available() {
        return;
    }
    tauri_invoke("request_app_restart");
}

/// Get the application version from Tauri
pub async fn get_app_version() -> String {
    use wasm_bindgen_futures::JsFuture;

    if !tauri_api_available() {
        return "unknown".to_string();
    }

    match JsFuture::from(tauri_get_version()).await {
        Ok(version_js) => version_js
            .as_string()
            .unwrap_or_else(|| "unknown".to_string()),
        Err(_) => "unknown".to_string(),
    }
}

/// Set up listeners for Tauri update events and bridge them to the notification system
pub fn setup_update_event_listeners(error_display: crate::error_display::ErrorDisplay) {
    if !tauri_api_available() {
        zoon::println!("platform(tauri): Tauri API unavailable, skipping update listeners");
        return;
    }

    // Listen for update_available event
    {
        let error_display = error_display.clone();
        let closure = Closure::new(move |event: JsValue| {
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                let current_version =
                    js_sys::Reflect::get(&payload, &JsValue::from_str("current_version"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_else(|| "unknown".to_string());
                let new_version = js_sys::Reflect::get(&payload, &JsValue::from_str("new_version"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let alert = crate::error_display::ErrorAlert::new_update_available(
                    current_version,
                    new_version,
                );
                error_display.add_toast(alert);
            }
        });
        let _ = tauri_listen("update_available", &closure);
        closure.forget();
    }

    // Listen for update_download_progress event
    {
        let error_display = error_display.clone();
        let closure = Closure::new(move |event: JsValue| {
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                let progress = js_sys::Reflect::get(&payload, &JsValue::from_str("progress"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;

                error_display.dismiss_toast("update_available");
                error_display.download_progress.set_neq(progress);
            }
        });
        let _ = tauri_listen("update_download_progress", &closure);
        closure.forget();
    }

    // Listen for update_ready event
    {
        let error_display = error_display.clone();
        let closure = Closure::new(move |event: JsValue| {
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                let version = js_sys::Reflect::get(&payload, &JsValue::from_str("version"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| "unknown".to_string());

                error_display.dismiss_toast("update_downloading");

                let alert = crate::error_display::ErrorAlert::new_update_ready(version);
                error_display.add_toast(alert);
            }
        });
        let _ = tauri_listen("update_ready", &closure);
        closure.forget();
    }

    // Listen for update_error event
    {
        let error_display = error_display.clone();
        let closure = Closure::new(move |event: JsValue| {
            if let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) {
                let error = js_sys::Reflect::get(&payload, &JsValue::from_str("error"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| "Unknown error".to_string());

                // Remove downloading toast and show error toast
                error_display.dismiss_toast("update_downloading");
                error_display.dismiss_toast("update_available");

                let alert = crate::error_display::ErrorAlert::new_update_error(error);
                error_display.add_toast(alert);
            }
        });
        let _ = tauri_listen("update_error", &closure);
        closure.forget();
    }
}

fn tauri_api_available() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };

    let tauri = js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null());

    let Some(tauri) = tauri else {
        return false;
    };

    let event = js_sys::Reflect::get(&tauri, &JsValue::from_str("event"))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null());
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core"))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null());

    event.is_some() && core.is_some()
}
