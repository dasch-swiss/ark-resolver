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

# Run all fmt and clippy checks
check:
    just --check --fmt --unstable
    cargo +nightly fmt --check
    cargo clippy -- -D warnings

# Format all rust code
fmt:
    cargo +nightly fmt

# Fix justfile formatting. Warning: will change existing file. Please first use check.
fix:
    just --fmt --unstable

# Build Rust using maturin
build: install
    uv run maturin develop

# Run ark-resolver Python unit tests which require Rust code
pytest: build
    uv run pytest

# Run Rust unit tests
test: build
    cargo test --lib

# Run smoke tests that will spinn up a Docker container and call the health endpoint
smoke-test: build
    cargo test --tests smoke_test

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
