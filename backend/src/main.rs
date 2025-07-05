use moon::*;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),  // Absolute file path
    GetParsingProgress(String), // File ID
    LoadConfig,
    SaveConfig(AppConfig),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DownMsg {
    ParsingStarted { file_id: String, filename: String },
    ParsingProgress { file_id: String, progress: f32 },
    FileLoaded { file_id: String, hierarchy: FileHierarchy },
    ParsingError { file_id: String, error: String },
    ConfigLoaded(AppConfig),
    ConfigSaved,
    ConfigError(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileHierarchy {
    pub files: Vec<WaveformFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WaveformFile {
    pub id: String,
    pub filename: String,
    pub format: FileFormat,
    pub scopes: Vec<ScopeData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileFormat {
    VCD,
    FST,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScopeData {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub children: Vec<ScopeData>,
    pub variables: Vec<Signal>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub signal_type: String,
    pub width: u32,
}

// Configuration structures
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    pub app: AppSection,
    pub ui: UiSection,
    pub files: FilesSection,
    pub workspace: WorkspaceSection,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSection {
    pub version: String,
    pub auto_load_previous_files: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UiSection {
    pub theme: String, // "dark" or "light"
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FilesSection {
    pub opened_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceSection {
    pub dock_to_bottom: bool,
    pub docked_to_bottom: DockedToBottomLayout,
    pub docked_to_right: DockedToRightLayout,
    pub scope_selection: HashMap<String, String>,
    pub expanded_scopes: HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DockedToBottomLayout {
    pub main_area_height: u32,
    pub files_panel_width: u32,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DockedToRightLayout {
    pub files_panel_height: u32,
    pub files_panel_width: u32,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            auto_load_previous_files: true,
        }
    }
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
        }
    }
}

async fn frontend() -> Frontend {
    Frontend::new()
        .title("NovyWave")
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
            
            let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy);
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


fn extract_scopes_from_hierarchy(hierarchy: &wellen::Hierarchy) -> Vec<ScopeData> {
    hierarchy.scopes().map(|scope_ref| {
        extract_scope_data(hierarchy, scope_ref)
    }).collect()
}

fn extract_scope_data(hierarchy: &wellen::Hierarchy, scope_ref: wellen::ScopeRef) -> ScopeData {
    let scope = &hierarchy[scope_ref];
    
    let variables: Vec<Signal> = scope.vars(hierarchy).map(|var_ref| {
        let var = &hierarchy[var_ref];
        Signal {
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
        extract_scope_data(hierarchy, child_scope_ref)
    }).collect();
    
    ScopeData {
        id: format!("scope_{}", scope_ref.index()),
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

fn generate_file_id(file_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    format!("file_{:x}", hasher.finish())
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
