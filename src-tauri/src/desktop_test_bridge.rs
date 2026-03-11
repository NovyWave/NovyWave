use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use tauri::{Listener, Manager};

const DEFAULT_TEST_BRIDGE_PORT: u16 = 9226;
const MAIN_WINDOW_LABEL: &str = "main";

#[derive(Debug, Serialize, Deserialize)]
struct JsEvalEnvelope {
    ok: bool,
    value: Option<Value>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CursorRequest {
    time_ps: u64,
}

#[derive(Debug, Deserialize)]
struct MarkerCreateRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct MarkerIndexRequest {
    index: usize,
}

#[derive(Debug, Deserialize)]
struct MarkerRenameRequest {
    index: usize,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RowHeightRequest {
    unique_id: String,
    row_height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalogLimitsRequest {
    unique_id: String,
    auto: bool,
    min: f64,
    max: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupCreateRequest {
    name: String,
    member_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GroupRenameRequest {
    index: usize,
    name: String,
}

#[derive(Debug, Deserialize)]
struct EvalRequest {
    expression: String,
}

pub fn start(app: &tauri::AppHandle) {
    let port = std::env::var("NOVYWAVE_DESKTOP_TEST_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_TEST_BRIDGE_PORT);

    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(error) => {
            println!("Desktop test bridge disabled: failed to bind 127.0.0.1:{port}: {error}");
            return;
        }
    };

    let app = app.clone();
    std::thread::spawn(move || {
        println!("Desktop test bridge listening on http://127.0.0.1:{port}");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => handle_connection(stream, &app),
                Err(error) => println!("Desktop test bridge connection error: {error}"),
            }
        }
    });
}

fn handle_connection(mut stream: TcpStream, app: &tauri::AppHandle) {
    let mut buffer = vec![0_u8; 64 * 1024];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(bytes_read) if bytes_read > 0 => bytes_read,
        Ok(_) => return,
        Err(error) => {
            write_response(
                &mut stream,
                "500 Internal Server Error",
                &json_error(format!("Failed to read request: {error}")),
            );
            return;
        }
    };
    buffer.truncate(bytes_read);

    let request = match String::from_utf8(buffer) {
        Ok(request) => request,
        Err(error) => {
            write_response(
                &mut stream,
                "400 Bad Request",
                &json_error(format!("Request was not valid UTF-8: {error}")),
            );
            return;
        }
    };

    let mut lines = request.split("\r\n");
    let request_line = match lines.next() {
        Some(line) if !line.is_empty() => line,
        _ => {
            write_response(
                &mut stream,
                "400 Bad Request",
                &json_error("Missing request line"),
            );
            return;
        }
    };

    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts.next().unwrap_or_default();
    let path = request_line_parts.next().unwrap_or_default();

    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default();

    let (status, payload) = route_request(method, path, body, app);
    write_response(&mut stream, status, &payload);
}

fn route_request(
    method: &str,
    path: &str,
    body: &str,
    app: &tauri::AppHandle,
) -> (&'static str, String) {
    match (method, path) {
        ("GET", "/health") => {
            let payload = serde_json::json!({
                "ok": true,
                "windowFound": app.get_webview_window(MAIN_WINDOW_LABEL).is_some(),
            });
            ("200 OK", payload.to_string())
        }
        ("GET", "/state/timeline") => state_response(
            app,
            "window.__novywave_test_api?.getTimelineState?.() ?? null",
        ),
        ("GET", "/state/cursor-values") => state_response(
            app,
            "window.__novywave_test_api?.getCursorValues?.() ?? null",
        ),
        ("GET", "/state/selected-variables") => state_response(
            app,
            "window.__novywave_test_api?.getSelectedVariables?.() ?? null",
        ),
        ("GET", "/state/loaded-files") => state_response(
            app,
            "window.__novywave_test_api?.getLoadedFiles?.() ?? null",
        ),
        ("GET", "/state/visible-rows") => state_response(
            app,
            "window.__novywave_test_api?.getVisibleRows?.() ?? null",
        ),
        ("GET", "/state/markers") => {
            state_response(app, "window.__novywave_test_api?.getMarkers?.() ?? null")
        }
        ("GET", "/state/file-picker-roots") => state_response(
            app,
            "window.__novywave_test_api?.getFilePickerRoots?.() ?? null",
        ),
        ("POST", "/eval") => {
            action_response::<EvalRequest, _>(app, body, |request| Ok(request.expression))
        }
        ("POST", "/window/focus") => focus_window_response(app),
        ("POST", "/workspace/select") => select_workspace_response(app, body.trim()),
        ("POST", "/action/set-cursor-ps") => {
            action_response::<CursorRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.setCursorPs?.({}) ?? false",
                    request.time_ps
                ))
            })
        }
        ("POST", "/action/add-marker") => {
            action_response::<MarkerCreateRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.addMarker?.({}) ?? false",
                    json_string(&request.name)?
                ))
            })
        }
        ("POST", "/action/remove-marker") => {
            action_response::<MarkerIndexRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.removeMarker?.({}) ?? false",
                    request.index
                ))
            })
        }
        ("POST", "/action/rename-marker") => {
            action_response::<MarkerRenameRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.renameMarker?.({}, {}) ?? false",
                    request.index,
                    json_string(&request.name)?
                ))
            })
        }
        ("POST", "/action/jump-to-marker") => {
            action_response::<MarkerIndexRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.jumpToMarker?.({}) ?? false",
                    request.index
                ))
            })
        }
        ("POST", "/action/set-row-height") => {
            action_response::<RowHeightRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.setRowHeight?.({}, {}) ?? false",
                    json_string(&request.unique_id)?,
                    request.row_height
                ))
            })
        }
        ("POST", "/action/set-analog-limits") => {
            action_response::<AnalogLimitsRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.setAnalogLimits?.({}, {}, {}, {}) ?? false",
                    json_string(&request.unique_id)?,
                    request.auto,
                    request.min,
                    request.max
                ))
            })
        }
        ("POST", "/action/create-group") => {
            action_response::<GroupCreateRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.createGroup?.({}, {}) ?? false",
                    json_string(&request.name)?,
                    serde_json::to_string(&request.member_ids)
                        .map_err(|error| format!("Failed to encode member IDs: {error}"))?
                ))
            })
        }
        ("POST", "/action/rename-group") => {
            action_response::<GroupRenameRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.renameGroup?.({}, {}) ?? false",
                    request.index,
                    json_string(&request.name)?
                ))
            })
        }
        ("POST", "/action/toggle-group-collapse") => {
            action_response::<MarkerIndexRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.toggleGroupCollapse?.({}) ?? false",
                    request.index
                ))
            })
        }
        ("POST", "/action/delete-group") => {
            action_response::<MarkerIndexRequest, _>(app, body, |request| {
                Ok(format!(
                    "window.__novywave_test_api?.deleteGroup?.({}) ?? false",
                    request.index
                ))
            })
        }
        _ => (
            "404 Not Found",
            json_error(format!("Unknown endpoint: {method} {path}")),
        ),
    }
}

fn state_response(app: &tauri::AppHandle, js_expression: &str) -> (&'static str, String) {
    match eval_webview_expression(app, js_expression) {
        Ok(value) => (
            "200 OK",
            serde_json::json!({ "ok": true, "value": value }).to_string(),
        ),
        Err(error) => ("500 Internal Server Error", json_error(error)),
    }
}

fn focus_window_response(app: &tauri::AppHandle) -> (&'static str, String) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return (
            "404 Not Found",
            json_error("Main Tauri window was not found"),
        );
    };

    if let Err(error) = window.show() {
        return (
            "500 Internal Server Error",
            json_error(format!("Failed to show main window: {error}")),
        );
    }
    if let Err(error) = window.set_focus() {
        return (
            "500 Internal Server Error",
            json_error(format!("Failed to focus main window: {error}")),
        );
    }

    (
        "200 OK",
        serde_json::json!({ "ok": true, "focused": true }).to_string(),
    )
}

fn select_workspace_response(app: &tauri::AppHandle, path: &str) -> (&'static str, String) {
    if path.is_empty() {
        return (
            "400 Bad Request",
            json_error("Expected raw workspace path in POST body"),
        );
    }

    let path_json = match serde_json::to_string(path) {
        Ok(path_json) => path_json,
        Err(error) => {
            return (
                "500 Internal Server Error",
                json_error(format!("Failed to encode workspace path: {error}")),
            );
        }
    };

    let expression = format!("window.__novywave_test_api?.selectWorkspace?.({path_json}) ?? false");

    match eval_webview_expression(app, &expression) {
        Ok(value) => (
            "200 OK",
            serde_json::json!({ "ok": true, "value": value }).to_string(),
        ),
        Err(error) => ("500 Internal Server Error", json_error(error)),
    }
}

fn action_response<T, F>(
    app: &tauri::AppHandle,
    body: &str,
    build_expression: F,
) -> (&'static str, String)
where
    T: DeserializeOwned,
    F: FnOnce(T) -> Result<String, String>,
{
    let request = match parse_json_body::<T>(body) {
        Ok(request) => request,
        Err(error) => return ("400 Bad Request", json_error(error)),
    };

    let expression = match build_expression(request) {
        Ok(expression) => expression,
        Err(error) => return ("400 Bad Request", json_error(error)),
    };

    match eval_webview_expression(app, &expression) {
        Ok(value) => (
            "200 OK",
            serde_json::json!({ "ok": true, "value": value }).to_string(),
        ),
        Err(error) => ("500 Internal Server Error", json_error(error)),
    }
}

fn parse_json_body<T: DeserializeOwned>(body: &str) -> Result<T, String> {
    serde_json::from_str(body).map_err(|error| format!("Invalid JSON body: {error}"))
}

fn json_string(value: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("Failed to encode string: {error}"))
}

fn eval_webview_expression(app: &tauri::AppHandle, js_expression: &str) -> Result<Value, String> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return Err("Main Tauri window was not found".to_string());
    };

    let event_name = format!("desktop-test-response-{}", uuid::Uuid::new_v4().simple());
    let (sender, receiver) = std::sync::mpsc::sync_channel::<Result<Value, String>>(1);

    let _listener = app.once_any(event_name.clone(), move |event| {
        let payload = serde_json::from_str::<JsEvalEnvelope>(event.payload())
            .map_err(|error| format!("Failed to decode desktop test payload: {error}"))
            .and_then(|payload| {
                if payload.ok {
                    Ok(payload.value.unwrap_or(Value::Null))
                } else {
                    Err(payload
                        .error
                        .unwrap_or_else(|| "Desktop test query failed".to_string()))
                }
            });
        let _ = sender.send(payload);
    });

    let event_name_json = serde_json::to_string(&event_name)
        .map_err(|error| format!("Invalid event name: {error}"))?;
    let expression_json = serde_json::to_string(js_expression)
        .map_err(|error| format!("Invalid JavaScript expression: {error}"))?;
    let script = format!(
        r#"(async () => {{
            const emit = window.__TAURI__?.event?.emit;
            const response = {{ ok: true, value: null, error: null }};
            try {{
                if (!emit) {{
                    throw new Error("window.__TAURI__.event.emit is unavailable");
                }}
                const expression = {expression_json};
                response.value = await Promise.resolve((0, eval)(expression));
            }} catch (error) {{
                response.ok = false;
                response.error = String((error && (error.stack || error.message)) || error);
            }}
            await emit({event_name_json}, response);
        }})()"#
    );

    window
        .eval(script)
        .map_err(|error| format!("Failed to evaluate desktop test query: {error}"))?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "Timed out waiting for desktop test response".to_string())?
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn json_error(error: impl Into<String>) -> String {
    serde_json::json!({
        "ok": false,
        "error": error.into(),
    })
    .to_string()
}
