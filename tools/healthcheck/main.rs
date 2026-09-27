//! Container healthcheck for the ark-resolver image.
//!
//! The distroless image has no shell, curl or jq, so deployments run this
//! binary directly: `["CMD", "/app/healthcheck"]`. It asks the server's own
//! `/health` endpoint and exits 0 only when it answers `{"status": "ok"}`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;
use std::time::Duration;

const DEFAULT_PORT: u16 = 3336;
const TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> ExitCode {
    let port = match std::env::var("ARK_INTERNAL_PORT") {
        Ok(value) => match value.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("unhealthy: ARK_INTERNAL_PORT={value:?} is not a port number");
                return ExitCode::FAILURE;
            }
        },
        Err(_) => DEFAULT_PORT,
    };

    match check(SocketAddr::from(([127, 0, 0, 1], port))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("unhealthy: {reason}");
            ExitCode::FAILURE
        }
    }
}

fn check(addr: SocketAddr) -> Result<(), String> {
    let response = fetch_health(addr)?;
    let body = parse_ok_response(&response)?;
    let json: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("body is not JSON: {e}"))?;
    match json.get("status").and_then(|s| s.as_str()) {
        Some("ok") => Ok(()),
        Some(other) => Err(format!("status is {other:?}")),
        None => Err("body has no string `status` field".to_string()),
    }
}

fn fetch_health(addr: SocketAddr) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect_timeout(&addr, TIMEOUT)
        .map_err(|e| format!("cannot connect to {addr}: {e}"))?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(TIMEOUT)))
        .map_err(|e| format!("cannot set socket timeouts: {e}"))?;
    // HTTP/1.0 with `Connection: close` lets us read the response to EOF.
    stream
        .write_all(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .map_err(|e| format!("cannot send request: {e}"))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| format!("cannot read response: {e}"))?;
    Ok(response)
}

/// Returns the body of a `200` response, decoding chunked transfer encoding.
fn parse_ok_response(response: &[u8]) -> Result<Vec<u8>, String> {
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("response has no header terminator")?;
    let head = std::str::from_utf8(&response[..split]).map_err(|_| "headers are not UTF-8")?;
    let body = &response[split + 4..];

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let code = status_line.split_whitespace().nth(1).unwrap_or_default();
    if code != "200" {
        return Err(format!("HTTP status line is {status_line:?}"));
    }

    let chunked = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("transfer-encoding")
                && value.trim().eq_ignore_ascii_case("chunked")
        })
    });
    if chunked {
        decode_chunked(body)
    } else {
        Ok(body.to_vec())
    }
}

fn decode_chunked(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("truncated chunk size line")?;
        let size_field = std::str::from_utf8(&body[..line_end]).map_err(|_| "bad chunk size")?;
        let size_hex = size_field.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_hex, 16).map_err(|_| "bad chunk size")?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if body.len() < size + 2 {
            return Err("truncated chunk".to_string());
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    /// Serves one canned response on an ephemeral port and returns its address.
    fn serve_once(response: &'static [u8]) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut conn, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = conn.read(&mut request);
            conn.write_all(response).unwrap();
        });
        addr
    }

    #[test]
    fn healthy_server_passes() {
        let addr = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 30\r\n\r\n{\"status\":\"ok\",\"uptime\":12345}",
        );
        assert_eq!(check(addr), Ok(()));
    }

    #[test]
    fn chunked_healthy_body_passes() {
        let addr = serve_once(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\n{\"stat\r\n9\r\nus\":\"ok\"}\r\n0\r\n\r\n",
        );
        assert_eq!(check(addr), Ok(()));
    }

    #[test]
    fn non_ok_status_fails() {
        let addr = serve_once(b"HTTP/1.1 200 OK\r\n\r\n{\"status\":\"degraded\"}");
        assert_eq!(check(addr), Err("status is \"degraded\"".to_string()));
    }

    #[test]
    fn non_200_fails() {
        let addr = serve_once(b"HTTP/1.1 503 Service Unavailable\r\n\r\n{\"status\":\"ok\"}");
        assert!(check(addr).unwrap_err().contains("503"));
    }

    #[test]
    fn malformed_body_fails() {
        let addr = serve_once(b"HTTP/1.1 200 OK\r\n\r\nnot json");
        assert!(check(addr).unwrap_err().contains("not JSON"));
    }

    #[test]
    fn nothing_listening_fails() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        assert!(check(addr).unwrap_err().contains("cannot connect"));
    }
}
