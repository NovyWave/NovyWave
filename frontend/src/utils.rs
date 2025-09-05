use zoon::*;
use crate::actors::global_domains::tracked_files_domain;
// Removed unused import: std::collections::HashSet

// Signal for completion state changes - triggers clearing of completed files
static LOADING_COMPLETION_TRIGGER: Lazy<Mutable<u32>> = Lazy::new(|| Mutable::new(0));

// Removed unused UI_UPDATE_SEQUENCE static




// Removed unused scope expansion functions - if needed in future, implement in proper Actor+Relay domain




// Initialize signal-based file clearing on loading completion
pub fn init_signal_chains() {
    // ✅ ACTOR+RELAY: Set up signal chain using domain signals
    Task::start(async {
        LOADING_COMPLETION_TRIGGER.signal().for_each(move |_| async move {
            // Clear files after completion state is visually confirmed
            Task::start(async {
                // ✅ ACTOR+RELAY: Clear completed loading files via domain event
                let tracked_files = tracked_files_domain();
                
                // Simple direct clearing instead of complex condition waiting
                // The loading completion trigger already indicates all files are done loading
                tracked_files.all_files_cleared_relay.send(());
            });
        }).await;
    });
}


