use zoon::*;
use std::collections::{HashMap, HashSet};
use shared::{WaveformFile, LoadingFile, FileSystemItem};

// Panel resizing state
pub static FILES_PANEL_WIDTH: Lazy<Mutable<u32>> = Lazy::new(|| 470.into());
pub static FILES_PANEL_HEIGHT: Lazy<Mutable<u32>> = Lazy::new(|| 300.into());
pub static VERTICAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();
pub static HORIZONTAL_DIVIDER_DRAGGING: Lazy<Mutable<bool>> = lazy::default();

// Search filter for Variables panel
pub static VARIABLES_SEARCH_FILTER: Lazy<Mutable<String>> = lazy::default();

// Dock state management - DEFAULT TO DOCKED MODE  
pub static IS_DOCKED_TO_BOTTOM: Lazy<Mutable<bool>> = Lazy::new(|| Mutable::new(true));

// File dialog state
pub static SHOW_FILE_DIALOG: Lazy<Mutable<bool>> = lazy::default();
pub static FILE_PATHS_INPUT: Lazy<Mutable<String>> = lazy::default();

// Dock toggle state to prevent cascading saves
pub static DOCK_TOGGLE_IN_PROGRESS: Lazy<Mutable<bool>> = lazy::default();

// File picker state for TreeView-based browser
pub static FILE_PICKER_EXPANDED: Lazy<Mutable<HashSet<String>>> = lazy::default();
pub static FILE_PICKER_SELECTED: Lazy<MutableVec<String>> = lazy::default();
pub static CURRENT_DIRECTORY: Lazy<Mutable<String>> = lazy::default();
pub static FILE_PICKER_DATA: Lazy<MutableVec<FileSystemItem>> = lazy::default();
pub static FILE_PICKER_ERROR: Lazy<Mutable<Option<String>>> = lazy::default();


// Test viewport scrolling for Load Files dialog  
pub static LOAD_FILES_VIEWPORT_Y: Lazy<Mutable<i32>> = lazy::default();

// Load Files dialog scroll position (persistent)
pub static LOAD_FILES_SCROLL_POSITION: Lazy<Mutable<i32>> = lazy::default();

// Prevent config saves during initialization to avoid race conditions
pub static CONFIG_INITIALIZATION_COMPLETE: Lazy<Mutable<bool>> = lazy::default();

// Hierarchical file tree storage - maps directory path to its contents
pub static FILE_TREE_CACHE: Lazy<Mutable<HashMap<String, Vec<FileSystemItem>>>> = lazy::default();

// File loading progress state
pub static LOADING_FILES: Lazy<MutableVec<LoadingFile>> = lazy::default();
pub static IS_LOADING: Lazy<Mutable<bool>> = lazy::default();

// Loaded files hierarchy for TreeView
pub static LOADED_FILES: Lazy<MutableVec<WaveformFile>> = lazy::default();
pub static SELECTED_SCOPE_ID: Lazy<Mutable<Option<String>>> = lazy::default();
pub static TREE_SELECTED_ITEMS: Lazy<Mutable<HashSet<String>>> = lazy::default(); // UI state only - not persisted
pub static USER_CLEARED_SELECTION: Lazy<Mutable<bool>> = lazy::default(); // Flag to prevent unwanted restoration

// Track file ID to full path mapping for config persistence
pub static FILE_PATHS: Lazy<Mutable<HashMap<String, String>>> = lazy::default();

// Track expanded scopes for TreeView persistence
pub static EXPANDED_SCOPES: Lazy<Mutable<HashSet<String>>> = lazy::default();