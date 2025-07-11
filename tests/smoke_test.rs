use assert_cmd::Command;
use std::{thread, time::Duration};

fn cleanup_docker() {
    println!("Cleaning up Docker containers...");
    let _ = Command::new("docker-compose")
        .args(["down"])
        .output();
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
    let mut cmd = Command::new("docker-compose");
    cmd.args(["up", "-d"]).assert().success();

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
    println!("Testing convert route...");
    let convert_url = "http://localhost:3336/convert/ark:/99999/0002-751e0b8a-6";
    match reqwest::blocking::get(convert_url) {
        Ok(response) => {
            if !response.status().is_success() {
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

    // Step 4: Test redirect route (Version 1 ARK -> redirect to resource)
    println!("Testing redirect route...");
    let redirect_url = "http://localhost:3336/ark:/99999/1/0002";
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client");
    
    match client.get(redirect_url).send() {
        Ok(response) => {
            if !response.status().is_redirection() {
                cleanup_docker();
                panic!("Redirect route should return 3xx status, got: {}", response.status());
            }
            println!("Redirect route test passed");
        }
        Err(e) => {
            cleanup_docker();
            panic!("Redirect route test failed: {}", e);
        }
    }

    // Step 5: Stop the service
    cleanup_docker();
    println!("All tests passed!");
}
