use thiserror::Error;

/// Domain-specific errors for check digit operations
#[derive(Error, Debug, PartialEq)]
pub enum CheckDigitError {
    #[error("Empty code provided")]
    EmptyCode,
    #[error("Invalid code: {0}")]
    InvalidCode(String),
    #[error("Invalid base64url character: '{0}'")]
    InvalidCharacter(char),
    #[error("Invalid character value: {0}")]
    InvalidCharacterValue(i32),
}
