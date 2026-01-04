pub mod protocol;

use anyhow::{Context, Result};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};

pub use protocol::*;

const SCREENSHOT_DIR: &str = "/tmp/novywave-screenshots";

fn save_screenshot_to_file(base64_data: &str, name_hint: &str) -> Result<String> {
    std::fs::create_dir_all(SCREENSHOT_DIR).context("Failed to create screenshot directory")?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let filename = format!("{}_{}.png", name_hint, timestamp);
    let filepath = format!("{}/{}", SCREENSHOT_DIR, filename);

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .context("Failed to decode base64")?;

    std::fs::write(&filepath, decoded).context("Failed to write screenshot")?;

    Ok(filepath)
}

pub struct ServerState {
    extension_tx: RwLock<Option<mpsc::Sender<String>>>,
    pending_requests: RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Response>>>,
    next_id: RwLock<u64>,
}

impl ServerState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            extension_tx: RwLock::new(None),
            pending_requests: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
        })
    }

    pub async fn send_command(&self, command: Command) -> Result<Response> {
        let extension_tx = self.extension_tx.read().await;
        let tx = extension_tx
            .as_ref()
            .context("No extension connected")?
            .clone();
        drop(extension_tx);

        let screenshot_hint = match &command {
            Command::Screenshot => Some("fullpage"),
            Command::ScreenshotCanvas => Some("canvas"),
            Command::ScreenshotElement { .. } => Some("element"),
            _ => None,
        };

        let mut next_id = self.next_id.write().await;
        let id = *next_id;
        *next_id += 1;
        drop(next_id);

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.pending_requests.write().await.insert(id, response_tx);

        let request = Request { id, command };
        let json = serde_json::to_string(&request)?;

        tx.send(json)
            .await
            .context("Failed to send to extension")?;

        let response = tokio::time::timeout(std::time::Duration::from_secs(30), response_rx)
            .await
            .context("Timeout waiting for response")?
            .context("Response channel closed")?;

        if let Some(hint) = screenshot_hint {
            if let Response::Screenshot { base64 } = &response {
                let filepath = save_screenshot_to_file(base64, hint)?;
                return Ok(Response::ScreenshotFile { filepath });
            }
        }

        Ok(response)
    }

    pub async fn is_connected(&self) -> bool {
        self.extension_tx.read().await.is_some()
    }

    pub async fn broadcast_reload(&self) -> Result<()> {
        let extension_tx = self.extension_tx.read().await;
        if let Some(tx) = extension_tx.as_ref() {
            let msg = serde_json::json!({
                "id": 0,
                "command": { "type": "reload" }
            });
            tx.send(msg.to_string())
                .await
                .context("Failed to send reload command")?;
            log::info!("Sent reload command to extension");
        } else {
            log::info!("No extension connected to reload");
        }
        Ok(())
    }
}

pub async fn run_server_daemon(port: u16) {
    use crate::mcp::find_extension_dir;

    log::info!("Starting NovyWave WebSocket server daemon on port {}...", port);

    let extension_dir = find_extension_dir();
    if let Some(ref dir) = extension_dir {
        log::info!("Extension directory: {}", dir.display());
    }

    let state = ServerState::new();

    if let Err(e) = start_server(port, state, extension_dir.as_deref()).await {
        log::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

pub async fn start_server(
    port: u16,
    state: Arc<ServerState>,
    watch_path: Option<&Path>,
) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    log::info!("WebSocket server listening on {}", addr);

    let _watcher = if let Some(path) = watch_path {
        log::info!("Watching for changes in: {}", path.display());
        Some(setup_file_watcher(path, state.clone())?)
    } else {
        None
    };

    loop {
        let (stream, peer) = listener.accept().await?;
        log::info!("New connection from {}", peer);
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, state).await {
                log::error!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, state: Arc<ServerState>) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (ws_tx, mut ws_rx) = ws_stream.split();

    // Peek first message to determine connection type
    if let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Try to parse as Request (CLI client), ExtensionHello, or ResponseMessage
                if let Ok(request) = serde_json::from_str::<Request>(&text) {
                    // This is a CLI client connection
                    log::debug!("CLI connection, forwarding command");
                    handle_cli_connection(request, ws_tx, ws_rx, &state).await?;
                } else if let Ok(hello) = serde_json::from_str::<ExtensionHello>(&text) {
                    // Extension sent hello message
                    log::info!("Extension connected (clientType: {})", hello.client_type);
                    let (tx, rx) = mpsc::channel::<String>(32);
                    *state.extension_tx.write().await = Some(tx);
                    handle_extension_loop(ws_tx, ws_rx, rx, &state).await?;
                } else if let Ok(response_msg) = serde_json::from_str::<ResponseMessage>(&text) {
                    // First message is a response - treat as extension
                    log::info!("Extension connected");
                    handle_extension_connection_with_first_msg(
                        response_msg,
                        ws_tx,
                        ws_rx,
                        &state,
                    )
                    .await?;
                } else {
                    // Unknown message format, assume extension
                    log::warn!("Unknown first message format, assuming extension: {}", text);
                    let (tx, rx) = mpsc::channel::<String>(32);
                    *state.extension_tx.write().await = Some(tx);
                    handle_extension_loop(ws_tx, ws_rx, rx, &state).await?;
                }
            }
            Ok(Message::Close(_)) => {
                return Ok(());
            }
            Err(e) => {
                anyhow::bail!("WebSocket error on first message: {}", e);
            }
            _ => {
                // Binary or other message, treat as extension
                log::info!("Extension connected");
                let (tx, rx) = mpsc::channel::<String>(32);
                *state.extension_tx.write().await = Some(tx);
                handle_extension_loop(ws_tx, ws_rx, rx, &state).await?;
            }
        }
    }

    Ok(())
}

async fn handle_cli_connection(
    request: Request,
    mut ws_tx: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    _ws_rx: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    state: &Arc<ServerState>,
) -> Result<()> {
    // Forward command to extension and get response
    let response = state.send_command(request.command).await?;
    let response_msg = ResponseMessage {
        id: request.id,
        response,
    };
    let json = serde_json::to_string(&response_msg)?;
    ws_tx.send(Message::Text(json.into())).await?;
    log::debug!("CLI command completed");
    Ok(())
}

async fn handle_extension_connection_with_first_msg(
    first_response: ResponseMessage,
    ws_tx: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    ws_rx: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    state: &Arc<ServerState>,
) -> Result<()> {
    // Handle the first response
    if let Some(tx) = state.pending_requests.write().await.remove(&first_response.id) {
        let _ = tx.send(first_response.response);
    }

    // Set up extension channel
    let (tx, rx) = mpsc::channel::<String>(32);
    *state.extension_tx.write().await = Some(tx);

    handle_extension_loop(ws_tx, ws_rx, rx, state).await
}

async fn handle_extension_loop(
    mut ws_tx: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    mut ws_rx: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    mut rx: mpsc::Receiver<String>,
    state: &Arc<ServerState>,
) -> Result<()> {
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                ws_tx.send(Message::Text(msg.into())).await?;
            }
            Some(msg) = ws_rx.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(response_msg) = serde_json::from_str::<ResponseMessage>(&text) {
                            if let Some(tx) = state.pending_requests.write().await.remove(&response_msg.id) {
                                let _ = tx.send(response_msg.response);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        log::info!("Extension disconnected");
                        break;
                    }
                    Err(e) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            else => break,
        }
    }

    *state.extension_tx.write().await = None;
    Ok(())
}

fn setup_file_watcher(path: &Path, state: Arc<ServerState>) -> Result<RecommendedWatcher> {
    use std::time::{Duration, Instant};

    let last_reload = Arc::new(std::sync::Mutex::new(
        Instant::now() - Duration::from_secs(10),
    ));
    let debounce_duration = Duration::from_millis(500);

    let watcher_state = state.clone();
    let watcher_last_reload = last_reload.clone();
    let runtime_handle = tokio::runtime::Handle::current();

    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        let is_extension_file = event.paths.iter().any(|p| {
                            let ext = p.extension().and_then(|e| e.to_str());
                            matches!(ext, Some("js") | Some("json") | Some("html") | Some("css"))
                        });

                        if !is_extension_file {
                            return;
                        }

                        let mut last = watcher_last_reload.lock().unwrap();
                        if last.elapsed() < debounce_duration {
                            return;
                        }
                        *last = Instant::now();
                        drop(last);

                        log::info!("Extension file changed: {:?}", event.paths);

                        let state_clone = watcher_state.clone();
                        runtime_handle.spawn(async move {
                            if let Err(e) = state_clone.broadcast_reload().await {
                                log::error!("Failed to broadcast reload: {}", e);
                            }
                        });
                    }
                    _ => {}
                }
            }
        })?;

    watcher.watch(path, RecursiveMode::Recursive)?;
    Ok(watcher)
}

/// CLI client to connect to the server and send commands
pub async fn send_command_to_server(port: u16, command: Command) -> Result<Response> {
    let url = format!("ws://127.0.0.1:{}", port);
    let (ws_stream, _) = connect_async(&url)
        .await
        .context(format!("Failed to connect to server at {}", url))?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let request = Request { id: 1, command };
    let json = serde_json::to_string(&request)?;
    ws_tx.send(Message::Text(json.into())).await?;

    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response: ResponseMessage = serde_json::from_str(&text)?;
                return Ok(response.response);
            }
            Ok(Message::Close(_)) => {
                anyhow::bail!("Connection closed before response");
            }
            Err(e) => {
                anyhow::bail!("WebSocket error: {}", e);
            }
            _ => {}
        }
    }

    anyhow::bail!("No response received")
}
