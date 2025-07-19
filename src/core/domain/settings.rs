use regex::Regex;
use std::collections::HashMap;

/// Core domain configuration for ARK URL settings
#[derive(Debug, Clone)]
pub struct ArkConfig {
    pub ark_external_host: String,
    pub ark_internal_host: String,
    pub ark_internal_port: String,
    pub ark_naan: String,
    pub ark_https_proxy: String,
    pub ark_registry: String,
    pub ark_github_secret: String,
}

impl ArkConfig {
    pub fn new(
        ark_external_host: String,
        ark_internal_host: String,
        ark_internal_port: String,
        ark_naan: String,
        ark_https_proxy: String,
        ark_registry: String,
        ark_github_secret: String,
    ) -> Self {
        Self {
            ark_external_host,
            ark_internal_host,
            ark_internal_port,
            ark_naan,
            ark_https_proxy,
            ark_registry,
            ark_github_secret,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ark_external_host.is_empty() {
            return Err("ARK external host cannot be empty".to_string());
        }
        if self.ark_internal_host.is_empty() {
            return Err("ARK internal host cannot be empty".to_string());
        }
        if self.ark_internal_port.is_empty() {
            return Err("ARK internal port cannot be empty".to_string());
        }
        if self.ark_naan.is_empty() {
            return Err("ARK NAAN cannot be empty".to_string());
        }
        Ok(())
    }

    pub fn to_config_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert(
            "ArkExternalHost".to_string(),
            self.ark_external_host.clone(),
        );
        map.insert(
            "ArkInternalHost".to_string(),
            self.ark_internal_host.clone(),
        );
        map.insert(
            "ArkInternalPort".to_string(),
            self.ark_internal_port.clone(),
        );
        map.insert("ArkNaan".to_string(), self.ark_naan.clone());
        map.insert("ArkHttpsProxy".to_string(), self.ark_https_proxy.clone());
        map.insert("ArkRegistry".to_string(), self.ark_registry.clone());
        map.insert(
            "ArkGithubSecret".to_string(),
            self.ark_github_secret.clone(),
        );
        map
    }
}

/// Project-specific configuration wrapper
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub config: HashMap<String, String>,
}

impl ProjectConfig {
    pub fn new(config: HashMap<String, String>) -> Self {
        Self { config }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    pub fn get_boolean(&self, key: &str) -> Result<bool, String> {
        match self.config.get(key) {
            Some(value) => match value.as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(format!(
                    "Invalid boolean value for key '{}': {}",
                    key, value
                )),
            },
            None => Ok(false),
        }
    }

    pub fn merge_with_defaults(&self, defaults: &HashMap<String, String>) -> Self {
        let mut merged = defaults.clone();
        for (key, value) in &self.config {
            merged.insert(key.clone(), value.clone());
        }
        Self::new(merged)
    }
}

/// Registry of all project configurations
#[derive(Debug, Clone)]
pub struct SettingsRegistry {
    pub projects: HashMap<String, ProjectConfig>,
    pub default_config: HashMap<String, String>,
}

impl SettingsRegistry {
    pub fn new(
        projects: HashMap<String, ProjectConfig>,
        default_config: HashMap<String, String>,
    ) -> Self {
        Self {
            projects,
            default_config,
        }
    }

    pub fn get_project_config(&self, project_id: &str) -> Option<ProjectConfig> {
        self.projects
            .get(&project_id.to_lowercase())
            .map(|config| config.merge_with_defaults(&self.default_config))
    }

    pub fn get_default_config(&self, key: &str) -> Option<String> {
        self.default_config.get(key).cloned()
    }
}

/// Core settings domain object containing all configuration data
#[derive(Debug, Clone)]
pub struct Settings {
    pub ark_config: ArkConfig,
    pub registry: SettingsRegistry,
    pub dsp_ark_version: u8,
    pub resource_int_id_factor: u32,
}

impl Settings {
    pub fn new(
        ark_config: ArkConfig,
        registry: SettingsRegistry,
        dsp_ark_version: u8,
        resource_int_id_factor: u32,
    ) -> Self {
        Self {
            ark_config,
            registry,
            dsp_ark_version,
            resource_int_id_factor,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.ark_config.validate()?;

        if self.dsp_ark_version == 0 {
            return Err("DSP ARK version cannot be 0".to_string());
        }

        if self.resource_int_id_factor == 0 {
            return Err("Resource integer ID factor cannot be 0".to_string());
        }

        Ok(())
    }
}

/// Compiled regex patterns for ARK URL matching
#[derive(Debug, Clone)]
pub struct CompiledRegexes {
    pub resource_iri_regex: Regex,
    pub ark_path_regex: Regex,
    pub v0_ark_path_regex: Regex,
}

impl CompiledRegexes {
    pub fn new(resource_iri_regex: Regex, ark_path_regex: Regex, v0_ark_path_regex: Regex) -> Self {
        Self {
            resource_iri_regex,
            ark_path_regex,
            v0_ark_path_regex,
        }
    }

    /// Match resource IRI and extract components
    pub fn match_resource_iri(&self, resource_iri: &str) -> Option<(String, String)> {
        self.resource_iri_regex
            .captures(resource_iri)
            .map(|captures| {
                (
                    captures.get(1).map_or("", |m| m.as_str()).to_string(),
                    captures.get(2).map_or("", |m| m.as_str()).to_string(),
                )
            })
    }

    /// Match ARK path and extract components
    #[allow(clippy::type_complexity)]
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
        let (processed, timestamp) = if let Some(index) = ark_path.find('.') {
            (&ark_path[..index], Some(&ark_path[index + 1..]))
        } else {
            (ark_path, None)
        };

        self.ark_path_regex.captures(processed).map(|captures| {
            (
                captures.get(1).map(|m| m.as_str().to_string()),
                captures.get(2).map(|m| m.as_str().to_string()),
                captures.get(3).map(|m| m.as_str().to_string()),
                captures.get(4).map(|m| m.as_str().to_string()),
                timestamp.map(|m| m.to_string()),
            )
        })
    }

    /// Match V0 ARK path and extract components
    pub fn match_v0_ark_path(
        &self,
        v0_ark_path: &str,
    ) -> Option<(Option<String>, Option<String>, Option<String>)> {
        self.v0_ark_path_regex
            .captures(v0_ark_path)
            .map(|captures| {
                (
                    captures.get(1).map(|m| m.as_str().to_string()),
                    captures.get(2).map(|m| m.as_str().to_string()),
                    captures.get(3).map(|m| m.as_str().to_string()),
                )
            })
    }
}

/// Complete settings domain object with compiled regexes
#[derive(Debug, Clone)]
pub struct SettingsWithRegexes {
    pub settings: Settings,
    pub regexes: CompiledRegexes,
}

impl SettingsWithRegexes {
    pub fn new(settings: Settings, regexes: CompiledRegexes) -> Self {
        Self { settings, regexes }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.settings.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ark_config_validation() {
        let valid_config = ArkConfig::new(
            "ark.example.org".to_string(),
            "0.0.0.0".to_string(),
            "3336".to_string(),
            "00000".to_string(),
            "true".to_string(),
            "registry.ini".to_string(),
            "secret".to_string(),
        );
        assert!(valid_config.validate().is_ok());

        let invalid_config = ArkConfig::new(
            "".to_string(),
            "0.0.0.0".to_string(),
            "3336".to_string(),
            "00000".to_string(),
            "true".to_string(),
            "registry.ini".to_string(),
            "secret".to_string(),
        );
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_project_config_boolean_parsing() {
        let mut config_map = HashMap::new();
        config_map.insert("BoolTrue".to_string(), "true".to_string());
        config_map.insert("BoolFalse".to_string(), "false".to_string());
        config_map.insert("BoolOne".to_string(), "1".to_string());
        config_map.insert("BoolZero".to_string(), "0".to_string());
        config_map.insert("BoolInvalid".to_string(), "invalid".to_string());

        let config = ProjectConfig::new(config_map);

        assert_eq!(config.get_boolean("BoolTrue"), Ok(true));
        assert_eq!(config.get_boolean("BoolFalse"), Ok(false));
        assert_eq!(config.get_boolean("BoolOne"), Ok(true));
        assert_eq!(config.get_boolean("BoolZero"), Ok(false));
        assert_eq!(config.get_boolean("NonExistent"), Ok(false));
        assert!(config.get_boolean("BoolInvalid").is_err());
    }

    #[test]
    fn test_project_config_merge_with_defaults() {
        let mut defaults = HashMap::new();
        defaults.insert("DefaultKey".to_string(), "default_value".to_string());
        defaults.insert("OverrideKey".to_string(), "default_override".to_string());

        let mut project_config = HashMap::new();
        project_config.insert("ProjectKey".to_string(), "project_value".to_string());
        project_config.insert("OverrideKey".to_string(), "project_override".to_string());

        let config = ProjectConfig::new(project_config);
        let merged = config.merge_with_defaults(&defaults);

        assert_eq!(merged.get("DefaultKey"), Some(&"default_value".to_string()));
        assert_eq!(merged.get("ProjectKey"), Some(&"project_value".to_string()));
        assert_eq!(
            merged.get("OverrideKey"),
            Some(&"project_override".to_string())
        );
    }

    #[test]
    fn test_settings_registry() {
        let mut defaults = HashMap::new();
        defaults.insert("DefaultKey".to_string(), "default_value".to_string());

        let mut project_config = HashMap::new();
        project_config.insert("ProjectKey".to_string(), "project_value".to_string());

        let mut projects = HashMap::new();
        projects.insert("test".to_string(), ProjectConfig::new(project_config));

        let registry = SettingsRegistry::new(projects, defaults);

        let config = registry.get_project_config("test").unwrap();
        assert_eq!(config.get("DefaultKey"), Some(&"default_value".to_string()));
        assert_eq!(config.get("ProjectKey"), Some(&"project_value".to_string()));

        let config_upper = registry.get_project_config("TEST").unwrap();
        assert_eq!(
            config_upper.get("ProjectKey"),
            Some(&"project_value".to_string())
        );

        assert!(registry.get_project_config("nonexistent").is_none());
    }

    #[test]
    fn test_settings_validation() {
        let ark_config = ArkConfig::new(
            "ark.example.org".to_string(),
            "0.0.0.0".to_string(),
            "3336".to_string(),
            "00000".to_string(),
            "true".to_string(),
            "registry.ini".to_string(),
            "secret".to_string(),
        );

        let registry = SettingsRegistry::new(HashMap::new(), HashMap::new());

        let valid_settings = Settings::new(ark_config.clone(), registry.clone(), 1, 982451653);
        assert!(valid_settings.validate().is_ok());

        let invalid_version = Settings::new(ark_config.clone(), registry.clone(), 0, 982451653);
        assert!(invalid_version.validate().is_err());

        let invalid_factor = Settings::new(ark_config, registry, 1, 0);
        assert!(invalid_factor.validate().is_err());
    }
}
