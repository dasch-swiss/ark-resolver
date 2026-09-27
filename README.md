# The DSP ARK Resolver

Resolves [ARK](https://tools.ietf.org/html/draft-kunze-ark-18) URLs referring to
resources in [DSP](https://dsp.dasch.swiss/) (formerly called Knora) repositories.

## Project Status

The DSP ARK Resolver is a **hybrid Python/Rust application**, built end to end with Bazel. Logic continues to
migrate from Python to Rust inside that hybrid, in two phases:

1. **Phase 1 (Current)**: Add functionality to Rust and run it in parallel with the Python implementation to verify
   correct behavior in production, while Python behavior remains user-facing. Rust functions are exposed as a
   Python extension module via PyO3.
2. **Phase 2**: Change user-facing behavior to the Rust implementation and continue removing Python where the
   migration is complete.

### Architecture
- **Python (Sanic)**: Main HTTP server, routing, and business logic
- **Rust (PyO3)**: Performance-critical functions exposed as a Python extension module
- **Environment-driven configuration**: Uses environment variables with defaults, registry loaded from `ARK_REGISTRY`
 - **HTTPS via Rustls**: Rust HTTP client uses `rustls` with embedded Mozilla roots, avoiding dependency on system CA bundles

## Modes of operation

The binary `//:ark_resolver_bin` (entry point `ark_resolver/ark.py`) has two modes of operation:

- When run as an HTTP server, it resolves DSP ARK URLs by redirecting
  to the actual location of each resource. Redirect URLs are generated
  from templates in a configuration file. The hostname used in the
  redirect URL, as well as the whole URL template, can be configured per
  project.

  To start the ark-resolver as server, type:
  ```bash
  bazel run //:ark_resolver_bin -- -s
  ```

- The ark-resolver can also be used as a command-line tool for converting between
  resource IRIs and ARK URLs, using the same configuration file.

For usage information, run `bazel run //:ark_resolver_bin -- --help`. The application is configured entirely through environment variables, with a sample registry file available at `tests/ark-registry.ini` for local testing.

### Environment Variables

The application can be configured using the following environment variables:

- `ARK_EXTERNAL_HOST`: External hostname used in ARK URLs (default: `ark.example.org`)
- `ARK_INTERNAL_HOST`: Internal hostname for the server (default: `0.0.0.0`)
- `ARK_INTERNAL_PORT`: Port for the server to bind to (default: `3336`)
- `ARK_NAAN`: Name Assigning Authority Number (default: `00000`)
- `ARK_HTTPS_PROXY`: Whether behind HTTPS proxy (default: `true`)
- `ARK_REGISTRY`: Path or URL to the project registry file (**required**)
- `ARK_GITHUB_SECRET`: Secret for GitHub webhook authentication

### Rust HTTP Client Configuration (Advanced)

Additional environment variables for debugging and timeout control in containerized environments:

- `ARK_RUST_LOAD_TIMEOUT_MS`: Application-level timeout for settings loading (default: `15000`) - prevents container SIGTERM
- `ARK_RUST_HTTP_TIMEOUT_MS`: HTTP request total timeout in milliseconds (default: `10000`) 
- `ARK_RUST_HTTP_CONNECT_TIMEOUT_MS`: HTTP connection timeout in milliseconds (default: `5000`)
- `ARK_RUST_FORCE_IPV4`: Force IPv4-only connections, disable IPv6 (default: `false`) - fixes container IPv6 connectivity issues
- `RUST_LOG`: Controls tracing verbosity (e.g., `RUST_LOG=ark_resolver=debug,reqwest=debug,hyper=debug`)
- `ARK_SENTRY_DEBUG`: Enable Sentry debug mode (default: `false`) - accepts "true"/"1"/"yes"/"on" for true

The Rust HTTP client also supports standard proxy environment variables (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`).

For production deployments, `ARK_REGISTRY` should point to the appropriate registry file from the [ark-resolver-data](https://github.com/dasch-swiss/ark-resolver-data) repository.

In the sample registry file, the redirect URLs are DSP-API URLs,
but it is recommended that in production, redirect URLs should refer to
human-readable representations provided by a user interface.


## Requirements / local setup

The build is Bazel-driven; a Nix dev shell provides `bazelisk` (as `bazel`), `just`, `uv` and
`cargo-audit` so no host toolchain needs installing separately:

```bash
nix develop
# or, with direnv installed, just: direnv allow
```

Then install the Python dependencies (as defined in `pyproject.toml` and `uv.lock`):

```bash
just install
```

### Local Development

You can run the server locally with the convenient just command, which sets `ARK_REGISTRY` to the
repository's sample registry file and runs the Bazel-built binary:

```bash
just run
```

For a manual invocation, `ARK_REGISTRY` needs to be an absolute path, since `bazel run` executes in
its own runfiles directory:

```bash
export ARK_REGISTRY="$(pwd)/tests/ark-registry.ini"
bazel run //:ark_resolver_bin -- -s
```


## Examples for using the ark-resolver on the command-line

Each example needs `ARK_REGISTRY` set, as shown above.

### Converting a DSP resource IRI to an ARK URL

```
$ bazel run //:ark_resolver_bin -- -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ
https://ark.example.org/ark:/00000/1/0002/70aWaB2kWsuiN6ujYgM0ZQD
```

### Converting a DSP value IRI to an ARK URL with Timestamp

```
$ bazel run //:ark_resolver_bin -- -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ -d 20220119T101727886178Z
https://ark.example.org/ark:/00000/1/0002/70aWaB2kWsuiN6ujYgM0ZQD.20220119T101727886178Z
```

### Converting an ARK URL from a project on salsah.org to a custom resource IRI for import into DSP

```
$ bazel run //:ark_resolver_bin -- -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519 -r
http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ
```

### Redirecting an ARK URL from a resource created on salsah.org to the location of the resource on DSP

```
$ bazel run //:ark_resolver_bin -- -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519
http://0.0.0.0:4200/resource/0002/70aWaB2kWsuiN6ujYgM0ZQ
```


## A note about the creation of Resource IRIs from Salsah ARK URLs
As permanent identifiers, ARKs need to be valid for an unlimited period of time. So, after resources have been migrated 
from salsah.org to DSP, their ARK URLs need to stay valid. This means that the same ARK URL that formerly was redirected 
to a resource on salsah.org, now has to be redirected to the same resource on DSP. 

To enable the correct redirection of ARK URLs coming from salsah.org to resources on DSP the DSP resource IRI 
(which contains a UUID) needs to be calculated from the resource ID provided in the ARK. To do so, UUIDs of version 5 
are used. The DaSCH specific namespace used for the creation of UUIDs is `cace8b00-717e-50d5-bcb9-486f39d733a2`. It is 
created from the generic `uuid.NAMESPACE_URL` the Python library [uuid](https://docs.python.org/3/library/uuid.html) 
provides and the string `https://dasch.swiss` and is therefore itself a UUID version 5.

Projects migrated from salsah.org to DSP need to have parameter `AllowVersion0` set to `true` in their project 
configuration (registry file). Otherwise, the ARK URLs of version 0 are rejected.


## Server routes

```
GET /config
```

Returns the server's configuration, including the project registry, but not
including `ArkGitHubSecret`.

```
POST /reload
```

Accepts a GitHub webhook request in JSON, and validates it according to
[Securing your webhooks](https://developer.github.com/webhooks/securing/), using
the secret configured as `ArkGitHubSecret`. If the request is valid, reloads the
configuration, including the project registry. Changes to `ArkInternalHost` and
`ArkInternalPort` are not taken into account.


All other GET requests are interpreted as ARK URLs.


## Using Docker

Images are published to the [daschswiss/ark-resolver](https://hub.docker.com/r/daschswiss/ark-resolver)
Docker Hub repository.

### Basic Usage

```bash
docker run -p 3336:3336 daschswiss/ark-resolver
```

### Environment Configuration

The Docker container can be configured using environment variables:

```bash
docker run -p 3336:3336 \
  -e ARK_EXTERNAL_HOST="ark.example.org" \
  -e ARK_INTERNAL_HOST="0.0.0.0" \
  -e ARK_INTERNAL_PORT="3336" \
  -e ARK_NAAN="72163" \
  -e ARK_HTTPS_PROXY="true" \
  -e ARK_REGISTRY="tests/ark-registry.ini" \
  -e ARK_GITHUB_SECRET="your-webhook-secret" \
  daschswiss/ark-resolver
```

### Production Deployment

For staging and production deployments, set the registry file to load from the external repository:

```bash
# Staging
docker run -p 3336:3336 \
  -e ARK_REGISTRY="https://raw.githubusercontent.com/dasch-swiss/ark-resolver-data/master/data/dasch_ark_registry_staging.ini" \
  daschswiss/ark-resolver
```

**Note on TLS**: The Rust settings loader fetches the registry over HTTPS using `reqwest` with `rustls` and its
embedded Mozilla roots, so it does not depend on the runtime image's own CA bundle.

**Note on SIGTERM Prevention**: The Rust HTTP client includes application-level timeouts (15s default) to prevent container orchestrators from killing the service during slow HTTP requests. Use `ARK_RUST_LOAD_TIMEOUT_MS` to adjust if needed.

```bash
# Production
docker run -p 3336:3336 \
  -e ARK_REGISTRY="https://raw.githubusercontent.com/dasch-swiss/ark-resolver-data/master/data/dasch_ark_registry_prod.ini" \
  daschswiss/ark-resolver
```

### Healthcheck

The image has no shell, curl or jq, and OCI images have no `HEALTHCHECK` field, so the image ships a small healthcheck binary, `/app/healthcheck` (source: `tools/healthcheck/`). It requests `http://127.0.0.1:${ARK_INTERNAL_PORT:-3336}/health` and exits 0 only when the server answers `{"status": "ok"}`, printing the reason otherwise. Deployments declare:

```yaml
healthcheck:
  test: ["CMD", "/app/healthcheck"]
```

To try it locally, start the server with `just run` and run `just healthcheck` in another shell.

Production timing (ops-deploy's `deploy_healthcheck` must match): `interval: 60s`, `timeout: 10s`, `retries: 3`, `start_period: 60s`. `docker-compose.yml` uses shorter timings on purpose, for faster local feedback.

### Docker Compose

See `docker-compose.yml` for a complete example configuration.

### Building Images

Images are linux/amd64 only, built by `rules_oci` from the Bazel graph:

```bash
# Build the stamped release image and load it into the local Docker daemon
# as daschswiss/ark-resolver:latest
just docker-build

# Verify the loaded image's interpreter, imports, certs, tzdata and uid
just image-check

# Run docker-build, image-check, and the Bazel-driven Docker smoke test
just smoke-test
```

Publishing (`just docker-publish`, pushes the stamped image to Docker Hub) runs in CI only.
