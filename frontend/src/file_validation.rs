// File Validation Service
// Monitors tracked files for changes in accessibility, existence, and state
// Automatically updates file states when issues are detected

use zoon::*;
use shared::{FileError, is_waveform_file, FileState};
use crate::state::{TRACKED_FILES, update_tracked_file_state};

/// Validate if a file path is accessible and supported
pub async fn validate_file_state(path: &str) -> Result<(), FileError> {
    // Note: In WASM environment, we cannot directly access the filesystem
    // File validation will need to be handled by the backend or through user interaction
    // For now, we implement basic format checking
    
    // Check if format is supported
    if !is_waveform_file(path) {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        return Err(FileError::UnsupportedFormat(extension.to_string()));
    }
    
    // In a real implementation with backend support:
    // - Check file existence via backend API
    // - Check file permissions via backend API
    // - Validate file is not corrupted via backend parsing attempt
    
    Ok(())
}

/// Create a periodic validation task that runs every 30 seconds
/// This monitors tracked files and updates their states if issues are detected
pub fn create_periodic_validation_task() {
    Task::start(async {
        loop {
            Timer::sleep(30000).await; // Check every 30 seconds
            validate_all_tracked_files().await;
        }
    });
}

/// Validate all currently tracked files and update states if needed
async fn validate_all_tracked_files() {
    let tracked_files = TRACKED_FILES.lock_ref().to_vec();
    
    for tracked_file in tracked_files {
        // Only validate files that are currently in loaded or loading states
        // Files already marked as failed/missing don't need re-validation
        match &tracked_file.state {
            FileState::Loaded(_) | FileState::Loading(_) => {
                // In a real implementation, this would make backend calls to check file status
                // For now, we just validate the format
                match validate_file_state(&tracked_file.path).await {
                    Ok(()) => {
                        // File is still valid - no action needed
                    }
                    Err(error) => {
                        // File has become invalid - update state
                        zoon::println!("File validation failed for {}: {:?}", tracked_file.path, error);
                        update_tracked_file_state(&tracked_file.id, FileState::Failed(error));
                    }
                }
            }
            FileState::Failed(_) | FileState::Missing(_) | FileState::Unsupported(_) => {
                // Files with error states - could potentially check if they've been fixed
                // For now, we leave them as-is unless user explicitly retries
            }
        }
    }
}


/// Initialize validation system - call this once during app startup
pub fn init_file_validation_system() {
    // File validation system starting
    
    // Start periodic validation task
    create_periodic_validation_task();
    
    // File validation system started
}



