#![allow(dead_code)]
#![allow(clippy::enum_variant_names)]

use pyo3::{exceptions::PyValueError, pyfunction, PyResult};
use thiserror::Error;

/// The base64url alphabet (without padding) from RFC 4648, Table 2.
const BASE64URL_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
/// The base64url alphabet length
const BASE64URL_ALPHABET_LENGTH: usize = 64;

/// Custom error type for check digit operations
#[derive(Error, Debug)]
pub enum CheckDigitError {
    #[error("Invalid code: {0}")]
    InvalidCode(String),
    #[error("Invalid base64url character: '{0}'")]
    InvalidCharacter(char),
    #[error("Invalid character value: {0}")]
    InvalidCharacterValue(i32),
}

/// Checks whether a code with a check digit is valid.
#[pyfunction]
pub fn is_valid(code: &str) -> PyResult<bool> {
    if code.is_empty() {
        return Ok(false);
    }

    match calculate_modulus_internal(code, true) {
        Ok(modulus_result) => Ok(modulus_result == 0),
        Err(_) => Ok(false),
    }
}

/// Calculates the check digit for a code.
#[pyfunction]
pub fn calculate_check_digit(code: &str) -> PyResult<String> {
    if code.is_empty() {
        return Err(PyValueError::new_err("No code provided"));
    }

    let modulus_result = calculate_modulus_internal(code, false)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let char_value = (BASE64URL_ALPHABET_LENGTH - modulus_result) % BASE64URL_ALPHABET_LENGTH;

    to_check_digit_internal(char_value as i32)
        .map_err(|e| PyValueError::new_err(e.to_string()))
        .map(|c| c.to_string())
}

/// Calculates the modulus for a code.
#[pyfunction]
pub fn calculate_modulus(code: &str, includes_check_digit: bool) -> PyResult<i32> {
    calculate_modulus_internal(code, includes_check_digit)
        .map_err(|e| PyValueError::new_err(e.to_string()))
        .map(|v| v as i32)
}

/// Internal modulus calculation function
fn calculate_modulus_internal(
    code: &str,
    includes_check_digit: bool,
) -> Result<usize, CheckDigitError> {
    let mut length = code.len();

    if !includes_check_digit {
        length += 1;
    }

    let mut total = 0;

    for (i, ch) in code.chars().enumerate() {
        let right_pos = length - i;
        let char_value = to_int_internal(ch)?;
        total += weighted_value_internal(char_value, right_pos);
    }

    if total == 0 {
        return Err(CheckDigitError::InvalidCode(code.to_string()));
    }

    Ok(total % BASE64URL_ALPHABET_LENGTH)
}

/// Calculates the weighted value of a character in the code at a specified position.
#[pyfunction]
pub fn weighted_value(char_value: i32, right_pos: i32) -> i32 {
    weighted_value_internal(char_value as usize, right_pos as usize) as i32
}

/// Internal weighted value calculation
fn weighted_value_internal(char_value: usize, right_pos: usize) -> usize {
    char_value * right_pos
}

/// Converts a character at a specified position to an integer value.
#[pyfunction]
pub fn to_int(ch: char) -> PyResult<i32> {
    to_int_internal(ch)
        .map_err(|e| PyValueError::new_err(e.to_string()))
        .map(|v| v as i32)
}

/// Internal character to integer conversion
fn to_int_internal(ch: char) -> Result<usize, CheckDigitError> {
    BASE64URL_ALPHABET
        .find(ch)
        .ok_or(CheckDigitError::InvalidCharacter(ch))
}

/// Converts an integer value to a check digit.
#[pyfunction]
pub fn to_check_digit(char_value: i32) -> PyResult<String> {
    to_check_digit_internal(char_value)
        .map_err(|e| PyValueError::new_err(e.to_string()))
        .map(|c| c.to_string())
}

/// Internal integer to check digit conversion
fn to_check_digit_internal(char_value: i32) -> Result<char, CheckDigitError> {
    if char_value < 0 || char_value >= BASE64URL_ALPHABET_LENGTH as i32 {
        return Err(CheckDigitError::InvalidCharacterValue(char_value));
    }

    BASE64URL_ALPHABET
        .chars()
        .nth(char_value as usize)
        .ok_or(CheckDigitError::InvalidCharacterValue(char_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid() {
        // Test valid code with check digit
        assert!(is_valid("cmfk1DMHRBiR4-_6HXpEFAn").unwrap());

        // Test code without check digit should be invalid
        assert!(!is_valid("cmfk1DMHRBiR4-_6HXpEFA").unwrap());

        // Test empty string
        assert!(!is_valid("").unwrap());

        // Test invalid character
        assert!(!is_valid("cmfk1DMHRBiR4-_6HXpEFA@").unwrap());
    }

    #[test]
    fn test_calculate_check_digit() {
        let code = "cmfk1DMHRBiR4-_6HXpEFA";
        let result = calculate_check_digit(code).unwrap();
        assert_eq!(result, "n");

        // Verify the calculated check digit makes the code valid
        let code_with_check_digit = format!("{}{}", code, result);
        assert!(is_valid(&code_with_check_digit).unwrap());
    }

    #[test]
    fn test_calculate_check_digit_empty() {
        assert!(calculate_check_digit("").is_err());
    }

    #[test]
    fn test_calculate_modulus() {
        let code = "cmfk1DMHRBiR4-_6HXpEFAn";
        let result = calculate_modulus(code, true).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_to_int() {
        assert_eq!(to_int('A').unwrap(), 0);
        assert_eq!(to_int('Z').unwrap(), 25);
        assert_eq!(to_int('a').unwrap(), 26);
        assert_eq!(to_int('z').unwrap(), 51);
        assert_eq!(to_int('0').unwrap(), 52);
        assert_eq!(to_int('9').unwrap(), 61);
        assert_eq!(to_int('-').unwrap(), 62);
        assert_eq!(to_int('_').unwrap(), 63);

        // Test invalid character
        assert!(to_int('@').is_err());
    }

    #[test]
    fn test_to_check_digit() {
        assert_eq!(to_check_digit(0).unwrap(), "A");
        assert_eq!(to_check_digit(25).unwrap(), "Z");
        assert_eq!(to_check_digit(26).unwrap(), "a");
        assert_eq!(to_check_digit(51).unwrap(), "z");
        assert_eq!(to_check_digit(52).unwrap(), "0");
        assert_eq!(to_check_digit(61).unwrap(), "9");
        assert_eq!(to_check_digit(62).unwrap(), "-");
        assert_eq!(to_check_digit(63).unwrap(), "_");

        // Test invalid values
        assert!(to_check_digit(-1).is_err());
        assert!(to_check_digit(64).is_err());
    }

    #[test]
    fn test_weighted_value() {
        assert_eq!(weighted_value(5, 3), 15);
        assert_eq!(weighted_value(0, 10), 0);
    }
}
