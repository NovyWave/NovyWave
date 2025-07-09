use moon::*;
use shared::{self, UpMsg, DownMsg, AppConfig, FileHierarchy, WaveformFile, FileFormat, ScopeData, generate_file_id};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs;

async fn frontend() -> Frontend {
    Frontend::new()
        .title("NovyWave ")
        .index_by_robots(false)
}

static PARSING_SESSIONS: Lazy<Arc<Mutex<HashMap<String, Arc<Mutex<f32>>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn up_msg_handler(req: UpMsgRequest<UpMsg>) {
    println!("Received UpMsg: {:?}", req.up_msg);
    let (session_id, cor_id) = (req.session_id, req.cor_id);
    
    match req.up_msg {
        UpMsg::LoadWaveformFile(file_path) => {
            load_waveform_file(file_path, session_id, cor_id).await;
        }
        UpMsg::GetParsingProgress(file_id) => {
            send_parsing_progress(file_id, session_id, cor_id).await;
        }
        UpMsg::LoadConfig => {
            load_config(session_id, cor_id).await;
        }
        UpMsg::SaveConfig(config) => {
            save_config(config, session_id, cor_id).await;
        }
        UpMsg::SaveTheme(theme) => {
            save_theme(theme, session_id, cor_id).await;
        }
    }
}

async fn load_waveform_file(file_path: String, session_id: SessionId, cor_id: CorId) {
    println!("Loading waveform file: {}", file_path);
    
    let path = Path::new(&file_path);
    if !path.exists() {
        let error_msg = format!("File not found: {}", file_path);
        send_down_msg(DownMsg::ParsingError { 
            file_id: file_path.clone(), 
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let file_id = generate_file_id(&file_path);
    let progress = Arc::new(Mutex::new(0.0));
    
    {
        let mut sessions = PARSING_SESSIONS.lock().unwrap();
        sessions.insert(file_id.clone(), progress.clone());
    }
    
    send_down_msg(DownMsg::ParsingStarted { 
        file_id: file_id.clone(), 
        filename: filename.clone() 
    }, session_id, cor_id).await;
    
    // Use wellen's automatic file format detection instead of extension-based detection
    parse_waveform_file(file_path, file_id, filename, progress, session_id, cor_id).await;
}

async fn parse_waveform_file(file_path: String, file_id: String, filename: String, 
                       progress: Arc<Mutex<f32>>, session_id: SessionId, cor_id: CorId) {
    
    let options = wellen::LoadOptions::default();
    
    match wellen::viewers::read_header_from_file(&file_path, &options) {
        Ok(header_result) => {
            {
                let mut p = progress.lock().unwrap();
                *p = 0.5; // Header parsed
            }
            send_progress_update(file_id.clone(), 0.5, session_id, cor_id).await;
            
            let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_id);
            let format = match header_result.file_format {
                wellen::FileFormat::Vcd => FileFormat::VCD,
                wellen::FileFormat::Fst => FileFormat::FST,
                wellen::FileFormat::Ghw => FileFormat::VCD, // Treat as VCD for now
                wellen::FileFormat::Unknown => FileFormat::VCD, // Fallback
            };
            
            let waveform_file = WaveformFile {
                id: file_id.clone(),
                filename,
                format,
                scopes,
            };
            
            let file_hierarchy = FileHierarchy {
                files: vec![waveform_file],
            };
            
            {
                let mut p = progress.lock().unwrap();
                *p = 1.0; // Complete
            }
            send_progress_update(file_id.clone(), 1.0, session_id, cor_id).await;
            
            send_down_msg(DownMsg::FileLoaded { 
                file_id: file_id.clone(), 
                hierarchy: file_hierarchy 
            }, session_id, cor_id).await;
            
            cleanup_parsing_session(&file_id);
        }
        Err(e) => {
            let error_msg = format!("Failed to parse waveform file: {}", e);
            send_down_msg(DownMsg::ParsingError { file_id, error: error_msg }, session_id, cor_id).await;
        }
    }
}


fn extract_scopes_from_hierarchy(hierarchy: &wellen::Hierarchy, file_id: &str) -> Vec<ScopeData> {
    hierarchy.scopes().map(|scope_ref| {
        extract_scope_data_with_file_id(hierarchy, scope_ref, file_id)
    }).collect()
}

fn extract_scope_data_with_file_id(hierarchy: &wellen::Hierarchy, scope_ref: wellen::ScopeRef, file_id: &str) -> ScopeData {
    let scope = &hierarchy[scope_ref];
    
    let variables: Vec<shared::Signal> = scope.vars(hierarchy).map(|var_ref| {
        let var = &hierarchy[var_ref];
        shared::Signal {
            id: format!("{}", var.signal_ref().index()),
            name: var.name(hierarchy).to_string(),
            signal_type: format!("{:?}", var.var_type()),
            width: match var.signal_encoding() {
                wellen::SignalEncoding::BitVector(width) => width.get(),
                wellen::SignalEncoding::Real => 1,
                wellen::SignalEncoding::String => 1,
            },
        }
    }).collect();
    
    let children: Vec<ScopeData> = scope.scopes(hierarchy).map(|child_scope_ref| {
        extract_scope_data_with_file_id(hierarchy, child_scope_ref, file_id)
    }).collect();
    
    ScopeData {
        id: format!("{}_scope_{}", file_id, scope_ref.index()),
        name: scope.name(hierarchy).to_string(),
        full_name: scope.full_name(hierarchy),
        children,
        variables,
    }
}

async fn send_parsing_progress(file_id: String, session_id: SessionId, cor_id: CorId) {
    let sessions = PARSING_SESSIONS.lock().unwrap();
    if let Some(progress) = sessions.get(&file_id) {
        let current_progress = {
            let p = progress.lock().unwrap();
            *p
        };
        send_progress_update(file_id, current_progress, session_id, cor_id).await;
    }
}

async fn send_progress_update(file_id: String, progress: f32, session_id: SessionId, cor_id: CorId) {
    send_down_msg(DownMsg::ParsingProgress { file_id, progress }, session_id, cor_id).await;
}

async fn send_down_msg(msg: DownMsg, session_id: SessionId, cor_id: CorId) {
    println!("Sending DownMsg: {:?}", msg);
    if let Some(session) = sessions::by_session_id().wait_for(session_id).await {
        session.send_down_msg(&msg, cor_id).await;
    } else {
        eprintln!("Cannot find session with id: {}", session_id);
    }
}


const CONFIG_FILE_PATH: &str = ".novywave";

async fn load_config(session_id: SessionId, cor_id: CorId) {
    println!("Loading config from {}", CONFIG_FILE_PATH);
    
    let config = match fs::read_to_string(CONFIG_FILE_PATH) {
        Ok(content) => {
            match toml::from_str::<AppConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    println!("Failed to parse config file: {}", e);
                    send_down_msg(DownMsg::ConfigError(format!("Failed to parse config: {}", e)), session_id, cor_id).await;
                    return;
                }
            }
        }
        Err(e) => {
            println!("Config file not found or unreadable: {}", e);
            // Create default config
            let default_config = AppConfig::default();
            
            // Try to save default config
            if let Err(save_err) = save_config_to_file(&default_config) {
                println!("Failed to create default config file: {}", save_err);
                send_down_msg(DownMsg::ConfigError(format!("Failed to create default config: {}", save_err)), session_id, cor_id).await;
                return;
            }
            
            default_config
        }
    };
    
    send_down_msg(DownMsg::ConfigLoaded(config), session_id, cor_id).await;
}

async fn save_config(config: AppConfig, session_id: SessionId, cor_id: CorId) {
    println!("Saving config to {}", CONFIG_FILE_PATH);
    
    match save_config_to_file(&config) {
        Ok(()) => {
            send_down_msg(DownMsg::ConfigSaved, session_id, cor_id).await;
        }
        Err(e) => {
            println!("Failed to save config: {}", e);
            send_down_msg(DownMsg::ConfigError(format!("Failed to save config: {}", e)), session_id, cor_id).await;
        }
    }
}

async fn save_theme(theme: String, session_id: SessionId, cor_id: CorId) {
    println!("Saving theme: {}", theme);
    
    // Load current config
    let mut config = match fs::read_to_string(CONFIG_FILE_PATH) {
        Ok(content) => {
            match toml::from_str::<AppConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    println!("Failed to parse config file: {}", e);
                    send_down_msg(DownMsg::ConfigError(format!("Failed to parse config: {}", e)), session_id, cor_id).await;
                    return;
                }
            }
        }
        Err(_) => {
            // Create default config if file doesn't exist
            AppConfig::default()
        }
    };
    
    // Update theme
    config.ui.theme = theme;
    
    // Save updated config
    match save_config_to_file(&config) {
        Ok(()) => {
            send_down_msg(DownMsg::ThemeSaved, session_id, cor_id).await;
        }
        Err(e) => {
            println!("Failed to save theme: {}", e);
            send_down_msg(DownMsg::ConfigError(format!("Failed to save theme: {}", e)), session_id, cor_id).await;
        }
    }
}

fn save_config_to_file(config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let toml_content = toml::to_string_pretty(config)?;
    
    // Add header comment
    let content_with_header = format!(
        "# NovyWave User Configuration\n\
         # This file stores your application preferences and workspace state\n\
         \n\
         {}", 
        toml_content
    );
    
    fs::write(CONFIG_FILE_PATH, content_with_header)?;
    println!("Config saved successfully");
    Ok(())
}

fn cleanup_parsing_session(file_id: &str) {
    let mut sessions = PARSING_SESSIONS.lock().unwrap();
    sessions.remove(file_id);
}

#[moon::main]
async fn main() -> std::io::Result<()> {
    start(frontend, up_msg_handler, |_| {}).await
}
