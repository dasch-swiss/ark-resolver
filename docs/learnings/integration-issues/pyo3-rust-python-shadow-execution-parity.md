---
title: "PyO3 Shadow Execution Parity: Exception Handling, Case Normalization, and Sentry Grouping"
date: 2026-02-14
category: "integration-issues"
component: "ark-resolver"
module: "parallel_execution, routes/redirect, ark_url_info_processor"
problem_type: "integration-issue"
severity: "high"
symptoms:
  - "Shadow execution silently swallowed Rust panics (BaseException vs Exception)"
  - "Parity mismatches between Python and Rust output (v1 project ID case)"
  - "Sentry flooded with individual events instead of grouped categories"
  - "Per-request settings loading in fork-based workers"
root_cause: "Multiple integration gaps between PyO3/Rust and Python runtime semantics"
tags:
  - pyo3
  - rust-python-integration
  - shadow-execution
  - exception-handling
  - case-normalization
  - sentry-fingerprinting
  - parallel-execution
  - settings-caching
  - check-digit
  - parity-testing
related: []
---

# PyO3 Shadow Execution Parity: Exception Handling, Case Normalization, and Sentry Grouping

## Problem

During Phase 1 of the ARK Resolver migration (Python to Rust), adding parallel shadow execution to the redirect route revealed several integration issues:

1. **Silent panic swallowing**: PyO3's `PanicException` inherits from `BaseException`, not `Exception`. The parallel executor's `except Exception` catch missed Rust panics entirely, allowing them to propagate and crash the request handler — defeating the purpose of shadow execution.

2. **Case normalization mismatch**: Python's `ArkUrlInfo.__init__` calls `.upper()` on v1 project IDs (e.g., `"080e"` -> `"080E"`). The Rust implementation preserved original case, causing parity mismatches in generated redirect URLs.

3. **Sentry event flooding**: Each unique ARK URL input created a separate Sentry issue. In production with thousands of daily requests, this would generate hundreds of meaningless issues instead of a handful of actionable categories.

4. **Wasteful settings loading**: The parallel executor needs Rust settings for every request. Loading settings per-request (file read or HTTP fetch) is prohibitively slow. Sanic's fork-based workers need settings cached before fork.

## Investigation

### PyO3 BaseException Discovery

1. Initial `execute_parallel` caught `Exception` for both Python and Rust paths.
2. During redirect route integration testing, a Rust panic propagated past the catch block, crashing the handler.
3. Checking PyO3 docs confirmed: `PanicException` is a `BaseException` subclass (analogous to `SystemExit`).
4. Widened catch to `BaseException`, but this also catches `KeyboardInterrupt`/`SystemExit` — must re-raise those immediately.
5. CI's RUF100 rule later flagged the `# noqa: BLE001` as unnecessary once the re-raise guard made the catch intentional.

### v1 Case Normalization

1. Parity test for `"ark:/00000/1/080e"` (lowercase) produced different redirect URLs.
2. Traced Python code: `.upper()` at `ark_url.py:97` for v1, `ark_url.py:119` for v0.
3. Checked Rust: v0 already had `.to_uppercase()` at line 93, but v1 was missing it.
4. One-line fix resolved all case-related parity failures.

### Check Digit Independence (Smoke Test Design)

1. Parity tests use NAAN `00000` (local test registry), smoke tests use NAAN `72163` (staging).
2. Question: would changing NAAN invalidate check digits?
3. Read `check_digit.py`: the algorithm operates only on resource ID base64url characters — NAAN and project ID are not inputs.
4. This allowed reusing the same resource IDs across NAANs, adapting 13 of 17 parity test cases for the smoke test.

## Root Cause

Multiple integration gaps between PyO3/Rust and Python runtime semantics:

- **Exception hierarchy**: PyO3's design choice to derive `PanicException` from `BaseException` (not `Exception`) is intentional but creates a trap for Python code that expects all catchable errors to be `Exception` subclasses.
- **Case normalization**: Python normalized early (at parse time), Rust preserved original case. The v0 path had already been ported correctly — only the v1 path was missed.
- **Sentry defaults**: Sentry's default fingerprinting works well for application errors but not for validation/shadow execution events where input variety creates noise.

## Solution

### 1. PyO3 BaseException Handling

```python
# ark_resolver/parallel_execution.py:112-118
try:
    rust_result = rust_func(*args, **kwargs)
except BaseException as e:  # Catch BaseException for PyO3 PanicException safety
    if isinstance(e, (KeyboardInterrupt, SystemExit)):
        raise
    rust_error = e
    self.logger.warning(f"Rust execution failed for {operation}: {e}", exc_info=True)
```

Type annotations reflect this: `rust_error: Optional[BaseException]` vs `python_error: Optional[Exception]`.

### 2. v1 Case Normalization in Rust

```rust
// src/core/use_cases/ark_url_info_processor.rs:51-52
// BR: Uppercase v1 project IDs to match Python parity (v0 already uppercases at line 92)
let project_id = components.1.map(|id| id.to_uppercase());
```

### 3. Custom Sentry Fingerprinting

```python
# ark_resolver/parallel_execution.py:190-207
if execution_result.comparison in (ComparisonResult.MISMATCH, ComparisonResult.RUST_ERROR):
    with sentry_sdk.push_scope() as scope:
        scope.fingerprint = ["shadow", execution_result.operation, execution_result.comparison.value]
        scope.set_tag("shadow.operation", execution_result.operation)
        scope.set_tag("shadow.comparison", execution_result.comparison.value)
        scope.set_context("shadow_details", {
            "python_result": str(execution_result.python_result)[:500],
            "rust_result": str(execution_result.rust_result)[:500],
        })
        sentry_sdk.capture_message(
            f"Shadow {execution_result.comparison.value}: {execution_result.operation}",
            level="warning",
        )
```

Route error handlers use the same pattern: `scope.fingerprint = ["redirect", "invalid-ark-id"]`.

### 4. Settings Caching

```python
# ark_resolver/ark.py:223-230 — cache at startup (before fork)
app.config.rust_settings = load_settings_rust()

# ark_resolver/routes/redirect.py:47-49 — read in worker
rust_settings = _.app.config.rust_settings
if rust_settings is None:
    raise RuntimeError("Rust settings not available")
```

Graceful degradation: if Rust settings fail to load, `app.config.rust_settings = None` and shadow execution raises `RuntimeError`, caught by the `BaseException` handler and logged as `RUST_ERROR`.

## Prevention

### Test Cases

- **PyO3 exception handling**: Test that `BaseException` subclasses (simulated panics) are caught while `KeyboardInterrupt`/`SystemExit` propagate. See `tests/test_sentry_fingerprinting.py`.
- **Case normalization parity**: Parametrized tests with both uppercase and lowercase project IDs. See `tests/test_redirect_parity.py` ("project-uppercase" and "project-lowercase" cases).
- **Sentry fingerprinting**: Mock `sentry_sdk` and assert fingerprint structure. See `tests/test_sentry_fingerprinting.py` (5 tests).
- **Parity matrix**: 17 parametrized cases covering all ARK URL variants (top-level, project, resource, value, v0 salsah, with timestamps). Run both Python and Rust implementations and assert identical output.

### Patterns to Adopt

1. **FFI exception handling**: Always catch `BaseException` for PyO3 calls, immediately re-raise `KeyboardInterrupt`/`SystemExit`.
2. **Normalize at parse time**: Apply case normalization during parsing, not during comparison. Document case-sensitivity rules per identifier type.
3. **Scoped Sentry fingerprinting**: Use `push_scope()` with semantic fingerprints `[category, operation, result_type]` for any validation/shadow execution events.
4. **Write-once-read-many config**: Initialize shared state before worker fork, provide graceful reload that preserves cached version on failure.

### Anti-patterns to Avoid

1. `except Exception` for PyO3 code — misses `PanicException`.
2. Relying on input-case preservation when the counterpart normalizes — always check both code paths.
3. Default Sentry fingerprinting for high-cardinality validation events — creates unmanageable noise.
4. Per-request settings loading in fork-based workers — use startup caching instead.
5. Assuming `# noqa` directives are permanent — CI's RUF100 rule serves as a cleanup reminder.

## References

- PR #140: Phase 1 Redirect Route Shadow Execution
- `ark_resolver/parallel_execution.py` — Core parallel execution framework
- `ark_resolver/routes/redirect.py` — Shadow execution integration
- `src/core/use_cases/ark_url_info_processor.rs` — v1 case normalization
- `tests/test_redirect_parity.py` — 17 parity tests
- `tests/test_sentry_fingerprinting.py` — 5 fingerprinting tests
- `tests/smoke_test.rs` — 14 redirect parity smoke test cases
