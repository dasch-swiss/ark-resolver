use regex::Regex;

const PROJECT_ID_PATTERN: &str = "([0-9A-Fa-f]{4})";
const ENCODED_UUID_PATTERN: &str = "([A-Za-z0-9=_]+)"; // Allows base64-like characters. The '-' is not allowed in the encoded UUID.

#[allow(dead_code)]
const TIMESTAMP_PATTERN: &str = r"([0-9]{8}T[0-9]{6,15}Z)";

pub fn resource_iri_regex() -> Regex {
    Regex::new(&format!(
        r"^http://rdfh.ch/{}/([A-Za-z0-9_-]+)$",
        PROJECT_ID_PATTERN
    ))
    .unwrap()
}
// FIXME: This regex excludes timestamps
pub fn ark_path_regex(ark_naan: &str) -> Regex {
    let ark_path_pattern = format!(
        r"^ark:/{}/([0-9]+)(?:/{}(?:/{}(?:/{})?)?)?$",
        ark_naan, PROJECT_ID_PATTERN, ENCODED_UUID_PATTERN, ENCODED_UUID_PATTERN
    );
    Regex::new(&ark_path_pattern).unwrap()
}

pub fn v0_ark_path_regex(ark_naan: &str) -> Regex {
    let v0_ark_path_pattern = format!(
        r"ark:/{}/([0-9A-Fa-f]+)-([A-Za-z0-9]+)-[A-Za-z0-9]+(?:\.([0-9]{{6,8}}))?",
        ark_naan
    );
    Regex::new(&format!(r"^{}$", v0_ark_path_pattern)).unwrap()
}

#[cfg(test)]
mod tests {
    use crate::core::domain::parsing::{resource_iri_regex, PROJECT_ID_PATTERN, TIMESTAMP_PATTERN};
    use regex::Regex;

    #[test]
    fn test_timestamp_pattern() {
        let re = Regex::new(&format!(r"^{}$", TIMESTAMP_PATTERN)).unwrap();
        assert!(re.is_match("20180604T085622Z"));
        assert!(re.is_match("20180604T085622513Z"));
        assert!(re.is_match("20190118T102919Z"));
    }

    #[test]
    fn test_project_id_regex() {
        let re = Regex::new(&format!(r"^{}$", PROJECT_ID_PATTERN)).unwrap();

        // Valid project IDs
        assert!(re.is_match("0000"));
        assert!(re.is_match("fFfF"));
        assert!(re.is_match("FFFF"));
        assert!(re.is_match("080E"));
        assert!(re.is_match("080e"));

        // Invalid project IDs
        assert!(!re.is_match("000"));
        assert!(!re.is_match("00000"));
        assert!(!re.is_match("FFFFF"));
    }

    #[test]
    fn test_resource_iri_regex() {
        let re = resource_iri_regex();
        assert!(re.is_match("http://rdfh.ch/0002/0_sWRg5jT3S0PLxakX9ffg"));

        let captures = re
            .captures("http://rdfh.ch/0002/0_sWRg5jT3S0PLxakX9ffg")
            .unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "0002");
        assert_eq!(captures.get(2).unwrap().as_str(), "0_sWRg5jT3S0PLxakX9ffg");
    }

    #[test]
    fn test_ark_path_regex() {
        let re = super::ark_path_regex("00000");
        assert!(re.is_match("ark:/00000/1"));
        assert!(re.is_match("ark:/00000/1/0003"));
        assert!(re.is_match("ark:/00000/1/0003/cmfk1DMHRBiR4=_6HXpEFAn"));
        // assert!(re.is_match("ark:/00000/1/0003/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z"));
        // assert!(re.is_match("ark:/00000/1/0003/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z"));
        assert!(re.is_match("ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo"));
        // assert!(re.is_match("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z"));
        assert!(re.is_match("ark:/00000/1/0803/751e0b8am"));
        // assert!(re.is_match("ark:/00000/1/0803/751e0b8am.20190118T102919Z"));

        // Version 0 ARK paths that should not match
        assert!(!re.is_match("ark:/00000/0002-779b9990a0c3f-6e"));
        assert!(!re.is_match("ark:/00000/0002-779b9990a0c3f-6e.20190129"));

        // project
        let captures = re.captures("ark:/00000/1/0003").unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "1");
        assert_eq!(captures.get(2).unwrap().as_str(), "0003");

        // resource
        let captures = re
            .captures("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn")
            .unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "1");
        assert_eq!(captures.get(2).unwrap().as_str(), "0001");
        assert_eq!(captures.get(3).unwrap().as_str(), "cmfk1DMHRBiR4=_6HXpEFAn");

        // resource with timestamp
        // let captures = re.captures("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z").unwrap();
        // assert_eq!(captures.get(1).unwrap().as_str(), "1");
        // assert_eq!(captures.get(2).unwrap().as_str(), "0001");
        // assert_eq!(captures.get(3).unwrap().as_str(), "cmfk1DMHRBiR4=_6HXpEFAn");
        // assert_eq!(captures.get(4).unwrap().as_str(), "20180604T085622Z");

        // resource with value
        let captures = re
            .captures("ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo")
            .unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "1");
        assert_eq!(captures.get(2).unwrap().as_str(), "0005");
        assert_eq!(captures.get(3).unwrap().as_str(), "SQkTPdHdTzq_gqbwj6QR=AR");
        assert_eq!(captures.get(4).unwrap().as_str(), "=SSbnPK3Q7WWxzBT1UPpRgo");

        // resource with value and timestamp
        // let captures = re.captures("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z").unwrap();
        // assert_eq!(captures.get(1).unwrap().as_str(), "1");
        // assert_eq!(captures.get(2).unwrap().as_str(), "0001");
        // assert_eq!(captures.get(3).unwrap().as_str(), "cmfk1DMHRBiR4=_6HXpEFAn");
        // assert_eq!(captures.get(4).unwrap().as_str(), "pLlW4ODASumZfZFbJdpw1gu");
        // assert_eq!(captures.get(5).unwrap().as_str(), "20180604T085622Z");
    }

    #[test]
    fn test_v0_ark_path_regex() {
        let re = super::v0_ark_path_regex("00000");
        assert!(re.is_match("ark:/00000/0002-779b9990a0c3f-6e"));
        assert!(re.is_match("ark:/00000/0002-779b9990a0c3f-6e.20190129"));
        assert!(re.is_match("ark:/00000/080e-76bb2132d30d6-0"));
        assert!(re.is_match("ark:/00000/080e-76bb2132d30d6-0.20190129"));
        assert!(re.is_match("ark:/00000/080e-76bb2132d30d6-0.2019111"));

        // Version 1 ARK paths that should not match
        assert!(!re.is_match("ark:/00000/1/0003"));
        assert!(!re.is_match("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn"));
        assert!(!re.is_match("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z"));
        assert!(!re.is_match("ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z"));
        assert!(!re.is_match("ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo"));
        assert!(!re.is_match(
            "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z"
        ));
    }
}
