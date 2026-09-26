DOCKER_REPO := "daschswiss/ark-resolver"
CARGO_VERSION := `cargo metadata --format-version=1 --no-deps | jq --raw-output '.packages[].version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG
DOCKER_LATEST := DOCKER_REPO + ":latest"

# List all recipes
default:
    just --list --unsorted

# Install python packages (as defined in pyproject.toml and uv.lock)
install:
    uv sync --locked --no-install-project

# Upgrade python packages (uv.lock)
upgrade:
    uv lock --upgrade

# Run all rust fmt and clippy checks
# Aspects run on the command line rather than via `lint_config` on the
# targets: `lint_config` on a `pyo3_extension` fails analysis before
# rules_rust PR #4256, which is in no release yet. `//:_rust_shared` is
# included because it is the only target compiled with the `pyo3` feature.
rustcheck:
    just --check --fmt --unstable
    bazel build //src:ark_resolver_lib //src:unit_tests //:_rust_shared --aspects=@rules_rust//rust:defs.bzl%rustfmt_aspect --output_groups=rustfmt_checks
    bazel build //src:ark_resolver_lib //src:unit_tests //:_rust_shared --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks --@rules_rust//rust/settings:clippy_flags=-Dwarnings

# Run all python checks
pycheck: build
    uv run ruff format --check .
    uv run ruff check .
    uv run pyright

# Run all checks
check: rustcheck pycheck

# Format all rust code
rustfmt:
    cargo +nightly fmt

# (Re)generate rust-project.json so rust-analyzer understands the Bazel crate
# graph (cargo can't see the rules_rust targets). The file is git-ignored.
rust-project:
    bazel run @rules_rust//tools/rust_analyzer:gen_rust_project

# Repin Cargo.Bazel.lock after a crate.spec change in MODULE.bazel.
crates-repin:
    CARGO_BAZEL_REPIN=1 bazel fetch @crates//:all

# Advisory scan over the checked-in Cargo-format lockfile (`Cargo.Bazel.lock`,
# materialized from `crate.from_specs` in MODULE.bazel) — cargo-audit reads
# Cargo lock syntax, not MODULE.bazel.lock's JSON. cargo-audit comes from the
# Nix dev shell.
audit:
    cargo audit --file Cargo.Bazel.lock

# Format all python code
pyfmt:
    uv run ruff format .
    uv run ruff check . --fix

# Format all code
fmt: rustfmt pyfmt

# Fix justfile formatting. Warning: will change existing file. Please first use check.
fix:
    just --fmt --unstable

# Build Rust using maturin
build: install
    uv run maturin develop

# Run ark-resolver Python unit tests which require Rust code
pytest:
    bazel test //tests/...

# Run ark-resolver locally
run: build
    export ARK_REGISTRY="tests/ark-registry.ini" && uv run ark_resolver/ark.py -s

# Run Rust unit tests
# ARK_REGISTRY is supplied via //src:unit_tests' `env` attribute, not here.
test:
    bazel test //src:unit_tests

# Run smoke tests that will spinn up a Docker container and call the health endpoint
smoke-test:
    cargo test --test smoke_test

# Clean up build artifacts
clean:
    cargo clean

# Build linux/amd64 Docker image locally
docker-build-intel:
    docker buildx build --platform linux/amd64 -t {{ DOCKER_IMAGE }} -t {{ DOCKER_LATEST }} --load .

# Build linux/arm64 Docker image locally
docker-build-arm:
    docker buildx build --platform linux/arm64 -t {{ DOCKER_IMAGE }} -t {{ DOCKER_LATEST }} --load .

# Build and push linux/amd64 and linux/arm64 Docker images to Docker hub
docker-publish-intel:
    docker buildx build --platform linux/amd64 -t {{ DOCKER_IMAGE }} --push .

# Output the BUILD_TAG
docker-image-tag:
    @echo {{ IMAGE_TAG }}
