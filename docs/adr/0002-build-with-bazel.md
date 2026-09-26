# ADR-0002: Build, test and ship ark-resolver with Bazel

## Status

**Proposed** - 2026-09-27

## Context

ark-resolver is a hybrid Python (Sanic) / Rust (PyO3) service: Rust functions are compiled as the
Python extension module `ark_resolver._rust` and imported by the Python server. Today this hybrid
is built with four separate tools: `cargo`/`cargo-nextest` for the Rust crate and its unit tests,
`maturin` for the PyO3 extension, `uv` for the Python dependencies and virtualenv, and an Alpine
Dockerfile that runs all three in sequence. CI resolves the Python side against 3.13 while the
production image runs 3.12, and there is no single build graph that ties Rust, the extension, and
Python together or caches across them.

The CLAUDE.md migration plan for this repo describes an eventual pure-Rust Axum service replacing
Sanic. That rewrite is not being pursued as part of this decision; the service stays the Python +
PyO3-extension hybrid, and the hybrid itself is what gets built with Bazel.

DaSCH's sipi repository already builds a comparable C++/native-extension service with Bazel
(bzlmod), pinned to rules_rust 0.70.0, Rust 1.89.0, and a hermetic LLVM 0.8.18 toolchain. Aligning
ark-resolver's Bazel setup with sipi's conventions and pins is a deliberate choice: it keeps both
repos on the same toolchain generation ahead of a possible future move into a shared monorepo, and
lets fixes and upgrades be applied identically in both places.

## Decision

We adopt Bazel (bzlmod) as the build, lint, test and audit tool for ark-resolver's Rust crate and
its PyO3 extension, matching sipi's toolchain pins (rules_rust 0.70.0, Rust 1.89.0, hermetic LLVM
0.8.18) rather than the versions the previous Cargo/Docker setup used (Rust 1.97.0 in the Docker
image, 1.88.0 in `rust-toolchain.toml`). The pin is raised from here if a crate needs a newer
compiler.

Rust crate dependencies are resolved with crate_universe's `crate.from_specs` into a single
`@crates` hub, seeded from the previous `Cargo.lock` so resolved versions match exactly.
`Cargo.Bazel.lock` (Cargo lock file format, not MODULE.bazel.lock's JSON) replaces `Cargo.lock` as
the checked-in lockfile; `cargo audit` is pointed at it directly (`cargo audit --file
Cargo.Bazel.lock`) since cargo-audit only understands Cargo lock syntax.

### PyO3 extension build

- The extension is built with the `pyo3_extension` rule from the `rules_rust_pyo3` 0.70.0 module
  (the same release train as rules_rust 0.70.0), declared in the root `BUILD.bazel` package with
  `module_name = "ark_resolver._rust"` so the built artifact lands at `ark_resolver/_rust.so`,
  matching where the Python side imports it. `stubs = False`. `compilation_mode = "current"` is
  set explicitly because the rule's default "opt" transition places the `.so` outside `bazel-bin`,
  breaking the local dev workflow that expects it next to the Python package.
- Lint aspects are run against `//:_rust_shared` directly from the command line rather than via
  `lint_config` on the `pyo3_extension` target: `lint_config` on this rule fails Bazel analysis
  until rules_rust PR #4256 lands (unreleased at the time of this decision).
- Toolchains: two PyO3 toolchains must be registered. `@rules_rust_pyo3//toolchains:toolchain`
  (the pyo3_toolchain, wired to the rules_python CPython 3.12 toolchain) is only registered by
  rules_rust_pyo3 as a dev dependency of its own repo, so a consuming module has to register it
  itself; every `@crates//:pyo3` consumer needs it because the `pyo3-ffi` and `pyo3-build-config`
  build scripts are annotated to read this toolchain instead of probing a host `python3`. Second,
  we register our own `//bazel/pyo3:rust_pyo3_toolchain`, binding `@crates//:pyo3` 0.29 and
  `@crates//:pyo3-introspection` 0.29, in place of rules_rust_pyo3 0.70.0's bundled PyO3 0.28.2.
- We did not take rules_rust_pyo3 0.71.0+ (which bundles PyO3 0.29): it requires rules_rust
  >= 0.71.0, ahead of sipi's 0.70.0 pin, and it builds PyO3 with the `abi3-py311` limited API,
  which the current maturin-based build does not use. Building our own toolchain on top of
  rules_rust_pyo3 0.70.0 gets PyO3 0.29 without moving either of those two things.
- The Cargo `extension-module` feature is deprecated in PyO3 0.29; its replacement is read from an
  environment variable at build-script time, so a single `@crates//:pyo3` cannot serve both the
  extension build and the Rust unit tests, which embed an interpreter via `Python::initialize()`.
  We build `pyo3` without extension-module mode; the extension then links libpython dynamically,
  which the previous maturin build did not do. Verified: the built `.so` imports correctly under
  the hermetic CPython 3.12 toolchain, and `otool -L` shows it linked against `libpython3.12`. If
  this ever fails to load in the production image, the documented fallback is a second, isolated
  `pyo3` hub built in extension-module mode via crate_universe's experimental `isolate` feature.
- The Cargo `build.rs` that previously called `add_extension_module_link_args()` is dropped: it
  only emitted `-undefined dynamic_lookup` on macOS, which `pyo3_extension` already adds itself
  (together with `-Wl,-no_fixup_chains`) via a `select`.
- Rust unit tests (`//src:unit_tests`) link `@rules_python//python/cc:current_py_cc_libs`; on
  macOS no `PYTHONHOME` or library-path environment variable is needed because the resulting
  binary carries an `@loader_path` rpath. This has not been proven hermetic under remote
  execution; the fallback if it breaks there is an `sh_test` wrapper that sets `PYTHONHOME`
  explicitly. All 168 Rust unit tests pass under Bazel, matching the count under cargo-nextest.

## Consequences

### Positive
- One build graph and one dependency lockfile (`Cargo.Bazel.lock`) drive the Rust crate, its unit
  tests, and the PyO3 extension, replacing three separately invoked tools (cargo, cargo-nextest,
  maturin).
- Toolchain and rules_rust pins are shared with sipi, so upgrades and fixes transfer directly
  between the two repos.
- The extension no longer depends on maturin's PyO3 packaging conventions, so its link behavior
  (module name, link flags) is explicit and visible in `BUILD.bazel` rather than inferred by a
  separate tool.

### Negative
- The extension now links libpython dynamically instead of statically, a behavior change from the
  previous maturin build; this needs revisiting if it ever fails to load in the production image.
- Dependabot cannot open PRs against `crate.from_specs` entries in `MODULE.bazel` the way it could
  against `Cargo.toml`/`Cargo.lock`, so dependency bumps and audits are manual for now. `just
  crates-repin` (re-resolve and re-pin `@crates`) and `just audit` (`cargo audit --file
  Cargo.Bazel.lock`) are the stopgap until an automated alternative is in place.

### Neutral
- Rust unit tests' hermeticity under remote execution is unverified for macOS's rpath-based
  approach; the `sh_test`/`PYTHONHOME` fallback described above is documented but not yet needed.

## References

- [sipi repository](https://github.com/dasch-swiss/sipi): source of the rules_rust 0.70.0 /
  Rust 1.89.0 / hermetic LLVM 0.8.18 toolchain pins this ADR aligns with.
- [rules_rust PyO3 documentation](https://bazelbuild.github.io/rules_rust/rust_pyo3.html)
- [rules_rust PR #4256](https://github.com/bazelbuild/rules_rust/pull/4256): fixes
  `lint_config` analysis failure on `pyo3_extension`, not yet released.
- [quarylabs/sqruff](https://github.com/quarylabs/sqruff): worked example of a `pyo3_extension`
  build under Bazel.
- [ADR-0001: Adopt Hexagonal Architecture for Python-to-Rust Migration](0001-adopt-hexagonal-architecture.md)
