use async_trait::async_trait;
use config::{Config, File, FileFormat};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::core::domain::settings::{ProjectConfig, SettingsRegistry};
use crate::core::errors::settings::{SettingsError, SettingsResult};
use crate::core::ports::settings::{
    ConfigurationProvider, FileSystemProvider as FileSystemProviderTrait,
};

/// File system adapter for reading INI configuration files
pub struct FileSystemConfigurationProvider;

impl FileSystemConfigurationProvider {
    pub fn new() -> Self {
        Self
    }

    /// Parse INI file and extract sections
    async fn parse_ini_file(
        &self,
        file_path: &str,
    ) -> SettingsResult<HashMap<String, serde_json::Value>> {
        let config = Config::builder()
            .add_source(File::with_name(file_path).format(FileFormat::Ini))
            .build()
            .map_err(|e| {
                SettingsError::ParseError(format!(
                    "Failed to parse INI file '{}': {}",
                    file_path, e
                ))
            })?;

        let raw_data: HashMap<String, serde_json::Value> =
            config.try_deserialize().map_err(|e| {
                SettingsError::ParseError(format!(
                    "Failed to deserialize INI file '{}': {}",
                    file_path, e
                ))
            })?;

        Ok(raw_data)
    }

    /// Extract DEFAULT section from raw INI data
    fn extract_default_section(
        &self,
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
    fn convert_to_project_configs(
        &self,
        raw_data: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, ProjectConfig> {
        let mut projects = HashMap::new();

        for (section, value) in raw_data.iter() {
            if section == "DEFAULT" {
                continue; // Skip DEFAULT section
            }

            let mut section_map = HashMap::new();

            if let Some(inner_map) = value.as_object() {
                for (key, inner_value) in inner_map {
                    section_map.insert(key.clone(), inner_value.as_str().unwrap_or("").to_string());
                }
            }

            projects.insert(section.to_lowercase(), ProjectConfig::new(section_map));
        }

        projects
    }
}

#[async_trait]
impl ConfigurationProvider for FileSystemConfigurationProvider {
    async fn load_registry(&self, registry_path: &str) -> SettingsResult<SettingsRegistry> {
        // Check if file exists
        if !Path::new(registry_path).exists() {
            return Err(SettingsError::FileSystemError(format!(
                "Registry file not found: {}",
                registry_path
            )));
        }

        // Parse INI file
        let raw_data = self.parse_ini_file(registry_path).await?;

        // Extract DEFAULT section
        let default_config = self.extract_default_section(&raw_data);

        // Convert to project configurations
        let projects = self.convert_to_project_configs(&raw_data);

        Ok(SettingsRegistry::new(projects, default_config))
    }

    async fn load_config(&self, config_path: &str) -> SettingsResult<HashMap<String, String>> {
        // Check if file exists
        if !Path::new(config_path).exists() {
            return Err(SettingsError::FileSystemError(format!(
                "Config file not found: {}",
                config_path
            )));
        }

        // Parse INI file
        let raw_data = self.parse_ini_file(config_path).await?;

        // Convert to flat key-value map
        let mut config_map = HashMap::new();
        for (section, value) in raw_data.iter() {
            if let Some(inner_map) = value.as_object() {
                for (key, inner_value) in inner_map {
                    let full_key = if section == "DEFAULT" {
                        key.clone()
                    } else {
                        format!("{}.{}", section, key)
                    };
                    config_map.insert(full_key, inner_value.as_str().unwrap_or("").to_string());
                }
            }
        }

        Ok(config_map)
    }
}

/// File system provider for direct file operations
pub struct FileSystemProvider;

impl FileSystemProvider {}

#[async_trait]
impl FileSystemProviderTrait for FileSystemProvider {
    async fn file_exists(&self, path: &str) -> SettingsResult<bool> {
        Ok(Path::new(path).exists())
    }

    async fn read_file(&self, path: &str) -> SettingsResult<String> {
        fs::read_to_string(path).await.map_err(|e| {
            SettingsError::FileSystemError(format!("Failed to read file '{}': {}", path, e))
        })
    }

    async fn get_file_metadata(&self, path: &str) -> SettingsResult<HashMap<String, String>> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            SettingsError::FileSystemError(format!("Failed to get metadata for '{}': {}", path, e))
        })?;

        let mut meta_map = HashMap::new();
        meta_map.insert("size".to_string(), metadata.len().to_string());
        meta_map.insert("is_file".to_string(), metadata.is_file().to_string());
        meta_map.insert("is_dir".to_string(), metadata.is_dir().to_string());

        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                meta_map.insert(
                    "modified_timestamp".to_string(),
                    duration.as_secs().to_string(),
                );
            }
        }

        Ok(meta_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_system_configuration_provider_load_registry() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[DEFAULT]
TopLevelObjectUrl = http://dasch.swiss
Host = app.dasch.swiss

[0003]
ProjectHost = meta.dasch.swiss

[080e]
Host = app.dasch.swiss
"#
        )
        .unwrap();

        let provider = FileSystemConfigurationProvider::new();
        let registry = provider
            .load_registry(temp_file.path().to_str().unwrap())
            .await
            .unwrap();

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
        assert_eq!(
            project_config.get("Host"),
            Some(&"app.dasch.swiss".to_string())
        ); // Inherited from DEFAULT

        let project_config_080e = registry.get_project_config("080e").unwrap();
        assert_eq!(
            project_config_080e.get("Host"),
            Some(&"app.dasch.swiss".to_string())
        );
    }

    #[tokio::test]
    async fn test_file_system_configuration_provider_load_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[DEFAULT]
key1 = value1
key2 = value2

[section1]
key3 = value3
"#
        )
        .unwrap();

        let provider = FileSystemConfigurationProvider::new();
        let config = provider
            .load_config(temp_file.path().to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(config.get("key1"), Some(&"value1".to_string()));
        assert_eq!(config.get("key2"), Some(&"value2".to_string()));
        assert_eq!(config.get("section1.key3"), Some(&"value3".to_string()));
    }

    #[tokio::test]
    async fn test_file_system_configuration_provider_file_not_found() {
        let provider = FileSystemConfigurationProvider::new();
        let result = provider.load_registry("non_existent_file.ini").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SettingsError::FileSystemError(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected FileSystemError"),
        }
    }

    #[tokio::test]
    async fn test_file_system_provider_file_exists() {
        let temp_file = NamedTempFile::new().unwrap();
        let provider = FileSystemProvider;

        let exists = provider
            .file_exists(temp_file.path().to_str().unwrap())
            .await
            .unwrap();
        assert!(exists);

        let not_exists = provider.file_exists("non_existent_file.txt").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_file_system_provider_read_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test content").unwrap();

        let provider = FileSystemProvider;
        let content = provider
            .read_file(temp_file.path().to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(content.trim(), "test content");
    }

    #[tokio::test]
    async fn test_file_system_provider_get_file_metadata() {
        let temp_file = NamedTempFile::new().unwrap();
        let provider = FileSystemProvider;

        let metadata = provider
            .get_file_metadata(temp_file.path().to_str().unwrap())
            .await
            .unwrap();

        assert!(metadata.contains_key("size"));
        assert!(metadata.contains_key("is_file"));
        assert!(metadata.contains_key("is_dir"));
        assert_eq!(metadata.get("is_file"), Some(&"true".to_string()));
        assert_eq!(metadata.get("is_dir"), Some(&"false".to_string()));
    }

    #[test]
    fn test_extract_default_section() {
        let provider = FileSystemConfigurationProvider::new();
        let mut raw_data = HashMap::new();
        raw_data.insert(
            "DEFAULT".to_string(),
            serde_json::json!({
                "key1": "value1",
                "key2": "value2"
            }),
        );

        let defaults = provider.extract_default_section(&raw_data);
        assert_eq!(defaults.get("key1"), Some(&"value1".to_string()));
        assert_eq!(defaults.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_convert_to_project_configs() {
        let provider = FileSystemConfigurationProvider::new();
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

        let projects = provider.convert_to_project_configs(&raw_data);
        assert_eq!(projects.len(), 1);
        assert!(projects.contains_key("project1"));
        assert!(!projects.contains_key("DEFAULT"));

        let project_config = projects.get("project1").unwrap();
        assert_eq!(
            project_config.get("project_key"),
            Some(&"project_value".to_string())
        );
    }
}
