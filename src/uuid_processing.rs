use crate::base64url_ckeck_digit::{calculate_check_digit, is_valid};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Add a check digit to a UUID and escape hyphens for ARK URL compatibility.
///
/// This function:
/// 1. Calculates a check digit for the given UUID
/// 2. Appends the check digit to the UUID
/// 3. Escapes all hyphens (-) as equals signs (=) because hyphens can be ignored in ARK URLs
///
/// # Arguments
/// * `uuid` - The Base64-encoded UUID string to process
///
/// # Returns
/// * `Result<String, PyErr>` - The UUID with check digit and escaped hyphens, or an error
pub fn add_check_digit_and_escape_internal(uuid: &str) -> PyResult<String> {
    let check_digit = calculate_check_digit(uuid)?;
    let uuid_with_check_digit = format!("{}{}", uuid, check_digit);
    Ok(uuid_with_check_digit.replace('-', "="))
}

/// Unescape and validate a UUID from an ARK URL.
///
/// This function:
/// 1. Unescapes equals signs (=) back to hyphens (-)
/// 2. Validates the UUID using check digit validation
/// 3. Returns the UUID without the check digit
/// 4. Raises an exception if validation fails
///
/// # Arguments
/// * `ark_url` - The original ARK URL (for error messages)
/// * `escaped_uuid` - The escaped UUID with check digit to process
///
/// # Returns
/// * `Result<String, PyErr>` - The validated UUID without check digit, or an error
pub fn unescape_and_validate_uuid_internal(ark_url: &str, escaped_uuid: &str) -> PyResult<String> {
    // Unescape: replace '=' with '-'
    let unescaped_uuid = escaped_uuid.replace('=', "-");

    // Validate using check digit
    if !is_valid(&unescaped_uuid)? {
        return Err(PyValueError::new_err(format!(
            "Invalid ARK ID: {}",
            ark_url
        )));
    }

    // Return UUID without the check digit (remove last character)
    if unescaped_uuid.is_empty() {
        return Err(PyValueError::new_err(format!(
            "Empty UUID in ARK ID: {}",
            ark_url
        )));
    }

    Ok(unescaped_uuid[..unescaped_uuid.len() - 1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_check_digit_and_escape_basic() {
        // Test with a simple UUID
        let uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let result = add_check_digit_and_escape_internal(uuid).unwrap();

        // Should contain the original UUID, a check digit, and have hyphens escaped
        assert!(result.contains("0001=12345678=abcd=ef12=3456=789012345678"));
        assert!(!result.contains('-')); // No hyphens should remain
        assert!(result.len() > uuid.len()); // Should be longer due to check digit
    }

    #[test]
    fn test_add_check_digit_and_escape_no_hyphens() {
        // Test with UUID that has no hyphens
        let uuid = "000112345678abcdef123456789012345678";
        let result = add_check_digit_and_escape_internal(uuid).unwrap();

        // Should be the UUID + check digit with no changes needed
        assert_eq!(result.len(), uuid.len() + 1); // Original + 1 check digit
        assert!(!result.contains('-'));
        assert!(!result.contains('='));
    }

    #[test]
    fn test_unescape_and_validate_uuid_valid() {
        // Create a valid UUID with check digit and escape it
        let original_uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let escaped_with_check = add_check_digit_and_escape_internal(original_uuid).unwrap();

        // Now test unescaping and validation
        let result = unescape_and_validate_uuid_internal("ark:/test", &escaped_with_check).unwrap();

        // Should get back the original UUID
        assert_eq!(result, original_uuid);
    }

    #[test]
    fn test_unescape_and_validate_uuid_invalid_check_digit() {
        // Create an invalid UUID (modify the check digit)
        let escaped_uuid = "0001=12345678=abcd=ef12=3456=789012345678X"; // Invalid check digit

        let result = unescape_and_validate_uuid_internal("ark:/test", escaped_uuid);

        // Should return an error
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Invalid ARK ID"));
    }

    #[test]
    fn test_unescape_and_validate_uuid_empty_input() {
        let result = unescape_and_validate_uuid_internal("ark:/test", "");

        // Should return an error for empty input
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Empty UUID"));
    }

    #[test]
    fn test_round_trip_processing() {
        // Test that add_check_digit_and_escape -> unescape_and_validate_uuid is a round trip
        let original_uuid = "0002-70aWaB2kWsuiN6ujYgM0ZQ";

        // Process forward
        let escaped = add_check_digit_and_escape_internal(original_uuid).unwrap();

        // Process backward
        let recovered = unescape_and_validate_uuid_internal("ark:/test", &escaped).unwrap();

        // Should get back the original
        assert_eq!(recovered, original_uuid);
    }

    #[test]
    fn test_escaping_behavior() {
        // Test that hyphens are properly escaped and unescaped
        let uuid_with_hyphens = "a-b-c-d-e";
        let escaped = add_check_digit_and_escape_internal(uuid_with_hyphens).unwrap();

        // Should have no hyphens
        assert!(!escaped.contains('-'));
        assert!(escaped.contains('='));

        // Unescaping should restore hyphens (though this specific UUID might not validate)
        let unescaped_portion = escaped.replace('=', "-");
        assert!(unescaped_portion.contains('-'));
        assert!(!unescaped_portion.contains('='));
    }

    #[test]
    fn test_known_python_parity_cases() {
        // Test cases that ensure our Rust implementation matches Python behavior exactly
        // These test cases are derived from the actual Python implementation to ensure parity

        let test_cases = vec![
            (
                "0001-12345678-abcd-ef12-3456-789012345678",
                "0001=12345678=abcd=ef12=3456=789012345678U",
            ),
            (
                "0002-70aWaB2kWsuiN6ujYgM0ZQ",
                "0002=70aWaB2kWsuiN6ujYgM0ZQ5",
            ),
            ("0003-abc123def456", "0003=abc123def4560"),
            (
                "000112345678abcdef123456789012345678",
                "000112345678abcdef1234567890123456789",
            ),
            ("a-b-c-d-e", "a=b=c=d=e8"),
            (
                "test-uuid-with-many-hyphens-here",
                "test=uuid=with=many=hyphens=here7",
            ),
            ("singleword", "singlewordn"),
            (
                "0000-0000-0000-0000-0000-000000000000",
                "0000=0000=0000=0000=0000=000000000000A",
            ),
        ];

        for (input, expected) in test_cases {
            let result = add_check_digit_and_escape_internal(input).unwrap();
            assert_eq!(
                result, expected,
                "Python parity failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_cross_implementation_compatibility() {
        // Test that our implementation can round-trip with known Python outputs
        let python_generated_cases = vec![
            (
                "0001-12345678-abcd-ef12-3456-789012345678",
                "0001=12345678=abcd=ef12=3456=789012345678U",
            ),
            (
                "0002-70aWaB2kWsuiN6ujYgM0ZQ",
                "0002=70aWaB2kWsuiN6ujYgM0ZQ5",
            ),
            ("0003-abc123def456", "0003=abc123def4560"),
        ];

        for (original_uuid, python_escaped) in python_generated_cases {
            // Our Rust implementation should produce the same escaped output
            let rust_escaped = add_check_digit_and_escape_internal(original_uuid).unwrap();
            assert_eq!(
                rust_escaped, python_escaped,
                "Rust should match Python output for: {}",
                original_uuid
            );

            // Our Rust implementation should be able to process Python-generated escaped UUIDs
            let recovered =
                unescape_and_validate_uuid_internal("ark:/test", &python_escaped).unwrap();
            assert_eq!(
                recovered, original_uuid,
                "Rust should recover Python-escaped UUID: {}",
                python_escaped
            );
        }
    }
}
