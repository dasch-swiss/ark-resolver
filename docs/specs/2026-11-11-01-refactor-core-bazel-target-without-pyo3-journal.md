---
plan: docs/specs/2026-11-11-01-refactor-core-bazel-target-without-pyo3-plan.md
target_repo: <repo root>
base_commit: 88d726c
branch: feature/dev-7853-ark-resolver-build-the-hexagonal-core-as
started: 2026-11-11
problem: >
  ADR-0001 says the hexagonal core never depends on PyO3 or infrastructure, but nothing enforces it: //src:ark_resolver_lib compiles src/core and src/adapters as one crate with pyo3 and config in its deps, so both are importable from any core file, and violations already slipped in (pyo3 in domain tests, an unused From<config::ConfigError>). Splitting the core into its own rust_library with an allowlisted deps list turns the rule into a build failure (DUNE-001).
status: complete
---

# Execution Journal: 2026-11-11-01-refactor-core-bazel-target-without-pyo3-plan

## Chunk queue

| id | files | depends_on | checkboxes | acceptance | context | replaces |
|----|-------|------------|------------|------------|---------|----------|
| 0 | none | — | "Baseline" | test count of //src:unit_tests recorded | — | — |
| 1 | src/core/domain/uuid_processing.rs, src/BUILD.bazel, MODULE.bazel | 0 | "Commit 1" | no Python::initialize in core; just test && just pytest green | uuid_processing.rs tests; unit_tests comment; pyo3 crate.spec comment | — |
| 2 | src/core/errors/settings.rs | 1 | "Commit 2" | four From impls and two tests gone; just check, test, pytest green | settings.rs:57-79 and its tests | — |
| 3 | src/core/**, src/adapters/**, src/lib.rs, src/BUILD.bazel, BUILD.bazel, justfile, CLAUDE.md, CONVENTIONS.md | 2 | "Commit 3" | //src:core builds with four deps; negative checks fail as expected; test total = baseline - 2 | plan "Target layout" and "Path changes" | — |
| 4 | docs/adr/0001-*.md, docs/adr/0002-*.md, CONVENTIONS.md, REVIEW.md | 3 | "Commit 4" | rule stated once, pointing to //src:core deps | plan Commit 4 list | — |
| 5 | ARCH-MAP.md | 4 | "Commit 5" | dune-map fresh; rust-core at structure, no Current violations | plan Commit 5 list | — |

## Chunks

| id | status | commit(s) | summary | blocker |
|----|--------|-----------|---------|---------|
| 0 | complete | — | baseline: //src:unit_tests reports 168 tests at 88d726c (same tree as 51e574b) | none |
| 1 | complete | c0f545c | removed Python::initialize() from two UUID tests; libpython comments in src/BUILD.bazel and MODULE.bazel now cite the pyo3 link | none |
| 2 | complete | 6ff7596 | deleted four unused From impls on SettingsError and their two tests; no map_err fallback needed; 166 tests | none |
| 3 | complete | a2885a3 | //src:core (ark_resolver_core) and //src:core_unit_tests added; 119 core + 47 adapter tests = 166; negative checks fail with E0433 (pyo3) and E0432 (crate::adapters); Linux libunwind fallback applied after review (see Deviations) | none |
| 4 | complete | 094d0f0 | ADR-0001 amended to the //src:core deps allowlist; CONVENTIONS.md, REVIEW.md, ADR-0002 point to it; Python::initialize grep clean after commit 5; review fix: the ADR says the build enforces the list and review checks each entry | none |
| 5 | complete | see `docs(arch-map)` commit | dune-map update: rust-core, rust-adapters, build-and-delivery re-fingerprinted; layering rule `structure` for crates, `review` for std::env/std::fs; Current violations removed | none |
| review | complete | folded into a2885a3, 094d0f0 | 5 reviewers (consistency, simplicity, dune, rust, devops): 3 findings fixed, rest triaged as no-fix | none |

## Deferrals

- clippy `disallowed-methods` for `std::env`/`std::fs` in the core (dune-reviewer DUNE-001): the plan's named residual gap, left as a follow-up; unverified whether rules_rust's clippy config can be scoped to `//src:core`.

## Deviations

- Commit 3's Linux libunwind fallback was applied before observing a Linux failure, not after: `//tools/healthcheck:healthcheck_test` is the same shape (pure Rust, no libpython) and needs it, and macOS cannot cross-build test targets for Linux (no matching toolchain). CI's Linux `just test` is the confirmation.

## Side findings

- `just --check --fmt --unstable` (first step of `just rustcheck`) fails on the unchanged justfile with Homebrew just 1.46.0, which would also delete a comment; CI's setup-just is unpinned. Version skew, not a diff problem: the other rustcheck and pycheck steps were run individually and pass.
- `src/core/mod.rs` to `src/core/lib.rs` records as delete plus add, not a rename: the 4-line file gained a 5-line crate doc, below git's similarity threshold. `git log --follow` on lib.rs stops at a2885a3.

## Closeout

- root_cause: `//src:ark_resolver_lib` compiled `src/core` and `src/adapters` as one crate with `pyo3` and `config` in its `deps`, so ADR-0001's "core never depends on PyO3 or infrastructure" rule had no mechanism behind it; violations (two `Python::initialize()` test calls, four unused `From` impls including one on `config::ConfigError`) slipped in unnoticed.
- investigation: Every violation was a crate import, which a Bazel `deps` allowlist catches. All four `From` impls were dead (the compiler confirmed no `?` site used them). No `pub(crate)` in the core and no adapter `impl` on a core type, so the split needed no visibility or orphan-rule changes. Review found the new pure-Rust test binary would lack an unwinder on Linux (hermetic LLVM, `--unwindlib=none`), matching `healthcheck_test`'s precedent, and that the module comment in `check_digit.rs` had been dropped unasked.
- solution: `src/core` is the crate `ark_resolver_core` (`//src:core`, deps regex, serde_json, thiserror, async-trait) with `//src:core_unit_tests` (119 tests, no libpython, static libunwind on Linux); adapters import `ark_resolver_core::`; ADR-0001 amended to the allowlist, CONVENTIONS.md, REVIEW.md and ADR-0002 point to it; ARCH-MAP.md raises the layering rule to `structure`. Negative checks: a `pyo3` import in the core fails with E0433, `crate::adapters` with E0432.
- prevention: The `deps` list is the rule, changed only by a reviewed BUILD diff; `just rustcheck` names `//src:core` and `//src:core_unit_tests` because lint aspects do not propagate through `deps`. `std::env`/`std::fs` remain review-only; a clippy `disallowed-methods` scoped to the core is the follow-up.
