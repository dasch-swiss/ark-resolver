// Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
// SPDX-License-Identifier: Apache-2.0

//! Use case layer for ARK URL information processing.
//! Orchestrates business logic using domain objects and external services.

use crate::core::domain::ark_url_info::ArkUrlInfo;
use crate::core::errors::ark_url_info::{ArkUrlInfoError, ArkUrlInfoResult};
use crate::core::ports::ark_url_info::{
    ArkUrlInfoPort, ArkUrlParsingPort, ConfigurationPort, TemplatePort, UuidGenerationPort,
};
use std::collections::HashMap;

/// Use case for processing ARK URL information.
/// Orchestrates the parsing, validation, and processing of ARK URLs.
pub struct ArkUrlInfoProcessor<P, C, T, U>
where
    P: ArkUrlParsingPort,
    C: ConfigurationPort,
    T: TemplatePort,
    U: UuidGenerationPort,
{
    parser: P,
    config: C,
    template: T,
    uuid_generator: U,
}

impl<P, C, T, U> ArkUrlInfoProcessor<P, C, T, U>
where
    P: ArkUrlParsingPort,
    C: ConfigurationPort,
    T: TemplatePort,
    U: UuidGenerationPort,
{
    /// Creates a new ArkUrlInfoProcessor with the given dependencies.
    pub fn new(parser: P, config: C, template: T, uuid_generator: U) -> Self {
        Self {
            parser,
            config,
            template,
            uuid_generator,
        }
    }

    /// Parses an ARK ID and returns the corresponding ArkUrlInfo.
    pub fn parse_ark_id(&self, ark_id: &str) -> ArkUrlInfoResult<ArkUrlInfo> {
        // First try to parse as version 1 ARK ID
        if let Some(components) = self.parser.parse_ark_v1(ark_id) {
            let url_version = components.0;
            let project_id = components.1;
            let escaped_resource_id = components.2;
            let escaped_value_id = components.3;
            let timestamp = components.4;

            // Validate that the version matches the expected DSP version
            if url_version != self.config.get_dsp_ark_version() {
                return Err(ArkUrlInfoError::version_mismatch(ark_id));
            }

            // Process resource ID if present
            let resource_id = if let Some(escaped_res_id) = escaped_resource_id {
                Some(
                    self.parser
                        .unescape_and_validate_uuid(ark_id, &escaped_res_id)?,
                )
            } else {
                None
            };

            // Process value ID if present
            let value_id = if let Some(escaped_val_id) = escaped_value_id {
                Some(
                    self.parser
                        .unescape_and_validate_uuid(ark_id, &escaped_val_id)?,
                )
            } else {
                None
            };

            return Ok(ArkUrlInfo::new(
                url_version as u8,
                project_id,
                resource_id,
                value_id,
                timestamp,
            ));
        }

        // Try to parse as version 0 ARK ID
        if let Some(components) = self.parser.parse_ark_v0(ark_id) {
            let project_id = components.0.to_uppercase();
            let resource_id = components.1;
            let submitted_timestamp = components.2;

            // Check if version 0 is allowed for this project
            if !self.config.is_version_0_allowed(&project_id)? {
                return Err(ArkUrlInfoError::version_0_not_allowed(ark_id));
            }

            // Process timestamp for version 0
            let timestamp = submitted_timestamp.filter(|ts| ts.len() >= 8);

            return Ok(ArkUrlInfo::new(
                0,
                Some(project_id),
                Some(resource_id),
                None,
                timestamp,
            ));
        }

        // If neither version matches, return error
        Err(ArkUrlInfoError::invalid_ark_id(ark_id))
    }

    /// Generates a redirect URL for the given ARK URL info.
    pub fn generate_redirect_url(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        // If no project ID, return top-level redirect
        if ark_info.project_id.is_none() {
            return Ok(self.config.get_top_level_redirect_url());
        }

        self.generate_dsp_redirect_url(ark_info)
    }

    /// Generates a DSP-specific redirect URL.
    pub fn generate_dsp_redirect_url(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        let project_id = ark_info
            .project_id
            .as_ref()
            .ok_or(ArkUrlInfoError::ProjectIdRequired)?;

        // Get the appropriate redirect template based on the ARK URL type
        let template_name = self.determine_redirect_template(ark_info)?;
        let template = self
            .config
            .get_project_template(project_id, &template_name)?;

        // Build template dictionary
        let mut template_dict = ark_info.to_template_dict();

        // Use regular Host for $host template variable
        template_dict.insert(
            "host".to_string(),
            self.config.get_project_template(project_id, "Host")?,
        );

        // Handle version 0 resource ID conversion
        if ark_info.is_version_0() {
            let resource_iri = self.generate_resource_iri(ark_info)?;
            let converted_resource_id = resource_iri.split('/').next_back().unwrap_or("");
            template_dict.insert("resource_id".to_string(), converted_resource_id.to_string());
        }

        // Add project host for project-level redirects (uses ProjectHost)
        if ark_info.is_project_level() {
            template_dict.insert(
                "project_host".to_string(),
                self.config.get_project_host(project_id)?,
            );
        }

        // Generate resource and project IRIs for the template
        let resource_iri = self.generate_resource_iri_for_template(ark_info, &template_dict)?;
        let project_iri = self.generate_project_iri_for_template(ark_info, &template_dict)?;

        template_dict.insert(
            "resource_iri".to_string(),
            self.template.url_encode(&resource_iri)?,
        );
        template_dict.insert(
            "project_iri".to_string(),
            self.template.url_encode(&project_iri)?,
        );

        // Apply template substitution
        self.template.substitute(&template, &template_dict)
    }

    /// Generates a resource IRI for the given ARK URL info.
    pub fn generate_resource_iri(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        let project_id = ark_info
            .project_id
            .as_ref()
            .ok_or(ArkUrlInfoError::ProjectIdRequired)?;
        let template = self
            .config
            .get_project_template(project_id, "DSPResourceIri")?;

        let mut template_dict = ark_info.to_template_dict();
        template_dict.insert(
            "host".to_string(),
            self.config.get_project_template(project_id, "Host")?,
        );

        // Handle version 0 UUID generation
        if ark_info.is_version_0() {
            let resource_id =
                ark_info
                    .resource_id
                    .as_ref()
                    .ok_or(ArkUrlInfoError::configuration_error(
                        "Resource ID required for version 0 ARK URLs",
                    ))?;
            let uuid_v5 = self.uuid_generator.generate_v5_uuid(resource_id)?;
            template_dict.insert("resource_id".to_string(), uuid_v5);
        }

        self.template.substitute(&template, &template_dict)
    }

    /// Determines the appropriate redirect template based on ARK URL characteristics.
    fn determine_redirect_template(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        match (
            ark_info.is_project_level(),
            ark_info.is_resource_level(),
            ark_info.is_value_level(),
            ark_info.has_timestamp(),
        ) {
            (true, false, false, _) => Ok("DSPProjectRedirectUrl".to_string()),
            (false, true, false, false) => Ok("DSPResourceRedirectUrl".to_string()),
            (false, true, false, true) => Ok("DSPResourceVersionRedirectUrl".to_string()),
            (false, false, true, false) => Ok("DSPValueRedirectUrl".to_string()),
            (false, false, true, true) => Ok("DSPValueVersionRedirectUrl".to_string()),
            _ => Err(ArkUrlInfoError::RedirectTemplateUndetermined),
        }
    }

    /// Generates a resource IRI for template substitution.
    fn generate_resource_iri_for_template(
        &self,
        ark_info: &ArkUrlInfo,
        template_dict: &HashMap<String, String>,
    ) -> ArkUrlInfoResult<String> {
        let project_id = ark_info
            .project_id
            .as_ref()
            .ok_or(ArkUrlInfoError::ProjectIdRequired)?;
        let template = self
            .config
            .get_project_template(project_id, "DSPResourceIri")?;
        self.template.substitute(&template, template_dict)
    }

    /// Generates a project IRI for template substitution.
    fn generate_project_iri_for_template(
        &self,
        ark_info: &ArkUrlInfo,
        template_dict: &HashMap<String, String>,
    ) -> ArkUrlInfoResult<String> {
        let project_id = ark_info
            .project_id
            .as_ref()
            .ok_or(ArkUrlInfoError::ProjectIdRequired)?;
        let template = self
            .config
            .get_project_template(project_id, "DSPProjectIri")?;
        self.template.substitute(&template, template_dict)
    }
}

impl<P, C, T, U> ArkUrlInfoPort for ArkUrlInfoProcessor<P, C, T, U>
where
    P: ArkUrlParsingPort,
    C: ConfigurationPort,
    T: TemplatePort,
    U: UuidGenerationPort,
{
    fn parse_ark_id(&self, ark_id: &str) -> ArkUrlInfoResult<ArkUrlInfo> {
        self.parse_ark_id(ark_id)
    }

    fn generate_redirect_url(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        self.generate_redirect_url(ark_info)
    }

    fn generate_resource_iri(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        self.generate_resource_iri(ark_info)
    }

    fn generate_dsp_redirect_url(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String> {
        self.generate_dsp_redirect_url(ark_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::errors::ark_url_info::ArkUrlInfoError;

    // Mock implementations for testing
    struct MockParser;
    struct MockConfig;
    struct MockTemplate;
    struct MockUuidGenerator;

    impl ArkUrlParsingPort for MockParser {
        fn parse_ark_v1(
            &self,
            _ark_id: &str,
        ) -> Option<(
            u32,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> {
            Some((
                1,
                Some("0001".to_string()),
                Some("resource123".to_string()),
                None,
                None,
            ))
        }

        fn parse_ark_v0(&self, _ark_id: &str) -> Option<(String, String, Option<String>)> {
            Some(("0001".to_string(), "resource123".to_string(), None))
        }

        fn unescape_and_validate_uuid(
            &self,
            _ark_url: &str,
            escaped_uuid: &str,
        ) -> ArkUrlInfoResult<String> {
            Ok(escaped_uuid.to_string())
        }
    }

    impl ConfigurationPort for MockConfig {
        fn get_dsp_ark_version(&self) -> u32 {
            1
        }

        fn is_version_0_allowed(&self, _project_id: &str) -> ArkUrlInfoResult<bool> {
            Ok(true)
        }

        fn get_top_level_redirect_url(&self) -> String {
            "https://example.com/top".to_string()
        }

        fn get_project_template(
            &self,
            _project_id: &str,
            _template_name: &str,
        ) -> ArkUrlInfoResult<String> {
            Ok("https://example.com/template".to_string())
        }

        fn get_project_host(&self, _project_id: &str) -> ArkUrlInfoResult<String> {
            Ok("example.com".to_string())
        }
    }

    impl TemplatePort for MockTemplate {
        fn substitute(
            &self,
            _template: &str,
            _values: &HashMap<String, String>,
        ) -> ArkUrlInfoResult<String> {
            Ok("https://example.com/substituted".to_string())
        }

        fn url_encode(&self, input: &str) -> ArkUrlInfoResult<String> {
            Ok(input.to_string())
        }
    }

    impl UuidGenerationPort for MockUuidGenerator {
        fn generate_v5_uuid(&self, _input: &str) -> ArkUrlInfoResult<String> {
            Ok("generated-uuid".to_string())
        }
    }

    #[test]
    fn test_parse_ark_v1_success() {
        let processor =
            ArkUrlInfoProcessor::new(MockParser, MockConfig, MockTemplate, MockUuidGenerator);

        let result = processor.parse_ark_id("ark:/12345/1/0001/resource123");
        assert!(result.is_ok());

        let ark_info = result.unwrap();
        assert_eq!(ark_info.url_version, 1);
        assert_eq!(ark_info.project_id, Some("0001".to_string()));
        assert_eq!(ark_info.resource_id, Some("resource123".to_string()));
    }

    #[test]
    fn test_generate_redirect_url_top_level() {
        let processor =
            ArkUrlInfoProcessor::new(MockParser, MockConfig, MockTemplate, MockUuidGenerator);

        let ark_info = ArkUrlInfo::new(1, None, None, None, None);
        let result = processor.generate_redirect_url(&ark_info);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com/top");
    }

    #[test]
    fn test_generate_redirect_url_project_level() {
        let processor =
            ArkUrlInfoProcessor::new(MockParser, MockConfig, MockTemplate, MockUuidGenerator);

        let ark_info = ArkUrlInfo::new(1, Some("0001".to_string()), None, None, None);
        let result = processor.generate_redirect_url(&ark_info);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com/substituted");
    }

    #[test]
    fn test_generate_resource_iri() {
        let processor =
            ArkUrlInfoProcessor::new(MockParser, MockConfig, MockTemplate, MockUuidGenerator);

        let ark_info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            None,
        );
        let result = processor.generate_resource_iri(&ark_info);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://example.com/substituted");
    }

    #[test]
    fn test_generate_resource_iri_no_project_id() {
        let processor =
            ArkUrlInfoProcessor::new(MockParser, MockConfig, MockTemplate, MockUuidGenerator);

        let ark_info = ArkUrlInfo::new(1, None, None, None, None);
        let result = processor.generate_resource_iri(&ark_info);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ArkUrlInfoError::ProjectIdRequired);
    }

    #[test]
    fn test_determine_redirect_template() {
        let processor =
            ArkUrlInfoProcessor::new(MockParser, MockConfig, MockTemplate, MockUuidGenerator);

        // Project level
        let project_info = ArkUrlInfo::new(1, Some("0001".to_string()), None, None, None);
        let result = processor.determine_redirect_template(&project_info);
        assert_eq!(result.unwrap(), "DSPProjectRedirectUrl");

        // Resource level
        let resource_info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            None,
        );
        let result = processor.determine_redirect_template(&resource_info);
        assert_eq!(result.unwrap(), "DSPResourceRedirectUrl");

        // Resource level with timestamp
        let resource_info_with_timestamp = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            Some("20240101T123456Z".to_string()),
        );
        let result = processor.determine_redirect_template(&resource_info_with_timestamp);
        assert_eq!(result.unwrap(), "DSPResourceVersionRedirectUrl");

        // Value level
        let value_info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            Some("value456".to_string()),
            None,
        );
        let result = processor.determine_redirect_template(&value_info);
        assert_eq!(result.unwrap(), "DSPValueRedirectUrl");

        // Value level with timestamp
        let value_info_with_timestamp = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            Some("value456".to_string()),
            Some("20240101T123456Z".to_string()),
        );
        let result = processor.determine_redirect_template(&value_info_with_timestamp);
        assert_eq!(result.unwrap(), "DSPValueVersionRedirectUrl");
    }
}
