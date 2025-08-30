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
                                    // Forward to initialization handler (same as web platform)
                                    crate::config::forward_config_load_response(config);
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
}
