//! PyO3 adapter for ARK URL formatting operations.
//! This adapter provides a Python-compatible interface that maintains exact API compatibility.
#![allow(clippy::useless_conversion)]
use crate::adapters::pyo3::settings::ArkUrlSettings;
use crate::core::errors::ark_url_formatter::ArkUrlFormatterError;
use crate::core::ports::ark_url_formatter::ArkUrlFormatterPort;
use crate::core::use_cases::ark_url_formatter::{ArkUrlFormatterConfig, ArkUrlFormatterService};
use pyo3::{exceptions::PyValueError, pyclass, pymethods, PyResult};

/// Convert domain errors to PyO3 errors
fn convert_error(error: ArkUrlFormatterError) -> pyo3::PyErr {
    // Map to the same exception type as the original Python implementation
    match error {
        ArkUrlFormatterError::InvalidResourceIri(msg) => {
            // In Python, this becomes ArkUrlException
            PyValueError::new_err(format!("Invalid resource IRI: {msg}"))
        }
        ArkUrlFormatterError::InvalidProjectId(msg) => {
            PyValueError::new_err(format!("Invalid project ID: {msg}"))
        }
        ArkUrlFormatterError::InvalidResourceId(msg) => {
            PyValueError::new_err(format!("Invalid resource ID: {msg}"))
        }
        ArkUrlFormatterError::InvalidTimestamp(msg) => {
            PyValueError::new_err(format!("Invalid timestamp: {msg}"))
        }
        ArkUrlFormatterError::InvalidRegexPattern(msg) => {
            PyValueError::new_err(format!("Invalid regex pattern: {msg}"))
        }
        ArkUrlFormatterError::MissingConfiguration(msg) => {
            PyValueError::new_err(format!("Missing configuration: {msg}"))
        }
        ArkUrlFormatterError::UuidProcessingError(msg) => {
            PyValueError::new_err(format!("UUID processing error: {msg}"))
        }
    }
}

/// Configuration adapter that bridges ArkUrlSettings to ArkUrlFormatterConfig
struct SettingsConfigAdapter {
    settings: ArkUrlSettings,
}

impl SettingsConfigAdapter {
    fn new(settings: ArkUrlSettings) -> Self {
        Self { settings }
    }
}

impl ArkUrlFormatterConfig for SettingsConfigAdapter {
    fn get_ark_naan(&self) -> Result<String, ArkUrlFormatterError> {
        self.settings
            .ark_config
            .get("ArkNaan")
            .ok_or_else(|| ArkUrlFormatterError::MissingConfiguration("ArkNaan".to_string()))
            .cloned()
    }

    fn get_dsp_ark_version(&self) -> Result<String, ArkUrlFormatterError> {
        Ok(self.settings.dsp_ark_version.to_string())
    }

    fn get_external_host(&self) -> Result<String, ArkUrlFormatterError> {
        self.settings
            .ark_config
            .get("ArkExternalHost")
            .ok_or_else(|| {
                ArkUrlFormatterError::MissingConfiguration("ArkExternalHost".to_string())
            })
            .cloned()
    }

    fn get_use_https_proxy(&self) -> Result<bool, ArkUrlFormatterError> {
        self.settings
            .ark_config
            .get_boolean("ArkHttpsProxy")
            .map_err(|e| ArkUrlFormatterError::MissingConfiguration(e.to_string()))
    }

    fn get_resource_iri_pattern(&self) -> Result<String, ArkUrlFormatterError> {
        // This is the pattern used in the original Python implementation
        Ok(r"^http://rdfh\.ch/([0-9A-Fa-f]{4})/(.*)$".to_string())
    }

    fn match_resource_iri(
        &self,
        resource_iri: &str,
    ) -> Result<(String, String), ArkUrlFormatterError> {
        self.settings
            .match_resource_iri(resource_iri)
            .ok_or_else(|| ArkUrlFormatterError::InvalidResourceIri(resource_iri.to_string()))
    }
}

/// PyO3 class for ARK URL formatting operations.
///
/// This class provides exact API compatibility with the Python implementation
/// while leveraging the hexagonal architecture underneath.
#[pyclass]
pub struct ArkUrlFormatter {
    service: ArkUrlFormatterService<SettingsConfigAdapter>,
}

#[pymethods]
impl ArkUrlFormatter {
    /// Create a new ArkUrlFormatter with the given settings.
    #[new]
    #[pyo3(text_signature = "(settings)")]
    pub fn new(settings: ArkUrlSettings) -> Self {
        let config = SettingsConfigAdapter::new(settings);
        let service = ArkUrlFormatterService::new(config);

        Self { service }
    }

    /// Converts a DSP resource IRI (not values) to an ARK ID.
    ///
    /// This method maintains exact compatibility with the Python implementation.
    #[pyo3(signature = (resource_iri, timestamp=None))]
    pub fn resource_iri_to_ark_id(
        &self,
        resource_iri: &str,
        timestamp: Option<&str>,
    ) -> PyResult<String> {
        self.service
            .resource_iri_to_ark_id(resource_iri, timestamp)
            .map_err(convert_error)
    }

    /// Converts a DSP resource IRI to an ARK URL.
    ///
    /// This method maintains exact compatibility with the Python implementation.
    #[pyo3(signature = (resource_iri, value_id=None, timestamp=None))]
    pub fn resource_iri_to_ark_url(
        &self,
        resource_iri: &str,
        value_id: Option<&str>,
        timestamp: Option<&str>,
    ) -> PyResult<String> {
        self.service
            .resource_iri_to_ark_url(resource_iri, value_id, timestamp)
            .map_err(convert_error)
    }

    /// Formats and returns a DSP ARK URL from the given parameters and configuration.
    ///
    /// This method maintains exact compatibility with the Python implementation.
    #[pyo3(signature = (project_id, resource_id_with_check_digit, value_id_with_check_digit=None, timestamp=None))]
    pub fn format_ark_url(
        &self,
        project_id: &str,
        resource_id_with_check_digit: &str,
        value_id_with_check_digit: Option<&str>,
        timestamp: Option<&str>,
    ) -> PyResult<String> {
        self.service
            .format_ark_url(
                project_id,
                resource_id_with_check_digit,
                value_id_with_check_digit,
                timestamp,
            )
            .map_err(convert_error)
    }
}

impl ArkUrlFormatterPort for ArkUrlFormatter {
    fn resource_iri_to_ark_id(
        &self,
        resource_iri: &str,
        timestamp: Option<&str>,
    ) -> Result<String, ArkUrlFormatterError> {
        self.service.resource_iri_to_ark_id(resource_iri, timestamp)
    }

    fn resource_iri_to_ark_url(
        &self,
        resource_iri: &str,
        value_id: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<String, ArkUrlFormatterError> {
        self.service
            .resource_iri_to_ark_url(resource_iri, value_id, timestamp)
    }

    fn format_ark_url(
        &self,
        project_id: &str,
        resource_id_with_check_digit: &str,
        value_id_with_check_digit: Option<&str>,
        timestamp: Option<&str>,
    ) -> Result<String, ArkUrlFormatterError> {
        self.service.format_ark_url(
            project_id,
            resource_id_with_check_digit,
            value_id_with_check_digit,
            timestamp,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::pyo3::settings::ArkUrlSettings;

    fn get_test_settings() -> ArkUrlSettings {
        ArkUrlSettings::new().unwrap()
    }

    #[test]
    fn test_pyo3_adapter_resource_iri_to_ark_id() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let result = formatter
            .resource_iri_to_ark_id(resource_iri, None)
            .unwrap();

        assert!(result.starts_with("ark:/00000/1/0001/"));
        assert!(result.contains("cmfk1DMHRBiR4=_6HXpEFA")); // hyphen should be escaped
        assert!(result.ends_with("n")); // should end with check digit
    }

    #[test]
    fn test_pyo3_adapter_resource_iri_to_ark_id_with_timestamp() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let timestamp = "20180604T085622513Z";
        let result = formatter
            .resource_iri_to_ark_id(resource_iri, Some(timestamp))
            .unwrap();

        assert!(result.starts_with("ark:/00000/1/0001/"));
        assert!(result.contains("cmfk1DMHRBiR4=_6HXpEFA"));
        assert!(result.ends_with(".20180604T085622513Z"));
    }

    #[test]
    fn test_pyo3_adapter_resource_iri_to_ark_url() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let result = formatter
            .resource_iri_to_ark_url(resource_iri, None, None)
            .unwrap();

        assert!(result.starts_with("https://ark.example.org/ark:/00000/1/0001/"));
        assert!(result.contains("cmfk1DMHRBiR4=_6HXpEFA"));
        assert!(result.ends_with("n")); // should end with check digit
    }

    #[test]
    fn test_pyo3_adapter_resource_iri_to_ark_url_with_value_and_timestamp() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let value_id = "pLlW4ODASumZfZFbJdpw1g";
        let timestamp = "20180604T085622513Z";
        let result = formatter
            .resource_iri_to_ark_url(resource_iri, Some(value_id), Some(timestamp))
            .unwrap();

        assert!(result.starts_with("https://ark.example.org/ark:/00000/1/0001/"));
        assert!(result.contains("cmfk1DMHRBiR4=_6HXpEFA"));
        assert!(result.contains("pLlW4ODASumZfZFbJdpw1g"));
        assert!(result.ends_with(".20180604T085622513Z"));
    }

    #[test]
    fn test_pyo3_adapter_format_ark_url() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let result = formatter
            .format_ark_url("0001", "cmfk1DMHRBiR4=_6HXpEFAn", None, None)
            .unwrap();

        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
        );
    }

    #[test]
    fn test_pyo3_adapter_format_ark_url_with_value_and_timestamp() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let result = formatter
            .format_ark_url(
                "0001",
                "cmfk1DMHRBiR4=_6HXpEFAn",
                Some("pLlW4ODASumZfZFbJdpw1gu"),
                Some("20180604T085622513Z"),
            )
            .unwrap();

        assert_eq!(result, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622513Z");
    }

    #[test]
    fn test_pyo3_adapter_invalid_resource_iri() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let result = formatter.resource_iri_to_ark_id("invalid://example.com", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_pyo3_adapter_empty_timestamp() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let result = formatter.resource_iri_to_ark_id(resource_iri, Some(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_pyo3_adapter_empty_project_id() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        let result = formatter.format_ark_url("", "test", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_pyo3_adapter_as_port() {
        let settings = get_test_settings();
        let formatter = ArkUrlFormatter::new(settings);

        // Test using the port interface
        let result = formatter
            .resource_iri_to_ark_id("http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA", None)
            .unwrap();
        assert!(result.starts_with("ark:/00000/1/0001/"));
    }
}
