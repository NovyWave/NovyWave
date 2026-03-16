use std::{
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command},
    time::{Duration, Instant},
};

const DEV_SERVER_PORT: u16 = 8082;
const BACKEND_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

fn main() {
    let (origin, backend_child) = select_runtime_target();

    println!("NovyWave Chrome mode: {origin}");

    let chrome = find_chrome().unwrap_or_else(|| {
        eprintln!("ERROR: Chrome/Chromium not found. Install chromium-browser or google-chrome.");
        std::process::exit(1);
    });
    println!("Using browser: {}", chrome.display());

    let temp_profile = tempfile::tempdir().expect("failed to create temp profile dir");

    let mut chrome_process = Command::new(&chrome)
        .arg(format!("--app={origin}"))
        .arg(format!("--user-data-dir={}", temp_profile.path().display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("ERROR: Failed to launch Chrome: {e}");
            std::process::exit(1);
        });

    let status = chrome_process.wait().unwrap_or_else(|e| {
        eprintln!("ERROR: Failed to wait for Chrome: {e}");
        std::process::exit(1);
    });

    if let Some(mut child) = backend_child {
        let _ = child.kill();
        let _ = child.wait();
    }

    std::process::exit(status.code().unwrap_or(0));
}

fn find_chrome() -> Option<PathBuf> {
    let candidates = [
        "chromium-browser",
        "chromium",
        "google-chrome-stable",
        "google-chrome",
    ];
    for name in &candidates {
        if let Ok(output) = Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

fn select_runtime_target() -> (String, Option<Child>) {
    if dev_server_is_available(DEV_SERVER_PORT) {
        println!("Using dev server on port {DEV_SERVER_PORT}");
        return (format!("http://127.0.0.1:{DEV_SERVER_PORT}"), None);
    }

    let backend_port = portpicker::pick_unused_port().expect("failed to find unused port");
    let child = spawn_backend(backend_port).unwrap_or_else(|e| {
        eprintln!("ERROR: Failed to spawn backend: {e}");
        std::process::exit(1);
    });
    wait_for_backend_ready(backend_port).unwrap_or_else(|e| {
        eprintln!("ERROR: Backend didn't start: {e}");
        std::process::exit(1);
    });

    (format!("http://127.0.0.1:{backend_port}"), Some(child))
}

fn dev_server_is_available(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

fn spawn_backend(port: u16) -> Result<Child, String> {
    if TcpListener::bind(("127.0.0.1", port)).is_err() {
        return Err(format!("port {port} already in use"));
    }

    let mut candidates: Vec<PathBuf> = vec![];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("backend"));
            candidates.push(parent.join("../backend"));
            candidates.push(parent.join("../../backend"));
            candidates.push(parent.join("../target/release/backend"));
            candidates.push(parent.join("../target/debug/backend"));
        }
    }
    candidates.push(PathBuf::from("target/release/backend"));
    candidates.push(PathBuf::from("target/debug/backend"));

    let backend_path = candidates
        .into_iter()
        .find(|p| p.exists())
        .ok_or("backend binary not found")?;

    println!("Starting backend from: {}", backend_path.display());

    let workspace_root = backend_path
        .ancestors()
        .find(|p| p.join("frontend_dist").exists() || p.join("public").exists())
        .map(|p| p.to_path_buf())
        .or_else(|| backend_path.parent().map(|p| p.to_path_buf()));

    let mut command = Command::new(&backend_path);
    if let Some(root) = &workspace_root {
        command.current_dir(root);
        command.env("FRONTEND_DIST_DIR", root.join("frontend_dist"));
        command.env("MOON_ASSETS_DIR", root.join("frontend_dist"));
    }
    command.env("COMPRESSED_PKG", "false");
    command.env("FRONTEND_DIST", "true");
    command.env("PORT", port.to_string());
    command.env("REDIRECT_ENABLED", "false");

    command.spawn().map_err(|e| format!("spawn failed: {e}"))
}

fn wait_for_backend_ready(port: u16) -> Result<(), String> {
    let deadline = Instant::now() + BACKEND_STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!("backend on port {port} didn't start within {BACKEND_STARTUP_TIMEOUT:?}"))
}
