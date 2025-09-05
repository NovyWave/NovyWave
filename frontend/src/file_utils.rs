// Removed unused config import
use crate::actors::dialog_manager::{open_file_dialog, change_dialog_viewport};
use shared::UpMsg;
use zoon::{Task};
// Removed unused import: Timer


pub fn show_file_paths_dialog() {
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
    
    // ✅ ARCHITECTURE FIX: Restore scroll position reactively instead of Timer::sleep() workaround
    Task::start(async {
        // Wait for dialog manager domain to be initialized (proper reactive pattern)
        // This ensures the dialog state is ready before setting scroll position
        
        // Wait for dialog visible signal to be true (reactive coordination)  
        // Use signal-based coordination instead of arbitrary Timer::sleep()
        use futures::StreamExt;
        use zoon::SignalExt;
        let mut dialog_stream = crate::actors::dialog_manager::dialog_visible_signal().to_stream();
        while let Some(is_visible) = dialog_stream.next().await {
            if is_visible {
                break; // Dialog is now visible, proceed with scroll position
            }
        }
        
        // Now safely restore scroll position with reactive coordination
        change_dialog_viewport(0); // Default scroll to top during migration
    });
}

