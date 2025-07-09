use zoon::*;
use shared::{UpMsg, AppConfig, AppSection, UiSection, FilesSection, WorkspaceSection, DockedToBottomLayout, DockedToRightLayout, generate_file_id};
use crate::{
    send_up_msg, FILES_PANEL_WIDTH, FILES_PANEL_HEIGHT, IS_DOCKED_TO_BOTTOM, 
    SELECTED_SCOPE_ID, TREE_SELECTED_ITEMS, FILE_PATHS, EXPANDED_SCOPES
};


pub static LOADED_CONFIG: Lazy<Mutable<Option<AppConfig>>> = Lazy::new(|| {
    Mutable::new(None)
});

pub static CONFIG_LOADED: Lazy<Mutable<bool>> = Lazy::new(|| {
    Mutable::new(false)
});

pub fn apply_config(config: AppConfig) {
    let is_docked_to_bottom = config.workspace.dock_mode == "bottom";
    IS_DOCKED_TO_BOTTOM.set(is_docked_to_bottom);
    
    if is_docked_to_bottom {
        FILES_PANEL_WIDTH.set(config.workspace.docked_to_bottom.files_panel_width as u32);
        FILES_PANEL_HEIGHT.set(config.workspace.docked_to_bottom.files_panel_height as u32);
    } else {
        FILES_PANEL_WIDTH.set(config.workspace.docked_to_right.files_panel_width as u32);
        FILES_PANEL_HEIGHT.set(config.workspace.docked_to_right.files_panel_height as u32);
    }
    
    if let Some(selected_scope_id) = config.workspace.selected_scope_id.clone() {
        SELECTED_SCOPE_ID.set(Some(selected_scope_id));
    }
    
    
    {
        let mut expanded = EXPANDED_SCOPES.lock_mut();
        expanded.clear();
        for scope_id in config.workspace.expanded_scopes.iter() {
            if !scope_id.is_empty() && scope_id.starts_with("file_") && scope_id.len() < 100 {
                expanded.insert(scope_id.clone());
            }
        }
    }
    
    
    if config.app.auto_load_last_session {
        let file_paths = config.files.opened_files.clone();
        for file_path in file_paths {
            let file_id = generate_file_id(&file_path);
            FILE_PATHS.lock_mut().insert(file_id, file_path.clone());
            send_up_msg(UpMsg::LoadWaveformFile(file_path));
        }
    }
    
    LOADED_CONFIG.set(Some(config));
    CONFIG_LOADED.set(true);
}

pub fn save_current_config() {
    let loaded_config = LOADED_CONFIG.lock_ref();
    let is_docked_to_bottom = IS_DOCKED_TO_BOTTOM.get();
    
    let (docked_to_bottom, docked_to_right) = if let Some(config) = loaded_config.as_ref() {
        if is_docked_to_bottom {
            (
                DockedToBottomLayout {
                    files_panel_width: FILES_PANEL_WIDTH.get() as f64,
                    files_panel_height: FILES_PANEL_HEIGHT.get() as f64,
                },
                config.workspace.docked_to_right.clone()
            )
        } else {
            (
                config.workspace.docked_to_bottom.clone(),
                DockedToRightLayout {
                    files_panel_width: FILES_PANEL_WIDTH.get() as f64,
                    files_panel_height: FILES_PANEL_HEIGHT.get() as f64,
                }
            )
        }
    } else {
        (
            DockedToBottomLayout {
                files_panel_width: FILES_PANEL_WIDTH.get() as f64,
                files_panel_height: FILES_PANEL_HEIGHT.get() as f64,
            },
            DockedToRightLayout {
                files_panel_width: FILES_PANEL_WIDTH.get() as f64,
                files_panel_height: FILES_PANEL_HEIGHT.get() as f64,
            }
        )
    };
    
    let current_config = AppConfig {
        app: AppSection {
            version: "1.0.0".to_string(),
            auto_load_last_session: true,
        },
        ui: UiSection {
            theme: "dark".to_string(),
        },
        files: FilesSection {
            opened_files: FILE_PATHS.lock_ref().values().cloned().collect(),
        },
        workspace: WorkspaceSection {
            dock_mode: if is_docked_to_bottom { "bottom".to_string() } else { "right".to_string() },
            docked_to_bottom,
            docked_to_right,
            selected_scope_id: SELECTED_SCOPE_ID.get_cloned(),
            expanded_scopes: EXPANDED_SCOPES.lock_ref().iter().cloned().collect(),
        },
    };
    
    send_up_msg(UpMsg::SaveConfig(current_config));
}