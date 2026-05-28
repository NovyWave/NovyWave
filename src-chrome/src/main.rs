use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const DEV_SERVER_PORT: u16 = 8082;
const BACKEND_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const CDP_PORT: u16 = 9222;
const MIN_CHROMIUM_MAJOR: u32 = 109;
const DEFAULT_WORKSPACE_ENV: &str = "NOVYWAVE_DEFAULT_WORKSPACE";

fn main() {
    let chrome = find_chrome().unwrap_or_else(|error| {
        exit_with_startup_error("Chrome/Chromium browser not available", &error.to_string());
    });
    println!(
        "Using browser: {} ({})",
        chrome.display(),
        chrome.version_summary()
    );

    let (origin, mut backend_child) = select_runtime_target().unwrap_or_else(|error| {
        exit_with_startup_error(
            "NovyWave (Chrome mode) could not start its backend",
            &format!(
                "{error}\n\nUse the packaged Chrome AppImage, run makers start before the \
                 launcher, or use the standard NovyWave binary instead."
            ),
        );
    });

    println!("NovyWave Chrome mode: {origin}");

    let temp_profile = tempfile::tempdir().unwrap_or_else(|error| {
        if let Some(child) = backend_child.take() {
            stop_backend(child);
        }
        exit_with_startup_error(
            "NovyWave failed to create a temporary browser profile",
            &error.to_string(),
        );
    });

    let cdp_port = chrome_devtools_port();
    println!("Chrome DevTools Protocol available at http://127.0.0.1:{cdp_port}");

    let mut command = Command::new(&chrome.program);
    command
        .args(&chrome.args)
        .arg(format!("--app={origin}"))
        .arg(format!("--user-data-dir={}", temp_profile.path().display()))
        .arg(format!("--remote-debugging-port={cdp_port}"))
        .arg("--no-first-run")
        .arg("--no-default-browser-check");

    let mut chrome_process = command.spawn().unwrap_or_else(|error| {
        if let Some(child) = backend_child.take() {
            stop_backend(child);
        }
        exit_with_startup_error("NovyWave failed to launch Chrome", &error.to_string());
    });

    let status = chrome_process.wait().unwrap_or_else(|error| {
        if let Some(child) = backend_child.take() {
            stop_backend(child);
        }
        exit_with_startup_error(
            "NovyWave failed while waiting for Chrome",
            &error.to_string(),
        );
    });

    if let Some(child) = backend_child {
        stop_backend(child);
    }

    std::process::exit(status.code().unwrap_or(0));
}

fn stop_backend(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn chrome_devtools_port() -> u16 {
    chrome_devtools_port_from_env(env::var("NOVYWAVE_CHROME_CDP_PORT").ok().as_deref())
}

fn chrome_devtools_port_from_env(value: Option<&str>) -> u16 {
    value
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port > 0)
        .unwrap_or(CDP_PORT)
}

// ---------------------------------------------------------------------------
// Chrome/Chromium detection (cross-platform)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
struct BrowserCommand {
    program: PathBuf,
    args: Vec<String>,
    label: String,
    version_text: String,
    chromium_major: Option<u32>,
}

impl BrowserCommand {
    fn new(program: PathBuf, label: impl Into<String>) -> Self {
        Self {
            program,
            args: vec![],
            label: label.into(),
            version_text: String::new(),
            chromium_major: None,
        }
    }

    fn with_args(program: PathBuf, args: Vec<String>, label: impl Into<String>) -> Self {
        Self {
            program,
            args,
            label: label.into(),
            version_text: String::new(),
            chromium_major: None,
        }
    }

    fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.display().to_string()
        } else {
            format!("{} {}", self.program.display(), self.args.join(" "))
        }
    }

    fn version_summary(&self) -> String {
        match self.chromium_major {
            Some(major) => format!("{}; Chromium major {}", self.version_text, major),
            None => self.version_text.clone(),
        }
    }

    fn with_version(mut self, version_text: String, chromium_major: Option<u32>) -> Self {
        self.version_text = version_text;
        self.chromium_major = chromium_major;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RejectedBrowser {
    label: String,
    command: String,
    reason: String,
}

impl RejectedBrowser {
    fn new(
        label: impl Into<String>,
        command: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            command: command.into(),
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BrowserSearchError {
    rejected: Vec<RejectedBrowser>,
}

impl std::fmt::Display for BrowserSearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            formatter,
            "NovyWave (Chrome mode) requires Chrome, Chromium, Edge, Brave, or another \
             Chromium-based browser with Chromium {MIN_CHROMIUM_MAJOR} or newer."
        )?;
        writeln!(formatter)?;
        writeln!(
            formatter,
            "How to fix: install or update a Chromium-based browser, then run this AppImage again."
        )?;
        writeln!(
            formatter,
            "If a supported browser is already installed, set NOVYWAVE_CHROME to its executable path."
        )?;
        writeln!(formatter)?;
        writeln!(formatter, "Install one of:")?;
        writeln!(
            formatter,
            "  Ubuntu/Debian:  sudo apt install chromium-browser"
        )?;
        writeln!(formatter, "  Fedora/RHEL:    sudo dnf install chromium")?;
        writeln!(formatter, "  Arch:           sudo pacman -S chromium")?;
        writeln!(formatter, "  Snap:           sudo snap install chromium")?;
        writeln!(
            formatter,
            "  Flatpak:        flatpak install flathub org.chromium.Chromium"
        )?;
        writeln!(formatter)?;
        writeln!(
            formatter,
            "You can also set NOVYWAVE_CHROME or CHROME_BIN to the browser executable path."
        )?;
        writeln!(
            formatter,
            "Example: NOVYWAVE_CHROME=/usr/bin/google-chrome-stable NovyWave_chrome.AppImage"
        )?;

        if !self.rejected.is_empty() {
            writeln!(formatter)?;
            writeln!(formatter, "Checked candidates:")?;
            for rejected in &self.rejected {
                writeln!(
                    formatter,
                    "  - {} ({}): {}",
                    rejected.label, rejected.command, rejected.reason
                )?;
            }
        }

        Ok(())
    }
}

fn find_chrome() -> Result<BrowserCommand, BrowserSearchError> {
    let mut rejected = vec![];

    if let Some(result) = find_explicit_chrome_override(&mut rejected) {
        return result.map_err(|rejected| BrowserSearchError { rejected });
    }

    if let Some(browser) = find_browser_env_candidate(&mut rejected) {
        return Ok(browser);
    }

    for candidate in browser_candidates(&mut rejected) {
        match validate_browser(candidate) {
            Ok(browser) => return Ok(browser),
            Err(reason) => rejected.push(reason),
        }
    }

    Err(BrowserSearchError { rejected })
}

fn find_explicit_chrome_override(
    rejected: &mut Vec<RejectedBrowser>,
) -> Option<Result<BrowserCommand, Vec<RejectedBrowser>>> {
    for variable in ["NOVYWAVE_CHROME", "CHROME_BIN"] {
        let Ok(value) = env::var(variable) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        let Some(candidate) = browser_command_from_override(value, variable, rejected) else {
            return Some(Err(rejected.clone()));
        };

        return Some(validate_browser(candidate).map_err(|reason| {
            let mut rejected = rejected.clone();
            rejected.push(reason);
            rejected
        }));
    }

    None
}

fn find_browser_env_candidate(rejected: &mut Vec<RejectedBrowser>) -> Option<BrowserCommand> {
    let Ok(value) = env::var("BROWSER") else {
        return None;
    };
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let candidate = browser_command_from_override(value, "BROWSER", rejected)?;
    match validate_browser(candidate) {
        Ok(browser) => Some(browser),
        Err(reason) => {
            rejected.push(reason);
            None
        }
    }
}

fn browser_command_from_override(
    value: &str,
    label: &str,
    rejected: &mut Vec<RejectedBrowser>,
) -> Option<BrowserCommand> {
    let path = PathBuf::from(value);
    if value.contains('/') || value.contains('\\') || path.is_absolute() {
        if !path.exists() {
            rejected.push(RejectedBrowser::new(label, value, "path does not exist"));
            return None;
        }
        if !is_executable_file(&path) {
            rejected.push(RejectedBrowser::new(label, value, "path is not executable"));
            return None;
        }
        return Some(BrowserCommand::new(path, label));
    }

    find_in_path(value)
        .map(|path| BrowserCommand::new(path, label))
        .or_else(|| {
            rejected.push(RejectedBrowser::new(label, value, "not found on PATH"));
            None
        })
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn browser_candidates(rejected: &mut Vec<RejectedBrowser>) -> Vec<BrowserCommand> {
    let mut candidates = vec![];

    for name in chrome_path_names() {
        match find_in_path(name) {
            Some(path) => candidates.push(BrowserCommand::new(path, *name)),
            None => rejected.push(RejectedBrowser::new(*name, *name, "not found on PATH")),
        }
    }

    candidates.extend(chrome_known_location_candidates(rejected));
    candidates.extend(chrome_flatpak_candidates(rejected));

    candidates
}

#[cfg(unix)]
fn find_in_path(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

#[cfg(windows)]
fn find_in_path(name: &str) -> Option<PathBuf> {
    let output = Command::new("where").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

#[cfg(unix)]
fn chrome_path_names() -> &'static [&'static str] {
    &[
        "chromium-browser",
        "chromium",
        "google-chrome-stable",
        "google-chrome",
        "microsoft-edge-stable",
        "microsoft-edge",
        "brave-browser",
        "brave",
        "vivaldi-stable",
        "vivaldi",
    ]
}

#[cfg(windows)]
fn chrome_path_names() -> &'static [&'static str] {
    &["chrome", "chromium", "msedge", "brave", "vivaldi"]
}

#[cfg(target_os = "macos")]
fn chrome_known_location_candidates(rejected: &mut Vec<RejectedBrowser>) -> Vec<BrowserCommand> {
    let candidates = [
        (
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "Google Chrome",
        ),
        (
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "Chromium",
        ),
        (
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "Microsoft Edge",
        ),
        (
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "Brave",
        ),
        (
            "/Applications/Vivaldi.app/Contents/MacOS/Vivaldi",
            "Vivaldi",
        ),
    ];
    known_path_candidates(&candidates, rejected)
}

#[cfg(target_os = "windows")]
fn chrome_known_location_candidates(rejected: &mut Vec<RejectedBrowser>) -> Vec<BrowserCommand> {
    let program_files = [
        env::var("ProgramFiles").unwrap_or_default(),
        env::var("ProgramFiles(x86)").unwrap_or_default(),
        env::var("LocalAppData").unwrap_or_default(),
    ];
    let relative_paths = [
        ("Google\\Chrome\\Application\\chrome.exe", "Google Chrome"),
        ("Microsoft\\Edge\\Application\\msedge.exe", "Microsoft Edge"),
        (
            "BraveSoftware\\Brave-Browser\\Application\\brave.exe",
            "Brave",
        ),
        ("Vivaldi\\Application\\vivaldi.exe", "Vivaldi"),
        ("Chromium\\Application\\chrome.exe", "Chromium"),
    ];
    let mut browsers = vec![];
    for base in &program_files {
        if base.is_empty() {
            continue;
        }
        for (relative_path, label) in &relative_paths {
            let candidate = PathBuf::from(base).join(relative_path);
            if candidate.exists() {
                browsers.push(BrowserCommand::new(candidate, *label));
            } else {
                rejected.push(RejectedBrowser::new(
                    *label,
                    candidate.display().to_string(),
                    "known install path not found",
                ));
            }
        }
    }
    browsers
}

#[cfg(target_os = "linux")]
fn chrome_known_location_candidates(rejected: &mut Vec<RejectedBrowser>) -> Vec<BrowserCommand> {
    let candidates = [
        ("/usr/bin/chromium-browser", "Chromium"),
        ("/usr/bin/chromium", "Chromium"),
        ("/usr/bin/google-chrome-stable", "Google Chrome"),
        ("/usr/bin/google-chrome", "Google Chrome"),
        ("/usr/bin/microsoft-edge-stable", "Microsoft Edge"),
        ("/usr/bin/microsoft-edge", "Microsoft Edge"),
        ("/usr/bin/brave-browser", "Brave"),
        ("/usr/bin/vivaldi-stable", "Vivaldi"),
        ("/usr/bin/vivaldi", "Vivaldi"),
        ("/snap/bin/chromium", "Chromium Snap"),
    ];
    known_path_candidates(&candidates, rejected)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn known_path_candidates(
    candidates: &[(&str, &str)],
    rejected: &mut Vec<RejectedBrowser>,
) -> Vec<BrowserCommand> {
    candidates
        .iter()
        .filter_map(|(path, label)| {
            let path = PathBuf::from(path);
            if path.exists() {
                Some(BrowserCommand::new(path, *label))
            } else {
                rejected.push(RejectedBrowser::new(
                    *label,
                    path.display().to_string(),
                    "known install path not found",
                ));
                None
            }
        })
        .collect()
}

#[cfg(not(target_os = "linux"))]
fn chrome_flatpak_candidates(_rejected: &mut Vec<RejectedBrowser>) -> Vec<BrowserCommand> {
    vec![]
}

#[cfg(target_os = "linux")]
fn chrome_flatpak_candidates(rejected: &mut Vec<RejectedBrowser>) -> Vec<BrowserCommand> {
    let Some(flatpak) = find_in_path("flatpak") else {
        rejected.push(RejectedBrowser::new(
            "Flatpak",
            "flatpak",
            "flatpak command not found",
        ));
        return vec![];
    };
    let candidates = [
        ("org.chromium.Chromium", "Chromium Flatpak"),
        ("com.google.Chrome", "Google Chrome Flatpak"),
        ("com.brave.Browser", "Brave Flatpak"),
        ("com.microsoft.Edge", "Microsoft Edge Flatpak"),
        ("com.vivaldi.Vivaldi", "Vivaldi Flatpak"),
    ];

    candidates
        .iter()
        .filter_map(|(app_id, label)| {
            let status = Command::new(&flatpak)
                .args(["info", app_id])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok()?;
            if status.success() {
                Some(BrowserCommand::with_args(
                    flatpak.clone(),
                    vec!["run".to_string(), (*app_id).to_string()],
                    *label,
                ))
            } else {
                rejected.push(RejectedBrowser::new(
                    *label,
                    format!("flatpak run {app_id}"),
                    "Flatpak app is not installed",
                ));
                None
            }
        })
        .collect()
}

fn validate_browser(candidate: BrowserCommand) -> Result<BrowserCommand, RejectedBrowser> {
    let output = Command::new(&candidate.program)
        .args(&candidate.args)
        .arg("--version")
        .output()
        .map_err(|error| {
            RejectedBrowser::new(
                candidate.label.clone(),
                candidate.display(),
                format!("failed to run --version: {error}"),
            )
        })?;

    let version_text = format_version_output(&output.stdout, &output.stderr);
    if !output.status.success() {
        return Err(RejectedBrowser::new(
            candidate.label.clone(),
            candidate.display(),
            format!("--version exited with {}", output.status),
        ));
    }

    if !is_chromium_based_version(&version_text) {
        return Err(RejectedBrowser::new(
            candidate.label.clone(),
            candidate.display(),
            format!("not a Chromium-based browser: '{version_text}'"),
        ));
    }

    let chromium_major = chromium_major_from_version(&version_text);
    if let Some(major) = chromium_major {
        if major < MIN_CHROMIUM_MAJOR {
            return Err(RejectedBrowser::new(
                candidate.label.clone(),
                candidate.display(),
                format!(
                    "Chromium major version {major} is too old; need {MIN_CHROMIUM_MAJOR} or newer"
                ),
            ));
        }
    }

    Ok(candidate.with_version(version_text, chromium_major))
}

fn format_version_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let version = [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if version.is_empty() {
        "<empty version output>".to_string()
    } else {
        truncate_for_error(version.lines().next().unwrap_or("").trim(), 240)
    }
}

fn truncate_for_error(text: &str, max_chars: usize) -> String {
    let mut truncated = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        truncated.push_str("...");
    }
    truncated
}

fn is_chromium_based_version(version: &str) -> bool {
    let lower = version.to_ascii_lowercase();
    [
        "chromium",
        "google chrome",
        "microsoft edge",
        "brave browser",
        "vivaldi",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn chromium_major_from_version(version: &str) -> Option<u32> {
    let lower = version.to_ascii_lowercase();
    for marker in [
        "chromium:",
        "chromium",
        "google chrome",
        "microsoft edge",
        "chrome",
    ] {
        if let Some(index) = lower.find(marker) {
            return first_version_major(&version[index + marker.len()..]);
        }
    }
    None
}

fn first_version_major(text: &str) -> Option<u32> {
    let mut digits = String::new();
    let mut started = false;

    for character in text.chars() {
        if character.is_ascii_digit() {
            started = true;
            digits.push(character);
        } else if started {
            break;
        }
    }

    digits.parse().ok()
}

// ---------------------------------------------------------------------------
// Native error dialog/logging (cross-platform)
// ---------------------------------------------------------------------------

fn exit_with_startup_error(step: &str, detail: &str) -> ! {
    let message = format!("NovyWave failed to start.\n\n{step}\n\n{detail}");
    eprintln!("{message}");
    let log_path = write_startup_error_log(&message);
    let message_with_log = match log_path {
        Some(path) => format!("{message}\n\nLog file:\n{}", path.display()),
        None => message,
    };
    show_error_dialog(&message_with_log);
    std::process::exit(1);
}

fn write_startup_error_log(message: &str) -> Option<PathBuf> {
    let dir = startup_log_dir()?;
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join("novywave-chrome.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    let _ = writeln!(file, "{}", "-".repeat(72));
    let _ = writeln!(file, "{message}");
    Some(path)
}

#[cfg(target_os = "linux")]
fn startup_log_dir() -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .map(|dir| dir.join("novywave"))
}

#[cfg(target_os = "linux")]
fn app_config_dir() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("novywave"))
}

#[cfg(target_os = "macos")]
fn startup_log_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("NovyWave")
    })
}

#[cfg(target_os = "macos")]
fn app_config_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("novywave")
    })
}

#[cfg(target_os = "windows")]
fn startup_log_dir() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| env::var_os("TEMP").map(PathBuf::from))
        .map(|dir| dir.join("NovyWave"))
}

#[cfg(target_os = "windows")]
fn app_config_dir() -> Option<PathBuf> {
    env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .map(|dir| dir.join("novywave"))
}

fn default_workspace_dir() -> Option<PathBuf> {
    app_config_dir().map(|dir| dir.join("default-workspace"))
}

#[cfg(target_os = "linux")]
fn show_error_dialog(message: &str) {
    let shown = Command::new("zenity")
        .args([
            "--error",
            "--no-markup",
            "--title=NovyWave",
            "--text",
            message,
            "--width=520",
        ])
        .status()
        .ok()
        .is_some_and(|status| status.success())
        || Command::new("kdialog")
            .args(["--error", message, "--title", "NovyWave"])
            .status()
            .ok()
            .is_some_and(|status| status.success())
        || Command::new("xmessage")
            .args(["-center", message])
            .status()
            .ok()
            .is_some_and(|status| status.success());

    if !shown {
        let _ = notify_rust::Notification::new()
            .summary("NovyWave failed to start")
            .body(message)
            .show();
    }
}

#[cfg(target_os = "macos")]
fn show_error_dialog(message: &str) {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    let _ = Command::new("osascript")
        .args([
            "-e",
            &format!(
                "display dialog \"{escaped}\" with title \"NovyWave\" buttons {{\"OK\"}} default button \"OK\" with icon stop"
            ),
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

fn select_runtime_target() -> Result<(String, Option<Child>), String> {
    if dev_server_is_available(DEV_SERVER_PORT) {
        println!("Using dev server on port {DEV_SERVER_PORT}");
        return Ok((format!("http://127.0.0.1:{DEV_SERVER_PORT}"), None));
    }

    let backend_port =
        portpicker::pick_unused_port().ok_or("failed to find unused port".to_string())?;
    let child = spawn_backend(backend_port)?;
    wait_for_backend_ready(backend_port)?;

    Ok((format!("http://127.0.0.1:{backend_port}"), Some(child)))
}

fn dev_server_is_available(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

fn backend_binary_name() -> &'static str {
    if cfg!(windows) {
        "backend.exe"
    } else {
        "backend"
    }
}

fn spawn_backend(port: u16) -> Result<Child, String> {
    if TcpListener::bind(("127.0.0.1", port)).is_err() {
        return Err(format!("port {port} already in use"));
    }

    let bin = backend_binary_name();
    let current_exe = env::current_exe().ok();
    let mut candidates = backend_candidates(current_exe.as_deref(), bin);
    candidates.push(PathBuf::from(format!("target/release/{bin}")));
    candidates.push(PathBuf::from(format!("target/debug/{bin}")));

    let backend_path = candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| {
            format!(
                "backend binary '{bin}' not found.\n\nSearched:\n{}",
                format_path_candidates(&candidates)
            )
        })?;

    println!("Starting backend from: {}", backend_path.display());

    let workspace_root = backend_path
        .ancestors()
        .find(|path| path.join("frontend_dist").exists() || path.join("public").exists())
        .map(|path| path.to_path_buf())
        .or_else(|| backend_path.parent().map(|path| path.to_path_buf()));

    let mut command = Command::new(&backend_path);
    if let Some(root) = &workspace_root {
        command.current_dir(root);
        command.env("FRONTEND_DIST_DIR", root.join("frontend_dist"));
        command.env("MOON_ASSETS_DIR", root.join("frontend_dist"));
    }
    if let Some(default_workspace) = default_workspace_dir() {
        command.env(DEFAULT_WORKSPACE_ENV, default_workspace);
    }
    command.env("COMPRESSED_PKG", "false");
    command.env("FRONTEND_DIST", "true");
    command.env("PORT", port.to_string());
    command.env("REDIRECT_ENABLED", "false");

    command
        .spawn()
        .map_err(|error| format!("spawn failed for {}: {error}", backend_path.display()))
}

fn backend_candidates(current_exe: Option<&Path>, backend_name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![];

    if let Some(exe) = current_exe {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join(backend_name));
            candidates.push(parent.join(format!("../{backend_name}")));
            candidates.push(parent.join(format!("../../{backend_name}")));
            candidates.push(parent.join(format!("../target/release/{backend_name}")));
            candidates.push(parent.join(format!("../target/debug/{backend_name}")));
        }
    }

    candidates
}

fn format_path_candidates(candidates: &[PathBuf]) -> String {
    candidates
        .iter()
        .map(|path| format!("  - {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn wait_for_backend_ready(port: u16) -> Result<(), String> {
    let deadline = Instant::now() + BACKEND_STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!(
        "backend on port {port} didn't start within {BACKEND_STARTUP_TIMEOUT:?}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn backend_candidates_include_packaged_sibling_backend() {
        let candidates = backend_candidates(
            Some(Path::new("/opt/NovyWave/novywave-chrome-linux-x64")),
            "backend",
        );

        assert!(candidates.contains(&PathBuf::from("/opt/NovyWave/backend")));
        assert!(candidates.contains(&PathBuf::from("/opt/NovyWave/../backend")));
        assert!(candidates.contains(&PathBuf::from("/opt/NovyWave/../target/release/backend")));
    }

    #[test]
    fn browser_command_display_includes_flatpak_args() {
        let command = BrowserCommand::with_args(
            PathBuf::from("/usr/bin/flatpak"),
            vec!["run".to_string(), "org.chromium.Chromium".to_string()],
            "Chromium Flatpak",
        );

        assert_eq!(
            command.display(),
            "/usr/bin/flatpak run org.chromium.Chromium"
        );
    }

    #[test]
    fn chrome_devtools_port_parses_valid_override_only() {
        assert_eq!(chrome_devtools_port_from_env(None), CDP_PORT);
        assert_eq!(chrome_devtools_port_from_env(Some("")), CDP_PORT);
        assert_eq!(chrome_devtools_port_from_env(Some("0")), CDP_PORT);
        assert_eq!(chrome_devtools_port_from_env(Some("not-a-port")), CDP_PORT);
        assert_eq!(chrome_devtools_port_from_env(Some("4567")), 4567);
    }

    #[test]
    fn chromium_version_parses_supported_browsers() {
        assert_eq!(
            chromium_major_from_version("Chromium 120.0.6099.224"),
            Some(120)
        );
        assert_eq!(
            chromium_major_from_version("Google Chrome 121.0.6167.85"),
            Some(121)
        );
        assert_eq!(
            chromium_major_from_version("Brave Browser 1.64.109 Chromium: 123.0.6312.105"),
            Some(123)
        );
        assert_eq!(
            chromium_major_from_version("Microsoft Edge 122.0.2365.80"),
            Some(122)
        );
    }

    #[cfg(unix)]
    #[test]
    fn validation_rejects_firefox_and_old_chromium() {
        let firefox = fake_browser("Mozilla Firefox 126.0");
        let firefox_error = validate_browser(firefox).expect_err("Firefox must be rejected");
        assert!(
            firefox_error
                .reason
                .contains("not a Chromium-based browser")
        );

        let old_chromium = fake_browser("Chromium 108.0.5359.125");
        let old_error = validate_browser(old_chromium).expect_err("old Chromium must be rejected");
        assert!(old_error.reason.contains("too old"));
    }

    #[cfg(unix)]
    #[test]
    fn validation_accepts_supported_chromium() {
        let browser = validate_browser(fake_browser("Chromium 120.0.6099.224"))
            .expect("supported Chromium should pass");

        assert_eq!(browser.chromium_major, Some(120));
        assert_eq!(browser.version_text, "Chromium 120.0.6099.224");
    }

    #[cfg(unix)]
    fn fake_browser(version: &str) -> BrowserCommand {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fake-browser");
        fs::write(
            &path,
            format!("#!/usr/bin/env bash\nprintf '%s\\n' '{}'\n", version),
        )
        .expect("write fake browser");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("chmod");

        // Keep the temp directory alive by leaking it for the duration of the test process.
        let _ = Box::leak(Box::new(dir));
        BrowserCommand::new(path, "fake")
    }
}
