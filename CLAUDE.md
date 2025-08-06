# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.


## Notes
Claude MUST first present plan. Only after confirmation, start implementing plan and changing code.
Claude MUST ask questions about code coverage and if adding new unit tests or extending existing tests is required.
Claude MUST keep the docs/todos.md file uptodate with the current plan, and ckeck off the tasks that where completed.

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
# This sets ARK_REGISTRY_FILE and runs: uv run ark_resolver/ark.py -s

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
