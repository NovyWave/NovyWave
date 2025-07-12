use serde::{Serialize, Deserialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ===== MESSAGE TYPES =====

#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),
    GetParsingProgress(String),
    LoadConfig,
    SaveConfig(AppConfig),
    BrowseDirectory(String),
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
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WorkspaceSection {
    pub opened_files: Vec<String>,
    pub dock_mode: String,
    pub expanded_scopes: Vec<String>,
    pub selected_scope_id: Option<String>,
    pub docked_to_bottom: DockedToBottomLayout,
    pub docked_to_right: DockedToRightLayout,
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            dock_mode: "right".to_string(),
            expanded_scopes: Vec::new(),
            selected_scope_id: None,
            docked_to_bottom: Default::default(),
            docked_to_right: Default::default(),
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
        variables.to_vec()
    } else {
        let filter_lower = search_filter.to_lowercase();
        let mut filtered: Vec<Signal> = variables.iter()
            .filter(|var| var.name.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect();
        // CRITICAL: Ensure filtered variables remain alphabetically sorted
        filtered.sort_by(|a, b| a.name.cmp(&b.name));
        filtered
    }
}

pub fn get_all_variables_from_files(files: &[WaveformFile]) -> Vec<Signal> {
    let mut variables = Vec::new();
    for file in files {
        collect_variables_from_scopes(&file.scopes, &mut variables);
    }
    variables
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
            
            // TODO: Test these formats with actual files before enabling
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