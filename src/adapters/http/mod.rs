use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Duration;

use crate::adapters::common::IniProcessor;
use crate::core::domain::settings::SettingsRegistry;
use crate::core::errors::settings::{SettingsError, SettingsResult};
use crate::core::ports::settings::ConfigurationProvider;

/// HTTP adapter for fetching configuration files from URLs
pub struct HttpConfigurationProvider {
    client: reqwest::Client,
}

impl HttpConfigurationProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10)) // BR: HTTP requests timeout after 10 seconds to match Python implementation
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Check if a path is an HTTP URL
    pub fn is_http_url(path: &str) -> bool {
        path.starts_with("http://") || path.starts_with("https://")
    }

    /// Fetch content from HTTP URL
    async fn fetch_url_content(&self, url: &str) -> SettingsResult<String> {
        let response = self.client.get(url).send().await.map_err(|e| {
            SettingsError::FileSystemError(format!("Failed to fetch URL '{url}': {e}"))
        })?;

        if !response.status().is_success() {
            return Err(SettingsError::FileSystemError(format!(
                "HTTP request failed for '{url}': status {}",
                response.status()
            )));
        }

        let content = response.text().await.map_err(|e| {
            SettingsError::FileSystemError(format!("Failed to read response from '{url}': {e}"))
        })?;

        Ok(content)
    }
}

#[async_trait]
impl ConfigurationProvider for HttpConfigurationProvider {
    async fn load_registry(&self, registry_url: &str) -> SettingsResult<SettingsRegistry> {
        let content = self.fetch_url_content(registry_url).await?;
        let raw_data = IniProcessor::parse_ini_from_content(&content, registry_url)?;
        Ok(IniProcessor::create_settings_registry(&raw_data))
    }

    async fn load_config(&self, config_url: &str) -> SettingsResult<HashMap<String, String>> {
        let content = self.fetch_url_content(config_url).await?;
        let raw_data = IniProcessor::parse_ini_from_content(&content, config_url)?;
        Ok(IniProcessor::flatten_config_sections(&raw_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_http_url() {
        assert!(HttpConfigurationProvider::is_http_url("http://example.com"));
        assert!(HttpConfigurationProvider::is_http_url(
            "https://example.com"
        ));
        assert!(!HttpConfigurationProvider::is_http_url("/path/to/file"));
        assert!(!HttpConfigurationProvider::is_http_url("file://path"));
        assert!(!HttpConfigurationProvider::is_http_url("ftp://example.com"));
    }

    // Integration tests with actual HTTP requests require a test server
    // and are better suited for integration test files rather than unit tests
}
