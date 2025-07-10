// Copyright © 2015 - 2025 Swiss National Data and Service Center for the Humanities and/or DaSCH Service Platform contributors.
// SPDX-License-Identifier: Apache-2.0

//! Domain layer for ARK URL information processing.
//! Contains pure business logic without external dependencies.

use std::collections::HashMap;

/// Represents the core information extracted from an ARK URL.
/// This is a pure domain object with no external dependencies.
#[derive(Debug, Clone, PartialEq)]
pub struct ArkUrlInfo {
    pub url_version: u8,
    pub project_id: Option<String>,
    pub resource_id: Option<String>,
    pub value_id: Option<String>,
    pub timestamp: Option<String>,
}

impl ArkUrlInfo {
    /// Creates a new ArkUrlInfo instance with the given components.
    pub fn new(
        url_version: u8,
        project_id: Option<String>,
        resource_id: Option<String>,
        value_id: Option<String>,
        timestamp: Option<String>,
    ) -> Self {
        Self {
            url_version,
            project_id,
            resource_id,
            value_id,
            timestamp,
        }
    }

    /// Returns the timestamp formatted for the specific ARK version.
    pub fn get_timestamp(&self) -> Option<String> {
        match (self.url_version, &self.timestamp) {
            (0, Some(ts)) => {
                // Version 0 ARK URLs need time appended
                Some(format!("{}T000000Z", ts))
            }
            (_, ts) => ts.clone(),
        }
    }

    /// Creates a template dictionary for string substitution.
    pub fn to_template_dict(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();
        
        dict.insert("url_version".to_string(), self.url_version.to_string());
        
        if let Some(ref project_id) = self.project_id {
            dict.insert("project_id".to_string(), project_id.clone());
        }
        
        if let Some(ref resource_id) = self.resource_id {
            dict.insert("resource_id".to_string(), resource_id.clone());
        }
        
        if let Some(ref value_id) = self.value_id {
            dict.insert("value_id".to_string(), value_id.clone());
        }
        
        if let Some(ref timestamp) = self.timestamp {
            dict.insert("timestamp".to_string(), timestamp.clone());
        }
        
        dict
    }

    /// Returns true if this ARK URL represents a project-level resource.
    pub fn is_project_level(&self) -> bool {
        self.resource_id.is_none()
    }

    /// Returns true if this ARK URL represents a resource-level (not value) resource.
    pub fn is_resource_level(&self) -> bool {
        self.resource_id.is_some() && self.value_id.is_none()
    }

    /// Returns true if this ARK URL represents a value-level resource.
    pub fn is_value_level(&self) -> bool {
        self.value_id.is_some()
    }

    /// Returns true if this ARK URL has a timestamp.
    pub fn has_timestamp(&self) -> bool {
        self.timestamp.is_some()
    }

    /// Returns true if this is a version 0 (legacy) ARK URL.
    pub fn is_version_0(&self) -> bool {
        self.url_version == 0
    }

    /// Returns true if this is a version 1 (current) ARK URL.
    pub fn is_version_1(&self) -> bool {
        self.url_version == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ark_url_info() {
        let info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            None,
        );

        assert_eq!(info.url_version, 1);
        assert_eq!(info.project_id, Some("0001".to_string()));
        assert_eq!(info.resource_id, Some("resource123".to_string()));
        assert_eq!(info.value_id, None);
        assert_eq!(info.timestamp, None);
    }

    #[test]
    fn test_get_timestamp_version_0() {
        let info = ArkUrlInfo::new(
            0,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            Some("20240101".to_string()),
        );

        assert_eq!(info.get_timestamp(), Some("20240101T000000Z".to_string()));
    }

    #[test]
    fn test_get_timestamp_version_1() {
        let info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            Some("20240101T123456Z".to_string()),
        );

        assert_eq!(info.get_timestamp(), Some("20240101T123456Z".to_string()));
    }

    #[test]
    fn test_get_timestamp_none() {
        let info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            None,
        );

        assert_eq!(info.get_timestamp(), None);
    }

    #[test]
    fn test_to_template_dict() {
        let info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            Some("value456".to_string()),
            Some("20240101T123456Z".to_string()),
        );

        let dict = info.to_template_dict();

        assert_eq!(dict.get("url_version"), Some(&"1".to_string()));
        assert_eq!(dict.get("project_id"), Some(&"0001".to_string()));
        assert_eq!(dict.get("resource_id"), Some(&"resource123".to_string()));
        assert_eq!(dict.get("value_id"), Some(&"value456".to_string()));
        assert_eq!(dict.get("timestamp"), Some(&"20240101T123456Z".to_string()));
    }

    #[test]
    fn test_level_detection() {
        let project_info = ArkUrlInfo::new(1, Some("0001".to_string()), None, None, None);
        assert!(project_info.is_project_level());
        assert!(!project_info.is_resource_level());
        assert!(!project_info.is_value_level());

        let resource_info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            None,
            None,
        );
        assert!(!resource_info.is_project_level());
        assert!(resource_info.is_resource_level());
        assert!(!resource_info.is_value_level());

        let value_info = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            Some("resource123".to_string()),
            Some("value456".to_string()),
            None,
        );
        assert!(!value_info.is_project_level());
        assert!(!value_info.is_resource_level());
        assert!(value_info.is_value_level());
    }

    #[test]
    fn test_version_detection() {
        let v0_info = ArkUrlInfo::new(0, Some("0001".to_string()), None, None, None);
        assert!(v0_info.is_version_0());
        assert!(!v0_info.is_version_1());

        let v1_info = ArkUrlInfo::new(1, Some("0001".to_string()), None, None, None);
        assert!(!v1_info.is_version_0());
        assert!(v1_info.is_version_1());
    }

    #[test]
    fn test_has_timestamp() {
        let with_timestamp = ArkUrlInfo::new(
            1,
            Some("0001".to_string()),
            None,
            None,
            Some("20240101T123456Z".to_string()),
        );
        assert!(with_timestamp.has_timestamp());

        let without_timestamp = ArkUrlInfo::new(1, Some("0001".to_string()), None, None, None);
        assert!(!without_timestamp.has_timestamp());
    }
}