use std::{
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command},
    time::{Duration, Instant},
};

const DEV_SERVER_PORT: u16 = 8082;
const BACKEND_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const CDP_PORT: u16 = 9222;

fn main() {
    let (origin, backend_child) = select_runtime_target();

    println!("NovyWave Chrome mode: {origin}");

    let chrome = find_chrome().unwrap_or_else(|| {
        let gui_message = "NovyWave (Chrome mode) requires Chrome or Chromium.\n\n\
            Install a Chromium-based browser and try again,\n\
            or use the standard NovyWave binary instead.";
        show_error_dialog(gui_message);
        eprintln!();
        eprintln!("NovyWave (Chrome mode) requires Chrome or Chromium to be installed.");
        eprintln!();
        eprintln!("Install one of:");
        eprintln!("  Ubuntu/Debian:  sudo apt install chromium-browser");
        eprintln!("  Fedora/RHEL:    sudo dnf install chromium");
        eprintln!("  Arch:           sudo pacman -S chromium");
        eprintln!("  Snap:           sudo snap install chromium");
        eprintln!("  Flatpak:        flatpak install flathub org.chromium.Chromium");
        eprintln!("  macOS:          brew install --cask chromium");
        eprintln!("  Windows:        winget install Google.Chrome");
        eprintln!();
        eprintln!("Or use the standard NovyWave binary (Tauri) instead.");
        std::process::exit(1);
    });
    println!("Using browser: {}", chrome.display());

    let temp_profile = tempfile::tempdir().expect("failed to create temp profile dir");

    println!("Chrome DevTools Protocol available at http://127.0.0.1:{CDP_PORT}");

    let mut chrome_process = Command::new(&chrome)
        .arg(format!("--app={origin}"))
        .arg(format!("--user-data-dir={}", temp_profile.path().display()))
        .arg(format!("--remote-debugging-port={CDP_PORT}"))
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

// ---------------------------------------------------------------------------
// Chrome/Chromium detection (cross-platform)
// ---------------------------------------------------------------------------

fn find_chrome() -> Option<PathBuf> {
    find_chrome_in_path().or_else(find_chrome_in_known_locations)
}

#[cfg(unix)]
fn find_chrome_in_path() -> Option<PathBuf> {
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

#[cfg(windows)]
fn find_chrome_in_path() -> Option<PathBuf> {
    let candidates = ["chrome", "chromium", "msedge"];
    for name in &candidates {
        if let Ok(output) = Command::new("where").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn find_chrome_in_known_locations() -> Option<PathBuf> {
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

#[cfg(target_os = "windows")]
fn find_chrome_in_known_locations() -> Option<PathBuf> {
    let program_files = [
        std::env::var("ProgramFiles").unwrap_or_default(),
        std::env::var("ProgramFiles(x86)").unwrap_or_default(),
        std::env::var("LocalAppData").unwrap_or_default(),
    ];
    let relative_paths = [
        "Google\\Chrome\\Application\\chrome.exe",
        "Microsoft\\Edge\\Application\\msedge.exe",
        "Chromium\\Application\\chrome.exe",
    ];
    for base in &program_files {
        if base.is_empty() {
            continue;
        }
        for rel in &relative_paths {
            let candidate = PathBuf::from(base).join(rel);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn find_chrome_in_known_locations() -> Option<PathBuf> {
    let candidates = [
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/google-chrome",
        "/snap/bin/chromium",
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

// ---------------------------------------------------------------------------
// Native error dialog (cross-platform)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn show_error_dialog(message: &str) {
    let _ = Command::new("zenity")
        .args(["--error", "--title=NovyWave", "--text", message, "--width=420"])
        .status()
        .ok()
        .is_some_and(|s| s.success())
    || Command::new("kdialog")
        .args(["--error", message, "--title", "NovyWave"])
        .status()
        .ok()
        .is_some_and(|s| s.success())
    || Command::new("xmessage")
        .args(["-center", message])
        .status()
        .ok()
        .is_some_and(|s| s.success());
}

#[cfg(target_os = "macos")]
fn show_error_dialog(message: &str) {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    let _ = Command::new("osascript")
        .args([
            "-e",
            &format!("display dialog \"{escaped}\" with title \"NovyWave\" buttons {{\"OK\"}} default button \"OK\" with icon stop"),
        ])
        .status();
}

#[cfg(target_os = "windows")]
fn show_error_dialog(message: &str) {
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', 'NovyWave', 'OK', 'Error')",
                message.replace('\'', "''")
            ),
        ])
        .status();
}

// ---------------------------------------------------------------------------
// Backend spawning (shared across platforms)
// ---------------------------------------------------------------------------

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

fn backend_binary_name() -> &'static str {
    if cfg!(windows) { "backend.exe" } else { "backend" }
}

fn spawn_backend(port: u16) -> Result<Child, String> {
    if TcpListener::bind(("127.0.0.1", port)).is_err() {
        return Err(format!("port {port} already in use"));
    }

    let bin = backend_binary_name();
    let mut candidates: Vec<PathBuf> = vec![];

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join(bin));
            candidates.push(parent.join(format!("../{bin}")));
            candidates.push(parent.join(format!("../../{bin}")));
            candidates.push(parent.join(format!("../target/release/{bin}")));
            candidates.push(parent.join(format!("../target/debug/{bin}")));
        }
    }
    candidates.push(PathBuf::from(format!("target/release/{bin}")));
    candidates.push(PathBuf::from(format!("target/debug/{bin}")));

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
