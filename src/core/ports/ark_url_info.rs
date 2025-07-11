// Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
// SPDX-License-Identifier: Apache-2.0

//! Port definitions for ARK URL information processing.
//! These traits define the abstract interfaces for the hexagonal architecture.

use crate::core::domain::ark_url_info::ArkUrlInfo;
use crate::core::errors::ark_url_info::{ArkUrlInfoError, ArkUrlInfoResult};
use std::collections::HashMap;

/// Port for ARK URL information processing operations.
/// This is the main interface for the ARK URL info functionality.
pub trait ArkUrlInfoPort {
    /// Parses an ARK ID string and returns the extracted information.
    fn parse_ark_id(&self, ark_id: &str) -> ArkUrlInfoResult<ArkUrlInfo>;

    /// Generates a redirect URL for the given ARK URL information.
    fn generate_redirect_url(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String>;

    /// Generates a resource IRI for the given ARK URL information.
    fn generate_resource_iri(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String>;

    /// Generates a DSP-specific redirect URL for the given ARK URL information.
    fn generate_dsp_redirect_url(&self, ark_info: &ArkUrlInfo) -> ArkUrlInfoResult<String>;
}

/// Port for ARK URL parsing operations.
/// Abstracts the actual parsing logic from the use case layer.
pub trait ArkUrlParsingPort {
    /// Parses an ARK ID as version 1 format.
    /// Returns tuple of (version, project_id, resource_id, value_id, timestamp).
    fn parse_ark_v1(
        &self,
        ark_id: &str,
    ) -> Option<(
        u32,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )>;

    /// Parses an ARK ID as version 0 format.
    /// Returns tuple of (project_id, resource_id, timestamp).
    fn parse_ark_v0(&self, ark_id: &str) -> Option<(String, String, Option<String>)>;

    /// Unescapes and validates a UUID from an ARK URL.
    fn unescape_and_validate_uuid(
        &self,
        ark_url: &str,
        escaped_uuid: &str,
    ) -> ArkUrlInfoResult<String>;
}

/// Port for configuration access operations.
/// Provides access to configuration data needed for ARK URL processing.
pub trait ConfigurationPort {
    /// Gets the DSP ARK version number.
    fn get_dsp_ark_version(&self) -> u32;

    /// Checks if version 0 ARK URLs are allowed for the given project.
    fn is_version_0_allowed(&self, project_id: &str) -> ArkUrlInfoResult<bool>;

    /// Gets the top-level redirect URL.
    fn get_top_level_redirect_url(&self) -> String;

    /// Gets a project-specific template by name.
    fn get_project_template(
        &self,
        project_id: &str,
        template_name: &str,
    ) -> ArkUrlInfoResult<String>;

    /// Gets the host for a project.
    fn get_project_host(&self, project_id: &str) -> ArkUrlInfoResult<String>;
}

/// Port for string template operations.
/// Abstracts template substitution and URL encoding.
pub trait TemplatePort {
    /// Substitutes values into a template string.
    fn substitute(
        &self,
        template: &str,
        values: &HashMap<String, String>,
    ) -> ArkUrlInfoResult<String>;

    /// URL-encodes a string.
    fn url_encode(&self, input: &str) -> ArkUrlInfoResult<String>;
}

/// Port for UUID generation operations.
/// Provides UUID generation functionality for legacy migration.
pub trait UuidGenerationPort {
    /// Generates a UUID v5 from the given input string.
    fn generate_v5_uuid(&self, input: &str) -> ArkUrlInfoResult<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that the traits can be used as trait objects
    #[test]
    fn test_trait_objects() {
        fn _test_ark_url_info_port(_port: &dyn ArkUrlInfoPort) {}
        fn _test_parsing_port(_port: &dyn ArkUrlParsingPort) {}
        fn _test_config_port(_port: &dyn ConfigurationPort) {}
        fn _test_template_port(_port: &dyn TemplatePort) {}
        fn _test_uuid_port(_port: &dyn UuidGenerationPort) {}
    }

    // Test that the error types work correctly
    #[test]
    fn test_error_types() {
        let error = ArkUrlInfoError::invalid_ark_id("test");
        assert!(error.to_string().contains("test"));
    }
}
