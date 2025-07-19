use crate::{FILE_PATHS_INPUT, SHOW_FILE_DIALOG, send_up_msg, IS_LOADING, LOAD_FILES_VIEWPORT_Y, config::config_store};
use crate::file_validation::validate_file_state;
use shared::{UpMsg, generate_file_id, FileState};
use zoon::{Task, Timer};


pub fn show_file_paths_dialog() {
    // Set both the global state AND the config store to work with sync system
    config_store().dialogs.lock_mut().show_file_dialog.set(true);
    SHOW_FILE_DIALOG.set(true);
    FILE_PATHS_INPUT.set_neq(String::new());
    
    // SMART CACHE REFRESH - Request fresh data without clearing cache
    // This ensures users see newly added files without "Loading..." flicker
    // Fresh data will overwrite cached data when it arrives
    send_up_msg(UpMsg::BrowseDirectory("/".to_string()));
    send_up_msg(UpMsg::BrowseDirectory("~".to_string()));
    
    // Clear previous file picker selection but preserve expanded directories
    // SMART DEFAULT EXPANSION - Only expand ~ when no previous expansion state exists
    crate::FILE_PICKER_SELECTED.lock_mut().clear();
    let mut expanded = crate::FILE_PICKER_EXPANDED.lock_mut();
    
    // Always ensure root "/" is expanded
    expanded.insert("/".to_string());
    
    // Smart default: only expand ~ if no previous expansion state exists
    // This provides good UX for first-time users without overriding user preferences
    if expanded.len() == 1 && expanded.contains("/") {
        expanded.insert("~".to_string());
    }
    crate::FILE_PICKER_ERROR.set_neq(None);
    // Don't clear error cache on dialog open - preserve errors until fresh data overwrites them
    crate::CURRENT_DIRECTORY.set_neq(String::new());
    
    // Restore scroll position from config
    Task::start(async {
        Timer::sleep(200).await;
        
        // Wait for config initialization to complete before accessing LOAD_FILES_SCROLL_POSITION
        // This prevents race condition where lazy static initializes with default value 0
        // instead of the loaded config value (e.g., 999)
        loop {
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                break;
            }
            Timer::sleep(50).await; // Check every 50ms
        }
        
        // Get saved scroll position directly from config store (not lazy static which may be stale)
        let saved_scroll_position = crate::config::config_store().session.lock_ref().file_picker.lock_ref().scroll_position.get();
        
        // Set viewport Y to the saved scroll position
        LOAD_FILES_VIEWPORT_Y.set(saved_scroll_position);
    });
}

#[allow(dead_code)]
pub fn process_file_paths() {
    let input = FILE_PATHS_INPUT.get_cloned();
    let paths: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    
    if !paths.is_empty() {
        IS_LOADING.set(true);
    }
    
    // CRITICAL: Validate files BEFORE sending to backend
    // This prevents non-existent files from being misclassified as UnsupportedFormat
    // and avoids wasting backend resources on invalid files
    for path in paths {
        Task::start({
            let path = path.clone();
            async move {
                // Validate file state before adding to tracked system
                match validate_file_state(&path).await {
                    Ok(()) => {
                        // File is valid - proceed with normal loading flow
                        crate::state::add_tracked_file(path.clone(), FileState::Loading(shared::LoadingStatus::Starting));
                        
                        // Also maintain legacy FILE_PATHS for backward compatibility during transition
                        let file_id = generate_file_id(&path);
                        crate::FILE_PATHS.lock_mut().insert(file_id, path.clone());
                        
                        send_up_msg(UpMsg::LoadWaveformFile(path));
                    },
                    Err(error) => {
                        // File validation failed - add with error state immediately, don't send to backend
                        zoon::println!("File validation failed for {}: {:?}", path, error);
                        crate::state::add_tracked_file(path.clone(), FileState::Failed(error));
                        
                        // Still add to legacy system for consistency, but mark as failed
                        let file_id = generate_file_id(&path);
                        crate::FILE_PATHS.lock_mut().insert(file_id, path.clone());
                        
                        // NOTE: We deliberately do NOT send UpMsg::LoadWaveformFile for failed validation
                        // This prevents the backend from wasting time on files we know are invalid
                    }
                }
            }
        });
    }
    
    SHOW_FILE_DIALOG.set(false);
}