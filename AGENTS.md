# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.


## Notes
The coding agent MUST first present plan. Only after confirmation, start implementing plan and changing code.
The coding agent MUST ask questions about code coverage and if adding new unit tests or extending existing tests is required.

## Documentation Conventions

### Business Rules Documentation

When working with coding agents, provide requirements and specifications at the **business rules level** rather than
implementation details. This approach enables coding agents to make informed decisions about how to best implement these
rules in code.

#### Business Rule Prefix Convention

All business rules embedded in code must be prefixed with `BR: ` to ensure:
- Easy searchability across the codebase
- Clear distinction between business logic and implementation details
- Traceability from requirements to code

**Example:**
```rust
// BR: Only public resources resolve to direct content URLs; restricted or embargoed resources redirect to a landing page
match (resource.visibility, resource.release_date) {
    (Visibility::Public, None) => redirect(content_url(&resource)),
    (Visibility::Public, Some(date)) if now() >= date => redirect(content_url(&resource)),
    _ => redirect(landing_page(&resource)),
}
```

### Comment Philosophy

#### Focus on "Why" Not "What"

Comments should explain the **reasoning** behind decisions, not describe what the code does:

❌ **Avoid "What" Comments:**
```rust
// Iterate over resources
for resource in resources.iter() {
    // Check if embargo is active
    if let Some(date) = resource.release_date {
        if now() < date {
            // Redirect to landing page
            redirect(landing_page(resource));
        }
    }
}
```

✅ **Prefer "Why" Comments:**
```rust
// BR: Embargoed resources must not expose content before release_date; redirect to landing page until release
for resource in resources.iter() {
    if resource.release_date.map_or(false, |d| now() < d) {
        redirect(landing_page(resource));
    } else {
        redirect(content_url(resource));
    }
}
```

#### Code Clarity Principle

If you find yourself needing to explain **what** the code does, this is a signal that the code should be refactored for clarity:

**Instead of commenting unclear code:**
```rust
// Compute ARK check digit
let d = ((ark_suffix.bytes().fold(0u32, |acc, b| acc + b as u32) * 3) % 10) as u8 + b'0';
```

**Refactor for self-documentation:**
```rust
fn compute_ark_check_digit(ark_suffix: &str) -> char {
    // BR: Validate ARK check digit before redirect
    // Placeholder for ISO 7064-style calculation
    compute_check_digit_impl(ark_suffix)
}

let check = compute_ark_check_digit(ark_suffix);
```

### Best Practices

1. **Business Rules First**: When describing requirements to Claude Code, focus on business rules and constraints rather than implementation specifics
2. **Searchable Documentation**: Use the `BR: ` prefix consistently to make business rules easily discoverable
3. **Self-Documenting Code**: Write code that clearly expresses its intent through meaningful names and structure
4. **Refactor Over Comment**: If code requires explanation of what it does, refactor it to be more understandable
5. **Context Over Description**: Comments should provide context, rationale, and business justification

### Example Business Rules Format

When providing requirements to Claude Code, structure them as business rules:

```
BR: Embargoed resources redirect to a landing page until release_date
BR: Tombstoned resources return HTTP 410 with an explanatory page
BR: Public resources resolve to direct content URLs; restricted resources resolve to landing pages
BR: Legacy salsah.org ARKs map deterministically to DSP IRIs using UUID v5 namespace
BR: ARK suffix must include a valid check digit before redirect
```

This approach ensures that the implementation remains flexible while the business logic is clearly documented and traceable.

## Project Overview

The DSP ARK Resolver is a hybrid Python/Rust application that resolves ARK URLs referring to resources in
DSP (DaSCH Service Platform) repository.

The project is in the process of migrating the codebase from Python to Rust. This will happen in three phases:

1. Add functionality to Rust and run in parallel with the Python implementation to verify correct behavior in production,
   while the Python behavior is user facing. The convention is, that the same Python code that now uses the Rust library
   should be duplicated into files that end with `_rust.py`. Important: Always add comparative unit tests between the
   Python and Rust implementations.
2. Change user facing behavior to Rust implementation, and start removing Python.
3. Refactor Rust code into a service using Axum, and removing Python(PyO3/Maturin.

The core architecture combines:

- **Python (Sanic)**: Main HTTP server, routing, and business logic (`ark_resolver/ark.py`)
- **Rust (PyO3)**: Phase 1 functions exposed as Python extensions (`src/lib.rs`)
- **Configuration-driven**: Uses INI files for ARK registry and server configuration

The resolver operates in two modes:
1. HTTP server that redirects ARK URLs to actual resource locations
2. Command-line tool for converting between resource IRIs and ARK URLs

## Development Commands

### Setup
```bash
# Install uv dependency manager (if not installed)
curl -LsSf https://astral.sh/uv/install.sh | sh

# Install dependencies
uv sync --locked --no-install-project
# or using just
just install
```

### Build and Development
```bash
# Build Rust extensions with maturin
just build
# or manually
uv run maturin develop

# Run the resolver locally
just run
# This sets ARK_REGISTRY and runs: uv run ark_resolver/ark.py -s

# Run as command-line tool (examples)
./ark_resolver/ark.py -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ  # IRI to ARK
./ark_resolver/ark.py -a http://ark.example.org/ark:/00000/0002-751e0b8a-6  # ARK redirect
```

### Testing
```bash
# Python tests (requires build first)
just pytest
# or
uv run pytest

# Rust unit tests
just test
# or
cargo test --lib

# Smoke tests (Docker-based)
just smoke-test
```

### Code Quality
```bash
# Check all code
just check  # Runs both rustcheck and pycheck

# Python checks
just pycheck
# Runs: uv run ruff format --check . && uv run ruff check . && uv run pyright

# Rust checks  
just rustcheck
# Runs: cargo +nightly fmt --check && cargo clippy -- -D warnings

# Format all code
just fmt  # Runs both rustfmt and pyfmt
```

## Architecture Details

### Python-Rust Integration
- Rust code is compiled as a Python extension module (`_rust`) using maturin/PyO3
- Key Rust functions exposed: `base64url_check_digit`, `load_settings`, `initialize_tracing`, `initialize_debug_tracing`
- Settings loading and parsing performance optimized in Rust (`src/ark_url_settings.rs`)
- HTTP configuration fetching with comprehensive error diagnostics and SIGTERM prevention
- Debug tracing integration for HTTP request troubleshooting in containerized environments

### Configuration System  
- All configuration is done via environment variables (host, port, GitHub webhook secret, registry file)
- `tests/ark-registry.ini`: Project-specific ARK URL templates and redirect targets (for local testing only)
- Settings are loaded via Rust for performance (`ArkUrlSettings` class)

### Rust HTTP Client Configuration
Additional environment variables for debugging and timeout control:
- `ARK_RUST_LOAD_TIMEOUT_MS`: Application-level timeout for settings loading (default: 15000ms) - prevents SIGTERM
- `ARK_RUST_HTTP_TIMEOUT_MS`: HTTP request total timeout (default: 10000ms) - matches Python behavior
- `ARK_RUST_HTTP_CONNECT_TIMEOUT_MS`: HTTP connection timeout (default: 5000ms)
- `ARK_RUST_FORCE_IPV4`: Force IPv4-only connections, disable IPv6 (default: false) - fixes container IPv6 connectivity issues
- `RUST_LOG`: Controls tracing verbosity (e.g., `RUST_LOG=ark_resolver=debug,reqwest=debug,hyper=debug`)
- `ARK_SENTRY_DEBUG`: Enable Sentry debug mode (default: false) - accepts "true"/"1"/"yes"/"on" for true
- Proxy support via standard environment variables (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`)

### Key Python Modules
- `ark_resolver/ark.py`: Main server and CLI entry point
- `ark_resolver/ark_url.py`: ARK URL parsing and formatting logic  
- `ark_resolver/routes/`: Sanic route handlers (health, convert endpoints)
- `ark_resolver/check_digit.py`: ARK check digit validation

### UUID Generation for Legacy Migration
Uses UUID v5 with DaSCH-specific namespace (`cace8b00-717e-50d5-bcb9-486f39d733a2`) to create deterministic resource IRIs from legacy salsah.org ARK URLs, ensuring permanent identifier continuity.

## Docker Usage
Images published to `daschswiss/ark-resolver`. Build commands available via just:
```bash
just docker-build-intel  # linux/amd64
just docker-build-arm    # linux/arm64  
```
