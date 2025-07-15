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
rustcheck:
    just --check --fmt --unstable
    cargo +nightly fmt --check
    cargo clippy -- -D warnings

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
pytest: build
    uv run pytest

# Run ark-resolver locally
run: build
    export ARK_REGISTRY="ark_resolver/ark-registry.ini" && uv run ark_resolver/ark.py -s -c ark_resolver/ark-config.ini

# Run Rust unit tests
test:
    @echo "🧪 Running Rust unit tests..."
    cargo test --lib --no-default-features
    @echo "✅ Rust unit tests completed successfully!"
    @echo "💡 Use 'just pytest' to run comprehensive Python integration tests"

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
