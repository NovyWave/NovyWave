use moon::*;
use shared::{self, UpMsg, DownMsg, AppConfig, FileHierarchy, WaveformFile, FileFormat, ScopeData, FileSystemItem, SignalValueQuery, SignalValueResult, SignalTransitionQuery, SignalTransition, SignalTransitionResult, FileError};
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
    timescale_factor: f64, // Conversion factor from VCD native units to seconds
}

static WAVEFORM_DATA_STORE: Lazy<Arc<Mutex<HashMap<String, WaveformData>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// Track VCD body loading in progress to prevent concurrent loading of same file
static VCD_LOADING_IN_PROGRESS: Lazy<Arc<Mutex<std::collections::HashSet<String>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(std::collections::HashSet::new())));

async fn up_msg_handler(req: UpMsgRequest<UpMsg>) {
    let (session_id, cor_id) = (req.session_id, req.cor_id);
    
    // Log all incoming requests for debugging
    // Removed spammy request type debug logging
    
    match &req.up_msg {
        UpMsg::LoadWaveformFile(file_path) => {
            println!("BACKEND: Loading waveform file: {}", file_path);
            load_waveform_file(file_path.clone(), session_id, cor_id).await;
        }
        UpMsg::GetParsingProgress(file_id) => {
            send_parsing_progress(file_id.clone(), session_id, cor_id).await;
        }
        UpMsg::LoadConfig => {
            println!("BACKEND: Loading config");
            load_config(session_id, cor_id).await;
        }
        UpMsg::SaveConfig(config) => {
            println!("BACKEND: Saving config");
            save_config(config.clone(), session_id, cor_id).await;
        }
        UpMsg::BrowseDirectory(dir_path) => {
            println!("BACKEND: Browsing directory: {}", dir_path);
            browse_directory(dir_path.clone(), session_id, cor_id).await;
        }
        UpMsg::BrowseDirectories(dir_paths) => {
            println!("BACKEND: Browsing {} directories", dir_paths.len());
            browse_directories_batch(dir_paths.clone(), session_id, cor_id).await;
        }
        UpMsg::QuerySignalValues { file_path, queries } => {
            // Removed spammy debug logging for signal queries
            query_signal_values(file_path.clone(), queries.clone(), session_id, cor_id).await;
        }
        UpMsg::QuerySignalTransitions { file_path, signal_queries, time_range } => {
            println!("BACKEND: Querying {} signal transitions from {} (range: {:?})", 
                    signal_queries.len(), file_path, time_range);
            query_signal_transitions(file_path.clone(), signal_queries.clone(), time_range.clone(), session_id, cor_id).await;
        }
        UpMsg::BatchQuerySignalValues { batch_id, file_queries } => {
            // Handle batch signal value queries - process multiple files in one request
            let mut file_results = Vec::new();
            
            for file_query in file_queries {
                // Process each file's queries
                let results = match process_signal_value_queries_internal(&file_query.file_path, &file_query.queries).await {
                    Ok(results) => results,
                    Err(_) => Vec::new(), // Skip failed file queries in batch
                };
                file_results.push(shared::FileSignalResults {
                    file_path: file_query.file_path.clone(),
                    results,
                });
            }
            
            // Send batch response
            send_down_msg(DownMsg::BatchSignalValues { batch_id: batch_id.clone(), file_results }, session_id, cor_id).await;
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
    
    // Let wellen handle all validation - it knows best
    
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let progress = Arc::new(Mutex::new(0.0));
    
    {
        match PARSING_SESSIONS.lock() {
            Ok(mut sessions) => {
                sessions.insert(file_path.clone(), progress.clone());
            }
            Err(e) => {
                let error_msg = format!("Internal error: Failed to access parsing sessions - {}", e);
                send_parsing_error(file_path.clone(), filename, error_msg, session_id, cor_id).await;
                return;
            }
        }
    }
    
    send_down_msg(DownMsg::ParsingStarted { 
        file_id: file_path.clone(), // Use full path to match frontend TrackedFile IDs
        filename: filename.clone() 
    }, session_id, cor_id).await;
    
    // Use wellen's automatic file format detection instead of extension-based detection
    parse_waveform_file(file_path.clone(), file_path, filename, progress, session_id, cor_id).await;
}


async fn send_parsing_error(file_id: String, filename: String, error: String, session_id: SessionId, cor_id: CorId) {
    println!("Parsing error for {}: {}", filename, error);
    
    send_down_msg(DownMsg::ParsingError { 
        file_id, 
        error 
    }, session_id, cor_id).await;
}

/// Enhanced error sending with structured FileError - provides better error context
async fn send_structured_parsing_error(file_id: String, filename: String, file_error: FileError, session_id: SessionId, cor_id: CorId) {
    // Log the error with structured context for debugging
    println!("Parsing error for {}: {} - {}", filename, file_error.category(), file_error.user_friendly_message());
    
    // Send the user-friendly message to maintain compatibility with existing frontend
    send_down_msg(DownMsg::ParsingError { 
        file_id, 
        error: file_error.user_friendly_message()
    }, session_id, cor_id).await;
}

async fn parse_waveform_file(file_path: String, file_id: String, filename: String, 
                       progress: Arc<Mutex<f32>>, session_id: SessionId, cor_id: CorId) {
    
    let options = wellen::LoadOptions::default();
    
    // Catch panics from wellen parsing to prevent crashes
    let parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_header_from_file(&file_path, &options)
    }));
    
    let header_result = match parse_result {
        Ok(Ok(header)) => header,
        Ok(Err(e)) => {
            let file_error = convert_wellen_error_to_file_error(&e.to_string(), &file_path);
            send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
            return;
        }
        Err(_panic) => {
            let file_error = convert_panic_to_file_error(&file_path);
            send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
            return;
        }
    };
    
    {
        match progress.lock() {
            Ok(mut p) => *p = 0.3, // Header parsed
            Err(_) => {
                // Progress mutex poisoned - continue without progress updates
                eprintln!("Warning: Progress tracking failed for {}", filename);
            }
        }
    }
    send_progress_update(file_id.clone(), 0.3, session_id, cor_id).await;
    
    // Handle FST and VCD differently for progressive loading
    match header_result.file_format {
        wellen::FileFormat::Fst => {
            // FST: Time table available immediately from header parsing
            // Progressive loading: extract time table quickly, defer full signal data
            let body_parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None)
            }));
            
            let body_result = match body_parse_result {
                Ok(Ok(body)) => body,
                Ok(Err(e)) => {
                    let file_error = convert_wellen_error_to_file_error(&e.to_string(), &file_path);
                    send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
                    return;
                }
                Err(_panic) => {
                    let file_error = convert_panic_to_file_error(&file_path);
                    send_structured_parsing_error(file_id, filename, file_error, session_id, cor_id).await;
                    return;
                }
            };
            {
                match progress.lock() {
                    Ok(mut p) => *p = 0.7, // Body parsed
                    Err(_) => {
                        // Progress mutex poisoned - continue without progress updates
                        eprintln!("Warning: Progress tracking failed for {}", filename);
                    }
                }
            }
            send_progress_update(file_id.clone(), 0.7, session_id, cor_id).await;
                    
                    // Calculate timescale factor with intelligent FST inference
                    let embedded_timescale_factor = match header_result.hierarchy.timescale() {
                        Some(ts) => {
                            use wellen::TimescaleUnit;
                            let factor = match ts.unit {
                                TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                                TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                                TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                                TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                                TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                                TimescaleUnit::Seconds => ts.factor as f64,
                                TimescaleUnit::Unknown => ts.factor as f64,
                            };
                            println!("FST TIMESCALE DEBUG: {} factor={}, unit={:?}, computed_factor={}", filename, ts.factor, ts.unit, factor);
                            factor
                        }
                        None => {
                            println!("FST TIMESCALE DEBUG: {} has NO timescale info, using nanosecond default", filename);
                            1e-9 // Default to nanoseconds if no timescale info
                        }
                    };
                    
                    // For FST files, check if embedded timescale produces unreasonable durations and override if needed
                    let timescale_factor = infer_reasonable_fst_timescale(&body_result, embedded_timescale_factor, &filename);
                    
                    // FST: Extract time range using proper timescale
                    let (min_seconds, max_seconds) = extract_fst_time_range(&body_result, timescale_factor);
                    let (min_time, max_time) = (Some(min_seconds), Some(max_seconds));
                    
                    // Extract scopes from hierarchy 
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
                        timescale_factor,
                    };
                    
                    {
                        match WAVEFORM_DATA_STORE.lock() {
                            Ok(mut store) => {
                                store.insert(file_path.clone(), waveform_data);
                            }
                            Err(e) => {
                                let error_msg = format!("Internal error: Failed to store waveform data - {}", e);
                                send_parsing_error(file_id.clone(), filename, error_msg, session_id, cor_id).await;
                                return;
                            }
                        }
                    }
                    
                    let format = FileFormat::FST;
                    
                    let waveform_file = WaveformFile {
                        id: file_id.clone(),
                        filename: filename.clone(),
                        format,
                        scopes,
                        min_time,
                        max_time,
                    };
                    
                    let file_hierarchy = FileHierarchy {
                        files: vec![waveform_file],
                    };
                    
                    {
                        match progress.lock() {
                            Ok(mut p) => *p = 1.0, // Complete
                            Err(_) => {
                                // Progress mutex poisoned - continue without progress updates
                                eprintln!("Warning: Progress tracking failed for {}", filename);
                            }
                        }
                    }
                    send_progress_update(file_id.clone(), 1.0, session_id, cor_id).await;
                    
                    send_down_msg(DownMsg::FileLoaded { 
                        file_id: file_id.clone(), 
                        hierarchy: file_hierarchy 
                    }, session_id, cor_id).await;
                    
                    cleanup_parsing_session(&file_id);
        }
        wellen::FileFormat::Vcd => {
                    // VCD: Use progressive loading with quick time bounds extraction
                    
                    // First: Try quick time bounds extraction (much faster)
                    let (min_seconds, max_seconds) = match extract_vcd_time_bounds_fast(&file_path) {
                        Ok((min_time, max_time)) => {
                            // Apply proper timescale conversion for VCD based on unit
                            let timescale_factor = header_result.hierarchy.timescale()
                                .map(|ts| {
                                    use wellen::TimescaleUnit;
                                    match ts.unit {
                                        TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                                        TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                                        TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                                        TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                                        TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                                        TimescaleUnit::Seconds => ts.factor as f64,
                                        TimescaleUnit::Unknown => ts.factor as f64, // Default to no conversion
                                    }
                                })
                                .unwrap_or(1.0);
                            
                            let converted_min = min_time * timescale_factor;
                            let converted_max = max_time * timescale_factor;
                            
                            (converted_min, converted_max)
                        }
                        Err(_) => {
                            // Fallback to slower method if quick scan fails
                            println!("VCD quick scan failed, falling back to full parsing");
                            (0.0, 100.0) // Will be updated after full parsing
                        }
                    };
                    
                    {
                        match progress.lock() {
                            Ok(mut p) => *p = 0.5, // Quick time bounds extracted
                            Err(_) => {
                                // Progress mutex poisoned - continue without progress updates
                                eprintln!("Warning: Progress tracking failed for {}", filename);
                            }
                        }
                    }
                    send_progress_update(file_id.clone(), 0.5, session_id, cor_id).await;
                    
                    // Extract scopes from header (immediate - no body parsing needed)
                    let scopes = extract_scopes_from_hierarchy(&header_result.hierarchy, &file_path);
                    
                    let format = FileFormat::VCD;
                    let (min_time, max_time) = (Some(min_seconds), Some(max_seconds));
                    
                    // Create lightweight file data WITHOUT full signal source
                    let waveform_file = WaveformFile {
                        id: file_id.clone(),
                        filename: filename.clone(),
                        format,
                        scopes,
                        min_time,
                        max_time,
                    };
                    
                    let file_hierarchy = FileHierarchy {
                        files: vec![waveform_file],
                    };
                    
                    {
                        match progress.lock() {
                            Ok(mut p) => *p = 1.0, // Complete - fast header-only loading!
                            Err(_) => {
                                // Progress mutex poisoned - continue without progress updates
                                eprintln!("Warning: Progress tracking failed for {}", filename);
                            }
                        }
                    }
                    send_progress_update(file_id.clone(), 1.0, session_id, cor_id).await;
                    
                    send_down_msg(DownMsg::FileLoaded { 
                        file_id: file_id.clone(), 
                        hierarchy: file_hierarchy 
                    }, session_id, cor_id).await;
                    
                    cleanup_parsing_session(&file_id);
                }
        wellen::FileFormat::Ghw | wellen::FileFormat::Unknown => {
            // TODO: Add proper error handling for GHW/Unknown formats
            let error_msg = format!("GHW and Unknown formats temporarily disabled during error handling implementation");
            send_parsing_error(file_id, filename, error_msg, session_id, cor_id).await;
        }
    }
}

/// Convert wellen parsing errors to structured FileError for better error handling
fn convert_wellen_error_to_file_error(wellen_error: &str, file_path: &str) -> FileError {
    let path = file_path.to_string();
    
    // Pattern match common wellen error messages to appropriate FileError types
    if wellen_error.contains("No such file") || wellen_error.contains("not found") {
        FileError::FileNotFound { path }
    } else if wellen_error.contains("Permission denied") || wellen_error.contains("permission") {
        FileError::PermissionDenied { path }
    } else if wellen_error.contains("corrupted") || wellen_error.contains("invalid format") || wellen_error.contains("malformed") {
        FileError::CorruptedFile { 
            path, 
            details: wellen_error.to_string() 
        }
    } else if wellen_error.contains("too large") || wellen_error.contains("size") {
        // Extract size information if available, otherwise use defaults
        FileError::FileTooLarge { 
            path, 
            size: 0, // Could parse from error message if needed
            max_size: 1_000_000_000 // 1GB default
        }
    } else if wellen_error.contains("unsupported") || wellen_error.contains("format") {
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        FileError::UnsupportedFormat { 
            path, 
            extension: extension.to_string(),
            supported_formats: vec!["vcd".to_string(), "fst".to_string()]
        }
    } else {
        // Generic parsing error for everything else
        FileError::ParseError { 
            source: wellen_error.to_string(),
            context: format!("Failed to parse waveform file: {}", file_path)
        }
    }
}

/// Convert panic messages to structured FileError
fn convert_panic_to_file_error(file_path: &str) -> FileError {
    FileError::CorruptedFile { 
        path: file_path.to_string(),
        details: "Critical error: Invalid waveform data or corrupted file".to_string()
    }
}

/// Intelligent FST timescale inference to handle files with incorrect embedded timescale
fn infer_reasonable_fst_timescale(body_result: &wellen::viewers::BodyResult, embedded_factor: f64, filename: &str) -> f64 {
    if body_result.time_table.is_empty() {
        return embedded_factor; // Can't infer from empty time table
    }
    
    let raw_min = body_result.time_table.first().map(|&t| t as f64).unwrap_or(0.0);
    let raw_max = body_result.time_table.last().map(|&t| t as f64).unwrap_or(0.0);
    let raw_range = raw_max - raw_min;
    
    // Check if embedded timescale produces unreasonably long duration (>1000 seconds)
    let computed_duration = raw_range * embedded_factor;
    
    if computed_duration > 1000.0 {
        println!("FST INFERENCE WARNING: {} embedded timescale produces {:.1}s duration, too long for typical simulation", filename, computed_duration);
        
        // Heuristic inference based on value magnitude - optimized for typical FPGA/digital designs
        let inferred_factor = if raw_range > 1e15 {
            println!("FST INFERENCE: Very large values ({}), assuming femtoseconds", raw_range);
            1e-15 // femtoseconds
        } else if raw_range > 1e12 {
            println!("FST INFERENCE: Large values ({}), assuming picoseconds", raw_range);
            1e-12 // picoseconds
        } else if raw_range > 1e6 {
            println!("FST INFERENCE: Medium values ({}), assuming nanoseconds", raw_range);
            1e-9  // nanoseconds - most common for FPGA/CPU designs (covers 1ms to 1000s of sim time)
        } else if raw_range > 1e3 {
            println!("FST INFERENCE: Small values ({}), assuming microseconds", raw_range);
            1e-6  // microseconds
        } else {
            println!("FST INFERENCE: Very small values ({}), assuming milliseconds", raw_range);
            1e-3  // milliseconds
        };
        
        let inferred_duration = raw_range * inferred_factor;
        println!("FST INFERENCE: Using factor {} -> {:.6}s duration", inferred_factor, inferred_duration);
        
        return inferred_factor;
    }
    
    println!("FST INFERENCE: Embedded timescale produces reasonable duration ({:.6}s), keeping it", computed_duration);
    embedded_factor
}

fn extract_fst_time_range(body_result: &wellen::viewers::BodyResult, timescale_factor: f64) -> (f64, f64) {
    if body_result.time_table.is_empty() {
        return (0.0, 100.0); // Default fallback
    }
    
    let raw_min = match body_result.time_table.first() {
        Some(time) => *time as f64,
        None => {
            eprintln!("Warning: Empty time table in FST file");
            return (0.0, 100.0);
        }
    };
    
    let raw_max = match body_result.time_table.last() {
        Some(time) => *time as f64,
        None => {
            eprintln!("Warning: Empty time table in FST file");
            return (0.0, 100.0);
        }
    };
    
    println!("FST TIME EXTRACTION DEBUG:");
    println!("  Raw time values: {} to {}", raw_min, raw_max);
    println!("  Timescale factor: {}", timescale_factor);
    println!("  Converted seconds: {} to {}", raw_min * timescale_factor, raw_max * timescale_factor);
    
    // Convert FST time values to seconds using the proper timescale factor
    (raw_min * timescale_factor, raw_max * timescale_factor)
}


fn extract_vcd_time_bounds_fast(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs::File;
    
    let file = File::open(file_path)?;
    let file_size = file.metadata()?.len();
    
    // For large files, use memory-mapped scanning
    if file_size > 100_000_000 { // 100MB threshold
        extract_vcd_time_bounds_mmap(file_path)
    } else {
        extract_vcd_time_bounds_small(file_path)
    }
}

fn extract_vcd_time_bounds_mmap(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs::File;
    
    let file = File::open(file_path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    
    // Find end of header ($enddefinitions $end)
    let header_end = find_vcd_definitions_end(&mmap)?;
    let body_section = &mmap[header_end..];
    
    // Scan for first timestamp from beginning of body
    let first_time = find_first_vcd_timestamp(body_section)?;
    
    // Scan for last timestamp from end (more efficient)
    let last_time = find_last_vcd_timestamp_reverse(body_section)?;
    
    Ok((first_time, last_time))
}

fn extract_vcd_time_bounds_small(file_path: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    use std::fs;
    
    let content = fs::read_to_string(file_path)?;
    
    // Find end of header
    let header_end = content.find("$enddefinitions $end")
        .ok_or("VCD header end marker not found")?;
    
    let body_section = &content[header_end..];
    
    // Find first and last timestamps
    let first_time = find_first_vcd_timestamp_str(body_section)?;
    let last_time = find_last_vcd_timestamp_str(body_section)?;
    
    Ok((first_time, last_time))
}

fn find_vcd_definitions_end(data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = b"$enddefinitions $end";
    let pos = data.windows(pattern.len())
        .position(|window| window == pattern)
        .ok_or("VCD header end marker not found")?;
    Ok(pos + pattern.len())
}

fn find_first_vcd_timestamp(data: &[u8]) -> Result<f64, Box<dyn std::error::Error>> {
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'#' {
            // Found timestamp marker, parse the number
            i += 1;
            let mut timestamp_str = String::new();
            while i < data.len() && data[i].is_ascii_digit() {
                timestamp_str.push(data[i] as char);
                i += 1;
            }
            if !timestamp_str.is_empty() {
                return Ok(timestamp_str.parse::<f64>()?);
            }
        }
        i += 1;
    }
    Err("No VCD timestamp found".into())
}

fn find_last_vcd_timestamp_reverse(data: &[u8]) -> Result<f64, Box<dyn std::error::Error>> {
    let mut i = data.len();
    while i > 0 {
        i -= 1;
        if data[i] == b'#' && i + 1 < data.len() && data[i + 1].is_ascii_digit() {
            // Found timestamp marker, parse the number forward
            let mut j = i + 1;
            let mut timestamp_str = String::new();
            while j < data.len() && data[j].is_ascii_digit() {
                timestamp_str.push(data[j] as char);
                j += 1;
            }
            if !timestamp_str.is_empty() {
                return Ok(timestamp_str.parse::<f64>()?);
            }
        }
    }
    Err("No VCD timestamp found in reverse scan".into())
}

fn find_first_vcd_timestamp_str(body: &str) -> Result<f64, Box<dyn std::error::Error>> {
    for line in body.lines() {
        if line.starts_with('#') {
            let timestamp_str = &line[1..].split_whitespace().next().unwrap_or("");
            if !timestamp_str.is_empty() {
                return Ok(timestamp_str.parse::<f64>()?);
            }
        }
    }
    Err("No VCD timestamp found".into())
}

fn find_last_vcd_timestamp_str(body: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let mut last_timestamp = None;
    for line in body.lines() {
        if line.starts_with('#') {
            let timestamp_str = &line[1..].split_whitespace().next().unwrap_or("");
            if !timestamp_str.is_empty() {
                if let Ok(timestamp) = timestamp_str.parse::<f64>() {
                    last_timestamp = Some(timestamp);
                }
            }
        }
    }
    last_timestamp.ok_or("No VCD timestamp found in body".into())
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
    let sessions = match PARSING_SESSIONS.lock() {
        Ok(sessions) => sessions,
        Err(_) => {
            eprintln!("Warning: Failed to access parsing sessions for progress update");
            return;
        }
    };
    
    if let Some(progress) = sessions.get(&file_id) {
        let current_progress = match progress.lock() {
            Ok(p) => *p,
            Err(_) => {
                eprintln!("Warning: Failed to read progress for file {}", file_id);
                return;
            }
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
    match PARSING_SESSIONS.lock() {
        Ok(mut sessions) => {
            sessions.remove(file_id);
        }
        Err(_) => {
            eprintln!("Warning: Failed to cleanup parsing session for file {}", file_id);
        }
    }
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
// Load VCD body data on-demand for signal value queries
async fn ensure_vcd_body_loaded(file_path: &str) -> Result<(), String> {
    // Check if already loaded
    {
        let store = match WAVEFORM_DATA_STORE.lock() {
            Ok(store) => store,
            Err(_) => return Err("Internal error: Failed to access waveform data store".to_string()),
        };
        if store.contains_key(file_path) {
            return Ok(());
        }
    }
    
    // Check if loading is already in progress for this file
    {
        let mut loading_in_progress = match VCD_LOADING_IN_PROGRESS.lock() {
            Ok(loading) => loading,
            Err(_) => return Err("Internal error: Failed to access VCD loading tracker".to_string()),
        };
        
        if loading_in_progress.contains(file_path) {
            // Another thread is already loading this file, just return success
            // The other thread will populate the data store
            return Ok(());
        }
        
        // Mark this file as being loaded
        loading_in_progress.insert(file_path.to_string());
    }
    
    // Need to parse VCD body - reparse the file
    let options = wellen::LoadOptions::default();
    
    // Catch panics from header parsing
    let header_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_header_from_file(file_path, &options)
    })) {
        Ok(Ok(header)) => header,
        Ok(Err(e)) => {
            // Remove from loading tracker on failure
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Failed to parse VCD header for signal queries: {}", e));
        }
        Err(_panic) => {
            // Remove from loading tracker on panic
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Critical error parsing VCD header: Invalid waveform data"));
        }
    };
    
    // Catch panics from body parsing
    let body_result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wellen::viewers::read_body(header_result.body, &header_result.hierarchy, None)
    })) {
        Ok(Ok(body)) => body,
        Ok(Err(e)) => {
            // Remove from loading tracker on failure
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Failed to parse VCD body for signal queries: {}", e));
        }
        Err(_panic) => {
            // Remove from loading tracker on panic
            if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                loading_in_progress.remove(file_path);
            }
            return Err(format!("Critical error parsing VCD body: Invalid signal data"));
        }
    };
    
    // Build signal reference map
    let mut signals: HashMap<String, wellen::SignalRef> = HashMap::new();
    build_signal_reference_map(&header_result.hierarchy, &mut signals);
    
    // Calculate timescale factor for VCD time conversion
    let timescale_factor = header_result.hierarchy.timescale()
        .map(|ts| {
            use wellen::TimescaleUnit;
            match ts.unit {
                TimescaleUnit::FemtoSeconds => ts.factor as f64 * 1e-15,
                TimescaleUnit::PicoSeconds => ts.factor as f64 * 1e-12,
                TimescaleUnit::NanoSeconds => ts.factor as f64 * 1e-9,
                TimescaleUnit::MicroSeconds => ts.factor as f64 * 1e-6,
                TimescaleUnit::MilliSeconds => ts.factor as f64 * 1e-3,
                TimescaleUnit::Seconds => ts.factor as f64,
                TimescaleUnit::Unknown => ts.factor as f64,
            }
        })
        .unwrap_or(1.0);
    
    // Store waveform data
    let waveform_data = WaveformData {
        hierarchy: header_result.hierarchy,
        signal_source: Arc::new(Mutex::new(body_result.source)),
        time_table: body_result.time_table.clone(),
        signals,
        file_format: header_result.file_format,
        timescale_factor,
    };
    
    {
        match WAVEFORM_DATA_STORE.lock() {
            Ok(mut store) => {
                store.insert(file_path.to_string(), waveform_data);
            }
            Err(_) => {
                // Remove from loading tracker on failure
                if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
                    loading_in_progress.remove(file_path);
                }
                return Err("Internal error: Failed to store waveform data".to_string());
            }
        }
    }
    
    // Remove from loading tracker on success
    if let Ok(mut loading_in_progress) = VCD_LOADING_IN_PROGRESS.lock() {
        loading_in_progress.remove(file_path);
    }
    
    Ok(())
}

async fn query_signal_values(file_path: String, queries: Vec<SignalValueQuery>, session_id: SessionId, cor_id: CorId) {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(&file_path).await {
            send_down_msg(DownMsg::SignalValuesError {
                file_path,
                error: e,
            }, session_id, cor_id).await;
            return;
        }
    }
    
    let store = match WAVEFORM_DATA_STORE.lock() {
        Ok(store) => store,
        Err(_) => {
            send_down_msg(DownMsg::SignalValuesError {
                file_path,
                error: "Internal error: Failed to access waveform data store".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    
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
                // Convert time to time table index based on file format using proper timescale
                let target_time = match waveform_data.file_format {
                    wellen::FileFormat::Vcd => {
                        // For VCD: Convert from seconds to VCD native units using stored timescale
                        (query.time_seconds / waveform_data.timescale_factor) as u64
                    },
                    _ => {
                        // For other formats, use proper timescale conversion
                        (query.time_seconds / waveform_data.timescale_factor) as u64
                    }
                };
                
                let time_idx = match waveform_data.time_table.binary_search(&target_time) {
                    Ok(exact_idx) => exact_idx as u32,
                    Err(insert_pos) => insert_pos.saturating_sub(1) as u32,
                };
                
                // Load signal and get value
                let mut signal_source = match waveform_data.signal_source.lock() {
                    Ok(source) => source,
                    Err(_) => {
                        results.push(SignalValueResult {
                            scope_path: query.scope_path,
                            variable_name: query.variable_name,
                            time_seconds: query.time_seconds,
                            raw_value: Some("Error: Signal source unavailable".to_string()),
                            formatted_value: Some("Error".to_string()),
                            format: query.format,
                        });
                        continue;
                    }
                };
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

// Helper function for batch processing - returns results instead of sending
async fn process_signal_value_queries_internal(file_path: &str, queries: &[SignalValueQuery]) -> Result<Vec<SignalValueResult>, String> {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(file_path).await {
            return Err(e);
        }
    }
    
    let store = WAVEFORM_DATA_STORE.lock().map_err(|_| "Failed to access waveform data store".to_string())?;
    
    let waveform_data = store.get(file_path)
        .ok_or_else(|| "Waveform file not loaded or signal data not available".to_string())?;
    
    let mut results = Vec::new();
    
    for query in queries {
        // Same logic as query_signal_values but collect results instead of sending
        let key = format!("{}|{}", query.scope_path, query.variable_name);
        let signal_ref = match waveform_data.signals.get(&key) {
            Some(&signal_ref) => signal_ref,
            None => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path.clone(),
                    variable_name: query.variable_name.clone(),
                    time_seconds: query.time_seconds,
                    raw_value: Some("Signal not found".to_string()),
                    formatted_value: Some("N/A".to_string()),
                    format: query.format.clone(),
                });
                continue;
            }
        };
        
        let target_time = match waveform_data.file_format {
            wellen::FileFormat::Vcd => {
                // For VCD: Convert from seconds to VCD native units using stored timescale
                (query.time_seconds / waveform_data.timescale_factor) as u64
            },
            _ => {
                // For other formats, use proper timescale conversion
                (query.time_seconds / waveform_data.timescale_factor) as u64
            }
        };
        
        let time_idx = match waveform_data.time_table.binary_search(&target_time) {
            Ok(exact_idx) => exact_idx as u32,
            Err(insert_pos) => insert_pos.saturating_sub(1) as u32,
        };
        
        let mut signal_source = match waveform_data.signal_source.lock() {
            Ok(source) => source,
            Err(_) => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path.clone(),
                    variable_name: query.variable_name.clone(),
                    time_seconds: query.time_seconds,
                    raw_value: Some("Error: Signal source unavailable".to_string()),
                    formatted_value: Some("Error".to_string()),
                    format: query.format.clone(),
                });
                continue;
            }
        };
        
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
                        scope_path: query.scope_path.clone(),
                        variable_name: query.variable_name.clone(),
                        time_seconds: query.time_seconds,
                        raw_value,
                        formatted_value,
                        format: query.format.clone(),
                    });
                } else {
                    results.push(SignalValueResult {
                        scope_path: query.scope_path.clone(),
                        variable_name: query.variable_name.clone(),
                        time_seconds: query.time_seconds,
                        raw_value: Some("No data".to_string()),
                        formatted_value: Some("X".to_string()),
                        format: query.format.clone(),
                    });
                }
            },
            None => {
                results.push(SignalValueResult {
                    scope_path: query.scope_path.clone(),
                    variable_name: query.variable_name.clone(),
                    time_seconds: query.time_seconds,
                    raw_value: Some("Failed to load signal".to_string()),
                    formatted_value: Some("Error".to_string()),
                    format: query.format.clone(),
                });
            }
        }
    }
    
    Ok(results)
}

async fn query_signal_transitions(
    file_path: String, 
    signal_queries: Vec<SignalTransitionQuery>, 
    time_range: (f64, f64), 
    session_id: SessionId, 
    cor_id: CorId
) {
    // For VCD files, ensure body is loaded on-demand
    if file_path.ends_with(".vcd") {
        if let Err(e) = ensure_vcd_body_loaded(&file_path).await {
            send_down_msg(DownMsg::SignalTransitionsError {
                file_path,
                error: e,
            }, session_id, cor_id).await;
            return;
        }
    }
    
    let store = match WAVEFORM_DATA_STORE.lock() {
        Ok(store) => store,
        Err(_) => {
            send_down_msg(DownMsg::SignalTransitionsError {
                file_path,
                error: "Internal error: Failed to access waveform data store".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    let waveform_data = match store.get(&file_path) {
        Some(data) => data,
        None => {
            send_down_msg(DownMsg::SignalTransitionsError {
                file_path,
                error: "Waveform file not loaded or signal data not available".to_string(),
            }, session_id, cor_id).await;
            return;
        }
    };
    
    let mut results = Vec::new();
    
    for query in signal_queries {
        let key = format!("{}|{}", query.scope_path, query.variable_name);
        
        match waveform_data.signals.get(&key) {
            Some(&signal_ref) => {
                let mut transitions = Vec::new();
                
                // Convert time range from seconds back to native file units
                let (mut start_time, mut end_time) = match waveform_data.file_format {
                    wellen::FileFormat::Vcd => {
                        // For VCD: Convert from seconds back to VCD native units using stored timescale
                        // time_range is in seconds, need to convert to VCD native units
                        let start_native = (time_range.0 / waveform_data.timescale_factor) as u64;
                        let end_native = (time_range.1 / waveform_data.timescale_factor) as u64;
                        (start_native, end_native)
                    },
                    _ => {
                        // For other formats (like FST), use proper timescale conversion
                        ((time_range.0 / waveform_data.timescale_factor) as u64, (time_range.1 / waveform_data.timescale_factor) as u64)
                    }
                };
                
                // PERFORMANCE OPTIMIZATION: Clamp to actual file bounds to avoid processing non-existent time ranges
                // This prevents scanning millions of non-existent time points when timeline is zoomed out beyond file data
                if !waveform_data.time_table.is_empty() {
                    let file_start = *waveform_data.time_table.first().unwrap();
                    let file_end = *waveform_data.time_table.last().unwrap();
                    start_time = start_time.max(file_start);
                    end_time = end_time.min(file_end);
                    
                    println!("BACKEND CLAMP: Requested ({}, {}), file bounds ({}, {}), clamped to ({}, {})", 
                        (time_range.0 / waveform_data.timescale_factor) as u64,
                        (time_range.1 / waveform_data.timescale_factor) as u64,
                        file_start, file_end, start_time, end_time);
                }
                
                // Load signal once for efficiency
                let mut signal_source = match waveform_data.signal_source.lock() {
                    Ok(source) => source,
                    Err(_) => {
                        // Return empty results for this query on mutex error
                        results.push(SignalTransitionResult {
                            scope_path: query.scope_path,
                            variable_name: query.variable_name,
                            transitions: vec![],
                        });
                        continue;
                    }
                };
                let loaded_signals = signal_source.load_signals(&[signal_ref], &waveform_data.hierarchy, true);
                
                if let Some((_, signal)) = loaded_signals.into_iter().next() {
                    let mut last_value: Option<String> = None;
                    let mut last_transition_time: Option<f64> = None;
                    
                    // Use binary search to find time range bounds - much faster than linear iteration
                    let start_idx = match waveform_data.time_table.binary_search(&start_time) {
                        Ok(exact_idx) => exact_idx,
                        Err(insert_pos) => insert_pos,
                    };
                    
                    let end_idx = match waveform_data.time_table.binary_search(&end_time) {
                        Ok(exact_idx) => exact_idx + 1, // Include exact match
                        Err(insert_pos) => insert_pos,
                    };
                    
                    // PERFORMANCE OPTIMIZATION: Pixel-level decimation for dense signals
                    let transition_count = end_idx - start_idx;
                    let canvas_width = 1200.0; // Approximate canvas width in pixels
                    let max_useful_transitions = (canvas_width * 2.0) as usize; // 2 transitions per pixel maximum
                    
                    let decimation_step = if transition_count > max_useful_transitions {
                        let step = transition_count / max_useful_transitions;
                        println!("BACKEND DECIMATION: {} transitions -> sampling every {} for performance", transition_count, step);
                        step.max(1)
                    } else {
                        1 // No decimation needed
                    };
                    
                    // Only iterate through the relevant time slice, with optional decimation
                    let mut idx = start_idx;
                    while idx < end_idx.min(waveform_data.time_table.len()) {
                        if let Some(&time_val) = waveform_data.time_table.get(idx) {
                            // Convert time back to seconds for frontend using proper timescale
                            let time_seconds = match waveform_data.file_format {
                                wellen::FileFormat::Vcd => {
                                    // Convert VCD native units back to seconds using stored timescale
                                    time_val as f64 * waveform_data.timescale_factor
                                },
                                _ => time_val as f64 * waveform_data.timescale_factor,
                            };
                            
                            // Get signal value at this time index
                            if let Some(offset) = signal.get_offset(idx as u32) {
                                let value = signal.get_value_at(&offset, 0);
                                
                                // Convert to string for frontend display
                                let value_str = match signal_value_to_binary_string(&value) {
                                    Some(binary_str) => binary_str,
                                    None => format!("{}", value),
                                };
                                
                                // TRANSITION DETECTION: Only send when value actually changes
                                if last_value.as_ref() != Some(&value_str) {
                                    transitions.push(SignalTransition {
                                        time_seconds,
                                        value: value_str.clone(),
                                    });
                                    last_value = Some(value_str);
                                    last_transition_time = Some(time_seconds);
                                }
                            }
                        }
                        
                        idx += decimation_step; // Increment by decimation step for performance
                    }
                    
                    // Add filler rectangle: add "0" transition at the actual signal end time  
                    // This shows users where signal values end (e.g., A=c and B=5 end at 150s in simple.vcd)
                    if let (Some(last_val), Some(last_time)) = (&last_value, last_transition_time) {
                        if last_val != "0" {
                            // Calculate actual file end time for proper filler timing using proper timescale
                            let file_end_time_seconds = match waveform_data.file_format {
                                wellen::FileFormat::Vcd => {
                                    // Convert VCD native units to seconds using stored timescale
                                    end_time as f64 * waveform_data.timescale_factor
                                },
                                _ => end_time as f64 * waveform_data.timescale_factor,
                            };
                            
                            // Add "0" filler at actual signal end time (not viewing window end)
                            if last_time < file_end_time_seconds {
                                transitions.push(SignalTransition {
                                    time_seconds: file_end_time_seconds,
                                    value: "0".to_string(),
                                });
                            }
                        }
                    }
                }
                
                results.push(SignalTransitionResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    transitions,
                });
            }
            None => {
                results.push(SignalTransitionResult {
                    scope_path: query.scope_path,
                    variable_name: query.variable_name,
                    transitions: vec![],
                });
            }
        }
    }
    
    send_down_msg(DownMsg::SignalTransitions {
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
    // Set panic hook to log all panics 
    std::panic::set_hook(Box::new(|panic_info| {
        println!("BACKEND PANIC: {:?}", panic_info);
    }));
    
    println!("BACKEND: Starting NovyWave backend server...");
    
    start(frontend, up_msg_handler, |_error| {
        println!("BACKEND ERROR: Request processing error occurred");
    }).await
}
