---
dune_map: true
schema_version: 1
last_verified_commit: a70d1e1f4b7b505f6f8d9e0f3316cb16c40ffe5d
date: 2026-09-26
---

# ARCH-MAP.md

## Overview

The ARK resolver turns DaSCH ARK identifiers (`ark:/72163/...`) into redirects to the pages they identify, and converts between ARKs and DSP resource IRIs. It is a Python Sanic service with a Rust extension (`ark_resolver._rust`, built by maturin from the same crate). The Rust side is a reimplementation in progress: every ARK request runs Python and Rust side by side, **the Python result is served**, and the Rust result is only compared and reported (shadow execution). Redirect targets come from an INI registry held in `dasch-swiss/ark-resolver-data`. Rust layering follows `docs/adr/0001-adopt-hexagonal-architecture.md`. Domain vocabulary is in `CONTEXT.md`.

## Components

### http-service

- **Paths:** `ark_resolver/ark.py`, `ark_resolver/routes/**`, `ark_resolver/error_diagnostics.py`, `ark_resolver/tracing.py`, `ark_resolver/__init__.py`, `tests/test_cors_headers.py`, `tests/test_error_diagnostics.py`, `tests/test_redirect_head.py`
- **Purpose:** The Sanic app. It serves the routes, owns settings loading and reloading, and wires in Sentry and OpenTelemetry.
- **Key entities:** `app`, `main`, `server`, `load_settings`, `reload_config`, `schedule_reload`, `get_safe_config`, `add_cors_headers`, `init_tracing_and_sentry`, `redirect_bp`, `convert_bp`, `health_bp`, `pre_validate_ark`, `classify_exception`, `report_error_to_sentry`, `error_response`, `ArkErrorCode`, `tracer`
- **Public interface:**
  - HTTP:
    - `GET|HEAD /<ark>`: redirect.
    - `GET /convert/<ark>`: ARK to IRI.
    - `GET /health/`.
    - `GET|HEAD /config`: config with the secret stripped.
    - `POST /reload`: GitHub webhook, HMAC-SHA1 via `X-Hub-Signature`.
  - CLI: `python -m ark_resolver.ark -s | -i | -a`.
  - Tests use `load_settings()` directly.
- **Local-context kit:** `ark_resolver/ark.py`, `ark_resolver/routes/redirect.py`, `ark_resolver/routes/convert.py`, `ark_resolver/error_diagnostics.py`, `ark_resolver/parallel_execution.py`, `ark_resolver/ark_url_rust.py`, `tests/test_redirect_head.py`
- **Depends on:** shadow-bridge, python-resolution, rust-adapters (direct `_rust` import in `ark.py` for `load_settings`, `initialize_debug_tracing`, `log_environment_variables`)
- **Used by:** build-and-delivery (entrypoint, Dockerfile HEALTHCHECK, smoke test)
- **Boundary rules:**
  - Every ARK route follows one sequence:
    1. A non-`ark:/` path goes to `diagnose_non_ark_path`, with no Sentry report.
    2. `unquote`.
    3. A `tracer` span.
    4. `pre_validate_ark`.
    5. `parallel_executor.execute_parallel`.
    6. On exception: `classify_exception`, then `report_error_to_sentry`, then `error_response`.

    Enforcement: `review`.
  - The catch-all `redirect_bp` is registered last in `ark.py`, because it shadows every path registered after it. Enforcement: `docs-only`.
  - Routes read settings only through `req.app.config.settings` and `req.app.config.rust_settings`. There is no typed accessor. Enforcement: `docs-only`.
- **Durable state:**
  - `app.config.settings` (Python `ArkUrlSettings`) and `app.config.rust_settings` (Rust settings, or `None`). The only writers are `server()` and `reload_config()` in `ark.py`.
  - On a failed Rust reload, `reload_config()` keeps the old Rust settings but replaces the Python ones, so the two can drift apart.
  - Sanic forks its workers, so `/reload` refreshes only the worker that receives it. This is inferred from the code, not verified.

### python-resolution

- **Paths:** `ark_resolver/ark_url.py`, `ark_resolver/check_digit.py`, `tests/test_ark_url.py`, `tests/test_ckeck_digit.py`, `tests/ark-registry.ini`, `tests/__init__.py`
- **Purpose:** The authoritative ARK logic, whose result users actually receive. It covers parsing, check digits, redirect-URL templating, and conversion between ARKs and IRIs.
- **Key entities:** `ArkUrlSettings`, `ArkUrlInfo`, `ArkUrlFormatter`, `ArkUrlException`, `VersionMismatchException`, `VersionZeroNotAllowedException`, `add_check_digit_and_escape`, `unescape_and_validate_uuid`, `CheckDigitException`, `calculate_check_digit`
- **Public interface:**
  - `ArkUrlInfo(settings, ark_id)` with `.to_redirect_url()`, `.to_resource_iri()` and `.get_timestamp()`.
  - `ArkUrlFormatter(settings)` with `.resource_iri_to_ark_id()` and `.format_ark_url()`.
  - The exception types. The routes' error classification depends on them.
- **Local-context kit:** `ark_resolver/ark_url.py`, `ark_resolver/check_digit.py`, `ark_resolver/ark_url_rust.py`, `tests/test_ark_url.py`, `tests/test_redirect_parity.py`, `tests/ark-registry.ini`
- **Depends on:** none (stdlib only)
- **Used by:** http-service, shadow-bridge (`ark_url_rust.py` reuses `ArkUrlException`)
- **Boundary rules:** It never imports `ark_resolver._rust`. Keeping this rule is what keeps the shadow comparison independent. Enforcement: `docs-only`.
- **Durable state:** None. Settings are parsed from the registry by `ark.load_settings()` (http-service).

### shadow-bridge

- **Paths:** `ark_resolver/ark_url_rust.py`, `ark_resolver/check_digit_rust.py`, `ark_resolver/parallel_execution.py`, `tests/test_ark_url_rust.py`, `tests/test_check_digit_rust.py`, `tests/test_redirect_parity.py`, `tests/test_convert_parity.py`, `tests/test_http_registry_rust.py`, `tests/test_sentry_fingerprinting.py`
- **Purpose:**
  - Holds the Python-facing wrappers over the Rust extension.
  - Runs Python and Rust on the same input and compares the results.
  - Reports mismatches to OTel spans and Sentry without ever changing the response.
- **Key entities:** `ParallelExecutor`, `parallel_executor`, `execute_parallel`, `ComparisonResult`, `ParallelExecutionResult`, `add_to_span`, `track_with_sentry`, `ArkUrlInfoRust`, `ArkUrlFormatterRust`
- **Public interface:**
  - `parallel_executor.execute_parallel(op, python_fn, rust_fn)`, followed by `add_to_span` and `track_with_sentry`.
  - Re-exported Rust `ArkUrlInfo`, `ArkUrlFormatter`, `ArkUrlSettings` and the two UUID functions, with Rust `ValueError` mapped to `ArkUrlException`.
- **Local-context kit:** `ark_resolver/parallel_execution.py`, `ark_resolver/ark_url_rust.py`, `ark_resolver/check_digit_rust.py`, `src/lib.rs`, `tests/test_redirect_parity.py`, `docs/learnings/integration-issues/pyo3-rust-python-shadow-execution-parity.md`
- **Depends on:** python-resolution, rust-adapters
- **Used by:** http-service
- **Boundary rules:**
  - `execute_parallel` returns the Python result and re-raises a Python exception. A Rust error or mismatch never reaches a user. Enforcement: `structure`.
  - It catches `BaseException` around the Rust call, because PyO3's `PanicException` is not an `Exception`, and re-raises `KeyboardInterrupt` and `SystemExit`. Enforcement: `review`.
  - Sentry events are fingerprinted `["shadow", op, comparison]`, not one issue per ARK. Enforcement: `static-analysis` (`test_sentry_fingerprinting.py`).
- **Durable state:**
  - The `parallel_executor` module singleton holds per-worker counters, which are not persisted.
  - Parity coverage: redirect is compared by `test_redirect_parity.py`, and convert step by step by `test_convert_parity.py`.

### rust-core

- **Paths:** `src/core/**`
- **Purpose:** The hexagonal Rust domain for the capabilities migrated so far: check digit, UUID processing, settings, ARK URL info, and ARK URL formatting. It is organised as domain, errors, ports and use cases.
- **Key entities:** `SettingsManager`, `SettingsWithRegexes`, `SettingsRegistry`, `ArkConfig`, `ConfigurationProvider`, `ArkUrlInfoProcessor`, `ArkUrlFormatterService`, `ArkUuidProcessor`, `CheckDigitValidator`, `SettingsError`, `ark_path_regex`
- **Public interface:**
  - Use cases and port traits: `ConfigurationProvider`, `EnvironmentProvider`, `RegexProvider`, `FileSystemProvider`, `ArkUrlParsingPort`, `ConfigurationPort`, `TemplatePort`, `UuidGenerationPort`, `ArkUrlFormatterPort`.
  - Errors.
  - The domain types that ports return.
  - Everything is `pub`; nothing is `pub(crate)`.
- **Local-context kit:** `src/core/use_cases/settings_manager.rs`, `src/core/ports/settings.rs`, `src/core/domain/settings.rs`, `src/core/errors/settings.rs`, `src/core/use_cases/ark_url_info_processor.rs`, `src/adapters/pyo3/settings.rs`, `docs/adr/0001-adopt-hexagonal-architecture.md`
- **Depends on:** none (crates: `regex`, `thiserror`, `async_trait`, `serde_json`, `config`)
- **Used by:** rust-adapters
- **Boundary rules:**
  - `src/core` never imports `crate::adapters`, `pyo3`, `std::fs`, `std::env` or `reqwest`. File, environment and HTTP access go through ports. Enforcement: `docs-only`. It is one crate with a non-optional `pyo3` dependency, so `cargo test --lib --no-default-features` cannot catch a violation.
  - Current violations:
    - `pyo3::Python::initialize()` in the tests of `domain/uuid_processing.rs`.
    - `From<std::io::Error | config::ConfigError | std::env::VarError>` on `SettingsError`.
    - `regex` inside the domain, although ADR-0001 says the domain uses std only.
  - One capability per file name across `domain/`, `errors/`, `ports/` and `use_cases/`. Enforcement: `docs-only`.
- **Durable state:** None. `SettingsWithRegexes` is built only by `SettingsManager::load_settings`, `load_minimal_settings` and `reload_settings`. `SettingsRepository` has no implementation.

### rust-adapters

- **Paths:** `src/adapters/**`, `src/lib.rs`, `build.rs`
- **Purpose:**
  - The PyO3 module `_rust` and its classes.
  - The infrastructure providers (file system, HTTP registry fetch, environment, INI parsing) that implement the core's ports.
  - Composition of the use cases by hand.
- **Key entities:** `fn _rust`, `ArkUrlSettings`, `load_settings`, `PyArkUrlInfo`, `ArkUrlFormatter`, `FileSystemConfigurationProvider`, `HttpConfigurationProvider`, `EnvironmentVariableProvider`, `IniProcessor`, `initialize_debug_tracing`, `TRACING_INITIALIZED`, `log_environment_variables`
- **Public interface:**
  - Only the `_rust` Python ABI registered in `src/lib.rs`:
    - check-digit functions
    - UUID functions
    - `load_settings` and `ArkUrlSettings`
    - `ArkUrlInfo` and `ArkUrlFormatter`
    - `initialize_debug_tracing` and `log_environment_variables`
  - `mod adapters` is private.
- **Local-context kit:** `src/lib.rs`, `src/adapters/pyo3/settings.rs`, `src/adapters/pyo3/ark_url_info.rs`, `src/adapters/file_system/settings.rs`, `src/adapters/http/mod.rs`, `src/adapters/environment/settings.rs`, `ark_resolver/ark_url_rust.py`
- **Depends on:** rust-core
- **Used by:** shadow-bridge, http-service
- **Boundary rules:**
  - A new Python-visible function or class is registered by hand in `fn _rust` and re-exported from a `*_rust.py` wrapper. Enforcement: `docs-only`.
  - Adapters reach into core domain internals directly: `parsing::*_regex` from `environment`, a domain `ArkUrlInfo` wrapped in `pyo3/ark_url_info.rs`, and `SettingsWithRegexes` fields. These are recorded as current state, not as a sanctioned pattern.
  - Adapter-to-adapter coupling:
    - `file_system` delegates to `http` for `http(s)` registries.
    - `pyo3/settings` composes `environment` and `file_system`.
- **Durable state:**
  - The only static is `TRACING_INITIALIZED` (`std::sync::Once`).
  - Each `ArkUrlSettings::new()` builds its own tokio runtime and fetches the registry itself, independently of the Python fetch. That means two fetches per load or reload.

### build-and-delivery

- **Paths:** `Dockerfile`, `.dockerignore`, `docker-compose.yml`, `entrypoint.sh`, `justfile`, `Makefile`, `vars.mk`, `pyproject.toml`, `uv.lock`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.github/**`, `.gitignore`, `.claude/**`, `eng.yaml`, `tests/smoke_test.rs`
- **Purpose:**
  - Builds the extension in place with maturin, then the two-stage Alpine image.
  - CI gates.
  - release-please versioning from `Cargo.toml`.
  - Docker Hub publishing and the Jenkins deploy webhooks.
- **Key entities:** `just test`, `just pytest`, `just pycheck`, `just smoke-test`, `docker-image-tag`, `check.yml`, `test.yml`, `security.yml`, `publish.yml`, `claude-review.yml`
- **Public interface:**
  - The `just` recipes.
  - The image `daschswiss/ark-resolver:<cargo-version>[-<sha>]`.
  - `entrypoint.sh`, which runs `python3 -m ark_resolver.ark "$@"` with the default argument `-s`.
- **Local-context kit:** `justfile`, `Dockerfile`, `pyproject.toml`, `Cargo.toml`, `.github/workflows/test.yml`, `.github/workflows/publish.yml`, `.github/workflows/security.yml`
- **Depends on:** http-service (it runs it)
- **Used by:** none
- **Boundary rules:**
  - The version lives in `Cargo.toml` (release-type `rust`). `pyproject.toml` stays at `0.1.0`. Enforcement: `static-analysis` (release-please).
  - `Makefile` and `vars.mk` are obsolete and reference files that do not exist. The `justfile` is authoritative. Enforcement: `docs-only`.
- **Durable state:**
  - Release state: `.github/release-please/manifest.json` and `CHANGELOG.md`. The only writer is release-please.
  - Deployment is outside this repo, reached through `JENKINS_*` webhooks.

### project-docs

- **Paths:** `README.md`, `CLAUDE.md`, `CONTEXT.md`, `CHANGELOG.md`, `LICENSE`, `ARCH-MAP.md`, `docs/**`
- **Purpose:** Documentation for users, operators and agents, plus the ADRs, specs and learnings.
- **Key entities:** `ADR-0001`, `log-schema`, `docs/specs/`
- **Public interface:** `README.md` (deployment, env vars, routes), `CLAUDE.md` (agent guidance), `CONTEXT.md` (domain vocabulary), `docs/adr/`
- **Local-context kit:** `README.md`, `CLAUDE.md`, `docs/adr/0001-adopt-hexagonal-architecture.md`, `docs/learnings/integration-issues/pyo3-rust-python-shadow-execution-parity.md`
- **Depends on:** none
- **Used by:** none
- **Boundary rules:**
  - Changes to environment variables, configuration or architecture must be reflected in both `CLAUDE.md` and `README.md`. Enforcement: `docs-only`.
  - `docs/log-schema.md` describes a schema that no code emits. Treat it as a target, not a description.
- **Durable state:** `CHANGELOG.md` (release-please).

## Cross-cutting concerns

- **Configuration.** `ARK_*` environment variables are read twice, once by Python `ark.load_settings()` and once by Rust `EnvironmentVariableProvider`. The code-side inventory is the `EnvVarDefinition` table in `src/adapters/environment/env_logger.rs`. A new variable must be added to both loaders, to that table, and to `README.md` and `CLAUDE.md`.
- **The registry.** `ARK_REGISTRY` is a path or an `http(s)` URL (production and staging: raw GitHub from `dasch-swiss/ark-resolver-data`). Python and Rust each parse it separately. `tests/ark-registry.ini` is the fixture for every test.
- **Observability.**
  - Sentry is Python-only (`init_tracing_and_sentry`).
  - The OTel TracerProvider is set when `tracing.py` is imported, and exports only to Sentry or the console.
  - Rust logs through `tracing` via `initialize_debug_tracing`.
- **Business-rule comments.** Rules embedded in code carry the `BR: ` prefix (`CLAUDE.md`).

## Conventions

- **Local-context kit budget:** ≤7 files per component. Enforcement: `docs-only`.
- **Dependency direction (Python):** `http-service → shadow-bridge → {python-resolution, rust-adapters}`. `python-resolution` never imports `_rust`. Enforcement: `docs-only`. ruff `TID` is enabled with no `banned-api`, so this could be promoted to `static-analysis`.
- **Dependency direction (Rust):** `rust-adapters → rust-core`. The core never imports adapters or `pyo3` (ADR-0001). Enforcement: `docs-only`. Splitting the core into its own crate without `pyo3` would make this `structure`.
- **Python is authoritative until Phase 2:**
  - New resolution behaviour is implemented in `python-resolution` and in `rust-core` plus `rust-adapters`.
  - It is wrapped in a `*_rust.py` module and run through `execute_parallel`.
  - It lands with a comparative parity test.

  Enforcement: `review` (the `claude-review.yml` prompt).
- **Rust extension access goes through `*_rust.py` wrappers.** Enforcement: `docs-only`. Violated by `ark.py` (`load_settings`, `initialize_debug_tracing`, `log_environment_variables`) and by several tests.
- **Wiring:**
  - A new route is a Blueprint module in `ark_resolver/routes/`, registered in `ark.py` before `redirect_bp`.
  - A new Rust export is registered in `fn _rust`.

  Both are registered by hand. Enforcement: `docs-only`.
- **Agent workflow:** present a plan before changing code, and ask about test coverage (`CLAUDE.md`). Enforcement: `docs-only`.

### Banned constructs

| Locally attractive pattern | Why it couples globally | Supported alternative | Enforcement |
|---|---|---|---|
| Serving the Rust result from a route | It bypasses the shadow contract, so a Rust divergence reaches users unnoticed | Keep `execute_parallel`, which returns the Python result; Phase 2 changes it in one place | `structure` |
| `import ark_resolver._rust` in a route or in `python-resolution` | Couples authoritative code to the shadow implementation and breaks the independent comparison | Import from `ark_url_rust.py` or `check_digit_rust.py` | `docs-only` |
| Assigning `app.config.settings` or `rust_settings` outside `server()` or `reload_config()` | A second writer means Python and Rust settings can diverge silently | Call `reload_config()` | `docs-only` |
| `pyo3`, `std::env` or `std::fs` in `src/core` | Violates ADR-0001; the core can no longer be tested or reused without Python | Add a port in `src/core/ports` and implement it in `src/adapters` | `docs-only` |
