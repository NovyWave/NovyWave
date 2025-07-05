use moon::*;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),  // Absolute file path
    GetParsingProgress(String), // File ID
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DownMsg {
    ParsingStarted { file_id: String, filename: String },
    ParsingProgress { file_id: String, progress: f32 },
    FileLoaded { file_id: String, hierarchy: FileHierarchy },
    ParsingError { file_id: String, error: String },
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
    pub signals: Vec<Signal>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FileFormat {
    VCD,
    FST,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub signal_type: String,
    pub width: u32,
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
    
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match extension.as_str() {
        "vcd" => parse_vcd_file(file_path, file_id, filename, progress, session_id, cor_id).await,
        "fst" => parse_fst_file(file_path, file_id, filename, progress, session_id, cor_id).await,
        _ => {
            let error_msg = format!("Unsupported file format: {}", extension);
            send_down_msg(DownMsg::ParsingError { 
                file_id, 
                error: error_msg 
            }, session_id, cor_id).await;
        }
    }
}

async fn parse_vcd_file(file_path: String, file_id: String, filename: String, 
                       progress: Arc<Mutex<f32>>, session_id: SessionId, cor_id: CorId) {
    
    let options = wellen::LoadOptions::default();
    
    match wellen::viewers::read_header(&file_path, &options) {
        Ok(header_result) => {
            {
                let mut p = progress.lock().unwrap();
                *p = 0.5; // Header parsed
            }
            send_progress_update(file_id.clone(), 0.5, session_id, cor_id).await;
            
            let signals = extract_signals_from_hierarchy(&header_result.hierarchy);
            let format = FileFormat::VCD;
            
            let waveform_file = WaveformFile {
                id: file_id.clone(),
                filename,
                format,
                signals,
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
            let error_msg = format!("Failed to parse VCD file: {}", e);
            send_down_msg(DownMsg::ParsingError { file_id, error: error_msg }, session_id, cor_id).await;
        }
    }
}

async fn parse_fst_file(file_path: String, file_id: String, filename: String, 
                       progress: Arc<Mutex<f32>>, session_id: SessionId, cor_id: CorId) {
    
    let options = wellen::LoadOptions::default();
    
    match wellen::viewers::read_header(&file_path, &options) {
        Ok(header_result) => {
            {
                let mut p = progress.lock().unwrap();
                *p = 0.5; // Header parsed
            }
            send_progress_update(file_id.clone(), 0.5, session_id, cor_id).await;
            
            let signals = extract_signals_from_hierarchy(&header_result.hierarchy);
            let format = FileFormat::FST;
            
            let waveform_file = WaveformFile {
                id: file_id.clone(),
                filename,
                format,
                signals,
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
            let error_msg = format!("Failed to parse FST file: {}", e);
            send_down_msg(DownMsg::ParsingError { file_id, error: error_msg }, session_id, cor_id).await;
        }
    }
}

fn extract_signals_from_hierarchy(hierarchy: &wellen::Hierarchy) -> Vec<Signal> {
    let mut signals = Vec::new();
    
    for var in hierarchy.iter_vars() {
        signals.push(Signal {
            id: format!("{}", var.signal_ref().index()),
            name: var.name(hierarchy).to_string(),
            signal_type: format!("{:?}", var.var_type()),
            width: match var.signal_tpe() {
                wellen::SignalType::BitVector(width, _) => width.get(),
                wellen::SignalType::Real => 1,
                wellen::SignalType::String => 1,
            },
        });
    }
    
    signals
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

fn cleanup_parsing_session(file_id: &str) {
    let mut sessions = PARSING_SESSIONS.lock().unwrap();
    sessions.remove(file_id);
}

#[moon::main]
async fn main() -> std::io::Result<()> {
    start(frontend, up_msg_handler, |_| {}).await
}
