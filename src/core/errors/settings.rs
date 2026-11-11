use std::fmt;

/// Settings-specific error types for validation failures
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsError {
    /// Configuration validation failed
    ValidationError(String),
    /// Configuration file parsing failed
    ParseError(String),
    /// Environment variable access failed
    EnvironmentError(String),
    /// Required configuration key is missing
    MissingKey(String),
    /// Invalid configuration value format
    InvalidValue {
        key: String,
        value: String,
        expected: String,
    },
    /// Regex compilation failed
    RegexError(String),
    /// File system access failed
    FileSystemError(String),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::ValidationError(msg) => {
                write!(f, "Configuration validation failed: {msg}")
            }
            SettingsError::ParseError(msg) => write!(f, "Configuration parsing failed: {msg}"),
            SettingsError::EnvironmentError(msg) => {
                write!(f, "Environment variable access failed: {msg}")
            }
            SettingsError::MissingKey(key) => {
                write!(f, "Required configuration key missing: {key}")
            }
            SettingsError::InvalidValue {
                key,
                value,
                expected,
            } => {
                write!(
                    f,
                    "Invalid value '{value}' for key '{key}', expected: {expected}"
                )
            }
            SettingsError::RegexError(msg) => write!(f, "Regex compilation failed: {msg}"),
            SettingsError::FileSystemError(msg) => write!(f, "File system access failed: {msg}"),
        }
    }
}

impl std::error::Error for SettingsError {}

/// Result type for settings operations
pub type SettingsResult<T> = Result<T, SettingsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_error_display() {
        let validation_error = SettingsError::ValidationError("Invalid config".to_string());
        assert_eq!(
            validation_error.to_string(),
            "Configuration validation failed: Invalid config"
        );

        let parse_error = SettingsError::ParseError("Invalid format".to_string());
        assert_eq!(
            parse_error.to_string(),
            "Configuration parsing failed: Invalid format"
        );

        let env_error = SettingsError::EnvironmentError("Variable not found".to_string());
        assert_eq!(
            env_error.to_string(),
            "Environment variable access failed: Variable not found"
        );

        let missing_key = SettingsError::MissingKey("required_key".to_string());
        assert_eq!(
            missing_key.to_string(),
            "Required configuration key missing: required_key"
        );

        let invalid_value = SettingsError::InvalidValue {
            key: "port".to_string(),
            value: "invalid".to_string(),
            expected: "integer".to_string(),
        };
        assert_eq!(
            invalid_value.to_string(),
            "Invalid value 'invalid' for key 'port', expected: integer"
        );

        let regex_error = SettingsError::RegexError("Invalid pattern".to_string());
        assert_eq!(
            regex_error.to_string(),
            "Regex compilation failed: Invalid pattern"
        );

        let fs_error = SettingsError::FileSystemError("File not found".to_string());
        assert_eq!(
            fs_error.to_string(),
            "File system access failed: File not found"
        );
    }

    #[test]
    fn test_settings_error_equality() {
        let error1 = SettingsError::ValidationError("test".to_string());
        let error2 = SettingsError::ValidationError("test".to_string());
        let error3 = SettingsError::ValidationError("different".to_string());

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }
}
