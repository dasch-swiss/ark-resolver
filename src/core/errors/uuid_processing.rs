use crate::core::errors::check_digit::CheckDigitError;
use thiserror::Error;

/// Domain-specific errors for UUID processing operations
#[derive(Error, Debug, PartialEq)]
pub enum UuidProcessingError {
    #[error("Check digit operation failed")]
    CheckDigitError(#[from] CheckDigitError),
    #[error("Invalid ARK ID: {0}")]
    InvalidArkId(String),
    #[error("Empty UUID in ARK ID: {0}")]
    EmptyUuid(String),
}
