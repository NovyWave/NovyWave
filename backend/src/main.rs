use moon::*;
use shared::{self, UpMsg, DownMsg, AppConfig, FileHierarchy, WaveformFile, FileFormat, ScopeData, generate_file_id, FileSystemItem};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs;
use jwalk::WalkDir;

async fn frontend() -> Frontend {
    Frontend::new()
        .title("NovyWave ")
        .index_by_robots(false)
}

static PARSING_SESSIONS: Lazy<Arc<Mutex<HashMap<String, Arc<Mutex<f32>>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn up_msg_handler(req: UpMsgRequest<UpMsg>) {
    // println!("Received UpMsg: {:?}", req.up_msg); // Disabled - too verbose
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
        UpMsg::BrowseDirectory(dir_path) => {
            browse_directory(dir_path, session_id, cor_id).await;
        }
        UpMsg::BrowseDirectories(dir_paths) => {
            browse_directories_batch(dir_paths, session_id, cor_id).await;
        }
    }
}

async fn load_waveform_file(file_path: String, session_id: SessionId, cor_id: CorId) {
    
    let path = Path::new(&file_path);
    if !path.exists() {
        let error_msg = format!("File not found: {}", file_path);
        let file_id = generate_file_id(&file_path);
        send_down_msg(DownMsg::ParsingError { 
            file_id, 
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
            let error_msg = format!("Failed to parse waveform file '{}': {}", file_path, e);
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
    
    let mut variables: Vec<shared::Signal> = scope.vars(hierarchy).map(|var_ref| {
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
    variables.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    let mut children: Vec<ScopeData> = scope.scopes(hierarchy).map(|child_scope_ref| {
        extract_scope_data_with_file_id(hierarchy, child_scope_ref, file_id)
    }).collect();
    children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
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
    // println!("Sending DownMsg: {:?}", msg); // Disabled - too verbose for large file data
    if let Some(session) = sessions::by_session_id().wait_for(session_id).await {
        session.send_down_msg(&msg, cor_id).await;
    } else {
        eprintln!("Cannot find session with id: {}", session_id);
    }
}


const CONFIG_FILE_PATH: &str = ".novywave";

async fn load_config(session_id: SessionId, cor_id: CorId) {
    // Loading config from filesystem
    
    let config = match fs::read_to_string(CONFIG_FILE_PATH) {
        Ok(content) => {
            match toml::from_str::<AppConfig>(&content) {
                Ok(mut config) => {
                    // Enable migration system - validate and fix config after loading
                    let migration_warnings = config.validate_and_fix();
                    
                    // Log migration warnings if any
                    if !migration_warnings.is_empty() {
                        println!("Config migration applied:");
                        for warning in &migration_warnings {
                            println!("  - {}", warning);
                        }
                        
                        // Save migrated config to persist changes
                        if let Err(save_err) = save_config_to_file(&config) {
                            println!("Warning: Failed to save migrated config: {}", save_err);
                            // Don't fail loading, just warn - the migration is still applied in memory
                        } else {
                            println!("Migrated config saved successfully");
                        }
                    }
                    
                    config
                },
                Err(e) => {
                    println!("Failed to parse config file: {}", e);
                    send_down_msg(DownMsg::ConfigError(format!("Failed to parse config: {}", e)), session_id, cor_id).await;
                    return;
                }
            }
        }
        Err(e) => {
            println!("Config file not found or unreadable: {}", e);
            // Create default config with validation already applied
            let mut default_config = AppConfig::default();
            let _warnings = default_config.validate_and_fix(); // Ensure defaults are validated too
            
            // Try to save default config
            if let Err(save_err) = save_config_to_file(&default_config) {
                println!("Failed to create default config file: {}", save_err);
                send_down_msg(DownMsg::ConfigError(format!("Failed to create default config: {}", save_err)), session_id, cor_id).await;
                return;
            }
            
            default_config
        }
    };
    
    // PARALLEL PRELOADING: Start preloading expanded directories in background for instant file dialog
    let expanded_dirs = config.workspace.load_files_expanded_directories.clone();
    if !expanded_dirs.is_empty() {
        // Starting parallel preloading of expanded directories
        
        // Spawn background task to preload directories - precompute for instant access
        tokio::spawn(async move {
            let mut preload_tasks = Vec::new();
            
            // Create async task for each expanded directory
            for dir_path in expanded_dirs {
                let path = dir_path.clone();
                
                preload_tasks.push(tokio::spawn(async move {
                    // Preloading directory: {path}
                    let path_obj = Path::new(&path);
                    
                    // Precompute directory contents for instant access
                    if path_obj.exists() && path_obj.is_dir() {
                        match scan_directory_async(path_obj).await {
                            Ok(_items) => {
                                // Cache computed for future instant access
                                // Successfully preloaded directory
                            }
                            Err(_e) => {
                                // Failed to preload directory: {path}, error: {e}
                            }
                        }
                    }
                }));
            }
            
            // Wait for all preloading tasks to complete
            for task in preload_tasks {
                let _ = task.await; // Ignore individual task errors
            }
            
            // Finished parallel preloading of expanded directories
        });
    }

    send_down_msg(DownMsg::ConfigLoaded(config), session_id, cor_id).await;
}

async fn save_config(config: AppConfig, session_id: SessionId, cor_id: CorId) {
    // Config saving (debug logs removed to reduce console noise)
    
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
    // Config saved successfully (log removed to reduce console noise)
    Ok(())
}

fn cleanup_parsing_session(file_id: &str) {
    let mut sessions = PARSING_SESSIONS.lock().unwrap();
    sessions.remove(file_id);
}


async fn browse_directory(dir_path: String, session_id: SessionId, cor_id: CorId) {
    
    // Handle Windows multi-root scenario - enumerate drives when browsing "/"
    #[cfg(windows)]
    if dir_path == "/" {
        let mut drive_items = Vec::new();
        
        // Enumerate available drives (A: through Z:)
        for drive_letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", drive_letter as char);
            let drive_root = Path::new(&drive_path);
            
            // Check if drive exists and is accessible
            if drive_root.exists() {
                drive_items.push(FileSystemItem {
                    name: format!("{}:", drive_letter as char),
                    path: drive_path.clone(),
                    is_directory: true,
                    file_size: None,
                    is_waveform_file: false,
                    file_extension: None,
                    has_expandable_content: true,
                });
            }
        }
        
        // Sort drives alphabetically
        drive_items.sort_by(|a, b| a.name.cmp(&b.name));
        
        send_down_msg(DownMsg::DirectoryContents { 
            path: "/".to_string(), 
            items: drive_items 
        }, session_id, cor_id).await;
        return;
    }
    
    // Expand ~ to home directory
    let expanded_path = if dir_path == "~" {
        match dirs::home_dir() {
            Some(home) => home.to_string_lossy().to_string(),
            None => {
                let error_msg = "Unable to determine home directory".to_string();
                send_down_msg(DownMsg::DirectoryError { 
                    path: dir_path, 
                    error: error_msg 
                }, session_id, cor_id).await;
                return;
            }
        }
    } else if dir_path.starts_with("~/") {
        match dirs::home_dir() {
            Some(home) => {
                let relative_path = &dir_path[2..]; // Remove "~/"
                home.join(relative_path).to_string_lossy().to_string()
            }
            None => {
                let error_msg = "Unable to determine home directory".to_string();
                send_down_msg(DownMsg::DirectoryError { 
                    path: dir_path, 
                    error: error_msg 
                }, session_id, cor_id).await;
                return;
            }
        }
    } else {
        dir_path.clone()
    };
    
    let path = Path::new(&expanded_path);
    
    // Check if directory exists and is readable
    if !path.exists() {
        let error_msg = format!("Directory not found: {}", expanded_path);
        send_down_msg(DownMsg::DirectoryError { 
            path: expanded_path, 
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    if !path.is_dir() {
        let error_msg = format!("Path is not a directory: {}", expanded_path);
        send_down_msg(DownMsg::DirectoryError { 
            path: expanded_path, 
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    // Use async parallel directory scanning for maximum performance
    match scan_directory_async(path).await {
        Ok(items) => {
            send_down_msg(DownMsg::DirectoryContents { 
                path: expanded_path.clone(), 
                items 
            }, session_id, cor_id).await;
        }
        Err(e) => {
            let error_msg = format!("Failed to scan directory: {}", e);
            send_down_msg(DownMsg::DirectoryError { 
                path: expanded_path, 
                error: error_msg 
            }, session_id, cor_id).await;
        }
    }
}

async fn browse_directories_batch(dir_paths: Vec<String>, session_id: SessionId, cor_id: CorId) {
    // Use jwalk's parallel processing capabilities for batch directory scanning
    let mut results = HashMap::new();
    
    // Process directories in parallel using jwalk's thread pool
    let parallel_tasks: Vec<_> = dir_paths.into_iter()
        .map(|dir_path| {
            tokio::spawn(async move {
                let expanded_path = if dir_path.starts_with("~/") {
                    // Expand home directory path
                    if let Some(home_dir) = dirs::home_dir() {
                        home_dir.join(&dir_path[2..]).to_string_lossy().to_string()
                    } else {
                        dir_path
                    }
                } else if dir_path == "~" {
                    // User home directory
                    dirs::home_dir()
                        .map_or(dir_path, |home| home.to_string_lossy().to_string())
                } else {
                    dir_path
                };
                
                let path = Path::new(&expanded_path);
                
                // Scan directory with jwalk
                let result = if !path.exists() {
                    Err(format!("Path does not exist: {}", expanded_path))
                } else if !path.is_dir() {
                    Err(format!("Path is not a directory: {}", expanded_path))
                } else {
                    match scan_directory_async(path).await {
                        Ok(items) => Ok(items),
                        Err(e) => Err(format!("Failed to scan directory: {}", e))
                    }
                };
                
                (expanded_path, result)
            })
        })
        .collect();
    
    // Collect all results
    for task in parallel_tasks {
        if let Ok((path, result)) = task.await {
            results.insert(path, result);
        }
    }
    
    // Send batch results to frontend
    send_down_msg(DownMsg::BatchDirectoryContents { results }, session_id, cor_id).await;
}

async fn scan_directory_async(path: &Path) -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
    let path_buf = path.to_path_buf();
    
    // Use jwalk for parallel directory traversal, bridged with tokio
    let items = tokio::task::spawn_blocking(move || -> Result<Vec<FileSystemItem>, Box<dyn std::error::Error + Send + Sync>> {
        let mut items = Vec::new();
        
        // Test directory access before jwalk to catch permission errors early
        match std::fs::read_dir(&path_buf) {
            Ok(_) => {
                // Directory is readable, proceed with jwalk
            }
            Err(e) => {
                // Return permission/access error immediately
                return Err(format!("Permission denied: {}", e).into());
            }
        }
        
        // jwalk with parallel processing, single directory level
        for entry in WalkDir::new(&path_buf)
            .sort(true)  // Enable sorting for consistent results
            .max_depth(1)  // Single level only (like TreeView expansion)
            .skip_hidden(false)  // We'll filter manually to match existing logic
            .process_read_dir(|_, _, _, dir_entry_results| {
                // Filter entries in parallel processing callback for better performance
                dir_entry_results.retain(|entry_result| {
                    if let Ok(entry) = entry_result {
                        let name = entry.file_name().to_string_lossy();
                        !name.starts_with('.') // Skip hidden files
                    } else {
                        true // Keep errors for proper handling
                    }
                });
            })
        {
            match entry {
                Ok(dir_entry) => {
                    let entry_path = dir_entry.path();
                    
                    // Skip the root directory itself (jwalk includes it)
                    if entry_path == path_buf {
                        continue;
                    }
                    
                    let name = entry_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    let is_directory = entry_path.is_dir();
                    let path_str = entry_path.to_string_lossy().to_string();
                    
                    // Only include directories and waveform files for cleaner file dialog
                    let is_waveform = if !is_directory {
                        let name_lower = name.to_lowercase();
                        name_lower.ends_with(".vcd") || name_lower.ends_with(".fst")
                    } else {
                        false
                    };
                    
                    // Skip non-waveform files to reduce clutter
                    if !is_directory && !is_waveform {
                        continue;
                    }
                    
                    let item = FileSystemItem {
                        name,
                        path: path_str,
                        is_directory,
                        file_size: None, // Skip file size for instant loading
                        is_waveform_file: is_waveform, // Proper waveform detection  
                        file_extension: None, // Skip extension parsing for instant loading
                        has_expandable_content: is_directory, // All directories expandable
                    };
                    
                    items.push(item);
                }
                Err(e) => {
                    eprintln!("jwalk error processing entry: {}", e);
                    // Continue processing other entries despite individual errors
                }
            }
        }
        
        // Sort items: directories first, then files, both alphabetically
        // jwalk's sort(true) provides basic ordering, but we need custom logic
        items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        
        Ok(items)
    }).await??;
    
    Ok(items)
}

// REMOVED: process_entry_async and should_disable_directory functions for instant loading

#[moon::main]
async fn main() -> std::io::Result<()> {
    start(frontend, up_msg_handler, |_| {}).await
}
