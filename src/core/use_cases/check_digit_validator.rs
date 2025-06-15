/// Use case layer for check digit validation operations.
/// This layer orchestrates domain functions and provides business logic coordination.
use crate::core::domain::check_digit;
use crate::core::errors::check_digit::CheckDigitError;

/// Use case orchestrator for check digit operations
pub struct CheckDigitValidator;

impl CheckDigitValidator {
    /// Create a new CheckDigitValidator instance
    pub fn new() -> Self {
        Self
    }

    /// Validates a code with check digit
    ///
    /// Business rule: Empty codes are considered invalid
    /// Business rule: Invalid characters result in false (not error)
    pub fn is_valid(&self, code: &str) -> Result<bool, CheckDigitError> {
        check_digit::is_valid(code)
    }

    /// Calculates and returns the check digit for a code
    ///
    /// Business rule: Empty codes are rejected with error
    /// Business rule: Invalid characters result in error
    pub fn calculate_check_digit(&self, code: &str) -> Result<char, CheckDigitError> {
        check_digit::calculate_check_digit(code)
    }

    /// Calculates the modulus for a code (used internally for validation)
    ///
    /// Business rule: Used for both validation and check digit calculation
    pub fn calculate_modulus(
        &self,
        code: &str,
        includes_check_digit: bool,
    ) -> Result<usize, CheckDigitError> {
        check_digit::calculate_modulus(code, includes_check_digit)
    }

    /// Calculates weighted value for a character at position
    ///
    /// This is a utility function exposed for compatibility with existing API
    pub fn weighted_value(&self, char_value: usize, right_pos: usize) -> usize {
        check_digit::weighted_value(char_value, right_pos)
    }

    /// Converts a character to integer value
    ///
    /// Business rule: Only base64url characters are valid
    pub fn to_int(&self, ch: char) -> Result<usize, CheckDigitError> {
        check_digit::to_int(ch)
    }

    /// Converts integer value to check digit character
    ///
    /// Business rule: Only values 0-63 are valid
    pub fn to_check_digit(&self, char_value: i32) -> Result<char, CheckDigitError> {
        check_digit::to_check_digit(char_value)
    }

    /// Generate a complete code with check digit
    ///
    /// Business rule: Combines the input code with calculated check digit
    pub fn add_check_digit(&self, code: &str) -> Result<String, CheckDigitError> {
        let check_digit_char = self.calculate_check_digit(code)?;
        Ok(format!("{}{}", code, check_digit_char))
    }

    /// Validate and strip check digit from a complete code
    ///
    /// Business rule: Returns the code without check digit if valid
    /// Business rule: Returns error if check digit validation fails
    pub fn validate_and_strip_check_digit(
        &self,
        code_with_check_digit: &str,
    ) -> Result<String, CheckDigitError> {
        if !self.is_valid(code_with_check_digit)? {
            return Err(CheckDigitError::InvalidCode(
                code_with_check_digit.to_string(),
            ));
        }

        if code_with_check_digit.is_empty() {
            return Err(CheckDigitError::EmptyCode);
        }

        // Remove the last character (check digit)
        let code_without_check_digit = &code_with_check_digit[..code_with_check_digit.len() - 1];
        Ok(code_without_check_digit.to_string())
    }
}

impl Default for CheckDigitValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_is_valid() {
        let validator = CheckDigitValidator::new();

        // Test valid code with check digit
        assert!(validator.is_valid("cmfk1DMHRBiR4-_6HXpEFAn").unwrap());

        // Test code without check digit should be invalid
        assert!(!validator.is_valid("cmfk1DMHRBiR4-_6HXpEFA").unwrap());

        // Test empty string
        assert!(!validator.is_valid("").unwrap());

        // Test invalid character
        assert!(!validator.is_valid("cmfk1DMHRBiR4-_6HXpEFA@").unwrap());
    }

    #[test]
    fn test_validator_calculate_check_digit() {
        let validator = CheckDigitValidator::new();

        let code = "cmfk1DMHRBiR4-_6HXpEFA";
        let result = validator.calculate_check_digit(code).unwrap();
        assert_eq!(result, 'n');

        // Verify the calculated check digit makes the code valid
        let code_with_check_digit = format!("{}{}", code, result);
        assert!(validator.is_valid(&code_with_check_digit).unwrap());
    }

    #[test]
    fn test_validator_add_check_digit() {
        let validator = CheckDigitValidator::new();

        let code = "cmfk1DMHRBiR4-_6HXpEFA";
        let result = validator.add_check_digit(code).unwrap();
        assert_eq!(result, "cmfk1DMHRBiR4-_6HXpEFAn");

        // Verify the result is valid
        assert!(validator.is_valid(&result).unwrap());
    }

    #[test]
    fn test_validator_validate_and_strip_check_digit() {
        let validator = CheckDigitValidator::new();

        let code_with_check_digit = "cmfk1DMHRBiR4-_6HXpEFAn";
        let result = validator
            .validate_and_strip_check_digit(code_with_check_digit)
            .unwrap();
        assert_eq!(result, "cmfk1DMHRBiR4-_6HXpEFA");
    }

    #[test]
    fn test_validator_validate_and_strip_check_digit_invalid() {
        let validator = CheckDigitValidator::new();

        let invalid_code = "cmfk1DMHRBiR4-_6HXpEFAx"; // wrong check digit
        assert!(validator
            .validate_and_strip_check_digit(invalid_code)
            .is_err());
    }

    #[test]
    fn test_validator_calculate_modulus() {
        let validator = CheckDigitValidator::new();

        let code = "cmfk1DMHRBiR4-_6HXpEFAn";
        let result = validator.calculate_modulus(code, true).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_validator_to_int() {
        let validator = CheckDigitValidator::new();

        assert_eq!(validator.to_int('A').unwrap(), 0);
        assert_eq!(validator.to_int('Z').unwrap(), 25);
        assert_eq!(validator.to_int('a').unwrap(), 26);
        assert_eq!(validator.to_int('z').unwrap(), 51);
        assert_eq!(validator.to_int('0').unwrap(), 52);
        assert_eq!(validator.to_int('9').unwrap(), 61);
        assert_eq!(validator.to_int('-').unwrap(), 62);
        assert_eq!(validator.to_int('_').unwrap(), 63);

        // Test invalid character
        assert!(validator.to_int('@').is_err());
    }

    #[test]
    fn test_validator_to_check_digit() {
        let validator = CheckDigitValidator::new();

        assert_eq!(validator.to_check_digit(0).unwrap(), 'A');
        assert_eq!(validator.to_check_digit(25).unwrap(), 'Z');
        assert_eq!(validator.to_check_digit(26).unwrap(), 'a');
        assert_eq!(validator.to_check_digit(51).unwrap(), 'z');
        assert_eq!(validator.to_check_digit(52).unwrap(), '0');
        assert_eq!(validator.to_check_digit(61).unwrap(), '9');
        assert_eq!(validator.to_check_digit(62).unwrap(), '-');
        assert_eq!(validator.to_check_digit(63).unwrap(), '_');

        // Test invalid values
        assert!(validator.to_check_digit(-1).is_err());
        assert!(validator.to_check_digit(64).is_err());
    }

    #[test]
    fn test_validator_weighted_value() {
        let validator = CheckDigitValidator::new();

        assert_eq!(validator.weighted_value(5, 3), 15);
        assert_eq!(validator.weighted_value(0, 10), 0);
    }

    #[test]
    fn test_validator_default() {
        let validator = CheckDigitValidator::default();
        assert!(validator.is_valid("cmfk1DMHRBiR4-_6HXpEFAn").unwrap());
    }
}
