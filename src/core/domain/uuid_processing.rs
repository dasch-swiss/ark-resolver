/// Pure domain logic for UUID processing operations.
/// This module contains UUID transformation functions without any external dependencies.
use crate::core::domain::check_digit;
use crate::core::errors::uuid_processing::UuidProcessingError;

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
/// * `Result<String, UuidProcessingError>` - The UUID with check digit and escaped hyphens, or an error
pub fn add_check_digit_and_escape(uuid: &str) -> Result<String, UuidProcessingError> {
    let check_digit =
        check_digit::calculate_check_digit(uuid).map_err(UuidProcessingError::CheckDigitError)?;

    let uuid_with_check_digit = format!("{}{}", uuid, check_digit);
    Ok(uuid_with_check_digit.replace('-', "="))
}

/// Unescape and validate a UUID from an ARK URL.
///
/// This function:
/// 1. Unescapes equals signs (=) back to hyphens (-)
/// 2. Validates the UUID using check digit validation
/// 3. Returns the UUID without the check digit
/// 4. Returns an error if validation fails
///
/// # Arguments
/// * `ark_url` - The original ARK URL (for error messages)
/// * `escaped_uuid` - The escaped UUID with check digit to process
///
/// # Returns
/// * `Result<String, UuidProcessingError>` - The validated UUID without check digit, or an error
pub fn unescape_and_validate_uuid(
    ark_url: &str,
    escaped_uuid: &str,
) -> Result<String, UuidProcessingError> {
    // Check for empty input first
    if escaped_uuid.is_empty() {
        return Err(UuidProcessingError::EmptyUuid(ark_url.to_string()));
    }

    // Unescape: replace '=' with '-'
    let unescaped_uuid = escaped_uuid.replace('=', "-");
    // Check for empty input first
    if unescaped_uuid.is_empty() {
        return Err(UuidProcessingError::EmptyUuid(ark_url.to_string()));
    }
    // Validate using check digit
    let is_valid =
        check_digit::is_valid(&unescaped_uuid).map_err(UuidProcessingError::CheckDigitError)?;

    if !is_valid {
        return Err(UuidProcessingError::InvalidArkId(ark_url.to_string()));
    }

    // Return UUID without the check digit (remove last character)
    Ok(unescaped_uuid[..unescaped_uuid.len() - 1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_check_digit_and_escape_basic() {
        // Test with a simple UUID
        let uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let result = add_check_digit_and_escape(uuid).unwrap();

        // Should contain the original UUID, a check digit, and have hyphens escaped
        assert!(result.contains("0001=12345678=abcd=ef12=3456=789012345678"));
        assert!(!result.contains('-')); // No hyphens should remain
        assert!(result.len() > uuid.len()); // Should be longer due to check digit
    }

    #[test]
    fn test_add_check_digit_and_escape_no_hyphens() {
        // Test with UUID that has no hyphens
        let uuid = "000112345678abcdef123456789012345678";
        let result = add_check_digit_and_escape(uuid).unwrap();

        // Should be the UUID + check digit with no changes needed
        assert_eq!(result.len(), uuid.len() + 1); // Original + 1 check digit
        assert!(!result.contains('-'));
        assert!(!result.contains('='));
    }

    #[test]
    fn test_unescape_and_validate_uuid_valid() {
        // Create a valid UUID with check digit and escape it
        let original_uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let escaped_with_check = add_check_digit_and_escape(original_uuid).unwrap();

        // Now test unescaping and validation
        let result = unescape_and_validate_uuid("ark:/test", &escaped_with_check).unwrap();

        // Should get back the original UUID
        assert_eq!(result, original_uuid);
    }

    #[test]
    fn test_unescape_and_validate_uuid_invalid_check_digit() {
        pyo3::prepare_freethreaded_python();

        // Create an invalid UUID (modify the check digit)
        let escaped_uuid = "0001=12345678=abcd=ef12=3456=789012345678X"; // Invalid check digit

        let result = unescape_and_validate_uuid("ark:/test", escaped_uuid);

        // Should return an error
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UuidProcessingError::InvalidArkId(_)
        ));
    }

    #[test]
    fn test_unescape_and_validate_uuid_empty_input() {
        pyo3::prepare_freethreaded_python();

        let result = unescape_and_validate_uuid("ark:/test", "");

        // Should return an error for empty input
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UuidProcessingError::EmptyUuid(_)
        ));
    }

    #[test]
    fn test_round_trip_processing() {
        // Test that add_check_digit_and_escape -> unescape_and_validate_uuid is a round trip
        let original_uuid = "0002-70aWaB2kWsuiN6ujYgM0ZQ";

        // Process forward
        let escaped = add_check_digit_and_escape(original_uuid).unwrap();

        // Process backward
        let recovered = unescape_and_validate_uuid("ark:/test", &escaped).unwrap();

        // Should get back the original
        assert_eq!(recovered, original_uuid);
    }

    #[test]
    fn test_escaping_behavior() {
        // Test that hyphens are properly escaped and unescaped
        let uuid_with_hyphens = "a-b-c-d-e";
        let escaped = add_check_digit_and_escape(uuid_with_hyphens).unwrap();

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
            let result = add_check_digit_and_escape(input).unwrap();
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
            let rust_escaped = add_check_digit_and_escape(original_uuid).unwrap();
            assert_eq!(
                rust_escaped, python_escaped,
                "Rust should match Python output for: {}",
                original_uuid
            );

            // Our Rust implementation should be able to process Python-generated escaped UUIDs
            let recovered = unescape_and_validate_uuid("ark:/test", &python_escaped).unwrap();
            assert_eq!(
                recovered, original_uuid,
                "Rust should recover Python-escaped UUID: {}",
                python_escaped
            );
        }
    }
}
