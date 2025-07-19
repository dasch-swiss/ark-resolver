use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;

use crate::core::domain::settings::{ArkConfig, ProjectConfig, SettingsRegistry};
use crate::core::errors::settings::SettingsResult;

/// Abstract trait for loading configuration data from various sources
#[async_trait]
pub trait ConfigurationProvider: Send + Sync {
    /// Load configuration from a registry file
    async fn load_registry(&self, registry_path: &str) -> SettingsResult<SettingsRegistry>;

    /// Load configuration from a config file
    async fn load_config(&self, config_path: &str) -> SettingsResult<HashMap<String, String>>;
}

/// Abstract trait for accessing environment variables
#[async_trait]
pub trait EnvironmentProvider: Send + Sync {
    /// Get environment variable value
    async fn get_env_var(&self, key: &str) -> SettingsResult<Option<String>>;

    /// Get environment variable with default value
    async fn get_env_var_or_default(&self, key: &str, default: &str) -> SettingsResult<String>;

    /// Build ARK configuration from environment variables
    async fn build_ark_config(&self, registry_path: String) -> SettingsResult<ArkConfig>;
}

/// Abstract trait for regex pattern compilation and matching
pub trait RegexProvider: Send + Sync {
    /// Compile resource IRI regex pattern
    fn compile_resource_iri_regex(&self) -> SettingsResult<Regex>;

    /// Compile ARK path regex pattern with NAAN
    fn compile_ark_path_regex(&self, ark_naan: &str) -> SettingsResult<Regex>;

    /// Compile V0 ARK path regex pattern with NAAN
    fn compile_v0_ark_path_regex(&self, ark_naan: &str) -> SettingsResult<Regex>;
}

/// Abstract trait for persisting/caching settings
#[async_trait]
pub trait SettingsRepository: Send + Sync {
    /// Cache compiled settings
    async fn cache_settings(
        &self,
        key: &str,
        settings: &crate::core::domain::settings::SettingsWithRegexes,
    ) -> SettingsResult<()>;

    /// Retrieve cached settings
    async fn get_cached_settings(
        &self,
        key: &str,
    ) -> SettingsResult<Option<crate::core::domain::settings::SettingsWithRegexes>>;

    /// Clear settings cache
    async fn clear_cache(&self) -> SettingsResult<()>;
}

/// Abstract trait for file system operations
#[async_trait]
pub trait FileSystemProvider: Send + Sync {
    /// Check if file exists
    async fn file_exists(&self, path: &str) -> SettingsResult<bool>;

    /// Read file contents
    async fn read_file(&self, path: &str) -> SettingsResult<String>;

    /// Get file metadata (modification time, size, etc.)
    async fn get_file_metadata(&self, path: &str) -> SettingsResult<HashMap<String, String>>;
}

/// Port for settings validation
pub trait SettingsValidator: Send + Sync {
    /// Validate ARK configuration
    fn validate_ark_config(&self, config: &ArkConfig) -> SettingsResult<()>;

    /// Validate project configuration
    fn validate_project_config(&self, config: &ProjectConfig) -> SettingsResult<()>;

    /// Validate registry configuration
    fn validate_registry(&self, registry: &SettingsRegistry) -> SettingsResult<()>;
}

/// Port for settings transformation
pub trait SettingsTransformer: Send + Sync {
    /// Transform raw configuration data to structured format
    fn transform_config_data(
        &self,
        raw_data: HashMap<String, serde_json::Value>,
    ) -> SettingsResult<HashMap<String, ProjectConfig>>;

    /// Transform ARK config to key-value map
    fn transform_ark_config(&self, config: &ArkConfig) -> HashMap<String, String>;

    /// Merge default and project-specific configurations
    fn merge_configurations(
        &self,
        defaults: &HashMap<String, String>,
        project: &HashMap<String, String>,
    ) -> HashMap<String, String>;
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Arc;

    /// Mock implementation for testing
    pub struct MockConfigurationProvider {
        pub registry_data: HashMap<String, serde_json::Value>,
        pub config_data: HashMap<String, String>,
    }

    #[async_trait]
    impl ConfigurationProvider for MockConfigurationProvider {
        async fn load_registry(&self, _registry_path: &str) -> SettingsResult<SettingsRegistry> {
            // Mock implementation for testing
            let mut projects = HashMap::new();
            let mut defaults = HashMap::new();
            defaults.insert("DefaultKey".to_string(), "DefaultValue".to_string());

            Ok(SettingsRegistry::new(projects, defaults))
        }

        async fn load_config(&self, _config_path: &str) -> SettingsResult<HashMap<String, String>> {
            Ok(self.config_data.clone())
        }
    }

    /// Mock environment provider for testing
    pub struct MockEnvironmentProvider {
        pub env_vars: HashMap<String, String>,
    }

    #[async_trait]
    impl EnvironmentProvider for MockEnvironmentProvider {
        async fn get_env_var(&self, key: &str) -> SettingsResult<Option<String>> {
            Ok(self.env_vars.get(key).cloned())
        }

        async fn get_env_var_or_default(&self, key: &str, default: &str) -> SettingsResult<String> {
            Ok(self
                .env_vars
                .get(key)
                .unwrap_or(&default.to_string())
                .clone())
        }

        async fn build_ark_config(&self, registry_path: String) -> SettingsResult<ArkConfig> {
            Ok(ArkConfig::new(
                self.get_env_var_or_default("ARK_EXTERNAL_HOST", "ark.example.org")
                    .await?,
                self.get_env_var_or_default("ARK_INTERNAL_HOST", "0.0.0.0")
                    .await?,
                self.get_env_var_or_default("ARK_INTERNAL_PORT", "3336")
                    .await?,
                self.get_env_var_or_default("ARK_NAAN", "00000").await?,
                self.get_env_var_or_default("ARK_HTTPS_PROXY", "true")
                    .await?,
                registry_path,
                self.get_env_var_or_default("ARK_GITHUB_SECRET", "").await?,
            ))
        }
    }

    /// Mock regex provider for testing
    pub struct MockRegexProvider;

    impl RegexProvider for MockRegexProvider {
        fn compile_resource_iri_regex(&self) -> SettingsResult<Regex> {
            Ok(Regex::new(r"^http://rdfh\.ch/([^/]+)/(.+)$").unwrap())
        }

        fn compile_ark_path_regex(&self, _ark_naan: &str) -> SettingsResult<Regex> {
            Ok(Regex::new(r"^ark:/00000/(\d)/([^/]+)(?:/([^/]+))?(?:/([^/]+))?$").unwrap())
        }

        fn compile_v0_ark_path_regex(&self, _ark_naan: &str) -> SettingsResult<Regex> {
            Ok(Regex::new(r"^ark:/00000/([^/]+)/([^/]+)(?:/([^/]+))?$").unwrap())
        }
    }

    #[tokio::test]
    async fn test_configuration_provider_trait() {
        let provider = MockConfigurationProvider {
            registry_data: HashMap::new(),
            config_data: HashMap::new(),
        };

        let registry = provider.load_registry("test.ini").await.unwrap();
        assert!(registry.projects.is_empty());

        let config = provider.load_config("test.ini").await.unwrap();
        assert!(config.is_empty());
    }

    #[tokio::test]
    async fn test_environment_provider_trait() {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());

        let provider = MockEnvironmentProvider { env_vars };

        let value = provider.get_env_var("TEST_VAR").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        let default_value = provider
            .get_env_var_or_default("MISSING_VAR", "default")
            .await
            .unwrap();
        assert_eq!(default_value, "default");
    }

    #[test]
    fn test_regex_provider_trait() {
        let provider = MockRegexProvider;

        let resource_regex = provider.compile_resource_iri_regex().unwrap();
        assert!(resource_regex.is_match("http://rdfh.ch/0001/test"));

        let ark_regex = provider.compile_ark_path_regex("00000").unwrap();
        assert!(ark_regex.is_match("ark:/00000/1/0001"));

        let v0_regex = provider.compile_v0_ark_path_regex("00000").unwrap();
        assert!(v0_regex.is_match("ark:/00000/project/resource"));
    }
}
