use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct NovyWaveConfig {
    #[serde(default)]
    pub app: AppSection,
    #[serde(default)]
    pub workspace: WorkspaceSection,
    #[serde(default)]
    pub ui: UiSection,
}

#[derive(Debug, Default, Deserialize)]
pub struct AppSection {
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct UiSection {
    #[serde(default)]
    pub theme: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct WorkspaceSection {
    #[serde(default)]
    pub opened_files: Vec<String>,
    #[serde(default)]
    pub selected_variables: Vec<SelectedVariable>,
    #[serde(default)]
    pub expanded_scopes: Vec<String>,
    #[serde(default)]
    pub selected_scope_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SelectedVariable {
    pub unique_id: String,
    #[serde(default)]
    pub formatter: Option<String>,
}

pub fn load_config(path: &Path) -> Result<NovyWaveConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: NovyWaveConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}
