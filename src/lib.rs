use crate::adapters::pyo3::ark_url_formatter::ArkUrlFormatter;
use crate::adapters::pyo3::check_digit::{
    calculate_check_digit, calculate_modulus, is_valid, to_check_digit, to_int, weighted_value,
};
use crate::ark_url_settings::load_settings;
use crate::ark_url_settings::ArkUrlSettings;
use crate::uuid_processing::{
    add_check_digit_and_escape_internal, unescape_and_validate_uuid_internal,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction_bound;
use tracing_subscriber::prelude::*;

mod adapters;
mod ark_url_settings;
mod base64url_ckeck_digit;
pub mod core;
mod parsing;
mod uuid_processing;

#[pyfunction]
pub fn initialize_tracing(py_impl: Bound<'_, PyAny>) {
    tracing_subscriber::registry()
        .with(pyo3_python_tracing_subscriber::PythonCallbackLayerBridge::new(py_impl))
        .init();
}

/// Add a check digit to a UUID and escape hyphens for ARK URL compatibility.
///
/// This is the PyO3 wrapper for the internal UUID processing function.
/// It adds a check digit to the given UUID and escapes all hyphens as equals signs.
#[pyfunction]
pub fn add_check_digit_and_escape(uuid: String) -> PyResult<String> {
    add_check_digit_and_escape_internal(&uuid)
}

/// Unescape and validate a UUID from an ARK URL.
///
/// This is the PyO3 wrapper for the internal UUID validation function.
/// It unescapes the UUID, validates it using check digit validation, and returns
/// the UUID without the check digit.
#[pyfunction]
pub fn unescape_and_validate_uuid(ark_url: String, escaped_uuid: String) -> PyResult<String> {
    unescape_and_validate_uuid_internal(&ark_url, &escaped_uuid)
}

/// Create Python module and add the functions and classes to it
#[pymodule]
fn _rust(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Check digit functions
    m.add_function(wrap_pyfunction_bound!(is_valid, py)?)?;
    m.add_function(wrap_pyfunction_bound!(calculate_check_digit, py)?)?;
    m.add_function(wrap_pyfunction_bound!(calculate_modulus, py)?)?;
    m.add_function(wrap_pyfunction_bound!(weighted_value, py)?)?;
    m.add_function(wrap_pyfunction_bound!(to_int, py)?)?;
    m.add_function(wrap_pyfunction_bound!(to_check_digit, py)?)?;

    // UUID processing functions
    m.add_function(wrap_pyfunction_bound!(add_check_digit_and_escape, py)?)?;
    m.add_function(wrap_pyfunction_bound!(unescape_and_validate_uuid, py)?)?;

    // Settings and tracing functions
    m.add_function(wrap_pyfunction_bound!(load_settings, py)?)?;
    m.add_function(wrap_pyfunction!(initialize_tracing, m)?)?;
    m.add_class::<ArkUrlSettings>()?;

    // ARK URL formatter
    m.add_class::<ArkUrlFormatter>()?;

    Ok(())
}
