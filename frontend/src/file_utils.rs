use crate::{FILE_PATHS_INPUT, SHOW_FILE_DIALOG, LOAD_FILES_VIEWPORT_Y, config::config_store};
use shared::UpMsg;
use zoon::{Task, Timer};


pub fn show_file_paths_dialog() {
    // Set both the global state AND the config store to work with sync system
    let mut dialogs = config_store().dialogs.current_value();
    dialogs.show_file_dialog = true;
    config_store().dialogs.set(dialogs);
    SHOW_FILE_DIALOG.set(true);
    FILE_PATHS_INPUT.set_neq(String::new());
    
    // SMART CACHE REFRESH - Request fresh data without clearing cache
    // This ensures users see newly added files without "Loading..." flicker
    // Fresh data will overwrite cached data when it arrives
    Task::start(async {
        use crate::platform::{Platform, CurrentPlatform};
        let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectory("/".to_string())).await;
        let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectory("~".to_string())).await;
    });
    
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
        let saved_scroll_position = crate::config::config_store().session.current_value().file_picker.scroll_position;
        
        // Set viewport Y to the saved scroll position
        LOAD_FILES_VIEWPORT_Y.set(saved_scroll_position);
    });
}

