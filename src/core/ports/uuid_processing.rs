/// Port (trait interface) for UUID processing operations.
/// This defines the abstract contract that adapters must implement.
use crate::core::errors::uuid_processing::UuidProcessingError;

/// Port trait defining the interface for UUID processing operations
///
/// This trait abstracts the UUID processing functionality, allowing different
/// implementations (PyO3, HTTP, CLI, etc.) to provide the same interface.
pub trait UuidProcessingPort {
    /// Add a check digit to a UUID and escape hyphens for ARK URL compatibility
    fn add_check_digit_and_escape(&self, uuid: &str) -> Result<String, UuidProcessingError>;

    /// Unescape and validate a UUID from an ARK URL
    fn unescape_and_validate_uuid(
        &self,
        ark_url: &str,
        escaped_uuid: &str,
    ) -> Result<String, UuidProcessingError>;

    /// Process a UUID for ARK URL embedding (convenience method)
    fn process_uuid_for_ark(&self, uuid: &str) -> Result<String, UuidProcessingError>;

    /// Extract and validate a UUID from an ARK URL (convenience method)
    fn extract_uuid_from_ark(
        &self,
        ark_url: &str,
        escaped_uuid: &str,
    ) -> Result<String, UuidProcessingError>;
}
