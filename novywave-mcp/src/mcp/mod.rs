pub mod tools;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::verify;
use crate::ws_server::{self, Command, Response};
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

pub fn find_extension_dir() -> Option<PathBuf> {
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

    log::info!("NovyWave MCP server starting (connecting to WS server on port {})...", ws_port);

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

        let response = handle_request(request, ws_port).await;

        let response_json = serde_json::to_string(&response).unwrap();
        log::debug!("Sending: {}", response_json);

        writeln!(stdout, "{}", response_json).unwrap();
        stdout.flush().unwrap();
    }
}

async fn handle_request(request: McpRequest, ws_port: u16) -> McpResponse {
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

            match call_tool(tool_name, arguments, ws_port).await {
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

async fn send_cmd(port: u16, command: Command) -> Result<Response, String> {
    ws_server::send_command_to_server(port, command)
        .await
        .map_err(|e| e.to_string())
}

async fn call_tool(name: &str, args: Value, ws_port: u16) -> Result<String, String> {
    match name {
        "novywave_status" => {
            match send_cmd(ws_port, Command::GetStatus).await {
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
                Ok(r) => Ok(format!("Response: {:?}", r)),
                Err(e) => Err(format!("WS server not running or extension not connected: {}", e)),
            }
        }

        "novywave_screenshot" => match send_cmd(ws_port,Command::Screenshot).await {
            Ok(Response::ScreenshotFile { filepath }) => Ok(format!("Screenshot saved: {}", filepath)),
            Ok(Response::Screenshot { .. }) => Ok("Screenshot captured".into()),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_screenshot_canvas" => match send_cmd(ws_port,Command::ScreenshotCanvas).await {
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

            match send_cmd(ws_port,Command::GetConsole).await {
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

        "novywave_refresh" => match send_cmd(ws_port,Command::Refresh).await {
            Ok(_) => Ok("Page refreshed".into()),
            Err(e) => Err(e.to_string()),
        },

        "novywave_detach" => match send_cmd(ws_port,Command::Detach).await {
            Ok(_) => Ok("Debugger detached".into()),
            Err(e) => Err(e.to_string()),
        },

        "novywave_timeline_zoom_in" => {
            let faster = args.get("faster").and_then(|v| v.as_bool()).unwrap_or(false);
            match send_cmd(ws_port,Command::PressKey {
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
            match send_cmd(ws_port,Command::PressKey {
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
            match send_cmd(ws_port,Command::PressKey {
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
            match send_cmd(ws_port,Command::PressKey {
                    key: "d".into(),
                    shift: faster,
                })
                .await
            {
                Ok(_) => Ok(format!("Panned right{}", if faster { " (fast)" } else { "" })),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_timeline_reset" => match send_cmd(ws_port,Command::PressKey {
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
            match send_cmd(ws_port,Command::PressKey {
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
            match send_cmd(ws_port,Command::PressKey {
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

        "novywave_get_timeline_state" => match send_cmd(ws_port,Command::GetTimelineState).await {
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

        "novywave_get_cursor_values" => match send_cmd(ws_port,Command::GetCursorValues).await {
            Ok(Response::CursorValues { values }) => Ok(serde_json::to_string_pretty(&values).unwrap()),
            Ok(Response::JsResult { result }) => Ok(serde_json::to_string_pretty(&result).unwrap()),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_get_selected_variables" => {
            match send_cmd(ws_port,Command::GetSelectedVariables).await {
                Ok(Response::SelectedVariables { variables }) => {
                    Ok(serde_json::to_string_pretty(&variables).unwrap())
                }
                Ok(Response::JsResult { result }) => Ok(serde_json::to_string_pretty(&result).unwrap()),
                Ok(r) => Err(format!("Unexpected response: {:?}", r)),
                Err(e) => Err(e.to_string()),
            }
        }

        "novywave_get_loaded_files" => match send_cmd(ws_port,Command::GetLoadedFiles).await {
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

            match send_cmd(ws_port,Command::ClickText {
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

            match send_cmd(ws_port,Command::FindText {
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

        "novywave_get_page_text" => match send_cmd(ws_port,Command::GetPageText).await {
            Ok(Response::PageText { text }) => Ok(text),
            Ok(r) => Err(format!("Unexpected response: {:?}", r)),
            Err(e) => Err(e.to_string()),
        },

        "novywave_type_text" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("text parameter required")?;

            match send_cmd(ws_port,Command::TypeText { text: text.into() })
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

            match send_cmd(ws_port,Command::PressKey {
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
            launch_browser(headless, ws_port).await
        }

        "novywave_verify" => {
            let workspace = args
                .get("workspace")
                .and_then(|v| v.as_str())
                .ok_or("workspace parameter required")?;
            let timeout_ms = args
                .get("timeout")
                .and_then(|v| v.as_u64())
                .unwrap_or(15000);

            run_verify_tests(workspace, timeout_ms, ws_port).await
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

async fn launch_browser(headless: bool, ws_port: u16) -> Result<String, String> {
    if send_cmd(ws_port, Command::GetStatus).await.is_ok() {
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
        if send_cmd(ws_port, Command::GetStatus).await.is_ok() {
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

async fn run_verify_tests(
    workspace: &str,
    timeout_ms: u64,
    ws_port: u16,
) -> Result<String, String> {
    use std::path::Path;

    let runner = verify::CommandRunner::Remote { port: ws_port };

    if !runner.is_connected().await {
        return Err("Extension not connected. Run novywave_launch_browser first.".into());
    }

    let workspace_path = Path::new(workspace);
    let config_path = workspace_path.join(".novywave");

    let config = if config_path.exists() {
        match verify::config::load_config(&config_path) {
            Ok(cfg) => Some(cfg),
            Err(e) => return Err(format!("Failed to load .novywave config: {}", e)),
        }
    } else {
        None
    };

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    let result = verify::tests::test_no_loading_stuck_runner(&runner, timeout_ms).await;
    match &result {
        verify::TestResult::Pass => {
            results.push("✅ No stuck 'Loading workspace...'".to_string());
            passed += 1;
        }
        verify::TestResult::Fail(msg) => {
            results.push(format!("❌ No stuck 'Loading workspace...': {}", msg));
            failed += 1;
        }
        verify::TestResult::Skip(msg) => {
            results.push(format!("⏭️  No stuck 'Loading workspace...' (skipped: {})", msg));
            skipped += 1;
        }
    }

    if let Some(ref cfg) = config {
        let result = verify::tests::test_files_restored_runner(&runner, cfg, timeout_ms).await;
        match &result {
            verify::TestResult::Pass => {
                results.push("✅ Files restored in Files & Scopes".to_string());
                passed += 1;
            }
            verify::TestResult::Fail(msg) => {
                results.push(format!("❌ Files restored: {}", msg));
                failed += 1;
            }
            verify::TestResult::Skip(msg) => {
                results.push(format!("⏭️  Files restored (skipped: {})", msg));
                skipped += 1;
            }
        }

        let result = verify::tests::test_variables_restored_runner(&runner, cfg, timeout_ms).await;
        match &result {
            verify::TestResult::Pass => {
                results.push("✅ Selected variables restored".to_string());
                passed += 1;
            }
            verify::TestResult::Fail(msg) => {
                results.push(format!("❌ Variables restored: {}", msg));
                failed += 1;
            }
            verify::TestResult::Skip(msg) => {
                results.push(format!("⏭️  Variables restored (skipped: {})", msg));
                skipped += 1;
            }
        }
    } else {
        results.push("⏭️  Files/variables tests skipped (no .novywave config)".to_string());
        skipped += 2;
    }

    let summary = format!(
        "\n═══════════════════════════════\nResults: {} passed, {} failed, {} skipped\n{}",
        passed,
        failed,
        skipped,
        if failed > 0 { "❌ VERIFICATION FAILED" } else { "✅ VERIFICATION PASSED" }
    );

    results.push(summary);

    if failed > 0 {
        Err(results.join("\n"))
    } else {
        Ok(results.join("\n"))
    }
}
