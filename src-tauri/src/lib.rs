mod commands;

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

            // Check for updates in background
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                check_for_updates(handle).await;
            });

            Ok(())
        })
        .on_window_event(|_window, _event| {
            // No backend cleanup needed - using external MoonZoon dev server
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
