use crate::parsing::{ark_path_regex, resource_iri_regex, v0_ark_path_regex};
use config::{Config, File, FileFormat};
use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::env;

struct ArkConfig {
    ark_external_host: String,
    ark_internal_host: String,
    ark_internal_port: String,
    ark_naan: String,
    ark_https_proxy: String,
    ark_registry: String,
    ark_github_secret: String,
}

impl From<ArkConfig> for ConfigWrapper {
    fn from(value: ArkConfig) -> Self {
        let mut map = HashMap::new();
        map.insert("ArkExternalHost".to_string(), value.ark_external_host);
        map.insert("ArkInternalHost".to_string(), value.ark_internal_host);
        map.insert("ArkInternalPort".to_string(), value.ark_internal_port);
        map.insert("ArkNaan".to_string(), value.ark_naan);
        map.insert("ArkHttpsProxy".to_string(), value.ark_https_proxy);
        map.insert("ArkRegistry".to_string(), value.ark_registry);
        map.insert("ArkGithubSecret".to_string(), value.ark_github_secret);

        ConfigWrapper { config: map }
    }
}

/// Wrapper for config settings, providing helper methods
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct ConfigWrapper {
    config: HashMap<String, String>,
}

impl From<HashMap<String, String>> for ConfigWrapper {
    fn from(config: HashMap<String, String>) -> Self {
        Self { config }
    }
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
                    "Invalid boolean value for key '{}': {}",
                    key, value
                ))),
            },
            None => Ok(false),
        }
    }
}

#[pyclass]
#[derive(Debug)]
pub struct ArkUrlSettings {
    #[pyo3(get)]
    ark_config: ConfigWrapper,
    #[pyo3(get)]
    default_config: HashMap<String, String>,
    registry: HashMap<String, ConfigWrapper>,
    #[pyo3(get)]
    dsp_ark_version: u8,
    #[pyo3(get)]
    resource_int_id_factor: u32,
    resource_iri_regex: Regex,
    ark_path_regex: Regex,
    v0_ark_path_regex: Regex,
}

#[pymethods]
impl ArkUrlSettings {
    #[new]
    #[pyo3(text_signature = "(config_path)")]
    pub fn new(config_path: String) -> PyResult<Self> {
        let settings = new_impl(config_path).map_err(pyo3::exceptions::PyIOError::new_err)?;
        Ok(settings)
    }

    #[pyo3(text_signature = "(self, key)")]
    pub fn get_default_config(&self, key: &str) -> Option<String> {
        self.default_config.get(key).map(|s| s.to_string())
    }

    /// Get a project configuration section from the registry
    #[pyo3(text_signature = "(self, project_id)")]
    pub fn get_project_config(&self, project_id: &str) -> Option<ConfigWrapper> {
        let mut defaults = self.default_config.clone();
        self.registry
            .get(&project_id.to_lowercase())?
            .clone()
            .config
            .into_iter()
            .for_each(|(k, v)| {
                defaults.insert(k, v);
            });
        Some(defaults.into())
    }

    /// Check resource IRI
    #[pyo3(text_signature = "(self, resource_iri)")]
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

    /// Check if a URL matches the V0 ARK path regex
    /// Returns a tuple with project ID, resource ID and optional timestamp
    #[pyo3(text_signature = "(self, v0_ark_path)")]
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

/// Expose a direct function for Python to load settings
#[pyfunction]
pub fn load_settings(config_path: String) -> PyResult<ArkUrlSettings> {
    ArkUrlSettings::new(config_path)
}

fn new_impl(_config_path: String) -> Result<ArkUrlSettings, String> {
    let registry_path =
        env::var("ARK_REGISTRY").unwrap_or("python/src/ark_resolver/ark-registry.ini".to_string());

    let ark_config: ConfigWrapper = ArkConfig {
        ark_external_host: env::var("ARK_EXTERNAL_HOST").unwrap_or("ark.example.org".to_string()),
        ark_internal_host: env::var("ARK_INTERNAL_HOST").unwrap_or("0.0.0.0".to_string()),
        ark_internal_port: env::var("ARK_INTERNAL_PORT").unwrap_or("3336".to_string()),
        ark_naan: env::var("ARK_NAAN").unwrap_or("00000".to_string()),
        ark_https_proxy: env::var("ARK_HTTPS_PROXY").unwrap_or("true".to_string()),
        ark_registry: registry_path.clone(),
        ark_github_secret: env::var("ARK_GITHUB_SECRET").unwrap_or("".to_string()),
    }
    .into();

    let registry_ini = Config::builder()
        .add_source(File::with_name(&registry_path).format(FileFormat::Ini))
        .build()
        .map_err(|e| e.to_string())?;

    // Deserialize into a nested map
    let raw_registry: HashMap<String, serde_json::Value> =
        registry_ini.try_deserialize().map_err(|e| e.to_string())?;

    // Extract DEFAULT section separately
    let default_section = raw_registry
        .get("DEFAULT")
        .and_then(|v| v.as_object())
        .map(|default_map| {
            default_map
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect::<HashMap<String, String>>()
        })
        .unwrap_or_default(); // Use empty map if no DEFAULT section

    let mut registry: HashMap<String, ConfigWrapper> = HashMap::new();

    // Convert JSON-like structure into Rust nested HashMap
    for (section, value) in raw_registry.iter() {
        if section == "DEFAULT" {
            continue; // Skip DEFAULT section
        }
        let mut section_map = HashMap::new(); // Start with an empty map

        if let Some(inner_map) = value.as_object() {
            for (key, inner_value) in inner_map {
                section_map.insert(key.clone(), inner_value.as_str().unwrap_or("").to_string());
            }
        }

        registry.insert(section.to_lowercase(), ConfigWrapper::new(section_map));
    }

    let default_ark_naan = "00000".to_string();
    let ark_naan = default_section.get("ArkNaan").unwrap_or(&default_ark_naan);

    Ok(ArkUrlSettings {
        ark_config,
        default_config: default_section.clone(),
        registry: registry.clone(),
        dsp_ark_version: 1,
        resource_int_id_factor: 982451653,
        resource_iri_regex: resource_iri_regex(),
        ark_path_regex: ark_path_regex(ark_naan),
        v0_ark_path_regex: v0_ark_path_regex(ark_naan),
    })
}

#[cfg(test)]
mod tests {
    use crate::ark_url_settings::new_impl;

    #[test]
    fn test_match_ark_path_impl() {
        let settings = new_impl("python/src/ark_resolver/ark-config.ini".to_string()).unwrap();

        // project
        let captures = settings.match_ark_path("ark:/00000/1/0003").unwrap();
        assert_eq!(
            captures,
            (
                Some("1".to_string()),
                Some("0003".to_string()),
                None,
                None,
                None
            )
        );

        // resource
        let captures = settings
            .match_ark_path("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
            .unwrap();
        assert_eq!(
            captures,
            (
                Some("1".to_string()),
                Some("0001".to_string()),
                Some("cmfk1DMHRBiR4=_6HXpEFAn".to_string()),
                None,
                None
            )
        );

        // resource with timestamp
        let captures = settings
            .match_ark_path("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z")
            .unwrap();
        assert_eq!(
            captures,
            (
                Some("1".to_string()),
                Some("0001".to_string()),
                Some("cmfk1DMHRBiR4=_6HXpEFAn".to_string()),
                None,
                Some("20180604T085622Z".to_string())
            )
        );

        // resource with value
        let captures = settings
            .match_ark_path("ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo")
            .unwrap();
        assert_eq!(
            captures,
            (
                Some("1".to_string()),
                Some("0005".to_string()),
                Some("SQkTPdHdTzq_gqbwj6QR=AR".to_string()),
                Some("=SSbnPK3Q7WWxzBT1UPpRgo".to_string()),
                None
            )
        );

        // resource with value and timestamp
        let captures = settings.match_ark_path("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z").unwrap();
        assert_eq!(
            captures,
            (
                Some("1".to_string()),
                Some("0001".to_string()),
                Some("cmfk1DMHRBiR4=_6HXpEFAn".to_string()),
                Some("pLlW4ODASumZfZFbJdpw1gu".to_string()),
                Some("20180604T085622Z".to_string())
            )
        );
    }

    #[test]
    fn test_settings() {
        let settings = new_impl("python/src/ark_resolver/ark-config.ini".to_string()).unwrap();

        assert_eq!(
            settings.ark_config.get("ArkNaan"),
            Some(&"00000".to_string())
        );
        assert_eq!(
            settings.ark_config.get("ArkExternalHost"),
            Some(&"ark.example.org".to_string())
        );
        assert_eq!(
            settings.ark_config.get("ArkInternalHost"),
            Some(&"0.0.0.0".to_string())
        );
        assert_eq!(
            settings.ark_config.get("ArkInternalPort"),
            Some(&"3336".to_string())
        );
        assert_eq!(
            settings.ark_config.get("ArkHttpsProxy"),
            Some(&"true".to_string())
        );
        assert_eq!(
            settings.default_config.get("TopLevelObjectUrl"),
            Some(&"http://dasch.swiss".to_string())
        );
        assert_eq!(
            settings.get_default_config("TopLevelObjectUrl"),
            Some("http://dasch.swiss".to_string())
        );
        assert_eq!(
            settings
                .get_project_config("0003")
                .unwrap()
                .get("ProjectHost"),
            Some(&"meta.dasch.swiss".to_string())
        );
        assert_eq!(
            settings.get_project_config("080e").unwrap().get("Host"),
            Some(&"data.dasch.swiss".to_string())
        );
        assert_eq!(
            settings.get_project_config("080E").unwrap().get("Host"),
            Some(&"data.dasch.swiss".to_string())
        );
    }
}
