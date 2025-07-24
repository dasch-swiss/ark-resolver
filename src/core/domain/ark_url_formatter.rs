/// Pure domain logic for ARK URL formatting operations.
/// This module contains formatting functions without any external dependencies.
use crate::core::errors::ark_url_formatter::ArkUrlFormatterError;

/// Validates and extracts components from a DSP resource IRI.
///
/// Returns a tuple of (project_id, resource_id) if the IRI is valid.
pub fn parse_resource_iri(
    resource_iri: &str,
    resource_iri_pattern: &str,
) -> Result<(String, String), ArkUrlFormatterError> {
    use regex::Regex;

    let regex = Regex::new(resource_iri_pattern)
        .map_err(|e| ArkUrlFormatterError::InvalidRegexPattern(e.to_string()))?;

    let captures = regex
        .captures(resource_iri)
        .ok_or_else(|| ArkUrlFormatterError::InvalidResourceIri(resource_iri.to_string()))?;

    let project_id = captures
        .get(1)
        .ok_or_else(|| ArkUrlFormatterError::InvalidResourceIri(resource_iri.to_string()))?
        .as_str()
        .to_string();

    let resource_id = captures
        .get(2)
        .ok_or_else(|| ArkUrlFormatterError::InvalidResourceIri(resource_iri.to_string()))?
        .as_str()
        .to_string();

    Ok((project_id, resource_id))
}

/// Formats an ARK ID from the given components.
///
/// This creates the ark:/ identifier format without the HTTP URL wrapper.
pub fn format_ark_id(
    ark_naan: &str,
    dsp_ark_version: &str,
    project_id: &str,
    escaped_resource_id_with_check_digit: &str,
    timestamp: Option<&str>,
) -> String {
    let mut ark_id = format!(
        "ark:/{ark_naan}/{dsp_ark_version}/{project_id}/{escaped_resource_id_with_check_digit}"
    );

    if let Some(ts) = timestamp {
        ark_id.push('.');
        ark_id.push_str(ts);
    }

    ark_id
}

/// Parameters for formatting ARK URLs
pub struct ArkUrlParams<'a> {
    pub use_https: bool,
    pub external_host: &'a str,
    pub ark_naan: &'a str,
    pub dsp_ark_version: &'a str,
    pub project_id: &'a str,
    pub escaped_resource_id_with_check_digit: &'a str,
    pub escaped_value_id_with_check_digit: Option<&'a str>,
    pub timestamp: Option<&'a str>,
}

/// Formats a complete ARK URL from the given components.
///
/// This creates the full HTTP(S) URL that can be used for redirection.
pub fn format_ark_url(params: ArkUrlParams) -> String {
    let protocol = if params.use_https { "https" } else { "http" };

    let mut url = format!(
        "{}://{}/ark:/{}/{}/{}/{}",
        protocol,
        params.external_host,
        params.ark_naan,
        params.dsp_ark_version,
        params.project_id,
        params.escaped_resource_id_with_check_digit
    );

    // Add value ID if present
    if let Some(value_id) = params.escaped_value_id_with_check_digit {
        url.push('/');
        url.push_str(value_id);
    }

    // Add timestamp if present
    if let Some(ts) = params.timestamp {
        url.push('.');
        url.push_str(ts);
    }

    url
}

/// Validates that a timestamp has the correct format.
///
/// This is a basic validation to ensure the timestamp is not empty.
pub fn validate_timestamp(timestamp: &str) -> Result<(), ArkUrlFormatterError> {
    if timestamp.is_empty() {
        return Err(ArkUrlFormatterError::InvalidTimestamp(
            timestamp.to_string(),
        ));
    }
    Ok(())
}

/// Validates that a project ID is not empty.
pub fn validate_project_id(project_id: &str) -> Result<(), ArkUrlFormatterError> {
    if project_id.is_empty() {
        return Err(ArkUrlFormatterError::InvalidProjectId(
            project_id.to_string(),
        ));
    }
    Ok(())
}

/// Validates that a resource ID is not empty.
pub fn validate_resource_id(resource_id: &str) -> Result<(), ArkUrlFormatterError> {
    if resource_id.is_empty() {
        return Err(ArkUrlFormatterError::InvalidResourceId(
            resource_id.to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resource_iri_valid() {
        let resource_iri = "http://rdfh.ch/0001/cmfk1DMHRBiR4-_6HXpEFA";
        let pattern = r"^http://rdfh\.ch/([0-9A-Fa-f]{4})/(.*)$";

        let result = parse_resource_iri(resource_iri, pattern).unwrap();
        assert_eq!(result.0, "0001");
        assert_eq!(result.1, "cmfk1DMHRBiR4-_6HXpEFA");
    }

    #[test]
    fn test_parse_resource_iri_invalid() {
        let resource_iri = "invalid://example.com/resource";
        let pattern = r"^http://rdfh\.ch/([0-9A-Fa-f]{4})/(.*)$";

        let result = parse_resource_iri(resource_iri, pattern);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidResourceIri(_)
        ));
    }

    #[test]
    fn test_format_ark_id_without_timestamp() {
        let result = format_ark_id("00000", "1", "0001", "cmfk1DMHRBiR4=_6HXpEFAn", None);
        assert_eq!(result, "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn");
    }

    #[test]
    fn test_format_ark_id_with_timestamp() {
        let result = format_ark_id(
            "00000",
            "1",
            "0001",
            "cmfk1DMHRBiR4=_6HXpEFAn",
            Some("20180604T085622513Z"),
        );
        assert_eq!(
            result,
            "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z"
        );
    }

    #[test]
    fn test_format_ark_url_https_no_value_no_timestamp() {
        let params = ArkUrlParams {
            use_https: true,
            external_host: "ark.example.org",
            ark_naan: "00000",
            dsp_ark_version: "1",
            project_id: "0001",
            escaped_resource_id_with_check_digit: "cmfk1DMHRBiR4=_6HXpEFAn",
            escaped_value_id_with_check_digit: None,
            timestamp: None,
        };
        let result = format_ark_url(params);
        assert_eq!(
            result,
            "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
        );
    }

    #[test]
    fn test_format_ark_url_with_value_and_timestamp() {
        let params = ArkUrlParams {
            use_https: true,
            external_host: "ark.example.org",
            ark_naan: "00000",
            dsp_ark_version: "1",
            project_id: "0001",
            escaped_resource_id_with_check_digit: "cmfk1DMHRBiR4=_6HXpEFAn",
            escaped_value_id_with_check_digit: Some("pLlW4ODASumZfZFbJdpw1gu"),
            timestamp: Some("20180604T085622513Z"),
        };
        let result = format_ark_url(params);
        assert_eq!(result, "https://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622513Z");
    }

    #[test]
    fn test_format_ark_url_http_protocol() {
        let params = ArkUrlParams {
            use_https: false,
            external_host: "ark.example.org",
            ark_naan: "00000",
            dsp_ark_version: "1",
            project_id: "0001",
            escaped_resource_id_with_check_digit: "cmfk1DMHRBiR4=_6HXpEFAn",
            escaped_value_id_with_check_digit: None,
            timestamp: None,
        };
        let result = format_ark_url(params);
        assert_eq!(
            result,
            "http://ark.example.org/ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"
        );
    }

    #[test]
    fn test_validate_timestamp_valid() {
        assert!(validate_timestamp("20180604T085622513Z").is_ok());
        assert!(validate_timestamp("2018").is_ok());
    }

    #[test]
    fn test_validate_timestamp_empty() {
        let result = validate_timestamp("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidTimestamp(_)
        ));
    }

    #[test]
    fn test_validate_project_id_valid() {
        assert!(validate_project_id("0001").is_ok());
        assert!(validate_project_id("080E").is_ok());
    }

    #[test]
    fn test_validate_project_id_empty() {
        let result = validate_project_id("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidProjectId(_)
        ));
    }

    #[test]
    fn test_validate_resource_id_valid() {
        assert!(validate_resource_id("cmfk1DMHRBiR4-_6HXpEFA").is_ok());
        assert!(validate_resource_id("test123").is_ok());
    }

    #[test]
    fn test_validate_resource_id_empty() {
        let result = validate_resource_id("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ArkUrlFormatterError::InvalidResourceId(_)
        ));
    }
}
