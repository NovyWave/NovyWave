// File Validation Service
// Monitors tracked files for changes in accessibility, existence, and state
// Automatically updates file states when issues are detected

use zoon::*;
use shared::{FileError, is_waveform_file, FileState, TrackedFile};
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

/// Validate a specific file and return its current state
/// This can be called when user attempts to reload a file
pub async fn validate_single_file(file_path: &str) -> FileState {
    match validate_file_state(file_path).await {
        Ok(()) => {
            // File appears valid - set to loading state for re-parsing
            FileState::Loading(shared::LoadingStatus::Starting)
        }
        Err(error) => {
            FileState::Failed(error)
        }
    }
}

/// Initialize validation system - call this once during app startup
pub fn init_file_validation_system() {
    zoon::println!("Initializing file validation system...");
    
    // Start periodic validation task
    create_periodic_validation_task();
    
    zoon::println!("File validation system started - checking files every 30 seconds");
}

/// Manual file validation trigger (for user-initiated validation)
pub async fn trigger_manual_validation() {
    zoon::println!("Manual file validation triggered");
    validate_all_tracked_files().await;
}

/// Check if a file path appears to be valid format without deep validation
pub fn quick_format_check(path: &str) -> Result<(), FileError> {
    if !is_waveform_file(path) {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        return Err(FileError::UnsupportedFormat(extension.to_string()));
    }
    Ok(())
}

/// Restore a file from error state by attempting to reload it
pub async fn attempt_file_recovery(file_id: &str) {
    let tracked_files = TRACKED_FILES.lock_ref();
    
    if let Some(tracked_file) = tracked_files.iter().find(|f| f.id == file_id) {
        let path = tracked_file.path.clone();
        drop(tracked_files); // Release the lock
        
        zoon::println!("Attempting recovery for file: {}", path);
        
        let new_state = validate_single_file(&path).await;
        update_tracked_file_state(file_id, new_state.clone());
        
        // If validation passes, trigger actual file loading via backend
        if matches!(new_state, FileState::Loading(_)) {
            crate::send_up_msg(shared::UpMsg::LoadWaveformFile(path));
        }
    }
}