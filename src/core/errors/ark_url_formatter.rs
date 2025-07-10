/// Error types for ARK URL formatting operations.
/// This module defines domain-specific errors for ARK URL formatting failures.
use std::fmt;

/// Errors that can occur during ARK URL formatting operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ArkUrlFormatterError {
    /// Invalid resource IRI format or content
    InvalidResourceIri(String),
    /// Invalid project ID (e.g., empty)
    InvalidProjectId(String),
    /// Invalid resource ID (e.g., empty)
    InvalidResourceId(String),
    /// Invalid timestamp format
    InvalidTimestamp(String),
    /// Invalid regex pattern for resource IRI parsing
    InvalidRegexPattern(String),
    /// Configuration error (missing required settings)
    MissingConfiguration(String),
    /// UUID processing error
    UuidProcessingError(String),
}

impl fmt::Display for ArkUrlFormatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArkUrlFormatterError::InvalidResourceIri(iri) => {
                write!(f, "Invalid resource IRI: {}", iri)
            }
            ArkUrlFormatterError::InvalidProjectId(id) => {
                write!(f, "Invalid project ID: {}", id)
            }
            ArkUrlFormatterError::InvalidResourceId(id) => {
                write!(f, "Invalid resource ID: {}", id)
            }
            ArkUrlFormatterError::InvalidTimestamp(ts) => {
                write!(f, "Invalid timestamp: {}", ts)
            }
            ArkUrlFormatterError::InvalidRegexPattern(pattern) => {
                write!(f, "Invalid regex pattern: {}", pattern)
            }
            ArkUrlFormatterError::MissingConfiguration(config) => {
                write!(f, "Missing configuration: {}", config)
            }
            ArkUrlFormatterError::UuidProcessingError(msg) => {
                write!(f, "UUID processing error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ArkUrlFormatterError {}

/// Convenience type alias for Results with ArkUrlFormatterError.
pub type ArkUrlFormatterResult<T> = Result<T, ArkUrlFormatterError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = ArkUrlFormatterError::InvalidResourceIri("test".to_string());
        assert_eq!(error.to_string(), "Invalid resource IRI: test");
    }

    #[test]
    fn test_error_equality() {
        let error1 = ArkUrlFormatterError::InvalidProjectId("test".to_string());
        let error2 = ArkUrlFormatterError::InvalidProjectId("test".to_string());
        let error3 = ArkUrlFormatterError::InvalidProjectId("other".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_error_debug() {
        let error = ArkUrlFormatterError::InvalidTimestamp("bad_timestamp".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InvalidTimestamp"));
        assert!(debug_str.contains("bad_timestamp"));
    }

    #[test]
    fn test_all_error_variants() {
        let errors = vec![
            ArkUrlFormatterError::InvalidResourceIri("test".to_string()),
            ArkUrlFormatterError::InvalidProjectId("test".to_string()),
            ArkUrlFormatterError::InvalidResourceId("test".to_string()),
            ArkUrlFormatterError::InvalidTimestamp("test".to_string()),
            ArkUrlFormatterError::InvalidRegexPattern("test".to_string()),
            ArkUrlFormatterError::MissingConfiguration("test".to_string()),
            ArkUrlFormatterError::UuidProcessingError("test".to_string()),
        ];

        // Ensure all errors implement Display
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
