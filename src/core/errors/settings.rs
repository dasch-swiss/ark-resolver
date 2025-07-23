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

impl From<regex::Error> for SettingsError {
    fn from(err: regex::Error) -> Self {
        SettingsError::RegexError(err.to_string())
    }
}

impl From<std::io::Error> for SettingsError {
    fn from(err: std::io::Error) -> Self {
        SettingsError::FileSystemError(err.to_string())
    }
}

impl From<config::ConfigError> for SettingsError {
    fn from(err: config::ConfigError) -> Self {
        SettingsError::ParseError(err.to_string())
    }
}

impl From<std::env::VarError> for SettingsError {
    fn from(err: std::env::VarError) -> Self {
        SettingsError::EnvironmentError(err.to_string())
    }
}

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

    #[test]
    fn test_settings_error_from_regex_error() {
        let regex_err = regex::Error::Syntax("Invalid regex".to_string());
        let settings_err = SettingsError::from(regex_err);

        match settings_err {
            SettingsError::RegexError(msg) => assert!(msg.contains("Invalid regex")),
            _ => panic!("Expected RegexError"),
        }
    }

    #[test]
    fn test_settings_error_from_env_error() {
        let env_err = std::env::VarError::NotPresent;
        let settings_err = SettingsError::from(env_err);

        match settings_err {
            SettingsError::EnvironmentError(_) => (),
            _ => panic!("Expected EnvironmentError"),
        }
    }
}
