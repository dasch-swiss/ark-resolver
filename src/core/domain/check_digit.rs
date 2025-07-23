/// Pure domain logic for check digit operations.
/// This module contains mathematical functions without any external dependencies.
use crate::core::errors::check_digit::CheckDigitError;

/// The base64url alphabet (without padding) from RFC 4648, Table 2.
const BASE64URL_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const BASE64URL_ALPHABET_LENGTH: usize = 64;

/// Checks whether a code with a check digit is valid.
pub fn is_valid(code: &str) -> Result<bool, CheckDigitError> {
    if code.is_empty() {
        return Ok(false);
    }

    match calculate_modulus(code, true) {
        Ok(modulus_result) => Ok(modulus_result == 0),
        Err(_) => Ok(false),
    }
}

/// Calculates the check digit for a code.
pub fn calculate_check_digit(code: &str) -> Result<char, CheckDigitError> {
    if code.is_empty() {
        return Err(CheckDigitError::EmptyCode);
    }

    let modulus_result = calculate_modulus(code, false)?;
    let char_value = (BASE64URL_ALPHABET_LENGTH - modulus_result) % BASE64URL_ALPHABET_LENGTH;

    to_check_digit(char_value as i32)
}

/// Calculates the modulus for a code.
pub fn calculate_modulus(code: &str, includes_check_digit: bool) -> Result<usize, CheckDigitError> {
    let mut length = code.len();

    if !includes_check_digit {
        length += 1;
    }

    let mut total = 0;

    for (i, ch) in code.chars().enumerate() {
        let right_pos = length - i;
        let char_value = to_int(ch)?;
        total += weighted_value(char_value, right_pos);
    }

    if total == 0 {
        return Err(CheckDigitError::InvalidCode(code.to_string()));
    }

    Ok(total % BASE64URL_ALPHABET_LENGTH)
}

/// Calculates the weighted value of a character in the code at a specified position.
pub fn weighted_value(char_value: usize, right_pos: usize) -> usize {
    char_value * right_pos
}

/// Converts a character to an integer value based on the base64url alphabet.
pub fn to_int(ch: char) -> Result<usize, CheckDigitError> {
    BASE64URL_ALPHABET
        .find(ch)
        .ok_or(CheckDigitError::InvalidCharacter(ch))
}

/// Converts an integer value to a check digit character.
pub fn to_check_digit(char_value: i32) -> Result<char, CheckDigitError> {
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
        assert_eq!(result, 'n');

        // Verify the calculated check digit makes the code valid
        let code_with_check_digit = format!("{code}{result}");
        assert!(is_valid(&code_with_check_digit).unwrap());
    }

    #[test]
    fn test_calculate_check_digit_empty() {
        assert!(matches!(
            calculate_check_digit(""),
            Err(CheckDigitError::EmptyCode)
        ));
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
        assert!(matches!(
            to_int('@'),
            Err(CheckDigitError::InvalidCharacter('@'))
        ));
    }

    #[test]
    fn test_to_check_digit() {
        assert_eq!(to_check_digit(0).unwrap(), 'A');
        assert_eq!(to_check_digit(25).unwrap(), 'Z');
        assert_eq!(to_check_digit(26).unwrap(), 'a');
        assert_eq!(to_check_digit(51).unwrap(), 'z');
        assert_eq!(to_check_digit(52).unwrap(), '0');
        assert_eq!(to_check_digit(61).unwrap(), '9');
        assert_eq!(to_check_digit(62).unwrap(), '-');
        assert_eq!(to_check_digit(63).unwrap(), '_');

        // Test invalid values
        assert!(matches!(
            to_check_digit(-1),
            Err(CheckDigitError::InvalidCharacterValue(-1))
        ));
        assert!(matches!(
            to_check_digit(64),
            Err(CheckDigitError::InvalidCharacterValue(64))
        ));
    }

    #[test]
    fn test_weighted_value() {
        assert_eq!(weighted_value(5, 3), 15);
        assert_eq!(weighted_value(0, 10), 0);
    }
}
