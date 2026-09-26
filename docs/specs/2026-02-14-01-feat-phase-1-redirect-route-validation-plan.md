---
title: "Phase 1: Redirect Route Validation"
type: feat
date: 2026-02-14
author: "Ivan Subotic"
status: implemented
linear: DEV-5871
repositories: []
prd: "2026-02-13-01-ark-resolver-migration-completion-PRD.md"
branch: feature/dev-5871-phase-1-redirect-route-validation
---

# Phase 1: Redirect Route Validation

## Enhancement Summary

**Deepened on:** 2026-02-14
**Reviewed on:** 2026-02-14, 2026-02-15
**Sections enhanced:** 9
**Research agents used:** repo-research-analyst, learnings-researcher, best-practices-researcher, security-reviewer, performance-reviewer, rust-patterns-explorer
**Review agents used:** specification-reviewer (code reference verification)

### Key Improvements
1. **URL encoding parity verified** — Both Python (`quote(safe="")`) and Rust (`urlencoding::encode()`) encode everything including `/`. No slash-encoding mismatch. Remaining risk: percent-encoding case (`%2f` vs `%2F`).
2. **PanicException handling** — PyO3 `PanicException` derives from `BaseException`, not `Exception`. The ParallelExecutor must catch `BaseException` for the Rust path, not just `Exception`.
3. **Convert route bonus fix** — While caching Rust settings for redirect, also update the convert route to use `app.config.rust_settings` instead of per-request `load_settings_rust()`. Eliminates tokio runtime creation per convert request.
4. **Sentry error grouping** — Sentry analysis (262 errors/30 days) reveals 100+ separate issues that should be ~6 grouped categories. Invalid ARK IDs create one issue per unique ARK. Add custom fingerprinting for both redirect and convert routes.
5. **Timestamp handling verified** — The `parsing.rs:15` FIXME is misleading. Rust handles timestamps via string splitting before regex matching (`settings.rs:222-236`). All integration tests pass. Not a gap.

### New Considerations Discovered
- Settings reload is non-atomic (two separate assignments for Python + Rust settings) — spurious mismatches possible during the reload window
- `ARK_GITHUB_SECRET` defaults to empty string — webhook security concern (not Phase 1 scope, but noted)
- `send_default_pii=True` in current Sentry config sends PII — PRD already flags this for Phase 2 fix
- INI processor normalizes project IDs to lowercase for lookup (`ini_processor.rs:92-93`) while the v1 case fix uppercases for display — these are independent operations, no conflict

### Learnings Applied
- **ADR-0001**: Hexagonal architecture — understand the ports/adapters layering when locating the v1 case fix
- **BR: Project configurations are case-insensitive and stored in lowercase** (`ini_processor.rs:92-93`) — lookup uses `.to_lowercase()`, display uses the parsed project_id. The case fix changes what gets stored in `ArkUrlInfo.project_id` for v1, which flows into template substitution for the redirect URL.
- **BR: Use generous timeouts** (`http/mod.rs:25`) — HTTP timeouts configured at 5s connect / 10s total, plus 15s application-level timeout via `ARK_RUST_LOAD_TIMEOUT_MS`
- **IPv4/IPv6 container fix** — `ARK_RUST_FORCE_IPV4=true` env var exists for container environments with broken IPv6. Keep in mind for production deployment.

---

## Overview

Add parallel (shadow) execution to the ARK Resolver's main redirect route (`/<path:path>`), running the Rust implementation alongside Python for every redirect request. This validates behavioral parity on the primary production traffic path before the Python-to-Rust cutover in Phase 2.

The convert route (`/convert/<ark_id>`) already uses the `ParallelExecutor` for shadow execution. Phase 1 replicates this exact pattern for the redirect route, plus fixes a known parity bug and caches Rust settings at startup.

## Problem Statement / Motivation

The redirect route handles the vast majority of production traffic (permanent ARK URL resolution for academic publications), but currently runs **only Python** — no Rust shadow validation. This is the critical gap identified in the PRD. Without parallel validation on the redirect route, we cannot confidently replace Python with Rust in Phase 2.

The convert route has been running with parallel execution in production, proving the pattern works. Phase 1 extends this to the redirect route so that both routes are validated before the Rust cutover.

## Proposed Solution

Follow the established `ParallelExecutor` pattern from the convert route. Five changes to the codebase (in implementation order):

1. **Fix Sentry error grouping** (Step 0) — Add custom fingerprinting so invalid ARK IDs group by error category (not by individual ARK ID). Apply to both redirect and convert routes. Must be done before adding shadow execution.
2. **Fix the v1 project ID case-sensitivity parity bug** (Step 1) — Python uppercases, Rust preserves case
3. **Cache Rust settings at startup** (Step 2, PRD decision D2) — eliminate per-request HTTP fetch
4. **Wire parallel execution into the redirect route** (Step 3) — Python remains primary, Rust runs as shadow
5. **Add explicit parity tests** (Step 4) — comparative tests asserting Python == Rust for all redirect scenarios

**Bonus (low-effort):** Also update the convert route to use cached `app.config.rust_settings` instead of per-request `load_settings_rust()`, eliminating tokio runtime creation on every convert request.

## Technical Considerations

### Known Parity Bug: Project ID Case Sensitivity (v1 ARK URLs)

The specification review uncovered a **confirmed parity discrepancy** in v1 project ID handling:

- **Python** (`ark_url.py:97`): `self.project_id = match.group(2).upper()` — always uppercases v1 project IDs
- **Rust** (`ark_url_info_processor.rs:49-51`): does NOT uppercase v1 project IDs — only v0 (`components.0.to_uppercase()` at line 92)

**Impact**: For input `ark:/00000/1/080e`:
- Python produces: `http://meta.dasch.swiss/projects/080E` (uppercased)
- Rust produces: `http://meta.dasch.swiss/projects/080e` (lowercase preserved)

This will cause **mismatches in production** if not fixed. The fix must be applied to the Rust v1 parsing path (`ArkUrlInfoProcessor::parse_ark_id` or equivalent) to match Python's behavior. The existing Rust test `test_ark_url_case_insensitive_project` already asserts the wrong (lowercase) behavior — it must be updated to match Python's expected output.

**Fix location**: `src/core/use_cases/ark_url_info_processor.rs` — add `.to_uppercase()` to the v1 project_id extraction, matching `ark_url.py:97`.

#### Research Insights

**How case flows through the system:**
1. ARK URL input: `ark:/00000/1/080e`
2. Regex capture: project_id = `080e` (from `parsing.rs` `PROJECT_ID_PATTERN: [0-9A-Fa-f]{4}`)
3. **Python**: `.upper()` → `080E` stored in `ArkUrlInfo.project_id`
4. **Rust (current)**: no transformation → `080e` stored in `ArkUrlInfo.project_id`
5. Config lookup: `settings.get_project_config("080e")` uses `.to_lowercase()` internally (`settings.rs:133`) — works for both cases
6. Template substitution: uses `project_id` as stored → different redirect URLs

**No conflict with INI normalization**: The INI processor stores sections lowercase (`ini_processor.rs:92-93`), and `get_project_config()` normalizes the lookup key to lowercase (`settings.rs:133`). The case fix changes what goes into the *redirect URL template*, not the config lookup key. These are independent paths.

**Python v0 also uppercases** (`ark_url.py:119`): `self.project_id = match.group(1).upper()` — Rust v0 already matches this behavior at `ark_url_info_processor.rs:92`.

### Settings Caching (D2)

The convert route currently calls `load_settings_rust()` on every request (`convert.py:50`), which creates a new tokio runtime and fetches config from HTTP each time. This is wasteful and adds latency.

For Phase 1, Rust settings are:
- Loaded once at startup alongside Python settings
- Stored as `app.config.rust_settings`
- Reloaded in `reload_config()` alongside Python settings
- If loading fails, set to `None` — redirect runs Python-only (graceful degradation)

#### Research Insights

**Sanic `app.config` safety:**
- Sanic uses cooperative multitasking on a single event loop per worker. Within a single worker, there are no true threads competing for `app.config` access.
- **Safe pattern**: Write once at startup (`before_server_start`), read-only from handlers. This is exactly the proposed approach.
- **Reload race**: `reload_config()` replaces `app.config.settings` and `app.config.rust_settings` in two separate assignments. A request reading between the two assignments could see new Python + old Rust settings (or vice versa), potentially causing a spurious mismatch logged to Sentry. The window is tiny and the impact is a single false-positive Sentry event — acceptable.
- **Multi-worker**: With `Sanic.start_method = "fork"` (`ark.py:49`), each forked worker gets its own `app.config` copy. No cross-process sharing concerns.

**Tokio runtime lifecycle:**
- `load_settings_rust()` creates a new `tokio::runtime::Runtime` each call (`settings.rs:72`), spawns a thread pool, runs async work, then drops the runtime.
- At startup (called once): ~1-10ms one-time cost — negligible.
- For reload (called on webhook): same one-time cost — acceptable.
- The Rust settings object is a plain data structure after loading — no ongoing runtime dependency.

### Performance

The `ParallelExecutor` runs Python and Rust sequentially (not in parallel threads) — Rust execution adds ~1-5ms overhead per request based on convert route observations. This is acceptable for a redirect service where latency tolerance is high (HTTP 302 redirects).

#### Research Insights

**Sequential vs truly parallel execution:**
- The `ParallelExecutor.execute_parallel()` runs Python first, then Rust, sequentially within the async handler (`parallel_execution.py:95-118`). During both executions, the Sanic event loop is blocked.
- Truly parallel execution via `asyncio.to_thread()` would reduce wall time from `python_time + rust_time` to `max(python_time, rust_time)` — but adds complexity and thread-pool overhead for a ~1-5ms saving.
- **Verdict**: Sequential is correct for Phase 1. The goal is validation, not performance optimization. The ~1-5ms overhead is negligible for redirect latency.

**Memory overhead:**
- Two copies of settings (Python ConfigParser + Rust `ArkUrlSettings`) in memory. The registry INI is small structured data (~20-50 projects). Overhead is well under 100KB. No concern.

### Error Handling

The `ParallelExecutor` guarantees:
- Python result always returned to the user (`parallel_execution.py:100-150`)
- Rust errors are caught and logged, never propagated to the user
- If Python raises, the exception is re-raised after recording metrics
- Mismatches are logged to Sentry as warnings (`parallel_execution.py:186-195`)

The redirect route's existing error handling (`ArkUrlException`, `CheckDigitException`, `KeyError`) remains unchanged — `ParallelExecutor.execute_parallel()` re-raises Python exceptions.

Note: Rust wraps all errors into `ArkUrlException` via the PyO3 adapter (`ark_url_info.rs:47,71,85`), while Python raises distinct exception types (`ArkUrlException`, `CheckDigitException`, `KeyError`). The `ParallelExecutor._compare_results` classifies both-failing as `BOTH_ERROR` regardless of error type — this is acceptable for Phase 1 monitoring since we care about result parity, not error type parity.

#### Research Insights

**PyO3 PanicException handling (IMPORTANT):**
- When Rust code panics, PyO3 catches the panic via `catch_unwind` and converts it to `pyo3_runtime.PanicException`.
- `PanicException` derives from **`BaseException`**, not `Exception`. A bare `except Exception` will NOT catch Rust panics.
- **Verify**: Check that the `ParallelExecutor`'s Rust execution catch block (`parallel_execution.py:108-118`) catches `BaseException` or at minimum `Exception` plus `pyo3_runtime.PanicException`. If it only catches `Exception`, a Rust panic would propagate upward and crash the request handler.
- The existing convert route works because Rust panics in the current codebase are extremely unlikely (no `unwrap()` on user input). But defense-in-depth says catch `BaseException` for the Rust shadow path.

**Error type mapping:**
- Rust errors map through PyO3 adapter (`ark_url_info.rs:61-72`): all Rust errors → `ArkUrlException` with `.to_string()` message
- Python raises 3 distinct types: `ArkUrlException`, `CheckDigitException`, `KeyError`
- `BOTH_ERROR` classification doesn't compare error types — this is fine for Phase 1

### URL Encoding Parity

The redirect URL construction involves URL encoding (Python `urllib.parse.quote` vs Rust `urlencoding::encode`). These must produce identical output for resource IRI strings. The parity tests (Step 4) will catch any divergence. If differences are found (e.g., percent-encoding case `%2f` vs `%2F`), normalize comparison in the test assertions.

#### Research Insights

**VERIFIED — encoding is compatible (risk downgraded):**

Code review confirmed both sides encode with no safe characters:
- **Python** (`ark_url.py:230`): `parse.quote(resource_iri, safe="")` — encodes everything including `/`
- **Rust** (`ark_url_info.rs:267-269`): `urlencoding::encode(input)` — also encodes everything

| Character | Python `quote(safe="")` | Rust `urlencoding::encode()` |
|-----------|------------------------|-------------------------------|
| `/` | Encoded as `%2F` | Encoded as `%2F` |
| `=` | Encoded as `%3D` | Encoded as `%3D` |
| `-` | Not encoded | Not encoded |
| `~` | Not encoded | Not encoded |

Both encode everything beyond RFC 3986 unreserved characters. The only remaining risk is percent-encoding case (`%2f` vs `%2F`) — parity tests in Step 4 will catch this if it exists.

**Note**: The initial risk assessment assumed Python used the default `safe='/'` parameter. Verification showed the actual code explicitly uses `safe=""`, making the encoding compatible.

### Reload Failure Handling

When `reload_config()` is triggered by the GitHub webhook and Rust settings reload fails:
- **Preserve the old cached Rust settings** — do not set to `None`
- Log a warning with the error details
- Python settings reload failure already causes the server to use stale Python settings (existing behavior)

#### Research Insights

**Non-atomic reload concern:**
- `reload_config()` performs two separate assignments: `app.config.settings = ...` then `app.config.rust_settings = ...`
- Between these assignments, a concurrent request could read new Python settings + old Rust settings
- This could cause a spurious mismatch event in Sentry (new config redirects differently from old config)
- **Mitigation**: Accept as known edge case. The reload window is sub-millisecond. Sentry fingerprinting (see below) will group these. Not worth adding complexity for atomicity.

### Timestamp Handling (Verified — Not a Gap)

The `FIXME` comment at `src/core/domain/parsing.rs:15` is **misleading** — it documents a design choice, not a bug.

**How Rust handles timestamps:** The v1 ARK regex intentionally excludes timestamps. Instead, `CompiledRegexes::match_ark_path()` (`settings.rs:222-236`) extracts timestamps **before** regex matching via string splitting:

```rust
let (processed, timestamp) = if let Some(index) = ark_path.find('.') {
    (&ark_path[..index], Some(&ark_path[index + 1..]))
} else {
    (ark_path, None)
};
self.ark_path_regex.captures(processed).map(|captures| {
    // ... extract captures ...
    timestamp.map(|m| m.to_string()),  // Return timestamp separately
})
```

**Why this approach:**
- Avoids complex nested optional capture groups in the regex
- Prevents capture group numbering issues
- Simpler and more maintainable than Python's approach (which embeds timestamps in the regex)
- **All integration tests pass**, including timestamp scenarios (e.g., `test_ark_url_info_redirect_resource` with `.20180604T085622513Z`)

**The Rust domain model fully supports timestamps:** `ArkUrlInfo` has a `timestamp: Option<String>` field with `get_timestamp()`, `has_timestamp()` helper methods.

**Parity tests must still cover timestamps** — they validate that both Python and Rust produce identical timestamped redirect URLs. The commented-out unit tests in `parsing.rs:77-123` are for regex-level matching and remain irrelevant since timestamps are handled at a higher level.

### Sentry Error Analysis (from production)

**262 errors in the last 30 days** across **100+ separate Sentry issues** — but most should be grouped.

**Current error distribution:**

| Category | Issues | Events | Example |
|----------|--------|--------|---------|
| **Project not found** (`KeyError`) | 2 | 137 | `KeyError: '0113'` — project not in registry |
| **HTML entity corruption** | ~30 | ~60 | `...080E/t8t0IDxjQ3yg_gcx6BsJSAG&lt` |
| **SEO spam injection** | ~20 | ~30 | `...084C;DaSCH;https:/www.thegameroom.org/...` |
| **Truncated resource IDs** | ~15 | ~20 | `...0105/BtF`, `...0105/iAruZWuO` |
| **Trailing slash/dot** | ~10 | ~15 | `...080e-74814fdcbeca1-e/`, `...1/0111.` |
| **Other malformed** | ~5 | ~5 | `ark:/`, `ark:/72163/1/[` |
| **Convert route errors** | 0 | 0 | (none — clean) |
| **Parallel execution mismatches** | 0 | 0 | (none — convert parity validated) |

**Root cause of issue explosion:** Each error message includes the full ARK ID (`"Invalid ARK ID: ark:/72163/1/080E/..."`). Sentry fingerprints by message → each unique ARK = separate issue.

**Required fix:** Custom Sentry fingerprinting to group errors by category, not by individual ARK ID. This must be done BEFORE adding shadow execution to the redirect route, otherwise shadow execution mismatches will also create one issue per unique ARK.

## Implementation Approach

### Step 0: Fix Sentry Error Grouping (redirect + convert routes)

**Prerequisite** — must be done before adding shadow execution to the redirect route.

Currently, each invalid ARK ID creates a separate Sentry issue because the error message includes the full ARK ID. After shadow execution is added, mismatch events would have the same problem. Fix grouping for both routes.

**File**: `ark_resolver/routes/redirect.py`

Add Sentry fingerprinting to each error handler in `catch_all()`:

```python
import sentry_sdk

# In the ArkUrlException handler:
except ArkUrlException as ex:
    with sentry_sdk.push_scope() as scope:
        scope.fingerprint = ["redirect", "invalid-ark-id"]
        scope.set_tag("ark_id", ark_id_decoded[:100])  # tag for searchability, truncated
        scope.set_tag("error_type", "ArkUrlException")
        sentry_sdk.capture_exception(ex)
    span.set_status(Status(StatusCode.ERROR, "Invalid ARK ID"))
    logger.error(f"Invalid ARK ID: {ark_id_decoded}")
    return response.text(body=ex.message, status=400)

# In the CheckDigitException handler:
except check_digit_py.CheckDigitException as ex:
    with sentry_sdk.push_scope() as scope:
        scope.fingerprint = ["redirect", "check-digit-error"]
        scope.set_tag("ark_id", ark_id_decoded[:100])
        scope.set_tag("error_type", "CheckDigitException")
        sentry_sdk.capture_exception(ex)
    ...

# In the KeyError handler:
except KeyError as ex:
    with sentry_sdk.push_scope() as scope:
        scope.fingerprint = ["redirect", "project-not-found"]
        scope.set_tag("ark_id", ark_id_decoded[:100])
        scope.set_tag("project_id", str(ex)[:10])
        sentry_sdk.capture_exception(ex)
    ...
```

**File**: `ark_resolver/routes/convert.py`

Apply the same pattern to the convert route's error handlers:

```python
# In each except block:
with sentry_sdk.push_scope() as scope:
    scope.fingerprint = ["convert", "<error-category>"]
    scope.set_tag("ark_id", ark_id[:100])
    sentry_sdk.capture_exception(ex)
```

**File**: `ark_resolver/parallel_execution.py`

**Replace** the existing `track_with_sentry()` Sentry reporting with fingerprinted grouping. The current implementation uses `capture_message` without fingerprinting, causing one Sentry issue per unique input. The replacement wraps the same `capture_message` call with a `push_scope` context that sets a custom fingerprint:

```python
def track_with_sentry(self, result: ParallelExecutionResult) -> None:
    if result.comparison in (ComparisonResult.MISMATCH, ComparisonResult.RUST_ERROR):
        with sentry_sdk.push_scope() as scope:
            # BR: Group shadow execution events by operation + result type, not by individual input
            scope.fingerprint = ["shadow", result.operation, result.comparison.value]
            scope.set_tag("shadow.operation", result.operation)
            scope.set_tag("shadow.comparison", result.comparison.value)
            scope.set_context("shadow_details", {
                "python_result": str(result.python_result)[:500],
                "rust_result": str(result.rust_result)[:500],
                "python_duration_ms": result.python_duration_ms,
                "rust_duration_ms": result.rust_duration_ms,
            })
            sentry_sdk.capture_message(
                f"Shadow {result.comparison.value}: {result.operation}",
                level="warning"
            )
```

**Result after this step:**
- All redirect errors grouped into 3 Sentry issues: `redirect/invalid-ark-id`, `redirect/check-digit-error`, `redirect/project-not-found`
- All convert errors similarly grouped
- All shadow execution mismatches grouped into: `shadow/redirect/MISMATCH`, `shadow/redirect/RUST_ERROR`, `shadow/convert/MISMATCH`, `shadow/convert/RUST_ERROR`
- Individual ARK IDs searchable via tags

**Unit tests for Step 0:**
```python
# test_sentry_fingerprinting.py
from unittest.mock import MagicMock, patch

def test_redirect_invalid_ark_uses_fingerprint(mocker):
    """Verify invalid ARK IDs use custom fingerprinting, not message-based grouping."""
    mock_scope = MagicMock()
    mocker.patch("sentry_sdk.push_scope", return_value=mock_scope)
    mocker.patch("sentry_sdk.capture_exception")
    # Hit the redirect endpoint with an invalid ARK via Sanic test client
    # Assert: mock_scope.__enter__().fingerprint == ["redirect", "invalid-ark-id"]
    # Assert: mock_scope.__enter__().set_tag.called_with("ark_id", ...)
    # Assert: sentry_sdk.capture_exception.called

def test_parallel_executor_mismatch_uses_fingerprint(mocker):
    """Verify shadow mismatches use operation-based fingerprinting."""
    mock_scope = MagicMock()
    mocker.patch("sentry_sdk.push_scope", return_value=mock_scope)
    mocker.patch("sentry_sdk.capture_message")
    # Create a ParallelExecutionResult with MISMATCH comparison
    # Call track_with_sentry(result)
    # Assert: mock_scope.__enter__().fingerprint == ["shadow", "redirect", "MISMATCH"]
    # Assert: sentry_sdk.capture_message.called_with(level="warning")
```

### Step 1: Fix v1 Project ID Case Parity Bug

**File**: `src/core/use_cases/ark_url_info_processor.rs`

Find the v1 parsing path where `project_id` is extracted from regex groups (lines 49-51). Add `.to_uppercase()` to match Python's behavior at `ark_url.py:97`.

```rust
// Before (bug):
let project_id = components.1.clone(); // preserves case

// After (fix):
// BR: ARK v1 project IDs are case-insensitive; normalize to uppercase for consistent redirect URLs
let project_id = components.1.as_ref().map(|p| p.to_uppercase());
```

**Update test**: `tests/test_ark_url_rust.py:84-86` — change expected output from `"http://meta.dasch.swiss/projects/080e"` to `"http://meta.dasch.swiss/projects/080E"`.

#### Research Insights

**Verify both uppercase and lowercase test inputs:**
- The test file has cases for both `080E` (uppercase input) and `080e` (lowercase input) at lines 77-86
- After the fix, BOTH should produce redirect URLs with `080E` (uppercase project ID in URL)
- The config lookup still works because `get_project_config()` lowercases the key internally

**Also update Rust unit tests:**
- Run `cargo test --lib` to find any Rust-side unit tests that assert lowercase v1 project IDs
- These must be updated to expect uppercase after the fix

### ~~Step 1.5: Verify URL Encoding Parity~~ (Removed — redundant)

**Review finding:** Both Python (`quote(safe="")`) and Rust (`urlencoding::encode()`) encode everything including slashes. URL encoding parity is implicitly verified by the resource-level and value-level ARK IDs in Step 4's parity tests. No separate encoding test needed.

### Step 2: Cache Rust Settings at Startup

**File**: `ark_resolver/ark.py`

In `server()` (line 218), add Rust settings loading after Python settings. **Note:** The existing code assigns `app.config.settings = settings` directly in `server()` before `app.run()`, not in a `@before_server_start` hook. Follow the same pattern for `rust_settings` — load it synchronously in `server()` before `app.run()`:

```python
from ark_resolver._rust import load_settings as load_settings_rust

def server(settings: ArkUrlSettings) -> None:
    app.config.settings = settings
    # BR: Cache Rust settings at startup for parallel validation (D2)
    try:
        app.config.rust_settings = load_settings_rust()
        logger.info("Rust settings cached at startup.")
    except Exception as e:
        logger.warning(f"Failed to load Rust settings at startup: {e}")
        app.config.rust_settings = None
    app.run(host=settings.top_config["ArkInternalHost"],
            port=settings.top_config.getint("ArkInternalPort"))
```

In `reload_config()` (line 240), reload Rust settings alongside Python:

```python
def reload_config() -> None:
    settings = load_settings()
    app.config.settings = settings
    # BR: Reload Rust settings alongside Python settings (D2)
    # BR: Preserve old cached Rust settings on reload failure — don't degrade to None
    try:
        app.config.rust_settings = load_settings_rust()
        logger.info("Rust settings reloaded.")
    except Exception as e:
        logger.warning(f"Failed to reload Rust settings, keeping cached version: {e}")
    logger.info("Configuration reloaded.")
```

#### Research Insights

**Startup exception scope:**
- The `except Exception` at startup is intentionally broad. PyO3 maps Rust errors to Python exceptions (`settings.rs:112-121`): `FileSystemError` → `PyIOError`, `ParseError` → `PyValueError`, etc. All derive from `Exception`, so this catches everything meaningful.
- `BaseException` (which includes `KeyboardInterrupt`, `SystemExit`) should NOT be caught here — let those propagate.
- If the Rust code panics during settings loading, PyO3 raises `PanicException` (from `BaseException`) — this would NOT be caught by `except Exception`. This is acceptable at startup: if Rust panics loading settings, it's a build issue, not a graceful-degradation scenario. Log the failure and let the service start without Rust shadow.

**Bonus fix — update convert route too:**
- `convert.py:49-50` currently calls `load_settings_rust()` per request, creating a tokio runtime each time
- After Step 2 caches `app.config.rust_settings`, update the convert route's `rust_convert()` to use `req.app.config.rust_settings` instead
- This is a one-line change with meaningful latency reduction on the convert path

### Step 3: Add Parallel Execution to Redirect Route

**File**: `ark_resolver/routes/redirect.py`

Add imports and modify `catch_all()` to use the `ParallelExecutor`:

```python
from ark_resolver.ark_url_rust import ArkUrlInfo as ArkUrlInfoRust
from ark_resolver.parallel_execution import parallel_executor

@redirect_bp.get("/<path:path>")
async def catch_all(_: Request, path: str = "") -> HTTPResponse:
    if not path.startswith("ark:/"):
        msg = f"Invalid ARK ID: {path}"
        return response.text(body=msg, status=400)

    ark_id_decoded = unquote(path)

    with tracer.start_as_current_span("redirect") as span:
        span.set_attribute("ark_id", ark_id_decoded)

        try:
            # BR: Shadow execution validates Rust parity on redirect route (Phase 1)
            def python_redirect():
                return ArkUrlInfo(
                    settings=_.app.config.settings,
                    ark_id=ark_id_decoded
                ).to_redirect_url()

            def rust_redirect():
                rust_settings = _.app.config.rust_settings
                if rust_settings is None:
                    raise RuntimeError("Rust settings not available")
                return ArkUrlInfoRust(
                    rust_settings, ark_id_decoded
                ).to_redirect_url()

            redirect_url, execution_result = parallel_executor.execute_parallel(
                "redirect", python_redirect, rust_redirect
            )

            # Add parallel execution metrics to span
            parallel_executor.add_to_span(span, execution_result)

            # Track with Sentry
            parallel_executor.track_with_sentry(execution_result)

            span.set_status(Status(StatusCode.OK))

        # NOTE: Error handlers below already have Sentry fingerprinting from Step 0.
        # The fingerprinting code is omitted here for brevity — see Step 0 for the
        # full pattern. The handlers below show the structural changes only.

        except ArkUrlException as ex:
            # Step 0 fingerprint: ["redirect", "invalid-ark-id"]
            span.set_status(Status(StatusCode.ERROR, "Invalid ARK ID"))
            logger.error(f"Invalid ARK ID: {ark_id_decoded}")
            return response.text(body=ex.message, status=400)

        except check_digit_py.CheckDigitException as ex:
            # Step 0 fingerprint: ["redirect", "check-digit-error"]
            span.set_status(Status(StatusCode.ERROR, "Check Digit Error"))
            logger.error(
                f"Invalid ARK ID (wrong check digit): {ark_id_decoded}",
                exc_info=ex
            )
            return response.text(body=ex.message, status=400)

        except KeyError as ex:
            # Step 0 fingerprint: ["redirect", "project-not-found"]
            span.set_status(Status(StatusCode.ERROR, "KeyError (project not found)"))
            logger.error(
                f"Invalid ARK ID (project not found): {ark_id_decoded}",
                exc_info=ex
            )
            return response.text(body="Invalid ARK ID (project not found)", status=400)

        span.add_event("Redirecting", {"redirect_url": redirect_url})
        logger.info(f"Redirecting {ark_id_decoded} to {redirect_url}")
        return response.redirect(redirect_url)
```

#### Research Insights

**ParallelExecutor exception handling — VERIFIED, fix required:**
- `parallel_execution.py:114` catches `Exception` (confirmed: `except Exception as e:  # noqa: BLE001`)
- Rust panics produce `PanicException` which derives from `BaseException`, NOT `Exception`
- A Rust panic will escape the catch block and crash the request handler
- **Fix**: Change line 114 from `except Exception as e:` to `except BaseException as e:` for the Rust execution path
- This is defense-in-depth — Rust panics on user input are unlikely but possible (e.g., regex engine edge case)

**Sentry flooding mitigation:**
- If a systematic parity bug exists (like the v1 case issue before the fix), every redirect request will fire `capture_message`
- The `ParallelExecutor.track_with_sentry()` sends a message per mismatch (`parallel_execution.py:186-195`)
- **Recommended**: Use Sentry fingerprinting to group mismatches by operation:
  ```python
  with sentry_sdk.push_scope() as scope:
      scope.fingerprint = ["shadow-mismatch", operation]
      sentry_sdk.capture_message(...)
  ```
- This groups all redirect mismatches into a single Sentry issue instead of thousands
- Consider rate limiting: after N mismatches of the same type, switch to periodic sampling

### Step 3.5: Update Convert Route to Use Cached Settings (Bonus)

**File**: `ark_resolver/routes/convert.py`

Update the `rust_convert()` closure to use cached settings instead of per-request loading:

```python
# Before (per-request tokio runtime + HTTP fetch):
def rust_convert():
    rust_settings = load_settings_rust()
    ...

# After (uses cached settings):
def rust_convert():
    rust_settings = req.app.config.rust_settings
    if rust_settings is None:
        raise RuntimeError("Rust settings not available")
    ...
```

Remove the `from ark_resolver._rust import load_settings as load_settings_rust` import from convert.py if no longer needed.

#### Research Insights

**Performance impact:**
- Current: `load_settings_rust()` per request creates a tokio runtime (~1-10ms), potentially fetches config via HTTP (~50-200ms)
- After: Zero overhead — reads from cached Python object
- This is the single biggest latency win available in Phase 1, and it's a one-line change

### Step 4: Add Parity Tests

**New file**: `tests/test_redirect_parity.py`

Explicit comparative tests that run the same ARK IDs through both Python and Rust `to_redirect_url()` and assert identical results:

```python
"""
Comparative parity tests for redirect URL generation.
Runs the same inputs through Python and Rust ArkUrlInfo.to_redirect_url()
and asserts identical output.
"""
import os
import pytest

from ark_resolver.ark_url import ArkUrlInfo as PythonArkUrlInfo
from ark_resolver.ark_url import ArkUrlSettings as PythonSettings
from ark_resolver._rust import load_settings as load_settings_rust
from ark_resolver.ark_url_rust import ArkUrlInfo as RustArkUrlInfo


@pytest.fixture(scope="module")
def python_settings():
    """Load Python settings using the same pattern as the application."""
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    # Reuse the application's load_settings() which handles env defaults
    from ark_resolver.ark import load_settings
    return load_settings()


@pytest.fixture(scope="module")
def rust_settings():
    os.environ["ARK_REGISTRY"] = "tests/ark-registry.ini"
    return load_settings_rust()


# Parameterize with all redirect test scenarios
@pytest.mark.parametrize("ark_id", [
    "ark:/00000/1",                                              # top-level
    "ark:/00000/1/0003",                                         # project (default host)
    "ark:/00000/1/0004",                                         # project (custom host)
    "ark:/00000/1/080E",                                         # project (uppercase)
    "ark:/00000/1/080e",                                         # project (lowercase)
    "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn",               # resource
    "ark:/00000/1/0001/cmfk1DMHRBiR4=_6HXpEFAn.20180604T085622513Z",  # resource+ts
    "ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo",  # value
    "ark:/00000/0002-779b9990a0c3f-6e",                         # v0 salsah
    "ark:/00000/0002-779b9990a0c3f-6e.20190129",                # v0 salsah+ts
    "ark:/00000/080e-76bb2132d30d6-0",                          # v0 salsah lowercase
])
def test_redirect_url_parity(python_settings, rust_settings, ark_id):
    """Python and Rust must produce identical redirect URLs."""
    python_url = PythonArkUrlInfo(python_settings, ark_id).to_redirect_url()
    rust_url = RustArkUrlInfo(rust_settings, ark_id).to_redirect_url()
    assert python_url == rust_url, (
        f"Parity mismatch for {ark_id}:\n"
        f"  Python: {python_url}\n"
        f"  Rust:   {rust_url}"
    )
```

#### Research Insights

**Test coverage gaps to address:**

1. **Timestamp scenarios (extra attention required)**: The parity tests include multiple timestamp variants (v1 resource+timestamp, v0+timestamp). Rust handles timestamps via string splitting before regex matching (`settings.rs:222-236`), which is verified working. However, because the approach differs from Python's regex-embedded timestamps, **cover every timestamp path with dedicated unit tests**:
   - v1 resource with ISO timestamp (`20180604T085622513Z`)
   - v0 salsah with date-only timestamp (`20190129`)
   - Timestamp with different formats (verify both Python and Rust parse identically)
   - Edge: ARK with `.` in resource ID (ensure splitting doesn't misinterpret as timestamp separator)

2. **All projects in test registry**: The test covers projects 0001, 0003, 0004, 0005, 080E/080e, and v0 projects 0002 and 080e. Verify the test registry (`tests/ark-registry.ini`) has all these sections.

3. **Edge case — v0 with timestamp**: `ark:/00000/0002-779b9990a0c3f-6e.20190129` tests the v0 timestamp format. Verify Python's `get_timestamp()` behavior matches Rust's handling.

4. **Edge case — value ARKs**: `ark:/00000/1/0005/SQkTPdHdTzq_gqbwj6QR=AR/=SSbnPK3Q7WWxzBT1UPpRgo` has `=` characters in the resource and value IDs. These need proper URL encoding in the redirect URL — this is where the URL encoding parity matters most.

**Consider adding error parity tests:**
```python
@pytest.mark.parametrize("ark_id", [
    "ark:/00000/1/ZZZZ",           # unknown project
    "ark:/00000/1/0001/invalid",    # bad check digit
    "not-an-ark",                   # totally invalid
])
def test_error_parity(python_settings, rust_settings, ark_id):
    """Both Python and Rust should fail for invalid ARK IDs."""
    python_error = None
    rust_error = None
    try:
        PythonArkUrlInfo(python_settings, ark_id).to_redirect_url()
    except Exception as e:
        python_error = type(e).__name__
    try:
        RustArkUrlInfo(rust_settings, ark_id).to_redirect_url()
    except Exception as e:
        rust_error = type(e).__name__
    # Both should fail (type doesn't need to match, just both-error)
    assert (python_error is not None) == (rust_error is not None), (
        f"Error parity mismatch for {ark_id}:\n"
        f"  Python error: {python_error}\n"
        f"  Rust error:   {rust_error}"
    )
```

### Step 5: Verify Existing Tests Pass

Run the full test suite to confirm:
- `just build` — rebuild Rust extensions with the v1 case fix
- `just pytest` — all Python tests pass (including updated case-sensitivity expectation)
- `cargo test --lib` — all Rust unit tests pass
- New parity tests pass

#### Research Insights

**Test commands from justfile:**
- `just build` — builds Rust extensions with maturin
- `just pytest` — runs Python tests (requires build)
- `just test` — runs Rust unit tests (`cargo test --lib`)
- `just check` — runs rustcheck + pycheck (formatting + linting)
- `just fmt` — formats all code (rust + python)

**Run `just check` before committing** to ensure formatting and linting pass for both Python and Rust.

### Step 6: Update Documentation

Update `CLAUDE.md` to note that the redirect route now uses parallel execution (matching the existing convert route documentation).

## Acceptance Criteria

- [x] v1 project ID case-sensitivity bug fixed in Rust (uppercases to match Python)
- [x] Rust settings cached at startup in `app.config.rust_settings`
- [x] Rust settings reloaded in `reload_config()` alongside Python settings
- [x] Redirect route uses `ParallelExecutor.execute_parallel("redirect", ...)` for shadow execution
- [x] Parallel execution metrics added to OTel span via `add_to_span()`
- [x] Mismatches tracked in Sentry via `track_with_sentry()`
- [x] Graceful degradation: Rust settings=None -> Python-only execution (no user impact)
- [x] Explicit parity tests comparing Python and Rust redirect URLs for all scenarios
- [x] All existing tests pass (`just pytest`, `cargo test --lib`)
- [x] `just check` passes (formatting + linting for both Python and Rust)
- [x] Single PR on branch `feature/dev-5871-phase-1-redirect-route-validation`
- [ ] Deployed to production, monitored for 2 weeks — zero discrepancies gate for Phase 2
- [x] ParallelExecutor Rust catch block widened to `BaseException` for PanicException safety
- [x] Sentry error grouping: redirect errors grouped by category (`redirect/invalid-ark-id`, `redirect/check-digit-error`, `redirect/project-not-found`)
- [x] Sentry error grouping: convert errors grouped by category (matching redirect pattern)
- [x] Sentry error grouping: shadow execution mismatches grouped by `["shadow", operation, comparison]`
- [x] Unit tests verify custom Sentry fingerprinting is applied to each error handler (redirect, convert, ParallelExecutor)
- [x] Timestamp parity tests pass — both Python and Rust produce identical redirect URLs for timestamped ARKs
- [x] (Bonus) Convert route updated to use cached `app.config.rust_settings`

## Dependencies & Risks

**Dependencies:**
- None — Phase 1 has no dependencies on other phases

**Risks:**
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Undiscovered parity bugs beyond case sensitivity | Medium | Medium | Parallel execution + Sentry logging will surface them; 2-week soak period |
| Rust settings fail to load in production | Low | Low | Graceful degradation — redirect runs Python-only, logged as warning |
| Parallel execution adds noticeable latency | Low | Low | Sequential execution adds ~1-5ms; acceptable for redirect service |
| `reload_config()` webhook fails for Rust | Low | Low | Rust reload failure preserves old cached settings; logged for investigation |
| URL encoding differences (Python vs Rust) | Low | Low | **Verified compatible**: Both use no safe characters (`safe=""` / `urlencoding::encode`). Only percent-encoding case risk remains. Parity tests will catch. |
| Rust timestamp handling divergence | Low | Low | **Verified**: Rust handles timestamps via string splitting before regex (`settings.rs:222-236`). All integration tests pass. FIXME at `parsing.rs:15` is a design choice, not a bug. Parity tests cover timestamped ARKs explicitly. |
| Sentry flooding from systematic parity bug | Medium | Low | Use Sentry fingerprinting to group by operation; consider sampling after N events |
| PyO3 PanicException not caught | Low | Medium | Widen ParallelExecutor Rust catch to `BaseException`; Rust panics on user input are unlikely but possible |

## Success Metrics

1. Zero parity mismatches in Sentry after 2 weeks of production traffic
2. Rust shadow execution runs on 100% of redirect requests (no `rust_settings=None` after startup)
3. All existing tests continue to pass
4. New parity tests cover all redirect scenarios (top-level, project, resource, value, v0 salsah, case variants)

## Security Notes (from review)

Items noted during deepening — not in scope for Phase 1, but relevant context:

| Finding | Severity | Phase |
|---------|----------|-------|
| `ARK_GITHUB_SECRET` defaults to `""` — empty key allows trivially crafted HMAC | Critical | Phase 3 (webhook removed entirely) |
| Webhook HMAC uses SHA-1 instead of SHA-256 (`X-Hub-Signature-256`) | Warning | Phase 3 (webhook removed entirely) |
| `send_default_pii=True` sends request headers/IPs to Sentry | Warning | Phase 2 (PRD D7: change to `False`) |
| CORS wildcard on all routes including `/reload` and `/config` | Warning | Phase 2 (Axum CORS config) |
| `/config` endpoint unauthenticated, exposes routing config | Warning | Phase 2 (endpoint dropped per PRD D9) |
| Mismatch logging sends redirect URLs to Sentry (low PII risk) | Info | Acceptable — redirect URLs are not sensitive |

## References & Research

- **Convert route pattern**: `ark_resolver/routes/convert.py:42-63` — the exact pattern to replicate
- **ParallelExecutor API**: `ark_resolver/parallel_execution.py:75-150` — `execute_parallel()`, `add_to_span()`, `track_with_sentry()`
- **ParallelExecutor comparison logic**: `parallel_execution.py:218-231` — `MATCH`, `MISMATCH`, `RUST_ERROR`, `PYTHON_ERROR`, `BOTH_ERROR`
- **ParallelExecutor Sentry tracking**: `parallel_execution.py:170-195` — tags, measurements, custom events
- **Settings startup**: `ark_resolver/ark.py:218-223` — `server()` function, `reload_config()` at line 240
- **Rust ArkUrlInfo**: `src/adapters/pyo3/ark_url_info.rs:61-72` — `to_redirect_url()` method
- **Rust settings loader**: `src/adapters/pyo3/settings.rs:68-124` — `ArkUrlSettings::new()` with tokio runtime + 15s timeout
- **Case bug location**: `src/core/use_cases/ark_url_info_processor.rs:49-51` — v1 preserves case; line 92: v0 uppercases
- **Python case behavior**: `ark_resolver/ark_url.py:97` — `.upper()` on v1 project_id; line 119: `.upper()` on v0
- **INI case normalization**: `src/adapters/common/ini_processor.rs:92-93` — projects stored lowercase
- **Settings lookup normalization**: `src/core/domain/settings.rs:133` — `.to_lowercase()` on lookup key
- **Timestamp FIXME**: `src/core/domain/parsing.rs:15` — regex excludes timestamps
- **Existing Rust redirect tests**: `tests/test_ark_url_rust.py:53-148` — comprehensive redirect scenarios
- **Existing smoke test**: `tests/smoke_test.rs` — Docker-based integration test (483 lines)
- **ADR-0001**: Hexagonal architecture — ports/adapters pattern for testability
- **IPv4 container fix**: `src/adapters/http/mod.rs:72-76` — `ARK_RUST_FORCE_IPV4=true`
- **PRD**: `01 - PROJECTS/DaSCH/Technical/2025-07 ARK Resolver to Rust Migration/prds/2026-02-13-ark-resolver-migration-completion-PRD.md`
