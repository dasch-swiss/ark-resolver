//! Environment variables logging module
//!
//! This module provides comprehensive environment variable logging with security masking,
//! categorization, and operational visibility. It's designed to work in both pure Rust
//! and Python contexts through PyO3 bindings.

use std::collections::HashMap;
use std::env;

/// Environment variable configuration definition
#[derive(Debug, Clone)]
pub struct EnvVarDefinition {
    pub name: &'static str,
    pub default: Option<&'static str>,
    pub secret: bool,
    pub required: bool,
    pub category: Category,
    #[allow(dead_code)]
    pub description: &'static str,
    pub truncate_long_values: bool,
}

/// Categories for grouping environment variables
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Core,
    RustHttp,
    Tracing,
    Proxy,
    Security,
    Container,
}

impl Category {
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Core => "Core Service Configuration",
            Category::RustHttp => "Rust HTTP Client Configuration",
            Category::Tracing => "Tracing Configuration",
            Category::Proxy => "Proxy Configuration (optional)",
            Category::Security => "Security Configuration",
            Category::Container => "Container Environment",
        }
    }
}

/// Statistics tracking for environment variables
#[derive(Debug, Default)]
pub struct EnvironmentStats {
    pub total_variables: usize,
    pub variables_set: usize,
    pub secrets_set: usize,
}

impl EnvironmentStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_total(&mut self) {
        self.total_variables += 1;
    }

    pub fn increment_set(&mut self) {
        self.variables_set += 1;
    }

    pub fn increment_secret(&mut self) {
        self.secrets_set += 1;
    }
}

/// Get all environment variable definitions
pub fn get_env_var_definitions() -> Vec<EnvVarDefinition> {
    vec![
        // Core Service Configuration
        EnvVarDefinition {
            name: "ARK_EXTERNAL_HOST",
            default: Some("ark.example.org"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "External hostname used in ARK URLs",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_INTERNAL_HOST",
            default: Some("0.0.0.0"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "Internal hostname for the server",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_INTERNAL_PORT",
            default: Some("3336"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "Port for the server to bind to",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_NAAN",
            default: Some("00000"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "Name Assigning Authority Number",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_HTTPS_PROXY",
            default: Some("true"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "Whether behind HTTPS proxy",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_REGISTRY",
            default: None,
            secret: false,
            required: true,
            category: Category::Core,
            description: "Path or URL to the project registry file",
            truncate_long_values: true,
        },
        // Security Configuration
        EnvVarDefinition {
            name: "ARK_GITHUB_SECRET",
            default: None,
            secret: true,
            required: false,
            category: Category::Security,
            description: "Secret for GitHub webhook authentication",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_SENTRY_DSN",
            default: None,
            secret: true,
            required: false,
            category: Category::Security,
            description: "Sentry DSN for error tracking",
            truncate_long_values: true,
        },
        // Rust HTTP Client Configuration
        EnvVarDefinition {
            name: "ARK_RUST_LOAD_TIMEOUT_MS",
            default: Some("15000"),
            secret: false,
            required: false,
            category: Category::RustHttp,
            description: "Application-level timeout for settings loading",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_RUST_HTTP_TIMEOUT_MS",
            default: Some("10000"),
            secret: false,
            required: false,
            category: Category::RustHttp,
            description: "HTTP request total timeout",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_RUST_HTTP_CONNECT_TIMEOUT_MS",
            default: Some("5000"),
            secret: false,
            required: false,
            category: Category::RustHttp,
            description: "HTTP connection timeout",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "ARK_RUST_FORCE_IPV4",
            default: Some("false"),
            secret: false,
            required: false,
            category: Category::RustHttp,
            description: "Force IPv4-only connections, disable IPv6",
            truncate_long_values: false,
        },
        // Tracing Configuration
        EnvVarDefinition {
            name: "RUST_LOG",
            default: None,
            secret: false,
            required: false,
            category: Category::Tracing,
            description: "Controls tracing verbosity",
            truncate_long_values: false,
        },
        // Proxy Configuration (dynamic discovery)
        EnvVarDefinition {
            name: "ARK_OUTBOUND_PROXY",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "Outbound proxy configuration",
            truncate_long_values: true,
        },
        EnvVarDefinition {
            name: "HTTPS_PROXY",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "HTTPS proxy configuration",
            truncate_long_values: true,
        },
        EnvVarDefinition {
            name: "https_proxy",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "HTTPS proxy configuration (lowercase)",
            truncate_long_values: true,
        },
        EnvVarDefinition {
            name: "HTTP_PROXY",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "HTTP proxy configuration",
            truncate_long_values: true,
        },
        EnvVarDefinition {
            name: "http_proxy",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "HTTP proxy configuration (lowercase)",
            truncate_long_values: true,
        },
        EnvVarDefinition {
            name: "ALL_PROXY",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "All protocols proxy configuration",
            truncate_long_values: true,
        },
        EnvVarDefinition {
            name: "all_proxy",
            default: None,
            secret: true,
            required: false,
            category: Category::Proxy,
            description: "All protocols proxy configuration (lowercase)",
            truncate_long_values: true,
        },
        // Container Environment
        EnvVarDefinition {
            name: "CONTAINER",
            default: None,
            secret: false,
            required: false,
            category: Category::Container,
            description: "Container environment indicator",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "KUBERNETES_SERVICE_HOST",
            default: None,
            secret: false,
            required: false,
            category: Category::Container,
            description: "Kubernetes service host",
            truncate_long_values: false,
        },
        EnvVarDefinition {
            name: "HOSTNAME",
            default: None,
            secret: false,
            required: false,
            category: Category::Container,
            description: "Container hostname",
            truncate_long_values: false,
        },
    ]
}

/// Check if any proxy environment variables are set
pub fn has_any_proxy_vars(proxy_vars: &[&EnvVarDefinition]) -> bool {
    proxy_vars.iter().any(|var| env::var(var.name).is_ok())
}

/// Format a single environment variable log line with proper handling
pub fn format_env_var_log_line(var_def: &EnvVarDefinition, stats: &mut EnvironmentStats) -> String {
    stats.increment_total();

    match env::var(var_def.name) {
        Ok(value) => {
            stats.increment_set();
            if var_def.secret {
                stats.increment_secret();
                if value.is_empty() {
                    format!("├─ {}: [empty] (set)", var_def.name)
                } else {
                    format!("├─ {}: [MASKED] (set)", var_def.name)
                }
            } else {
                let display_value = if var_def.truncate_long_values && value.len() > 80 {
                    format!("{}...", &value[..77])
                } else {
                    value
                };
                format!("├─ {}: \"{}\" (set)", var_def.name, display_value)
            }
        }
        Err(_) => {
            if let Some(default_val) = var_def.default {
                format!("├─ {}: \"{}\" (default)", var_def.name, default_val)
            } else if var_def.required {
                format!("├─ {}: [REQUIRED - NOT SET]", var_def.name)
            } else {
                format!("├─ {}: [not set]", var_def.name)
            }
        }
    }
}

/// Log environment variables using Rust's tracing system
/// This is the pure Rust interface for environment logging
#[allow(dead_code)]
pub fn log_environment_variables_rust() -> Result<EnvironmentStats, String> {
    let definitions = get_env_var_definitions();
    let mut log_messages = Vec::new();
    let mut stats = EnvironmentStats::new();

    // Group by category
    let mut by_category: HashMap<Category, Vec<&EnvVarDefinition>> = HashMap::new();
    for def in &definitions {
        by_category
            .entry(def.category.clone())
            .or_default()
            .push(def);
    }

    log_messages.push("Environment Configuration:".to_string());

    // Process each category
    let category_order = [
        Category::Core,
        Category::Security,
        Category::RustHttp,
        Category::Tracing,
        Category::Proxy,
        Category::Container,
    ];

    for category in &category_order {
        if let Some(vars) = by_category.get(category) {
            if vars.is_empty() || (*category == Category::Proxy && !has_any_proxy_vars(vars)) {
                continue;
            }

            log_messages.push(format!("┌─ {}", category.display_name()));

            for var_def in vars {
                let log_line = format_env_var_log_line(var_def, &mut stats);
                if !log_line.is_empty() {
                    log_messages.push(log_line);
                }
            }
        }
    }

    // Add summary statistics
    log_messages.push(format!(
        "┌─ Summary: {}/{} variables configured, {} secrets set",
        stats.variables_set, stats.total_variables, stats.secrets_set
    ));

    // Log all messages using Rust tracing
    for message in &log_messages {
        tracing::info!("{}", message);
    }

    Ok(stats)
}

#[cfg(feature = "pyo3")]
mod pyo3_bindings {
    use super::*;
    use pyo3::prelude::*;

    /// Log all relevant environment variables with their values and defaults
    /// This is the PyO3 wrapper that provides dual logging (Rust tracing + Python logging)
    #[pyfunction]
    pub fn log_environment_variables_python(py: Python<'_>) -> PyResult<()> {
        // Get Python's logging module for batched output
        let logging = py.import("logging")?;
        let logger = logging.getattr("getLogger")?.call1(("ark_resolver",))?;

        let definitions = get_env_var_definitions();
        let mut log_messages = Vec::new();
        let mut stats = EnvironmentStats::new();

        // Group by category
        let mut by_category: HashMap<Category, Vec<&EnvVarDefinition>> = HashMap::new();
        for def in &definitions {
            by_category
                .entry(def.category.clone())
                .or_default()
                .push(def);
        }

        log_messages.push("Environment Configuration:".to_string());

        // Process each category
        let category_order = [
            Category::Core,
            Category::Security,
            Category::RustHttp,
            Category::Tracing,
            Category::Proxy,
            Category::Container,
        ];

        for category in &category_order {
            if let Some(vars) = by_category.get(category) {
                if vars.is_empty() || (*category == Category::Proxy && !has_any_proxy_vars(vars)) {
                    continue;
                }

                log_messages.push(format!("┌─ {}", category.display_name()));

                for var_def in vars {
                    let log_line = format_env_var_log_line(var_def, &mut stats);
                    if !log_line.is_empty() {
                        log_messages.push(log_line);
                    }
                }
            }
        }

        // Add summary statistics
        log_messages.push(format!(
            "┌─ Summary: {}/{} variables configured, {} secrets set",
            stats.variables_set, stats.total_variables, stats.secrets_set
        ));

        // Batch log all messages to both Rust tracing and Python logging system
        for message in &log_messages {
            tracing::info!("{}", message);
            logger.call_method1("info", (message,))?;
        }

        Ok(())
    }
}

#[cfg(feature = "pyo3")]
pub use pyo3_bindings::log_environment_variables_python;

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_env_var_definition_creation() {
        let def = EnvVarDefinition {
            name: "TEST_VAR",
            default: Some("default_value"),
            secret: false,
            required: true,
            category: Category::Core,
            description: "Test variable",
            truncate_long_values: false,
        };

        assert_eq!(def.name, "TEST_VAR");
        assert_eq!(def.default, Some("default_value"));
        assert!(!def.secret);
        assert!(def.required);
        assert_eq!(def.category, Category::Core);
    }

    #[test]
    fn test_category_display_names() {
        assert_eq!(Category::Core.display_name(), "Core Service Configuration");
        assert_eq!(Category::Security.display_name(), "Security Configuration");
        assert_eq!(
            Category::RustHttp.display_name(),
            "Rust HTTP Client Configuration"
        );
        assert_eq!(Category::Tracing.display_name(), "Tracing Configuration");
        assert_eq!(
            Category::Proxy.display_name(),
            "Proxy Configuration (optional)"
        );
        assert_eq!(Category::Container.display_name(), "Container Environment");
    }

    #[test]
    fn test_environment_stats() {
        let mut stats = EnvironmentStats::new();
        assert_eq!(stats.total_variables, 0);
        assert_eq!(stats.variables_set, 0);
        assert_eq!(stats.secrets_set, 0);

        stats.increment_total();
        stats.increment_set();
        stats.increment_secret();

        assert_eq!(stats.total_variables, 1);
        assert_eq!(stats.variables_set, 1);
        assert_eq!(stats.secrets_set, 1);
    }

    #[test]
    fn test_get_env_var_definitions() {
        let definitions = get_env_var_definitions();

        // Should have all the expected environment variables
        assert!(!definitions.is_empty());

        // Check that ARK_REGISTRY is marked as required
        let ark_registry = definitions
            .iter()
            .find(|def| def.name == "ARK_REGISTRY")
            .unwrap();
        assert!(ark_registry.required);
        assert_eq!(ark_registry.category, Category::Core);

        // Check that secrets are properly marked
        let github_secret = definitions
            .iter()
            .find(|def| def.name == "ARK_GITHUB_SECRET")
            .unwrap();
        assert!(github_secret.secret);
        assert_eq!(github_secret.category, Category::Security);
    }

    #[test]
    #[serial]
    fn test_format_env_var_log_line_set_variable() {
        env::set_var("TEST_FORMAT_VAR", "test_value");

        let var_def = EnvVarDefinition {
            name: "TEST_FORMAT_VAR",
            default: Some("default"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "Test variable",
            truncate_long_values: false,
        };

        let mut stats = EnvironmentStats::new();
        let line = format_env_var_log_line(&var_def, &mut stats);

        assert_eq!(line, "├─ TEST_FORMAT_VAR: \"test_value\" (set)");
        assert_eq!(stats.total_variables, 1);
        assert_eq!(stats.variables_set, 1);
        assert_eq!(stats.secrets_set, 0);

        env::remove_var("TEST_FORMAT_VAR");
    }

    #[test]
    #[serial]
    fn test_format_env_var_log_line_secret_variable() {
        env::set_var("TEST_SECRET_VAR", "secret_value");

        let var_def = EnvVarDefinition {
            name: "TEST_SECRET_VAR",
            default: None,
            secret: true,
            required: false,
            category: Category::Security,
            description: "Test secret",
            truncate_long_values: false,
        };

        let mut stats = EnvironmentStats::new();
        let line = format_env_var_log_line(&var_def, &mut stats);

        assert_eq!(line, "├─ TEST_SECRET_VAR: [MASKED] (set)");
        assert_eq!(stats.total_variables, 1);
        assert_eq!(stats.variables_set, 1);
        assert_eq!(stats.secrets_set, 1);

        env::remove_var("TEST_SECRET_VAR");
    }

    #[test]
    fn test_format_env_var_log_line_default_value() {
        env::remove_var("TEST_DEFAULT_VAR");

        let var_def = EnvVarDefinition {
            name: "TEST_DEFAULT_VAR",
            default: Some("default_value"),
            secret: false,
            required: false,
            category: Category::Core,
            description: "Test variable",
            truncate_long_values: false,
        };

        let mut stats = EnvironmentStats::new();
        let line = format_env_var_log_line(&var_def, &mut stats);

        assert_eq!(line, "├─ TEST_DEFAULT_VAR: \"default_value\" (default)");
        assert_eq!(stats.total_variables, 1);
        assert_eq!(stats.variables_set, 0);
        assert_eq!(stats.secrets_set, 0);
    }

    #[test]
    fn test_format_env_var_log_line_required_not_set() {
        env::remove_var("TEST_REQUIRED_VAR");

        let var_def = EnvVarDefinition {
            name: "TEST_REQUIRED_VAR",
            default: None,
            secret: false,
            required: true,
            category: Category::Core,
            description: "Test required variable",
            truncate_long_values: false,
        };

        let mut stats = EnvironmentStats::new();
        let line = format_env_var_log_line(&var_def, &mut stats);

        assert_eq!(line, "├─ TEST_REQUIRED_VAR: [REQUIRED - NOT SET]");
        assert_eq!(stats.total_variables, 1);
        assert_eq!(stats.variables_set, 0);
        assert_eq!(stats.secrets_set, 0);
    }

    #[test]
    #[serial]
    fn test_format_env_var_log_line_truncation() {
        let long_value = "a".repeat(100);
        env::set_var("TEST_LONG_VAR", &long_value);

        let var_def = EnvVarDefinition {
            name: "TEST_LONG_VAR",
            default: None,
            secret: false,
            required: false,
            category: Category::Core,
            description: "Test long variable",
            truncate_long_values: true,
        };

        let mut stats = EnvironmentStats::new();
        let line = format_env_var_log_line(&var_def, &mut stats);

        // Should truncate to 77 chars + "..."
        let expected = format!("├─ TEST_LONG_VAR: \"{}...\" (set)", "a".repeat(77));
        assert_eq!(line, expected);

        env::remove_var("TEST_LONG_VAR");
    }

    #[test]
    #[serial]
    fn test_has_any_proxy_vars() {
        // Remove any existing proxy vars
        let proxy_names = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"];
        for name in &proxy_names {
            env::remove_var(name);
        }

        let definitions = get_env_var_definitions();
        let proxy_definitions: Vec<_> = definitions
            .iter()
            .filter(|def| def.category == Category::Proxy)
            .collect();

        // Should return false when no proxy vars are set
        assert!(!has_any_proxy_vars(&proxy_definitions));

        // Set one proxy var
        env::set_var("HTTPS_PROXY", "proxy.example.com");

        // Should return true when at least one proxy var is set
        assert!(has_any_proxy_vars(&proxy_definitions));

        env::remove_var("HTTPS_PROXY");
    }
}
