use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::adapters::common::IniProcessor;
use crate::adapters::http::HttpConfigurationProvider;
use crate::core::domain::settings::SettingsRegistry;
use crate::core::errors::settings::{SettingsError, SettingsResult};
use crate::core::ports::settings::{
    ConfigurationProvider, FileSystemProvider as FileSystemProviderTrait,
};

/// File system adapter for reading INI configuration files and HTTP URLs
/// BR: FileSystem provider delegates HTTP requests to maintain separation of concerns
pub struct FileSystemConfigurationProvider {
    http_provider: HttpConfigurationProvider,
}

impl FileSystemConfigurationProvider {
    pub fn new() -> Self {
        // Embedding HTTP provider enables transparent handling of HTTP URLs
        // without coupling the file system adapter to HTTP implementation details
        Self {
            http_provider: HttpConfigurationProvider::new(),
        }
    }
}

#[async_trait]
impl ConfigurationProvider for FileSystemConfigurationProvider {
    async fn load_registry(&self, registry_path: &str) -> SettingsResult<SettingsRegistry> {
        // BR: HTTP URLs take precedence over file system paths for transparent configuration loading
        if HttpConfigurationProvider::is_http_url(registry_path) {
            return self.http_provider.load_registry(registry_path).await;
        }

        if !Path::new(registry_path).exists() {
            return Err(SettingsError::FileSystemError(format!(
                "Registry file not found: {registry_path}"
            )));
        }

        let raw_data = IniProcessor::parse_ini_from_file(registry_path)?;
        Ok(IniProcessor::create_settings_registry(&raw_data))
    }

    async fn load_config(&self, config_path: &str) -> SettingsResult<HashMap<String, String>> {
        // BR: Configuration sources must support both local files and HTTP URLs transparently
        if HttpConfigurationProvider::is_http_url(config_path) {
            return self.http_provider.load_config(config_path).await;
        }

        if !Path::new(config_path).exists() {
            return Err(SettingsError::FileSystemError(format!(
                "Config file not found: {config_path}"
            )));
        }

        let raw_data = IniProcessor::parse_ini_from_file(config_path)?;
        Ok(IniProcessor::flatten_config_sections(&raw_data))
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
            SettingsError::FileSystemError(format!("Failed to read file '{path}': {e}"))
        })
    }

    async fn get_file_metadata(&self, path: &str) -> SettingsResult<HashMap<String, String>> {
        let metadata = fs::metadata(path).await.map_err(|e| {
            SettingsError::FileSystemError(format!("Failed to get metadata for '{path}': {e}"))
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
}
