/// Use case layer for ARK URL formatting operations.
/// This layer orchestrates domain functions and provides business logic coordination.
use crate::core::domain::ark_url_formatter;
use crate::core::errors::ark_url_formatter::{ArkUrlFormatterError, ArkUrlFormatterResult};
use crate::uuid_processing::add_check_digit_and_escape_internal;

/// Configuration interface for ARK URL formatting operations.
/// This allows the use case to work with different configuration sources.
pub trait ArkUrlFormatterConfig {
    /// Get the ARK NAAN (Name Assigning Authority Number)
    fn get_ark_naan(&self) -> ArkUrlFormatterResult<String>;

    /// Get the DSP ARK version
    fn get_dsp_ark_version(&self) -> ArkUrlFormatterResult<String>;

    /// Get the external host for ARK URLs
    fn get_external_host(&self) -> ArkUrlFormatterResult<String>;

    /// Get whether to use HTTPS proxy
    fn get_use_https_proxy(&self) -> ArkUrlFormatterResult<bool>;

    /// Get the resource IRI regex pattern
    fn get_resource_iri_pattern(&self) -> ArkUrlFormatterResult<String>;

    /// Match a resource IRI and return (project_id, resource_id)
    fn match_resource_iri(&self, resource_iri: &str) -> ArkUrlFormatterResult<(String, String)>;
}

/// Use case orchestrator for ARK URL formatting operations
pub struct ArkUrlFormatterService<C: ArkUrlFormatterConfig> {
    config: C,
}

impl<C: ArkUrlFormatterConfig> ArkUrlFormatterService<C> {
    /// Create a new ArkUrlFormatterService with the given configuration
    pub fn new(config: C) -> Self {
        Self { config }
    }

    /// Converts a DSP resource IRI to an ARK ID.
    ///
    /// Business rules:
    /// - Resource IRI must match the configured pattern
    /// - Project ID and resource ID must be valid
    /// - Check digit is calculated and appended to resource ID
    /// - Timestamp is optional and appended if provided
    pub fn resource_iri_to_ark_id(
        &self,
        resource_iri: &str,
        timestamp: Option<&str>,
    ) -> ArkUrlFormatterResult<String> {
        // Validate and parse resource IRI
        let (project_id, resource_id) = self.config.match_resource_iri(resource_iri)?;

        // Validate components
        ark_url_formatter::validate_project_id(&project_id)?;
        ark_url_formatter::validate_resource_id(&resource_id)?;

        // Validate timestamp if provided
        if let Some(ts) = timestamp {
            ark_url_formatter::validate_timestamp(ts)?;
        }

        // Add check digit and escape the resource ID
        let escaped_resource_id = add_check_digit_and_escape_internal(&resource_id)
            .map_err(|e| ArkUrlFormatterError::UuidProcessingError(e.to_string()))?;

        // Get configuration values
        let ark_naan = self.config.get_ark_naan()?;
        let dsp_ark_version = self.config.get_dsp_ark_version()?;

        // Format the ARK ID
        Ok(ark_url_formatter::format_ark_id(
            &ark_naan,
            &dsp_ark_version,
            &project_id,
            &escaped_resource_id,
            timestamp,
        ))
    }

    /// Converts a DSP resource IRI to an ARK URL.
    ///
    /// Business rules:
    /// - Resource IRI must match the configured pattern
    /// - Value ID is optional and gets check digit if provided
    /// - Timestamp is optional
    /// - Uses configured protocol and host
    pub fn resource_iri_to_ark_url(
        &self,
        resource_iri: &str,
        value_id: Option<&str>,
        timestamp: Option<&str>,
    ) -> ArkUrlFormatterResult<String> {
        // Validate and parse resource IRI
        let (project_id, resource_id) = self.config.match_resource_iri(resource_iri)?;

        // Validate components
        ark_url_formatter::validate_project_id(&project_id)?;
        ark_url_formatter::validate_resource_id(&resource_id)?;

        // Validate timestamp if provided
        if let Some(ts) = timestamp {
            ark_url_formatter::validate_timestamp(ts)?;
        }

        // Add check digit and escape the resource ID
        let escaped_resource_id = add_check_digit_and_escape_internal(&resource_id)
            .map_err(|e| ArkUrlFormatterError::UuidProcessingError(e.to_string()))?;

        // Process value ID if provided
        let escaped_value_id = if let Some(vid) = value_id {
            Some(
                add_check_digit_and_escape_internal(vid)
                    .map_err(|e| ArkUrlFormatterError::UuidProcessingError(e.to_string()))?,
            )
        } else {
            None
        };

        // Get configuration values
        let ark_naan = self.config.get_ark_naan()?;
        let dsp_ark_version = self.config.get_dsp_ark_version()?;
        let external_host = self.config.get_external_host()?;
        let use_https = self.config.get_use_https_proxy()?;

        // Format the ARK URL
        let params = ark_url_formatter::ArkUrlParams {
            use_https,
            external_host: &external_host,
            ark_naan: &ark_naan,
            dsp_ark_version: &dsp_ark_version,
            project_id: &project_id,
            escaped_resource_id_with_check_digit: &escaped_resource_id,
            escaped_value_id_with_check_digit: escaped_value_id.as_deref(),
            timestamp,
        };
        Ok(ark_url_formatter::format_ark_url(params))
    }

    /// Formats an ARK URL from pre-processed components.
    ///
    /// Business rules:
    /// - All components are assumed to be pre-validated
    /// - Resource ID should already have check digit and be escaped
    /// - Value ID should already have check digit and be escaped (if provided)
    /// - Uses configured protocol and host
    pub fn format_ark_url(
        &self,
        project_id: &str,
        resource_id_with_check_digit: &str,
        value_id_with_check_digit: Option<&str>,
        timestamp: Option<&str>,
    ) -> ArkUrlFormatterResult<String> {
        // Basic validation
        ark_url_formatter::validate_project_id(project_id)?;

        if resource_id_with_check_digit.is_empty() {
            return Err(ArkUrlFormatterError::InvalidResourceId(
                resource_id_with_check_digit.to_string(),
            ));
        }

        // Validate timestamp if provided
        if let Some(ts) = timestamp {
            ark_url_formatter::validate_timestamp(ts)?;
        }

        // Get configuration values
        let ark_naan = self.config.get_ark_naan()?;
        let dsp_ark_version = self.config.get_dsp_ark_version()?;
        let external_host = self.config.get_external_host()?;
        let use_https = self.config.get_use_https_proxy()?;

        // Format the ARK URL
        let params = ark_url_formatter::ArkUrlParams {
            use_https,
            external_host: &external_host,
            ark_naan: &ark_naan,
            dsp_ark_version: &dsp_ark_version,
            project_id,
            escaped_resource_id_with_check_digit: resource_id_with_check_digit,
            escaped_value_id_with_check_digit: value_id_with_check_digit,
            timestamp,
        };
        Ok(ark_url_formatter::format_ark_url(params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock configuration for testing
    struct MockConfig;

    impl ArkUrlFormatterConfig for MockConfig {
        fn get_ark_naan(&self) -> ArkUrlFormatterResult<String> {
            Ok("00000".to_string())
        }

        fn get_dsp_ark_version(&self) -> ArkUrlFormatterResult<String> {
            Ok("1".to_string())
        }

        fn get_external_host(&self) -> ArkUrlFormatterResult<String> {
            Ok("ark.example.org".to_string())
        }

        fn get_use_https_proxy(&self) -> ArkUrlFormatterResult<bool> {
            Ok(true)
        }

        fn get_resource_iri_pattern(&self) -> ArkUrlFormatterResult<String> {
            Ok(r"^http://rdfh\.ch/([0-9A-Fa-f]{4})/(.*)$".to_string())
        }

        fn match_resource_iri(
            &self,
            resource_iri: &str,
        ) -> ArkUrlFormatterResult<(String, String)> {
            let pattern = self.get_resource_iri_pattern()?;
            ark_url_formatter::parse_resource_iri(resource_iri, &pattern)
        }
    }

    #[test]
    fn test_service_resource_iri_to_ark_id() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let result = service.resource_iri_to_ark_id(resource_iri, None).unwrap();

        assert!(result.starts_with("ark:/00000/1/0001/"));
        assert!(result.contains("cmfk1DMHRBiR4=_6HXpEFA")); // hyphen should be escaped
    }

    #[test]
    fn test_service_resource_iri_to_ark_id_with_timestamp() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let timestamp = "20180604T085622513Z";
        let result = service
            .resource_iri_to_ark_id(resource_iri, Some(timestamp))
            .unwrap();

        assert!(result.ends_with(".20180604T085622513Z"));
    }

    #[test]
    fn test_service_resource_iri_to_ark_url() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let result = service
            .resource_iri_to_ark_url(resource_iri, None, None)
            .unwrap();

        assert!(result.starts_with("https://ark.example.org/ark:/00000/1/0001/"));
        assert!(result.contains("cmfk1DMHRBiR4=_6HXpEFA")); // hyphen should be escaped
    }

    #[test]
    fn test_service_resource_iri_to_ark_url_with_value_and_timestamp() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let value_id = "pLlW4ODASumZfZFbJdpw1g";
        let timestamp = "20180604T085622513Z";
        let result = service
            .resource_iri_to_ark_url(resource_iri, Some(value_id), Some(timestamp))
            .unwrap();

        assert!(result.contains("pLlW4ODASumZfZFbJdpw1g"));
        assert!(result.ends_with(".20180604T085622513Z"));
    }

    #[test]
    fn test_service_format_ark_url() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let result = service
            .format_ark_url("0001", "cmfk1DMHRBiR4=_6HXpEFAn", None, None)
            .unwrap();

        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
        );
    }

    #[test]
    fn test_service_format_ark_url_with_value_and_timestamp() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let result = service
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
    fn test_service_invalid_resource_iri() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let result = service.resource_iri_to_ark_id("invalid://example.com", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidResourceIri(_)
        ));
    }

    #[test]
    fn test_service_empty_timestamp() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let result = service.resource_iri_to_ark_id(resource_iri, Some(""));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidTimestamp(_)
        ));
    }

    #[test]
    fn test_service_empty_project_id() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let result = service.format_ark_url("", "test", None, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidProjectId(_)
        ));
    }

    #[test]
    fn test_service_empty_resource_id() {
        let config = MockConfig;
        let service = ArkUrlFormatterService::new(config);

        let result = service.format_ark_url("0001", "", None, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidResourceId(_)
        ));
    }
}
