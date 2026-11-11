# Code Review Guidelines

Agent reference card for the **review phase**. Pair with `CONVENTIONS.md` (work phase). Component boundaries and banned constructs are in `ARCH-MAP.md`.

## Always check

### Served and shadow parity

- A change to resolution logic in `ark_resolver/ark_url.py` or `ark_resolver/check_digit.py` has the matching change in `src/core` and its adapters, or the PR says why both still agree. Name the ARK or IRI that would make them diverge.
- New or changed resolution behaviour has a comparative test feeding both implementations the same input (`tests/test_redirect_parity.py`, `tests/test_convert_parity.py`, `tests/test_check_digit_rust.py`).
- A route returns the Python result from `execute_parallel`. A route that returns, or branches on, the Rust result lets a Rust divergence reach users.
- Routes and the served modules never `import ark_resolver._rust`; they reach Rust through a `*_rust.py` wrapper.

### Identifiers

- A changed ARK or IRI construction, or a regex over identifiers, is checked against the ARK specification and the templates in `tests/ark-registry.ini`: NAAN, check digit, version and timestamp parts, URL encoding.
- `check_digit.py` and `src/core/domain/check_digit.rs` agree on alphabet and weighting.
- Legacy salsah.org ARKs map to resource IRIs through UUID v5 in the DaSCH namespace `cace8b00-717e-50d5-bcb9-486f39d733a2`. Any change to that mapping breaks every published legacy ARK.

### Security

- `POST /reload` verifies the `X-Hub-Signature` HMAC before it reloads anything; `GET /config` never returns the GitHub secret.
- A redirect target comes from a registry template; request input reaches the `Location` header only as a validated ARK part.

### Rust layering

- `src/core` stays free of `std::env` and `std::fs`; new I/O goes through a port in `src/core/ports` (ADR-0001). `pyo3` and any other crate outside `//src:core`'s `deps` fail the build, so they need no review check.
- A new entry in `//src:core`'s `deps` does no I/O and binds to no runtime; a new entry in `//src:core_unit_tests`' `deps` is a test-only crate.

### Configuration and build

- A new or renamed environment variable is in each loader that reads it, in the `EnvVarDefinition` table (`src/adapters/environment/env_logger.rs`) and in `.env.example`.
- `app.config.settings` and `app.config.rust_settings` are assigned only in `server()` and `reload_config()`.
- A `crate.spec` change in `MODULE.bazel` comes with the repinned `Cargo.Bazel.lock`.
- No `Cargo.toml`, `maturin` or `Dockerfile` build path comes back: Bazel is the only build (ADR-0002).

## Style

- A business rule in code carries a `BR: ` comment, with the same sentence in both implementations.
- Comments follow `CONVENTIONS.md` § Comments: none restates the code or narrates the session.
- Domain terms are the `CONTEXT.md` ones ("Served implementation", "Shadow implementation"), not their _Avoid_ alternatives.
- Each commit subject reads as a changelog line: the repo merges by rebase, so every commit reaches `CHANGELOG.md`.

## Skip

- `CHANGELOG.md`, `version.txt` and `.github/release-please/manifest.json`: release-please writes them.
- The content of `Cargo.Bazel.lock`, `MODULE.bazel.lock`, `uv.lock` and `flake.lock`: review the manifest change that caused the diff instead.
- `docs/specs/`: dated plans and journals, not a description of the current code.
- Formatting-only diffs that `just check` accepts.
