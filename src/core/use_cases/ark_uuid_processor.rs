/// Use case layer for ARK UUID processing operations.
/// This layer orchestrates domain functions and provides business logic coordination.
use crate::core::domain::uuid_processing;
use crate::core::errors::uuid_processing::UuidProcessingError;

/// Use case orchestrator for ARK UUID processing operations
pub struct ArkUuidProcessor;

impl ArkUuidProcessor {
    /// Create a new ArkUuidProcessor instance
    pub fn new() -> Self {
        Self
    }

    /// Add a check digit to a UUID and escape hyphens for ARK URL compatibility.
    ///
    /// Business rule: UUIDs must have valid check digits for ARK compliance
    /// Business rule: Hyphens must be escaped for ARK URL compatibility
    ///
    /// # Arguments
    /// * `uuid` - The Base64-encoded UUID string to process
    ///
    /// # Returns
    /// * `Result<String, UuidProcessingError>` - The UUID with check digit and escaped hyphens
    pub fn add_check_digit_and_escape(&self, uuid: &str) -> Result<String, UuidProcessingError> {
        uuid_processing::add_check_digit_and_escape(uuid)
    }

    /// Unescape and validate a UUID from an ARK URL.
    ///
    /// Business rule: ARK URLs must contain valid UUIDs with check digits
    /// Business rule: Escaped characters must be properly unescaped
    /// Business rule: Check digit validation is mandatory for security
    ///
    /// # Arguments
    /// * `ark_url` - The original ARK URL (for error context)
    /// * `escaped_uuid` - The escaped UUID with check digit to process
    ///
    /// # Returns
    /// * `Result<String, UuidProcessingError>` - The validated UUID without check digit
    pub fn unescape_and_validate_uuid(
        &self,
        ark_url: &str,
        escaped_uuid: &str,
    ) -> Result<String, UuidProcessingError> {
        uuid_processing::unescape_and_validate_uuid(ark_url, escaped_uuid)
    }

    /// Process a UUID for ARK URL embedding (convenience method)
    ///
    /// Business rule: Complete ARK processing workflow in one step
    /// This combines check digit calculation and escaping for ARK URL compatibility
    ///
    /// # Arguments
    /// * `uuid` - The Base64-encoded UUID string to process
    ///
    /// # Returns
    /// * `Result<String, UuidProcessingError>` - The processed UUID ready for ARK URL embedding
    pub fn process_uuid_for_ark(&self, uuid: &str) -> Result<String, UuidProcessingError> {
        self.add_check_digit_and_escape(uuid)
    }

    /// Extract and validate a UUID from an ARK URL (convenience method)
    ///
    /// Business rule: Complete ARK extraction workflow in one step
    /// This combines unescaping and validation for ARK URL processing
    ///
    /// # Arguments
    /// * `ark_url` - The original ARK URL (for error context)
    /// * `escaped_uuid` - The escaped UUID with check digit to extract
    ///
    /// # Returns
    /// * `Result<String, UuidProcessingError>` - The extracted and validated UUID
    pub fn extract_uuid_from_ark(
        &self,
        ark_url: &str,
        escaped_uuid: &str,
    ) -> Result<String, UuidProcessingError> {
        self.unescape_and_validate_uuid(ark_url, escaped_uuid)
    }
}

impl Default for ArkUuidProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_add_check_digit_and_escape() {
        let processor = ArkUuidProcessor::new();

        // Test with a simple UUID
        let uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let result = processor.add_check_digit_and_escape(uuid).unwrap();

        // Should contain the original UUID, a check digit, and have hyphens escaped
        assert!(result.contains("0001=12345678=abcd=ef12=3456=789012345678"));
        assert!(!result.contains('-')); // No hyphens should remain
        assert!(result.len() > uuid.len()); // Should be longer due to check digit
    }

    #[test]
    fn test_processor_unescape_and_validate_uuid() {
        let processor = ArkUuidProcessor::new();

        // Create a valid UUID with check digit and escape it
        let original_uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let escaped_with_check = processor.add_check_digit_and_escape(original_uuid).unwrap();

        // Now test unescaping and validation
        let result = processor
            .unescape_and_validate_uuid("ark:/test", &escaped_with_check)
            .unwrap();

        // Should get back the original UUID
        assert_eq!(result, original_uuid);
    }

    #[test]
    fn test_processor_round_trip_processing() {
        let processor = ArkUuidProcessor::new();

        // Test that add_check_digit_and_escape -> unescape_and_validate_uuid is a round trip
        let original_uuid = "0002-70aWaB2kWsuiN6ujYgM0ZQ";

        // Process forward
        let escaped = processor.add_check_digit_and_escape(original_uuid).unwrap();

        // Process backward
        let recovered = processor
            .unescape_and_validate_uuid("ark:/test", &escaped)
            .unwrap();

        // Should get back the original
        assert_eq!(recovered, original_uuid);
    }

    #[test]
    fn test_processor_process_uuid_for_ark() {
        let processor = ArkUuidProcessor::new();

        let uuid = "0003-abc123def456";
        let result = processor.process_uuid_for_ark(uuid).unwrap();

        // Should be equivalent to add_check_digit_and_escape
        let direct_result = processor.add_check_digit_and_escape(uuid).unwrap();
        assert_eq!(result, direct_result);
    }

    #[test]
    fn test_processor_extract_uuid_from_ark() {
        let processor = ArkUuidProcessor::new();

        let original_uuid = "0002-70aWaB2kWsuiN6ujYgM0ZQ";
        let escaped = processor.add_check_digit_and_escape(original_uuid).unwrap();

        // Test extraction convenience method
        let extracted = processor
            .extract_uuid_from_ark("ark:/test", &escaped)
            .unwrap();

        // Should be equivalent to unescape_and_validate_uuid
        let direct_result = processor
            .unescape_and_validate_uuid("ark:/test", &escaped)
            .unwrap();
        assert_eq!(extracted, direct_result);
        assert_eq!(extracted, original_uuid);
    }

    #[test]
    fn test_processor_invalid_check_digit() {
        let processor = ArkUuidProcessor::new();

        // Create an invalid UUID (modify the check digit)
        let escaped_uuid = "0001=12345678=abcd=ef12=3456=789012345678X"; // Invalid check digit

        let result = processor.unescape_and_validate_uuid("ark:/test", escaped_uuid);

        // Should return an error
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UuidProcessingError::InvalidArkId(_)
        ));
    }

    #[test]
    fn test_processor_empty_input() {
        let processor = ArkUuidProcessor::new();

        let result = processor.unescape_and_validate_uuid("ark:/test", "");

        // Should return an error for empty input
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UuidProcessingError::EmptyUuid(_)
        ));
    }

    #[test]
    fn test_processor_python_parity_cases() {
        let processor = ArkUuidProcessor::new();

        // Test cases that ensure our use case layer matches Python behavior exactly
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
        ];

        for (input, expected) in test_cases {
            let result = processor.add_check_digit_and_escape(input).unwrap();
            assert_eq!(
                result, expected,
                "Python parity failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_processor_default() {
        let processor = ArkUuidProcessor::default();

        let uuid = "0001-12345678-abcd-ef12-3456-789012345678";
        let result = processor.add_check_digit_and_escape(uuid).unwrap();

        assert!(result.contains("0001=12345678=abcd=ef12=3456=789012345678"));
        assert!(!result.contains('-'));
    }
}
