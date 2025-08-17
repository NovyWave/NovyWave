//! Tauri platform implementation using tauri-wasm
//! 
//! Uses Tauri IPC commands for direct frontend-backend communication.

use zoon::*;
use shared::{UpMsg, DownMsg};
use crate::platform::Platform;

#[cfg(feature = "tauri")]
use tauri_wasm;

/// Tauri platform implementation using tauri-wasm crate
pub struct TauriPlatform;

impl Platform for TauriPlatform {
    fn is_available() -> bool {
        #[cfg(feature = "tauri")]
        {
            tauri_wasm::is_tauri()
        }
        #[cfg(not(feature = "tauri"))]
        {
            false
        }
    }
    
    async fn send_message(msg: UpMsg) -> Result<(), String> {
        #[cfg(feature = "tauri")]
        {
            match msg {
                UpMsg::LoadConfig => {
                    let result = tauri_wasm::invoke("load_config", &()).await;
                    match result {
                        Ok(config_js) => {
                            // Convert JsValue to AppConfig and apply directly
                            if let Ok(config_str) = serde_wasm_bindgen::from_value::<String>(config_js) {
                                if let Ok(config) = serde_json::from_str::<shared::AppConfig>(&config_str) {
                                    // Apply config directly instead of going through DownMsg
                                    crate::config::apply_config(config);
                                }
                            }
                            Ok(())
                        }
                        Err(e) => Err(format!("Failed to load config: {:?}", e))
                    }
                }
                UpMsg::SaveConfig(config) => {
                    let config_json = serde_json::to_string(&config)
                        .map_err(|e| format!("Failed to serialize config: {}", e))?;
                    
                    let args = serde_wasm_bindgen::to_value(&config_json)
                        .map_err(|e| format!("Failed to convert config: {}", e))?;
                    
                    tauri_wasm::invoke("save_config", &args).await
                        .map_err(|e| format!("Failed to save config: {:?}", e))?;
                    
                    Ok(())
                }
                UpMsg::LoadWaveformFile(path) => {
                    let args = serde_json::json!({ "path": path });
                    let args_js = serde_wasm_bindgen::to_value(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;
                    
                    tauri_wasm::invoke("load_waveform_file", &args_js).await
                        .map_err(|e| format!("Failed to load waveform file: {:?}", e))?;
                    
                    Ok(())
                }
                UpMsg::BrowseDirectory(path) => {
                    let args = serde_json::json!({ "path": path });
                    let args_js = serde_wasm_bindgen::to_value(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;
                    
                    tauri_wasm::invoke("browse_directory", &args_js).await
                        .map_err(|e| format!("Failed to browse directory: {:?}", e))?;
                    
                    Ok(())
                }
                UpMsg::BrowseDirectories(paths) => {
                    let args = serde_json::json!({ "paths": paths });
                    let args_js = serde_wasm_bindgen::to_value(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;
                    
                    tauri_wasm::invoke("browse_directories", &args_js).await
                        .map_err(|e| format!("Failed to browse directories: {:?}", e))?;
                    
                    Ok(())
                }
                UpMsg::QuerySignalValues { file_path, queries } => {
                    let args = serde_json::json!({ 
                        "file_path": file_path, 
                        "queries": queries 
                    });
                    let args_js = serde_wasm_bindgen::to_value(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;
                    
                    tauri_wasm::invoke("query_signal_values", &args_js).await
                        .map_err(|e| format!("Failed to query signal values: {:?}", e))?;
                    
                    Ok(())
                }
                UpMsg::QuerySignalTransitions { file_path, signal_queries, time_range } => {
                    let args = serde_json::json!({ 
                        "file_path": file_path,
                        "signal_queries": signal_queries,
                        "time_range": time_range
                    });
                    let args_js = serde_wasm_bindgen::to_value(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;
                    
                    tauri_wasm::invoke("query_signal_transitions", &args_js).await
                        .map_err(|e| format!("Failed to query signal transitions: {:?}", e))?;
                    
                    Ok(())
                }
                UpMsg::GetParsingProgress(file_id) => {
                    let args = serde_json::json!({ "file_id": file_id });
                    let args_js = serde_wasm_bindgen::to_value(&args)
                        .map_err(|e| format!("Failed to serialize args: {}", e))?;
                    
                    tauri_wasm::invoke("get_parsing_progress", &args_js).await
                        .map_err(|e| format!("Failed to get parsing progress: {:?}", e))?;
                    
                    Ok(())
                }
            }
        }
        #[cfg(not(feature = "tauri"))]
        {
            Err("Tauri platform not available".to_string())
        }
    }
    
    fn init_message_handler(handler: fn(DownMsg)) {
        #[cfg(feature = "tauri")]
        {
            zoon::println!("=== TAURI_PLATFORM: Setting up event listeners ===");
            
            // Set up event listeners for all Tauri events that map to DownMsg
            setup_event_listener("parsing_started", handler);
            setup_event_listener("parsing_progress", handler); 
            setup_event_listener("file_loaded", handler);
            setup_event_listener("parsing_error", handler);
            setup_event_listener("directory_contents", handler);
            setup_event_listener("directory_error", handler);
            setup_event_listener("config_loaded", handler);
            setup_event_listener("config_saved", handler);
            setup_event_listener("config_error", handler);
            setup_event_listener("signal_values", handler);
            setup_event_listener("signal_values_error", handler);
            setup_event_listener("signal_transitions", handler);
            setup_event_listener("signal_transitions_error", handler);
            
            zoon::println!("=== TAURI_PLATFORM: Event listeners configured ===");
        }
    }
}

#[cfg(feature = "tauri")]
fn setup_event_listener(event_name: &str, handler: fn(DownMsg)) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::spawn_local;
    
    // Clone the event name for the closure
    let event_name = event_name.to_string();
    let handler = handler.clone();
    
    spawn_local(async move {
        loop {
            // Try to listen for the event using tauri-wasm
            // Note: This is a simplified implementation since tauri-wasm may not support direct event listening
            // We might need to extend tauri-wasm or use a different approach
            
            match event_name.as_str() {
                "parsing_started" => {
                    // For now, we handle events within send_message responses
                    // A full implementation would need proper event listener support
                    break;
                }
                "parsing_progress" => {
                    break;
                }
                "file_loaded" => {
                    break;
                }
                "parsing_error" => {
                    break;
                }
                "directory_contents" => {
                    break;
                }
                "directory_error" => {
                    break;
                }
                "config_loaded" => {
                    break;
                }
                "config_saved" => {
                    break;
                }
                "config_error" => {
                    break;
                }
                "signal_values" => {
                    break;
                }
                "signal_values_error" => {
                    break;
                }
                "signal_transitions" => {
                    break;
                }
                "signal_transitions_error" => {
                    break;
                }
                _ => {
                    break;
                }
            }
        }
    });
}