mod commands;

use std::{
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command},
    sync::{Mutex, OnceLock},
};

use tauri::{Manager, WindowEvent};

// Keep handle to embedded backend so we can terminate it when the app closes.
static BACKEND_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // https://github.com/tauri-apps/tauri/issues/8462
    // WebKit flags no longer needed - Tauri works fine without them

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
            commands::get_parsing_progress
        ])
        .setup(|app| {
            println!("=== Tauri app setup completed ===");

            if let Err(e) = spawn_backend_if_needed(app) {
                println!("⚠️ Failed to spawn embedded backend: {}", e);
            }

            // Create the main window pointing directly at the embedded backend.
            let target: tauri::Url = "http://127.0.0.1:8080".parse().unwrap();
            tauri::WebviewWindowBuilder::new(
                app,
                "main".to_string(),
                tauri::WebviewUrl::External(target),
            )
            .title("NovyWave")
            .inner_size(1200.0, 800.0)
            .devtools(true)
            .build()
            .map_err(|e| {
                println!("⚠️ Failed to create main window: {e}");
                e
            })?;

            // Check for updates in background only when configured with a real endpoint & pubkey
            let handle = app.handle();
            if updates_configured(&handle) {
                let handle_for_task = handle.clone();
                tauri::async_runtime::spawn(async move {
                    check_for_updates(handle_for_task).await;
                });
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

fn spawn_backend_if_needed(app: &tauri::App) -> Result<(), String> {
    // If port already in use, assume user/dev server is running; skip spawn
    if TcpListener::bind("127.0.0.1:8080").is_err() {
        println!("Backend already running on 127.0.0.1:8080, skipping embedded spawn.");
        return Ok(());
    }

    // Resolve backend binary from resources or fallback to workspace target
    let resolver = app.path();
    let mut candidates: Vec<PathBuf> = vec![];

    if let Ok(dir) = resolver.resource_dir() {
        // In AppImage/Tauri bundle resources land under:
        //   {resource_dir}/_up_/target/{profile}/backend
        candidates.push(dir.join("_up_/target/release/backend"));
        candidates.push(dir.join("_up_/target/debug/backend"));
        // Older layout we tried first
        candidates.push(dir.join("backend"));
    }

    // Fallbacks for dev runs (not packaged)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            // target/{profile}/novywave -> add sibling backend
            candidates.push(parent.join("../backend"));
            candidates.push(parent.join("../../backend"));
            // AppImage style: ../lib/NovyWave/_up_/target/{profile}/backend
            candidates.push(parent.join("../lib/NovyWave/_up_/target/release/backend"));
            candidates.push(parent.join("../lib/NovyWave/_up_/target/debug/backend"));
        }
    }

    let backend_path = candidates
        .into_iter()
        .find(|p| p.exists())
        .ok_or("backend binary not found in resources or target paths".to_string())?;

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
    if let Some(dist) = dist_env.clone() {
        command.env("MOON_ASSETS_DIR", dist);
    }

    let child = command
        .spawn()
        .map_err(|e| format!("failed to spawn backend: {}", e))?;

    // Stash handle for clean shutdown
    let cell = BACKEND_CHILD.get_or_init(|| Mutex::new(None));
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(child);
    }

    Ok(())
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

async fn check_for_updates(app: tauri::AppHandle) {
    use tauri_plugin_updater::UpdaterExt;

    // Wait a bit before checking for updates to not slow down startup
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(Some(update)) => {
                    println!(
                        "Update available: {} -> {}",
                        update.current_version, update.version
                    );
                    // For now, just log. In the future, prompt user via frontend
                    // update.download_and_install(|_, _| {}, || {}).await.ok();
                }
                Ok(None) => {
                    println!("App is up to date");
                }
                Err(e) => {
                    // Update check failed - not critical, just log
                    println!("Update check failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Updater not available: {}", e);
        }
    }
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
