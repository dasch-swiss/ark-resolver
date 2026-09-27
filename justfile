DOCKER_REPO := "daschswiss/ark-resolver"
# Single source of truth for the tag scheme: tools/workspace_status.sh derives
# STABLE_IMAGE_TAG from version.txt the same way Bazel's stamped image build does.
IMAGE_TAG := `tools/workspace_status.sh | awk '$1 == "STABLE_IMAGE_TAG" { print $2 }'`
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

# Aspects run on the command line rather than via `lint_config` on the
# targets: `lint_config` on a `pyo3_extension` fails analysis before
# rules_rust PR #4256, which is in no release yet. `//:_rust_shared` is
# included because it is the only target compiled with the `pyo3` feature.
[doc("Run all rust fmt and clippy checks")]
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
[doc("(Re)generate rust-project.json for rust-analyzer")]
rust-project:
    bazel run @rules_rust//tools/rust_analyzer:gen_rust_project

# Repin Cargo.Bazel.lock after a crate.spec change in MODULE.bazel.
crates-repin:
    CARGO_BAZEL_REPIN=workspace bazel fetch @crates//:all

# Advisory scan over the checked-in Cargo-format lockfile (`Cargo.Bazel.lock`,
# materialized from `crate.from_specs` in MODULE.bazel). cargo-audit reads
# Cargo lock syntax, not MODULE.bazel.lock's JSON. cargo-audit comes from the
# Nix dev shell.
[doc("Advisory scan over the checked-in Cargo lockfile")]
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

# Build the Rust extension so pyright and IDEs can resolve `ark_resolver._rust`.
# Not imported by `uv run`: uv's statically linked python-build-standalone
# interpreter can't load the Bazel .so, which links libpython3.12 dynamically.
[doc("Build the Rust extension for pyright and IDEs")]
build: install
    bazel build //:_rust
    rm -f ark_resolver/_rust*.so
    install -m 0644 bazel-bin/ark_resolver/_rust.so ark_resolver/_rust.so

# Run ark-resolver Python unit tests which require Rust code
pytest:
    bazel test //tests/...

# Run ark-resolver locally. `bazel run` executes in the runfiles dir, so
# ARK_REGISTRY needs an absolute path.
[doc("Run ark-resolver locally")]
run:
    ARK_REGISTRY="{{ justfile_directory() }}/tests/ark-registry.ini" bazel run //:ark_resolver_bin -- -s

# ARK_REGISTRY is supplied via //src:unit_tests' `env` attribute, not here.
[doc("Run Rust unit tests")]
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

# `--platforms=//platforms:linux_x86_64` is omitted: `:image_load` already
# consumes `:image_linux_amd64` (a `platform_transition_filegroup`) to
# transition itself, so passing it here would also transition `oci_load`'s
# runner script, pasting a Linux `tar` into a script the macOS host executes.
[doc("Build the stamped release image and load it into the local Docker daemon")]
docker-build:
    bazel run --config=release --stamp //:image_load

# No tag arguments: the pushed tag comes from the stamped `//:image_remote_tags`
# (STABLE_IMAGE_TAG only, no `latest`). Same --platforms omission as docker-build.
[doc("Push the stamped release image to Docker Hub")]
docker-publish:
    bazel run --config=release --stamp //:image_push

# Prints the same tag scheme //:image_remote_tags stamps into the pushed image
# (tools/workspace_status.sh is now the single source for it); consumed by
# .github/workflows/publish.yml.
[doc("Print the image tag derived from version.txt")]
docker-image-tag:
    @echo {{ IMAGE_TAG }}
