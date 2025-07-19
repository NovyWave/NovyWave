use serde::{Serialize, Deserialize};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

// ===== MESSAGE TYPES =====

#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),
    GetParsingProgress(String),
    LoadConfig,
    SaveConfig(AppConfig),
    BrowseDirectory(String),
    BrowseDirectories(Vec<String>), // Batch directory requests for parallel processing
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DownMsg {
    ParsingStarted { file_id: String, filename: String },
    ParsingProgress { file_id: String, progress: f32 },
    FileLoaded { file_id: String, hierarchy: FileHierarchy },
    ParsingError { file_id: String, error: String },
    ConfigLoaded(AppConfig),
    ConfigSaved,
    ConfigError(String),
    DirectoryContents { path: String, items: Vec<FileSystemItem> },
    DirectoryError { path: String, error: String },
    BatchDirectoryContents { results: HashMap<String, Result<Vec<FileSystemItem>, String>> }, // Parallel directory results
}

// ===== FILESYSTEM TYPES =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSystemItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub file_size: Option<u64>,
    pub is_waveform_file: bool,
    pub file_extension: Option<String>,
    pub has_expandable_content: bool,
}

// ===== CORE DATA TYPES =====

#[derive(Clone, Debug)]
pub struct LoadingFile {
    pub file_id: String,
    pub filename: String,
    pub progress: f32,
    pub status: LoadingStatus,
}

#[derive(Clone, Debug)]
pub enum LoadingStatus {
    Starting,
    Parsing,
    Completed,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileHierarchy {
    pub files: Vec<WaveformFile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WaveformFile {
    pub id: String,
    pub filename: String,
    pub format: FileFormat,
    pub scopes: Vec<ScopeData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FileFormat {
    VCD,
    FST,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScopeData {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub children: Vec<ScopeData>,
    pub variables: Vec<Signal>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub signal_type: String,
    pub width: u32,
}

// ===== ENHANCED FILE STATE TYPES =====

#[derive(Clone, Debug)]
pub enum FileError {
    ParseError(String),
    FileNotFound,
    PermissionDenied,
    UnsupportedFormat(String),
    CorruptedFile(String),
}

impl FileError {
    pub fn user_friendly_message(&self) -> String {
        match self {
            FileError::ParseError(msg) => msg.clone(),
            FileError::FileNotFound => "File not found".to_string(),
            FileError::PermissionDenied => "Permission denied".to_string(),
            FileError::UnsupportedFormat(format) => format!("Unsupported format: {}", format),
            FileError::CorruptedFile(msg) => format!("Corrupted file: {}", msg),
        }
    }
    
    pub fn icon_name(&self) -> &'static str {
        match self {
            FileError::ParseError(_) => "triangle-alert",
            FileError::FileNotFound => "file",
            FileError::PermissionDenied => "lock",
            FileError::UnsupportedFormat(_) => "circle-help",
            FileError::CorruptedFile(_) => "circle-alert",
        }
    }
}

#[derive(Clone, Debug)]
pub enum FileState {
    Loading(LoadingStatus),
    Loaded(WaveformFile),
    Failed(FileError),
    Missing(String), // file path
    Unsupported(String), // file path + reason
}

#[derive(Clone, Debug)]
pub struct TrackedFile {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub state: FileState,
    pub smart_label: String, // Generated from disambiguation algorithm
}

// ===== CONFIG TYPES =====

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AppConfig {
    pub app: AppSection,
    pub ui: UiSection,
    pub workspace: WorkspaceSection,
}

// AppSection contains configuration metadata, primarily for versioning and migration
// The version field enables proper config migration when the AppConfig format changes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppSection {
    pub version: String,
}

impl AppSection {
    /// Current configuration format version
    pub const CURRENT_VERSION: &'static str = "1.0.0";
    
    /// Check if this config version is supported
    pub fn is_supported_version(&self) -> bool {
        match self.version.as_str() {
            "1.0.0" => true,
            _ => false,
        }
    }
    
    /// Check if this config needs migration to current version
    pub fn needs_migration(&self) -> bool {
        self.version != Self::CURRENT_VERSION
    }
    
    /// Get migration path for unsupported versions
    pub fn get_migration_strategy(&self) -> MigrationStrategy {
        match self.version.as_str() {
            "1.0.0" => MigrationStrategy::None,
            // Future versions would be handled here:
            // "0.9.0" => MigrationStrategy::Upgrade("0.9.0 -> 1.0.0"),
            // When updating CURRENT_VERSION to "1.1.0", add:
            // "1.0.0" => MigrationStrategy::Upgrade("1.0.0 -> 1.1.0"),
            _ => MigrationStrategy::Recreate,
        }
    }
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationStrategy {
    None,                    // No migration needed
    Upgrade(String),         // Automatic upgrade with description
    Recreate,               // Unknown version, create new config
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UiSection {
    pub theme: String,
    #[serde(default = "default_toast_dismiss_ms")]
    pub toast_dismiss_ms: u64,
}

fn default_toast_dismiss_ms() -> u64 {
    10000 // Default 10 seconds
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            toast_dismiss_ms: 10000, // Default 10 seconds
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WorkspaceSection {
    pub opened_files: Vec<String>,
    pub dock_mode: String,
    pub expanded_scopes: Vec<String>,
    pub load_files_expanded_directories: Vec<String>,
    pub selected_scope_id: Option<String>,
    pub docked_to_bottom: DockedToBottomLayout,
    pub docked_to_right: DockedToRightLayout,
    pub load_files_scroll_position: i32,
    pub variables_search_filter: String,
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            dock_mode: "right".to_string(),
            expanded_scopes: Vec::new(),
            load_files_expanded_directories: Vec::new(),
            selected_scope_id: None,
            docked_to_bottom: Default::default(),
            docked_to_right: Default::default(),
            load_files_scroll_position: 0,
            variables_search_filter: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DockedToBottomLayout {
    pub files_panel_width: f64,
    pub files_panel_height: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DockedToRightLayout {
    pub files_panel_width: f64,
    pub files_panel_height: f64,
}

// ===== UTILITY FUNCTIONS =====

pub fn generate_file_id(file_path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    format!("file_{:x}", hasher.finish())
}

pub fn file_contains_scope(scopes: &[ScopeData], scope_id: &str) -> bool {
    for scope in scopes {
        if scope.id == scope_id {
            return true;
        }
        
        if file_contains_scope(&scope.children, scope_id) {
            return true;
        }
    }
    false
}

pub fn find_variables_in_scope(scopes: &[ScopeData], scope_id: &str) -> Option<Vec<Signal>> {
    for scope in scopes {
        if scope.id == scope_id {
            return Some(scope.variables.clone());
        }
        
        if let Some(variables) = find_variables_in_scope(&scope.children, scope_id) {
            return Some(variables);
        }
    }
    None
}

pub fn collect_variables_from_scopes(scopes: &[ScopeData], variables: &mut Vec<Signal>) {
    for scope in scopes {
        variables.extend(scope.variables.clone());
        collect_variables_from_scopes(&scope.children, variables);
    }
}

pub fn count_variables_in_scopes(scopes: &[ScopeData]) -> usize {
    let mut count = 0;
    for scope in scopes {
        count += scope.variables.len();
        count += count_variables_in_scopes(&scope.children);
    }
    count
}

pub fn filter_variables(variables: &[Signal], search_filter: &str) -> Vec<Signal> {
    if search_filter.is_empty() {
        variables.to_vec()  // Already sorted from backend
    } else {
        // Filter only, order preserved from backend sorting
        let filter_lower = search_filter.to_lowercase();
        variables.iter()
            .filter(|var| var.name.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect()
    }
}

pub fn get_all_variables_from_files(files: &[WaveformFile]) -> Vec<Signal> {
    let mut variables = Vec::new();
    for file in files {
        collect_variables_from_scopes(&file.scopes, &mut variables);
    }
    variables
}

// ===== SMART LABELING UTILITIES =====

/// Generate smart labels for file paths that minimize visual clutter while ensuring uniqueness
/// Files with unique names display as filename only, duplicates show disambiguating path segments
pub fn generate_smart_labels(file_paths: &[String]) -> HashMap<String, String> {
    let mut path_to_label = HashMap::new();
    let mut filename_to_paths: HashMap<String, Vec<String>> = HashMap::new();
    
    // Group paths by filename
    for path in file_paths {
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();
        filename_to_paths.entry(filename).or_default().push(path.clone());
    }
    
    for (filename, paths) in filename_to_paths {
        if paths.len() == 1 {
            // Unique filename - use as-is
            path_to_label.insert(paths[0].clone(), filename);
        } else {
            // Duplicate filenames - find minimal disambiguating segments
            let labels = find_minimal_disambiguation(&paths);
            for (path, label) in paths.iter().zip(labels) {
                path_to_label.insert(path.clone(), label);
            }
        }
    }
    
    path_to_label
}

/// Find minimal disambiguating path segments for a group of files with the same name
/// Uses shortest unique path suffix that distinguishes each file from others in the group
fn find_minimal_disambiguation(paths: &[String]) -> Vec<String> {
    let mut labels = Vec::new();
    
    for path in paths {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let filename = segments.last().map(|s| *s).unwrap_or(path.as_str());
        
        // Start with just filename, then add parent directories until unique
        let mut label = filename.to_string();
        for depth in 1..segments.len() {
            let start_idx = segments.len().saturating_sub(depth + 1);
            let suffix = segments[start_idx..].join("/");
            
            // Check if this suffix is unique among other paths
            let is_unique = paths.iter()
                .filter(|&other_path| other_path != path)
                .all(|other_path| !other_path.ends_with(&suffix));
                
            if is_unique || depth == segments.len() - 1 {
                label = suffix;
                break;
            }
        }
        
        labels.push(label);
    }
    
    labels
}

/// Create a TrackedFile from basic file information with initial state
pub fn create_tracked_file(file_path: String, state: FileState) -> TrackedFile {
    let file_id = generate_file_id(&file_path);
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&file_path)
        .to_string();
    
    TrackedFile {
        id: file_id,
        path: file_path,
        filename: filename.clone(),
        state,
        smart_label: filename, // Will be updated by smart labeling system
    }
}

/// Update smart labels for a collection of tracked files
pub fn update_smart_labels(tracked_files: &mut [TrackedFile]) {
    let paths: Vec<String> = tracked_files.iter().map(|f| f.path.clone()).collect();
    let smart_labels = generate_smart_labels(&paths);
    
    for tracked_file in tracked_files.iter_mut() {
        if let Some(smart_label) = smart_labels.get(&tracked_file.path) {
            tracked_file.smart_label = smart_label.clone();
        }
    }
}

// ===== FILESYSTEM UTILITIES =====

pub fn is_waveform_file(path: &str) -> bool {
    if let Some(extension) = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        match extension.to_lowercase().as_str() {
            // ✅ TESTED: Confirmed working with test files
            "vcd" | "fst" => true,
            
            // DISABLED: Additional waveform formats pending testing
            // Enable these once test files are available and parsing is verified:
            // "ghw" => true,  // GHDL waveform format
            // "vzt" => true,  // GTKWave compressed format  
            // "lxt" => true,  // GTKWave format
            // "lx2" => true,  // GTKWave format
            // "shm" => true,  // Cadence format
            
            _ => false,
        }
    } else {
        false
    }
}

pub fn get_file_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}