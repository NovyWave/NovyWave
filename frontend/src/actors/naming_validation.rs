//! Event-Source Relay Naming Validation
//!
//! Automated checks to ensure all relay fields follow the mandatory {source}_{event}_relay pattern
//! and prevent Manager/Service/Controller enterprise antipatterns.

#[cfg(test)]
mod naming_compliance_tests {
    use regex::Regex;
    use std::fs;
    use std::path::Path;

    /// Validates that all relay field declarations follow event-source naming pattern
    #[test]
    fn verify_relay_naming_compliance() {
        let relay_pattern = Regex::new(r"pub\s+(\w+):\s*Relay<").unwrap();
        let event_source_pattern = Regex::new(r"^\w+_\w+_relay$").unwrap();
        
        let mut violations = Vec::new();
        let actors_dir = Path::new("frontend/src/actors");
        
        if actors_dir.exists() {
            scan_directory_for_relay_violations(&actors_dir, &relay_pattern, &event_source_pattern, &mut violations);
        }
        
        if !violations.is_empty() {
            panic!(
                "❌ Relay naming violations found:\n{}\n\n✅ Correct pattern: {{source}}_{{event}}_relay\n📖 See: docs/actors_relays/novywave/event_source_naming_guide.md",
                violations.join("\n")
            );
        }
    }
    
    /// Validates that no Manager/Service/Controller patterns exist in struct names
    #[test]
    fn verify_no_enterprise_patterns() {
        let struct_pattern = Regex::new(r"struct\s+(\w+)").unwrap();
        let forbidden_patterns = [
            "Manager", "Service", "Controller", 
            "Handler", "Processor", "Helper",
            "Provider", "Factory", "Builder"
        ];
        
        let mut violations = Vec::new();
        let frontend_dir = Path::new("frontend/src");
        
        if frontend_dir.exists() {
            scan_directory_for_enterprise_violations(&frontend_dir, &struct_pattern, &forbidden_patterns, &mut violations);
        }
        
        if !violations.is_empty() {
            panic!(
                "❌ Enterprise pattern violations found:\n{}\n\n✅ Use domain-driven naming instead\n📖 See: docs/actors_relays/novywave/migration_strategy.md",
                violations.join("\n")
            );
        }
    }
    
    fn scan_directory_for_relay_violations(
        dir: &Path,
        relay_pattern: &Regex,
        event_source_pattern: &Regex,
        violations: &mut Vec<String>
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for cap in relay_pattern.captures_iter(&content) {
                            if let Some(relay_name) = cap.get(1) {
                                let name = relay_name.as_str();
                                if !event_source_pattern.is_match(name) {
                                    violations.push(format!(
                                        "  {} in {}:{} - should follow {{source}}_{{event}}_relay pattern",
                                        name,
                                        path.display(),
                                        get_line_number(&content, relay_name.start())
                                    ));
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    scan_directory_for_relay_violations(&path, relay_pattern, event_source_pattern, violations);
                }
            }
        }
    }
    
    fn scan_directory_for_enterprise_violations(
        dir: &Path,
        struct_pattern: &Regex,
        forbidden_patterns: &[&str],
        violations: &mut Vec<String>
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for cap in struct_pattern.captures_iter(&content) {
                            if let Some(struct_name) = cap.get(1) {
                                let name = struct_name.as_str();
                                for &forbidden in forbidden_patterns {
                                    if name.contains(forbidden) {
                                        violations.push(format!(
                                            "  {} in {}:{} - contains forbidden pattern '{}'",
                                            name,
                                            path.display(),
                                            get_line_number(&content, struct_name.start()),
                                            forbidden
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                    scan_directory_for_enterprise_violations(&path, struct_pattern, forbidden_patterns, violations);
                }
            }
        }
    }
    
    fn get_line_number(content: &str, byte_offset: usize) -> usize {
        content[..byte_offset].lines().count()
    }
}

/// Manual validation helper for development
pub fn _validate_current_relay_naming() {
    println!("🔍 Scanning for relay naming compliance...");
    
    // This will be called during development to check naming patterns
    // The actual validation logic is in the tests above
    
    println!("✅ Run `cargo test naming_compliance` to validate relay naming");
    println!("📖 See docs/actors_relays/novywave/event_source_naming_guide.md for guidance");
}

/// Get examples of correct event-source relay naming for each domain
pub fn _get_naming_examples() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("TrackedFiles", vec![
            "file_dropped_relay: Relay<Vec<PathBuf>>",
            "file_selected_relay: Relay<PathBuf>", 
            "parse_completed_relay: Relay<(String, ParseResult)>",
            "reload_button_clicked_relay: Relay<String>",
        ]),
        ("SelectedVariables", vec![
            "variable_clicked_relay: Relay<String>",
            "variable_removed_relay: Relay<String>",
            "scope_expanded_relay: Relay<String>",
            "selection_cleared_relay: Relay",
        ]),
        ("WaveformTimeline", vec![
            "cursor_clicked_relay: Relay<f64>",
            "cursor_dragged_relay: Relay<f64>",
            "zoom_changed_relay: Relay<f32>",
            "mouse_moved_relay: Relay<(f32, f32)>",
        ]),
        ("UserConfiguration", vec![
            "theme_changed_relay: Relay<Theme>",
            "config_loaded_relay: Relay<WorkspaceConfig>",
            "save_requested_relay: Relay",
            "panel_resized_relay: Relay<(PanelId, f32, f32)>",
        ]),
    ]
}