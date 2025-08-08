use async_trait::async_trait;
use reqwest::Proxy;
use std::collections::HashMap;
use std::env;
use std::error::Error as StdError;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::net::lookup_host;
use url::Url;

use crate::adapters::common::IniProcessor;
use crate::core::domain::settings::SettingsRegistry;
use crate::core::errors::settings::{SettingsError, SettingsResult};
use crate::core::ports::settings::ConfigurationProvider;

/// HTTP adapter for fetching configuration files from URLs
pub struct HttpConfigurationProvider {
    client: reqwest::Client,
}

impl HttpConfigurationProvider {
    pub fn new() -> Self {
        // BR: Match Python requests timeout behavior (10s total)
        // BR: Use generous timeouts since this runs in background parallel execution
        let connect_timeout_ms = env::var("ARK_RUST_HTTP_CONNECT_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(5_000); // 5s connect timeout
        let total_timeout_ms = env::var("ARK_RUST_HTTP_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10_000); // 10s total timeout to match Python

        // BR: Support outbound proxies via environment
        let proxy_from_env = || -> Option<String> {
            let keys = [
                "ARK_OUTBOUND_PROXY",
                "HTTPS_PROXY",
                "https_proxy",
                "HTTP_PROXY",
                "http_proxy",
                "ALL_PROXY",
                "all_proxy",
            ];
            for key in keys {
                if let Ok(val) = env::var(key) {
                    if !val.trim().is_empty() {
                        return Some(val);
                    }
                }
            }
            None
        }();

        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(connect_timeout_ms))
            .timeout(Duration::from_millis(total_timeout_ms))
            // BR: Use standard User-Agent to match Python requests behavior
            .user_agent("ark-resolver/1.0 (Rust)")
            // BR: Follow redirects like Python requests (up to 10)
            .redirect(reqwest::redirect::Policy::limited(10))
            // BR: Use HTTP/1.1 to match Python requests behavior (avoid HTTP/2 issues)
            .http1_only()
            // BR: Enable TCP keepalive for better connection reuse
            .tcp_keepalive(Duration::from_secs(30))
            // BR: Disable connection pooling to force fresh connections (helps with Stage DNS issues)
            .pool_max_idle_per_host(0)
            // BR: Set longer read timeout for slow networks
            .read_timeout(Duration::from_millis(total_timeout_ms / 2));

        if let Some(proxy_url) = proxy_from_env {
            if let Ok(p) = Proxy::all(&proxy_url) {
                builder = builder.proxy(p);
            }
        }

        let client = builder.build().expect("Failed to create HTTP client");

        Self { client }
    }

    /// Check if a path is an HTTP URL
    pub fn is_http_url(path: &str) -> bool {
        path.starts_with("http://") || path.starts_with("https://")
    }

    /// Fetch content from HTTP URL
    async fn fetch_url_content(&self, url: &str) -> SettingsResult<String> {
        // BR: Provide rich diagnostics to help debug Stage TLS/network issues safely
        let start_time = Instant::now();

        // BR: Perform comprehensive pre-flight diagnostics for debugging
        let diagnostics = match NetworkDiagnostics::new(url).await {
            Ok(diag) => diag,
            Err(e) => {
                return Err(SettingsError::FileSystemError(format!(
                    "Pre-flight diagnostics failed for '{url}': {e}"
                )));
            }
        };

        let response = self.client.get(url).send().await.map_err(|e| {
            let elapsed = start_time.elapsed();
            let classification = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "connect"
            } else if e.is_request() {
                "request"
            } else if e.is_redirect() {
                "redirect"
            } else if e.is_body() {
                "body"
            } else if e.is_decode() {
                "decode"
            } else {
                "network"
            };

            let proxy_present = [
                "ARK_OUTBOUND_PROXY",
                "HTTPS_PROXY",
                "https_proxy",
                "HTTP_PROXY",
                "http_proxy",
                "ALL_PROXY",
                "all_proxy",
            ]
            .iter()
            .any(|k| env::var(k).ok().filter(|v| !v.trim().is_empty()).is_some());

            let connect_timeout = env::var("ARK_RUST_HTTP_CONNECT_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(5_000);
            let total_timeout = env::var("ARK_RUST_HTTP_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(10_000);

            // BR: Create comprehensive error message with diagnostics
            let mut msg = create_detailed_error_message(ErrorContext {
                url,
                diagnostics: &diagnostics,
                classification,
                elapsed,
                connect_timeout,
                total_timeout,
                error: &e,
                proxy_present,
            });

            if let Some(src) = e.source() {
                // Include one level of source error for deeper insight
                msg.push_str(&format!("; source: {src}"));
            }

            SettingsError::FileSystemError(msg)
        })?;

        let final_url = response.url().to_string();
        let status = response.status();

        if !status.is_success() {
            // Try to extract a short snippet of the body for additional context
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unavailable>".to_string());
            let snippet = if body_text.len() > 256 {
                format!("{}…", &body_text[..256])
            } else {
                body_text
            };
            return Err(SettingsError::FileSystemError(format!(
                "HTTP request failed for '{final_url}': status {status}; body: {snippet}"
            )));
        }

        let content = response.text().await.map_err(|e| {
            SettingsError::FileSystemError(format!(
                "Failed to read response from '{final_url}': {e}"
            ))
        })?;

        Ok(content)
    }
}

/// Network diagnostics information for debugging connection issues
#[derive(Debug, Clone)]
struct NetworkDiagnostics {
    host: String,
    port: u16,
    scheme: String,
    dns_resolution_time: Duration,
    resolved_ips: Vec<IpAddr>,
    dns_success: bool,
    container_context: ContainerContext,
}

/// Container and environment context for debugging
#[derive(Debug, Clone)]
struct ContainerContext {
    is_container: bool,
    hostname: Option<String>,
    relevant_env_vars: HashMap<String, String>,
}

impl NetworkDiagnostics {
    /// Create comprehensive network diagnostics for a URL
    async fn new(url: &str) -> Result<Self, String> {
        let start_time = Instant::now();

        // Parse URL
        let parsed = Url::parse(url).map_err(|e| format!("URL parse error: {e}"))?;
        let host = parsed.host_str().unwrap_or("unknown").to_string();
        let port = parsed.port_or_known_default().unwrap_or(80);
        let scheme = parsed.scheme().to_string();

        // Attempt DNS resolution
        let (resolved_ips, dns_success) = match lookup_host(&format!("{host}:{port}")).await {
            Ok(addrs) => {
                let ips: Vec<IpAddr> = addrs.map(|addr| addr.ip()).collect();
                (ips, true)
            }
            Err(_) => (Vec::new(), false),
        };

        let dns_resolution_time = start_time.elapsed();

        // Gather container context
        let container_context = ContainerContext::detect();

        Ok(Self {
            host,
            port,
            scheme,
            dns_resolution_time,
            resolved_ips,
            dns_success,
            container_context,
        })
    }
}

impl ContainerContext {
    /// Detect if running in container and gather relevant context
    fn detect() -> Self {
        let mut relevant_env_vars = HashMap::new();

        // Container detection environment variables
        let container_indicators = [
            "CONTAINER",
            "DOCKER",
            "KUBERNETES_SERVICE_HOST",
            "K8S_NODE_NAME",
            "ARK_REGISTRY",
            "ARK_RUST_HTTP_CONNECT_TIMEOUT_MS",
            "ARK_RUST_HTTP_TIMEOUT_MS",
        ];

        for var in &container_indicators {
            if let Ok(val) = env::var(var) {
                relevant_env_vars.insert(var.to_string(), val);
            }
        }

        // Check for container indicators
        let is_container = std::fs::read_to_string("/proc/1/cgroup")
            .map(|content| content.contains("docker") || content.contains("containerd"))
            .unwrap_or(false)
            || env::var("CONTAINER").is_ok()
            || env::var("KUBERNETES_SERVICE_HOST").is_ok();

        let hostname = env::var("HOSTNAME").ok();

        Self {
            is_container,
            hostname,
            relevant_env_vars,
        }
    }
}

/// Error context for creating detailed messages
struct ErrorContext<'a> {
    url: &'a str,
    diagnostics: &'a NetworkDiagnostics,
    classification: &'a str,
    elapsed: Duration,
    connect_timeout: u64,
    total_timeout: u64,
    error: &'a reqwest::Error,
    proxy_present: bool,
}

/// Create detailed error message with comprehensive diagnostics
fn create_detailed_error_message(ctx: ErrorContext) -> String {
    let mut parts = Vec::new();

    // Basic error info
    parts.push(format!("Failed to fetch URL '{}'", ctx.url));

    // Host and connection details
    parts.push(format!(
        "host: {} port: {} scheme: {}",
        ctx.diagnostics.host, ctx.diagnostics.port, ctx.diagnostics.scheme
    ));

    // DNS resolution info
    if ctx.diagnostics.dns_success {
        let ips: Vec<String> = ctx
            .diagnostics
            .resolved_ips
            .iter()
            .map(|ip| ip.to_string())
            .collect();
        parts.push(format!(
            "dns: resolved in {:?} to [{}]",
            ctx.diagnostics.dns_resolution_time,
            ips.join(", ")
        ));
    } else {
        parts.push(format!(
            "dns: FAILED to resolve in {:?}",
            ctx.diagnostics.dns_resolution_time
        ));
    }

    // Error classification and timing
    parts.push(format!(
        "class: {classification} tls: rustls elapsed: {elapsed:?}",
        classification = ctx.classification,
        elapsed = ctx.elapsed
    ));

    // Timeout configuration
    parts.push(format!(
        "timeouts: connect={connect_timeout}ms total={total_timeout}ms",
        connect_timeout = ctx.connect_timeout,
        total_timeout = ctx.total_timeout
    ));

    // Proxy info
    if ctx.proxy_present {
        parts.push("proxy: detected".to_string());
    }

    // Container context
    if ctx.diagnostics.container_context.is_container {
        let mut container_info = vec!["container: true".to_string()];
        if let Some(hostname) = &ctx.diagnostics.container_context.hostname {
            container_info.push(format!("hostname: {hostname}"));
        }
        parts.push(container_info.join(" "));
    }

    // Environment context
    if !ctx
        .diagnostics
        .container_context
        .relevant_env_vars
        .is_empty()
    {
        let env_info: Vec<String> = ctx
            .diagnostics
            .container_context
            .relevant_env_vars
            .iter()
            .map(|(k, v)| {
                // Truncate long values for readability
                let display_val = if v.len() > 50 {
                    format!("{}...", &v[..47])
                } else {
                    v.clone()
                };
                format!("{k}={display_val}")
            })
            .collect();
        parts.push(format!("env: [{}]", env_info.join(", ")));
    }

    format!("{}): {}", parts.join(" ("), ctx.error)
}

#[async_trait]
impl ConfigurationProvider for HttpConfigurationProvider {
    async fn load_registry(&self, registry_url: &str) -> SettingsResult<SettingsRegistry> {
        let content = self.fetch_url_content(registry_url).await?;
        let raw_data = IniProcessor::parse_ini_from_content(&content, registry_url)?;
        Ok(IniProcessor::create_settings_registry(&raw_data))
    }

    async fn load_config(&self, config_url: &str) -> SettingsResult<HashMap<String, String>> {
        let content = self.fetch_url_content(config_url).await?;
        let raw_data = IniProcessor::parse_ini_from_content(&content, config_url)?;
        Ok(IniProcessor::flatten_config_sections(&raw_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_http_url() {
        assert!(HttpConfigurationProvider::is_http_url("http://example.com"));
        assert!(HttpConfigurationProvider::is_http_url(
            "https://example.com"
        ));
        assert!(!HttpConfigurationProvider::is_http_url("/path/to/file"));
        assert!(!HttpConfigurationProvider::is_http_url("file://path"));
        assert!(!HttpConfigurationProvider::is_http_url("ftp://example.com"));
    }

    // Integration tests with actual HTTP requests require a test server
    // and are better suited for integration test files rather than unit tests
}
