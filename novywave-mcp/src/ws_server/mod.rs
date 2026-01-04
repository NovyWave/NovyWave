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
use tokio_tungstenite::{accept_async, tungstenite::Message};

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
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let (tx, mut rx) = mpsc::channel::<String>(32);
    *state.extension_tx.write().await = Some(tx);

    log::info!("Extension connected");

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
