mod commands;
mod desktop_test_bridge;
mod update_policy;

use std::{
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use portpicker::pick_unused_port;
use tauri::{Manager, WindowEvent};

// Keep handle to embedded backend so we can terminate it when the app closes.
static BACKEND_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
const DEV_SERVER_PORT: u16 = 8082;
const EMBEDDED_BACKEND_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

struct TauriRuntimeTarget {
    origin: String,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let test_updater = std::env::args().any(|a| a == "--test-updater");

    #[cfg(target_os = "linux")]
    let linux_webkit_profile = configure_linux_webkit_environment();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::load_config,
            commands::save_config,
            commands::load_workspace_history,
            commands::save_workspace_history,
            commands::load_waveform_file,
            commands::browse_directory,
            commands::browse_directories,
            commands::query_signal_values,
            commands::query_signal_transitions,
            commands::get_parsing_progress,
            commands::request_update_download,
            commands::request_app_restart
        ])
        .setup(move |app| {
            println!("=== Tauri app setup completed ===");

            if !test_updater {
                let runtime_target = select_runtime_target(app).map_err(|error| {
                    startup_io_error("Failed to prepare NovyWave runtime", &error)
                })?;

                let target: tauri::Url = runtime_target.origin.parse().map_err(|error| {
                    startup_io_error(
                        "Failed to parse NovyWave backend URL",
                        &format!("{}\n\n{error}", runtime_target.origin),
                    )
                })?;
                let init_script = format!(
                    "window.__NOVYWAVE_BACKEND_ORIGIN = {};",
                    serde_json::to_string(&runtime_target.origin).map_err(|error| {
                        println!("Failed to encode runtime origin: {error}");
                        std::io::Error::other("failed to encode runtime origin")
                    })?
                );

                let main_window = tauri::WebviewWindowBuilder::new(
                    app,
                    "main".to_string(),
                    tauri::WebviewUrl::External(target),
                )
                .initialization_script(init_script)
                .title("NovyWave")
                .inner_size(1200.0, 800.0)
                .devtools(true)
                .build()
                .map_err(|e| {
                    show_startup_error("Failed to create NovyWave window", &e.to_string());
                    e
                })?;

                #[cfg(target_os = "linux")]
                {
                    let _ = main_window.with_webview(move |webview| {
                        use webkit2gtk::{SettingsExt, WebViewExt};
                        if let Some(settings) = webview.inner().settings() {
                            let (policy, policy_name, accelerated_2d_canvas) =
                                match linux_webkit_profile {
                                    LinuxWebkitProfile::Software => (
                                        webkit2gtk::HardwareAccelerationPolicy::Never,
                                        "Never",
                                        false,
                                    ),
                                    LinuxWebkitProfile::Auto
                                    | LinuxWebkitProfile::LegacyNvidia => (
                                        webkit2gtk::HardwareAccelerationPolicy::Always,
                                        "Always",
                                        true,
                                    ),
                                };
                            settings.set_hardware_acceleration_policy(policy);
                            #[allow(deprecated)]
                            settings.set_enable_accelerated_2d_canvas(accelerated_2d_canvas);
                            println!(
                                "WebKitGTK: profile={}, hardware_acceleration={}, accelerated_2d_canvas={}",
                                linux_webkit_profile, policy_name, accelerated_2d_canvas
                            );
                        }
                    });
                }
                let _ = main_window;

                desktop_test_bridge::start(&app.handle());
            }

            let handle = app.handle();
            let update_capability = update_policy::UpdateCapability::current(test_updater);
            if updates_configured(&handle) {
                if test_updater {
                    let handle = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        match test_update_flow(handle).await {
                            Ok(()) => {
                                println!("VERIFY_UPDATER: PASSED");
                                std::process::exit(0);
                            }
                            Err(e) => {
                                println!("VERIFY_UPDATER: FAILED - {e}");
                                std::process::exit(1);
                            }
                        }
                    });
                } else if update_capability.is_supported() {
                    let handle_for_task = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        check_for_updates(handle_for_task, update_capability).await;
                    });
                } else if let Some(reason) = update_capability.hidden_reason() {
                    println!("Updater hidden for this build: {reason}");
                }
            } else if test_updater {
                println!("VERIFY_UPDATER: FAILED - updater not configured");
                std::process::exit(1);
            } else {
                println!(
                    "Updater disabled: missing endpoints or pubkey in tauri.conf.json (local build)."
                );
            }

            Ok(())
        })
        .on_window_event(|_window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed) {
                stop_backend();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn select_runtime_target(app: &tauri::App) -> Result<TauriRuntimeTarget, String> {
    if tauri::is_dev() && dev_server_is_available(DEV_SERVER_PORT) {
        let origin = format!("http://127.0.0.1:{DEV_SERVER_PORT}");
        return Ok(TauriRuntimeTarget { origin });
    }

    if tauri::is_dev() {
        println!(
            "Dev server on 127.0.0.1:{DEV_SERVER_PORT} is unavailable, falling back to embedded backend."
        );
    }

    let backend_port =
        pick_unused_port().ok_or("failed to allocate an unused localhost port".to_string())?;
    spawn_embedded_backend(app, backend_port)?;
    wait_for_backend_ready(backend_port)?;

    Ok(TauriRuntimeTarget {
        origin: format!("http://127.0.0.1:{backend_port}"),
    })
}

fn dev_server_is_available(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

fn spawn_embedded_backend(app: &tauri::App, backend_port: u16) -> Result<(), String> {
    if TcpListener::bind(("127.0.0.1", backend_port)).is_err() {
        return Err(format!(
            "embedded backend port {backend_port} is already in use before spawn"
        ));
    }

    // Resolve backend binary from resources or fallback to workspace target
    let resolver = app.path();
    let resource_dir = resolver.resource_dir().ok();
    let current_exe = std::env::current_exe().ok();
    let candidates = backend_candidates(
        resource_dir.as_deref(),
        current_exe.as_deref(),
        backend_binary_name(),
    );

    let backend_path = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| {
            format!(
                "backend binary '{}' not found in resources or target paths.\n\nSearched:\n{}",
                backend_binary_name(),
                format_path_candidates(&candidates)
            )
        })?;

    println!("Starting embedded backend from: {}", backend_path.display());

    // Prefer running from the `_up_` root so the bundled `public/` assets are found.
    let mut command = Command::new(&backend_path);
    let mut up_root_opt = backend_path
        .ancestors()
        .find(|p| p.file_name().map(|n| n == "_up_").unwrap_or(false))
        .map(|p| p.to_path_buf());
    if up_root_opt.is_none() {
        if let Ok(dir) = resolver.resource_dir() {
            up_root_opt = Some(dir);
        }
    }
    if up_root_opt.is_none() {
        up_root_opt = backend_path.parent().map(|p| p.to_path_buf());
    }
    if let Some(up_root) = &up_root_opt {
        let dist_dir = up_root.join("frontend_dist");
        command.current_dir(up_root);
        command.env("FRONTEND_DIST_DIR", dist_dir);
    }
    if let Some(cwd) = command.get_current_dir() {
        println!("spawn backend cwd {}", cwd.display());
    }
    let env_snapshot: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)> = command
        .get_envs()
        .map(|(k, v)| (k.to_os_string(), v.map(|s| s.to_os_string())))
        .collect();
    let dist_env = env_snapshot
        .iter()
        .find(|(k, _)| k == "FRONTEND_DIST_DIR")
        .and_then(|(_, v)| v.clone());
    let compressed_env = env_snapshot
        .iter()
        .find(|(k, _)| k == "COMPRESSED_PKG")
        .and_then(|(_, v)| v.clone());

    println!(
        "spawn backend env FRONTEND_DIST_DIR={:?} COMPRESSED_PKG={:?}",
        dist_env, compressed_env
    );

    let dist_env = {
        let env_snapshot: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)> = command
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.map(|s| s.to_os_string())))
            .collect();
        env_snapshot
            .iter()
            .find(|(k, _)| k == "FRONTEND_DIST_DIR")
            .and_then(|(_, v)| v.clone())
    };
    // Force backend to serve uncompressed pkg and built frontend_dist for desktop bundle.
    command.env("COMPRESSED_PKG", "false");
    command.env("FRONTEND_DIST", "true");
    command.env("PORT", backend_port.to_string());
    command.env("REDIRECT_ENABLED", "false");
    if let Some(dist) = dist_env.clone() {
        command.env("MOON_ASSETS_DIR", dist);
    }

    let child = command.spawn().map_err(|e| {
        format!(
            "failed to spawn backend at {}: {}",
            backend_path.display(),
            e
        )
    })?;

    // Stash handle for clean shutdown
    let cell = BACKEND_CHILD.get_or_init(|| Mutex::new(None));
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(child);
    }

    Ok(())
}

fn backend_binary_name() -> &'static str {
    if cfg!(windows) {
        "backend.exe"
    } else {
        "backend"
    }
}

fn backend_candidates(
    resource_dir: Option<&Path>,
    current_exe: Option<&Path>,
    backend_name: &str,
) -> Vec<PathBuf> {
    let mut candidates = vec![];

    if let Some(dir) = resource_dir {
        // In AppImage/Tauri bundle resources land under:
        //   {resource_dir}/_up_/target/{profile}/{backend}
        candidates.push(dir.join("_up_").join("target/release").join(backend_name));
        candidates.push(dir.join("_up_").join("target/debug").join(backend_name));
        candidates.push(dir.join(backend_name));
    }

    if let Some(current_exe) = current_exe {
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join(format!("../{backend_name}")));
            candidates.push(parent.join(format!("../../{backend_name}")));
            candidates.push(
                parent
                    .join("../lib/NovyWave/_up_/target/release")
                    .join(backend_name),
            );
            candidates.push(
                parent
                    .join("../lib/NovyWave/_up_/target/debug")
                    .join(backend_name),
            );
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

fn startup_io_error(step: &str, detail: &str) -> std::io::Error {
    show_startup_error(step, detail);
    std::io::Error::other(format!("{step}: {detail}"))
}

fn show_startup_error(step: &str, detail: &str) {
    let message = format!("NovyWave failed to start.\n\n{step}\n\n{detail}");
    println!("{message}");
    show_startup_error_dialog(&message);
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinuxWebkitProfile {
    Auto,
    LegacyNvidia,
    Software,
}

#[cfg(target_os = "linux")]
impl LinuxWebkitProfile {
    fn from_env_value(value: Option<&str>) -> Self {
        match value.map(|value| value.trim().to_ascii_lowercase()) {
            Some(value) if value == "legacy-nvidia" || value == "legacy_nvidia" => {
                Self::LegacyNvidia
            }
            Some(value) if value == "software" => Self::Software,
            _ => Self::Auto,
        }
    }
}

#[cfg(target_os = "linux")]
impl std::fmt::Display for LinuxWebkitProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Auto => "auto",
            Self::LegacyNvidia => "legacy-nvidia",
            Self::Software => "software",
        };
        formatter.write_str(value)
    }
}

#[cfg(target_os = "linux")]
fn configure_linux_webkit_environment() -> LinuxWebkitProfile {
    let raw_profile = std::env::var("NOVYWAVE_WEBKIT_PROFILE").ok();
    let profile = LinuxWebkitProfile::from_env_value(raw_profile.as_deref());

    if let Some(raw_profile) = raw_profile.as_deref() {
        let normalized = raw_profile.trim().to_ascii_lowercase();
        if !matches!(
            normalized.as_str(),
            "auto" | "legacy-nvidia" | "legacy_nvidia" | "software"
        ) {
            println!("Unknown NOVYWAVE_WEBKIT_PROFILE='{raw_profile}', using auto");
        }
    }

    match profile {
        LinuxWebkitProfile::Auto => {}
        LinuxWebkitProfile::LegacyNvidia => {
            std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        LinuxWebkitProfile::Software => {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    println!("Linux WebKit profile: {profile}");
    profile
}

#[cfg(target_os = "linux")]
fn show_startup_error_dialog(message: &str) {
    let _ = Command::new("zenity")
        .args([
            "--error",
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
}

#[cfg(target_os = "macos")]
fn show_startup_error_dialog(message: &str) {
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
fn show_startup_error_dialog(message: &str) {
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

fn wait_for_backend_ready(port: u16) -> Result<(), String> {
    let deadline = Instant::now() + EMBEDDED_BACKEND_STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!(
        "embedded backend on 127.0.0.1:{port} did not accept connections within {:?}",
        EMBEDDED_BACKEND_STARTUP_TIMEOUT
    ))
}

fn updates_configured(app: &tauri::AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|cfg| {
            let endpoints_ok = cfg
                .get("endpoints")
                .and_then(|v| v.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);
            let pubkey_ok = cfg
                .get("pubkey")
                .and_then(|v| v.as_str())
                .map(|key| !key.trim().is_empty() && !key.contains("REPLACE_WITH_PUBLIC_KEY"))
                .unwrap_or(false);
            Some(endpoints_ok && pubkey_ok)
        })
        .unwrap_or(false)
}

async fn check_for_updates(app: tauri::AppHandle, capability: update_policy::UpdateCapability) {
    use tauri::Emitter;
    use tauri_plugin_updater::UpdaterExt;

    // Wait a bit before checking for updates to not slow down startup
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    println!(
                        "✨ Update available: {} -> {}",
                        update.current_version, update.version
                    );
                    let message = capability
                        .banner_message(
                            &update.current_version.to_string(),
                            &update.version.to_string(),
                        )
                        .unwrap_or_else(|| {
                            format!("v{} -> v{}", update.current_version, update.version)
                        });
                    // Emit event to frontend instead of auto-downloading
                    // Frontend will show a notification with a "Download and install" button.
                    if let Err(e) = app.emit(
                        "update_available",
                        serde_json::json!({
                            "current_version": update.current_version.to_string(),
                            "new_version": update.version.to_string(),
                            "message": message,
                            "action_label": capability.action_label().unwrap_or("Download and install")
                        }),
                    ) {
                        println!("❌ Failed to emit update_available event: {:?}", e);
                    }
                }
                Ok(None) => {
                    println!("✅ App is up to date");
                }
                Err(e) => {
                    // Update check failed - not critical, just log
                    println!("⚠️ Update check failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠️ Updater not available: {}", e);
        }
    }
}

async fn test_update_flow(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    println!("VERIFY_UPDATER: Checking for updates...");
    let updater = app
        .updater()
        .map_err(|e| format!("updater init: {:?}", e))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("check failed: {:?}", e))?
        .ok_or_else(|| "No update available".to_string())?;

    println!(
        "VERIFY_UPDATER: Update found {} -> {}",
        update.current_version, update.version
    );

    let mut total_downloaded: usize = 0;
    let mut last_pct: i32 = -1;

    let bytes = update
        .download(
            |chunk_len, total| {
                total_downloaded += chunk_len;
                if let Some(t) = total {
                    if t > 0 {
                        let pct = ((total_downloaded as f64 / t as f64) * 100.0) as i32;
                        if pct / 10 > last_pct / 10 {
                            println!("VERIFY_UPDATER: Download progress {}%", pct);
                            last_pct = pct;
                        }
                    }
                }
            },
            || {
                println!("VERIFY_UPDATER: Download finished");
            },
        )
        .await
        .map_err(|e| format!("download failed: {:?}", e))?;

    println!(
        "VERIFY_UPDATER: Downloaded {} bytes successfully",
        bytes.len()
    );

    if bytes.is_empty() {
        return Err("Downloaded 0 bytes".to_string());
    }

    println!("VERIFY_UPDATER: Update download verified successfully");
    Ok(())
}

fn stop_backend() {
    if let Some(cell) = BACKEND_CHILD.get() {
        if let Ok(mut slot) = cell.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_candidates_use_requested_backend_name() {
        let candidates = backend_candidates(
            Some(Path::new("/opt/NovyWave/resources")),
            Some(Path::new("/opt/NovyWave/novywave.exe")),
            "backend.exe",
        );

        assert!(candidates.contains(&PathBuf::from(
            "/opt/NovyWave/resources/_up_/target/release/backend.exe"
        )));
        assert!(candidates.contains(&PathBuf::from("/opt/NovyWave/resources/backend.exe")));
        assert!(candidates.contains(&PathBuf::from("/opt/NovyWave/../backend.exe")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_webkit_profile_parses_supported_values() {
        assert_eq!(
            LinuxWebkitProfile::from_env_value(Some("legacy-nvidia")),
            LinuxWebkitProfile::LegacyNvidia
        );
        assert_eq!(
            LinuxWebkitProfile::from_env_value(Some("legacy_nvidia")),
            LinuxWebkitProfile::LegacyNvidia
        );
        assert_eq!(
            LinuxWebkitProfile::from_env_value(Some("software")),
            LinuxWebkitProfile::Software
        );
        assert_eq!(
            LinuxWebkitProfile::from_env_value(Some("unknown")),
            LinuxWebkitProfile::Auto
        );
    }
}
