use moon::*;
use shared::{self, UpMsg, DownMsg, AppConfig, FileHierarchy, WaveformFile, FileFormat, ScopeData, FileSystemItem, SignalValueQuery, SignalValueResult};
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

// Storage for parsed waveform data to enable signal value queries
struct WaveformData {
    hierarchy: wellen::Hierarchy,
    signal_source: Arc<Mutex<wellen::SignalSource>>,
    time_table: Vec<wellen::Time>,
    signals: HashMap<String, wellen::SignalRef>, // scope_path|variable_name -> SignalRef
    file_format: wellen::FileFormat, // Store file format for proper time conversion
}

static WAVEFORM_DATA_STORE: Lazy<Arc<Mutex<HashMap<String, WaveformData>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

async fn up_msg_handler(req: UpMsgRequest<UpMsg>) {
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
        UpMsg::QuerySignalValues { file_path, queries } => {
            query_signal_values(file_path, queries, session_id, cor_id).await;
        }
    }
}

async fn load_waveform_file(file_path: String, session_id: SessionId, cor_id: CorId) {
    
    let path = Path::new(&file_path);
    if !path.exists() {
        let error_msg = format!("File not found: {}", file_path);
        send_down_msg(DownMsg::ParsingError { 
            file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
            error: error_msg 
        }, session_id, cor_id).await;
        return;
    }
    
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let progress = Arc::new(Mutex::new(0.0));
    
    {
        let mut sessions = PARSING_SESSIONS.lock().unwrap();
        sessions.insert(file_path.clone(), progress.clone());
    }
    
    send_down_msg(DownMsg::ParsingStarted { 
        file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
        filename: filename.clone() 
    }, session_id, cor_id).await;
    
    // Use wellen's automatic file format detection instead of extension-based detection
    parse_waveform_file(file_path.clone(), file_path, filename, progress, session_id, cor_id).await;
}

async fn parse_waveform_file(file_path: String, file_id: String, filename: String, 
                       progress: Arc<Mutex<f32>>, session_id: SessionId, cor_id: CorId) {
    
    let options = wellen::LoadOptions::default();
    
    match wellen::viewers::read_header_from_file(&file_path, &options) {
        Ok(header_result) => {
            {
                let mut p = progress.lock().unwrap();
                *p = 0.3; // Header parsed
            }
            send_progress_update(file_id.clone(), 0.3, session_id, cor_id).await;
            
            // Read the body to get signal data and time table
            match wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None) {
                Ok(body_result) => {
                    {
                        let mut p = progress.lock().unwrap();
                        *p = 0.7; // Body parsed
                    }
                    send_progress_update(file_id.clone(), 0.7, session_id, cor_id).await;
                    
                    // Extract scopes first 
                    let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_path);
                    
                    // Build signal reference map for quick lookup
                    let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
                    build_signal_reference_map(&header_result.hierarchy, &mut signals);
                    
                    // Store waveform data for signal value queries
                    let waveform_data = WaveformData {
                        hierarchy: header_result.hierarchy,
                        signal_source: Arc::new(Mutex::new(body_result.source)),
                        time_table: body_result.time_table.clone(),
                        signals,
                        file_format: header_result.file_format,
                    };
                    
                    {
                        let mut store = WAVEFORM_DATA_STORE.lock().unwrap();
                        store.insert(file_path.clone(), waveform_data);
                    }
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
                    let error_msg = format!("Failed to read waveform body from '{}': {}", file_path, e);
                    send_down_msg(DownMsg::ParsingError { file_id, error: error_msg }, session_id, cor_id).await;
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to parse waveform file '{}': {}", file_path, e);
            send_down_msg(DownMsg::ParsingError { file_id, error: error_msg }, session_id, cor_id).await;
        }
    }
}


fn extract_scopes_from_hierarchy(hierarchy: &wellen::Hierarchy, file_path: &str) -> Vec<ScopeData> {
    hierarchy.scopes().map(|scope_ref| {
        extract_scope_data_with_file_path(hierarchy, scope_ref, file_path)
    }).collect()
}

fn extract_scope_data_with_file_path(hierarchy: &wellen::Hierarchy, scope_ref: wellen::ScopeRef, file_path: &str) -> ScopeData {
    let scope = &hierarchy[scope_ref];
    
    let mut variables: Vec<shared::Signal> = scope.vars(hierarchy).map(|var_ref| {
        let var = &hierarchy[var_ref];
        shared::Signal {
            id: var.name(hierarchy).to_string(), // Use variable name as ID
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
        extract_scope_data_with_file_path(hierarchy, child_scope_ref, file_path)
    }).collect();
    children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    
    ScopeData {
        id: format!("{}|{}", file_path, scope.full_name(hierarchy)), // Use full file path + | separator + scope path for unique ID
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
    if let Some(session) = sessions::by_session_id().wait_for(session_id).await {
        session.send_down_msg(&msg, cor_id).await;
    } else {
        // Session not found - likely disconnected
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
                        // Save migrated config to persist changes
                        if let Err(_save_err) = save_config_to_file(&config) {
                            // Migration applied but failed to save - continue with in-memory config
                        }
                    }
                    
                    config
                },
                Err(e) => {
                    send_down_msg(DownMsg::ConfigError(format!("Failed to parse config: {}", e)), session_id, cor_id).await;
                    return;
                }
            }
        }
        Err(_e) => {
            // Config file not found - create default
            // Create default config with validation already applied
            let mut default_config = AppConfig::default();
            let _warnings = default_config.validate_and_fix(); // Ensure defaults are validated too
            
            // Try to save default config
            if let Err(save_err) = save_config_to_file(&default_config) {
                send_down_msg(DownMsg::ConfigError(format!("Failed to create default config: {}", save_err)), session_id, cor_id).await;
                return;
            }
            
            default_config
        }
    };
    
    // PARALLEL PRELOADING: Start preloading expanded directories in background for instant file dialog
    let expanded_dirs = config.workspace.load_files_expanded_directories.clone();
    if !expanded_dirs.is_empty() {
        // Spawn background task to preload directories - precompute for instant access
        tokio::spawn(async move {
            let mut preload_tasks = Vec::new();
            
            // Create async task for each expanded directory
            for dir_path in expanded_dirs {
                let path = dir_path.clone();
                
                preload_tasks.push(tokio::spawn(async move {
                    let path_obj = Path::new(&path);
                    
                    // Precompute directory contents for instant access
                    if path_obj.exists() && path_obj.is_dir() {
                        match scan_directory_async(path_obj).await {
                            Ok(_items) => {
                            }
                            Err(_e) => {
                            }
                        }
                    }
                }));
            }
            
            // Wait for all preloading tasks to complete
            for task in preload_tasks {
                let _ = task.await; // Ignore individual task errors
            }
            
        });
    }

    send_down_msg(DownMsg::ConfigLoaded(config), session_id, cor_id).await;
}

async fn save_config(config: AppConfig, session_id: SessionId, cor_id: CorId) {
    match save_config_to_file(&config) {
        Ok(()) => {
            send_down_msg(DownMsg::ConfigSaved, session_id, cor_id).await;
        }
        Err(e) => {
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
                Err(_e) => {
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

// Build signal reference map for efficient lookup during value queries
fn build_signal_reference_map(hierarchy: &wellen::Hierarchy, signals: &mut HashMap<String, wellen::SignalRef>) {
    // Recursively process all scopes in the hierarchy
    for scope_ref in hierarchy.scopes() {
        build_signals_for_scope_recursive(hierarchy, scope_ref, signals);
    }
}

// Recursively process a scope and all its child scopes
fn build_signals_for_scope_recursive(hierarchy: &wellen::Hierarchy, scope_ref: wellen::ScopeRef, signals: &mut HashMap<String, wellen::SignalRef>) {
    let scope = &hierarchy[scope_ref];
    let scope_path = scope.full_name(hierarchy);
    
    // Process variables in this scope
    for var_ref in scope.vars(hierarchy) {
        let var = &hierarchy[var_ref];
        let variable_name = var.name(hierarchy);
        let signal_ref = var.signal_ref();
        
        // Key format: "scope_path|variable_name" to match SelectedVariable format
        let key = format!("{}|{}", scope_path, variable_name);
        signals.insert(key, signal_ref);
    }
    
    // Recursively process child scopes
    for child_scope_ref in scope.scopes(hierarchy) {
        build_signals_for_scope_recursive(hierarchy, child_scope_ref, signals);
    }
}

// Handle signal value queries
async fn query_signal_values(file_path: String, queries: Vec<SignalValueQuery>, session_id: SessionId, cor_id: CorId) {
    let store = WAVEFORM_DATA_STORE.lock().unwrap();
    let waveform_data = match store.get(&file_path) {
        Some(data) => data,
        None => {
            send_down_msg(DownMsg::SignalValuesError {
                file_path,
                error: "Waveform file not loaded or signal data not available".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    
    let mut results = Vec::new();
    
    for query in queries {
        let key = format!("{}|{}", query.scope_path, query.variable_name);
        
        match waveform_data.signals.get(&key) {
            Some(&signal_ref) => {
                // Convert time to time table index based on file format
                let target_time = match waveform_data.file_format {
                    wellen::FileFormat::Vcd => {
                        // For VCD: time table is in VCD's native units (depends on $timescale)
                        // For $timescale 1s: values are already in seconds
                        query.time_seconds as u64
                    },
                    _ => {
                        // For other formats, use femtosecond conversion  
                        (query.time_seconds * 1_000_000_000_000.0) as u64
                    }
                };
                
                let time_idx = match waveform_data.time_table.binary_search(&target_time) {
                    Ok(exact_idx) => exact_idx as u32,
                    Err(insert_pos) => insert_pos.saturating_sub(1) as u32,
                };
                
                // Load signal and get value
                let mut signal_source = waveform_data.signal_source.lock().unwrap();
                let loaded_signals = signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);
                
                match loaded_signals.into_iter().next() {
                    Some((_, signal)) => {
                        if let Some(offset) = signal.get_offset(time_idx) {
                            let value = signal.get_value_at(&offset, 0);
                            
                            // Try to convert to binary string for VarFormat processing
                            let (raw_value, formatted_value) = match signal_value_to_binary_string(&value) {
                                Some(binary_str) => {
                                    // Successfully converted to binary - apply requested format
                                    let formatted = query.format.format(&binary_str);
                                    (Some(binary_str), Some(formatted))
                                }
                                None => {
                                    // Cannot convert to binary (e.g., X/Z states, strings, reals)
                                    // Use fallback formatting and set raw_value to original string representation
                                    let fallback_formatted = format_non_binary_signal_value(&value);
                                    (Some(format!("{}", value)), Some(fallback_formatted))
                                }
                            };
                            
                            results.push(SignalValueResult {
                                scope_path: query.scope_path,
                                variable_name: query.variable_name,
                                time_seconds: query.time_seconds,
                                raw_value,
                                formatted_value,
                                format: query.format,
                            });
                        } else {
                            results.push(SignalValueResult {
                                scope_path: query.scope_path,
                                variable_name: query.variable_name,
                                time_seconds: query.time_seconds,
                                raw_value: None,
                                formatted_value: None,
                                format: query.format,
                            });
                        }
                    }
                    None => {
                        results.push(SignalValueResult {
                            scope_path: query.scope_path,
                            variable_name: query.variable_name,
                            time_seconds: query.time_seconds,
                            raw_value: None,
                            formatted_value: Some("Signal load failed".to_string()),
                            format: query.format,
                        });
                    }
                }
            }
            None => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    time_seconds: query.time_seconds,
                    raw_value: None,
                    formatted_value: Some("Signal not found".to_string()),
                    format: query.format,
                });
            }
        }
    }
    
    send_down_msg(DownMsg::SignalValues {
        file_path,
        results,
    }, session_id, cor_id).await;
}

// Convert wellen::SignalValue to binary string for VarFormat processing
fn signal_value_to_binary_string(value: &wellen::SignalValue) -> Option<String> {
    match value {
        wellen::SignalValue::Binary(bits, width) => {
            if *width == 1 {
                // Single bit
                if bits.is_empty() { 
                    None // Cannot convert unknown/undefined values
                } else { 
                    Some(format!("{}", bits[0] & 1))
                }
            } else {
                // Multi-bit binary - convert to binary string
                value.to_bit_string()
            }
        }
        wellen::SignalValue::FourValue(bits, width) => {
            if *width == 1 {
                // Single bit 4-state - only convert 0/1, not X/Z
                if bits.is_empty() { 
                    None
                } else {
                    match bits[0] & 3 {
                        0 => Some("0".to_string()),
                        1 => Some("1".to_string()),
                        _ => None, // X, Z cannot be converted to binary for formatting
                    }
                }
            } else {
                // Multi-bit 4-state - try to convert, may fail for X/Z values
                value.to_bit_string()
            }
        }
        wellen::SignalValue::NineValue(_bits, _width) => {
            // Try to convert, may fail for non-binary values
            value.to_bit_string()
        }
        wellen::SignalValue::String(_) => {
            // String values cannot be converted to binary format
            None
        }
        wellen::SignalValue::Real(_) => {
            // Real values cannot be converted to binary format
            None
        }
    }
}

// Fallback formatting for non-binary signal values
fn format_non_binary_signal_value(value: &wellen::SignalValue) -> String {
    match value {
        wellen::SignalValue::Binary(bits, width) => {
            if *width == 1 {
                if bits.is_empty() { "X".to_string() } else { format!("{}", bits[0] & 1) }
            } else {
                value.to_bit_string().unwrap_or_else(|| "?".to_string())
            }
        }
        wellen::SignalValue::FourValue(bits, width) => {
            if *width == 1 {
                if bits.is_empty() { "X".to_string() } 
                else {
                    match bits[0] & 3 {
                        0 => "0".to_string(),
                        1 => "1".to_string(),
                        2 => "X".to_string(),
                        3 => "Z".to_string(),
                        _ => "?".to_string(),
                    }
                }
            } else {
                value.to_bit_string().unwrap_or_else(|| "?".to_string())
            }
        }
        wellen::SignalValue::NineValue(_bits, _width) => {
            value.to_bit_string().unwrap_or_else(|| "?".to_string())
        }
        wellen::SignalValue::String(s) => s.to_string(),
        wellen::SignalValue::Real(f) => format!("{:.6}", f),
    }
}

#[moon::main]
async fn main() -> std::io::Result<()> {
    start(frontend, up_msg_handler, |_| {}).await
}
