use crate::{FILE_PATHS_INPUT, SHOW_FILE_DIALOG, send_up_msg, IS_LOADING};
use shared::{UpMsg, generate_file_id};


pub fn show_file_paths_dialog() {
    SHOW_FILE_DIALOG.set(true);
    FILE_PATHS_INPUT.set_neq(String::new());
}

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