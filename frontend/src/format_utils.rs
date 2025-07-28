use shared::VarFormat;
use std::collections::HashMap;

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
        
        if formatted_value.is_empty() || formatted_value == "(empty)" {
            format!("({})", format_name)
        } else {
            format!("{} {}", formatted_value, format_name)
        }
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
    pub disabled: bool,
}

impl DropdownFormatOption {
    pub fn new(format: VarFormat, display_text: String, disabled: bool) -> Self {
        Self {
            format,
            display_text,
            disabled,
        }
    }
}

/// Generate dropdown options with formatted values for a signal
pub fn generate_dropdown_options(
    multi_value: &MultiFormatValue, 
    signal_type: &str
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
            let display_text = if multi_value.is_valid() {
                multi_value.get_display_with_format(format)
            } else {
                format!("({}) {}", "Loading", format.as_static_str())
            };

            let disabled = is_format_disabled_for_signal_type(format, signal_type);

            DropdownFormatOption::new(*format, display_text, disabled)
        })
        .collect()
}

/// Determine if a format should be disabled for a given signal type
fn is_format_disabled_for_signal_type(format: &VarFormat, signal_type: &str) -> bool {
    match format {
        VarFormat::ASCII => {
            // ASCII only makes sense for signals that are multiples of 8 bits
            !signal_type.contains("Wire") || signal_type.contains("1-bit")
        }
        _ => false, // All other formats are generally applicable
    }
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