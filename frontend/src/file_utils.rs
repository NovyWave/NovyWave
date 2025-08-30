use crate::config::{self, DialogsData};
use crate::actors::dialog_manager::{open_file_dialog, change_dialog_viewport};
use shared::UpMsg;
use zoon::{Task, Timer};


pub fn show_file_paths_dialog() {
    // Update dialogs state through domain
    let dialogs = DialogsData {
        show_file_dialog: true,
    };
    config::app_config().dialogs_data_changed_relay.send(dialogs);
    
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
        
        // Config is loaded in main with await, so always ready
        
        // TODO: Implement proper reactive scroll position restoration
        // For now, use default scroll position during Actor+Relay migration
        change_dialog_viewport(0); // Default scroll to top
    });
}

