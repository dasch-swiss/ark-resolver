---
title: "refactor: Build the hexagonal core as its own Bazel target without pyo3"
type: refactor
date: 2026-11-11
author: "Marcin Procyk"
status: implemented
repositories: []
linear: DEV-7853
branch: feature/dev-7853-ark-resolver-build-the-hexagonal-core-as
---

# refactor: Build the hexagonal core as its own Bazel target without pyo3

## Overview

`src/core` becomes its own Rust crate, `ark_resolver_core`, built by a new `rust_library` `//src:core` whose `deps` hold four crates and no `pyo3`. A `pyo3`, `reqwest`, `config` or non-test `tokio` import in the core, or a reference to `crate::adapters`, then fails the build instead of waiting for a reviewer. The two things that stop the core from compiling without `pyo3` and `config` today are removed first. This closes DUNE-001 from the 2026-10-20 readiness baseline.

## Problem Statement / Motivation

ADR-0001 says the core never depends on PyO3 or on infrastructure. Nothing enforces it: `//src:ark_resolver_lib` compiles `src/core` and `src/adapters` as one crate with `@crates//:pyo3` and `@crates//:config` in its `deps`, so both are importable from any core file. ARCH-MAP.md records the rule as `docs-only` and lists the violations that already slipped in. An agent that needs environment or file access in the core copies those instead of adding a port.

What the code shows today (branch base `3571ac4`):

| Violation | Where | Finding |
|---|---|---|
| `pyo3::Python::initialize()` in domain tests | `src/core/domain/uuid_processing.rs:110`, `:127` | The functions under test are pure Rust. The calls do nothing for the assertions and are the only reason `//src:unit_tests` needs libpython for core code. |
| `From<config::ConfigError> for SettingsError` | `src/core/errors/settings.rs:69` | Unused. The only `config` user is `src/adapters/common/ini_processor.rs`, which builds `SettingsError::ParseError` with `map_err`. |
| `From<std::io::Error>`, `From<std::env::VarError>`, `From<regex::Error>` for `SettingsError` | `src/core/errors/settings.rs:57`, `:63`, `:75` | Unused outside their own two tests (`test_settings_error_from_regex_error`, `test_settings_error_from_env_error`). Every adapter site already maps explicitly. |
| `regex` in the domain although ADR-0001 says std only | `src/core/domain/parsing.rs`, `domain/settings.rs`, `ports/settings.rs` | Real use. Decided: `regex` stays, and the rule is restated (see Proposed Solution). |

So no conversion moves anywhere: the four `From` impls are dead code and are deleted. Rust's orphan rule would forbid moving them into an adapter crate in any case, since both the trait and `SettingsError` would be foreign there.

## Proposed Solution

1. Delete the two `pyo3::Python::initialize()` calls and the four `From` impls with their two tests. Each is its own commit and builds in today's single-crate layout.
2. Make `src/core` a crate. `src/core/mod.rs` is renamed to `src/core/lib.rs` and becomes the crate root of `rust_library` `//src:core` (`crate_name = "ark_resolver_core"`). `//src:ark_resolver_lib` and the `pyo3_extension` `//:_rust` drop the core sources and depend on `//src:core` instead.
3. The core's dependency rule becomes its `deps` list. Decided with the owner: the allowlist is `regex`, `thiserror`, `serde_json` and the proc-macro `async-trait`, the four crates the core uses today. ADR-0001 and CONVENTIONS.md are amended in place from "std only" to "only the crates in `//src:core`'s `deps`"; adding a crate there is a reviewed `BUILD.bazel` change.
4. Raise the enforcement level in ARCH-MAP.md and remove the "Current violations" list.

### Target layout after the change

```starlark
# src/BUILD.bazel
filegroup(
    name = "core_srcs",          # src/core/**, explicit list, private
    srcs = ["core/lib.rs", "core/domain/mod.rs", ...],
)

filegroup(
    name = "srcs",               # src/adapters/** and lib.rs only
    srcs = ["adapters/...", "lib.rs"],
    visibility = ["//:__pkg__"],
)

rust_library(
    name = "core",
    srcs = [":core_srcs"],
    crate_name = "ark_resolver_core",
    crate_root = "core/lib.rs",
    edition = "2021",
    proc_macro_deps = ["@crates//:async-trait"],
    # This list is ADR-0001's dependency rule for the core. No pyo3, no
    # I/O crate: file, environment and HTTP access go through src/core/ports.
    # Adding a crate here changes ADR-0001's rule; never add one to get I/O.
    deps = [
        "@crates//:regex",
        "@crates//:serde_json",
        "@crates//:thiserror",
    ],
    visibility = ["//:__pkg__"],  # //:_rust, the pyo3_extension
)

rust_test(
    name = "core_unit_tests",
    crate = ":core",
    deps = ["@crates//:tokio"],   # #[tokio::test] in ports and use cases
)

rust_library(
    name = "ark_resolver_lib",
    srcs = [":srcs"],
    deps = [":core", "@crates//:pyo3", ...],   # unchanged otherwise
)
```

`//:_rust` in the root `BUILD.bazel` gains `"//src:core"` in `deps` and keeps `srcs = ["//src:srcs"]`.

### Path changes

```rust
// src/core/**: the crate root is now src/core, so the `core::` segment goes
use crate::core::errors::check_digit::CheckDigitError;   // before
use crate::errors::check_digit::CheckDigitError;         // after

// src/adapters/**: the core is an external crate
use crate::core::errors::settings::{SettingsError, SettingsResult};        // before
use ark_resolver_core::errors::settings::{SettingsError, SettingsResult};  // after
```

`src/lib.rs` drops `pub mod core;`. Count the sites with `grep -rn 'crate::core::' src/core | wc -l` and `grep -rn 'crate::core::' src/adapters src/lib.rs | wc -l`; both return 0 afterwards.

## Alternative Approaches Considered

- **Cheapest shape: keep every path.** A wrapper root file (`pub mod core;`) for the new crate plus `use ark_resolver_core::core;` in `src/lib.rs` leaves all `crate::core::` lines untouched. Rejected: adapter code would still read `crate::core::…`, identical to the pre-split form, so the crate boundary this change exists to create is invisible at the call site. A `pub(crate)` item added in the core then fails in adapters with an error that contradicts what the path says. The explicit form costs one mechanical rewrite.
- **Remove `regex` from the core as well** (literal ADR-0001). Rejected for this issue: `CompiledRegexes`, `RegexProvider` and the parsing functions return `regex::Regex`, so it redesigns the settings and parsing ports and touches code under shadow comparison. `regex` does no I/O, and the allowlist catches what the rule is for.
- **A checked-in negative build test** that expects a `pyo3` import to fail. Rejected: re-adding `pyo3` to `//src:core`'s `deps` is a visible, reviewed BUILD diff, and the test would only restate that attribute.

## Technical Considerations

- **No behaviour change.** No Python-visible signature, error type or message changes. `SettingsError`'s variants and `Display` text stay; only the unused `From` impls go. The parity tests under `//tests/...` are the regression check for the PyO3 boundary.
- **Test-only mocks stay in the core crate.** `src/core/ports/settings.rs` exposes its mocks from a `#[cfg(test)] pub mod tests`, used only by `src/core/use_cases/settings_manager.rs`. No adapter uses them, so the split does not need a test-support crate.
- **`//src:core_unit_tests` needs no libpython, no `data` and no `env`.** Core tests read the registry path through `MockEnvironmentProvider`. `//src:unit_tests` keeps `current_py_cc_libs`, `data` and `env`: it still links `pyo3` and its adapter tests read `ARK_REGISTRY`.
- **No repin.** No `crate.spec` changes, so `Cargo.Bazel.lock` and `MODULE.bazel.lock` stay as they are.
- **Lint coverage.** `just rustcheck` names its targets on the command line, so the new library and test target must be added to both aspect invocations or the core's `#[cfg(test)]` code stops being linted.
- **What the build still cannot see.** `std::env`, `std::fs` and `std::io` are part of std, so a `deps` list cannot ban them. That half of the rule stays at `review` (REVIEW.md, Rust layering). `tokio` is in `core_unit_tests`' `deps`, so it resolves inside `#[cfg(test)]` code only; a non-test use fails `//src:core`.
- **The lint aspects do not propagate through `deps`** in rules_rust 0.70.0, so `//src:core` and `//src:core_unit_tests` are named explicitly in `rustcheck`; otherwise the core is neither formatted-checked nor linted.
- **The rule is stated once.** ADR-0001 holds the decision and the `deps` list holds the crates. CONVENTIONS.md, REVIEW.md and ARCH-MAP.md point to those two and do not repeat the crate names, so a later change to the allowlist touches one BUILD attribute.
- **ARCH-MAP.md lags by two commits.** Only `dune:dune-map` writes it, so its statements about the single target are corrected in commit 5; the `just test` mentions in CLAUDE.md and CONVENTIONS.md change with commit 3.
- **Commits.** The repo rebase-merges and commit-lint checks every commit. Planned subjects, in order, each building and passing on its own:
  1. `test: drop the Python interpreter start-up from the UUID domain tests`
  2. `refactor: remove the unused From conversions on SettingsError`
  3. `build(bazel): build the hexagonal core as its own target without pyo3`
  4. `docs: state the core's dependency rule as the //src:core deps list`
  5. `docs(arch-map): record the core split and raise the layering rule to structure`

## Implementation Approach

**Baseline**

- [x] Run `bazel test //src:unit_tests --test_output=all --nocache_test_results` on the unchanged branch and note the number of tests the binary reports; this is the baseline named in Success Metrics

**Commit 1: UUID domain tests without PyO3**

- [x] Delete the two `pyo3::Python::initialize();` lines in the tests of `src/core/domain/uuid_processing.rs`
- [x] Update the `unit_tests` comment in `src/BUILD.bazel`: libpython is linked because the test binary links `pyo3`, no longer because of `uuid_processing.rs`
- [x] Update the comment on the `pyo3` `crate.spec` in `MODULE.bazel`: the unit-test binary links `pyo3` and so needs libpython; no test calls `Python::initialize()` any more
- [x] Run `just test && just pytest`; it passes

**Commit 2: unused conversions**

- [x] Delete the four `impl From<…> for SettingsError` blocks in `src/core/errors/settings.rs` (`regex::Error`, `std::io::Error`, `config::ConfigError`, `std::env::VarError`)
- [x] Delete `test_settings_error_from_regex_error` and `test_settings_error_from_env_error` from the same file
- [x] If the build names a `?` site that relied on one of the impls, give that site an explicit `map_err` to the same `SettingsError` variant with the same message text, in the adapter
- [x] Run `just check`; it passes
- [x] Run `just test && just pytest`; it passes

**Commit 3: the split**

- [x] `git mv src/core/mod.rs src/core/lib.rs`
- [x] Add a short `//!` crate doc to `src/core/lib.rs`: I/O, environment and HTTP access go through `ports/` and are implemented in `src/adapters`; the crates the core may use are the `deps` of `//src:core` in `src/BUILD.bazel`, and adding one is a reviewed change to ADR-0001's rule
- [x] In `src/core/**`, rewrite `crate::core::` to `crate::`
- [x] In `src/adapters/**`, rewrite `crate::core::` to `ark_resolver_core::`
- [x] Remove `pub mod core;` from `src/lib.rs`
- [x] In `src/BUILD.bazel`, split the source list into a private `core_srcs` filegroup (`core/**`, with `core/lib.rs`) and `srcs` (adapters and `lib.rs`)
- [x] Put a one-line comment on each filegroup saying which directory's files it lists, so a new file lands in the right list
- [x] Add `rust_library` `core` to `src/BUILD.bazel` as sketched above, visible to `//:__pkg__`, with the comment that its `deps` are the core's dependency rule
- [x] Add `rust_test` `core_unit_tests` (`crate = ":core"`, `deps = ["@crates//:tokio"]`)
- [x] Add `":core"` to the `deps` of `ark_resolver_lib`
- [x] Update the module docstring of `src/BUILD.bazel` to describe the four targets
- [x] Add `"//src:core"` to the `deps` of `pyo3_extension` `_rust` in the root `BUILD.bazel`
- [x] In `justfile`, add `//src:core` and `//src:core_unit_tests` to both `bazel build` lines of `rustcheck`
- [x] In `justfile`, add `//src:core_unit_tests` to `test`
- [x] `CLAUDE.md`, Development Commands: update the `just test` comment to the two targets, so it is true in this commit
- [x] `CONVENTIONS.md`, Testing Conventions: name both test targets behind `just test` (`//src:core_unit_tests`, `//src:unit_tests`)
- [x] Negative check: add `use pyo3::prelude::*;` to `src/core/domain/check_digit.rs`, run `bazel build //src:core`, confirm it fails with an unresolved `pyo3` crate error, then remove the line
- [x] Negative check: add `use crate::adapters;` to the same file, run `bazel build //src:core`, confirm it fails, then remove the line
- [x] `git diff --exit-code src/core/domain/check_digit.rs` exits 0 after both negative checks
- [x] `grep -rn 'crate::core::' src` returns nothing
- [x] Run `just fmt`, then `just check`; it passes
- [x] Run `just test && just pytest`; it passes, and the tests reported by `//src:core_unit_tests` plus `//src:unit_tests` equal the baseline in Success Metrics minus 2
- [x] Run `just build`; `ark_resolver/_rust.so` is produced
- [x] If `//src:core_unit_tests` fails to link on Linux with undefined `_Unwind_*` symbols, add the `select` on `//:_libunwind_static` that `//tools/healthcheck` uses to `core_unit_tests`' `deps` (never to `//src:core`) and widen that target's visibility in the root `BUILD.bazel` to `//src:__pkg__`

**Commit 4: the rule in the docs**

- [x] `docs/adr/0001-adopt-hexagonal-architecture.md`: amend the Domain Layer bullet "Zero external dependencies beyond Rust std library" to say the core (domain, use cases, ports, errors) depends only on the crates in `//src:core`'s `deps`, none of which does I/O or binds to Python, and that the build enforces it
- [x] `docs/adr/0001-adopt-hexagonal-architecture.md`: add a dated line under Status, "Amended 2026-11-11: the core may use the crates in `//src:core`'s `deps`; the rest of the decision stands", so the relaxation from std only reads as a decision
- [x] `docs/adr/0001-adopt-hexagonal-architecture.md`: add `core/lib.rs` to the File Structure block
- [x] `CONVENTIONS.md`, Served and Shadow Implementations: replace "`src/core` uses only the standard library, never `pyo3`, `std::env` or `std::fs`" with one line that points to ADR-0001 and `//src:core`'s `deps` without listing the crates, and keeps `std::env` and `std::fs` as banned
- [x] `REVIEW.md`, Rust layering: keep `std::env` and `std::fs` as the review check, say `pyo3` and other crates are caught by the build, and add the check that a new entry in `//src:core`'s `deps` does no I/O and that `core_unit_tests`' `deps` hold test-only crates
- [x] `docs/adr/0002-build-with-bazel.md`: name `//src:core_unit_tests` beside `//src:unit_tests` where the Rust unit tests are listed, and say it links no libpython
- [x] `docs/adr/0002-build-with-bazel.md`: reword the sentence that the Rust unit tests embed an interpreter via `Python::initialize()`; they link `pyo3` but start no interpreter
- [x] `grep -rn 'Python::initialize' --include='*.bazel' --include='*.md' --include='*.rs' . | grep -v docs/specs` returns nothing
- [x] `grep -rn 'standard library\|std library' CONVENTIONS.md REVIEW.md CLAUDE.md docs/adr` shows no remaining "std only" claim about the core

**Commit 5: ARCH-MAP.md**

- [x] Run `dune:dune-map` in update mode; `rust-core`, `rust-adapters`, `build-and-delivery` and `project-docs` are re-fingerprinted and every component is fresh
- [x] `rust-core` entry: paths include `src/core/lib.rs`, "Depends on" names the crates as "the `deps` of `//src:core`" without listing them, the "Current violations" list is gone
- [x] `rust-core` local-context kit: `src/BUILD.bazel` replaces `src/adapters/pyo3/settings.rs`, keeping the kit at seven files
- [x] `rust-adapters` boundary rules: one line that the core's test mocks are `#[cfg(test)]` and not visible to adapter tests
- [x] `rust-core` boundary rule: `crate::adapters`, `pyo3`, `reqwest` and any crate outside `//src:core`'s `deps` are Enforcement `structure`; `std::fs` and `std::env` are Enforcement `review`
- [x] Conventions, "Dependency direction (Rust)": Enforcement `structure`, and the sentence about a future split is removed
- [x] Banned constructs: the `pyo3`, `std::env` or `std::fs` row is split so `pyo3` reads `structure` and `std::env`/`std::fs` read `review`; both rows keep the alternative "add a port in `src/core/ports` and implement it in `src/adapters`"
- [x] `build-and-delivery` entry: the explicit-source-list rule names both `//src:core_srcs` and `//src:srcs`, and Key entities include `//src:core` and `//src:core_unit_tests`
- [x] The spec-sink check `grep -rnE "docs/specs/[0-9]{4}" --exclude-dir=specs --exclude-dir=.git .` returns nothing
- [x] Run `just check && just test && just pytest`; it passes
- [x] Run `just commit-lint origin/feature/dev-7852-ark-resolver-repo-hygiene-editorconfig`; it passes

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Run the execution where `bazel` is on the PATH (the Nix dev shell). The planning machine has neither `bazel` nor `nix`; without it every build and negative check is a CI round trip of several minutes | Repo owner | before start | Installing or entering a toolchain on the owner's machine is the owner's call |
| H2 | Merge #211, then #212, then this PR. After each parent merges, have the branch rebased with `git rebase --onto origin/main <old parent tip>` and force-pushed with lease before merging it | Repo owner | after ship | Merging is the owner's decision |

## Acceptance Criteria

- [x] `//src:core` builds with `deps` of exactly `regex`, `serde_json`, `thiserror` and the proc-macro `async-trait`
- [x] A `pyo3` import in `src/core` fails `bazel build //src:core` (shown once by the negative check)
- [x] `src/core` contains no `pyo3`, no `config::` and no `From<std::io::Error>`, `From<std::env::VarError>` or `From<regex::Error>` on `SettingsError`
- [x] `just check && just test && just pytest` passes, with the Rust test total equal to the baseline minus 2 (166 = 119 + 47; locally `just check`'s `just --fmt` step fails on version skew of the unpinned `just`, every other step passes)
- [x] `just rustcheck` lints `//src:core` and `//src:core_unit_tests`
- [x] ADR-0001, CONVENTIONS.md and REVIEW.md state the same rule as the `deps` list, and ARCH-MAP.md records it as `structure` with no "Current violations"
- [x] Five commits, each a valid Conventional Commit that builds and passes on its own

## Dependencies & Risks

- **Stacked on #212, which is stacked on #211.** CONVENTIONS.md and REVIEW.md exist only in #211, and #212 rewrote the ARCH-MAP.md lines this plan edits. The branch starts at the #212 tip (`3571ac4`) and its PR targets the #212 branch. The commit-lint job takes its base from the PR, so it counts only this branch's commits. A review change in a parent that touches the same doc lines means a rebase conflict here (H2).
- **`pyo3_extension` with a first-party `rust_library` dep is new in this repo.** `//:_rust` has only taken `@crates` deps and a `cc_import` so far. If the extension's configuration differs from the library's, the core is compiled twice, which costs build time and nothing else. The image build is exercised by CI's smoke test.
- **A hidden user of a deleted `From` impl.** Reading found none; the compiler is the proof. The fallback is the explicit `map_err` deliverable in commit 2.
- **macOS and Linux differ in how test binaries link.** `//src:core_unit_tests` links no libpython on either; CI covers Linux.
- **Residual gap.** `std::env` and `std::fs` in the core stay review-enforced. A possible follow-up, not in this plan: clippy's `disallowed-methods` for the std I/O entry points, with adapters opting out. It is unverified whether rules_rust's clippy configuration can be scoped that way.
- **`//src:core_unit_tests` on Linux.** The hermetic LLVM links with `--unwindlib=none`. `//src:unit_tests` and `//tests:smoke_test` link without the static libunwind, so the new test is expected to as well; the fallback is the last deliverable of commit 3.

## Success Metrics

- Baseline: the number of tests `//src:unit_tests` reports at `3571ac4`, measured by the first deliverable (168 when last recorded at the Bazel cutover). After the change, `//src:core_unit_tests` plus `//src:unit_tests` report the baseline minus 2.
- The readiness check's `architecture-adherence` no longer reports DUNE-001.
- ARCH-MAP.md lists no current violation for `rust-core`.

## References

- `src/BUILD.bazel`, root `BUILD.bazel` (`pyo3_extension` `_rust`), `justfile` (`rustcheck`, `test`)
- `src/core/errors/settings.rs:57-79`, `src/core/domain/uuid_processing.rs:110`, `:127`, `src/core/ports/settings.rs:107`
- `docs/adr/0001-adopt-hexagonal-architecture.md`, `docs/adr/0002-build-with-bazel.md`
- `ARCH-MAP.md` (`rust-core`, Conventions, Banned constructs), `CONVENTIONS.md`, `REVIEW.md`
- `docs/learnings/integration-issues/pyo3-rust-python-shadow-execution-parity.md`: the parity tests are the regression check for the PyO3 boundary
