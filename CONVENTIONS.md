# ark-resolver Conventions

Agent reference card for the **work phase**. Pair with `REVIEW.md` (review phase). Authoritative detail lives in `ARCH-MAP.md` (components, boundaries, banned constructs), `CONTEXT.md` (vocabulary) and `docs/adr/`.

## Stack

Python 3.12 with Sanic serves HTTP and the user-facing resolution logic. Rust (edition 2021) is built as the PyO3 extension `ark_resolver._rust` and runs as the shadow implementation. Bazel with bzlmod builds everything: `rules_python` from `uv.lock`, `rules_rust` with crates declared as `crate.spec` in `MODULE.bazel` and locked in `Cargo.Bazel.lock`, `rules_rust_pyo3`, and `rules_oci` for the distroless image. The Nix dev shell provides bazel, just, uv and cargo-audit. Python is checked by ruff and pyright, Rust by rustfmt and clippy with `-Dwarnings`.

## Naming Conventions

| Entity | Convention | Example |
|--------|-----------|---------|
| Served Python module | `<feature>.py` | `check_digit.py`, `ark_url.py` |
| Shadow wrapper over the Rust extension | `<feature>_rust.py`, exposing the same public names as the served module | `check_digit_rust.py` (`is_valid`, `calculate_check_digit`) |
| Rust core feature | one file per feature, same stem in `src/core/domain/`, `errors/` and `ports/` | `src/core/errors/check_digit.rs` |
| Rust use case | `<feature>_<role>.rs` in `src/core/use_cases/` | `check_digit_validator.rs` |
| Rust error type | one `<Feature>Error` enum per feature; a new variant names the failed condition | `CheckDigitError::InvalidCharacter`, `ArkUrlInfoError::ProjectIdRequired` |
| Python test module | `tests/test_<module>.py`; parity tests `tests/test_<route>_parity.py` | `test_ark_url_rust.py`, `test_redirect_parity.py` |
| Domain terms | the `CONTEXT.md` term, never its listed _Avoid_ alternatives | "Served implementation", not "primary" |

## Served and Shadow Implementations

- Python is the served implementation; Rust runs beside it as the shadow implementation and is only compared and reported (`execute_parallel` in `parallel_execution.py`). A route never returns the Rust result.
- New resolution behaviour is implemented twice: in Python, and in Rust core plus adapters. It is exposed to Python through the feature's `*_rust.py` wrapper, run through `execute_parallel`, and lands with a comparative parity test.
- A change to one implementation changes the other in the same PR, or the PR says why both still agree and names the input that would show a divergence.
- Migration phases: in phase 1 (now) Rust runs in shadow while Python is user-facing; in phase 2 the served implementation switches to Rust and Python is removed where the migration is complete.
- Rust layering follows ADR-0001: `src/core` uses only the standard library, never `pyo3`, `std::env` or `std::fs`. I/O goes behind a trait in `src/core/ports` implemented in `src/adapters`.
- A new Python-visible Rust function or class is registered in `fn _rust` (`src/lib.rs`) and re-exported from the feature's `*_rust.py` wrapper. Routes and `python-resolution` modules never import `ark_resolver._rust` directly.
- A new route is a Blueprint module in `ark_resolver/routes/`, registered in `ark.py` before `redirect_bp`, which catches every path.

## Business Rules

A business rule embedded in code carries a `BR: ` comment that states the rule, so `grep -rn "BR: "` lists every rule in both languages. The same rule carries the same sentence in both implementations:

```python
# BR: A project or top-level ARK names no resource, so it has no resource IRI.
```

(`ark_resolver/ark_url.py` and `src/core/use_cases/ark_url_info_processor.rs`). Give requirements to an agent in the same form: the rule and its constraint, not the implementation.

## Configuration Pattern

- Every setting is an environment variable; `.env.example` lists each one with its default.
- The `ARK_*` server variables are read twice: by Python `load_settings()` in `ark.py` and by Rust `EnvironmentVariableProvider`. A new variable goes into each loader that reads it, into the `EnvVarDefinition` table in `src/adapters/environment/env_logger.rs` when Rust reads or logs it, and into `.env.example`.
- The registry INI named by `ARK_REGISTRY` (a path or an http(s) URL) is parsed separately by Python and Rust. `tests/ark-registry.ini` is the only test fixture.
- `app.config.settings` and `app.config.rust_settings` are written only by `server()` and `reload_config()`, so the two languages' settings cannot drift apart.

## Error Handling

- Python resolution raises `ArkUrlException` or a subclass (`ark_url.py`). A route turns an exception into a response with `classify_exception` and `error_response` from `error_diagnostics.py`; a new failure kind adds an `ArkErrorCode` rather than a hand-built response.
- Rust returns the feature's `thiserror` enum from `src/core/errors/`; the PyO3 adapter turns it into a Python exception. The types do not all match the served module: Rust `ArkUrlInfo` raises `ArkUrlException`, Rust `ArkUrlFormatter` raises `ValueError`, and `check_digit_rust.py` has its own `CheckDigitException`. The parity tests compare whether each side raised, not the type, so a route must never branch on the Rust exception type.

## Testing Conventions

- Rust unit tests live in the module under `#[cfg(test)]` and run with `just test` (`//src:unit_tests`).
- Python tests are `tests/test_*.py` with pytest. `tests/BUILD.bazel` globs each module into its own `py_test`, so a new file runs without a BUILD edit. Run them with `just pytest`.
- Tested means the behaviour is pinned in the implementation that changed and in a comparative test that feeds both implementations the same input (`test_redirect_parity.py`, `test_convert_parity.py`, `test_check_digit_rust.py`).
- Test data comes from `tests/ark-registry.ini`. A test that reaches the network is tagged `requires-network` and `external` in `tests/BUILD.bazel`, as `test_http_registry_rust` is.

## Commit Conventions

- Conventional Commits. The repo merges by rebase, so every commit lands on `main` and release-please turns each one into a CHANGELOG line (sections in `.github/release-please/config.json`; `ci` is not listed and stays out of the changelog). Write each subject as a changelog entry, one logical change per commit.
- Types in use: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build` (Bazel, just, Nix, image), `ci`, `chore`, `style`. The scope names the area: `fix(redirect)`, `fix(deps)`, `build(bazel)`, `docs(adr)`.
- The version lives in `version.txt` and only release-please changes it and `CHANGELOG.md`.

## Reviewers & Git History

Before opening a PR, read `main`'s `git log` and recent merged PRs. Use them for two things: to confirm the commit-message and PR-title conventions above hold in practice, and to find reviewers, the people who have recently authored or committed changes to the files this PR touches. This repo keeps no `CODEOWNERS`, so git history is the reviewer signal, and it is the ground truth for PR-title format wherever the Commit Conventions section above doesn't cover an edge case.

## Before Committing

- `just fmt`, then `just check` (ruff format check, ruff, pyright; rustfmt and clippy), `just test` and `just pytest`. All must be clean.
- After adding or renaming a Rust export: `just build`, so pyright sees the new `ark_resolver/_rust.so`.
- After a `crate.spec` change in `MODULE.bazel`: `just crates-repin`, then `just audit`.
- After a change to the image, the interpreter or Python dependencies: `just smoke-test`.

## Where to Go for Depth

- `ARCH-MAP.md`: components, dependency direction, banned constructs.
- `CONTEXT.md`: domain vocabulary (Served and Shadow implementation, ARK parts).
- `docs/adr/0001-adopt-hexagonal-architecture.md` and `docs/adr/0002-build-with-bazel.md`.
- `docs/learnings/integration-issues/pyo3-rust-python-shadow-execution-parity.md`: parity pitfalls across the PyO3 boundary.
- `README.md`: routes, deployment and the CLI.

## Comments

- A comment states what the reader must not break, never what the session discovered. Test: would it still be true and useful for someone who never saw the change that added it?
- Keep: an invariant a reader would otherwise break, a non-obvious third-party contract (lifetime, ownership, error/return semantics), the public API contract. One or two sentences each.
- Move out: what a change fixed or a test caught → commit body or PR; probe tables, benchmarks, corpus counts → `docs/` or a learning; a rejected alternative → a docs page or ADR, leaving one line and a link in the source.
- Delete: restatements of the code below, history ("previously", "now uses", "was changed to"), and any REQ id, user story, or plan-phase reference. A `TODO` without an issue id belongs in the tracker. A doc comment on an item a caller reaches is not a restatement: it is API surface, measured against what a caller needs rather than against the line below it.
- A PR description describes the code and the diff, never the commit history. "Commit 1 did X, commit 2 fixed Y" describes the journey.
- A "why" longer than about five lines belongs in a file; the comment becomes a pointer to it. A block past ~12 lines is a routing signal.
- Delete by default, when adding and when trimming: per sentence, name what a reader breaks without it, cut the ones with no answer, unclear included, and hold each survivor to one or two sentences.
- Doc comments use Google-style docstrings in Python (ruff's pydocstyle `google` convention, `D417` enforced) and `///` for items, `//!` for modules in Rust.
