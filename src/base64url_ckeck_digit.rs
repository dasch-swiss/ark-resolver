use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use pyo3::{pyfunction, PyResult};

/// Encodes input using base64url and returns the result
#[pyfunction]
pub fn base64url_check_digit(data: &str) -> PyResult<String> {
    Ok(URL_SAFE.encode(data))
}
