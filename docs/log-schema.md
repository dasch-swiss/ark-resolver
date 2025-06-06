# ARK Resolver Error Log Schema

## Overview

This document defines the structured JSON schema for logging error and discrepancy events in the ARK Resolver service. Only server errors (5xx), configuration/registry errors, and discrepancies between Python and Rust implementations are logged. User input validation errors (4xx invalid_input) and successful requests are not individually logged, but are tracked in metrics.

## JSON Log Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | string | ISO 8601 formatted timestamp when the request was processed (e.g., "2024-01-15T14:30:25.123Z") |
| `ark` | string | The full ARK string that was requested by the user |
| `project` | string\|null | Project identifier extracted from the ARK path segment (4-character hex, e.g., "0001"). Null if not present or parseable |
| `error_type` | string | Category of error. One of: "invalid_input", "not_found", "backend_failure", "discrepancy" |
| `python_result` | object | Result from Python implementation containing either success data or error details |
| `rust_result` | object | Result from Rust implementation containing either success data or error details |
| `discrepancy` | boolean | True if Python and Rust results don't match, false otherwise |
| `user_hash` | string | One-way hashed identifier derived from the request's source IP address |

### Error Type Categories

- **invalid_input**: Malformed ARK ID, invalid format, failed validation, or completely unparseable input (tracked in metrics only, not logged)
- **not_found**: Valid ARK format but project not found in registry, or NAAN mismatch with configured value
- **backend_failure**: Internal service errors, configuration issues, or system failures
- **discrepancy**: Python and Rust implementations returned different results

### Result Object Structure

Both `python_result` and `rust_result` fields contain objects with:
- `status`: "success" or "error"
- `data`: For success - the converted ARK or resource data; for error - error message
- `error_code`: Optional error classification (e.g., "ArkUrlException", "CheckDigitException")

## Privacy and User Identification

### IP Address Hashing

Raw IP addresses are never stored or logged. Instead, a `user_hash` is generated using:

1. **One-way hashing**: SHA-256 cryptographic hash function
2. **Salted hashing**: A secret salt value is prepended to the IP before hashing
3. **Purpose**: Enable counting unique users affected by errors without storing identifiable information
4. **Privacy rationale**: Ensures compliance with GDPR and Swiss privacy laws by preventing user identification while maintaining operational visibility

The hash is computed as: `SHA-256(SECRET_SALT + source_ip_address)`

## Example Log Entries

### Backend Failure

```json
{
  "timestamp": "2024-01-15T14:30:25.123Z",
  "ark": "ark:/72163/1/0001/ABC123XYZ",
  "project": "0001",
  "error_type": "backend_failure",
  "python_result": {
    "status": "error",
    "data": "Database connection timeout",
    "error_code": "ConnectionTimeout"
  },
  "rust_result": {
    "status": "error", 
    "data": "Registry service unavailable",
    "error_code": "ServiceUnavailable"
  },
  "discrepancy": false,
  "user_hash": "a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456"
}
```

### Discrepancy Between Implementations

```json
{
  "timestamp": "2024-01-15T15:45:10.456Z",
  "ark": "ark:/72163/1/0001/ABC123XYZ",
  "project": "0001",
  "error_type": "discrepancy",
  "python_result": {
    "status": "success",
    "data": "http://rdfh.ch/0001/converted-resource-id"
  },
  "rust_result": {
    "status": "success",
    "data": "http://rdfh.ch/0001/different-resource-id"
  },
  "discrepancy": true,
  "user_hash": "b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef1234567"
}
```

### Project Not Found

```json
{
  "timestamp": "2024-01-15T16:12:33.789Z",
  "ark": "ark:/72163/1/9999/ValidFormat",
  "project": "9999",
  "error_type": "not_found",
  "python_result": {
    "status": "error",
    "data": "Invalid ARK ID",
    "error_code": "KeyError"
  },
  "rust_result": {
    "status": "error",
    "data": "Project not found in registry",
    "error_code": "ProjectNotFound"
  },
  "discrepancy": false,
  "user_hash": "c3d4e5f6789012345678901234567890abcdef1234567890abcdef12345678"
}
```

### NAAN Mismatch

```json
{
  "timestamp": "2024-01-15T17:20:15.234Z",
  "ark": "ark:/99999/1/0001/ValidFormat",
  "project": "0001",
  "error_type": "not_found",
  "python_result": {
    "status": "error",
    "data": "Invalid ARK ID: ark:/99999/1/0001/ValidFormat",
    "error_code": "ArkUrlException"
  },
  "rust_result": {
    "status": "error",
    "data": "NAAN mismatch: expected 72163, got 99999",
    "error_code": "NaanMismatch"
  },
  "discrepancy": false,
  "user_hash": "d4e5f6789012345678901234567890abcdef1234567890abcdef123456789"
}
```

## Usage Notes

- Logs are emitted via OpenTelemetry to Grafana Cloud Loki
- Schema must be followed exactly for dashboard compatibility
- All timestamps should be in UTC
- Field values should be properly escaped for JSON serialization
- Missing or null values should be explicitly represented rather than omitted

## Edge Case Handling

### Invalid Input Handling
User input validation errors (4xx status codes) are classified as `invalid_input` but are **NOT logged** as individual entries. Instead, they are tracked only in aggregate metrics to monitor patterns of invalid requests without creating log noise.

Examples of invalid input that generate metrics but no logs:
- Completely unparseable input (random strings, invalid formats)
- Malformed ARK IDs that fail basic validation
- Input that cannot be processed by either implementation due to format issues

### Server Errors and Configuration Issues
Only server errors (5xx), configuration/registry issues (valid ARK format but missing projects in registry, NAAN mismatches), and discrepancies between implementations generate individual log entries. These represent actionable issues that require investigation.

Note: The ARK Resolver only handles conversion and forwarding - it does not validate whether the final target resource actually exists, so "resource not found" errors in that sense do not occur at this level.