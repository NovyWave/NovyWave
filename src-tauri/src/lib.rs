
mod commands;


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // https://github.com/tauri-apps/tauri/issues/8462
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::load_config,
            commands::save_config,
            commands::load_waveform_file,
            commands::browse_directory,
            commands::browse_directories,
            commands::query_signal_values,
            commands::query_signal_transitions,
            commands::get_parsing_progress
        ])
        .setup(|_app| {
            println!("=== Tauri app setup completed ===");
            Ok(())
        })
        .on_window_event(|_window, _event| {
            // No backend cleanup needed - using external MoonZoon dev server
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
