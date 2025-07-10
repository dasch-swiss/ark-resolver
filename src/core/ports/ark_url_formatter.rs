/// Port layer for ARK URL formatting operations.
/// This layer defines abstract interfaces for ARK URL formatting capabilities.
use crate::core::errors::ark_url_formatter::ArkUrlFormatterResult;

/// Port trait defining the interface for ARK URL formatting operations.
///
/// This trait abstracts the ARK URL formatting functionality, allowing different
/// implementations (e.g., PyO3 adapter, HTTP adapter, CLI adapter) to provide
/// the same formatting capabilities.
pub trait ArkUrlFormatterPort {
    /// Converts a DSP resource IRI to an ARK ID.
    ///
    /// An ARK ID is the core identifier part without the HTTP URL wrapper.
    ///
    /// # Arguments
    /// * `resource_iri` - The DSP resource IRI to convert
    /// * `timestamp` - Optional timestamp to append to the ARK ID
    ///
    /// # Returns
    /// * `Ok(String)` - The formatted ARK ID
    /// * `Err(ArkUrlFormatterError)` - If the resource IRI is invalid or formatting fails
    fn resource_iri_to_ark_id(
        &self,
        resource_iri: &str,
        timestamp: Option<&str>,
    ) -> ArkUrlFormatterResult<String>;

    /// Converts a DSP resource IRI to an ARK URL.
    ///
    /// An ARK URL is the complete HTTP(S) URL that can be used for redirection.
    ///
    /// # Arguments
    /// * `resource_iri` - The DSP resource IRI to convert
    /// * `value_id` - Optional value UUID to include in the ARK URL
    /// * `timestamp` - Optional timestamp to append to the ARK URL
    ///
    /// # Returns
    /// * `Ok(String)` - The formatted ARK URL
    /// * `Err(ArkUrlFormatterError)` - If the resource IRI is invalid or formatting fails
    fn resource_iri_to_ark_url(
        &self,
        resource_iri: &str,
        value_id: Option<&str>,
        timestamp: Option<&str>,
    ) -> ArkUrlFormatterResult<String>;

    /// Formats an ARK URL from pre-processed components.
    ///
    /// This method assumes all components are already validated and processed
    /// (e.g., resource and value IDs already have check digits and are escaped).
    ///
    /// # Arguments
    /// * `project_id` - The project identifier
    /// * `resource_id_with_check_digit` - The resource ID with check digit and escaped
    /// * `value_id_with_check_digit` - Optional value ID with check digit and escaped
    /// * `timestamp` - Optional timestamp to append to the ARK URL
    ///
    /// # Returns
    /// * `Ok(String)` - The formatted ARK URL
    /// * `Err(ArkUrlFormatterError)` - If components are invalid or formatting fails
    fn format_ark_url(
        &self,
        project_id: &str,
        resource_id_with_check_digit: &str,
        value_id_with_check_digit: Option<&str>,
        timestamp: Option<&str>,
    ) -> ArkUrlFormatterResult<String>;
}

/// Extension trait for additional ARK URL formatting operations.
///
/// This trait can be implemented by types that already implement `ArkUrlFormatterPort`
/// to provide additional formatting capabilities.
pub trait ArkUrlFormatterPortExt: ArkUrlFormatterPort {
    /// Formats an ARK URL for a resource without any value or timestamp.
    ///
    /// This is a convenience method that calls `resource_iri_to_ark_url` with None values.
    fn resource_iri_to_simple_ark_url(&self, resource_iri: &str) -> ArkUrlFormatterResult<String> {
        self.resource_iri_to_ark_url(resource_iri, None, None)
    }

    /// Formats an ARK URL for a resource with a timestamp but no value.
    ///
    /// This is a convenience method for timestamped resources.
    fn resource_iri_to_timestamped_ark_url(
        &self,
        resource_iri: &str,
        timestamp: &str,
    ) -> ArkUrlFormatterResult<String> {
        self.resource_iri_to_ark_url(resource_iri, None, Some(timestamp))
    }

    /// Formats an ARK URL for a resource with a value but no timestamp.
    ///
    /// This is a convenience method for resources with values.
    fn resource_iri_to_value_ark_url(
        &self,
        resource_iri: &str,
        value_id: &str,
    ) -> ArkUrlFormatterResult<String> {
        self.resource_iri_to_ark_url(resource_iri, Some(value_id), None)
    }
}

/// Blanket implementation of ArkUrlFormatterPortExt for all types that implement ArkUrlFormatterPort.
impl<T: ArkUrlFormatterPort> ArkUrlFormatterPortExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::errors::ark_url_formatter::ArkUrlFormatterError;

    // Mock implementation for testing
    struct MockArkUrlFormatter;

    impl ArkUrlFormatterPort for MockArkUrlFormatter {
        fn resource_iri_to_ark_id(
            &self,
            resource_iri: &str,
            timestamp: Option<&str>,
        ) -> ArkUrlFormatterResult<String> {
            if resource_iri.is_empty() {
                return Err(ArkUrlFormatterError::InvalidResourceIri(
                    resource_iri.to_string(),
                ));
            }

            let mut result = format!("ark:/00000/1/0001/{}", resource_iri);
            if let Some(ts) = timestamp {
                result.push('.');
                result.push_str(ts);
            }
            Ok(result)
        }

        fn resource_iri_to_ark_url(
            &self,
            resource_iri: &str,
            value_id: Option<&str>,
            timestamp: Option<&str>,
        ) -> ArkUrlFormatterResult<String> {
            if resource_iri.is_empty() {
                return Err(ArkUrlFormatterError::InvalidResourceIri(
                    resource_iri.to_string(),
                ));
            }

            let mut result = format!("https://ark.example.org/ark:/00000/1/0001/{}", resource_iri);

            if let Some(vid) = value_id {
                result.push('/');
                result.push_str(vid);
            }

            if let Some(ts) = timestamp {
                result.push('.');
                result.push_str(ts);
            }

            Ok(result)
        }

        fn format_ark_url(
            &self,
            project_id: &str,
            resource_id_with_check_digit: &str,
            value_id_with_check_digit: Option<&str>,
            timestamp: Option<&str>,
        ) -> ArkUrlFormatterResult<String> {
            if project_id.is_empty() {
                return Err(ArkUrlFormatterError::InvalidProjectId(
                    project_id.to_string(),
                ));
            }

            let mut result = format!(
                "https://ark.example.org/ark:/00000/1/{}/{}",
                project_id, resource_id_with_check_digit
            );

            if let Some(vid) = value_id_with_check_digit {
                result.push('/');
                result.push_str(vid);
            }

            if let Some(ts) = timestamp {
                result.push('.');
                result.push_str(ts);
            }

            Ok(result)
        }
    }

    #[test]
    fn test_port_resource_iri_to_ark_id() {
        let formatter = MockArkUrlFormatter;
        let result = formatter.resource_iri_to_ark_id("test", None).unwrap();
        assert_eq!(result, "ark:/00000/1/0001/test");
    }

    #[test]
    fn test_port_resource_iri_to_ark_id_with_timestamp() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .resource_iri_to_ark_id("test", Some("20180604"))
            .unwrap();
        assert_eq!(result, "ark:/00000/1/0001/test.20180604");
    }

    #[test]
    fn test_port_resource_iri_to_ark_url() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .resource_iri_to_ark_url("test", None, None)
            .unwrap();
        assert_eq!(result, "https://ark.example.org/ark:/00000/1/0001/test");
    }

    #[test]
    fn test_port_resource_iri_to_ark_url_with_value_and_timestamp() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .resource_iri_to_ark_url("test", Some("value"), Some("20180604"))
            .unwrap();
        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/test/value.20180604"
        );
    }

    #[test]
    fn test_port_format_ark_url() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .format_ark_url("0001", "test", None, None)
            .unwrap();
        assert_eq!(result, "https://ark.example.org/ark:/00000/1/0001/test");
    }

    #[test]
    fn test_port_format_ark_url_with_value_and_timestamp() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .format_ark_url("0001", "test", Some("value"), Some("20180604"))
            .unwrap();
        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/test/value.20180604"
        );
    }

    #[test]
    fn test_port_extension_simple_ark_url() {
        let formatter = MockArkUrlFormatter;
        let result = formatter.resource_iri_to_simple_ark_url("test").unwrap();
        assert_eq!(result, "https://ark.example.org/ark:/00000/1/0001/test");
    }

    #[test]
    fn test_port_extension_timestamped_ark_url() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .resource_iri_to_timestamped_ark_url("test", "20180604")
            .unwrap();
        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/test.20180604"
        );
    }

    #[test]
    fn test_port_extension_value_ark_url() {
        let formatter = MockArkUrlFormatter;
        let result = formatter
            .resource_iri_to_value_ark_url("test", "value")
            .unwrap();
        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/test/value"
        );
    }

    #[test]
    fn test_port_error_handling() {
        let formatter = MockArkUrlFormatter;

        // Test empty resource IRI
        let result = formatter.resource_iri_to_ark_id("", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidResourceIri(_)
        ));

        // Test empty project ID
        let result = formatter.format_ark_url("", "test", None, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidProjectId(_)
        ));
    }
}
