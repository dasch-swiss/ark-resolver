# The DSP ARK Resolver

Resolves [ARK](https://tools.ietf.org/html/draft-kunze-ark-18) URLs referring to
resources in [DSP](https://dsp.dasch.swiss/) (formerly called Knora) repositories.

## Project Status

The DSP ARK Resolver is a **hybrid Python/Rust application** currently undergoing migration from Python to Rust in three phases:

1. **Phase 1 (Current)**: Add functionality to Rust and run in parallel with Python implementation to verify correct behavior in production, while Python behavior remains user-facing. Rust functions are exposed as Python extensions via PyO3/Maturin.
2. **Phase 2**: Change user-facing behavior to Rust implementation and start removing Python components.
3. **Phase 3**: Refactor Rust code into a standalone service using Axum, completely removing Python dependencies.

### Architecture
- **Python (Sanic)**: Main HTTP server, routing, and business logic
- **Rust (PyO3)**: Performance-critical functions exposed as Python extensions
- **Environment-driven configuration**: Uses environment variables with defaults, registry loaded from `ARK_REGISTRY`
 - **HTTPS via Rustls**: Rust HTTP client uses `rustls` with embedded Mozilla roots, avoiding dependency on system CA bundles

## Modes of operation

The program `ark.py` has two modes of operation:

- When run as an HTTP server, it resolves DSP ARK URLs by redirecting
  to the actual location of each resource. Redirect URLs are generated
  from templates in a configuration file. The hostname used in the
  redirect URL, as well as the whole URL template, can be configured per
  project.

  To start the ark-resolver as server, type:
  ```bash
  python ark.py -s
  ```

- The ark-resolver can also be used as a command-line tool for converting between
  resource IRIs and ARK URLs, using the same configuration file.

For usage information, run `./ark.py --help`. The application is configured entirely through environment variables, with a sample registry file available at `tests/ark-registry.ini` for local testing.

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

First, install `uv`, which will automatically handle your Python installations,
virtual environments, and dependencies:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Then, create the virtual environment and install the dependencies with:

```bash
uv sync
```

### Local Development

For local development and testing, set the registry file environment variable:

```bash
export ARK_REGISTRY="tests/ark-registry.ini"
```

You can then run the server locally:

```bash
./ark.py -s
```

Or use the convenient just command:

```bash
just run
```


## Examples for using the ark-resolver on the command-line

### Converting a DSP resource IRI to an ARK URL

```
$ ./ark.py -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ
https://ark.example.org/ark:/00000/1/0002/70aWaB2kWsuiN6ujYgM0ZQD
```

### Converting a DSP value IRI to an ARK URL with Timestamp

```
$ ./ark.py -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ -d 20220119T101727886178Z
https://ark.example.org/ark:/00000/1/0002/70aWaB2kWsuiN6ujYgM0ZQD.20220119T101727886178Z
```

### Converting an ARK URL from a project on salsah.org to a custom resource IRI for import into DSP

```
$ ./ark.py -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519 -r
http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ
```

### Redirecting an ARK URL from a resource created on salsah.org to the location of the resource on DSP

```
$ ./ark.py -a http://ark.example.org/ark:/00000/0002-751e0b8a-6.2021519
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

**Note on TLS**: The Rust settings loader fetches the registry over HTTPS using `reqwest` with `rustls`. No `ca-certificates` package is required in the runtime image.

**Note on SIGTERM Prevention**: The Rust HTTP client includes application-level timeouts (15s default) to prevent container orchestrators from killing the service during slow HTTP requests. Use `ARK_RUST_LOAD_TIMEOUT_MS` to adjust if needed.

# Production
docker run -p 3336:3336 \
  -e ARK_REGISTRY="https://raw.githubusercontent.com/dasch-swiss/ark-resolver-data/master/data/dasch_ark_registry_prod.ini" \
  daschswiss/ark-resolver
```

### Docker Compose

See `docker-compose.yml` for a complete example configuration.

### Building Images

Multi-architecture images can be built using the provided just commands:

```bash
# For linux/amd64
just docker-build-intel

# For linux/arm64  
just docker-build-arm
```
