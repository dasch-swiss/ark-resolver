//! PyO3 adapter for check digit operations.
//! This adapter provides PyO3-compatible functions that maintain exact API compatibility.
#![allow(clippy::useless_conversion)]
use crate::core::errors::check_digit::CheckDigitError;
use crate::core::use_cases::check_digit_validator::CheckDigitValidator;
use pyo3::{exceptions::PyValueError, pyfunction, PyResult};

/// Convert domain errors to PyO3 errors
fn convert_error(error: CheckDigitError) -> pyo3::PyErr {
    PyValueError::new_err(error.to_string())
}

/// Checks whether a code with a check digit is valid.
#[pyfunction]
pub fn is_valid(code: &str) -> PyResult<bool> {
    CheckDigitValidator::new()
        .is_valid(code)
        .map_err(convert_error)
}

/// Calculates the check digit for a code.
#[pyfunction]
pub fn calculate_check_digit(code: &str) -> PyResult<String> {
    CheckDigitValidator::new()
        .calculate_check_digit(code)
        .map(|c| c.to_string())
        .map_err(convert_error)
}

/// Calculates the modulus for a code.
#[pyfunction]
pub fn calculate_modulus(code: &str, includes_check_digit: bool) -> PyResult<i32> {
    CheckDigitValidator::new()
        .calculate_modulus(code, includes_check_digit)
        .map(|v| v as i32)
        .map_err(convert_error)
}

/// Calculates the weighted value of a character in the code at a specified position.
#[pyfunction]
pub fn weighted_value(char_value: i32, right_pos: i32) -> i32 {
    CheckDigitValidator::new().weighted_value(char_value as usize, right_pos as usize) as i32
}

/// Converts a character at a specified position to an integer value.
#[pyfunction]
pub fn to_int(ch: char) -> PyResult<i32> {
    CheckDigitValidator::new()
        .to_int(ch)
        .map(|v| v as i32)
        .map_err(convert_error)
}

/// Converts an integer value to a check digit.
#[pyfunction]
pub fn to_check_digit(char_value: i32) -> PyResult<String> {
    CheckDigitValidator::new()
        .to_check_digit(char_value)
        .map(|c| c.to_string())
        .map_err(convert_error)
}
