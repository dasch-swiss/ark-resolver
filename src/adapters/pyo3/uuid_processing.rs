//! PyO3 adapter for UUID processing operations.
//! This adapter provides PyO3-compatible functions that maintain exact API compatibility.
#![allow(clippy::useless_conversion)]
use crate::core::errors::uuid_processing::UuidProcessingError;
use crate::core::use_cases::ark_uuid_processor::ArkUuidProcessor;
use pyo3::{exceptions::PyValueError, pyfunction, PyResult};

/// Convert domain errors to PyO3 errors
fn convert_error(error: UuidProcessingError) -> pyo3::PyErr {
    PyValueError::new_err(error.to_string())
}

/// Add a check digit to a UUID and escape hyphens for ARK URL compatibility.
///
/// This is the PyO3 wrapper for the UUID processing function.
/// It adds a check digit to the given UUID and escapes all hyphens as equals signs.
#[pyfunction]
pub fn add_check_digit_and_escape(uuid: String) -> PyResult<String> {
    ArkUuidProcessor::new()
        .add_check_digit_and_escape(&uuid)
        .map_err(convert_error)
}

/// Unescape and validate a UUID from an ARK URL.
///
/// This is the PyO3 wrapper for the UUID validation function.
/// It unescapes the UUID, validates it using check digit validation, and returns
/// the UUID without the check digit.
#[pyfunction]
pub fn unescape_and_validate_uuid(ark_url: String, escaped_uuid: String) -> PyResult<String> {
    ArkUuidProcessor::new()
        .unescape_and_validate_uuid(&ark_url, &escaped_uuid)
        .map_err(convert_error)
}
