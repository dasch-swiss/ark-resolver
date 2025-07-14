// Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
// SPDX-License-Identifier: Apache-2.0

//! Error types for ARK URL information processing.

use thiserror::Error;

/// Errors that can occur during ARK URL information processing.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ArkUrlInfoError {
    #[error("Invalid ARK ID: {ark_id}")]
    InvalidArkId { ark_id: String },

    #[error("Invalid ARK ID {ark_id}. The version of the ARK ID doesn't match the version defined in the settings.")]
    VersionMismatch { ark_id: String },

    #[error("Invalid ARK ID (version 0 not allowed): {ark_id}")]
    Version0NotAllowed { ark_id: String },

    #[error("Project ID is required for resource IRI generation")]
    ProjectIdRequired,

    #[error("Configuration template not found: {template_name}")]
    TemplateNotFound { template_name: String },

    #[error("Template substitution failed: {message}")]
    TemplateSubstitutionFailed { message: String },

    #[error("UUID processing failed: {message}")]
    UuidProcessingFailed { message: String },

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("String template error: {message}")]
    StringTemplateError { message: String },

    #[error("Unable to determine redirect template for ARK URL")]
    RedirectTemplateUndetermined,
}

impl ArkUrlInfoError {
    /// Creates a new InvalidArkId error.
    pub fn invalid_ark_id(ark_id: impl Into<String>) -> Self {
        Self::InvalidArkId {
            ark_id: ark_id.into(),
        }
    }

    /// Creates a new VersionMismatch error.
    pub fn version_mismatch(ark_id: impl Into<String>) -> Self {
        Self::VersionMismatch {
            ark_id: ark_id.into(),
        }
    }

    /// Creates a new Version0NotAllowed error.
    pub fn version_0_not_allowed(ark_id: impl Into<String>) -> Self {
        Self::Version0NotAllowed {
            ark_id: ark_id.into(),
        }
    }

    /// Creates a new TemplateNotFound error.
    pub fn template_not_found(template_name: impl Into<String>) -> Self {
        Self::TemplateNotFound {
            template_name: template_name.into(),
        }
    }

    /// Creates a new TemplateSubstitutionFailed error.
    pub fn template_substitution_failed(message: impl Into<String>) -> Self {
        Self::TemplateSubstitutionFailed {
            message: message.into(),
        }
    }

    /// Creates a new UuidProcessingFailed error.
    pub fn uuid_processing_failed(message: impl Into<String>) -> Self {
        Self::UuidProcessingFailed {
            message: message.into(),
        }
    }

    /// Creates a new ConfigurationError.
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Creates a new StringTemplateError.
    pub fn string_template_error(message: impl Into<String>) -> Self {
        Self::StringTemplateError {
            message: message.into(),
        }
    }
}

/// Type alias for Results using ArkUrlInfoError.
pub type ArkUrlInfoResult<T> = Result<T, ArkUrlInfoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_ark_id_error() {
        let error = ArkUrlInfoError::invalid_ark_id("invalid-ark");
        assert_eq!(error.to_string(), "Invalid ARK ID: invalid-ark");
    }

    #[test]
    fn test_version_mismatch_error() {
        let error = ArkUrlInfoError::version_mismatch("ark:/12345/1");
        assert_eq!(
            error.to_string(),
            "Invalid ARK ID ark:/12345/1. The version of the ARK ID doesn't match the version defined in the settings."
        );
    }

    #[test]
    fn test_version_0_not_allowed_error() {
        let error = ArkUrlInfoError::version_0_not_allowed("ark:/12345/0001-abc-def");
        assert_eq!(
            error.to_string(),
            "Invalid ARK ID (version 0 not allowed): ark:/12345/0001-abc-def"
        );
    }

    #[test]
    fn test_project_id_required_error() {
        let error = ArkUrlInfoError::ProjectIdRequired;
        assert_eq!(
            error.to_string(),
            "Project ID is required for resource IRI generation"
        );
    }

    #[test]
    fn test_template_not_found_error() {
        let error = ArkUrlInfoError::template_not_found("DSPResourceIri");
        assert_eq!(
            error.to_string(),
            "Configuration template not found: DSPResourceIri"
        );
    }

    #[test]
    fn test_template_substitution_failed_error() {
        let error = ArkUrlInfoError::template_substitution_failed("Missing key 'host'");
        assert_eq!(
            error.to_string(),
            "Template substitution failed: Missing key 'host'"
        );
    }

    #[test]
    fn test_uuid_processing_failed_error() {
        let error = ArkUrlInfoError::uuid_processing_failed("Invalid UUID format");
        assert_eq!(
            error.to_string(),
            "UUID processing failed: Invalid UUID format"
        );
    }

    #[test]
    fn test_configuration_error() {
        let error = ArkUrlInfoError::configuration_error("Missing project config");
        assert_eq!(
            error.to_string(),
            "Configuration error: Missing project config"
        );
    }

    #[test]
    fn test_string_template_error() {
        let error = ArkUrlInfoError::string_template_error("Template parsing failed");
        assert_eq!(
            error.to_string(),
            "String template error: Template parsing failed"
        );
    }

    #[test]
    fn test_redirect_template_undetermined_error() {
        let error = ArkUrlInfoError::RedirectTemplateUndetermined;
        assert_eq!(
            error.to_string(),
            "Unable to determine redirect template for ARK URL"
        );
    }
}
