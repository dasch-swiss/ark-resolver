# ADR-0002: Build, test and ship ark-resolver with Bazel

## Status

**Accepted** - 2026-09-27

## Context

ark-resolver is a hybrid Python (Sanic) / Rust (PyO3) service: Rust functions are compiled as the
Python extension module `ark_resolver._rust` and imported by the Python server. Before this
decision, the hybrid was built with four separate tools: `cargo`/`cargo-nextest` for the Rust
crate and its unit tests, `maturin` for the PyO3 extension, `uv` for the Python dependencies and
virtualenv, and an Alpine Dockerfile that ran all three in sequence. CI resolved the Python side
against 3.13 while the production image ran 3.12, and there was no single build graph tying Rust,
the extension, and Python together or caching across them.

The CLAUDE.md migration plan for this repo describes an eventual pure-Rust Axum service replacing
Sanic. That rewrite is not being pursued; the service stays the Python + PyO3-extension hybrid, and
the hybrid itself is what Bazel now builds.

DaSCH's sipi repository already builds a comparable C++/native-extension service with Bazel
(bzlmod), pinned to rules_rust 0.70.0, Rust 1.89.0, and a hermetic LLVM 0.8.18 toolchain. Aligning
ark-resolver's Bazel setup with sipi's conventions and pins is a deliberate choice: it keeps both
repos on the same toolchain generation ahead of a possible future move into a shared monorepo, and
lets fixes and upgrades be applied identically in both places.

## Decision

We adopted Bazel (bzlmod) as the build, lint, test, audit and image tool for ark-resolver, matching
sipi's toolchain pins (rules_rust 0.70.0, Rust 1.89.0, hermetic LLVM 0.8.18) rather than the
versions the previous Cargo/Docker setup used (Rust 1.97.0 in the Docker image, 1.88.0 in the
now-deleted `rust-toolchain.toml`). The pin is raised from here if a crate needs a newer compiler.

One bzlmod graph now builds the PyO3 extension (`//:_rust`), runs the Rust unit tests
(`//src:unit_tests`) and the Python tests (`//tests/...`, one `py_test` per module via a pytest
shim), and builds and pushes the OCI image. Bazel is the only build path; see Retired build below
for what this replaced.

### Rust dependencies: `crate.from_specs`

Rust crate dependencies are resolved with crate_universe's `crate.from_specs`, not `from_cargo`:
the `crate.spec` calls in `MODULE.bazel` are the Rust dependency source of truth, and there is no
`Cargo.toml` any more. Three reasons drove this over keeping `Cargo.toml` with cargo as a parallel
backstop:

- sipi itself chose `from_cargo` in its own migration plan and reversed to `from_specs` in
  sipi#725.
- A live cargo path is a second build graph that drifts from the Bazel one over time; sipi ships
  no cargo for the same reason.
- This migration is a one-PR full cutover, which calls for a single build path rather than two
  kept in sync by hand.

`Cargo.Bazel.lock` (Cargo lock file format, not MODULE.bazel.lock's JSON) is the checked-in
lockfile, materialized by `crate.from_specs(cargo_lockfile = "//:Cargo.Bazel.lock")`. `cargo audit`
remains, pointed at that file directly (`cargo audit --file Cargo.Bazel.lock`, run via `just
audit`); cargo's only remaining role is as an audit tool, it is never invoked to build or test.

### PyO3 extension

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
  which the previous maturin build did not use. Building our own toolchain on top of rules_rust_pyo3
  0.70.0 gets PyO3 0.29 without moving either of those two things.
- The Cargo `extension-module` feature is deprecated in PyO3 0.29; its replacement is read from an
  environment variable at build-script time, so a single `@crates//:pyo3` cannot serve both the
  extension build and the Rust unit tests, which embed an interpreter via `Python::initialize()`.
  We build `pyo3` without extension-module mode; the extension then links libpython dynamically,
  which the previous maturin build did not do (see Libpython below).
- The Cargo `build.rs` that called `add_extension_module_link_args()` was deleted: it only emitted
  `-undefined dynamic_lookup` on macOS, which `pyo3_extension` adds itself (together with
  `-Wl,-no_fixup_chains`) via a `select`.
- Rust unit tests (`//src:unit_tests`) link `@rules_python//python/cc:current_py_cc_libs`; on
  macOS no `PYTHONHOME` or library-path environment variable is needed because the resulting
  binary carries an `@loader_path` rpath. This has not been proven hermetic under remote
  execution; the fallback if it breaks there is an `sh_test` wrapper that sets `PYTHONHOME`
  explicitly. All 168 Rust unit tests pass under Bazel, matching the count under cargo-nextest.

### Python

- `uv.lock` is the single Python lock, dev dependencies included. `pip.parse(uv_lock =
  "//:uv.lock", download_only = True)` resolves wheels directly from it for both `{os}_{arch}`
  (host, used by tests) and `linux_x86_64` (the image); there is no separate
  `requirements_lock.txt`. `pip.parse` resolves the Sanic/uvloop/httptools wheels for both
  platforms straight from `uv.lock`; a second lock would need its own staleness check and would
  sit outside the directory Dependabot's `uv` ecosystem scans.
- All 126 Python tests pass under Bazel (`bazel test //tests/...`), matching the count under the
  previous `pytest` path.
- The Python toolchain is hermetic CPython 3.12 everywhere, CI and the production image alike,
  replacing the previous split (CI on 3.13, production on 3.12).
- `ruff` and `pyright` stay uv-driven behind `just check` (via `just pycheck`), not run as Bazel
  aspects (`aspect_rules_lint`): `ruff` through `uv` is already fast and cached, and `pyright`
  downloads Node at runtime, which fights the Bazel sandbox.

### Image

- The image is built with `rules_oci` on `gcr.io/distroless/cc-debian13`, pinned by digest (never
  a floating tag), `linux/amd64` only.
- The entrypoint names the hermetic interpreter directly, the `python3` binary inside the
  `py_binary`'s own runfiles tree, because the distroless base has no shell and every `py_binary`
  launcher `rules_python` generates is a bash script.
- The image runs as uid 65532 (distroless's `nonroot`).
- The interpreter is stripped: `install_only_stripped` (python-build-standalone) instead of the
  default `install_only` archive, which is about 194 MB of the 228 MB Linux interpreter in debug
  symbols. `pip`, `ensurepip` and `wheel` are additionally excluded from the image layer
  (`bazel/tar/exclude_pip.awk`).
- The layer is assembled with `mtree_spec` / `mtree_mutate` / `tar` (zstd compression) on the
  untransitioned binary, then wrapped in a `platform_transition_filegroup` to `linux_x86_64`:
  building the tar on the already-transitioned target loses the runfiles manifest.
- Rejected bases and layer strategies:
  - Alpine/musl: `rules_oci` and the manylinux wheel ecosystem are glibc-first, and
    hermetic-llvm's musl path caused header contamination.
  - `gcr.io/distroless/base-debian12`: reached end of life on 2026-09-10.
  - `gcr.io/distroless/base-debian13`: rejected because it lacks `libgcc_s`, which a Rust gnu
    cdylib needs. The libunwind fix below (a static link, not a dynamic one) removes that need, but
    `cc-debian13` remains the base image; the choice was not re-derived.
  - `gcr.io/distroless/python3-debian13`: ships Python 3.13, mismatching the cp312 wheels and the
    3.12 PyO3 ABI.
  - aspect_rules_py's venv launchers: every launcher is a bash script, and the base has no shell.
  - A shell-carrying base (`:debug` or a debian-slim image): the fallback if the
    explicit-interpreter entrypoint ever fails, at the cost of a larger attack surface.
- OCI images have no `HEALTHCHECK` field, and the base has no shell or curl. The image ships
  a small std-only Rust binary, `/app/healthcheck` (`//tools/healthcheck`), that checks
  `/health`; deployments declare `["CMD", "/app/healthcheck"]`. It keeps probe logic out of
  deployment config, runs locally with `just healthcheck`, and needs no Python interpreter per
  probe. Like the extension, it links llvm libunwind statically on Linux.

### libunwind

On Linux, `_rust.so` statically links llvm-project's libunwind, as a linux-only `cc_import`
dependency of `//:_rust`. The hermetic LLVM toolchain links with `--unwindlib=none`, and rustc
links the extension's cdylib itself rather than going through `cc_common`, so without this the
extension had `_Unwind_*` symbols undefined when imported inside the image. The statically linked
unwinder's `_Unwind_*` symbols are LOCAL (none appear in `.dynsym`), so there is no symbol clash
with cc-debian13's own `libgcc_s`.

### Libpython

The extension links libpython dynamically (see PyO3 extension above); the risk that this would
fail to load in the production image did not materialise. The `.so`'s NEEDED entry
`libpython3.12.so.1.0` resolves inside the container via the hermetic interpreter's own
`DT_RPATH`, `$ORIGIN/../lib`. This was verified with `just image-check` importing
`ark_resolver._rust` (and `sanic`, `uvloop`, `httptools`) inside the built image; the
isolate-feature fallback was not needed.

### Dev loop

- `just run` uses `bazel run //:ark_resolver_bin -- -s`, not `uv run`: uv's statically linked
  python-build-standalone interpreter on macOS would load a second libpython alongside the
  Bazel-built `.so`'s `@rpath/libpython3.12.dylib` and segfault, while Bazel's own hermetic
  interpreter links the same dylib and works.
- The `just build` and `just docker-build` mechanics are documented as comments in the `justfile`
  itself.

### Stamping and tags

`tools/workspace_status.sh` defines `STABLE_IMAGE_TAG`: `<version>` when `HEAD` sits exactly on a
git tag, otherwise `<version>-<shortsha>`. `<version>` comes from `version.txt`, which
release-please now bumps directly (release-type `simple`, replacing the previous `rust`
release-type keyed off `Cargo.toml`). `//:image_push` pushes only that one stamped tag; `latest` is
never pushed to Docker Hub, only loaded locally by `just docker-build` (`oci_load`), matching the
release flow that never moved `latest` on Docker Hub either.

### Retired build

Bazel is the only build path. `Cargo.toml`, `Cargo.lock`, `build.rs`, `rust-toolchain.toml`, the
Alpine `Dockerfile`, `.dockerignore`, `entrypoint.sh`, `Makefile` and `vars.mk` were deleted, and
`maturin` was removed from `pyproject.toml` and `uv.lock`. Cargo itself is retained only as the
input to `cargo audit` (see Rust dependencies above), never to build or test.

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
- The distroless production image is about 52 MB, against 382 MB for the previous Alpine-based
  image.
- CI (`check.yml`, `test.yml`, `security.yml`, `publish.yml`) calls only `just` recipes; none of
  the workflows invoke `bazel` directly, so the local and CI build surfaces stay identical.
- `just rustfmt` runs rules_rust's own formatter target under the toolchain's stable Rust,
  replacing the previous nightly `cargo +nightly fmt`.

### Negative
- The extension links libpython dynamically instead of statically, a behavior change from the
  previous maturin build, verified to work in the production image (see Libpython above). It stays
  worth revisiting if a future toolchain or base-image change removes the interpreter's
  `DT_RPATH`.
- Dependabot cannot open PRs against `crate.from_specs` entries in `MODULE.bazel` the way it could
  against `Cargo.toml`/`Cargo.lock`, so Rust dependency bumps are manual: `just crates-repin`
  (re-resolve and re-pin `@crates`) followed by `just audit` (`cargo audit --file
  Cargo.Bazel.lock`). `.github/dependabot.yml` dropped the `cargo` and `docker` ecosystems; it now
  tracks only `uv` (Python, via `uv.lock`) and `github-actions`.

### Neutral
- Rust unit tests' hermeticity under remote execution is unverified for macOS's rpath-based
  approach; the `sh_test`/`PYTHONHOME` fallback described above is documented but not yet needed.

## References

- [sipi repository](https://github.com/dasch-swiss/sipi): source of the rules_rust 0.70.0 /
  Rust 1.89.0 / hermetic LLVM 0.8.18 toolchain pins this ADR aligns with.
- [sipi#725](https://github.com/dasch-swiss/sipi/pull/725): sipi's reversal from `from_cargo` to
  `from_specs`, the precedent this decision follows.
- [rules_rust PyO3 documentation](https://bazelbuild.github.io/rules_rust/rust_pyo3.html)
- [rules_rust PR #4256](https://github.com/bazelbuild/rules_rust/pull/4256): fixes
  `lint_config` analysis failure on `pyo3_extension`, not yet released.
- [quarylabs/sqruff](https://github.com/quarylabs/sqruff): worked example of a `pyo3_extension`
  build under Bazel.
- [distroless](https://github.com/GoogleContainerTools/distroless): source of the `cc-debian13`
  base image.
- [ADR-0001: Adopt Hexagonal Architecture for Python-to-Rust Migration](0001-adopt-hexagonal-architecture.md)
