use serde::{Serialize, Deserialize, Deserializer};
use std::collections::HashMap;
use std::str::FromStr;

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub signal_type: String,
    pub width: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SelectedVariable {
    /// Human-readable identifier: "filename:scope_path:variable_name"
    /// Example: "simple.vcd:simple_tb.s:A"
    pub unique_id: String,
    pub file_name: String,
    pub scope_path: String,
    pub variable_name: String,
    pub variable_type: String,
    pub variable_width: u32,
    pub selected_at: u64,
}

impl SelectedVariable {
    pub fn new(variable: Signal, file_name: String, scope_full_name: String) -> Self {
        let unique_id = format!("{}:{}:{}", file_name, scope_full_name, variable.name);
        
        Self {
            unique_id,
            file_name,
            scope_path: scope_full_name,
            variable_name: variable.name,
            variable_type: variable.signal_type,
            variable_width: variable.width,
            selected_at: 0,
        }
    }
    
    pub fn new_with_timestamp(variable: Signal, file_name: String, scope_full_name: String, timestamp: u64) -> Self {
        let unique_id = format!("{}:{}:{}", file_name, scope_full_name, variable.name);
        
        Self {
            unique_id,
            file_name,
            scope_path: scope_full_name,
            variable_name: variable.name,
            variable_type: variable.signal_type,
            variable_width: variable.width,
            selected_at: timestamp,
        }
    }
    
    /// Create a Signal struct from this SelectedVariable (for backward compatibility)
    pub fn to_signal(&self) -> Signal {
        Signal {
            id: self.variable_name.clone(), // Use variable name as ID
            name: self.variable_name.clone(),
            signal_type: self.variable_type.clone(),
            width: self.variable_width,
        }
    }
    
    /// Get display name for UI purposes
    pub fn display_name(&self) -> String {
        format!("{}: {}.{}", self.file_name, self.scope_path, self.variable_name)
    }
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

// Type-safe theme handling with validation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

impl FromStr for Theme {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dark" => Ok(Theme::Dark),
            "light" => Ok(Theme::Light),
            _ => Err(format!("Invalid theme: '{}'. Valid themes are: dark, light", s)),
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Dark => write!(f, "dark"),
            Theme::Light => write!(f, "light"),
        }
    }
}

// Type-safe dock mode handling with validation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DockMode {
    Right,
    Bottom,
}

impl Default for DockMode {
    fn default() -> Self {
        DockMode::Right
    }
}

impl FromStr for DockMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "right" => Ok(DockMode::Right),
            "bottom" => Ok(DockMode::Bottom),
            _ => Err(format!("Invalid dock mode: '{}'. Valid modes are: right, bottom", s)),
        }
    }
}

impl std::fmt::Display for DockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DockMode::Right => write!(f, "right"),
            DockMode::Bottom => write!(f, "bottom"),
        }
    }
}

// Unified panel dimensions struct that handles frontend/backend differences
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PanelDimensions {
    pub width: f64,
    pub height: f64,
    // These fields are only used by frontend for more detailed layout control
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<f64>,
}

impl Default for PanelDimensions {
    fn default() -> Self {
        Self {
            width: 300.0,
            height: 200.0,
            min_width: None,
            min_height: None,
        }
    }
}

impl PanelDimensions {
    /// Create basic dimensions (backend usage)
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            min_width: None,
            min_height: None,
        }
    }
    
    /// Create dimensions with constraints (frontend usage)
    pub fn with_constraints(width: f64, height: f64, min_width: f64, min_height: f64) -> Self {
        Self {
            width,
            height,
            min_width: Some(min_width),
            min_height: Some(min_height),
        }
    }
    
    /// Convert to basic dimensions for backend compatibility
    pub fn to_basic(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}

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
    
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION.to_string(),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UiSection {
    #[serde(default, deserialize_with = "deserialize_theme")]
    pub theme: Theme,
    #[serde(default = "default_toast_dismiss_ms")]
    pub toast_dismiss_ms: u64,
}

fn default_toast_dismiss_ms() -> u64 {
    10000 // Default 10 seconds
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            toast_dismiss_ms: 10000, // Default 10 seconds
        }
    }
}

// Custom deserializer for theme with backward compatibility
fn deserialize_theme<'de, D>(deserializer: D) -> Result<Theme, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    Theme::from_str(&s).map_err(D::Error::custom)
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WorkspaceSection {
    #[serde(default)]
    pub opened_files: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_dock_mode")]
    pub dock_mode: DockMode,
    #[serde(default)]
    pub expanded_scopes: Vec<String>,
    #[serde(default)]
    pub load_files_expanded_directories: Vec<String>,
    #[serde(default)]
    pub selected_scope_id: Option<String>,
    #[serde(default)]
    pub panel_dimensions_bottom: PanelDimensions,
    #[serde(default)]
    pub panel_dimensions_right: PanelDimensions,
    #[serde(default)]
    pub load_files_scroll_position: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub variables_search_filter: String,
    #[serde(default)]
    pub selected_variables: Vec<SelectedVariable>,
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            dock_mode: DockMode::Right,
            expanded_scopes: Vec::new(),
            load_files_expanded_directories: Vec::new(),
            selected_scope_id: None,
            panel_dimensions_bottom: PanelDimensions::new(1400.0, 600.0), // Wide layout for bottom dock
            panel_dimensions_right: PanelDimensions::new(400.0, 300.0),   // Tall layout for right dock
            load_files_scroll_position: 0,
            variables_search_filter: String::new(),
            selected_variables: Vec::new(),
        }
    }
}

// Custom deserializer for dock mode with backward compatibility
fn deserialize_dock_mode<'de, D>(deserializer: D) -> Result<DockMode, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    DockMode::from_str(&s).map_err(D::Error::custom)
}


impl WorkspaceSection {
    /// Get current panel dimensions for the active dock mode
    pub fn get_panel_dimensions(&self) -> &PanelDimensions {
        match self.dock_mode {
            DockMode::Bottom => &self.panel_dimensions_bottom,
            DockMode::Right => &self.panel_dimensions_right,
        }
    }
    
    /// Update panel dimensions for the current dock mode
    pub fn set_panel_dimensions(&mut self, dimensions: PanelDimensions) {
        match self.dock_mode {
            DockMode::Bottom => self.panel_dimensions_bottom = dimensions,
            DockMode::Right => self.panel_dimensions_right = dimensions,
        }
    }

    /// Get panel dimensions for a specific dock mode
    pub fn get_panel_dimensions_for_mode(&self, dock_mode: DockMode) -> &PanelDimensions {
        match dock_mode {
            DockMode::Bottom => &self.panel_dimensions_bottom,
            DockMode::Right => &self.panel_dimensions_right,
        }
    }

    /// Update panel dimensions for a specific dock mode
    pub fn set_panel_dimensions_for_mode(&mut self, dock_mode: DockMode, dimensions: PanelDimensions) {
        match dock_mode {
            DockMode::Bottom => self.panel_dimensions_bottom = dimensions,
            DockMode::Right => self.panel_dimensions_right = dimensions,
        }
    }
}

// Validation methods for config integrity
impl AppConfig {
    /// Validate the entire configuration and fix any inconsistencies
    pub fn validate_and_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        
        // Validate theme
        // Theme validation is handled by the custom deserializer
        
        // Validate dock mode
        // Dock mode validation is handled by the custom deserializer
        
        // Validate panel dimensions for both dock modes
        if self.workspace.panel_dimensions_bottom.width < 50.0 {
            warnings.push("Bottom dock panel width too small, setting to minimum 50px".to_string());
            self.workspace.panel_dimensions_bottom.width = 50.0;
        }
        if self.workspace.panel_dimensions_bottom.height < 50.0 {
            warnings.push("Bottom dock panel height too small, setting to minimum 50px".to_string());
            self.workspace.panel_dimensions_bottom.height = 50.0;
        }
        if self.workspace.panel_dimensions_right.width < 50.0 {
            warnings.push("Right dock panel width too small, setting to minimum 50px".to_string());
            self.workspace.panel_dimensions_right.width = 50.0;
        }
        if self.workspace.panel_dimensions_right.height < 50.0 {
            warnings.push("Right dock panel height too small, setting to minimum 50px".to_string());
            self.workspace.panel_dimensions_right.height = 50.0;
        }
        
        // Validate toast dismiss time
        if self.ui.toast_dismiss_ms < 1000 {
            warnings.push("Toast dismiss time too short, setting to minimum 1 second".to_string());
            self.ui.toast_dismiss_ms = 1000;
        }
        if self.ui.toast_dismiss_ms > 300000 {
            warnings.push("Toast dismiss time too long, setting to maximum 5 minutes".to_string());
            self.ui.toast_dismiss_ms = 300000;
        }
        
        warnings
    }
    
    /// Create a config with validation applied
    pub fn new_validated() -> Self {
        let mut config = Self::default();
        let _warnings = config.validate_and_fix();
        config
    }
}

// ===== UTILITY FUNCTIONS =====

/// Generate a simple, human-readable file identifier from file path
/// Just uses the filename without extension as the identifier
pub fn generate_file_id(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown_file")
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_theme_serialization() {
        // Test enum serialization
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
        assert_eq!(serde_json::to_string(&Theme::Light).unwrap(), "\"light\"");
        
        // Test enum deserialization
        assert_eq!(serde_json::from_str::<Theme>("\"dark\"").unwrap(), Theme::Dark);
        assert_eq!(serde_json::from_str::<Theme>("\"light\"").unwrap(), Theme::Light);
        
        // Test case insensitive deserialization via custom deserializer
        let ui_section: UiSection = serde_json::from_str(r#"{"theme": "DARK"}"#).unwrap();
        assert_eq!(ui_section.theme, Theme::Dark);
    }
    
    #[test]
    fn test_dock_mode_serialization() {
        // Test enum serialization
        assert_eq!(serde_json::to_string(&DockMode::Right).unwrap(), "\"right\"");
        assert_eq!(serde_json::to_string(&DockMode::Bottom).unwrap(), "\"bottom\"");
        
        // Test enum deserialization
        assert_eq!(serde_json::from_str::<DockMode>("\"right\"").unwrap(), DockMode::Right);
        assert_eq!(serde_json::from_str::<DockMode>("\"bottom\"").unwrap(), DockMode::Bottom);
    }
    
    #[test]
    fn test_panel_dimensions_serialization() {
        let dims = PanelDimensions::new(300.0, 200.0);
        let json = serde_json::to_string(&dims).unwrap();
        let deserialized: PanelDimensions = serde_json::from_str(&json).unwrap();
        assert_eq!(dims, deserialized);
        
        // Test that optional fields are omitted when None (skip_serializing_if works)
        let basic_json = serde_json::to_value(&dims).unwrap();
        // When skip_serializing_if = "Option::is_none", the fields are omitted entirely
        assert!(!basic_json.as_object().unwrap().contains_key("min_width"));
        assert!(!basic_json.as_object().unwrap().contains_key("min_height"));
        
        // Test with constraints - fields should be present
        let dims_with_constraints = PanelDimensions::with_constraints(400.0, 300.0, 100.0, 80.0);
        let json_with_constraints = serde_json::to_value(&dims_with_constraints).unwrap();
        assert!(json_with_constraints.as_object().unwrap().contains_key("min_width"));
        assert!(json_with_constraints.as_object().unwrap().contains_key("min_height"));
        assert_eq!(json_with_constraints["min_width"], 100.0);
        assert_eq!(json_with_constraints["min_height"], 80.0);
    }
    
    
    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        
        // Set invalid values
        config.workspace.panel_dimensions.width = 10.0; // Too small
        config.workspace.panel_dimensions.height = 10.0; // Too small
        config.ui.toast_dismiss_ms = 500; // Too short
        
        let warnings = config.validate_and_fix();
        
        // Check that values were fixed
        assert_eq!(config.workspace.panel_dimensions.width, 50.0);
        assert_eq!(config.workspace.panel_dimensions.height, 50.0);
        assert_eq!(config.ui.toast_dismiss_ms, 1000);
        
        // Check that warnings were generated
        assert_eq!(warnings.len(), 3);
    }
    
}