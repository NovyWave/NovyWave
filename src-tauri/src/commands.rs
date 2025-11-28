//! Tauri command handlers for NovyWave desktop mode
//!
//! Provides direct file system access and waveform processing
//! without needing HTTP/SSE communication.

use serde_json;
use shared::{AppConfig, GlobalSection, SignalTransitionQuery, SignalValueQuery};
use std::path::PathBuf;
use tauri::Emitter;

/// Per-project config filename (hidden dotfile in workspace)
const PER_PROJECT_CONFIG_FILENAME: &str = ".novywave";

/// Global workspace history filename (same as browser mode)
const GLOBAL_HISTORY_FILENAME: &str = ".novywave_global";

/// Determine the config path to use
/// Resolution order:
/// 1. Per-project: {cwd}/.novywave (if exists)
/// 2. Global: {platform_config_dir}/novywave/config.toml
fn get_config_path() -> Result<(PathBuf, bool), String> {
    // Check for per-project config in current working directory
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let per_project_path = cwd.join(PER_PROJECT_CONFIG_FILENAME);

    if per_project_path.exists() {
        return Ok((per_project_path, true)); // (path, is_per_project)
    }

    // Fall back to global config
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?;
    let global_path = config_dir.join("novywave").join("config.toml");

    Ok((global_path, false))
}

/// Get global workspace history path
/// Always at: {platform_config_dir}/novywave/.novywave_global
fn get_global_history_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir().ok_or("Could not find config directory")?;
    Ok(config_dir.join("novywave").join(GLOBAL_HISTORY_FILENAME))
}

/// Load application configuration from file system
/// Checks for per-project .novywave first, then falls back to global config
#[tauri::command]
pub async fn load_config() -> Result<String, String> {
    let (config_path, is_per_project) = get_config_path()?;

    // Log which config is being used
    if is_per_project {
        println!("📁 Loading per-project config from: {}", config_path.display());
    } else {
        println!("🌐 Loading global config from: {}", config_path.display());
    }

    // Create config directory if it doesn't exist (only for global config)
    if !is_per_project {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
    }

    // Try to read existing config file
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            // Parse TOML and convert to AppConfig
            let config: AppConfig = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse config file: {}", e))?;

            // Return as JSON string for consistent serialization
            serde_json::to_string(&config).map_err(|e| format!("Failed to serialize config: {}", e))
        }
        Err(_) => {
            // Config file doesn't exist, return default config
            let default_config = AppConfig::default();
            serde_json::to_string(&default_config)
                .map_err(|e| format!("Failed to serialize default config: {}", e))
        }
    }
}

/// Save application configuration to file system
/// Saves to per-project .novywave if it exists, otherwise to global config
#[tauri::command]
pub async fn save_config(config_json: String) -> Result<(), String> {
    // Parse JSON back to AppConfig
    let config: AppConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    // Determine where to save (same logic as load)
    let (config_path, is_per_project) = get_config_path()?;

    // Log which config is being saved to
    if is_per_project {
        println!("💾 Saving to per-project config: {}", config_path.display());
    } else {
        println!("💾 Saving to global config: {}", config_path.display());
    }

    // Create config directory if it doesn't exist (only for global config)
    if !is_per_project {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
    }

    // Serialize to TOML and write to file
    let toml_content = toml::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config to TOML: {}", e))?;

    std::fs::write(&config_path, toml_content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Wrapper for GlobalSection to match backend's TOML structure
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct GlobalConfigFile {
    #[serde(default)]
    global: GlobalSection,
}

/// Load workspace history from global config
/// Path: {platform_config_dir}/novywave/.novywave_global
#[tauri::command]
pub async fn load_workspace_history() -> Result<String, String> {
    let history_path = get_global_history_path()?;

    println!(
        "📂 Loading workspace history from: {}",
        history_path.display()
    );

    // Create config directory if it doesn't exist
    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Try to read existing history file
    match std::fs::read_to_string(&history_path) {
        Ok(content) => {
            // Parse TOML and extract GlobalSection
            let global_file: GlobalConfigFile = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse workspace history: {}", e))?;

            // Return as JSON string for consistent serialization
            serde_json::to_string(&global_file.global)
                .map_err(|e| format!("Failed to serialize workspace history: {}", e))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist, return default
            let default_section = GlobalSection::default();
            serde_json::to_string(&default_section)
                .map_err(|e| format!("Failed to serialize default workspace history: {}", e))
        }
        Err(e) => Err(format!("Failed to read workspace history: {}", e)),
    }
}

/// Save workspace history to global config
/// Path: {platform_config_dir}/novywave/.novywave_global
#[tauri::command]
pub async fn save_workspace_history(history_json: String) -> Result<(), String> {
    // Parse JSON back to GlobalSection
    let global_section: GlobalSection = serde_json::from_str(&history_json)
        .map_err(|e| format!("Failed to parse workspace history JSON: {}", e))?;

    let history_path = get_global_history_path()?;

    println!(
        "💾 Saving workspace history to: {}",
        history_path.display()
    );

    // Create config directory if it doesn't exist
    if let Some(parent) = history_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    // Wrap in GlobalConfigFile for consistent TOML structure
    let global_file = GlobalConfigFile {
        global: global_section,
    };

    // Serialize to TOML and write to file
    let toml_content = toml::to_string_pretty(&global_file)
        .map_err(|e| format!("Failed to serialize workspace history to TOML: {}", e))?;

    std::fs::write(&history_path, toml_content)
        .map_err(|e| format!("Failed to write workspace history file: {}", e))?;

    Ok(())
}

/// Load waveform file with progress updates
#[tauri::command]
pub async fn load_waveform_file(path: String, window: tauri::Window) -> Result<(), String> {
    let file_path = PathBuf::from(&path);

    // Validate file exists
    if !file_path.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    // Generate unique file ID for tracking
    let file_id = uuid::Uuid::new_v4().to_string();
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Emit parsing started event
    window
        .emit(
            "parsing_started",
            serde_json::json!({
                "file_id": file_id,
                "filename": filename
            }),
        )
        .map_err(|e| format!("Failed to emit parsing_started: {}", e))?;

    // Waveform parsing simulation with progress events
    // Real parsing would integrate with wellen library here
    for progress in (0..=100).step_by(10) {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        window
            .emit(
                "parsing_progress",
                serde_json::json!({
                    "file_id": file_id,
                    "progress": progress as f32 / 100.0
                }),
            )
            .map_err(|e| format!("Failed to emit parsing_progress: {}", e))?;
    }

    // Emit file loaded event with mock hierarchy
    let mock_hierarchy = shared::FileHierarchy {
        files: vec![shared::WaveformFile {
            id: file_id.clone(),
            filename: filename.clone(),
            format: shared::FileFormat::VCD,
            scopes: vec![shared::ScopeData {
                id: "top".to_string(),
                name: "top".to_string(),
                full_name: "top".to_string(),
                variables: vec![],
                children: vec![],
            }],
            min_time_ns: Some(0),
            max_time_ns: Some(1_000_000_000),
        }],
    };

    window
        .emit(
            "file_loaded",
            serde_json::json!({
                "file_id": file_id,
                "hierarchy": mock_hierarchy
            }),
        )
        .map_err(|e| format!("Failed to emit file_loaded: {}", e))?;

    Ok(())
}

/// Browse directory contents
#[tauri::command]
pub async fn browse_directory(path: String) -> Result<(), String> {
    let _dir_path = PathBuf::from(&path);

    // Directory browsing placeholder - would scan filesystem
    // and emit directory_contents event with file listings
    Ok(())
}

/// Browse multiple directories
#[tauri::command]
pub async fn browse_directories(_paths: Vec<String>) -> Result<(), String> {
    // Batch directory browsing placeholder - would process multiple paths
    // and aggregate file listings for efficient bulk operations
    Ok(())
}

/// Query signal values at specific times
#[tauri::command]
pub async fn query_signal_values(
    _file_path: String,
    _queries: Vec<SignalValueQuery>,
) -> Result<(), String> {
    // Signal value query placeholder - would extract values from parsed waveforms
    // at specific time points using wellen library integration
    Ok(())
}

/// Query signal transitions over time ranges
#[tauri::command]
pub async fn query_signal_transitions(
    _file_path: String,
    _signal_queries: Vec<SignalTransitionQuery>,
    _time_range: (f64, f64),
) -> Result<(), String> {
    // Signal transition query placeholder - would extract transition data
    // over time ranges from parsed waveform databases
    Ok(())
}

/// Get parsing progress for a file
#[tauri::command]
pub async fn get_parsing_progress(_file_id: String) -> Result<(), String> {
    // Parsing progress tracking placeholder - would maintain progress state
    // for active file parsing operations
    Ok(())
}
