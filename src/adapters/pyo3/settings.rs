use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use crate::adapters::environment::settings::{DefaultRegexProvider, EnvironmentVariableProvider};
use crate::adapters::file_system::settings::FileSystemConfigurationProvider;
use crate::core::domain::settings::SettingsWithRegexes;
use crate::core::errors::settings::SettingsError;
use crate::core::use_cases::settings_manager::{
    DefaultSettingsTransformer, DefaultSettingsValidator, SettingsManager,
};

/// Python wrapper for project configuration
#[pyclass]
#[derive(Debug, Clone)]
pub struct ConfigWrapper {
    config: HashMap<String, String>,
}

#[pymethods]
impl ConfigWrapper {
    #[new]
    pub fn new(config: HashMap<String, String>) -> Self {
        Self { config }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    pub fn get_boolean(&self, key: &str) -> PyResult<bool> {
        match self.config.get(key) {
            Some(value) => match value.as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid boolean value for key '{key}': {value}"
                ))),
            },
            None => Ok(false),
        }
    }
}

impl From<HashMap<String, String>> for ConfigWrapper {
    fn from(config: HashMap<String, String>) -> Self {
        Self { config }
    }
}

/// Python wrapper for ARK URL settings using hexagonal architecture
#[pyclass]
#[derive(Debug, Clone)]
pub struct ArkUrlSettings {
    settings: SettingsWithRegexes,
    #[pyo3(get)]
    pub ark_config: ConfigWrapper,
    #[pyo3(get)]
    pub default_config: HashMap<String, String>,
    #[pyo3(get)]
    pub dsp_ark_version: u8,
    #[pyo3(get)]
    pub resource_int_id_factor: u32,
}

#[pymethods]
impl ArkUrlSettings {
    #[new]
    #[pyo3(text_signature = "()")]
    pub fn new() -> PyResult<Self> {
        // Create runtime for async operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let settings = rt
            .block_on(async {
                // Create adapters
                let config_provider = Arc::new(FileSystemConfigurationProvider::new());
                let env_provider = Arc::new(EnvironmentVariableProvider::new());
                let regex_provider = Arc::new(DefaultRegexProvider::new());
                let transformer = Arc::new(DefaultSettingsTransformer);
                let validator = Arc::new(DefaultSettingsValidator);

                // Create settings manager
                let manager = SettingsManager::new(
                    config_provider,
                    env_provider,
                    regex_provider,
                    None, // No repository for now
                    transformer,
                    validator,
                );

                // Load settings using environment variables only (no config file)
                manager.load_settings().await
            })
            .map_err(|e: SettingsError| match e {
                SettingsError::FileSystemError(msg) => pyo3::exceptions::PyIOError::new_err(msg),
                SettingsError::ParseError(msg) => pyo3::exceptions::PyValueError::new_err(msg),
                SettingsError::ValidationError(msg) => pyo3::exceptions::PyValueError::new_err(msg),
                SettingsError::EnvironmentError(msg) => {
                    pyo3::exceptions::PyRuntimeError::new_err(msg)
                }
                SettingsError::RegexError(msg) => pyo3::exceptions::PyValueError::new_err(msg),
                _ => pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
            })?;

        Ok(Self::from_settings(settings))
    }

    #[pyo3(text_signature = "(self, key)")]
    pub fn get_default_config(&self, key: &str) -> Option<String> {
        self.default_config.get(key).cloned()
    }

    /// Get a project configuration section from the registry
    #[pyo3(text_signature = "(self, project_id)")]
    pub fn get_project_config(&self, project_id: &str) -> Option<ConfigWrapper> {
        self.settings
            .settings
            .registry
            .get_project_config(project_id)
            .map(|config| ConfigWrapper::from(config.config))
    }

    /// Check resource IRI
    #[pyo3(text_signature = "(self, resource_iri)")]
    pub fn match_resource_iri(&self, resource_iri: &str) -> Option<(String, String)> {
        self.settings.regexes.match_resource_iri(resource_iri)
    }

    /// Check structure and extract ARK path components
    /// Returns a tuple with ARK version, project ID, resource ID, value ID and timestamp
    #[allow(clippy::type_complexity)]
    #[pyo3(text_signature = "(self, ark_path)")]
    pub fn match_ark_path(
        &self,
        ark_path: &str,
    ) -> Option<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> {
        self.settings.regexes.match_ark_path(ark_path)
    }

    /// Check if a URL matches the V0 ARK path regex
    /// Returns a tuple with project ID, resource ID and optional timestamp
    #[pyo3(text_signature = "(self, v0_ark_path)")]
    pub fn match_v0_ark_path(
        &self,
        v0_ark_path: &str,
    ) -> Option<(Option<String>, Option<String>, Option<String>)> {
        self.settings.regexes.match_v0_ark_path(v0_ark_path)
    }
}

impl ArkUrlSettings {
    fn from_settings(settings: SettingsWithRegexes) -> Self {
        let ark_config = ConfigWrapper::from(settings.settings.ark_config.to_config_map());
        let default_config = settings.settings.registry.default_config.clone();
        let dsp_ark_version = settings.settings.dsp_ark_version;
        let resource_int_id_factor = settings.settings.resource_int_id_factor;

        Self {
            settings,
            ark_config,
            default_config,
            dsp_ark_version,
            resource_int_id_factor,
        }
    }
}

/// Expose a direct function for Python to load settings
#[pyfunction]
pub fn load_settings() -> PyResult<ArkUrlSettings> {
    ArkUrlSettings::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_wrapper_boolean_parsing() {
        let mut config = HashMap::new();
        config.insert("true_val".to_string(), "true".to_string());
        config.insert("false_val".to_string(), "false".to_string());
        config.insert("one_val".to_string(), "1".to_string());
        config.insert("zero_val".to_string(), "0".to_string());
        config.insert("invalid_val".to_string(), "invalid".to_string());

        let wrapper = ConfigWrapper::new(config);

        assert_eq!(wrapper.get_boolean("true_val").unwrap(), true);
        assert_eq!(wrapper.get_boolean("false_val").unwrap(), false);
        assert_eq!(wrapper.get_boolean("one_val").unwrap(), true);
        assert_eq!(wrapper.get_boolean("zero_val").unwrap(), false);
        assert_eq!(wrapper.get_boolean("missing_val").unwrap(), false);
        assert!(wrapper.get_boolean("invalid_val").is_err());
    }

    #[test]
    fn test_config_wrapper_get() {
        let mut config = HashMap::new();
        config.insert("key1".to_string(), "value1".to_string());
        config.insert("key2".to_string(), "value2".to_string());

        let wrapper = ConfigWrapper::new(config);

        assert_eq!(wrapper.get("key1"), Some(&"value1".to_string()));
        assert_eq!(wrapper.get("key2"), Some(&"value2".to_string()));
        assert_eq!(wrapper.get("missing"), None);
    }

    #[test]
    fn test_config_wrapper_from_hashmap() {
        let mut config = HashMap::new();
        config.insert("test_key".to_string(), "test_value".to_string());

        let wrapper = ConfigWrapper::from(config);
        assert_eq!(wrapper.get("test_key"), Some(&"test_value".to_string()));
    }

    // Note: Testing ArkUrlSettings requires actual file system and environment setup
    // These tests would be integration tests that require the full application context
}
