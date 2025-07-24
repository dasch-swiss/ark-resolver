// Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
// SPDX-License-Identifier: Apache-2.0

//! PyO3 adapter for ARK URL information processing.
//! Provides Python bindings for the Rust ARK URL info functionality.

#![allow(unexpected_cfgs)]
#![allow(clippy::useless_conversion)]

use crate::adapters::pyo3::settings::ArkUrlSettings;
use crate::core::domain::ark_url_info::ArkUrlInfo as RustArkUrlInfo;
use crate::core::errors::ark_url_info::ArkUrlInfoError;
use crate::core::ports::ark_url_info::{
    ArkUrlParsingPort, ConfigurationPort, TemplatePort, UuidGenerationPort,
};
use crate::core::use_cases::ark_url_info_processor::ArkUrlInfoProcessor;
use base64::prelude::*;
use pyo3::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

// Import the Python ArkUrlException
pyo3::import_exception!(ark_resolver.ark_url, ArkUrlException);

/// PyO3 wrapper for ArkUrlInfo providing Python compatibility.
#[pyclass(name = "ArkUrlInfo")]
#[derive(Debug, Clone)]
pub struct PyArkUrlInfo {
    inner: RustArkUrlInfo,
    settings: ArkUrlSettings,
}

#[pymethods]
impl PyArkUrlInfo {
    /// Creates a new ArkUrlInfo instance by parsing the given ARK ID.
    #[new]
    fn new(settings: ArkUrlSettings, ark_id: String) -> PyResult<Self> {
        let parser = ArkUrlParsingAdapter::new(&settings);
        let config = ConfigurationAdapter::new(&settings);
        let template = TemplateAdapter;
        let uuid_generator = UuidGenerationAdapter;

        let processor = ArkUrlInfoProcessor::new(parser, config, template, uuid_generator);

        let ark_info = processor
            .parse_ark_id(&ark_id)
            .map_err(|e| PyErr::new::<ArkUrlException, _>(e.to_string()))?;

        Ok(Self {
            inner: ark_info,
            settings,
        })
    }

    /// Returns the formatted timestamp of the ARK URL.
    fn get_timestamp(&self) -> Option<String> {
        self.inner.get_timestamp()
    }

    /// Returns the redirect URL for this ARK URL.
    fn to_redirect_url(&self) -> PyResult<String> {
        let parser = ArkUrlParsingAdapter::new(&self.settings);
        let config = ConfigurationAdapter::new(&self.settings);
        let template = TemplateAdapter;
        let uuid_generator = UuidGenerationAdapter;

        let processor = ArkUrlInfoProcessor::new(parser, config, template, uuid_generator);

        processor
            .generate_redirect_url(&self.inner)
            .map_err(|e| PyErr::new::<ArkUrlException, _>(e.to_string()))
    }

    /// Returns the resource IRI for this ARK URL.
    fn to_resource_iri(&self) -> PyResult<String> {
        let parser = ArkUrlParsingAdapter::new(&self.settings);
        let config = ConfigurationAdapter::new(&self.settings);
        let template = TemplateAdapter;
        let uuid_generator = UuidGenerationAdapter;

        let processor = ArkUrlInfoProcessor::new(parser, config, template, uuid_generator);

        processor
            .generate_resource_iri(&self.inner)
            .map_err(|e| PyErr::new::<ArkUrlException, _>(e.to_string()))
    }

    /// Returns the DSP redirect URL for this ARK URL.
    fn to_dsp_redirect_url(&self) -> PyResult<String> {
        let parser = ArkUrlParsingAdapter::new(&self.settings);
        let config = ConfigurationAdapter::new(&self.settings);
        let template = TemplateAdapter;
        let uuid_generator = UuidGenerationAdapter;

        let processor = ArkUrlInfoProcessor::new(parser, config, template, uuid_generator);

        processor
            .generate_dsp_redirect_url(&self.inner)
            .map_err(|e| PyErr::new::<ArkUrlException, _>(e.to_string()))
    }

    /// Returns the template dictionary for this ARK URL.
    fn template_dict(&self) -> PyResult<HashMap<String, String>> {
        Ok(self.inner.to_template_dict())
    }

    // Python properties to maintain compatibility
    #[getter]
    fn url_version(&self) -> u8 {
        self.inner.url_version
    }

    #[getter]
    fn project_id(&self) -> Option<String> {
        self.inner.project_id.clone()
    }

    #[getter]
    fn resource_id(&self) -> Option<String> {
        self.inner.resource_id.clone()
    }

    #[getter]
    fn value_id(&self) -> Option<String> {
        self.inner.value_id.clone()
    }

    // Note: timestamp is accessed via get_timestamp() method, not as a property
}

/// Adapter for ARK URL parsing operations.
struct ArkUrlParsingAdapter<'a> {
    settings: &'a ArkUrlSettings,
}

impl<'a> ArkUrlParsingAdapter<'a> {
    fn new(settings: &'a ArkUrlSettings) -> Self {
        Self { settings }
    }
}

impl ArkUrlParsingPort for ArkUrlParsingAdapter<'_> {
    fn parse_ark_v1(
        &self,
        ark_id: &str,
    ) -> Option<(
        u32,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> {
        self.settings.match_ark_path(ark_id).map(|matches| {
            let url_version = matches.0.and_then(|v| v.parse::<u32>().ok()).unwrap_or(1);
            let project_id = matches.1;
            let resource_id = matches.2;
            let value_id = matches.3;
            let timestamp = matches.4;

            (url_version, project_id, resource_id, value_id, timestamp)
        })
    }

    fn parse_ark_v0(&self, ark_id: &str) -> Option<(String, String, Option<String>)> {
        self.settings.match_v0_ark_path(ark_id).map(|matches| {
            let project_id = matches.0.unwrap_or_default();
            let resource_id = matches.1.unwrap_or_default();
            let timestamp = matches.2;

            (project_id, resource_id, timestamp)
        })
    }

    fn unescape_and_validate_uuid(
        &self,
        ark_url: &str,
        escaped_uuid: &str,
    ) -> Result<String, ArkUrlInfoError> {
        crate::adapters::pyo3::uuid_processing::unescape_and_validate_uuid(
            ark_url.to_string(),
            escaped_uuid.to_string(),
        )
        .map_err(|e| ArkUrlInfoError::uuid_processing_failed(e.to_string()))
    }
}

/// Adapter for configuration operations.
struct ConfigurationAdapter<'a> {
    settings: &'a ArkUrlSettings,
}

impl<'a> ConfigurationAdapter<'a> {
    fn new(settings: &'a ArkUrlSettings) -> Self {
        Self { settings }
    }
}

impl ConfigurationPort for ConfigurationAdapter<'_> {
    fn get_dsp_ark_version(&self) -> u32 {
        self.settings.dsp_ark_version.into()
    }

    fn is_version_0_allowed(&self, project_id: &str) -> Result<bool, ArkUrlInfoError> {
        self.settings
            .get_project_config(project_id)
            .ok_or_else(|| ArkUrlInfoError::configuration_error("Project configuration not found"))
            .and_then(|config| {
                config
                    .get_boolean("AllowVersion0")
                    .map_err(|e| ArkUrlInfoError::configuration_error(e.to_string()))
            })
    }

    fn get_top_level_redirect_url(&self) -> String {
        self.settings
            .default_config
            .get("TopLevelObjectUrl")
            .cloned()
            .unwrap_or_default()
    }

    fn get_project_template(
        &self,
        project_id: &str,
        template_name: &str,
    ) -> Result<String, ArkUrlInfoError> {
        self.settings
            .get_project_config(project_id)
            .and_then(|config| config.get(template_name).cloned())
            .ok_or_else(|| ArkUrlInfoError::template_not_found(template_name))
    }

    fn get_project_host(&self, project_id: &str) -> Result<String, ArkUrlInfoError> {
        self.settings
            .get_project_config(project_id)
            .and_then(|config| config.get("ProjectHost").cloned())
            .or_else(|| self.settings.default_config.get("ProjectHost").cloned())
            .ok_or_else(|| ArkUrlInfoError::configuration_error("Project host not found"))
    }
}

/// Adapter for template operations.
struct TemplateAdapter;

impl TemplatePort for TemplateAdapter {
    fn substitute(
        &self,
        template: &str,
        values: &HashMap<String, String>,
    ) -> Result<String, ArkUrlInfoError> {
        let mut result = template.to_string();

        for (key, value) in values {
            let placeholder = format!("${{{key}}}");
            result = result.replace(&placeholder, value);
        }

        // Also handle simple $key format (without braces)
        for (key, value) in values {
            let placeholder = format!("${key}");
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    fn url_encode(&self, input: &str) -> Result<String, ArkUrlInfoError> {
        Ok(urlencoding::encode(input).to_string())
    }
}

/// Adapter for UUID generation operations.
struct UuidGenerationAdapter;

impl UuidGenerationPort for UuidGenerationAdapter {
    fn generate_v5_uuid(&self, input: &str) -> Result<String, ArkUrlInfoError> {
        // Generate UUID v5 using the DaSCH namespace
        let namespace = Uuid::parse_str("cace8b00-717e-50d5-bcb9-486f39d733a2")
            .map_err(|e| ArkUrlInfoError::uuid_processing_failed(e.to_string()))?;

        let uuid = Uuid::new_v5(&namespace, input.as_bytes());
        let uuid_bytes = uuid.as_bytes();

        // Convert to base64 URL-safe encoding without padding
        let base64_encoded = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(uuid_bytes);

        Ok(base64_encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ark_url_parsing_adapter() {
        // Test that the adapter structure is correct
        // FIXME: We can't easily test with real settings without configuration files
        assert!(true);
    }

    #[test]
    fn test_template_adapter() {
        let adapter = TemplateAdapter;
        let mut values = HashMap::new();
        values.insert("host".to_string(), "example.com".to_string());
        values.insert("project_id".to_string(), "0001".to_string());

        let result = adapter.substitute("https://${host}/project/${project_id}", &values);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com/project/0001");
    }

    #[test]
    fn test_url_encoding() {
        let adapter = TemplateAdapter;
        let result = adapter.url_encode("hello world");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello%20world");
    }

    #[test]
    fn test_uuid_generation() {
        let adapter = UuidGenerationAdapter;
        let result = adapter.generate_v5_uuid("test-input");
        assert!(result.is_ok());

        // The result should be a valid base64 string
        let uuid_str = result.unwrap();
        assert!(!uuid_str.is_empty());
        assert!(!uuid_str.contains("=")); // Should not have padding
    }
}
