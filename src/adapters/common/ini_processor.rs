use config::{Config, File, FileFormat};
use std::collections::HashMap;

use crate::core::domain::settings::{ProjectConfig, SettingsRegistry};
use crate::core::errors::settings::{SettingsError, SettingsResult};

/// Common INI processing utilities for both HTTP and file system adapters
/// BR: Configuration processing logic is centralized to ensure consistent behavior across adapters
pub struct IniProcessor;

impl IniProcessor {
    /// Parse INI content from string
    pub fn parse_ini_from_content(
        content: &str,
        source_name: &str,
    ) -> SettingsResult<HashMap<String, serde_json::Value>> {
        let config = Config::builder()
            .add_source(File::from_str(content, FileFormat::Ini))
            .build()
            .map_err(|e| {
                SettingsError::ParseError(format!(
                    "Failed to parse INI content from '{source_name}': {e}"
                ))
            })?;

        let raw_data: HashMap<String, serde_json::Value> =
            config.try_deserialize().map_err(|e| {
                SettingsError::ParseError(format!(
                    "Failed to deserialize INI content from '{source_name}': {e}"
                ))
            })?;

        Ok(raw_data)
    }

    /// Parse INI content from file path
    pub fn parse_ini_from_file(
        file_path: &str,
    ) -> SettingsResult<HashMap<String, serde_json::Value>> {
        let config = Config::builder()
            .add_source(File::with_name(file_path).format(FileFormat::Ini))
            .build()
            .map_err(|e| {
                SettingsError::ParseError(format!("Failed to parse INI file '{file_path}': {e}"))
            })?;

        let raw_data: HashMap<String, serde_json::Value> =
            config.try_deserialize().map_err(|e| {
                SettingsError::ParseError(format!(
                    "Failed to deserialize INI file '{file_path}': {e}"
                ))
            })?;

        Ok(raw_data)
    }

    /// Extract DEFAULT section from raw INI data
    pub fn extract_default_section(
        raw_data: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, String> {
        raw_data
            .get("DEFAULT")
            .and_then(|v| v.as_object())
            .map(|default_map| {
                default_map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect::<HashMap<String, String>>()
            })
            .unwrap_or_default()
    }

    /// Convert raw INI data to project configurations
    pub fn convert_to_project_configs(
        raw_data: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, ProjectConfig> {
        let mut projects = HashMap::new();

        for (section, value) in raw_data.iter() {
            if section == "DEFAULT" {
                continue;
            }

            let mut section_map = HashMap::new();

            if let Some(inner_map) = value.as_object() {
                for (key, inner_value) in inner_map {
                    section_map.insert(key.clone(), inner_value.as_str().unwrap_or("").to_string());
                }
            }

            // BR: Project configurations are case-insensitive and stored in lowercase for consistent lookup
            projects.insert(section.to_lowercase(), ProjectConfig::new(section_map));
        }

        projects
    }

    /// Create SettingsRegistry from raw INI data
    pub fn create_settings_registry(
        raw_data: &HashMap<String, serde_json::Value>,
    ) -> SettingsRegistry {
        let default_config = Self::extract_default_section(raw_data);
        let projects = Self::convert_to_project_configs(raw_data);
        SettingsRegistry::new(projects, default_config)
    }

    /// Flatten INI sections into a single key-value configuration map
    pub fn flatten_config_sections(
        raw_data: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, String> {
        let mut config_map = HashMap::new();

        for (section, value) in raw_data.iter() {
            if let Some(inner_map) = value.as_object() {
                for (key, inner_value) in inner_map {
                    let full_key = if section == "DEFAULT" {
                        key.clone()
                    } else {
                        format!("{section}.{key}")
                    };
                    config_map.insert(full_key, inner_value.as_str().unwrap_or("").to_string());
                }
            }
        }

        config_map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_default_section() {
        let mut raw_data = HashMap::new();
        raw_data.insert(
            "DEFAULT".to_string(),
            serde_json::json!({
                "key1": "value1",
                "key2": "value2"
            }),
        );

        let defaults = IniProcessor::extract_default_section(&raw_data);
        assert_eq!(defaults.get("key1"), Some(&"value1".to_string()));
        assert_eq!(defaults.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_extract_default_section_empty() {
        let raw_data = HashMap::new();
        let defaults = IniProcessor::extract_default_section(&raw_data);
        assert!(defaults.is_empty());
    }

    #[test]
    fn test_convert_to_project_configs() {
        let mut raw_data = HashMap::new();
        raw_data.insert(
            "DEFAULT".to_string(),
            serde_json::json!({
                "default_key": "default_value"
            }),
        );
        raw_data.insert(
            "project1".to_string(),
            serde_json::json!({
                "project_key": "project_value"
            }),
        );
        raw_data.insert(
            "Project2".to_string(),
            serde_json::json!({
                "project_key2": "project_value2"
            }),
        );

        let projects = IniProcessor::convert_to_project_configs(&raw_data);
        assert_eq!(projects.len(), 2);
        assert!(projects.contains_key("project1"));
        assert!(projects.contains_key("project2")); // lowercase
        assert!(!projects.contains_key("DEFAULT"));

        let project_config = projects.get("project1").unwrap();
        assert_eq!(
            project_config.get("project_key"),
            Some(&"project_value".to_string())
        );
    }

    #[test]
    fn test_create_settings_registry() {
        let mut raw_data = HashMap::new();
        raw_data.insert(
            "DEFAULT".to_string(),
            serde_json::json!({
                "TopLevelObjectUrl": "http://dasch.swiss",
                "Host": "app.dasch.swiss"
            }),
        );
        raw_data.insert(
            "0003".to_string(),
            serde_json::json!({
                "ProjectHost": "meta.dasch.swiss"
            }),
        );

        let registry = IniProcessor::create_settings_registry(&raw_data);

        assert_eq!(
            registry.get_default_config("TopLevelObjectUrl"),
            Some("http://dasch.swiss".to_string())
        );
        assert_eq!(
            registry.get_default_config("Host"),
            Some("app.dasch.swiss".to_string())
        );

        let project_config = registry.get_project_config("0003").unwrap();
        assert_eq!(
            project_config.get("ProjectHost"),
            Some(&"meta.dasch.swiss".to_string())
        );
    }

    #[test]
    fn test_flatten_config_sections() {
        let mut raw_data = HashMap::new();
        raw_data.insert(
            "DEFAULT".to_string(),
            serde_json::json!({
                "key1": "value1",
                "key2": "value2"
            }),
        );
        raw_data.insert(
            "section1".to_string(),
            serde_json::json!({
                "key3": "value3",
                "key4": "value4"
            }),
        );

        let config = IniProcessor::flatten_config_sections(&raw_data);

        assert_eq!(config.get("key1"), Some(&"value1".to_string()));
        assert_eq!(config.get("key2"), Some(&"value2".to_string()));
        assert_eq!(config.get("section1.key3"), Some(&"value3".to_string()));
        assert_eq!(config.get("section1.key4"), Some(&"value4".to_string()));
    }

    #[test]
    fn test_parse_ini_from_content() {
        let ini_content = r#"
[DEFAULT]
key1 = value1
key2 = value2

[section1]
key3 = value3
"#;

        let raw_data = IniProcessor::parse_ini_from_content(ini_content, "test_source").unwrap();

        assert!(raw_data.contains_key("DEFAULT"));
        assert!(raw_data.contains_key("section1"));

        let default_section = raw_data.get("DEFAULT").unwrap().as_object().unwrap();
        assert_eq!(
            default_section.get("key1").unwrap().as_str(),
            Some("value1")
        );
    }
}
