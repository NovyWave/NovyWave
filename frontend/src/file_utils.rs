use crate::{FILE_PATHS_INPUT, SHOW_FILE_DIALOG, send_up_msg, IS_LOADING, LOAD_FILES_VIEWPORT_Y, LOAD_FILES_SCROLL_POSITION};
use shared::{UpMsg, generate_file_id};
use zoon::{Task, Timer};


pub fn show_file_paths_dialog() {
    SHOW_FILE_DIALOG.set(true);
    FILE_PATHS_INPUT.set_neq(String::new());
    
    // Initialize file picker by browsing to filesystem root and user home directory
    // Note: TreeView will also request "/" if not cached, but that's handled automatically
    send_up_msg(UpMsg::BrowseDirectory("/".to_string()));
    send_up_msg(UpMsg::BrowseDirectory("~".to_string()));
    
    // Clear previous file picker selection but preserve expanded directories
    // Only ensure root "/" is expanded, keeping user's saved expanded folders
    crate::FILE_PICKER_SELECTED.lock_mut().clear();
    let mut expanded = crate::FILE_PICKER_EXPANDED.lock_mut();
    zoon::println!("📂 Dialog opening - current expanded directories: {:?}", expanded.iter().collect::<Vec<_>>());
    let was_empty = expanded.is_empty();
    let inserted = expanded.insert("/".to_string());
    zoon::println!("📂 Dialog opening - inserted '/' = {}, signal was empty = {}", inserted, was_empty);
    crate::FILE_PICKER_ERROR.set_neq(None);
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
        zoon::println!("📂 Restoring scroll position from config store: {}", saved_scroll_position);
        
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
    
    for path in paths {
        // Generate file ID and store path mapping for config persistence
        let file_id = generate_file_id(&path);
        crate::FILE_PATHS.lock_mut().insert(file_id, path.clone());
        
        send_up_msg(UpMsg::LoadWaveformFile(path));
    }
    
    SHOW_FILE_DIALOG.set(false);
}