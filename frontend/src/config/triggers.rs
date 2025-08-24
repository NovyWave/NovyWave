use crate::config::config_store;
use crate::state::*;
use crate::platform::{Platform, CurrentPlatform};
use shared::{UpMsg, DockMode, TrackedFile, FileState, create_tracked_file};
use indexmap::IndexSet;
use zoon::{Task, *};

/// One-shot config synchronization - runs once after config loads
/// 
/// This initializes global state from config values without creating
/// continuous reactive chains that could cause loops.
pub fn setup_one_time_config_sync() {
    sync_file_management_from_config();
    sync_ui_state_from_config();
    sync_panel_layout_from_config();
    sync_timeline_from_config();
    sync_session_state_from_config();
    sync_selection_from_config();
}

/// DEPRECATED: Replaced with one-shot initialization to prevent loops
#[allow(dead_code)]
pub fn setup_reactive_config_system() {
    // This function caused infinite loops due to bidirectional sync
    // Use setup_one_time_config_sync() instead
}

/// One-shot file management sync with state preservation
fn sync_file_management_from_config() {
    // Get config values once
    let file_paths = config_store().session.lock_ref().opened_files.lock_ref().to_vec();
    let selected_vars = config_store().workspace.lock_ref().selected_variables.lock_ref().to_vec();
    
    // Handle TRACKED_FILES with state preservation
    if !file_paths.is_empty() {
        let current_files = TRACKED_FILES.lock_ref().to_vec();
        
        // ✅ OPTIMIZED: Compute smart labels for all files first to avoid empty labels
        let smart_labels = shared::generate_smart_labels(&file_paths);
        zoon::println!("📋 [CONFIG SMART_LABELS] Computed smart labels for {} files: {:?}", file_paths.len(), smart_labels);
        
        let mut new_tracked_files = Vec::new();
        
        for file_path in file_paths {
            // Check if file already exists and preserve its state
            if let Some(existing_file) = current_files.iter().find(|f| f.path == file_path) {
                // Preserve existing file state (Loaded/Parsing/etc) but update smart label
                let mut updated_file = existing_file.clone();
                if updated_file.smart_label.is_empty() {
                    updated_file.smart_label = smart_labels.get(&file_path)
                        .unwrap_or(&updated_file.filename)
                        .clone();
                }
                new_tracked_files.push(updated_file);
            } else {
                // Create new file with Starting status and computed smart label
                // ✅ CRITICAL FIX: Use file path as ID to match backend expectations
                let mut new_file = create_tracked_file(file_path.clone(), FileState::Loading(shared::LoadingStatus::Starting));
                new_file.id = file_path.clone(); // Backend uses file path as ID
                new_file.smart_label = smart_labels.get(&file_path)
                    .unwrap_or(&new_file.filename)
                    .clone();
                zoon::println!("📁 [CONFIG DEBUG] Creating new file: id='{}', path='{}', smart_label='{}'", 
                    new_file.id, new_file.path, new_file.smart_label);
                new_tracked_files.push(new_file);
                
                // Send backend loading message for new files only
                Task::start({
                    let path = file_path.clone();
                    async move {
                        zoon::println!("📤 [CONFIG BACKEND] Sending LoadWaveformFile for: {}", path);
                        let _ = CurrentPlatform::send_message(UpMsg::LoadWaveformFile(path)).await;
                    }
                });
            }
        }
        
        // Only update if actually different to avoid unnecessary signals
        let current_paths: Vec<String> = current_files.iter().map(|f| f.path.clone()).collect();
        let new_paths: Vec<String> = new_tracked_files.iter().map(|f| f.path.clone()).collect();
        
        if current_paths != new_paths {
            zoon::println!("🔄 [TRACKED_FILES DEBUG] Config restore: replacing {} files with {} files", current_paths.len(), new_paths.len());
            TRACKED_FILES.lock_mut().replace_cloned(new_tracked_files);
        } else {
            zoon::println!("⏹️ [TRACKED_FILES DEBUG] Config restore: no change needed ({} files)", current_paths.len());
        }
    }
    
    // Handle SELECTED_VARIABLES (simple replacement - no state preservation needed)
    if !selected_vars.is_empty() {
        let index: IndexSet<String> = selected_vars.iter()
            .map(|var| var.unique_id.clone())
            .collect();
            
        SELECTED_VARIABLES.lock_mut().replace_cloned(selected_vars);
        SELECTED_VARIABLES_INDEX.set_neq(index);
    }
}

/// One-shot UI state sync
fn sync_ui_state_from_config() {
    // EXPANDED_SCOPES with scope_ prefix transform
    let expanded_scopes = config_store().workspace.lock_ref().expanded_scopes.lock_ref().to_vec();
    let mut expanded_set = IndexSet::new();
    for scope_id in expanded_scopes {
        if scope_id.contains('|') {
            expanded_set.insert(format!("scope_{}", scope_id));
        } else {
            expanded_set.insert(scope_id);
        }
    }
    EXPANDED_SCOPES.set_neq(expanded_set);
    
    // FILE_PICKER_EXPANDED
    let load_files_dirs = config_store().workspace.lock_ref().load_files_expanded_directories.lock_ref().to_vec();
    let expanded_set: IndexSet<String> = load_files_dirs.into_iter().collect();
    FILE_PICKER_EXPANDED.set_neq(expanded_set);

    // VARIABLES_SEARCH_FILTER
    let search_filter = config_store().session.lock_ref().variables_search_filter.get_cloned();
    VARIABLES_SEARCH_FILTER.set_neq(search_filter);
}

/// One-shot panel layout sync
fn sync_panel_layout_from_config() {
    // Get dock mode and set boolean
    let dock_mode = config_store().workspace.lock_ref().dock_mode.get_cloned();
    IS_DOCKED_TO_BOTTOM.set_neq(matches!(dock_mode, DockMode::Bottom));
    
    // Load appropriate dimensions for current dock mode
    let workspace = config_store().workspace.lock_ref();
    let layouts = workspace.panel_layouts.lock_ref();
    
    let (files_width, files_height, name_col_width, value_col_width) = match dock_mode {
        DockMode::Bottom => {
            let dims = layouts.docked_to_bottom.lock_ref();
            (dims.files_panel_width.get(), 
             dims.files_panel_height.get(),
             dims.variables_name_column_width.get(), 
             dims.variables_value_column_width.get())
        }
        DockMode::Right => {
            let dims = layouts.docked_to_right.lock_ref();
            (dims.files_panel_width.get(), 
             dims.files_panel_height.get(),
             dims.variables_name_column_width.get(), 
             dims.variables_value_column_width.get())
        }
    };
    
    FILES_PANEL_WIDTH.set_neq(files_width as u32);
    FILES_PANEL_HEIGHT.set_neq(files_height as u32);
    VARIABLES_NAME_COLUMN_WIDTH.set_neq(name_col_width as u32);
    VARIABLES_VALUE_COLUMN_WIDTH.set_neq(value_col_width as u32);
}

/// One-shot timeline state sync with NaN validation
fn sync_timeline_from_config() {
    let workspace = config_store().workspace.lock_ref();
    
    // Timeline cursor position with NaN validation
    let cursor_pos = workspace.timeline_cursor_position.get();
    if cursor_pos.is_finite() {
        TIMELINE_CURSOR_POSITION.set_neq(cursor_pos);
    }
    
    // Timeline zoom level with NaN validation  
    let zoom_level = workspace.timeline_zoom_level.get();
    if zoom_level.is_finite() {
        TIMELINE_ZOOM_LEVEL.set_neq(zoom_level);
    }
    
    // Timeline visible range with validation
    let range_start = workspace.timeline_visible_range_start.get();
    if range_start.is_finite() {
        TIMELINE_VISIBLE_RANGE_START.set_neq(range_start);
    }
    
    let range_end = workspace.timeline_visible_range_end.get();
    if range_end.is_finite() {
        TIMELINE_VISIBLE_RANGE_END.set_neq(range_end);
    }
}

/// One-shot session UI state sync
fn sync_session_state_from_config() {
    let session = config_store().session.lock_ref();
    let file_picker = session.file_picker.lock_ref();
    
    // CURRENT_DIRECTORY with validation
    if let Some(current_dir) = file_picker.current_directory.get_cloned() {
        // Validate directory exists before setting
        if std::path::Path::new(&current_dir).is_dir() {
            CURRENT_DIRECTORY.set_neq(current_dir);
        }
    }
    
    // LOAD_FILES_SCROLL_POSITION
    let scroll_pos = file_picker.scroll_position.get();
    LOAD_FILES_SCROLL_POSITION.set_neq(scroll_pos);
}

/// One-shot selection state sync
fn sync_selection_from_config() {
    // SELECTED_SCOPE_ID
    let selected_scope_id = config_store().workspace.lock_ref().selected_scope_id.get_cloned();
    SELECTED_SCOPE_ID.set_neq(selected_scope_id);
}