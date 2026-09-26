---
title: "refactor: Build, test and ship ark-resolver with Bazel"
type: refactor
date: 2026-09-26
author: "Ivan Subotic"
status: implemented
repositories: []
prd: "2026-02-13-01-ark-resolver-migration-completion-PRD.md"
linear: DEV-5874
linear_project: "Migrate Ark-Resolver to Rust"
branch: feature/dev-5874-phase-3-bazel-build-deployment-docker-hub-jenkins
---

# refactor: Build, test and ship ark-resolver with Bazel

## Overview

Replace cargo, maturin, uv-as-builder and the Dockerfile with one Bazel (bzlmod) build graph for the service **as it is today**: the Python/Sanic app in `ark_resolver/` plus the Rust crate in `src/`, loaded as the PyO3 extension `ark_resolver._rust`.

- Bazel builds the extension, runs the Rust and Python tests, and produces the OCI image with `rules_oci`.
- Bazel becomes the only path CI uses to check, test and publish.
- Conventions, rule versions and CI shape mirror sipi's Bazel setup, so the eventual monorepo move stays mechanical.

Delivery is one PR on this branch. The phases below are commit groups inside it. Each group leaves the repository buildable, and the old path (cargo, maturin, Dockerfile) keeps working until Phase 6 deletes it.

**Out of scope** (the rest of PRD Phase 3, left for later):

- baking the registry config into the image;
- removing `/reload` and `ARK_GITHUB_SECRET`;
- the ark-resolver-data rebuild trigger;
- the Grafana Alloy sidecar;
- a multi-arch image.

Runtime behaviour, routes, env vars and the deployment flow stay as they are.

## Problem Statement / Motivation

- **Decision change.** The PRD sequenced Bazel after the Axum rewrite (D18) "so Bazel meets a clean single-language Rust crate rather than the PyO3/maturin hybrid". That rewrite (PRD Phase 2) is not going ahead. The hybrid stays the production service for the foreseeable future, so the build has to be modernised for the hybrid itself. Phase 6 records this as PRD decision D20.
- **Architecture enforcement.** Same driver as sipi: build-graph visibility turns boundary rules into analysis errors instead of review comments. This plan lays the graph. Splitting `src/core` into its own target to enforce ADR-0001's dependency direction is a follow-up (see Dependencies & Risks).
- **Today's build is four tools and two Python versions.**
  - cargo and nextest run the Rust tests (`justfile:65-68`).
  - maturin builds the extension (`justfile:52-54`).
  - uv installs dependencies.
  - Docker builds a musl/Alpine image with its own ad-hoc Rust targets (`Dockerfile:52-56`).
  - CI tests on Python 3.13 (`check.yml`, `test.yml`) while production runs 3.12 (`Dockerfile:59`).
  - Rust targets differ by context: gnu locally, musl in Docker.
- **Monorepo readiness.** A bzlmod module that mirrors sipi's conventions relocates into a `dasch-monorepo` by moving directories, not by rewriting builds.

## Proposed Solution

One root Bazel module, `ark_resolver`, with four build areas.

1. **Rust.**
   - A `rust_library` for the crate, and a `rust_test` for its unit tests that links libpython from the hermetic toolchain.
   - A `pyo3_extension` (from the `rules_rust_pyo3` module) that produces `ark_resolver/_rust.so`.
   - Crates are resolved by crate_universe `from_specs` into a single `@crates` hub, as sipi does (`sipi/MODULE.bazel:545-626`).
   - We register our own `rust_pyo3_toolchain`, so the extension uses PyO3 0.29 from `@crates` rather than the 0.28.2 bundled with `rules_rust_pyo3` 0.70.0.
2. **Python.**
   - `rules_python` with a hermetic CPython 3.12 toolchain.
   - Third-party wheels come from `pip.parse(uv_lock = "//:uv.lock")`, so `uv.lock` stays the single Python lock.
   - Targets: a `py_library` for `ark_resolver`, a `py_binary` for the CLI and server, and one `py_test` per test module through a pytest entry point.
3. **Image.**
   - `rules_oci` on a digest-pinned `gcr.io/distroless/cc-debian13`.
   - The image carries the `py_binary` runfiles: interpreter, wheels, app and extension.
   - The entrypoint names the hermetic interpreter explicitly, because distroless has no shell and every rules_python launcher needs one.
   - The image is stamped from `version.txt` and git through `tools/workspace_status.sh`.
4. **CI.**
   - A composite setup action installs `bazel-contrib/setup-bazel` and just, and restores the repository cache.
   - Workflows call only `just <recipe>`, never `bazel` directly (`sipi/justfile:4-8`).

**Versions**, following sipi where sipi has a pin. The notes column marks the exceptions.

| Item | Version | Source / note |
|---|---|---|
| Bazel | 9.1.0 | `sipi/.bazelversion` |
| `rules_rust` | 0.70.0 | `sipi/MODULE.bazel:481`. Keep sipi's `single_version_override` policy. |
| `rules_rust_pyo3` | 0.70.0 | Same release train as `rules_rust`. Its `module(name = "rules_rust_pyo3")` is in `extensions/pyo3/MODULE.bazel` at the 0.70.0 tag. |
| Rust | 1.89.0 | `sipi/MODULE.bazel:499-520`. Deliberately sipi's pin; today's image builds with 1.97.0 and `rust-toolchain.toml` says 1.88.0. |
| Hermetic LLVM | 0.8.18 | `sipi/MODULE.bazel:168`. `ring` (rustls via reqwest) compiles C in its build script; without a registered toolchain it silently uses the host compiler. |
| `rules_python` | 2.3.4 | Latest stable on BCR, which tests it against Bazel 9.x. New to this repo; sipi uses 1.8.0 as a dev dependency only. |
| `rules_oci` | 2.3.0 | `sipi/MODULE.bazel:200` |
| `tar.bzl` | 0.10.9 | Newer than sipi's 0.10.4, because 0.10.5–0.10.9 fix `preserve_symlinks` under transitioned configs, which the image needs. |
| `aspect_bazel_lib` | 2.22.5 | `sipi/MODULE.bazel:49`. Provides `platform_transition_filegroup`, `expand_template` and stamping. |
| `platforms` | 1.1.0 | `sipi/MODULE.bazel:24` |
| `apple_support` | 2.9.1 | Needed on macOS with Xcode 27: the default-resolved 1.24.2 fails building gawk (used by `mtree_mutate`) with an absolute-SDK-path error. Verified in a spike. |
| `rules_cc` | 0.2.19 | `sipi/MODULE.bazel:25` |

Sketch (label names are the targets this plan creates; attribute names verified against the pinned rule sources unless marked):

```starlark
# MODULE.bazel
module(name = "ark_resolver", version = "0.0.0", compatibility_level = 1)  # BR: the real version lives in version.txt, stamped at build time

bazel_dep(name = "platforms", version = "1.1.0")
bazel_dep(name = "rules_cc", version = "0.2.19")
bazel_dep(name = "aspect_bazel_lib", version = "2.22.5")
bazel_dep(name = "llvm", version = "0.8.18")
bazel_dep(name = "rules_rust", version = "0.70.0")
bazel_dep(name = "rules_rust_pyo3", version = "0.70.0")
bazel_dep(name = "rules_python", version = "2.3.4")
bazel_dep(name = "rules_oci", version = "2.3.0")
bazel_dep(name = "tar.bzl", version = "0.10.9")
bazel_dep(name = "apple_support", version = "2.9.1")

# hermetic-llvm toolchain setup as in sipi, then:
register_toolchains("@llvm_toolchains//:all")      # must precede the Rust toolchains (sipi MODULE.bazel:444 vs :523)

python = use_extension("@rules_python//python/extensions:python.bzl", "python")
python.toolchain(python_version = "3.12", is_default = True)   # explicit: rules_python 2.4 changes the default to 3.14

pip = use_extension("@rules_python//python/extensions:pip.bzl", "pip")
pip.parse(
    hub_name = "pypi",
    python_version = "3.12",
    uv_lock = "//:uv.lock",
    download_only = True,
    target_platforms = ["{os}_{arch}", "linux_x86_64"],   # host wheels for tests, manylinux x86_64 for the image
)
use_repo(pip, "pypi")

crate = use_extension("@rules_rust//crate_universe:extensions.bzl", "crate")
crate.spec(package = "pyo3", version = "0.29", features = [...])
crate.spec(package = "pyo3-introspection", version = "0.29")         # required by rust_pyo3_toolchain
crate.spec(package = "reqwest", version = "0.12.12", default_features = False, features = ["blocking", "rustls-tls", "stream"])
crate.spec(package = "rustls", version = ">=0.23.45", default_features = False, features = [...])  # RUSTSEC-2026-0285 floor
# ... one crate.spec per Cargo.toml [dependencies]/[dev-dependencies] entry, same versions and features
crate.annotation(
    crate = "pyo3-ffi",
    build_script_data = ["@@rules_rust_pyo3+//:current_pyo3_toolchain"],        # canonical label: the crate repo cannot see @rules_rust_pyo3
    build_script_toolchains = ["@@rules_rust_pyo3+//:current_pyo3_toolchain"],
    build_script_env = {...},   # the PYO3_* make variables the pyo3 toolchain exports
)
crate.from_specs(cargo_lockfile = "//:Cargo.Bazel.lock")
use_repo(crate, "crates")

register_toolchains(
    "@rust_toolchains//:all",
    "@rules_rust_pyo3//toolchains:toolchain",   # pyo3_toolchain: Python config from rules_python
    "//bazel/pyo3:rust_pyo3_toolchain",         # ours: PyO3 0.29 from @crates
)
```

```starlark
# BUILD.bazel (root): module_name "ark_resolver._rust" places the .so at ark_resolver/_rust.so
load("@rules_rust_pyo3//:defs.bzl", "pyo3_extension")

pyo3_extension(
    name = "_rust",
    srcs = ["//src:srcs"],
    crate_root = "//src:lib.rs",
    crate_name = "ark_resolver",
    crate_features = ["pyo3"],
    deps = ["@crates//:reqwest", ...],    # never @crates//:pyo3 here: the toolchain injects it
    edition = "2021",
    module_name = "ark_resolver._rust",
    stubs = False,                         # stub generation needs pyo3's experimental-inspect feature
    compilation_mode = "current",          # the default "opt" transition puts the .so outside bazel-bin
    visibility = ["//visibility:private"],
)
```

```starlark
# src/BUILD.bazel
rust_library(
    name = "ark_resolver_lib",
    crate_name = "ark_resolver",
    srcs = [...],   # explicit list, no glob (sipi convention)
    deps = ["@crates//:pyo3", "@crates//:reqwest", ...],
    edition = "2021",
)

rust_test(
    name = "unit_tests",
    crate = ":ark_resolver_lib",
    deps = ["@rules_python//python/cc:current_py_cc_libs"],   # tests call pyo3::Python::initialize()
    data = ["//tests:ark-registry.ini"],
    env = {"ARK_REGISTRY": "$(rootpath //tests:ark-registry.ini)"},
)
```

## Alternative Approaches Considered

- **Sequence Bazel after the Axum rewrite (PRD D18).** Rejected: the rewrite is not going ahead. Waiting for it would mean no Bazel at all.
- **Keep `Cargo.toml` as the source of truth, with cargo as a parallel backstop.** This is PRD D8/D18's `from_cargo` and Phase 3a's "keep `cargo` working in parallel". Rejected for three reasons:
  - sipi chose `from_cargo` in its plan and reversed to `from_specs` in sipi#725.
  - A live cargo path is a second build that drifts. sipi deliberately ships no cargo for that reason (`sipi/flake.nix`).
  - The one-PR full cutover calls for a single path.
- **`rules_rust_pyo3` 0.71.0+ with its bundled PyO3 0.29.** Rejected:
  - It drags `rules_rust` to at least 0.71.0, off sipi's pin.
  - It builds PyO3 with `abi3-py311`, a limited API that today's maturin build does not use.
  - Our own `rust_pyo3_toolchain` keeps both unchanged.
- **Keep Alpine/musl for the image.** Rejected:
  - `rules_oci` and the manylinux wheel ecosystem are glibc-first.
  - hermetic-llvm's musl path brings the header contamination documented in `dasch-specs/learnings/build-errors/zig-cc-glibc-header-contamination-musl-target.md`.
- **`gcr.io/distroless/base-debian12` (sipi's base).** Rejected: distroless dropped all debian12 images on 2026-09-10 (end of life). `base-debian13` lacks `libgcc_s`, which a Rust gnu cdylib needs, so we use `cc-debian13`.
- **`gcr.io/distroless/python3-debian13`.** Rejected: it ships Python 3.13, which does not match the cp312 wheels or the 3.12 PyO3 ABI.
- **aspect_rules_py `py_image_layer` / venv launchers.** Rejected for now: every launcher is a bash script, and the base has no shell. The layer-group regexes remain a good model if the app layer is later split.
- **A base with a shell (`:debug` busybox or debian-slim).** This is the fallback if the explicit-interpreter entrypoint fails in Phase 4. It has less integration risk but a larger attack surface.
- **`uv export` into a separate `requirements_lock.txt`.** Rejected: a spike showed `pip.parse(uv_lock = ...)` resolves the Sanic/uvloop/httptools wheels for host and Linux targets directly. A second lock would need a staleness check and would have to sit outside the directory Dependabot scans for `uv`.
- **pyright and ruff as Bazel aspects (aspect_rules_lint).** Rejected:
  - ruff through uv is already fast and cached.
  - pyright downloads Node at runtime, which fights the sandbox.
  - Both stay uv-driven dev tools behind `just check`.
- **`oci_image_index` for a multi-arch manifest.** Out of scope: only amd64 is published today (`publish.yml` uses `docker-publish-intel`).

## Technical Considerations

### PyO3 extension (highest risk)

- **Rule shape.** `pyo3_extension` wraps a `rust_shared_library` (`<name>_shared`) in `py_pyo3_library`, which symlinks it to `<package>/<module path>.so`.
  - The file name is always plain `_rust.so`, never `.abi3.so` or a `cpython-312-…` tag.
  - It returns `PyInfo`, so it goes straight into `py_library` and `py_test` `deps`.
  - With `module_name = "ark_resolver._rust"` in the **root** package it lands at `ark_resolver/_rust.so`. Declaring it inside `//ark_resolver` with that module name would produce `ark_resolver/ark_resolver/_rust.so`.
- **Toolchains.**
  - `@rules_rust_pyo3//toolchains:toolchain` (the pyo3_toolchain) supplies Python config from rules_python.
  - `//bazel/pyo3:rust_pyo3_toolchain` is our `rust_pyo3_toolchain(pyo3 = "@crates//:pyo3", pyo3_introspection = "@crates//:pyo3-introspection")`, with its `toolchain()` bound to `@rules_rust_pyo3//:rust_toolchain_type`.
  - The module's own `register_toolchains` is `dev_dependency = True`, so we register both ourselves.
- **abi3 and version.** The pyo3_toolchain sets `PYO3_CROSS_PYTHON_VERSION` from the rules_python toolchain. Our PyO3 build is non-abi3, targeting 3.12, as today. The PyO3 config and the rules_python toolchain must both say 3.12; a mismatch builds silently and fails at import.
- **extension-module.** The Cargo feature is deprecated in PyO3 0.29; its replacement, the `PYO3_BUILD_EXTENSION_MODULE` env var, is read by `pyo3-build-config` at build-script time. So one `@crates//:pyo3` cannot serve both the extension and the embedded unit tests.
  - Decision: build `@crates//:pyo3` without extension-module mode. The extension then links libpython dynamically, where maturin builds without that link today.
  - A spike confirmed it loads under the hermetic interpreter. The Phase 4 `just image-check` import of `ark_resolver._rust` inside the container covers the image.
  - A second, extension-mode pyo3 hub is the fallback. It needs crate_universe's experimental `isolate` (sipi hit duplicate-spec collisions without it).
  - `pyproject.toml`'s maturin `module-name` goes away, and so does our `extension-module` feature.
- **build.rs.** Dropped. `add_extension_module_link_args()` only emits `-undefined dynamic_lookup` on macOS, and `pyo3_extension` already adds `-undefined dynamic_lookup` and `-Wl,-no_fixup_chains` on macOS through a `select`.
- **crate annotation.** `pyo3-ffi`'s build script gets the pyo3 toolchain (canonical label `@@rules_rust_pyo3+//:current_pyo3_toolchain`) as data/toolchain plus the `PYO3_*` make variables (`PYO3_CROSS`, `PYO3_CROSS_LIB_DIR`, `PYO3_CROSS_PYTHON_IMPLEMENTATION`, `PYO3_CROSS_PYTHON_VERSION`, `PYO3_NO_PYTHON`, `PYO3_PYTHON`). Without them it probes the host `python3`, which may be 3.13. This is the same setup as quarylabs/sqruff's `MODULE.bazel`.
- **Rust unit tests and libpython.**
  - Some tests call `pyo3::Python::initialize()` (`src/core/domain/uuid_processing.rs` tests; ARCH-MAP.md:102-105), so `unit_tests` links `@rules_python//python/cc:current_py_cc_libs`.
  - A spike on macOS showed no `PYTHONHOME` or library-path env is needed: `current_py_cc_libs` adds an `@loader_path` rpath, and `sys.prefix` resolves through symlinks.
  - That resolution points at Bazel's repository cache, so it is not hermetic and may break under remote execution. The fallback is an `sh_test` wrapper setting `PYTHONHOME` (sqruff pattern).
- **Test isolation.** `serial_test` guards env-var tests in `src/adapters/environment/`. Do not set `shard_count` on `unit_tests`. Bazel's clean test env drops `ARK_*` and `RUST_LOG`, so pass any a test needs through `env`.
- **macOS exec builds.** rules_rust builds exec tools with `-Cstrip=debuginfo`. Under Xcode 27 that makes proc-macro dylibs unloadable (`can't find crate for 'thiserror_impl'`).
  - `build:macos --@rules_rust//rust/settings:extra_exec_rustc_flag=-Cstrip=none` fixes it, verified in a spike.
  - Linux CI is probably unaffected.
- **Lints.**
  - `lint_config` on `pyo3_extension` fails analysis before rules_rust PR #4256 (merged 2026-09-14, in no release yet).
  - The clippy and rustfmt aspects are applied from the command line instead, to `//src:ark_resolver_lib`, `//src:unit_tests` and `//:_rust_shared`.
  - The `//:_rust_shared` target is needed because it is the only one compiled with feature `pyo3`, which gates code in `src/lib.rs` and `src/adapters/environment/env_logger.rs`.
  - Cargo.toml has no `[lints]` table, so `-Dwarnings` is the whole lint policy.

### Python

- **Wheels from `uv.lock`.**
  - `pip.parse(uv_lock = ...)` is supported since rules_python 2.2.0.
  - `target_platforms` pulls host wheels for tests plus manylinux x86_64 wheels for the image. Every runtime dependency in `uv.lock` has a cp312 manylinux x86_64 or pure wheel.
  - `--@rules_python//python/config_settings:current_config=fail` in `.bazelrc` makes a platform mismatch a build error instead of a warning.
- **Import layout.** A root-package `py_library(name = "ark_resolver", srcs = glob(["ark_resolver/**/*.py"]), deps = [":_rust", "@pypi//..."])`, so `import ark_resolver` and `from ark_resolver import _rust` resolve from the workspace root.
- **pytest shim.** `tests/pytest_main.py` calls `pytest.main()`, passing:
  - `-c` pointing at the root `pyproject.toml` in data;
  - `--verbose -s`, the current `addopts`;
  - `-p no:cacheprovider`, because runfiles are read-only;
  - `--basetemp=$TEST_TMPDIR`;
  - `--junitxml=$XML_OUTPUT_FILE`, so per-case pass counts are measurable;
  - the test file.

  Check that `tests/__init__.py` imports still resolve with the runfiles root as rootdir.
- **Network tests.** `tests/test_http_registry_rust.py` hits `raw.githubusercontent.com` (lines 20, 48, 121, 139). It is tagged `requires-network` and `external`. `external` stops Bazel from caching a result that depends on a remote service.
- **Dev loop.**
  - `just run` is `bazel run //:ark_resolver_bin -- -s` with `ARK_REGISTRY` set to the absolute path of `tests/ark-registry.ini`. It no longer runs through `uv run`.
  - Reason, found in execution: the Bazel `.so` links `libpython3.12` dynamically (pyo3 without extension-module). uv's python-build-standalone interpreter on macOS is statically linked, so importing the `.so` into it loads a second libpython and segfaults. The hermetic interpreter links the same dylib and works.
  - `just build` builds `//:_rust`, removes any stale `ark_resolver/_rust*.so` (a leftover maturin `_rust.cpython-312-darwin.so` would shadow it), and copies `_rust.so` into `ark_resolver/`, covered by the existing `*.so` rule in `.gitignore`. The copy exists only so pyright and IDEs resolve `ark_resolver._rust`; it is not imported by `uv run`.
  - `.python-version` of `3.12` still pins uv's venv for ruff and pyright. `requires-python = ">=3.12"` alone would allow 3.13.
- **pyproject.toml.** Drop `[build-system]` (maturin), the `maturin` runtime and dev dependencies, and `[tool.maturin]`. Set `[tool.uv] package = false`, and keep the ruff, pyright and pytest config.
  - `version = "0.1.0"` stays static and is not release-managed, as today.

### Image

- **Base.** `gcr.io/distroless/cc-debian13`, digest-pinned via `oci.pull` for `linux/amd64`, with the digest-refresh command as a comment (sipi pattern, `sipi/MODULE.bazel:216-232`).
  - `cc` adds `libgcc-s1` and `libstdc++6` to `base`.
  - Phase 4 confirms the need with `readelf -d` on `_rust.so`.
- **Entrypoint without a shell.**
  - `.bazelrc` sets `--@rules_python//python/config_settings:venvs_use_declare_symlink=no`, so the rules_python stub rebuilds its venv at runtime instead of relying on a symlink tar could drop.
  - The entrypoint names the interpreter: `[/app/ark_resolver_bin.runfiles/rules_python++python+python_3_12_x86_64-unknown-linux-gnu/bin/python3, /app/ark_resolver_bin]`. This is the repo directory name a spike observed; it is arch-specific, so an arm64 image would need its own `oci_image`. The stub is pure Python and ends in `os.execv`. Its `subprocess(shell=True)` path only runs when `RESOLVE_PYTHON_BINARY_AT_RUNTIME=1`, which is `0` for a rules_python toolchain.
  - `env = {"RULES_PYTHON_EXTRACT_ROOT": "/tmp/rp", "PYTHONUNBUFFERED": "1"}`, `cmd = ["-s"]` (same contract as `entrypoint.sh`), `workdir = "/app"`, `user = "65532:65532"` (distroless nonroot), `exposed_ports = ["3336/tcp"]`.
  - The spike showed the venv path under `/tmp/rp` is reused across restarts, while without the variable every start leaves a new `/tmp/bazel.*` directory. It also works with a read-only rootfs plus a `/tmp` tmpfs.
  - The runfiles repo name is hard-coded into the entrypoint, so the Phase 4 checks assert that path exists.
- **Layers.**
  - `mtree_spec(srcs = [":ark_resolver_bin"], include_runfiles = True)`.
  - `mtree_mutate(srcs = [":ark_resolver_bin"], package_dir = "app", mtime = "0", preserve_symlinks = True)`. `srcs` is required with `preserve_symlinks`.
  - `tar(compress = "zstd")`, all on the **untransitioned** binary.
  - Then `platform_transition_filegroup(target_platform = "//platforms:linux_x86_64")` wraps the finished tar. Transitioning first silently drops all runfiles: tar.bzl skips srcs without a runfiles manifest, and a filegroup has none (spike).
  - rules_py#390 is the failure this guards against: a host-arch interpreter leaking into the image.
- **Interpreter contents.**
  - The python-build-standalone interpreter ships `pip` and `ensurepip` in its own `site-packages`, so pip is importable in the image unless it is filtered out of the mtree.
  - Its `python3.12` and `libpython3.12.so` are unstripped: about 194 MB of the 228 MB interpreter is debug symbols. Without stripping, the image is around 392 MB against 382 MB for 1.14.1 today.
  - The layer therefore drops `pip`/`ensurepip` from the mtree and uses a stripped interpreter. Use a stripped python-build-standalone variant if `rules_python` can select one, otherwise add a strip step on the Linux-configured files.
- **Working directory and files.** Today `WORKDIR /app` and `PYTHONPATH=/app:...` (Dockerfile). ops-deploy sets env only and mounts nothing under `/app` (`ops-deploy/ark.yml:28-75`). Phase 4 greps `ark_resolver/` for cwd-relative file access.
- **TLS roots and time zones.** reqwest/rustls (registry fetch), httpx and requests, and the Sentry SDK all need CA certificates. Phase 4 verifies `ca-certificates` and `tzdata` in the built image rather than assuming them.
- **PID 1.** `entrypoint.sh` `exec`s Python with no init today, and Sanic forks workers. Keep that (no tini) so signal handling does not change in this PR.
- **Healthcheck.**
  - OCI has no HEALTHCHECK field, and the image has no curl, jq or shell.
  - The image ships a std-only Rust binary, `/app/healthcheck` (`//tools/healthcheck`), that checks `/health` on `ARK_INTERNAL_PORT`. Deployments declare `["CMD", "/app/healthcheck"]` in ops-deploy's `deploy_healthcheck` (`ops-deploy/roles/deploy/defaults/main.yml:19-27`, H1) and in `docker-compose.yml`.
  - Changed during review (2026-09-28): the planned exec-form Python one-liner put probe code into every deployment file and could not be run locally. The binary runs locally with `just healthcheck` and needs no interpreter per probe.
- **Stamping.**
  - `STABLE_*` keys only (sipi; rules_oci#269): `STABLE_GIT_COMMIT`, `STABLE_ARK_VERSION` (from `version.txt`), `STABLE_IMAGE_CREATED` (commit date).
  - Tags keep today's scheme: `<version>` on a release tag, otherwise `<version>-<shortsha>`. Only that tag is pushed; `latest` is a local tag from `image_load`, as `docker-build-intel` did (corrected in the Phase 4 review: today's publish never pushes `latest`).
  - Test recipes run unstamped.
- **Push.** `oci_push` to `docker.io/daschswiss/ark-resolver`. `docker/login-action` provides credentials, reusing the existing secrets.
- **Docker Scout.** SBOM generation and the CVE compare stay outside Bazel and run on the `oci_load`-ed image.

### Rust dependency management without Cargo.toml

- **Lock file.** `Cargo.Bazel.lock` (Cargo lock format, generated by crate_universe) replaces `Cargo.lock`. Repin with `CARGO_BAZEL_REPIN=1 bazel fetch @crates//:all`, behind `just crates-repin`.
- **Audit.** `just audit` runs `cargo audit --file Cargo.Bazel.lock`, as sipi's `just audit` does, and `security.yml` calls it.
- **Security floors.** Explicit `crate.spec` pins:
  - rustls `>=0.23.45` (RUSTSEC-2026-0285, PR #187);
  - `default_features = False` on reqwest, so `aws-lc-sys` stays out of the graph. Its cmake build script panics inside Bazel (`sipi/MODULE.bazel:538-542`).
- **Dependabot.**
  - Drop the `cargo` and `docker` ecosystems.
  - Switch `pip` to `uv`, which is GA and updates `uv.lock`.
  - No Dependabot ecosystem parses `crate.spec`. Renovate's `bazel-module` manager does (see Dependencies & Risks).
- **rust-analyzer.** `just rust-project` runs `bazel run @rules_rust//tools/rust_analyzer:gen_rust_project`.

### Versioning and release

- `version.txt` (initial value `1.14.1`, the current `Cargo.toml` version) becomes the single version source. It must exist before release-please switches, because the `simple` strategy updates it with `createIfMissing: false`.
- release-please changes `release-type` from `rust` to `simple` in both places it is set in `.github/release-please/config.json`: top level and `packages["."]`.
- `.github/release-please/manifest.json` keeps supplying the current version.

### Visibility

Follow sipi's `CONVENTIONS.md:201-233`:

- private by default;
- narrow explicit `//pkg:__pkg__` grants;
- no `//visibility:public` without a comment saying why;
- no `package_group`.

`platforms/BUILD.bazel` defines the `platform()`s and `config_setting`s, visible to the packages that use them. Bazel 9 enforces visibility on both, and the spike failed with private platforms.

### Security

- The image changes base, from Alpine to distroless Debian 13. The Docker Scout compare on the PR shows the CVE delta against production (H3).
- The Dockerfile's `wheel>=0.46.2` CVE fix (`Dockerfile:69-70`) no longer applies, because the image has no pip and no wheel package. Phase 4 verifies that.
- No new secrets. `/reload` and `ARK_GITHUB_SECRET` are unchanged.

## Implementation Phases

**Delivery:** one PR on `feature/dev-5874-phase-3-bazel-build-deployment-docker-hub-jenkins`. Each phase is a commit group on that branch, not a separate slice or PR.

**Verification commands** are the repository's recipes. The names `eng.yaml` declares (`just test`, `just pytest`, `just check`) are kept and rewired to Bazel, so `just check && just test && just pytest` stays the gate throughout.

#### Phase 1: Bazel skeleton and toolchains

- [x] Add `flake.nix` (dev shell only, as `sipi/flake.nix`): bazelisk plus a `bazel` wrapper script, just, uv, cargo-audit; commit `flake.lock`
- [x] Add `.envrc` with `use flake`, and `.direnv` to `.gitignore`
- [x] Add `.bazelversion` with `9.1.0`
- [x] Add `MODULE.bazel` with the `bazel_dep`s and versions from Proposed Solution
- [x] Configure the hermetic-llvm toolchain as sipi does (`sipi/MODULE.bazel:168-444`), declaring macos-aarch64, linux-x86_64 and linux-aarch64 exec/targets
- [x] Place `register_toolchains("@llvm_toolchains//:all")` before the Rust toolchain registration, so `ring`'s build script uses the hermetic cc
- [x] Register the Rust `1.89.0` toolchain with `extra_target_triples` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`
- [x] Register the hermetic CPython `3.12` toolchain as default, with `python_version = "3.12"` explicit
- [x] Add `pip.parse` hub `@pypi` with `uv_lock = "//:uv.lock"`, `download_only = True`, `target_platforms = ["{os}_{arch}", "linux_x86_64"]`
- [x] Add a `crate.spec` for every `[dependencies]` and `[dev-dependencies]` entry in `Cargo.toml`, same versions and features, minus `pyo3/extension-module`
- [x] Add `crate.spec` for `pyo3-introspection` `0.29`
- [x] Add the rustls `>=0.23.45` security-floor `crate.spec`
- [x] Add the `crate.annotation` for `pyo3-ffi` wiring `@@rules_rust_pyo3+//:current_pyo3_toolchain` and the six `PYO3_*` build-script env variables
- [x] Generate `Cargo.Bazel.lock` with `CARGO_BAZEL_REPIN=1 bazel fetch @crates//:all`
- [x] Diff resolved crate versions in `Cargo.Bazel.lock` against `Cargo.lock`; pin back any change beyond patch level, or list it in the PR description
- [x] Add `.bazelrc` with sipi's always-on block (`--incompatible_strict_action_env`, `--enable_platform_specific_config`, `--workspace_status_command=tools/workspace_status.sh`, `common --repository_cache=~/.cache/bazel-repo`, `try-import %workspace%/user.bazelrc`), `:linux`/`:macos`/`:release` configs and `test --test_output=errors`
- [x] Add `venvs_use_declare_symlink=no` and `current_config=fail` rules_python settings to `.bazelrc`
- [x] Add `build:macos --@rules_rust//rust/settings:extra_exec_rustc_flag=-Cstrip=none` to `.bazelrc`
- [x] ~~Add a downloader config with mirror fallbacks as sipi does~~ Dropped in the Phases 1-3 review: sipi's mirrors cover single-host C distfiles (lua, jbigkit, jansson) that ark-resolver does not fetch, so the file had no rewrites to carry
- [x] Add `.bazelignore` (`.venv`, `target`, `.idea`, `.vscode`, `.claude`)
- [x] Add `bazel-*` and `user.bazelrc` to `.gitignore`
- [x] Commit `MODULE.bazel.lock`
- [x] Add `platforms/BUILD.bazel` with `linux_x86_64`, `linux_aarch64`, `darwin_aarch64` platforms and `is_*` config settings, visible to the packages that use them (Bazel 9 enforces platform visibility)
- [x] Add `version.txt` containing `1.14.1`
- [x] Add `.python-version` containing `3.12`
- [x] Add `tools/workspace_status.sh` printing `STABLE_GIT_COMMIT`, `STABLE_ARK_VERSION`, `STABLE_IMAGE_CREATED`
- [x] Run `bazel fetch //...`; it completes on the local machine (Linux x86_64 is covered by the Phase 5 CI checks)
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Rust crate, unit tests and the PyO3 extension

- [x] Add `src/BUILD.bazel` with a `srcs` filegroup, `exports_files(["lib.rs"])` and `rust_library` `ark_resolver_lib` (explicit `srcs`, `edition = "2021"`, private visibility)
- [x] Add `tests/BUILD.bazel` with `exports_files(["ark-registry.ini"])`, visible to `//src:__pkg__`
- [x] Add `rust_test` `unit_tests` over the library, linking `@rules_python//python/cc:current_py_cc_libs`, with `tests/ark-registry.ini` in data and `ARK_REGISTRY` set
- [x] Run one `Python::initialize()` test under Bazel; it passes with only `current_py_cc_libs` (fall back to an `sh_test` wrapper setting `PYTHONHOME` if it cannot find the stdlib)
- [x] Add `bazel/pyo3/BUILD.bazel` with our `rust_pyo3_toolchain` (`@crates//:pyo3`, `@crates//:pyo3-introspection`) and its `toolchain()` for `@rules_rust_pyo3//:rust_toolchain_type`
- [x] Register `@rules_rust_pyo3//toolchains:toolchain`
- [x] Register `//bazel/pyo3:rust_pyo3_toolchain`
- [x] Add root `BUILD.bazel` `pyo3_extension` `_rust` with `module_name = "ark_resolver._rust"`, `crate_features = ["pyo3"]`, `stubs = False`, `compilation_mode = "current"`, and no `@crates//:pyo3` in `deps`
- [x] Build `//:_rust` on the local machine (Linux is covered by the Phase 5 CI checks); `bazel-bin/ark_resolver/_rust.so` exists (the Phase 3 `py_test`s prove the import under the hermetic interpreter)
- [x] (not needed: the pyo3_toolchain path worked) If the pyo3_toolchain path fails on PyO3 0.29, switch to the fallback (`rust_shared_library` + the macOS `select` link flags + `copy_file` to `ark_resolver/_rust.so`), and record why in ADR-0002
- [x] Wire `rustfmt_aspect` behind `just rustcheck`, applied to `//src:ark_resolver_lib`, `//src:unit_tests` and `//:_rust_shared`, using the toolchain's stable rustfmt with default config (the repo has no `rustfmt.toml`; CI already checks with stable `cargo fmt --check`)
- [x] Wire `rust_clippy_aspect` with `-Dwarnings` behind `just rustcheck`, on the same three targets
- [x] Add `just rust-project` running `bazel run @rules_rust//tools/rust_analyzer:gen_rust_project`
- [x] Add `just crates-repin`
- [x] Add `just audit` running `cargo audit --file Cargo.Bazel.lock`
- [x] Rewire `just test` to `bazel test //src:unit_tests`
- [x] Run `just test` before the rewire (cargo) and after it (Bazel) on the same commit; the passed-test counts are equal
- [x] Record the PyO3 build decision in `docs/adr/0002-build-with-bazel.md`, with the toolchain choice and why `rules_rust_pyo3` 0.71+ was not taken
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Python app and tests under Bazel

- [x] Add a root `BUILD.bazel` `py_library` `ark_resolver` over `ark_resolver/**/*.py` with deps `:_rust` and the `@pypi` packages it imports
- [x] Add `py_binary` `//:ark_resolver_bin` with main `ark_resolver/ark.py`
- [x] Add `tests/pytest_main.py` with the arguments from Technical Considerations
- [x] Add one `py_test` per `tests/test_*.py` to `tests/BUILD.bazel`, sharing `ark-registry.ini` and the root `pyproject.toml` as data, and deps on `//:ark_resolver`, `@pypi//pytest`, `@pypi//sanic_testing`
- [x] Tag the `test_http_registry_rust` target `requires-network` and `external`
- [x] Rewire `just pytest` to `bazel test //tests/...`
- [x] Run `just pytest` before the rewire (maturin) and after it (Bazel, summing the `test.xml` outputs) on the same commit; the passed-test counts are equal
- [x] Rewire `just build` to build `//:_rust`, remove stale `ark_resolver/_rust*.so`, and copy `_rust.so` from `bazel-bin` into `ark_resolver/` (for pyright and IDE resolution only)
- [x] Rewire `just run` to `bazel run //:ark_resolver_bin -- -s` with an absolute `ARK_REGISTRY` (see Dev loop: uv's static interpreter cannot load the Bazel `.so`)
- [x] Keep `pycheck: build` in the justfile, so `just pycheck` (ruff format, ruff check, pyright via uv) always runs against a freshly built extension
- [x] Run `just pycheck`; it passes
- [x] Run `just run` and request `/health`; the server answers `ok`
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: OCI image, healthcheck and smoke test

- [x] Add `oci.pull` of `gcr.io/distroless/cc-debian13` pinned by digest for `linux/amd64`, with the digest-refresh command as a comment
- [x] Add `mtree_spec` (`include_runfiles = True`), `mtree_mutate` (`srcs`, `package_dir = "app"`, `mtime = "0"`, `preserve_symlinks = True`) and `tar` (`zstd`) over the untransitioned `//:ark_resolver_bin`
- [x] Drop the interpreter's `site-packages/pip` and `ensurepip` from the mtree
- [x] Package a stripped interpreter (stripped python-build-standalone variant, or a strip step on the Linux-configured files)
- [x] Wrap the finished tar in `platform_transition_filegroup` to `//platforms:linux_x86_64`
- [x] Add `oci_image` `//:image` with the entrypoint, cmd, env, workdir, user and port from Technical Considerations, and `target_compatible_with = ["@platforms//os:linux"]`
- [x] Add stamped image labels and remote tags via `expand_template` with `STABLE_*` substitutions and unstamped fallbacks
- [x] Add `oci_load` `//:image_load` (`daschswiss/ark-resolver:latest`)
- [x] Add `oci_push` `//:image_push` to `docker.io/daschswiss/ark-resolver`
- [x] Add `just docker-build` running `bazel run --config=release --stamp --platforms=//platforms:linux_x86_64 //:image_load`
- [x] Add `just docker-publish` running `//:image_push` with the computed tags
- [x] Keep `just docker-image-tag`, now sourcing the version from `version.txt`
- [x] Add `just image-check`, which runs these assertions against the loaded image with `docker run --entrypoint <interpreter>`:
  - the entrypoint interpreter path exists;
  - `import ark_resolver._rust, sanic, uvloop, httptools` succeeds;
  - `pip` and `wheel` are not importable;
  - `/etc/ssl/certs/ca-certificates.crt` and `/usr/share/zoneinfo` exist;
  - the process runs as uid 65532.
- [x] Run `readelf -d` on the image's `_rust.so` and confirm every `NEEDED` library is present in `cc-debian13`
- [x] Grep `ark_resolver/` for cwd-relative file access; none depends on `/app`
- [x] Document the healthcheck (`/app/healthcheck`) in `README.md`'s deployment section
- [x] Add that probe as the `healthcheck` of `docker-compose.yml`'s service
- [x] Run the probe with `docker exec` against a running container of the Bazel image; it exits 0 while healthy and non-zero after the server is stopped
- [x] Add `rust_test` `//tests:smoke_test` over `tests/smoke_test.rs` with its dev crates, tagged `manual`, `no-sandbox`, `external`, `requires-network`, `requires-docker`
- [x] Make `smoke_test.rs` find `docker-compose.yml` and write `docker-compose-test-failure.yml` via `BUILD_WORKSPACE_DIRECTORY` (falling back to the current directory), because a Bazel test runs in the runfiles tree; pass `-f <path>` to every `docker compose` call
- [x] Rewire `just smoke-test` to run `just docker-build`, then `just image-check`, then `bazel test //tests:smoke_test`
- [x] Run `just smoke-test`; it passes (health, `/convert`, all 13 redirect-parity paths, the registry-fetch-failure case)
- [x] Compare the loaded image's size (`docker image inspect`) with the baseline in Success Metrics; it is not larger
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: CI and release cutover

- [x] Add `.github/actions/ci-setup/action.yml` with `bazel-contrib/setup-bazel` (bazelisk), `extractions/setup-just@v3`, `astral-sh/setup-uv` with the pinned uv version, and repository-cache restore on `~/.cache/bazel-repo` keyed by OS, arch and `hashFiles('MODULE.bazel.lock')`
- [x] Save the repository cache at the end of every Bazel job with `actions/cache/save` under `!cancelled()`
- [x] Rewrite `check.yml` so its jobs call `just rustcheck` and `just pycheck`
- [x] Rewrite `test.yml`'s `test` job to call `just test` and `just pytest`
- [x] Remove `test.yml`'s `docker/setup-buildx-action` and `docker/build-push-action` steps from the `smoke` job
- [x] Rewrite `test.yml`'s `smoke` job to call `just smoke-test`, then run the Docker Scout compare and CVE scan on the loaded image
- [x] Rewrite `publish.yml`'s `publish` job to call `just docker-publish` and generate the SBOM from the pushed image
- [x] Keep the Jenkins stage and prod webhook jobs, Docker Scout environment recording and the Sentry release job unchanged (they consume `just docker-image-tag`)
- [x] Change `security.yml`'s cargo-audit job to run `just audit`
- [x] Keep `security.yml`'s pip-audit and dependency-review unchanged
- [x] Set `release-type: simple` at the top level and in `packages["."]` of `.github/release-please/config.json`
- [x] Remove the `cargo` and `docker` ecosystems from `.github/dependabot.yml`
- [x] Switch Dependabot's `pip` ecosystem to `uv`
- [x] Replace `setup-python` 3.13 with 3.12 in jobs that run uv (`pycheck`, pip-audit), and drop it from jobs that only run Bazel
- [x] Push the branch; every workflow check on the PR is green
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: Retire the legacy build and update the docs

- [x] Delete `Cargo.toml` and `Cargo.lock`
- [x] Delete `build.rs` and `rust-toolchain.toml`
- [x] Delete `Dockerfile`, `.dockerignore` and `entrypoint.sh`
- [x] Delete the obsolete `Makefile` and `vars.mk` (ARCH-MAP.md:156)
- [x] Remove maturin from `pyproject.toml` and `uv.lock`
- [x] Remove the `cargo`, `nextest` and `docker buildx` recipes from the `justfile`
- [x] Update `CLAUDE.md` Setup, Build, Testing, Code Quality and Docker sections to the Bazel recipes
- [x] Update `CLAUDE.md`'s Project Overview so it no longer promises the Axum/PyO3-removal phase
- [x] Update `README.md` development and deployment sections to the Bazel recipes
- [x] Update `ARCH-MAP.md` `build-and-delivery`: paths, version source `version.txt`, release-type `simple`, Bazel as the only build path
- [x] Update `ARCH-MAP.md` `http-service`'s "Used by" line, which names `entrypoint.sh` and the Dockerfile HEALTHCHECK
- [x] Update `eng.yaml`: add a `devops-reviewer` override for `MODULE.bazel`, `**/BUILD.bazel`, `.bazelrc`, `bazel/**`, `platforms/**`, `tools/**`
- [x] Complete `docs/adr/0002-build-with-bazel.md`: context, decisions (Bazel-first on the hybrid, `from_specs`, own PyO3 toolchain, distroless `cc-debian13` with an explicit-interpreter entrypoint, `uv.lock` as the Python lock), consequences
- [x] Add decision D20 to the migration-completion PRD: Bazel builds the hybrid; the Axum rewrite (Phase 2) is not pursued; supersedes D18's sequencing and Phase 3a's `from_cargo` / cargo-backstop wording, with the reasons from Alternative Approaches Considered
- [x] Run `grep -rnE 'cargo |maturin|Dockerfile|nextest' --exclude-dir=specs --exclude-dir=.git --exclude-dir=.venv .`; only intentional mentions remain (CHANGELOG history, `cargo audit`, `Cargo.Bazel.lock`)
- [x] Run `just check && just test && just pytest`; it passes
- [x] Run `just smoke-test`; it passes
- [x] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Set `deploy_healthcheck` for ark in `ops-deploy/ark.yml` to `["CMD", "/app/healthcheck"]` as documented in `README.md` (Phase 4), with `interval: 60s`, `timeout: 10s`, `retries: 3`, `start_period: 60s` (today's Dockerfile HEALTHCHECK timing; the role defaults differ), and merge it | Infrastructure (Lukas or Samuel) or Ivan | After this PR is merged and stage runs the Bazel image; before the first Bazel prod release (ops-deploy#1445) | Change to shared deployment config in another repo, reviewed by Infrastructure |
| H2 | Update branch protection required checks if workflow or job names changed in Phase 5 | Ivan (repo admin) | Before merge | Repository admin setting |
| H3 | Review the Docker Scout CVE compare on the PR (Alpine to distroless Debian 13) and accept or block. Includes setuptools' vendored `wheel` (setuptools arrives transitively via sanic) | Ivan | Before merge | Security acceptance decision |
| H4 | After merge, confirm the stage ARK host resolves known ARKs and reports healthy before cutting the next release | Ivan | After merge, before the next release | Production-readiness sign-off; prod deploys automatically on release |

## Acceptance Criteria

- [x] `just check && just test && just pytest` passes, and no recipe invokes `cargo build`, `cargo test`, `maturin` or `docker build`
- [x] Rust unit-test and pytest passed-test counts under Bazel equal the cargo and maturin counts on the same commit
- [x] `just smoke-test` passes against the Bazel-built image, including `just image-check`
- [x] The image is built by `rules_oci` on digest-pinned distroless `cc-debian13`, runs as uid 65532, and contains no pip or wheel
- [x] Image tags match the pre-migration tag scheme
- [x] The documented healthcheck probe exits 0 against a healthy container and non-zero otherwise
- [x] CI workflows call only `just` recipes; the PR's checks are green
- [x] release-please versions `version.txt`
- [x] `Cargo.toml`, `Cargo.lock`, `Dockerfile`, `entrypoint.sh`, `build.rs`, `rust-toolchain.toml`, `Makefile` and `vars.mk` no longer exist
- [x] `CLAUDE.md`, `README.md`, `ARCH-MAP.md` and `eng.yaml` describe the Bazel build
- [x] ADR-0002 records the decisions
- [x] The PRD carries decision D20

## Dependencies & Risks

- **Depends on** #185, #186, #187 and #189 (this branch is stacked on them). #187's rustls floor must carry over into the `crate.spec` pins.
- **External.** H1 in ops-deploy lands **after** this PR is on stage (corrected 2026-09-28, ops-deploy#1445). ark runs as a Swarm stack and Swarm replaces unhealthy tasks; the probe's interpreter path exists only in the Bazel image, so merging H1 first would restart-loop the old Alpine image. Until H1 lands, the Bazel image on stage runs without a healthcheck, which is harmless.
- **Rust dependency updates become manual.** Dependabot cannot bump `crate.spec` entries. `just crates-repin` plus `just audit` in `security.yml` is the stopgap. Renovate's `bazel-module` manager extracts `crate_spec` and `bazel_dep`, but does not regenerate `Cargo.Bazel.lock`. That makes it a follow-up, not part of this plan.
- **Unreleased upstream fix.** rules_rust PR #4256 (`lint_config` on `pyo3_extension`) is in no release yet. The aspects-on-the-command-line approach works around it. Revisit when rules_rust is bumped.
- **sipi's base image.** sipi also pins `distroless/base-debian12`, which is now end-of-life. That is outside this plan; flag it to sipi separately.
- **Follow-ups (not in this plan):**
  - Split `src/core` into its own `rust_library` so ADR-0001's direction becomes a build error. This needs the `pyo3::Python::initialize()` test coupling in `uuid_processing.rs` removed first.
  - A multi-arch image.
  - Remote cache: sipi's NativeLink could be reused.
  - Split the image app layer into interpreter, site-packages and app, using aspect_rules_py's layer groups as the model.

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Extension links libpython dynamically (no extension-module mode) and fails to load in the image | L | H | Spike loaded it under the hermetic interpreter; `just image-check` imports it in the container; fallback is a second, extension-mode pyo3 hub via crate_universe `isolate` |
| Own `rust_pyo3_toolchain` with PyO3 0.29 fails with `rules_rust_pyo3` 0.70.0 (spike on macOS passed) | L | H | Phase 2 builds and imports `_rust.so` before anything depends on it; `rust_shared_library` + rename fallback, as quarylabs/sqruff does |
| `rust_test` cannot find the hermetic stdlib on Linux or under remote execution (the macOS spike needed no env) | L | M | Phase 2 proves one test first; Phase 5 CI covers Linux; `sh_test` wrapper setting `PYTHONHOME` (sqruff pattern) |
| The explicit-interpreter entrypoint fails in the shell-less image (spike passed on arm64 and amd64; the runfiles repo name is hard-coded) | L | H | `just image-check` asserts the path and imports; fallback is a base with a shell, recorded in ADR-0002 |
| `from_specs` resolves different transitive crate versions than `Cargo.lock` (sipi hit a latent timing bug this way) | M | M | Phase 1 diff against `Cargo.lock`; pin back any non-patch drift |
| A resolved crate needs a newer Rust than 1.89.0 (rules_rust does not enforce `rust-version`) | L | M | Phase 2 builds and tests on 1.89.0 against the baselines; raise the pin in ADR-0002 if a crate fails |
| Behaviour change from musl/Alpine to glibc/Debian 13 (DNS, TLS roots, IPv6; `ARK_RUST_FORCE_IPV4` exists for container IPv6 issues) | M | H | `just image-check`, then the smoke test's HTTPS registry fetch and redirect parity; H4 stage check before release |
| Wrong-arch interpreter or wheels in the image | L | H | `platform_transition_filegroup` to `linux_x86_64`; `current_config=fail`; `just image-check` imports the extension in the container |
| Hermetic toolchain fetch flakiness (Apple SDK CDN 403s seen in sipi) | L | M | Downloader-config mirrors; repository cache in CI |
| Image larger than today because the interpreter is unstripped (the spike's unstripped image was about 392 MB against 382 MB) | M | L | Stripped interpreter and pip removal in Phase 4; size check against the Success Metrics baseline |
| Loss of the image-level HEALTHCHECK | M | L | `/app/healthcheck` binary + H1 |

## Success Metrics

- **Parity.** The Rust unit-test and pytest passed-test counts under Bazel equal the counts from the cargo and maturin recipes, run on the same commit before each rewire.
- **Image size.** The baseline is the size Docker Hub reports for `daschswiss/ark-resolver:1.14.1` amd64, 382 MB as of 2026-09-26. The Bazel image, measured the same way after the first push (or with `docker image inspect` on the loaded image as a proxy), is not larger.
- **Behaviour.** `just smoke-test` passes, covering 13 redirect-parity paths, `/convert` and health. After merge, stage passes H4.
- **One build path.** The Phase 6 grep finds no `cargo build`, `cargo test`, `maturin` or `docker build` invocation.

## References

- sipi Bazel setup:
  - `sipi/MODULE.bazel` (versions, crate specs, llvm ordering, `oci.pull`)
  - `sipi/.bazelrc`
  - `sipi/platforms/BUILD.bazel`
  - `sipi/src/BUILD.bazel:341-787` (image, stamping, push)
  - `sipi/.github/actions/ci-setup/action.yml`
  - `sipi/CONVENTIONS.md:201-233` (visibility)
  - `sipi/tools/workspace_status.sh`
  - `sipi/justfile` (`audit`, rustfmt and clippy aspect recipes)
- ark-resolver today:
  - `justfile`, `Cargo.toml`, `build.rs`, `Dockerfile`
  - `.github/workflows/{check,test,publish,security}.yml`
  - `.github/release-please/config.json`
  - `tests/smoke_test.rs`
  - `ARCH-MAP.md:150-175`
  - `docs/adr/0001-adopt-hexagonal-architecture.md`
- ops-deploy healthcheck hook: `ops-deploy/roles/deploy/defaults/main.yml:19-27`, `ops-deploy/ark.yml:28-75`
- rules_rust PyO3:
  - `extensions/pyo3/` at tag 0.70.0
  - https://bazelbuild.github.io/rules_rust/rust_pyo3.html
  - https://github.com/bazelbuild/rules_rust/pull/4256
- crate_universe bzlmod: https://bazelbuild.github.io/rules_rust/crate_universe_bzlmod.html
- rules_python:
  - https://rules-python.readthedocs.io/
  - https://github.com/bazel-contrib/rules_python/issues/2500 (no shell-less launcher)
- aspect_rules_py#390 (wrong-arch interpreter in images): https://github.com/aspect-build/rules_py/issues/390
- distroless support policy (debian12 end of life): https://github.com/GoogleContainerTools/distroless/blob/main/SUPPORT_POLICY.md
- Worked example of rules_rust PyO3 + rules_python: quarylabs/sqruff (`MODULE.bazel`, `crates/lib/BUILD.bazel`, `crates/cli-python/BUILD.bazel`)
- release-please simple strategy: https://github.com/googleapis/release-please/blob/main/src/strategies/simple.ts
- Learnings:
  - `dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md`
  - `dasch-specs/learnings/best-practices/bzlmod-dev-dependency-upstream-tools-unconsumable-vendor.md`
  - `dasch-specs/learnings/build-errors/zig-cc-glibc-header-contamination-musl-target.md`
  - `dasch-specs/learnings/best-practices/docker-image-publish-workflow-design.md`
  - `docs/learnings/integration-issues/pyo3-rust-python-shadow-execution-parity.md`
