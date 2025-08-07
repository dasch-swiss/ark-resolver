use std::collections::HashMap;
use std::sync::Arc;

use crate::core::domain::settings::{
    ArkConfig, CompiledRegexes, ProjectConfig, Settings, SettingsRegistry, SettingsWithRegexes,
};
use crate::core::errors::settings::{SettingsError, SettingsResult};
use crate::core::ports::settings::{
    ConfigurationProvider, EnvironmentProvider, RegexProvider, SettingsRepository,
    SettingsTransformer, SettingsValidator,
};

/// Use case for managing settings loading and configuration
pub struct SettingsManager {
    configuration_provider: Arc<dyn ConfigurationProvider>,
    environment_provider: Arc<dyn EnvironmentProvider>,
    regex_provider: Arc<dyn RegexProvider>,
    repository: Option<Arc<dyn SettingsRepository>>,
    #[allow(dead_code)]
    transformer: Arc<dyn SettingsTransformer>,
    validator: Arc<dyn SettingsValidator>,
}

impl SettingsManager {
    pub fn new(
        configuration_provider: Arc<dyn ConfigurationProvider>,
        environment_provider: Arc<dyn EnvironmentProvider>,
        regex_provider: Arc<dyn RegexProvider>,
        repository: Option<Arc<dyn SettingsRepository>>,
        transformer: Arc<dyn SettingsTransformer>,
        validator: Arc<dyn SettingsValidator>,
    ) -> Self {
        Self {
            configuration_provider,
            environment_provider,
            regex_provider,
            repository,
            transformer,
            validator,
        }
    }

    /// Load complete settings from configuration file
    pub async fn load_settings(&self) -> SettingsResult<SettingsWithRegexes> {
        // Check cache first if repository is available
        if let Some(repo) = &self.repository {
            if let Ok(Some(cached)) = repo.get_cached_settings("env_settings").await {
                return Ok(cached);
            }
        }

        // Load environment-based ARK configuration
        let registry_path = self
            .environment_provider
            .get_env_var("ARK_REGISTRY")
            .await?
            .ok_or_else(|| {
                crate::core::errors::settings::SettingsError::EnvironmentError(
                    "ARK_REGISTRY environment variable is required".to_string(),
                )
            })?;

        let ark_config = self
            .environment_provider
            .build_ark_config("".to_string()) // Empty string since no config file
            .await?;

        // Validate ARK configuration
        self.validator.validate_ark_config(&ark_config)?;

        // Load registry configuration
        let registry = self
            .configuration_provider
            .load_registry(&registry_path)
            .await?;

        // Validate registry
        self.validator.validate_registry(&registry)?;

        // Create settings domain object
        let settings = Settings::new(
            ark_config, registry, 1,         // DSP ARK version
            982451653, // Resource integer ID factor
        );

        // Validate complete settings
        settings
            .validate()
            .map_err(SettingsError::ValidationError)?;

        // Compile regex patterns
        let regexes = self.compile_regexes(&settings.ark_config.ark_naan).await?;

        let settings_with_regexes = SettingsWithRegexes::new(settings, regexes);

        // Cache the settings if repository is available
        if let Some(repo) = &self.repository {
            let _ = repo
                .cache_settings("env_settings", &settings_with_regexes)
                .await;
        }

        Ok(settings_with_regexes)
    }

    /// Load settings from environment variables only (minimal configuration)
    pub async fn load_minimal_settings(&self) -> SettingsResult<SettingsWithRegexes> {
        let registry_path = self
            .environment_provider
            .get_env_var("ARK_REGISTRY")
            .await?
            .ok_or_else(|| {
                crate::core::errors::settings::SettingsError::EnvironmentError(
                    "ARK_REGISTRY environment variable is required".to_string(),
                )
            })?;

        let ark_config = self
            .environment_provider
            .build_ark_config(registry_path)
            .await?;

        self.validator.validate_ark_config(&ark_config)?;

        // Create minimal registry with empty projects
        let registry = SettingsRegistry::new(HashMap::new(), HashMap::new());

        let settings = Settings::new(ark_config, registry, 1, 982451653);

        settings
            .validate()
            .map_err(SettingsError::ValidationError)?;

        let regexes = self.compile_regexes(&settings.ark_config.ark_naan).await?;

        Ok(SettingsWithRegexes::new(settings, regexes))
    }

    /// Reload settings and clear cache
    pub async fn reload_settings(&self) -> SettingsResult<SettingsWithRegexes> {
        // Clear cache first
        if let Some(repo) = &self.repository {
            repo.clear_cache().await?;
        }

        // Load fresh settings
        self.load_settings().await
    }

    /// Get project configuration with defaults merged
    pub fn get_project_config(
        &self,
        settings: &SettingsWithRegexes,
        project_id: &str,
    ) -> Option<ProjectConfig> {
        settings.settings.registry.get_project_config(project_id)
    }

    /// Get default configuration value
    pub fn get_default_config(&self, settings: &SettingsWithRegexes, key: &str) -> Option<String> {
        settings.settings.registry.get_default_config(key)
    }

    /// Match resource IRI using compiled regex
    pub fn match_resource_iri(
        &self,
        settings: &SettingsWithRegexes,
        resource_iri: &str,
    ) -> Option<(String, String)> {
        settings.regexes.match_resource_iri(resource_iri)
    }

    /// Match ARK path using compiled regex
    #[allow(clippy::type_complexity)]
    pub fn match_ark_path(
        &self,
        settings: &SettingsWithRegexes,
        ark_path: &str,
    ) -> Option<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> {
        settings.regexes.match_ark_path(ark_path)
    }

    /// Match V0 ARK path using compiled regex
    pub fn match_v0_ark_path(
        &self,
        settings: &SettingsWithRegexes,
        v0_ark_path: &str,
    ) -> Option<(Option<String>, Option<String>, Option<String>)> {
        settings.regexes.match_v0_ark_path(v0_ark_path)
    }

    /// Private helper to compile all regex patterns
    async fn compile_regexes(&self, ark_naan: &str) -> SettingsResult<CompiledRegexes> {
        let resource_iri_regex = self.regex_provider.compile_resource_iri_regex()?;
        let ark_path_regex = self.regex_provider.compile_ark_path_regex(ark_naan)?;
        let v0_ark_path_regex = self.regex_provider.compile_v0_ark_path_regex(ark_naan)?;

        Ok(CompiledRegexes::new(
            resource_iri_regex,
            ark_path_regex,
            v0_ark_path_regex,
        ))
    }
}

/// Default settings transformer implementation
pub struct DefaultSettingsTransformer;

impl SettingsTransformer for DefaultSettingsTransformer {
    fn transform_config_data(
        &self,
        raw_data: HashMap<String, serde_json::Value>,
    ) -> SettingsResult<HashMap<String, ProjectConfig>> {
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

        Ok(projects)
    }

    fn transform_ark_config(&self, config: &ArkConfig) -> HashMap<String, String> {
        config.to_config_map()
    }

    fn merge_configurations(
        &self,
        defaults: &HashMap<String, String>,
        project: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = defaults.clone();
        for (key, value) in project {
            merged.insert(key.clone(), value.clone());
        }
        merged
    }
}

/// Default settings validator implementation
pub struct DefaultSettingsValidator;

impl SettingsValidator for DefaultSettingsValidator {
    fn validate_ark_config(&self, config: &ArkConfig) -> SettingsResult<()> {
        config.validate().map_err(SettingsError::ValidationError)
    }

    fn validate_project_config(&self, config: &ProjectConfig) -> SettingsResult<()> {
        // Basic validation for project configuration
        if config.config.is_empty() {
            return Err(SettingsError::ValidationError(
                "Project configuration cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_registry(&self, registry: &SettingsRegistry) -> SettingsResult<()> {
        // Validate each project configuration
        for (project_id, project_config) in &registry.projects {
            self.validate_project_config(project_config).map_err(|e| {
                SettingsError::ValidationError(format!(
                    "Invalid project configuration for '{project_id}': {e}"
                ))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ports::settings::tests::{
        MockConfigurationProvider, MockEnvironmentProvider, MockRegexProvider,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_settings_manager_load_settings() {
        let config_provider = Arc::new(MockConfigurationProvider {
            registry_data: HashMap::new(),
            config_data: HashMap::new(),
        });
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "ARK_REGISTRY".to_string(),
            "tests/ark-registry.ini".to_string(),
        );
        let env_provider = Arc::new(MockEnvironmentProvider { env_vars });
        let regex_provider = Arc::new(MockRegexProvider);
        let transformer = Arc::new(DefaultSettingsTransformer);
        let validator = Arc::new(DefaultSettingsValidator);

        let manager = SettingsManager::new(
            config_provider,
            env_provider,
            regex_provider,
            None,
            transformer,
            validator,
        );

        let settings = manager.load_settings().await.unwrap();
        assert_eq!(settings.settings.dsp_ark_version, 1);
        assert_eq!(settings.settings.resource_int_id_factor, 982451653);
    }

    #[tokio::test]
    async fn test_settings_manager_load_minimal_settings() {
        let config_provider = Arc::new(MockConfigurationProvider {
            registry_data: HashMap::new(),
            config_data: HashMap::new(),
        });
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "ARK_REGISTRY".to_string(),
            "tests/ark-registry.ini".to_string(),
        );
        let env_provider = Arc::new(MockEnvironmentProvider { env_vars });
        let regex_provider = Arc::new(MockRegexProvider);
        let transformer = Arc::new(DefaultSettingsTransformer);
        let validator = Arc::new(DefaultSettingsValidator);

        let manager = SettingsManager::new(
            config_provider,
            env_provider,
            regex_provider,
            None,
            transformer,
            validator,
        );

        let settings = manager.load_minimal_settings().await.unwrap();
        assert_eq!(settings.settings.dsp_ark_version, 1);
        assert!(settings.settings.registry.projects.is_empty());
    }

    #[test]
    fn test_default_settings_transformer() {
        let transformer = DefaultSettingsTransformer;

        let mut raw_data = HashMap::new();
        raw_data.insert(
            "project1".to_string(),
            serde_json::json!({
                "key1": "value1",
                "key2": "value2"
            }),
        );
        raw_data.insert(
            "DEFAULT".to_string(),
            serde_json::json!({
                "default_key": "default_value"
            }),
        );

        let result = transformer.transform_config_data(raw_data).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("project1"));
        assert!(!result.contains_key("DEFAULT"));

        let project_config = result.get("project1").unwrap();
        assert_eq!(project_config.get("key1"), Some(&"value1".to_string()));
        assert_eq!(project_config.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_default_settings_validator() {
        let validator = DefaultSettingsValidator;

        let valid_config = ArkConfig::new(
            "ark.example.org".to_string(),
            "0.0.0.0".to_string(),
            "3336".to_string(),
            "00000".to_string(),
            "true".to_string(),
            "registry.ini".to_string(),
            "secret".to_string(),
        );
        assert!(validator.validate_ark_config(&valid_config).is_ok());

        let invalid_config = ArkConfig::new(
            "".to_string(),
            "0.0.0.0".to_string(),
            "3336".to_string(),
            "00000".to_string(),
            "true".to_string(),
            "registry.ini".to_string(),
            "secret".to_string(),
        );
        assert!(validator.validate_ark_config(&invalid_config).is_err());
    }
}
