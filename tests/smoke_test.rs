use assert_cmd::Command;
use std::{thread, time::Duration};

#[test]
fn smoke_test() {
    // Step 1: Start the service using Docker
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

    assert!(success, "Service did not become healthy in time!");

    // Step 3: Stop the service
    Command::new("docker")
        .args(["compose", "down"])
        .assert()
        .success();
}
