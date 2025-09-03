use serde::{Serialize, Deserialize, Deserializer};
use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;
use convert_base;

// ===== TIME TYPES =====


// ===== MESSAGE TYPES =====

#[derive(Serialize, Deserialize, Debug)]
pub enum UpMsg {
    LoadWaveformFile(String),
    GetParsingProgress(String),
    LoadConfig,
    SaveConfig(AppConfig),
    BrowseDirectory(String),
    BrowseDirectories(Vec<String>), // Batch directory requests for parallel processing
    QuerySignalValues {
        file_path: String,
        queries: Vec<SignalValueQuery>,
    },
    BatchQuerySignalValues {
        batch_id: String,
        file_queries: Vec<FileSignalQueries>,
    },
    QuerySignalTransitions {
        file_path: String,
        signal_queries: Vec<SignalTransitionQuery>,
        time_range: (u64, u64),
    },
    /// Unified signal data query - serves both timeline and cursor value needs
    UnifiedSignalQuery {
        signal_requests: Vec<UnifiedSignalRequest>,
        cursor_time_ns: Option<u64>,
        request_id: String, // For deduplication and tracking
    },
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
    SignalValues {
        file_path: String,
        results: Vec<SignalValueResult>,
    },
    BatchSignalValues {
        batch_id: String,
        file_results: Vec<FileSignalResults>,
    },
    SignalValuesError {
        file_path: String,
        error: String,
    },
    SignalTransitions {
        file_path: String,
        results: Vec<SignalTransitionResult>,
    },
    SignalTransitionsError {
        file_path: String,
        error: String,
    },
    /// Unified signal data response with all requested information
    UnifiedSignalResponse {
        request_id: String,
        signal_data: Vec<UnifiedSignalData>,
        cursor_values: BTreeMap<String, SignalValue>,
        cached_time_range_ns: Option<(u64, u64)>, // What's available in backend cache
        statistics: Option<SignalStatistics>,
    },
    UnifiedSignalError {
        request_id: String,
        error: String,
    },
}

// ===== SIGNAL VALUE QUERY TYPES =====

/// Represents a signal value that can be either present or missing
/// This allows distinguishing between actual "0" values and missing/no-data regions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SignalValue {
    /// Signal has an actual value at this time
    Present(String),
    /// Signal has no data at this time (beyond file boundaries, gaps, etc.)
    Missing,
}

impl SignalValue {
    /// Create a present value
    pub fn present(value: impl Into<String>) -> Self {
        SignalValue::Present(value.into())
    }
    
    /// Create a missing value
    pub fn missing() -> Self {
        SignalValue::Missing
    }
    
    /// Check if the value is present
    pub fn is_present(&self) -> bool {
        matches!(self, SignalValue::Present(_))
    }
    
    /// Check if the value is missing
    pub fn is_missing(&self) -> bool {
        matches!(self, SignalValue::Missing)
    }
    
    /// Get the value as Option<String> for backward compatibility
    pub fn as_option(&self) -> Option<String> {
        match self {
            SignalValue::Present(value) => Some(value.clone()),
            SignalValue::Missing => None,
        }
    }
    
    /// Get the value or a placeholder for UI display
    pub fn display_value(&self, placeholder: &str) -> String {
        match self {
            SignalValue::Present(value) => value.clone(),
            SignalValue::Missing => placeholder.to_string(),
        }
    }
    
    /// Get the value with default "-" placeholder (matches UI conventions)
    pub fn display_value_or_dash(&self) -> String {
        self.display_value("-")
    }
    
    /// Check if this represents actual signal data (not missing)
    pub fn has_data(&self) -> bool {
        self.is_present()
    }
    
    /// Apply a format transformation to present values only
    pub fn map_present<F>(&self, f: F) -> SignalValue
    where
        F: FnOnce(&str) -> String,
    {
        match self {
            SignalValue::Present(value) => SignalValue::Present(f(value)),
            SignalValue::Missing => SignalValue::Missing,
        }
    }
}

impl From<Option<String>> for SignalValue {
    fn from(opt: Option<String>) -> Self {
        match opt {
            Some(value) => SignalValue::Present(value),
            None => SignalValue::Missing,
        }
    }
}

impl From<String> for SignalValue {
    fn from(value: String) -> Self {
        SignalValue::Present(value)
    }
}

impl From<&str> for SignalValue {
    fn from(value: &str) -> Self {
        SignalValue::Present(value.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalValueQuery {
    pub scope_path: String,
    pub variable_name: String,
    pub time_seconds: f64,
    pub format: VarFormat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalValueResult {
    pub scope_path: String,
    pub variable_name: String,
    pub time_seconds: f64,
    pub raw_value: Option<String>, // Raw binary value from waveform
    pub formatted_value: Option<String>, // Formatted according to requested format
    pub format: VarFormat, // Format used for this result
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSignalQueries {
    pub file_path: String,
    pub queries: Vec<SignalValueQuery>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSignalResults {
    pub file_path: String,
    pub results: Vec<SignalValueResult>,
}

// ===== SIGNAL TRANSITION QUERY TYPES =====

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalTransitionQuery {
    pub scope_path: String,
    pub variable_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalTransitionResult {
    pub scope_path: String,
    pub variable_name: String,
    pub transitions: Vec<SignalTransition>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignalTransition {
    pub time_ns: u64,
    pub value: String,
    
}

impl SignalTransition {
    /// Create new signal transition with nanosecond precision
    pub fn new(time_ns: u64, value: String) -> Self {
        Self {
            time_ns,
            value,
        }
    }
}

// ===== UNIFIED SIGNAL QUERY TYPES =====

/// Single request for signal data that can include both transitions and cursor values
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnifiedSignalRequest {
    pub file_path: String,
    pub scope_path: String,
    pub variable_name: String,
    pub time_range_ns: Option<(u64, u64)>, // None = all available data
    pub max_transitions: Option<usize>, // For downsampling large datasets
    pub format: VarFormat,
}

/// Response containing all requested signal data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnifiedSignalData {
    pub file_path: String,
    pub scope_path: String,
    pub variable_name: String,
    pub unique_id: String, // Computed unique identifier
    pub transitions: Vec<SignalTransition>,
    pub total_transitions: usize, // Before any downsampling
    pub actual_time_range_ns: Option<(u64, u64)>, // Actual data boundaries
}

/// Statistics about signals for performance optimization
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy)]
pub struct SignalStatistics {
    pub total_signals: usize,
    pub cached_signals: usize,
    pub query_time_ms: u64,
    pub cache_hit_ratio: f64,
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

#[derive(Clone, Debug, PartialEq)]
pub struct LoadingFile {
    pub file_id: String,
    pub filename: String,
    pub progress: f32,
    pub status: LoadingStatus,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WaveformFile {
    pub id: String,
    pub filename: String,
    pub format: FileFormat,
    pub scopes: Vec<ScopeData>,
    pub min_time_ns: Option<u64>,
    pub max_time_ns: Option<u64>,
    
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FileFormat {
    VCD,
    FST,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
    /// Pipe-separated identifier: "/full/path/file.vcd|scope_path|variable_name"
    /// Example: "/home/user/test_files/simple.vcd|simple_tb.s|A"
    pub unique_id: String,
    /// Format type for display - None uses default (Hexadecimal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatter: Option<VarFormat>,
}


impl SelectedVariable {
    pub fn new(variable: Signal, file_path: String, scope_full_name: String) -> Self {
        let unique_id = format!("{}|{}|{}", file_path, scope_full_name, variable.name);
        
        Self {
            unique_id,
            formatter: None, // Use default (Hexadecimal)
        }
    }
    
    pub fn new_with_formatter(variable: Signal, file_path: String, scope_full_name: String, formatter: VarFormat) -> Self {
        let unique_id = format!("{}|{}|{}", file_path, scope_full_name, variable.name);
        
        Self {
            unique_id,
            formatter: Some(formatter), // Explicitly set by user
        }
    }
    
    /// Parse the unique_id into its components
    pub fn parse_unique_id(&self) -> Option<(String, String, String)> {
        let parts: Vec<&str> = self.unique_id.splitn(3, '|').collect();
        if parts.len() == 3 {
            Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
        } else {
            None
        }
    }
    
    /// Get the file path component
    pub fn file_path(&self) -> Option<String> {
        self.parse_unique_id().map(|(path, _, _)| path)
    }
    
    /// Get the file name component
    pub fn file_name(&self) -> Option<String> {
        let path = self.file_path()?;
        let path_obj = std::path::Path::new(&path);
        path_obj
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
    }
    
    /// Get the scope path component
    pub fn scope_path(&self) -> Option<String> {
        self.parse_unique_id().map(|(_, scope, _)| scope)
    }
    
    /// Get the variable name component
    pub fn variable_name(&self) -> Option<String> {
        self.parse_unique_id().map(|(_, _, var)| var)
    }
    
    /// Create a Signal struct from this SelectedVariable (for backward compatibility)
    pub fn to_signal(&self) -> Option<Signal> {
        let (_, _, variable_name) = self.parse_unique_id()?;
        Some(Signal {
            id: variable_name.clone(),
            name: variable_name,
            signal_type: "Unknown".to_string(), // Type info not stored in new format
            width: 0, // Width info not stored in new format
        })
    }
    
    /// Get display name for UI purposes
    pub fn display_name(&self) -> String {
        if let Some((file_path, scope_path, variable_name)) = self.parse_unique_id() {
            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&file_path);
            format!("{}: {}.{}", file_name, scope_path, variable_name)
        } else {
            format!("Invalid: {}", self.unique_id)
        }
    }
}

// ===== VARIABLE FORMATTING =====

#[derive(Default, Clone, Copy, Debug, Serialize, PartialEq, Eq, Hash)]
#[serde(crate = "serde")]
pub enum VarFormat {
    ASCII,
    Binary,
    BinaryWithGroups,
    #[default]
    Hexadecimal,
    Octal,
    Signed,
    Unsigned,
}

// Custom deserializer for VarFormat with backward compatibility for "DEFAULT"
impl<'de> Deserialize<'de> for VarFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "ASCII" => Ok(VarFormat::ASCII),
            "Binary" => Ok(VarFormat::Binary),
            "BinaryWithGroups" => Ok(VarFormat::BinaryWithGroups),
            "Hexadecimal" => Ok(VarFormat::Hexadecimal),
            "Octal" => Ok(VarFormat::Octal),
            "Signed" => Ok(VarFormat::Signed),
            "Unsigned" => Ok(VarFormat::Unsigned),
            // Backward compatibility for old config files
            "DEFAULT" => Ok(VarFormat::Hexadecimal), // Map old DEFAULT to Hexadecimal
            _ => Err(D::Error::custom(format!(
                "unknown variant `{}`, expected one of `ASCII`, `Binary`, `BinaryWithGroups`, `Hexadecimal`, `Octal`, `Signed`, `Unsigned` (or legacy `DEFAULT`)", 
                s
            ))),
        }
    }
}

impl VarFormat {
    pub fn as_static_str(&self) -> &'static str {
        match self {
            VarFormat::ASCII => "ASCII",
            VarFormat::Binary => "Bin",
            VarFormat::BinaryWithGroups => "Bins",
            VarFormat::Hexadecimal => "Hex",
            VarFormat::Octal => "Oct",
            VarFormat::Signed => "Int",
            VarFormat::Unsigned => "UInt",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            VarFormat::ASCII => VarFormat::Binary,
            VarFormat::Binary => VarFormat::BinaryWithGroups,
            VarFormat::BinaryWithGroups => VarFormat::Hexadecimal,
            VarFormat::Hexadecimal => VarFormat::Octal,
            VarFormat::Octal => VarFormat::Signed,
            VarFormat::Signed => VarFormat::Unsigned,
            VarFormat::Unsigned => VarFormat::ASCII,
        }
    }

    /// Format a binary string value according to the selected format
    /// Expects the input to be a binary string (e.g., "10110101")
    pub fn format(&self, binary_value: &str) -> String {
        if binary_value.is_empty() {
            return binary_value.to_string();
        }

        match self {
            VarFormat::ASCII => {
                // ASCII requires at least 8 bits to form one character
                if binary_value.len() < 8 {
                    return "-".to_string();
                }
                
                let mut ascii_bytes = Vec::with_capacity(binary_value.len() / 8);
                for group_index in 0..binary_value.len() / 8 {
                    let offset = group_index * 8;
                    let group = &binary_value[offset..offset + 8];
                    match u8::from_str_radix(group, 2) {
                        Ok(byte_value) => {
                            // Handle NULL bytes (0x00) specially - keep them for now, we'll trim later
                            if byte_value == 0x00 {
                                ascii_bytes.push(0x00);
                            } else if byte_value.is_ascii() && (byte_value.is_ascii_graphic() || byte_value.is_ascii_whitespace()) {
                                ascii_bytes.push(byte_value);
                            } else {
                                // Non-printable or control character - replace with '?'
                                ascii_bytes.push(b'?');
                            }
                        }
                        Err(_) => {
                            // Invalid binary group - replace with '?' 
                            ascii_bytes.push(b'?');
                        }
                    }
                }
                
                // Strip trailing NULL bytes for cleaner display
                while let Some(&0x00) = ascii_bytes.last() {
                    ascii_bytes.pop();
                }
                
                // Now replace any remaining NULL bytes with '?' (embedded nulls)
                for byte in ascii_bytes.iter_mut() {
                    if *byte == 0x00 {
                        *byte = b'?';
                    }
                }
                
                // Convert to string - this should always succeed since we only push valid ASCII
                String::from_utf8(ascii_bytes).unwrap_or_else(|_| "?".to_string())
            }
            VarFormat::Binary => binary_value.to_string(),
            VarFormat::BinaryWithGroups => {
                let char_count = binary_value.len();
                binary_value
                    .chars()
                    .enumerate()
                    .fold(String::new(), |mut value, (index, one_or_zero)| {
                        value.push(one_or_zero);
                        let is_last = index == char_count - 1;
                        if !is_last && (index + 1) % 4 == 0 {
                            value.push(' ');
                        }
                        value
                    })
            }
            VarFormat::Hexadecimal => {
                let ones_and_zeros = binary_value
                    .chars()
                    .rev()
                    .map(|char| char.to_digit(2).unwrap_or(0))
                    .collect::<Vec<_>>();
                let mut base = convert_base::Convert::new(2, 16);
                let output = base.convert::<u32, u32>(&ones_and_zeros);
                let value: String = output
                    .into_iter()
                    .rev()
                    .map(|number| char::from_digit(number, 16).unwrap_or('?'))
                    .collect();
                // Remove leading zeros but keep at least one digit
                value.trim_start_matches('0').chars().next().map_or("0".to_string(), |_| {
                    let trimmed = value.trim_start_matches('0');
                    if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }
                })
            }
            VarFormat::Octal => {
                let ones_and_zeros = binary_value
                    .chars()
                    .rev()
                    .map(|char| char.to_digit(2).unwrap_or(0))
                    .collect::<Vec<_>>();
                let mut base = convert_base::Convert::new(2, 8);
                let output = base.convert::<u32, u32>(&ones_and_zeros);
                let value: String = output
                    .into_iter()
                    .rev()
                    .map(|number| char::from_digit(number, 8).unwrap_or('?'))
                    .collect();
                // Remove leading zeros but keep at least one digit
                let trimmed = value.trim_start_matches('0');
                if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }
            }
            VarFormat::Signed => {
                let mut ones_and_zeros = binary_value
                    .chars()
                    .rev()
                    .map(|char| char.to_digit(2).unwrap_or(0))
                    .collect::<Vec<_>>();

                // Two's complement conversion - https://builtin.com/articles/twos-complement
                let sign = if ones_and_zeros.last().unwrap_or(&0) == &0 {
                    ""
                } else {
                    "-"
                };
                if sign == "-" {
                    let mut one_found = false;
                    for one_or_zero in &mut ones_and_zeros {
                        if one_found {
                            *one_or_zero = if one_or_zero == &0 { 1 } else { 0 }
                        } else if one_or_zero == &1 {
                            one_found = true;
                        }
                    }
                }

                let mut base = convert_base::Convert::new(2, 10);
                let output = base.convert::<u32, u32>(&ones_and_zeros);
                let value_without_sign: String = output
                    .into_iter()
                    .rev()
                    .map(|number| char::from_digit(number, 10).unwrap_or('?'))
                    .collect();
                // Remove leading zeros but keep at least one digit for the number part
                let trimmed = value_without_sign.trim_start_matches('0');
                let number_part = if trimmed.is_empty() { "0" } else { trimmed };
                format!("{}{}", sign, number_part)
            }
            VarFormat::Unsigned => {
                let ones_and_zeros = binary_value
                    .chars()
                    .rev()
                    .map(|char| char.to_digit(2).unwrap_or(0))
                    .collect::<Vec<_>>();
                let mut base = convert_base::Convert::new(2, 10);
                let output = base.convert::<u32, u32>(&ones_and_zeros);
                let value: String = output
                    .into_iter()
                    .rev()
                    .map(|number| char::from_digit(number, 10).unwrap_or('?'))
                    .collect();
                // Remove leading zeros but keep at least one digit
                let trimmed = value.trim_start_matches('0');
                if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }
            }
        }
    }
}

// ===== ENHANCED FILE STATE TYPES =====

#[derive(Clone, Debug, PartialEq)]
pub enum FileError {
    /// Wellen parsing error with context
    ParseError { 
        source: String, // wellen error description
        context: String, // additional context (file path, format, etc.)
    },
    /// File not found at specified path
    FileNotFound { 
        path: String,
    },
    /// Permission denied accessing file
    PermissionDenied { 
        path: String,
    },
    /// Unsupported file format
    UnsupportedFormat { 
        path: String,
        extension: String,
        supported_formats: Vec<String>,
    },
    /// File appears corrupted or has invalid structure
    CorruptedFile { 
        path: String,
        details: String,
    },
    /// File is too large to process
    FileTooLarge { 
        path: String,
        size: u64,
        max_size: u64,
    },
    /// I/O error during file operations
    IoError { 
        path: String,
        error: String,
    },
    /// Invalid file content (not valid VCD/FST)
    InvalidFormat { 
        path: String,
        expected_format: String,
        reason: String,
    },
}

impl FileError {
    pub fn user_friendly_message(&self) -> String {
        match self {
            FileError::ParseError { source, context } => {
                format!("Failed to parse file: {} ({})", source, context)
            },
            FileError::FileNotFound { path } => {
                format!("File not found: {}", path)
            },
            FileError::PermissionDenied { path } => {
                format!("Permission denied accessing: {}", path)
            },
            FileError::UnsupportedFormat { path, extension, supported_formats } => {
                let supported = supported_formats.join(", ");
                format!("Unsupported format '{}' in file: {}. Supported formats: {}", 
                        extension, path, supported)
            },
            FileError::CorruptedFile { path, details } => {
                format!("Corrupted file: {} ({})", path, details)
            },
            FileError::FileTooLarge { path, size, max_size } => {
                format!("File too large: {} ({} bytes). Maximum size: {} bytes", 
                        path, size, max_size)
            },
            FileError::IoError { path, error } => {
                format!("I/O error reading: {} ({})", path, error)
            },
            FileError::InvalidFormat { path, expected_format, reason } => {
                format!("Invalid {} file: {} ({})", expected_format, path, reason)
            },
        }
    }
    
    pub fn icon_name(&self) -> &'static str {
        match self {
            FileError::ParseError { .. } => "triangle-alert",
            FileError::FileNotFound { .. } => "file",
            FileError::PermissionDenied { .. } => "lock",
            FileError::UnsupportedFormat { .. } => "circle-help",
            FileError::CorruptedFile { .. } => "circle-alert",
            FileError::FileTooLarge { .. } => "circle-alert",
            FileError::IoError { .. } => "triangle-alert",
            FileError::InvalidFormat { .. } => "circle-help",
        }
    }

    /// Get a short error category for logging/debugging
    pub fn category(&self) -> &'static str {
        match self {
            FileError::ParseError { .. } => "ParseError",
            FileError::FileNotFound { .. } => "FileNotFound",
            FileError::PermissionDenied { .. } => "PermissionDenied",
            FileError::UnsupportedFormat { .. } => "UnsupportedFormat",
            FileError::CorruptedFile { .. } => "CorruptedFile",
            FileError::FileTooLarge { .. } => "FileTooLarge",
            FileError::IoError { .. } => "IoError",
            FileError::InvalidFormat { .. } => "InvalidFormat",
        }
    }

    /// Get the file path associated with this error
    pub fn file_path(&self) -> &str {
        match self {
            FileError::ParseError { context, .. } => context, // Contains path
            FileError::FileNotFound { path } => path,
            FileError::PermissionDenied { path } => path,
            FileError::UnsupportedFormat { path, .. } => path,
            FileError::CorruptedFile { path, .. } => path,
            FileError::FileTooLarge { path, .. } => path,
            FileError::IoError { path, .. } => path,
            FileError::InvalidFormat { path, .. } => path,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileState {
    Loading(LoadingStatus),
    Loaded(WaveformFile),
    Failed(FileError),
    Missing(String), // file path
    Unsupported(String), // file path + reason
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedFile {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub state: FileState,
    pub smart_label: String, // Generated from disambiguation algorithm
}

// ===== CONFIG TYPES =====

// Type-safe theme handling with validation
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
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
    // Selected Variables panel column widths
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables_name_column_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables_value_column_width: Option<f64>,
}

// Docked bottom dimensions with semantic field names
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DockedBottomDimensions {
    pub files_and_scopes_panel_width: f64,
    pub files_and_scopes_panel_height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_variables_panel_name_column_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_variables_panel_value_column_width: Option<f64>,
}

// Docked right dimensions with semantic field names
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DockedRightDimensions {
    pub files_and_scopes_panel_width: f64,
    pub files_and_scopes_panel_height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_variables_panel_name_column_width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_variables_panel_value_column_width: Option<f64>,
}

impl Default for PanelDimensions {
    fn default() -> Self {
        Self {
            width: 300.0,
            height: 200.0,
            min_width: None,
            min_height: None,
            variables_name_column_width: None,
            variables_value_column_width: None,
        }
    }
}

impl Default for DockedBottomDimensions {
    fn default() -> Self {
        Self {
            files_and_scopes_panel_width: 1400.0,
            files_and_scopes_panel_height: 600.0,
            selected_variables_panel_name_column_width: None,
            selected_variables_panel_value_column_width: None,
        }
    }
}

impl Default for DockedRightDimensions {
    fn default() -> Self {
        Self {
            files_and_scopes_panel_width: 400.0,
            files_and_scopes_panel_height: 300.0,
            selected_variables_panel_name_column_width: None,
            selected_variables_panel_value_column_width: None,
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
            variables_name_column_width: None,
            variables_value_column_width: None,
        }
    }
    
    /// Create dimensions with constraints (frontend usage)
    pub fn with_constraints(width: f64, height: f64, min_width: f64, min_height: f64) -> Self {
        Self {
            width,
            height,
            min_width: Some(min_width),
            min_height: Some(min_height),
            variables_name_column_width: None,
            variables_value_column_width: None,
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

fn default_timeline_cursor_position_ns() -> u64 {
    10_000_000_000 // Default timeline cursor position: 10 seconds in nanoseconds
}


fn default_timeline_zoom_level() -> f32 {
    1.0 // Default zoom level (1.0 = normal, no zoom)
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
    pub docked_bottom_dimensions: DockedBottomDimensions,
    #[serde(default)]
    pub docked_right_dimensions: DockedRightDimensions,
    #[serde(default)]
    pub load_files_scroll_position: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub variables_search_filter: String,
    #[serde(default)]
    pub selected_variables: Vec<SelectedVariable>,
    // Timeline cursor position in nanoseconds
    #[serde(default = "default_timeline_cursor_position_ns")]
    pub timeline_cursor_position_ns: u64,
    
    // Timeline zoom level (deprecated - replaced by NsPerPixel in frontend)
    #[serde(default = "default_timeline_zoom_level")]
    pub timeline_zoom_level: f32,
    
    // Timeline visible range in nanoseconds
    #[serde(default)]
    pub timeline_visible_range_start_ns: Option<u64>,
    #[serde(default)]
    pub timeline_visible_range_end_ns: Option<u64>,
    
}

impl Default for WorkspaceSection {
    fn default() -> Self {
        Self {
            opened_files: Vec::new(),
            dock_mode: DockMode::Right,
            expanded_scopes: Vec::new(),
            load_files_expanded_directories: Vec::new(),
            selected_scope_id: None,
            docked_bottom_dimensions: DockedBottomDimensions::default(),
            docked_right_dimensions: DockedRightDimensions::default(),
            load_files_scroll_position: 0,
            variables_search_filter: String::new(),
            selected_variables: Vec::new(),
            // New nanosecond fields
            timeline_cursor_position_ns: default_timeline_cursor_position_ns(),
            timeline_visible_range_start_ns: None,
            timeline_visible_range_end_ns: None,
            
            timeline_zoom_level: default_timeline_zoom_level(),
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
    /// Get current docked bottom dimensions
    pub fn get_docked_bottom_dimensions(&self) -> &DockedBottomDimensions {
        &self.docked_bottom_dimensions
    }
    
    /// Get current docked right dimensions
    pub fn get_docked_right_dimensions(&self) -> &DockedRightDimensions {
        &self.docked_right_dimensions
    }
    
    /// Update docked bottom dimensions
    pub fn set_docked_bottom_dimensions(&mut self, dimensions: DockedBottomDimensions) {
        self.docked_bottom_dimensions = dimensions;
    }
    
    /// Update docked right dimensions  
    pub fn set_docked_right_dimensions(&mut self, dimensions: DockedRightDimensions) {
        self.docked_right_dimensions = dimensions;
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
        
        // Validate docked dimensions for both dock modes
        if self.workspace.docked_bottom_dimensions.files_and_scopes_panel_width < 50.0 {
            warnings.push("Bottom dock files and scopes panel width too small, setting to minimum 50px".to_string());
            self.workspace.docked_bottom_dimensions.files_and_scopes_panel_width = 50.0;
        }
        if self.workspace.docked_bottom_dimensions.files_and_scopes_panel_height < 50.0 {
            warnings.push("Bottom dock files and scopes panel height too small, setting to minimum 50px".to_string());
            self.workspace.docked_bottom_dimensions.files_and_scopes_panel_height = 50.0;
        }
        if self.workspace.docked_right_dimensions.files_and_scopes_panel_width < 50.0 {
            warnings.push("Right dock files and scopes panel width too small, setting to minimum 50px".to_string());
            self.workspace.docked_right_dimensions.files_and_scopes_panel_width = 50.0;
        }
        if self.workspace.docked_right_dimensions.files_and_scopes_panel_height < 50.0 {
            warnings.push("Right dock files and scopes panel height too small, setting to minimum 50px".to_string());
            self.workspace.docked_right_dimensions.files_and_scopes_panel_height = 50.0;
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
        
        // Migrate expanded_scopes from old format to new format
        warnings.extend(self.migrate_expanded_scopes());
        
        warnings
    }
    
    /// Migrate expanded_scopes from old format ({file_id}_{scope}) to new format ({full_path}|{scope})
    fn migrate_expanded_scopes(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut migrated_scopes = Vec::new();
        let mut migration_occurred = false;
        
        for scope_id in &self.workspace.expanded_scopes {
            if self.is_old_scope_format(scope_id) {
                if let Some(migrated_id) = self.convert_to_new_format(scope_id) {
                    warnings.push(format!("Migrated scope ID: '{}' -> '{}'", scope_id, migrated_id));
                    migrated_scopes.push(migrated_id);
                    migration_occurred = true;
                } else {
                    warnings.push(format!("Could not migrate scope ID '{}' - keeping old format", scope_id));
                    migrated_scopes.push(scope_id.clone());
                }
            } else {
                // Already in new format or file-only entry
                migrated_scopes.push(scope_id.clone());
            }
        }
        
        if migration_occurred {
            self.workspace.expanded_scopes = migrated_scopes;
            warnings.push("Expanded scopes format migration completed".to_string());
        }
        
        warnings
    }
    
    /// Check if scope ID is in old format
    fn is_old_scope_format(&self, scope_id: &str) -> bool {
        // New format: {full_path}|{scope} or just {full_path} (starting with /)
        if scope_id.contains('|') || scope_id.starts_with('/') {
            return false;
        }
        
        // Old format includes:
        // 1. {file_id}_{scope} entries (with underscore separator)
        // 2. Sanitized file-only entries that match generate_file_id() output
        //    (e.g., "home_user_path_file.ext" should become "/home/user/path/file.ext")
        
        // Check if it matches generate_file_id() output - if so, it's old sanitized format
        for opened_file_path in &self.workspace.opened_files {
            let generated_file_id = generate_file_id(opened_file_path);
            if generated_file_id == scope_id {
                // This is an old sanitized file-only entry that needs migration
                return true;
            }
        }
        
        // Check if it contains underscore (old scope format like "file_scope")
        if scope_id.contains('_') {
            return true;
        }
        
        // If we get here, it's likely an orphaned entry - don't migrate
        false
    }
    
    /// Convert old format scope ID to new format using opened_files as lookup
    fn convert_to_new_format(&self, old_scope_id: &str) -> Option<String> {
        // First check if this is a sanitized file-only entry (direct match with generate_file_id)
        for opened_file_path in &self.workspace.opened_files {
            let generated_file_id = generate_file_id(opened_file_path);
            if generated_file_id == old_scope_id {
                // This is a sanitized file-only entry - convert to full path
                return Some(opened_file_path.clone());
            }
        }
        
        // Parse old format: {file_id}_{scope} or {file_id}
        let parts: Vec<&str> = old_scope_id.splitn(2, '_').collect();
        if parts.is_empty() {
            return None;
        }
        
        let file_id = parts[0];
        let scope_name = if parts.len() > 1 { Some(parts[1]) } else { None };
        
        // Find matching file in opened_files by looking for files whose generated ID matches
        for opened_file_path in &self.workspace.opened_files {
            let generated_file_id = generate_file_id(opened_file_path);
            
            // Check for file_id match (for entries like "wave_27.fst_TOP")
            if generated_file_id == file_id {
                // Found matching file - construct new format
                return Some(match scope_name {
                    Some(scope) => format!("{}|{}", opened_file_path, scope),
                    None => opened_file_path.clone(), // File-only entry
                });
            }
            
            // Check legacy format (filename-only ID) for backwards compatibility
            let filename = std::path::Path::new(opened_file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if filename == file_id {
                // Found matching file using legacy filename-only matching
                return Some(match scope_name {
                    Some(scope) => format!("{}|{}", opened_file_path, scope),
                    None => opened_file_path.clone(), // File-only entry
                });
            }
        }
        
        // Could not find matching file
        None
    }
    
    /// Create a config with validation applied
    pub fn new_validated() -> Self {
        let mut config = Self::default();
        let _warnings = config.validate_and_fix();
        config
    }
}

// ===== UTILITY FUNCTIONS =====

/// Generate a unique file identifier from the full file path
/// Uses the sanitized full path to ensure uniqueness and readability
pub fn generate_file_id(file_path: &str) -> String {
    let sanitized = sanitize_path_for_id(file_path);
    
    // For extremely long paths (>255 chars), truncate but preserve uniqueness
    // This is a reasonable limit for HTML IDs and config keys
    if sanitized.len() > 255 {
        // Keep first 200 chars + hash of full path for uniqueness
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        sanitized.hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{}_{:x}", &sanitized[..200], hash)
    } else {
        sanitized
    }
}

/// Sanitize a file path for use in scope IDs and other identifiers
/// Replaces path separators and other problematic characters with underscores
/// Ensures the result is suitable for use in HTML IDs and configuration keys
pub fn sanitize_path_for_id(path: &str) -> String {
    path.chars()
        .map(|c| match c {
            // Replace path separators
            '/' | '\\' => '_',
            // Replace other problematic characters that might cause issues in IDs
            ':' | ' ' | '\t' | '\n' | '\r' => '_',
            // Replace characters that might be problematic in HTML IDs or config keys
            '<' | '>' | '"' | '\'' | '&' | '?' | '*' | '|' => '_',
            // Keep alphanumeric, dots, hyphens, and underscores
            c if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' => c,
            // Replace anything else with underscore
            _ => '_',
        })
        .collect::<String>()
        // Remove multiple consecutive underscores for cleaner IDs
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("_")
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
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&file_path)
        .to_string();
    
    TrackedFile {
        id: file_path.clone(), // Use full path as ID for new format consistency
        path: file_path,
        filename: filename.clone(),
        state,
        smart_label: String::new(), // Unused - smart labels computed by derived signal
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
        config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width = 10.0; // Too small
        config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height = 10.0; // Too small
        config.workspace.docked_right_dimensions.files_and_scopes_panel_width = 10.0; // Too small 
        config.workspace.docked_right_dimensions.files_and_scopes_panel_height = 10.0; // Too small
        config.ui.toast_dismiss_ms = 500; // Too short
        
        let warnings = config.validate_and_fix();
        
        // Check that values were fixed
        assert_eq!(config.workspace.docked_bottom_dimensions.files_and_scopes_panel_width, 50.0);
        assert_eq!(config.workspace.docked_bottom_dimensions.files_and_scopes_panel_height, 50.0);
        assert_eq!(config.workspace.docked_right_dimensions.files_and_scopes_panel_width, 50.0);
        assert_eq!(config.workspace.docked_right_dimensions.files_and_scopes_panel_height, 50.0);
        assert_eq!(config.ui.toast_dismiss_ms, 1000);
        
        // Check that warnings were generated
        assert_eq!(warnings.len(), 5);
    }
    
    #[test]
    fn test_var_format_serialization() {
        // Test enum serialization
        assert_eq!(serde_json::to_string(&VarFormat::Hexadecimal).unwrap(), "\"Hexadecimal\"");
        assert_eq!(serde_json::to_string(&VarFormat::Binary).unwrap(), "\"Binary\"");
        
        // Test enum deserialization
        assert_eq!(serde_json::from_str::<VarFormat>("\"Binary\"").unwrap(), VarFormat::Binary);
        assert_eq!(serde_json::from_str::<VarFormat>("\"Hexadecimal\"").unwrap(), VarFormat::Hexadecimal);
        
        // Test default
        assert_eq!(VarFormat::default(), VarFormat::Hexadecimal);
    }
    
    #[test]
    fn test_var_format_as_static_str() {
        assert_eq!(VarFormat::ASCII.as_static_str(), "ASCII");
        assert_eq!(VarFormat::Binary.as_static_str(), "Bin");
        assert_eq!(VarFormat::BinaryWithGroups.as_static_str(), "Bins");
        assert_eq!(VarFormat::Hexadecimal.as_static_str(), "Hex");
        assert_eq!(VarFormat::Octal.as_static_str(), "Oct");
        assert_eq!(VarFormat::Signed.as_static_str(), "Int");
        assert_eq!(VarFormat::Unsigned.as_static_str(), "UInt");
    }
    
    #[test]
    fn test_var_format_next() {
        assert_eq!(VarFormat::ASCII.next(), VarFormat::Binary);
        assert_eq!(VarFormat::Binary.next(), VarFormat::BinaryWithGroups);
        assert_eq!(VarFormat::BinaryWithGroups.next(), VarFormat::Hexadecimal);
        assert_eq!(VarFormat::Hexadecimal.next(), VarFormat::Octal);
        assert_eq!(VarFormat::Octal.next(), VarFormat::Signed);
        assert_eq!(VarFormat::Signed.next(), VarFormat::Unsigned);
        assert_eq!(VarFormat::Unsigned.next(), VarFormat::ASCII);
    }
    
    #[test]
    fn test_var_format_binary_formatting() {
        let binary_value = "10110101";
        
        // Test binary format (should be unchanged)
        assert_eq!(VarFormat::Binary.format(binary_value), "10110101");
        
        // Test binary with groups
        assert_eq!(VarFormat::BinaryWithGroups.format(binary_value), "1011 0101");
        
        // Test empty string
        assert_eq!(VarFormat::Binary.format(""), "");
        assert_eq!(VarFormat::Hexadecimal.format(""), "");
    }
    
    #[test]
    fn test_var_format_hexadecimal() {
        let binary_value = "10110101"; // Should be B5 in hex
        assert_eq!(VarFormat::Hexadecimal.format(binary_value), "b5");
        
        let binary_value2 = "11111111"; // Should be FF in hex
        assert_eq!(VarFormat::Hexadecimal.format(binary_value2), "ff");
        
        let binary_value3 = "00001010"; // Should be 0A in hex
        assert_eq!(VarFormat::Hexadecimal.format(binary_value3), "a");
    }
    
    #[test]
    fn test_var_format_octal() {
        let binary_value = "10110101"; // Should be 265 in octal
        assert_eq!(VarFormat::Octal.format(binary_value), "265");
        
        let binary_value2 = "11111111"; // Should be 377 in octal
        assert_eq!(VarFormat::Octal.format(binary_value2), "377");
    }
    
    #[test]
    fn test_var_format_unsigned() {
        let binary_value = "10110101"; // Should be 181 in decimal
        assert_eq!(VarFormat::Unsigned.format(binary_value), "181");
        
        let binary_value2 = "11111111"; // Should be 255 in decimal
        assert_eq!(VarFormat::Unsigned.format(binary_value2), "255");
        
        let binary_value3 = "00000000"; // Should be 0 in decimal
        assert_eq!(VarFormat::Unsigned.format(binary_value3), "0");
    }
    
    #[test]
    fn test_var_format_signed() {
        // Positive numbers (MSB = 0)
        let binary_value = "01111111"; // Should be 127 in signed decimal
        assert_eq!(VarFormat::Signed.format(binary_value), "127");
        
        // Negative numbers (MSB = 1) - two's complement
        let binary_value2 = "10000001"; // Should be -127 in signed decimal
        assert_eq!(VarFormat::Signed.format(binary_value2), "-127");
        
        let binary_value3 = "11111111"; // Should be -1 in signed decimal
        assert_eq!(VarFormat::Signed.format(binary_value3), "-1");
        
        let binary_value4 = "10000000"; // Should be -128 in signed decimal
        assert_eq!(VarFormat::Signed.format(binary_value4), "-128");
    }
    
    #[test]
    fn test_var_format_ascii() {
        // Test ASCII conversion - 8-bit groups
        let binary_value = "0100100001000101"; // Should be "HE" (0x48, 0x45)
        assert_eq!(VarFormat::ASCII.format(binary_value), "HE");
        
        // Test single character
        let binary_value2 = "01001000"; // Should be "H" (0x48)
        assert_eq!(VarFormat::ASCII.format(binary_value2), "H");
        
        // Test incomplete group (should be ignored)
        let binary_value3 = "0100100001000101010"; // 17 bits, only first 16 used
        assert_eq!(VarFormat::ASCII.format(binary_value3), "HE");
        
        // Test binary values shorter than 8 bits should show placeholder
        assert_eq!(VarFormat::ASCII.format("0"), "-");
        assert_eq!(VarFormat::ASCII.format("1"), "-");
        assert_eq!(VarFormat::ASCII.format("101"), "-");
        assert_eq!(VarFormat::ASCII.format("1010101"), "-"); // 7 bits
        
        // Test 9+ bits processes complete 8-bit groups, ignores remainder
        assert_eq!(VarFormat::ASCII.format("010010000"), "H"); // 9 bits, processes first 8
        
        // Test control characters get replaced with '?'
        assert_eq!(VarFormat::ASCII.format("00000000"), "?"); // NULL character
        assert_eq!(VarFormat::ASCII.format("00000001"), "?"); // SOH control character
        
        // Test printable ASCII works
        assert_eq!(VarFormat::ASCII.format("00100000"), " "); // Space (32)
        assert_eq!(VarFormat::ASCII.format("00110000"), "0"); // '0' character (48)
    }
    
    #[test]
    fn test_signal_value_enum() {
        // Test creation
        let present_value = SignalValue::present("10110101");
        let missing_value = SignalValue::missing();
        
        // Test type checks
        assert!(present_value.is_present());
        assert!(!present_value.is_missing());
        assert!(present_value.has_data());
        
        assert!(!missing_value.is_present());
        assert!(missing_value.is_missing());
        assert!(!missing_value.has_data());
        
        // Test as_option conversion
        assert_eq!(present_value.as_option(), Some("10110101".to_string()));
        assert_eq!(missing_value.as_option(), None);
        
        // Test display methods
        assert_eq!(present_value.display_value("N/A"), "10110101");
        assert_eq!(missing_value.display_value("N/A"), "N/A");
        assert_eq!(present_value.display_value_or_dash(), "10110101");
        assert_eq!(missing_value.display_value_or_dash(), "-");
        
        // Test map_present
        let formatted = present_value.map_present(|v| format!("0b{}", v));
        assert_eq!(formatted.as_option(), Some("0b10110101".to_string()));
        
        let formatted_missing = missing_value.map_present(|v| format!("0b{}", v));
        assert_eq!(formatted_missing.as_option(), None);
    }
    
    #[test]
    fn test_signal_value_from_conversions() {
        // Test From<Option<String>>
        let from_some: SignalValue = Some("test".to_string()).into();
        let from_none: SignalValue = None::<String>.into();
        assert_eq!(from_some, SignalValue::Present("test".to_string()));
        assert_eq!(from_none, SignalValue::Missing);
        
        // Test From<String>
        let from_string: SignalValue = "hello".to_string().into();
        assert_eq!(from_string, SignalValue::Present("hello".to_string()));
        
        // Test From<&str>
        let from_str: SignalValue = "world".into();
        assert_eq!(from_str, SignalValue::Present("world".to_string()));
    }
    
    #[test]
    fn test_signal_value_distinguishes_zero_from_missing() {
        // This is the core enhancement - distinguish actual "0" from missing data
        let actual_zero = SignalValue::present("0");
        let missing_data = SignalValue::missing();
        
        // Both could be displayed as "0" or "-" but we can now distinguish them programmatically
        assert!(actual_zero.is_present());
        assert!(missing_data.is_missing());
        
        // Actual zero has real data
        assert_eq!(actual_zero.display_value_or_dash(), "0");
        // Missing data shows placeholder
        assert_eq!(missing_data.display_value_or_dash(), "-");
        
        // This enables proper UI styling - missing data can be styled differently
        assert_eq!(actual_zero.has_data(), true);
        assert_eq!(missing_data.has_data(), false);
    }
    
    #[test]
    fn test_generate_file_id_path_based() {
        // Test that different paths generate different IDs using full path
        let id1 = generate_file_id("/path/to/test.vcd");
        let id2 = generate_file_id("/different/path/test.vcd");
        let id3 = generate_file_id("/path/to/other.vcd");
        
        // IDs should be different for different paths, even with same filename
        assert_ne!(id1, id2);
        assert_ne!(id1, id3);
        assert_ne!(id2, id3);
        
        // Same path should generate same ID consistently
        let id1_repeat = generate_file_id("/path/to/test.vcd");
        assert_eq!(id1, id1_repeat);
        
        // ID should be the sanitized full path
        assert_eq!(id1, "path_to_test.vcd");
        assert_eq!(id2, "different_path_test.vcd");
        assert_eq!(id3, "path_to_other.vcd");
        
        // Test Windows-style paths
        let windows_id = generate_file_id("C:\\Users\\Test\\file.vcd");
        assert_eq!(windows_id, "C_Users_Test_file.vcd");
        
        // Test path with spaces and special characters
        let special_id = generate_file_id("/home/user name/test file: v2.vcd");
        assert_eq!(special_id, "home_user_name_test_file_v2.vcd");
        
        // Test very long path gets truncated but remains unique
        let long_path = format!("/very/long/path/{}/test.vcd", "x".repeat(300));
        let long_id = generate_file_id(&long_path);
        assert!(long_id.len() <= 255, "Long path ID should be truncated to max 255 chars");
        assert!(long_id.starts_with("very_long_path"), "Long path ID should start with path prefix");
        assert!(long_id.contains("_"), "Long path ID should contain hash separator");
        
        // Different long paths should generate different IDs
        let long_path2 = format!("/very/long/path/{}/test.vcd", "y".repeat(300));
        let long_id2 = generate_file_id(&long_path2);
        assert_ne!(long_id, long_id2, "Different long paths should have different IDs");
    }
    
    #[test]
    fn test_sanitize_path_for_id() {
        // Test basic path sanitization
        assert_eq!(sanitize_path_for_id("/home/user/test.vcd"), "home_user_test.vcd");
        assert_eq!(sanitize_path_for_id("C:\\Windows\\test.vcd"), "C_Windows_test.vcd");
        
        // Test special characters
        assert_eq!(sanitize_path_for_id("/path with spaces/test.vcd"), "path_with_spaces_test.vcd");
        assert_eq!(sanitize_path_for_id("/path:with:colons/test.vcd"), "path_with_colons_test.vcd");
        assert_eq!(sanitize_path_for_id("/path<with>problematic\"chars/test.vcd"), "path_with_problematic_chars_test.vcd");
        
        // Test consecutive separators are cleaned up
        assert_eq!(sanitize_path_for_id("//multiple///separators//test.vcd"), "multiple_separators_test.vcd");
        
        // Test that valid characters are preserved
        assert_eq!(sanitize_path_for_id("/valid-path_123/test-file_v2.vcd"), "valid-path_123_test-file_v2.vcd");
    }
    
}