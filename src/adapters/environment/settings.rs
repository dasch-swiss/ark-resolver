use async_trait::async_trait;
use regex::Regex;
use std::env;

use crate::core::domain::parsing::{ark_path_regex, resource_iri_regex, v0_ark_path_regex};
use crate::core::domain::settings::ArkConfig;
use crate::core::errors::settings::{SettingsError, SettingsResult};
use crate::core::ports::settings::{EnvironmentProvider, RegexProvider};

/// Environment variable adapter for reading configuration from environment
pub struct EnvironmentVariableProvider;

impl EnvironmentVariableProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EnvironmentProvider for EnvironmentVariableProvider {
    async fn get_env_var(&self, key: &str) -> SettingsResult<Option<String>> {
        match env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(SettingsError::EnvironmentError(format!(
                "Failed to read environment variable '{}': {}",
                key, e
            ))),
        }
    }

    async fn get_env_var_or_default(&self, key: &str, default: &str) -> SettingsResult<String> {
        match self.get_env_var(key).await? {
            Some(value) => Ok(value),
            None => Ok(default.to_string()),
        }
    }

    async fn build_ark_config(&self, registry_path: String) -> SettingsResult<ArkConfig> {
        let ark_external_host = self
            .get_env_var_or_default("ARK_EXTERNAL_HOST", "ark.example.org")
            .await?;
        let ark_internal_host = self
            .get_env_var_or_default("ARK_INTERNAL_HOST", "0.0.0.0")
            .await?;
        let ark_internal_port = self
            .get_env_var_or_default("ARK_INTERNAL_PORT", "3336")
            .await?;
        let ark_naan = self.get_env_var_or_default("ARK_NAAN", "00000").await?;
        let ark_https_proxy = self
            .get_env_var_or_default("ARK_HTTPS_PROXY", "true")
            .await?;
        let ark_github_secret = self.get_env_var_or_default("ARK_GITHUB_SECRET", "").await?;

        Ok(ArkConfig::new(
            ark_external_host,
            ark_internal_host,
            ark_internal_port,
            ark_naan,
            ark_https_proxy,
            registry_path,
            ark_github_secret,
        ))
    }
}

/// Regex provider that uses the existing parsing functions
pub struct DefaultRegexProvider;

impl DefaultRegexProvider {
    pub fn new() -> Self {
        Self
    }
}

impl RegexProvider for DefaultRegexProvider {
    fn compile_resource_iri_regex(&self) -> SettingsResult<Regex> {
        Ok(resource_iri_regex())
    }

    fn compile_ark_path_regex(&self, ark_naan: &str) -> SettingsResult<Regex> {
        Ok(ark_path_regex(ark_naan))
    }

    fn compile_v0_ark_path_regex(&self, ark_naan: &str) -> SettingsResult<Regex> {
        Ok(v0_ark_path_regex(ark_naan))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[tokio::test]
    async fn test_environment_variable_provider_get_env_var() {
        let provider = EnvironmentVariableProvider::new();

        // Set a test environment variable
        env::set_var("TEST_ENV_VAR", "test_value");

        let result = provider.get_env_var("TEST_ENV_VAR").await.unwrap();
        assert_eq!(result, Some("test_value".to_string()));

        let result = provider.get_env_var("NON_EXISTENT_VAR").await.unwrap();
        assert_eq!(result, None);

        // Clean up
        env::remove_var("TEST_ENV_VAR");
    }

    #[tokio::test]
    async fn test_environment_variable_provider_get_env_var_or_default() {
        let provider = EnvironmentVariableProvider::new();

        // Set a test environment variable
        env::set_var("TEST_ENV_VAR_DEFAULT", "env_value");

        let result = provider
            .get_env_var_or_default("TEST_ENV_VAR_DEFAULT", "default_value")
            .await
            .unwrap();
        assert_eq!(result, "env_value");

        let result = provider
            .get_env_var_or_default("NON_EXISTENT_VAR_DEFAULT", "default_value")
            .await
            .unwrap();
        assert_eq!(result, "default_value");

        // Clean up
        env::remove_var("TEST_ENV_VAR_DEFAULT");
    }

    #[tokio::test]
    #[serial]
    async fn test_environment_variable_provider_build_ark_config() {
        let provider = EnvironmentVariableProvider::new();

        // Save current environment variables
        let original_external_host = env::var("ARK_EXTERNAL_HOST").ok();
        let original_internal_host = env::var("ARK_INTERNAL_HOST").ok();
        let original_internal_port = env::var("ARK_INTERNAL_PORT").ok();
        let original_naan = env::var("ARK_NAAN").ok();
        let original_https_proxy = env::var("ARK_HTTPS_PROXY").ok();
        let original_github_secret = env::var("ARK_GITHUB_SECRET").ok();

        // Set test environment variables
        env::set_var("ARK_EXTERNAL_HOST", "test.example.org");
        env::set_var("ARK_INTERNAL_HOST", "127.0.0.1");
        env::set_var("ARK_INTERNAL_PORT", "8080");
        env::set_var("ARK_NAAN", "12345");
        env::set_var("ARK_HTTPS_PROXY", "false");
        env::set_var("ARK_GITHUB_SECRET", "secret123");

        let config = provider
            .build_ark_config("test_registry.ini".to_string())
            .await
            .unwrap();

        assert_eq!(config.ark_external_host, "test.example.org");
        assert_eq!(config.ark_internal_host, "127.0.0.1");
        assert_eq!(config.ark_internal_port, "8080");
        assert_eq!(config.ark_naan, "12345");
        assert_eq!(config.ark_https_proxy, "false");
        assert_eq!(config.ark_registry, "test_registry.ini");
        assert_eq!(config.ark_github_secret, "secret123");

        // Restore original environment variables
        if let Some(val) = original_external_host {
            env::set_var("ARK_EXTERNAL_HOST", val);
        } else {
            env::remove_var("ARK_EXTERNAL_HOST");
        }
        if let Some(val) = original_internal_host {
            env::set_var("ARK_INTERNAL_HOST", val);
        } else {
            env::remove_var("ARK_INTERNAL_HOST");
        }
        if let Some(val) = original_internal_port {
            env::set_var("ARK_INTERNAL_PORT", val);
        } else {
            env::remove_var("ARK_INTERNAL_PORT");
        }
        if let Some(val) = original_naan {
            env::set_var("ARK_NAAN", val);
        } else {
            env::remove_var("ARK_NAAN");
        }
        if let Some(val) = original_https_proxy {
            env::set_var("ARK_HTTPS_PROXY", val);
        } else {
            env::remove_var("ARK_HTTPS_PROXY");
        }
        if let Some(val) = original_github_secret {
            env::set_var("ARK_GITHUB_SECRET", val);
        } else {
            env::remove_var("ARK_GITHUB_SECRET");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_environment_variable_provider_build_ark_config_with_defaults() {
        let provider = EnvironmentVariableProvider::new();

        // Clear any existing environment variables
        env::remove_var("ARK_EXTERNAL_HOST");
        env::remove_var("ARK_INTERNAL_HOST");
        env::remove_var("ARK_INTERNAL_PORT");
        env::remove_var("ARK_NAAN");
        env::remove_var("ARK_HTTPS_PROXY");
        env::remove_var("ARK_GITHUB_SECRET");

        let config = provider
            .build_ark_config("default_registry.ini".to_string())
            .await
            .unwrap();

        assert_eq!(config.ark_external_host, "ark.example.org");
        assert_eq!(config.ark_internal_host, "0.0.0.0");
        assert_eq!(config.ark_internal_port, "3336");
        assert_eq!(config.ark_naan, "00000");
        assert_eq!(config.ark_https_proxy, "true");
        assert_eq!(config.ark_registry, "default_registry.ini");
        assert_eq!(config.ark_github_secret, "");
    }

    #[test]
    fn test_default_regex_provider_compile_resource_iri_regex() {
        let provider = DefaultRegexProvider::new();
        let regex = provider.compile_resource_iri_regex().unwrap();

        assert!(regex.is_match("http://rdfh.ch/0001/test"));
        assert!(!regex.is_match("invalid_iri"));
    }

    #[test]
    fn test_default_regex_provider_compile_ark_path_regex() {
        let provider = DefaultRegexProvider::new();
        let regex = provider.compile_ark_path_regex("00000").unwrap();

        // Test with the actual regex pattern
        assert!(regex.is_match("ark:/00000/1/0001"));
        assert!(regex.is_match("ark:/00000/1/0001/resource"));
        assert!(!regex.is_match("invalid_ark_path"));
    }

    #[test]
    fn test_default_regex_provider_compile_v0_ark_path_regex() {
        let provider = DefaultRegexProvider::new();
        let regex = provider.compile_v0_ark_path_regex("00000").unwrap();

        // Test with the actual v0 regex pattern format: hex-alphanumeric-alphanumeric
        assert!(regex.is_match("ark:/00000/0002-779b9990a0c3f-6e"));
        assert!(regex.is_match("ark:/00000/0002-779b9990a0c3f-6e.20190129"));
        assert!(regex.is_match("ark:/00000/080e-76bb2132d30d6-0"));
        assert!(!regex.is_match("invalid_v0_ark_path"));
        assert!(!regex.is_match("ark:/00000/project/resource"));
    }
}
