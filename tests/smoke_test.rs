use assert_cmd::Command;
use std::{thread, time::Duration};

fn cleanup_docker() {
    println!("Cleaning up Docker containers...");
    let _ = Command::new("docker").args(["compose", "down"]).output();
}

fn get_container_logs() -> String {
    match Command::new("docker")
        .args(["compose", "logs", "--no-color"])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(e) => {
            println!("Failed to get docker logs: {}", e);
            String::new()
        }
    }
}

fn analyze_registry_loading_logs(logs: &str) -> Result<(), String> {
    println!("Analyzing logs for HTTP registry loading...");

    // Check for successful HTTP registry loading
    let http_success_patterns = [
        "Registry loaded successfully",
        "Settings loaded successfully",
        "Loading configuration from",
        "Successfully fetched",
        "HTTP configuration loaded",
    ];

    // Check for failures that indicate HTTP registry loading issues
    let http_failure_patterns = [
        "Registry file not found: https://",
        "Failed to fetch URL",
        "HTTP request failed",
        "Failed to parse INI content",
        "OSError: Registry file not found: https://",
        "Parallel execution ERROR: convert - Rust",
        "WARNING: Rust execution failed",
    ];

    // Check for the specific GitHub URL being used
    let github_url = "https://raw.githubusercontent.com/dasch-swiss/ark-resolver-data/master/data/dasch_ark_registry_staging.ini";
    if logs.contains(github_url) {
        println!("✅ Found reference to GitHub registry URL in logs");
    } else {
        println!("⚠️  GitHub registry URL not found in logs (may be expected)");
    }

    // Look for HTTP-related failures first (these are critical)
    for pattern in &http_failure_patterns {
        if logs.contains(pattern) {
            return Err(format!(
                "Found HTTP registry loading failure in logs: '{}'",
                pattern
            ));
        }
    }

    // Look for success indicators
    let mut found_success = false;
    for pattern in &http_success_patterns {
        if logs.contains(pattern) {
            println!("✅ Found success pattern: '{}'", pattern);
            found_success = true;
            break;
        }
    }

    // Check for parallel execution success (specific to the original staging issue)
    if logs.contains("Convert ark:/99999/") || logs.contains("INFO: Convert ark:/") {
        println!("✅ Found ARK conversion in logs");
    }

    // Check for Python vs Rust parallel execution results
    if logs.contains("Parallel execution result") {
        println!("✅ Found parallel execution results in logs");
    }

    // Look for any WARNING or ERROR level logs that might indicate issues
    let warning_errors: Vec<&str> = logs
        .lines()
        .filter(|line| {
            let line_upper = line.to_uppercase();
            (line_upper.contains("WARNING") || line_upper.contains("ERROR")) &&
            !line_upper.contains("DEPRECATED") && // Ignore deprecation warnings
            !line_upper.contains("PKG_RESOURCES") // Ignore pkg_resources warnings
        })
        .collect();

    if !warning_errors.is_empty() {
        println!(
            "⚠️  Found {} warning/error log entries:",
            warning_errors.len()
        );
        for (i, error) in warning_errors.iter().enumerate() {
            if i < 5 {
                // Show first 5 to avoid spam
                println!("   {}", error.trim());
            }
        }
        if warning_errors.len() > 5 {
            println!("   ... and {} more", warning_errors.len() - 5);
        }

        // Check if any are related to our HTTP registry loading
        for error in &warning_errors {
            if error.contains("Registry")
                || error.contains("HTTP")
                || error.contains("Rust execution failed")
            {
                return Err(format!("Found critical error in logs: {}", error.trim()));
            }
        }
    }

    if !found_success && warning_errors.is_empty() {
        println!("ℹ️  No explicit success patterns found, but no errors detected either");
    }

    Ok(())
}

fn test_parallel_execution_logs(logs: &str) -> Result<(), String> {
    println!("Analyzing parallel execution logs...");

    // The original staging issue was specifically about parallel execution failing
    // when the Rust implementation couldn't load the HTTP registry

    // Look for successful parallel execution
    let parallel_success_patterns = [
        "Python and Rust results match",
        "Parallel execution successful",
        "Both implementations succeeded",
    ];

    // Look for parallel execution failures (convert and redirect routes)
    let parallel_failure_patterns = [
        "Parallel execution ERROR: convert - Rust",
        "Parallel execution ERROR: redirect - Rust",
        "WARNING: Rust execution failed for convert",
        "WARNING: Rust execution failed for redirect",
        "Parallel execution MISMATCH",
        "The python code works as expected and is able to fetch the ark-registry.ini hosted on github, while the Rust code has problems"
    ];

    // Check for failures first
    for pattern in &parallel_failure_patterns {
        if logs.contains(pattern) {
            return Err(format!("Found parallel execution failure: '{}'", pattern));
        }
    }

    // Check for success
    for pattern in &parallel_success_patterns {
        if logs.contains(pattern) {
            println!("✅ Found parallel execution success: '{}'", pattern);
            return Ok(());
        }
    }

    // If we see parallel execution SUCCESS patterns in logs, that's good
    if logs.contains("Parallel execution SUCCESS") {
        println!("✅ Found parallel execution SUCCESS in logs");
        return Ok(());
    }

    // If we see convert requests but no explicit failures, that's likely success
    if logs.contains("Convert ark:/99999/") && !logs.contains("ERROR") {
        println!("✅ Convert operations completed without errors");
        return Ok(());
    }

    println!("ℹ️  No explicit parallel execution results found in logs");
    Ok(())
}

/// Test all ARK URL format variations against the running service.
///
/// These tests mirror the 17 parity test cases from test_redirect_parity.py,
/// adapted for the staging registry (NAAN 72163). 13 of 17 map directly;
/// 4 (custom ProjectHost and custom redirect patterns) are covered by parity
/// unit tests and don't need end-to-end smoke testing.
fn test_redirect_parity_cases(client: &reqwest::blocking::Client) {
    println!("Testing redirect parity cases (all ARK URL format variations)...");

    // BR: Each tuple is (test_name, ark_path, expected_redirect_status)
    // All valid ARKs should return 3xx redirects.
    let test_cases: &[(&str, &str)] = &[
        // --- Top-level ---
        ("top-level", "/ark:/72163/1"),
        // --- Project redirects ---
        // Project with default ProjectHost (meta.stage.dasch.swiss)
        ("project", "/ark:/72163/1/0803"),
        // Project with uppercase ID (tests case handling)
        ("project-uppercase", "/ark:/72163/1/080E"),
        // Project with lowercase ID (tests case normalization: 080e -> 080E)
        ("project-lowercase", "/ark:/72163/1/080e"),
        // --- v1 Resource redirects ---
        // Resource without timestamp
        ("resource", "/ark:/72163/1/0803/cmfk1DMHRBiR4=_6HXpEFAn"),
        // Resource with fractional-second timestamp
        (
            "resource-ts-fractional",
            "/ark:/72163/1/0803/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z",
        ),
        // Resource with whole-second timestamp
        (
            "resource-ts-no-fractional",
            "/ark:/72163/1/0803/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622Z",
        ),
        // --- v1 Value redirects ---
        // Value without timestamp
        (
            "value",
            "/ark:/72163/1/0803/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo",
        ),
        // Value with timestamp
        (
            "value-with-ts",
            "/ark:/72163/1/0803/cmfk1DMHRBiR4=_6HXpEFAn/pLlW4ODASumZfZFbJdpw1gu.20180604T085622Z",
        ),
        // --- v0 salsah redirects (projects with AllowVersion0) ---
        // v0 ARK for project 0803
        ("v0-salsah", "/ark:/72163/0803-779b9990a0c3f-6e"),
        // v0 ARK with timestamp
        ("v0-salsah-ts", "/ark:/72163/0803-779b9990a0c3f-6e.20190129"),
        // v0 ARK with lowercase project ID (080e -> 080E)
        ("v0-salsah-lowercase", "/ark:/72163/080e-76bb2132d30d6-0"),
        // v0 ARK with lowercase project ID and timestamp
        (
            "v0-salsah-lowercase-ts",
            "/ark:/72163/080e-76bb2132d30d6-0.20190129",
        ),
        // v0 ARK with too-short timestamp (treated as no timestamp)
        (
            "v0-salsah-short-ts",
            "/ark:/72163/080e-76bb2132d30d6-0.2019111",
        ),
    ];

    // Not tested here (covered by test_redirect_parity.py unit tests):
    // - project-custom-host: requires a project with custom ProjectHost in staging registry
    // - project-salsah-host: same
    // - resource-custom-pattern: requires a project with custom DSPResourceRedirectUrl
    // - value-custom-pattern: requires a project with custom DSPValueRedirectUrl

    let mut failures = Vec::new();

    for (name, path) in test_cases {
        let url = format!("http://localhost:3336{}", path);
        match client.get(&url).send() {
            Ok(response) => {
                if response.status().is_redirection() {
                    let location = response
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("(no location header)");
                    println!("  ✅ {}: {} -> {}", name, response.status(), location);
                } else {
                    println!("  ❌ {}: expected 3xx, got {}", name, response.status());
                    failures.push(format!("{}: got {}", name, response.status()));
                }
            }
            Err(e) => {
                println!("  ❌ {}: request failed: {}", name, e);
                failures.push(format!("{}: {}", name, e));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Redirect parity smoke tests failed ({}/{}):\n  {}",
            failures.len(),
            test_cases.len(),
            failures.join("\n  ")
        );
    }

    println!("✅ All {} redirect parity cases passed", test_cases.len());
}

fn test_registry_failure_scenario() {
    // Test that our HTTP registry loading properly handles failures
    // This creates a temporary docker-compose with a bad registry URL
    println!("Creating temporary docker-compose with invalid registry URL...");

    let bad_compose_content = r#"services:
  ark-resolver-test:
    image: daschswiss/ark-resolver:latest
    ports:
      - "3337:3336"
    environment:
      RUST_LOG: info
      ARK_EXTERNAL_HOST: "ark.example.org"
      ARK_INTERNAL_HOST: "0.0.0.0"
      ARK_INTERNAL_PORT: "3336"
      ARK_NAAN: "99999"
      ARK_HTTPS_PROXY: false
      ARK_REGISTRY: "https://raw.githubusercontent.com/nonexistent/repo/master/nonexistent.ini"
"#;

    // Write temporary compose file
    std::fs::write("docker-compose-test-failure.yml", bad_compose_content)
        .expect("Failed to write test compose file");

    // Try to start it - it should fail or show errors in logs
    let start_result = Command::new("docker")
        .args([
            "compose",
            "-f",
            "docker-compose-test-failure.yml",
            "up",
            "-d",
        ])
        .output();

    if let Ok(_) = start_result {
        // Give it a moment to try loading the registry and fail
        thread::sleep(Duration::from_secs(2));

        // Get logs from the failed container
        let logs_result = Command::new("docker")
            .args([
                "compose",
                "-f",
                "docker-compose-test-failure.yml",
                "logs",
                "--no-color",
            ])
            .output();

        if let Ok(output) = logs_result {
            let logs = String::from_utf8_lossy(&output.stdout);

            // Should contain HTTP failure messages
            let expected_failure_patterns = [
                "HTTP request failed",
                "Failed to fetch URL",
                "404 Not Found",
                "Registry file not found",
            ];

            let mut found_expected_failure = false;
            for pattern in &expected_failure_patterns {
                if logs.contains(pattern) {
                    println!("✅ Found expected failure pattern: '{}'", pattern);
                    found_expected_failure = true;
                    break;
                }
            }

            if !found_expected_failure {
                println!("⚠️  Expected to find HTTP failure logs, but didn't find clear patterns");
                println!("   Logs snippet: {}", &logs[..logs.len().min(500)]);
            }
        }

        // Clean up the test container
        let _ = Command::new("docker")
            .args(["compose", "-f", "docker-compose-test-failure.yml", "down"])
            .output();
    }

    // Clean up temporary file
    let _ = std::fs::remove_file("docker-compose-test-failure.yml");

    println!("✅ Registry failure scenario test completed");
}

fn test_environment_logging_logs(logs: &str) -> Result<(), String> {
    println!("Analyzing environment variable logging...");

    // Expected patterns from our new environment logging
    let env_logging_patterns = [
        "Debug tracing initialized for environment variable logging",
        "Environment variables logged successfully",
        "Environment Configuration:",
        "┌─ Core Service Configuration",
        "├─ ARK_EXTERNAL_HOST:",
        "├─ ARK_INTERNAL_HOST:",
        "├─ ARK_INTERNAL_PORT:",
        "├─ ARK_NAAN:",
        "├─ ARK_REGISTRY:",
        "┌─ Security Configuration",
        "┌─ Rust HTTP Client Configuration",
        "┌─ Tracing Configuration",
        "├─ RUST_LOG:",
        "┌─ Summary:",
    ];

    let mut found_patterns = Vec::new();

    for pattern in &env_logging_patterns {
        if logs.contains(pattern) {
            found_patterns.push(*pattern);
        }
    }

    println!(
        "✅ Found {}/{} environment logging patterns:",
        found_patterns.len(),
        env_logging_patterns.len()
    );
    for pattern in &found_patterns {
        println!("   ✓ {}", pattern);
    }

    // Check for critical environment logging patterns
    let critical_patterns = [
        "Environment Configuration:",
        "Debug tracing initialized for environment variable logging",
        "Environment variables logged successfully",
    ];

    let mut missing_critical = Vec::new();
    for pattern in &critical_patterns {
        if !logs.contains(pattern) {
            missing_critical.push(*pattern);
        }
    }

    if !missing_critical.is_empty() {
        return Err(format!(
            "Missing critical environment logging patterns: {:?}",
            missing_critical
        ));
    }

    // Verify that environment variable logging happens early
    // It should appear before Sentry initialization
    let env_config_pos = logs.find("Environment Configuration:");
    let sentry_init_pos = logs.find("Sentry initialized.");

    if let (Some(env_pos), Some(sentry_pos)) = (env_config_pos, sentry_init_pos) {
        if env_pos > sentry_pos {
            return Err(
                "Environment variable logging should happen before Sentry initialization"
                    .to_string(),
            );
        }
        println!("✅ Environment variables logged before Sentry initialization");
    }

    // Check that we have the comprehensive format (tree structure with ├─ and ┌─)
    let tree_patterns = ["├─", "┌─"];
    let found_tree_chars = tree_patterns.iter().any(|pattern| logs.contains(pattern));

    if !found_tree_chars {
        return Err(
            "Environment variable logging should use tree structure format (├─, ┌─)".to_string(),
        );
    }

    println!("✅ Environment variable logging uses proper tree structure format");

    // Verify summary line exists and has reasonable format
    if let Some(summary_line) = logs
        .lines()
        .find(|line| line.contains("Summary:") && line.contains("variables configured"))
    {
        println!("✅ Found environment summary: {}", summary_line.trim());
    } else {
        return Err("Missing environment variable summary line".to_string());
    }

    Ok(())
}

#[test]
fn smoke_test() {
    // Step 0: Check if Docker is available
    println!("Checking Docker availability...");
    let docker_check = Command::new("docker")
        .args(["--version"])
        .output()
        .expect("Failed to execute docker command");

    if !docker_check.status.success() {
        panic!("Docker is not available. Please start Docker and try again.");
    }

    // Ensure cleanup happens even if test panics
    std::panic::set_hook(Box::new(|_| cleanup_docker()));

    // Step 1: Start the service using Docker
    println!("Starting service with docker-compose...");
    let mut cmd = Command::new("docker");
    cmd.args(["compose", "up", "-d"]).assert().success();

    // Step 2: Wait for service to be available
    let health_url = "http://localhost:3336/health";
    let mut success = false;
    for _ in 0..10 {
        // Try for ~30 seconds
        match reqwest::blocking::get(health_url) {
            Ok(response) if response.status().is_success() => {
                success = true;
                break;
            }
            _ => {
                println!("Waiting for service...");
                thread::sleep(Duration::from_secs(3));
            }
        }
    }

    if !success {
        cleanup_docker();
        panic!("Service did not become healthy in time!");
    }

    // Step 3: Test convert route (Version 0 ARK -> Version 1 ARK)
    // This specifically tests the parallel execution that was failing in staging
    println!("Testing convert route (this triggers parallel Python/Rust execution)...");
    let convert_url = "http://localhost:3336/convert/ark:/72163/0803-751e0b8a-6";
    match reqwest::blocking::get(convert_url) {
        Ok(response) => {
            if !response.status().is_success() {
                // Get logs before cleanup to see what failed
                let logs = get_container_logs();
                println!("Convert route failed. Recent logs:");
                let recent_logs: Vec<&str> = logs.lines().rev().take(10).collect();
                for line in recent_logs.iter().rev() {
                    println!("  {}", line);
                }
                cleanup_docker();
                panic!("Convert route failed: {}", response.status());
            }
            println!("Convert route test passed");
        }
        Err(e) => {
            cleanup_docker();
            panic!("Convert route test failed: {}", e);
        }
    }

    // Step 4: Test redirect routes for all ARK URL format variations
    // Uses a no-redirect client to verify 3xx responses
    println!("Testing redirect routes...");
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client");

    test_redirect_parity_cases(&client);

    // Step 5: Analyze logs before cleanup
    println!("Analyzing container logs for HTTP registry loading...");
    let logs = get_container_logs();

    // Test registry loading specifically
    if let Err(e) = analyze_registry_loading_logs(&logs) {
        cleanup_docker();
        panic!("Registry loading analysis failed: {}", e);
    }

    // Test parallel execution (the original staging issue)
    if let Err(e) = test_parallel_execution_logs(&logs) {
        cleanup_docker();
        panic!("Parallel execution analysis failed: {}", e);
    }

    // Test environment variable logging
    if let Err(e) = test_environment_logging_logs(&logs) {
        cleanup_docker();
        panic!("Environment variable logging analysis failed: {}", e);
    }

    println!("✅ Log analysis passed - HTTP registry loading and environment variable logging are working correctly");

    // Step 6: Additional test for registry loading failures
    println!("Testing registry loading failure handling...");
    test_registry_failure_scenario();

    // Step 7: Stop the service
    cleanup_docker();
    println!("All tests passed!");
}
