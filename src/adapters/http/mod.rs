use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use std::error::Error as StdError;
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
        // BR: Use rustls with embedded Mozilla roots; avoid system CA dependencies
        // BR: Fail fast for shadow execution to avoid delaying user responses
        let connect_timeout_ms = env::var("ARK_RUST_HTTP_CONNECT_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1_000);
        let total_timeout_ms = env::var("ARK_RUST_HTTP_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(2_000);

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(connect_timeout_ms))
            .timeout(Duration::from_millis(total_timeout_ms))
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
        // BR: Provide rich diagnostics to help debug Stage TLS/network issues safely
        let response = self.client.get(url).send().await.map_err(|e| {
            let classification = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "connect"
            } else if e.is_request() {
                "request"
            } else if e.is_redirect() {
                "redirect"
            } else if e.is_body() {
                "body"
            } else if e.is_decode() {
                "decode"
            } else {
                "network"
            };

            let mut msg =
                format!("Failed to fetch URL '{url}' (class: {classification}, tls: rustls): {e}");

            if let Some(src) = e.source() {
                // Include one level of source error for deeper insight
                msg.push_str(&format!("; source: {src}"));
            }

            SettingsError::FileSystemError(msg)
        })?;

        let final_url = response.url().to_string();
        let status = response.status();

        if !status.is_success() {
            // Try to extract a short snippet of the body for additional context
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unavailable>".to_string());
            let snippet = if body_text.len() > 256 {
                format!("{}…", &body_text[..256])
            } else {
                body_text
            };
            return Err(SettingsError::FileSystemError(format!(
                "HTTP request failed for '{final_url}': status {status}; body: {snippet}"
            )));
        }

        let content = response.text().await.map_err(|e| {
            SettingsError::FileSystemError(format!(
                "Failed to read response from '{final_url}': {e}"
            ))
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
