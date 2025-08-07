# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.


## Notes
Claude MUST first present plan. Only after confirmation, start implementing plan and changing code.
Claude MUST ask questions about code coverage and if adding new unit tests or extending existing tests is required.

## Documentation Conventions

### Business Rules Documentation

When working with Claude Code, provide requirements and specifications at the **business rules level** rather than implementation details. This approach enables Claude Code to make informed decisions about how to best implement these rules in code.

#### Business Rule Prefix Convention

All business rules embedded in code must be prefixed with `BR: ` to ensure:
- Easy searchability across the codebase
- Clear distinction between business logic and implementation details
- Traceability from requirements to code

**Example:**
```rust
// BR: Premium customers receive 20% discount on orders over $100
if customer.is_premium && order.total > 100.0 {
    order.apply_discount(0.20);
}
```

### Comment Philosophy

#### Focus on "Why" Not "What"

Comments should explain the **reasoning** behind decisions, not describe what the code does:

❌ **Avoid "What" Comments:**
```rust
// Loop through all users
for user in users.iter() {
    // Check if user is active
    if user.is_active {
        // Send email to user
        send_email(user);
    }
}
```

✅ **Prefer "Why" Comments:**
```rust
// BR: Inactive users should not receive promotional emails per GDPR compliance
for user in users.iter() {
    if user.is_active {
        send_email(user);
    }
}
```

#### Code Clarity Principle

If you find yourself needing to explain **what** the code does, this is a signal that the code should be refactored for clarity:

**Instead of commenting unclear code:**
```rust
// Calculate compound interest
let result = p * (1.0 + r/n).powf(n*t);
```

**Refactor for self-documentation:**
```rust
fn calculate_compound_interest(principal: f64, rate: f64, times_compounded: f64, years: f64) -> f64 {
    // BR: Compound interest formula required by financial regulations
    principal * (1.0 + rate/times_compounded).powf(times_compounded * years)
}

let result = calculate_compound_interest(principal, rate, times_compounded, years);
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
BR: Orders must be processed within 24 hours during business days
BR: Customers with failed payments should be notified after 3 attempts
BR: Inventory levels below 20% trigger automatic reorder
BR: Price changes require manager approval if variance exceeds 15%
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
- Key Rust functions exposed: `base64url_check_digit`, `load_settings`, `initialize_tracing`
- Settings loading and parsing performance optimized in Rust (`src/ark_url_settings.rs`)

### Configuration System  
- All configuration is done via environment variables (host, port, GitHub webhook secret, registry file)
- `tests/ark-registry.ini`: Project-specific ARK URL templates and redirect targets (for local testing only)
- Settings are loaded via Rust for performance (`ArkUrlSettings` class)

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
