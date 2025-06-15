/// Port (trait interface) for check digit operations.
/// This defines the abstract contract that adapters must implement.
use crate::core::errors::check_digit::CheckDigitError;

/// Port trait defining the interface for check digit operations
///
/// This trait abstracts the check digit functionality, allowing different
/// implementations (PyO3, HTTP, CLI, etc.) to provide the same interface.
pub trait CheckDigitPort {
    /// Validates a code with check digit
    fn is_valid(&self, code: &str) -> Result<bool, CheckDigitError>;

    /// Calculates and returns the check digit for a code
    fn calculate_check_digit(&self, code: &str) -> Result<char, CheckDigitError>;

    /// Calculates the modulus for a code
    fn calculate_modulus(
        &self,
        code: &str,
        includes_check_digit: bool,
    ) -> Result<usize, CheckDigitError>;

    /// Calculates weighted value for a character at position
    fn weighted_value(&self, char_value: usize, right_pos: usize) -> usize;

    /// Converts a character to integer value
    fn to_int(&self, ch: char) -> Result<usize, CheckDigitError>;

    /// Converts integer value to check digit character
    fn to_check_digit(&self, char_value: i32) -> Result<char, CheckDigitError>;

    /// Generate a complete code with check digit
    fn add_check_digit(&self, code: &str) -> Result<String, CheckDigitError>;

    /// Validate and strip check digit from a complete code
    fn validate_and_strip_check_digit(
        &self,
        code_with_check_digit: &str,
    ) -> Result<String, CheckDigitError>;
}
