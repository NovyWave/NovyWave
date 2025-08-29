use crate::config::config_store;
use crate::actors::dialog_manager::{open_file_dialog, change_dialog_viewport};
use shared::UpMsg;
use zoon::{Task, Timer};


pub fn show_file_paths_dialog() {
    // Set config store for sync system
    let mut dialogs = config_store().dialogs.current_value();
    dialogs.show_file_dialog = true;
    config_store().dialogs.set(dialogs);
    
    // Use domain function to open dialog and manage state
    open_file_dialog();
    
    // SMART CACHE REFRESH - Request fresh data without clearing cache
    // This ensures users see newly added files without "Loading..." flicker
    // Fresh data will overwrite cached data when it arrives
    Task::start(async {
        use crate::platform::{Platform, CurrentPlatform};
        let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectory("/".to_string())).await;
        let _ = CurrentPlatform::send_message(UpMsg::BrowseDirectory("~".to_string())).await;
    });
    
    // Restore scroll position from config
    Task::start(async {
        Timer::sleep(200).await;
        
        // Wait for config initialization to complete
        loop {
            if crate::CONFIG_INITIALIZATION_COMPLETE.get() {
                break;
            }
            Timer::sleep(50).await; // Check every 50ms
        }
        
        // Get saved scroll position directly from config store
        let saved_scroll_position = crate::config::config_store().session.current_value().file_picker.scroll_position;
        
        // Set viewport Y using domain function
        change_dialog_viewport(saved_scroll_position);
    });
}

