pub mod tools;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crate::ws_server::{self, Command, Response, ServerState};
use tools::get_tools;

#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

fn find_extension_dir() -> Option<PathBuf> {
    let cwd_paths = [
        PathBuf::from("novywave-mcp/extension"),
        PathBuf::from("extension"),
        PathBuf::from("../novywave-mcp/extension"),
    ];

    for path in &cwd_paths {
        if path.exists() {
            return path.canonicalize().ok();
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let ext_path = parent.join("../../novywave-mcp/extension");
            if ext_path.exists() {
                return Some(ext_path.canonicalize().ok()?);
            }
            let ext_path = parent.join("../../../novywave-mcp/extension");
            if ext_path.exists() {
                return Some(ext_path.canonicalize().ok()?);
            }
        }
    }

    None
}

pub async fn run_mcp_server(ws_port: u16) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    log::info!("NovyWave MCP server starting (ws_port: {})...", ws_port);

    let extension_dir = find_extension_dir();
    if let Some(ref dir) = extension_dir {
        log::info!("Found extension directory: {}", dir.display());
    } else {
        log::warn!("Extension directory not found");
    }

    let state = ServerState::new();
    let state_clone = state.clone();
    let watch_path = extension_dir.clone();

    tokio::spawn(async move {
        log::info!("Starting WebSocket server on port {}...", ws_port);
        if let Err(e) =
            ws_server::start_server(ws_port, state_clone, watch_path.as_deref()).await
        {
            log::error!("WebSocket server error: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    log::info!("WebSocket server started, ready for browser connections");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!("Read error: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        log::debug!("Received: {}", line);

        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Parse error: {}", e);
                continue;
            }
        };

        let response = handle_request(request, &state).await;

        let response_json = serde_json::to_string(&response).unwrap();
        log::debug!("Sending: {}", response_json);

        writeln!(stdout, "{}", response_json).unwrap();
        stdout.flush().unwrap();
    }
}

async fn handle_request(request: McpRequest, state: &Arc<ServerState>) -> McpResponse {
    let id = request.id.unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "novywave-mcp",
                    "version": "0.1.0"
                }
            })),
            error: None,
        },

        "tools/list" => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "tools": get_tools().into_iter().map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })).collect::<Vec<_>>()
            })),
            error: None,
        },

        "tools/call" => {
            let tool_name = request
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));

            match call_tool(tool_name, arguments, state).await {
                Ok(result) => McpResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": result
                        }]
                    })),
                    error: None,
                },
                Err(e) => McpResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32000,
                        message: e,
                    }),
                },
            }
        }

        "notifications/initialized" => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!(null)),
            error: None,
        },

        _ => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
            }),
        },
    }
}

async fn call_tool(name: &str, args: Value, state: &Arc<ServerState>) -> Result<String, String> {
    match name {
        "novywave_status" => {
            let connected = state.is_connected().await;
            if connected {
                match state.send_command(Command::GetStatus).await {
                    Ok(Response::Status {
                        connected,
                        page_url,
                        app_ready,
                    }) => Ok(format!(
                        "Connected: {}\nPage URL: {}\nApp Ready: {}",
                        connected,
                        page_url.unwrap_or_else(|| "N/A".into()),
                        app_ready
                    )),
                    Ok(r) => Ok(format!("Connected: true\nResponse: {:?}", r)),
                    Err(e) => Err(format!("Status check failed: {}", e)),
                }
            } else {
                Ok("Extension not connected. Run novywave_launch_browser first.".into())
            }
        }

        "novywave_screenshot" => match state.send_command(Command::Screenshot).await {
            Ok(Response::ScreenshotFile { filepath }) => Ok(format!("Screenshot saved: {}", filepath)),
            Ok(Response::Screenshot { .. }) => Ok("Screenshot captured".into()),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_screenshot_canvas" => match state.send_command(Command::ScreenshotCanvas).await {
            Ok(Response::ScreenshotFile { filepath }) => Ok(format!("Canvas screenshot saved: {}", filepath)),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_console" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100);
            let level = args
                .get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("all");
            let pattern = args.get("pattern").and_then(|v| v.as_str());

            match state.send_command(Command::GetConsole).await {
                Ok(Response::Console { messages }) => {
                    let filtered: Vec<_> = messages
                        .into_iter()
                        .filter(|m| level == "all" || m.level == level)
                        .filter(|m| pattern.is_none() || m.text.contains(pattern.unwrap()))
                        .take(limit as usize)
                        .collect();
                    Ok(serde_json::to_string_pretty(&filtered).unwrap())
                }
                Ok(r) => Err(format!("Unexpected response: {:?}", r)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_refresh" => match state.send_command(Command::Refresh).await {
            Ok(_) => Ok("Page refreshed".into()),
            Err(e) => Err(e.to_string()),
        },

        "novywave_detach" => match state.send_command(Command::Detach).await {
            Ok(_) => Ok("Debugger detached".into()),
            Err(e) => Err(e.to_string()),
        },

        "novywave_timeline_zoom_in" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match state
                .send_command(Command::PressKey {
                    key: "w".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!("Zoomed in{}", if faster { " (fast)" } else { "" })),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_timeline_zoom_out" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match state
                .send_command(Command::PressKey {
                    key: "s".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!("Zoomed out{}", if faster { " (fast)" } else { "" })),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_timeline_pan_left" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match state
                .send_command(Command::PressKey {
                    key: "a".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!("Panned left{}", if faster { " (fast)" } else { "" })),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_timeline_pan_right" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match state
                .send_command(Command::PressKey {
                    key: "d".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!("Panned right{}", if faster { " (fast)" } else { "" })),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_timeline_reset" => match state
            .send_command(Command::PressKey {
                key: "r".into(),
                shift: false,
            })
            .await
        {
            Ok(_) => Ok("Timeline reset".into()),
            Err(e) => Err(e.to_string()),
        },

        "novywave_cursor_left" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match state
                .send_command(Command::PressKey {
                    key: "q".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!(
                    "Cursor moved left{}",
                    if faster { " (to transition)" } else { "" }
                )),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_cursor_right" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match state
                .send_command(Command::PressKey {
                    key: "e".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!(
                    "Cursor moved right{}",
                    if faster { " (to transition)" } else { "" }
                )),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_get_timeline_state" => match state.send_command(Command::GetTimelineState).await {
            Ok(Response::TimelineState {
                viewport_start_ps,
                viewport_end_ps,
                cursor_ps,
                zoom_center_ps,
            }) => Ok(format!(
                "Viewport: {:?} - {:?} ps\nCursor: {:?} ps\nZoom Center: {:?} ps",
                viewport_start_ps, viewport_end_ps, cursor_ps, zoom_center_ps
            )),
            Ok(Response::JsResult { result }) => Ok(serde_json::to_string_pretty(&result).unwrap()),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_get_cursor_values" => match state.send_command(Command::GetCursorValues).await {
            Ok(Response::CursorValues { values }) => Ok(serde_json::to_string_pretty(&values).unwrap()),
            Ok(Response::JsResult { result }) => Ok(serde_json::to_string_pretty(&result).unwrap()),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_get_selected_variables" => {
            match state.send_command(Command::GetSelectedVariables).await {
                Ok(Response::SelectedVariables { variables }) => {
                    Ok(serde_json::to_string_pretty(&variables).unwrap())
                }
                Ok(Response::JsResult { result }) => Ok(serde_json::to_string_pretty(&result).unwrap()),
                Ok(r) => Err(format!("Unexpected response: {:?}", r)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_get_loaded_files" => match state.send_command(Command::GetLoadedFiles).await {
            Ok(Response::LoadedFiles { files }) => Ok(serde_json::to_string_pretty(&files).unwrap()),
            Ok(Response::JsResult { result }) => Ok(serde_json::to_string_pretty(&result).unwrap()),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_click_text" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("text parameter required")?;
            let exact = args.get("exact").and_then(|v| v.as_bool()).unwrap_or(false);

            match state
                .send_command(Command::ClickText {
                    text: text.into(),
                    exact,
                })
                .await
            {
                Ok(_) => Ok(format!("Clicked: {}", text)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_find_text" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("text parameter required")?;
            let exact = args.get("exact").and_then(|v| v.as_bool()).unwrap_or(false);

            match state
                .send_command(Command::FindText {
                    text: text.into(),
                    exact,
                })
                .await
            {
                Ok(Response::TextMatches { found, count, matches }) => {
                    Ok(format!("Found: {}\nCount: {}\nMatches: {:?}", found, count, matches))
                }
                Ok(r) => Err(format!("Unexpected response: {:?}", r)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_get_page_text" => match state.send_command(Command::GetPageText).await {
            Ok(Response::PageText { text }) => Ok(text),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_type_text" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("text parameter required")?;

            match state
                .send_command(Command::TypeText { text: text.into() })
                .await
            {
                Ok(_) => Ok(format!("Typed: {}", text)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_press_key" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("key parameter required")?;

            match state
                .send_command(Command::PressKey {
                    key: key.into(),
                    shift: false,
                })
                .await
            {
                Ok(_) => Ok(format!("Pressed: {}", key)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_launch_browser" => {
            let headless = args.get("headless").and_then(|v| v.as_bool()).unwrap_or(false);
            launch_browser(headless, state).await
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}

fn find_profile_dir() -> Option<PathBuf> {
    let cwd_paths = [
        PathBuf::from("novywave-mcp/.chrome-profile"),
        PathBuf::from(".chrome-profile"),
        PathBuf::from("../novywave-mcp/.chrome-profile"),
    ];

    for path in &cwd_paths {
        if path.exists() {
            return path.canonicalize().ok();
        }
    }

    for path in &cwd_paths {
        if let Some(parent) = path.parent() {
            if parent.exists() || parent == std::path::Path::new("") {
                if std::fs::create_dir_all(&path).is_ok() {
                    return path.canonicalize().ok();
                }
            }
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let profile_path = parent.join("../../novywave-mcp/.chrome-profile");
            if let Ok(canonical) = profile_path.canonicalize() {
                return Some(canonical);
            }
            let profile_path = parent.join("../../../novywave-mcp/.chrome-profile");
            if let Ok(canonical) = profile_path.canonicalize() {
                return Some(canonical);
            }
        }
    }

    None
}

fn find_chromium_binary() -> Option<PathBuf> {
    let candidates = ["chromium-browser", "chromium"];
    for name in candidates {
        if let Ok(path) = which::which(name) {
            log::info!("Found Chromium at: {}", path.display());
            return Some(path);
        }
    }
    None
}

async fn launch_browser(headless: bool, state: &Arc<ServerState>) -> Result<String, String> {
    if state.is_connected().await {
        return Ok("Browser already connected. Use novywave_refresh to reload the page.".into());
    }

    let extension_dir = find_extension_dir().ok_or("Extension directory not found")?;
    let profile_dir = find_profile_dir().ok_or("Could not create profile directory")?;
    std::fs::create_dir_all(&profile_dir).map_err(|e| format!("Failed to create profile dir: {}", e))?;

    let browser = find_chromium_binary().ok_or(
        "Chromium not found in PATH.\n\
        Install with: apt install chromium-browser (Debian/Ubuntu)\n\
        Note: Chrome is not supported because --load-extension was deprecated in Chrome 137+"
    )?;

    let load_ext_arg = format!("--load-extension={}", extension_dir.display());
    let user_data_arg = format!("--user-data-dir={}", profile_dir.display());

    let mut cmd = std::process::Command::new(&browser);
    cmd.args([
        &load_ext_arg,
        &user_data_arg,
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-default-apps",
        "--disable-popup-blocking",
        "--disable-translate",
        "--disable-sync",
        "--disable-session-crashed-bubble",
        "--hide-crash-restore-bubble",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",
    ]);

    if headless {
        cmd.arg("--headless=new");
    }

    cmd.arg("http://localhost:8080");
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    log::info!("Launching Chromium: {}", browser.display());
    log::info!("Extension: {}", extension_dir.display());
    log::info!("Profile: {}", profile_dir.display());

    let child = cmd.spawn().map_err(|e| format!("Failed to launch browser: {}", e))?;
    let pid = child.id();

    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if state.is_connected().await {
            return Ok(format!(
                "Browser launched (PID: {}).\nExtension connected.\nExtension: {}\nProfile: {}",
                pid,
                extension_dir.display(),
                profile_dir.display()
            ));
        }
    }

    Ok(format!(
        "Browser launched (PID: {}) but extension connection timed out after 15s.\n\
        Check chrome://extensions for errors.\nExtension: {}\nProfile: {}",
        pid,
        extension_dir.display(),
        profile_dir.display()
    ))
}
