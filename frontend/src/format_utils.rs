use shared::VarFormat;
use std::collections::HashMap;

/// Truncate a value string if it's longer than max_chars, using simple right truncation
pub fn truncate_value(value: &str, max_chars: usize) -> String {
    // ASCII-only: can use byte length directly since 1 char = 1 byte
    if value.len() <= max_chars {
        return value.to_string();
    }
    
    // Simple right truncation with ellipsis
    let truncate_at = max_chars.saturating_sub(3);
    format!("{}...", &value[..truncate_at])
}

/// Container for multi-format signal values
#[derive(Debug, Clone)]
pub struct MultiFormatValue {
    pub raw_binary: String,
    pub formatted_values: HashMap<VarFormat, String>,
}

impl MultiFormatValue {
    /// Create a new MultiFormatValue from raw binary string
    pub fn new(raw_binary: String) -> Self {
        let formatted_values = Self::generate_all_formats(&raw_binary);
        Self {
            raw_binary,
            formatted_values,
        }
    }

    /// Generate formatted values for all VarFormat types
    fn generate_all_formats(raw_binary: &str) -> HashMap<VarFormat, String> {
        let mut formatted = HashMap::new();
        
        let formats = [
            VarFormat::ASCII,
            VarFormat::Binary,
            VarFormat::BinaryWithGroups,
            VarFormat::Hexadecimal,
            VarFormat::Octal,
            VarFormat::Signed,
            VarFormat::Unsigned,
        ];

        for format in formats {
            let formatted_value = if raw_binary.is_empty() {
                "(empty)".to_string()
            } else {
                format.format(raw_binary)
            };
            formatted.insert(format, formatted_value);
        }

        formatted
    }

    /// Get formatted value for specific format
    pub fn get_formatted(&self, format: &VarFormat) -> String {
        self.formatted_values
            .get(format)
            .cloned()
            .unwrap_or_else(|| "(error)".to_string())
    }

    /// Get display string with value and format name (e.g., "1010 Bin")
    pub fn get_display_with_format(&self, format: &VarFormat) -> String {
        let formatted_value = self.get_formatted(format);
        let format_name = format.as_static_str();
        
        let display_value = if formatted_value.is_empty() { "-" } else { &formatted_value };
        format!("{} {}", display_value, format_name)
    }

    /// Get display string with truncated value for dropdowns (e.g., "101010101...1010 Bin")
    pub fn get_truncated_display_with_format(&self, format: &VarFormat, max_chars: usize) -> String {
        let formatted_value = self.get_formatted(format);
        let format_name = format.as_static_str();
        
        let display_value = if formatted_value.is_empty() { "-" } else { &formatted_value };
        let truncated_value = truncate_value(display_value, max_chars);
        format!("{} {}", truncated_value, format_name)
    }

    /// Get full untruncated display string for tooltip
    pub fn get_full_display_with_format(&self, format: &VarFormat) -> String {
        self.get_display_with_format(format)
    }

    /// Check if raw binary value is valid (not empty, loading, or error)
    pub fn is_valid(&self) -> bool {
        !self.raw_binary.is_empty() 
            && self.raw_binary != "Loading..." 
            && self.raw_binary != "No value"
    }
}

/// Format options for dropdown - contains value and disabled state
#[derive(Debug, Clone)]
pub struct DropdownFormatOption {
    pub format: VarFormat,
    pub display_text: String,
    pub full_text: String,    // For tooltip
    pub disabled: bool,
}

impl DropdownFormatOption {
    pub fn new(format: VarFormat, display_text: String, full_text: String, disabled: bool) -> Self {
        Self {
            format,
            display_text,
            full_text,
            disabled,
        }
    }
}

/// Generate dropdown options with formatted values for a signal
pub fn generate_dropdown_options(
    multi_value: &MultiFormatValue, 
    _signal_type: &str
) -> Vec<DropdownFormatOption> {
    generate_dropdown_options_with_truncation(multi_value, _signal_type, 30)
}

/// Generate dropdown options with configurable value truncation
pub fn generate_dropdown_options_with_truncation(
    multi_value: &MultiFormatValue, 
    _signal_type: &str,
    max_value_chars: usize
) -> Vec<DropdownFormatOption> {
    let all_formats = [
        VarFormat::ASCII,
        VarFormat::Binary,
        VarFormat::BinaryWithGroups,
        VarFormat::Hexadecimal,
        VarFormat::Octal,
        VarFormat::Signed,
        VarFormat::Unsigned,
    ];

    all_formats
        .iter()
        .map(|format| {
            let (display_text, full_text) = if multi_value.is_valid() {
                let full = multi_value.get_full_display_with_format(format);
                let truncated = multi_value.get_truncated_display_with_format(format, max_value_chars);
                (truncated, full)
            } else {
                let text = format!("({}) {}", "Loading", format.as_static_str());
                (text.clone(), text)
            };

            DropdownFormatOption::new(*format, display_text, full_text, false)
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_format_value_creation() {
        let binary = "1010".to_string();
        let multi_value = MultiFormatValue::new(binary);
        
        assert_eq!(multi_value.get_formatted(&VarFormat::Binary), "1010");
        assert_eq!(multi_value.get_formatted(&VarFormat::Hexadecimal), "a");
        assert_eq!(multi_value.get_formatted(&VarFormat::Unsigned), "10");
    }

    #[test]
    fn test_display_with_format() {
        let binary = "1010".to_string();
        let multi_value = MultiFormatValue::new(binary);
        
        assert_eq!(multi_value.get_display_with_format(&VarFormat::Binary), "1010 Bin");
        assert_eq!(multi_value.get_display_with_format(&VarFormat::Hexadecimal), "a Hex");
        assert_eq!(multi_value.get_display_with_format(&VarFormat::Unsigned), "10 UInt");
    }

    #[test]
    fn test_dropdown_options_generation() {
        let binary = "1010".to_string();
        let multi_value = MultiFormatValue::new(binary);
        let options = generate_dropdown_options(&multi_value, "Wire 4-bit");
        
        assert_eq!(options.len(), 7);
        
        // Check that we get formatted values
        let hex_option = options.iter().find(|opt| matches!(opt.format, VarFormat::Hexadecimal)).unwrap();
        assert_eq!(hex_option.display_text, "a Hex");
        
        let bin_option = options.iter().find(|opt| matches!(opt.format, VarFormat::Binary)).unwrap();
        assert_eq!(bin_option.display_text, "1010 Bin");
    }
}