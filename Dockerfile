# ---- Build Rust Extension for Multi-Arch ----
FROM rust:1.87.0-alpine3.21 AS builder
COPY --from=ghcr.io/astral-sh/uv:0.6.2 /uv /uvx /bin/

RUN \
    apk add --no-cache \
    build-base \
    musl-dev \
    gcc \
    ca-certificates \
    python3 \
    python3-dev \
    pkgconf \
    gcompat \
    perl \
    && rm -rf /var/cache/apk/*

# - Copy from the cache instead of linking since it's a mounted volume,
# - tell uv to byte-compile packages for faster application startups,
# - prevent uv from accidentally downloading isolated Python builds,
# - pick a Python version,
# - and finally declare `/app` as the target for `uv sync` (venv directory).
ENV UV_LINK_MODE=copy \
    UV_COMPILE_BYTECODE=1 \
    UV_PYTHON_DOWNLOADS=never \
    UV_PYTHON=python3.12 \
    UV_PROJECT_ENVIRONMENT=/app

# Synchronize DEPENDENCIES without the application itself.
# This layer is cached until uv.lock or pyproject.toml change, which are
# only temporarily mounted into the build container since we don't need
# them in the production one.
# You can create `/app` using `uv venv` in a separate `RUN`
# step to have it cached, but with uv it's so fast, it's not worth
# it, so we let `uv sync` create it for us automagically.
RUN --mount=type=cache,target=/root/.cache \
    --mount=type=bind,source=uv.lock,target=uv.lock \
    --mount=type=bind,source=pyproject.toml,target=pyproject.toml \
    uv sync \
        --locked \
        --no-install-project

# Detect target architecture and set appropriate Rust target
ARG TARGETPLATFORM
RUN echo "Building for platform: $TARGETPLATFORM"

# Copy the rest of the project source code and install it
ADD . /app
WORKDIR /app

# Build the Rust Python package with maturin for the correct architecture
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
        uv run maturin develop --release --target aarch64-unknown-linux-musl; \
    else \
        uv run maturin develop --release --target x86_64-unknown-linux-musl; \
    fi

# ---- Setup Python Service ----
FROM python:3.12-alpine3.21 AS runtime

# Install necessary dependencies
RUN apk add --no-cache \
    shadow \
    libgcc \
    libstdc++ \
    jq \
    curl


# Don't run your app as root.
RUN \
    groupadd -r app \
    && useradd -r -d /app -g app -N app

# Copy the pre-built `/app` directory to the runtime container
# and change the ownership to user app and group app in one step.
COPY --from=builder --chown=app:app /app /app

USER app
WORKDIR /app

# Optional: add the application virtualenv to search path.
# We set the venv in the previous stage to `/app`, so we add it to the PATH.
ENV PATH=/app/bin:$PATH
ENV PYTHONPATH=/app:/app/lib/python3.12/site-packages
ENV VIRTUAL_ENV=/app

# Verify the installation of the ark_resolver module
# Strictly optional, but I like it for introspection of what I've built
# and run a smoke test that the application can, in fact, be imported.
RUN \
    python3 -V \
    && python3 -m site \
    && python3 -c "import sys; print(sys.path)" \
    && python3 -c 'import ark_resolver; print(ark_resolver)' \
    && python3 -c 'from ark_resolver import _rust; print(_rust)'

# Enable periodic container health check
HEALTHCHECK --start-period=60s --interval=60s --timeout=10s --retries=3 \
  CMD curl -sS --fail "http://127.0.0.1:${ARK_INTERNAL_PORT:-3336}/health" \
      | jq -r .status \
      | grep '^ok$' \
      || exit 1

COPY --chmod=0755 entrypoint.sh /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]
CMD ["-s"]
