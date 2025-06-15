use crate::ark_url_settings::load_settings;
use crate::ark_url_settings::ArkUrlSettings;
use crate::base64url_ckeck_digit::{
    calculate_check_digit, calculate_modulus, is_valid, to_check_digit, to_int, weighted_value,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::wrap_pyfunction_bound;
use tracing_subscriber::prelude::*;

mod ark_url_settings;
mod base64url_ckeck_digit;
mod parsing;

#[pyfunction]
pub fn initialize_tracing(py_impl: Bound<'_, PyAny>) {
    tracing_subscriber::registry()
        .with(pyo3_python_tracing_subscriber::PythonCallbackLayerBridge::new(py_impl))
        .init();
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
    
    // Settings and tracing functions
    m.add_function(wrap_pyfunction_bound!(load_settings, py)?)?;
    m.add_function(wrap_pyfunction!(initialize_tracing, m)?)?;
    m.add_class::<ArkUrlSettings>()?;

    Ok(())
}
