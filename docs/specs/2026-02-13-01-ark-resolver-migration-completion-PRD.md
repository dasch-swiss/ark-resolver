---
title: "ARK Resolver Migration Completion: Pure Rust, Bazel-Built"
status: draft
date: 2026-02-13
author: Ivan Subotic
repositories:
  - ark-resolver-data
linear_project: "Migrate Ark-Resolver to Rust"
linear_project_id: fc0274ef-a867-47e0-be19-18fc3dcfcead
supersedes: "2025-06-06-01-structured-logging-PRD.md (observability approach only)"
---

# ARK Resolver Migration Completion: Pure Rust, Bazel-Built

## TL;DR

Complete the Python-to-Rust migration of the DSP ARK Resolver by adding parallel validation to the main redirect route, replacing the Python/Sanic server with Axum, introducing a Bazel build (single-repo now, monorepo-ready), migrating configuration from INI to TOML, and adopting an OpenTelemetry observability pipeline that exports traces, metrics, and logs to Grafana Cloud (with Sentry retained for error tracking). The service continues to be deployed the standard DaSCH way — a container image published to Docker Hub and rolled out to DaSCH infrastructure via Jenkins. The end state is a standalone Rust binary with zero Python dependencies.

---

## Context

The DSP ARK Resolver is a long-running service (since ~2015) that resolves ARK persistent identifiers to resource URLs for the DaSCH Service Platform. ARK URLs are permanent identifiers embedded in academic publications — they must remain resolvable indefinitely.

The service currently runs as a hybrid Python/Rust application:

- **Python (Sanic)** serves HTTP and handles routing
- **Rust (via PyO3/maturin)** provides core business logic through a Python extension module
- **Configuration** is loaded from INI files hosted in the `ark-resolver-data` repository, fetched via raw GitHub URL at startup and on-demand via a GitHub webhook-triggered `/reload` endpoint

All 7 core business logic modules have been ported to Rust using hexagonal architecture (ADR-0001), and a parallel execution framework validates behavioral parity on the `/convert` endpoint. The main `/redirect` route has since had shadow execution added and its parity gate verified (see Phase 1).

Phase 1 of the migration is complete. This PRD drives the remaining work to reach the end state: a pure Rust binary, Bazel-built, deployed on DaSCH infrastructure.

### Current Infrastructure

- **Hosting**: Docker Swarm on a University of Basel ITS VM (`its-bs-dasch-meta-01.dasch.swiss`), Traefik + Let's Encrypt fronted, co-located with Meta/MLS/WordWeb/Mosaic
- **Deployment**: Jenkins + Ansible (via the `ops-deploy` repository); images built by GitHub Actions
- **Stage**: Automatic deployment after merge to main (GitHub Actions → Jenkins webhook)
- **Production**: Automatic deployment on GitHub release (release-please tag → Jenkins webhook); the same image is re-deployed to stage
- **Images**: Published to Docker Hub (`daschswiss/ark-resolver`)

A DaSCH-owned bare-metal cluster (`dasch-virt-*`, managed in `ops-tf`) exists and some services have moved to it, but **ark-resolver has not migrated** — it still runs on the ITS VM. Moving it there later is a possible follow-up (out of scope here).

This is the same deployment pattern used by our other services (e.g. `dpe`/dsp-repository and dsp-api): build a container image, push to Docker Hub, deploy via Jenkins. This migration **keeps that pattern** — it does not move the service to a new hosting platform.

### Current Config Reload Flow

1. Changes pushed to `ark-resolver-data` → GitHub webhook fires
2. `POST /reload` on ark-resolver validates `X-Hub-Signature` with shared secret
3. Service immediately reloads config from `ARK_REGISTRY` (raw GitHub URL)
4. Schedules another reload 5 minutes later (GitHub CDN cache expiry)

This webhook mechanism will be eliminated in favor of a stateless model: config is baked into the container image at build time, and a config change in `ark-resolver-data` triggers a new image build + redeploy through the standard Docker Hub + Jenkins pipeline. No runtime dependency on GitHub availability, and no more shared-secret webhook endpoint.

---

## Goals

1. **Complete migration validation** — Add parallel execution to the redirect route; run Rust shadow alongside Python in production for 2 weeks with zero discrepancies
2. **Eliminate Python runtime** — Replace Python/Sanic with Axum, remove PyO3, produce a standalone Rust binary
3. **Modernize the build** — Introduce a Bazel build for ark-resolver (`rules_rust` + `rules_oci`), self-contained in the repo now and structured so the eventual move into the DaSCH monorepo is a mechanical relocation
4. **Simplify deployment** — Keep the standard DaSCH pipeline (image → Docker Hub → Jenkins), but make it stateless: config baked into the image, redeploy on config change; eliminate the GitHub webhook reload mechanism and its shared secret
5. **Modernize configuration** — Migrate ark-resolver-data registry from INI to TOML; config is embedded in the image at build time
6. **Reduce operational overhead** — Eliminate dual-language maintenance and reduce container image size
7. **Update test infrastructure** — Migrate ark-resolver-data tests to work with the pure Rust service
8. **Full observability via OpenTelemetry** — Grafana Cloud is the primary observability stack: export traces, metrics, and logs via OTLP, following the DaSCH house pattern. Keep Sentry only for 5xx/broken-invariant error tracking (4xx client errors are not sent to Sentry). Defer Grafana dashboards and alerting.

---

## Core Features

### Phase 1: Redirect Route Validation (prerequisite)

- Add Rust shadow execution to the `/<path:path>` redirect route using the existing `ParallelExecutor`
- **Cache Rust settings at startup** alongside Python settings (not per-request HTTP fetch like the convert route currently does). Reload Rust settings in `reload_config()` alongside Python settings.
- Run both Python and Rust implementations for every redirect request
- Compare computed redirect URL strings (exact match — normalize URL encoding if needed)
- Log discrepancies via Sentry (matching existing convert route pattern) and structured logs
- Deploy to production, monitor for 2 weeks
- **Gate**: Zero discrepancies before proceeding to Phase 2 — **VERIFIED PASSED (2026-07-13)**: 17,536 redirect shadow executions over 90d in Sentry at 100% match, corroborated by Grafana Loki (14,212 matches / 0 mismatches over 30d). See DEV-5871.

### Phase 2: Axum HTTP Service

- **Simplify Rust architecture** — The current hexagonal architecture (42 files, ports/use_cases/adapters layers) is over-engineered for a redirect service. Flatten to a simple module structure:
  - Keep domain logic files as-is (`check_digit.rs`, `uuid_processing.rs`, `ark_url_info.rs`, `ark_url_formatter.rs`) — they're already pure functions
  - Remove the ports layer (`src/core/ports/`) — traits with single implementations add indirection without value
  - Remove the use_cases layer (`src/core/use_cases/`) — pure pass-through wrappers (e.g., `CheckDigitValidator.is_valid()` just calls `check_digit::is_valid()`)
  - Consolidate 5 error modules into a single `error.rs`
  - Simplify `SettingsManager` from 6 `Arc<dyn Trait>` parameters to direct function calls
  - Target: ~7 files instead of 42
- Implement Axum HTTP server with equivalent routes:
  - `GET /<path>` — ARK redirect (HTTP 302)
  - `GET /convert/<ark_id>` — V0-to-V1 conversion (JSON response)
  - `GET /health` — Health check
  - ~~`GET /config`~~ — Dropped (less surface area; also closes the old `/config` secret-leak issue, DEV-3597)
- Load configuration at startup from embedded config file (baked into image) or `ARK_REGISTRY` env var fallback for local development
- **Listen on a configurable port** via the `PORT`/`ARK_*` env var (default 8080)
- **OpenTelemetry instrumentation** — Instrument the Axum service with the DaSCH house OTel stack, mirroring `dpe` (dsp-repository): `init-tracing-opentelemetry` (`TracingConfig::production()`) + `opentelemetry-otlp` (grpc-tonic), exporting **traces, metrics, and logs**. Configuration is driven entirely by standard `OTEL_*` env vars, with a **no-op export fallback when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset** (safe for local dev). Route panics through `tracing`. `RUST_LOG` controls verbosity. Pyroscope profiling optional/deferred.
- **Logging** — Continue writing structured logs to **stdout** (captured by the host's Grafana Alloy collector, as today) **and** bridge `tracing` logs to OTLP → Grafana Loki via `opentelemetry-appender-tracing`. Both paths kept: stdout is a free backstop, OTLP gives single-pane-of-glass in Grafana. (This service's log volume is tiny, so the extra Loki ingest is negligible.)
- Sentry integration for **error tracking of 5xx / broken-invariant failures only** — gate the capture on `status >= 500`; client errors (4xx) are NOT sent to Sentry (they go to Grafana via logs/metrics/traces). **Do not send PII** (change from current `send_default_pii=True`)
- CORS middleware via `tower-http` (permissive, matching current behavior)
- Graceful error handling with proper HTTP status codes
- Address DEV-5182: use crypto library instead of own algorithms
- Address DEV-5906: port structured error diagnostics to Rust, and give **each `ArkErrorCode` an explicit HTTP status class**. 4xx (client error): malformed ARK, bad check digit, bad UUID, invalid chars, **unknown project/resource**. 5xx (our bug): a *known* project with a missing/malformed config template, unexpected state, panics. **Only 5xx reaches Sentry.** Note: production shadow logs show Python and Rust diverge in error *representation* here — unknown project `084D` throws a raw Python `KeyError` (which would surface as a 500 and wrongly hit Sentry) vs Rust `ArkUrlException`; the Rust service must classify unknown-project as a clean 4xx. Align the Rust error surface with the Python-side structured errors from DEV-5907.
- Remove all Python code (`ark_resolver/`, `pyproject.toml`, maturin config)
- Remove PyO3 adapter layer (`src/adapters/pyo3/`)
- **CLI mode**: The current `ark.py` also serves as a command-line tool (`-i`, `-a` flags for IRI↔ARK conversion). Port this as a subcommand of the Rust binary (e.g., `ark-resolver serve` vs `ark-resolver convert`)
- **Build/packaging deferred to Phase 3**: Phase 2 is built and tested with `cargo`; it does **not** introduce a new container image or deployment change. Because we do not want to build any new container tooling before Bazel (see D17/D18), production continues to run the last released hybrid image until the Phase 3 cutover. The existing `/reload` webhook keeps config updates flowing to production during that window. (This is a deliberate, bounded exception to the "every phase deploys to prod" cadence — see D14.)

### Phase 3: Bazel Build + Deployment

Two chunks of work, potentially two PRs (see D14):

**3a — Bazel build (single-repo bridge):**

- Introduce a Bazel build for ark-resolver using **bzlmod (`MODULE.bazel`)**, `rules_rust` + Crate Universe (generated from `Cargo.toml`, kept as the dependency source of truth), and `rules_oci` for the container image
- Mirror the Sipi Bazel migration's conventions and rules versions so the setup is consistent with, and mechanically relocatable into, the DaSCH monorepo (see D18)
- Keep `cargo` working in parallel as a backstop/local-dev build; validate `bazel build`/`bazel test` parity
- No new container tooling exists before this step — the OCI image is Bazel-native from the first time it is built

**3b — Deployment (Docker Hub + Jenkins, DaSCH infrastructure):**

- Build the OCI image with `rules_oci` and publish to Docker Hub (`daschswiss/ark-resolver`), then deploy via Jenkins + Ansible — the same pattern as `dpe`/dsp-api
- **Config baked into image**: the Bazel image build embeds the latest config from ark-resolver-data. No runtime dependency on GitHub availability.
- GitHub Actions workflow in ark-resolver:
  - On merge to main: build image → push to Docker Hub → Jenkins deploys to **staging**
  - On release (release-please): publish the release-tagged image → deploy to **production** (per current Jenkins practice)
- GitHub Actions workflow in ark-resolver-data: on push to main → trigger an ark-resolver image rebuild + redeploy (picks up new config)
- Remove the `/reload` webhook endpoint and `ARK_GITHUB_SECRET` — no longer needed
- **Enable the per-stack Grafana Alloy sidecar** (`deploy_alloy: true` in `ops-deploy`) so the app's OTLP (traces/metrics/logs) is forwarded to Grafana Cloud, mirroring dpe (see D15)
- **No change to the production host or DNS.** `ark.dasch.swiss` keeps pointing where it does today (Docker Swarm on the ITS meta VM); this phase only changes how the image is built and rolled out. No Cloud Run, no GCP for production, no preview environments (see D17).

### Phase 4: Configuration Modernization

- Migrate ark-resolver-data registry from INI to TOML format
  - Automated migration script (Rust CLI or one-time conversion) to convert existing INI to TOML
  - **TOML schema design**: INI's `[DEFAULT]` section inheritance has no TOML equivalent. Use an explicit `[defaults]` table + per-project tables that only specify overrides, with merge logic in Rust's `serde` deserialization (e.g., `#[serde(default)]` from the defaults table)
  - Maintain backwards compatibility during transition (support both formats temporarily)
- TOML parser in ark-resolver using the `toml` + `serde` crates — typed deserialization validates schema at parse time
- **Dead code cleanup**: Remove `UsePhp`, `PhpResourceRedirectUrl`, `PhpResourceVersionRedirectUrl`, and `resource_int_id_factor` — these are vestigial (the `UsePhp` flag is never read by the application code; project 0816/Vitrosearch gets standard DSP redirects)
- **Data quality fix**: Normalize `Host` values (e.g., project `[084A]` has `app.dasch.Swiss` with capital S)
- Update ark-resolver-data README with new format documentation

### Phase 5: Test Infrastructure

- Migrate ark-resolver-data Tavern/pytest tests to HTTP integration tests (curl-based or Rust integration tests)
- **Container smoke test in CI** — `docker run` the built Bazel `oci_image`, hit it with a set of known ARKs, and assert the 302 `Location` (and `/convert` output). This is the per-PR check on the *real deployed artifact* (image packaging, startup, config-baking, port binding) — the coverage a preview deployment would have given, at a fraction of the cost (see D17)
- Smoke tests targeting the staging environment
- CI/CD pipeline validation for both repos
- Remove Python test dependencies from ark-resolver-data (`requirements.txt`, Makefile)

---

## User Stories

1. As a **DaSCH developer**, I want the redirect route to run Rust in shadow mode so that I can validate behavioral parity on the main production traffic path before cutover.

2. As a **DaSCH operator**, I want a pure Rust binary, built and deployed like our other services (Bazel image → Docker Hub → Jenkins), so that the service has a minimal footprint, no Python runtime maintenance, and no bespoke hosting to look after.

3. As a **DaSCH developer**, I want ark-resolver to build with Bazel in a monorepo-ready way so that adopting the monorepo later is a mechanical move rather than a rewrite.

4. As a **DaSCH developer**, I want the configuration in TOML format so that it's type-safe, supports comments, and aligns with the Rust ecosystem.

5. As a **DaSCH operator**, I want config changes in ark-resolver-data to automatically trigger a new image build + redeploy so that the service always reflects the latest registry without manual intervention or webhook complexity.

6. As a **DaSCH developer**, I want the tests in ark-resolver-data to work against the new Rust service so that project registry changes are validated end-to-end.

7. As a **DaSCH operator**, I want traces, metrics, and logs from ark-resolver in Grafana Cloud (alongside our other services) so that I can debug and monitor it from the same place as the rest of the platform.

---

## Acceptance Criteria

| Story | Criteria |
|-------|----------|
| 1 | Redirect route runs both Python + Rust in parallel; mismatches logged to Sentry; zero discrepancies over 2 weeks of production traffic |
| 2 | `ark-resolver` produces a single static Rust binary; container image has no Python runtime; deployed via Docker Hub + Jenkins; `ark.dasch.swiss` continues to resolve with no host/DNS change |
| 3 | ark-resolver builds with Bazel (`bazel build`/`bazel test` green); the OCI image is produced by `rules_oci`; BUILD files and lockfiles are structured for a mechanical monorepo move |
| 4 | ark-resolver-data registry files converted to TOML; Rust parser loads TOML natively; INI files removed |
| 5 | GitHub Actions workflow in ark-resolver-data triggers an ark-resolver image rebuild + redeploy on push to main; the new image embeds the latest config |
| 6 | ark-resolver-data CI runs integration tests against the Rust service; all existing test scenarios pass |
| 7 | ark-resolver traces, metrics, and logs appear in Grafana Cloud (Tempo/Mimir/Loki) via OTLP; Sentry receives only 5xx/broken-invariant errors (4xx client errors are not sent); no PII sent |

---

## Constraints

- **Zero-downtime**: `ark.dasch.swiss` must remain operational throughout migration — ARK URLs are permanent identifiers used in academic publications
- **Backwards compatibility**: All existing ARK URLs must continue to resolve correctly after each phase
- **Config source**: ark-resolver-data remains the single source of truth for project registry data
- **Deployment substrate**: Keep the standard DaSCH pipeline (image → Docker Hub → Jenkins → DaSCH infrastructure). Do **not** introduce a new hosting platform (no Cloud Run/GCP) for this service (see D17)
- **No containerization before Bazel**: no new container image or deploy tooling is built before the Bazel bridge (Phase 3a); the OCI image is Bazel-native from the start (see D17/D18)
- **Team**: Primarily Claude Code + Ivan — no external team dependencies
- **Crypto debt**: DEV-5182 (use crypto library instead of own algorithms) should be addressed during Phase 2
- **One PR per phase**: Each phase is implemented as a separate GitHub PR, merged independently. Every merged state must be production-ready — with one bounded exception: Phase 2 lands without a deploy (prod holds on the last hybrid release until the Phase 3 cutover; see Phase 2 and D14). Use the Linear-generated branch name from the issue to automatically link the PR back to the Linear issue.

---

## Success Criteria

1. Pure Rust binary (no Python dependencies) serving all ARK resolution traffic
2. Built with Bazel (`rules_rust` + `rules_oci`), in a monorepo-ready layout
3. Deployed via Docker Hub + Jenkins to DaSCH infrastructure; `ark.dasch.swiss` unchanged (same host, same DNS)
4. Config changes in ark-resolver-data automatically trigger a new image build + redeploy
5. Registry format is TOML
6. All existing ARK URLs continue to resolve correctly
7. Container image size reduced significantly (no Python runtime); service startup under ~2 seconds
8. Grafana Cloud (via OpenTelemetry) is the primary observability stack for traces, metrics, and logs; Sentry tracks only 5xx/broken-invariant errors (no 4xx); no PII sent

---

## Out of Scope (YAGNI)

- Earthly build system integration (canceled — DEV-5271)
- **Cloud Run / GCP for production** — explicitly out of scope; ark-resolver stays on the standard DaSCH pipeline (see D17)
- **Preview (per-PR) deployments** — not worth it for a UI-less redirect service; the built-artifact check they'd provide is covered by a CI container smoke test instead (Phase 5, D17)
- **Moving ark-resolver into the DaSCH monorepo** — the Bazel setup is built to make this a later mechanical relocation, but the move itself is a separate, follow-up effort (see D18)
- Grafana dashboards and alerting for ark-resolver (deferred — get raw telemetry flowing to Grafana Cloud first; build dashboards/alerts later once we see what's useful)
- Custom Pyroscope continuous profiling (optional; wire the endpoint later if needed — the OTel stack leaves room for it)
- New ARK resolution features or format changes
- Multi-region deployment
- API versioning
- User authentication beyond existing mechanisms
- Analytics dashboards (can be added later if needed)

---

## Repository Impact

### ark-resolver

- Remove all Python code (`ark_resolver/`, `pyproject.toml`, maturin config, `uv.lock`)
- Remove PyO3 adapter layer (`src/adapters/pyo3/`)
- Remove parallel execution framework (`parallel_execution.py`) — no longer needed after validation
- Flatten hexagonal architecture: remove ports and use_cases layers, consolidate errors
- Add Axum HTTP server (`main.rs`)
- Add TOML config parser (replace INI parsing)
- Sentry instrumentation for **5xx / broken-invariant errors only** (4xx client errors not sent) **+ OpenTelemetry instrumentation (traces, metrics, logs)** via the DaSCH house stack (`init-tracing-opentelemetry` + `opentelemetry-otlp`, grpc-tonic), mirroring `dpe`; env-driven with no-op local fallback
- Add Bazel build: `MODULE.bazel` (bzlmod), `rules_rust` + Crate Universe (from `Cargo.toml`), `rules_oci` for the image — monorepo-ready, mirroring Sipi's setup
- New GitHub Actions: build image (Bazel) → Docker Hub → Jenkins deploy (replacing the current build/publish workflow)
- Updated justfile, CI workflows
- Remove `/reload` endpoint and GitHub webhook handling (`ARK_GITHUB_SECRET`)
- Address DEV-5182 (crypto library)
- Address DEV-5906 (port structured error diagnostics to Rust)

### ark-resolver-data

- Convert `data/dasch_ark_registry.ini` → `data/dasch_ark_registry.toml`
- Convert `data/dasch_ark_registry_staging.ini` → `data/dasch_ark_registry_staging.toml`
- Migrate tests from Python/Tavern to HTTP-based integration tests
- New GitHub Actions: on push to main → trigger an ark-resolver image rebuild + redeploy
- Remove Python test dependencies (`requirements.txt`, Makefile)
- Update README with TOML format documentation and new project onboarding workflow

---

## Observability Approach

**Grafana is DaSCH's primary observability stack, and ark-resolver follows suit** — full OpenTelemetry (traces, metrics, logs) to Grafana Cloud, so the service is debugged and monitored from the same place as everything else. Sentry is kept only as a focused, high-signal error tracker for real bugs (see below). (This revises the earlier "Sentry-only" stance from 2026-07-13.)

- **Traces**: OpenTelemetry spans exported via OTLP → Grafana Cloud Tempo.
- **Metrics**: OTel metrics (RED + any app counters/histograms) via OTLP → Grafana Cloud (Mimir/Prometheus).
- **Logs**: structured logs to **stdout** (shipped to Grafana Loki by the host's Alloy collector, as today) **and** bridged to OTLP → Grafana Cloud Loki. Both paths are kept because this service's log volume is tiny, so mirroring to Loki costs almost nothing and gives single-pane-of-glass in Grafana.
- **Error tracking**: **Sentry — for 5xx / broken-invariant failures only.** A Sentry event should mean a bug in our code (unhandled exception, violated assumption, panic). **Client errors (4xx) are NOT sent to Sentry** — they're normal operating data and live in Grafana (logs/metrics/traces). Implementation: gate the capture on `status >= 500`. This keeps Sentry near-silent in steady state, so anything that appears there is real. Good UX + MCP support enables debugging with Claude. **Do not send PII** (change from current `send_default_pii=True`).
- **Export mechanism**: the app emits OTLP via standard `OTEL_*` env vars to a **per-stack Grafana Alloy sidecar** (`deploy_alloy: true` in `ops-deploy`), which forwards to Grafana Cloud — the DaSCH house pattern on the Swarm/VM substrate, as `dpe` runs today (ark currently has `deploy_alloy: false`; this migration enables it). No-op export when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset (safe local dev). Concrete Alloy + endpoint/auth config is lifted from dpe's stack in Phase 3.
- **Dashboards & alerting**: **deferred** (YAGNI) — get raw telemetry flowing first, then build dashboards/alerts once we know what's worth watching.

> **Note on the current deployment**: today, ark-resolver runs on University of Basel VMs and its container stdout is shipped to Grafana Cloud Loki by the host's Grafana Alloy collector (`service_name="ark-prod-01_ark"` / `ark-stage-01_ark`). Since this migration keeps the VM/Jenkins deployment, **that host-level Alloy log path continues to work**; the OTLP export (via the per-stack Alloy sidecar) adds traces and metrics (and a second log path) on top of it.

During Phase 1: discrepancies were logged to Sentry (and stdout→Loki) for investigation — gate verified clean.

---

## Resolved Decisions

Decisions made during discovery and specification review:

| # | Decision | Resolution |
|---|----------|-----------|
| D1 | `UsePhp` code path | **Dead code** — the flag exists in config but the application never reads it. `PhpResourceRedirectUrl`, `PhpResourceVersionRedirectUrl`, and `resource_int_id_factor` are vestigial. Remove during TOML migration (Phase 4). |
| D2 | Rust settings for Phase 1 shadow execution | **Cache at startup** — load Rust settings once at app startup alongside Python settings, reload together in `reload_config()`. Do not re-fetch from HTTP per request (as the convert route currently does). |
| D3 | Config loading | **Bake into image** — config is embedded from ark-resolver-data during the container image build step. No runtime dependency on GitHub availability. |
| D4 | Current production hosting | **Docker Swarm on a University of Basel ITS VM** (`its-bs-dasch-meta-01.dasch.swiss`, Traefik-fronted), deployed by Jenkins + Ansible via `ops-deploy`. Stage: auto after merge to main; prod: auto on GitHub release (release-please). Retained (see D17). A DaSCH-owned bare-metal cluster (`ops-tf`) exists but ark has not migrated. _Corrected 2026-07-17 — was "University of Basel VMs … manual Jenkins button"; verified against ops-deploy/infra + live telemetry._ |
| D5 | Deployment flow | **Standard DaSCH pipeline (image → Docker Hub → Jenkins), same as `dpe`/dsp-api.** Stage: auto-deploy after merge to main. Prod: deploy on release (release-please), per current Jenkins practice. _Revised 2026-07-15: previously specified Cloud Run — dropped (see D17)._ |
| D6 | Observability approach | **OpenTelemetry → Grafana Cloud** for traces, metrics, and logs (via OTLP, standard `OTEL_*` env vars, no-op locally), following the DaSCH house pattern — **Grafana is the primary stack**. **Sentry retained for 5xx/broken-invariant error tracking only** (see D16). Grafana dashboards/alerting deferred. _Revised 2026-07-13, superseding the earlier "Sentry-only / no Grafana integration" stance and partially re-adopting PRD-001's direction._ |
| D7 | Sentry PII | **Do not send PII** — the current `send_default_pii=True` should be changed in the Axum service. |
| D8 | Build system | **Bazel** (`rules_rust` + Crate Universe + `rules_oci`), self-contained in the ark-resolver repo, built to be monorepo-ready (see D18). _Revised 2026-07-15: previously "one GCP project, multiple Cloud Run services" — dropped with Cloud Run (see D17)._ |
| D9 | `/config` endpoint | **Drop it** — less surface area, and it removes the old secret-leak issue (DEV-3597). |
| D10 | Observability depth | **Full OTel signal export (traces + metrics + logs) to Grafana Cloud.** Logs go to stdout (→ Alloy → Loki, as today) **and** OTLP → Loki, justified by this service's low log volume. Dashboards and alerting deferred (YAGNI). _Revised 2026-07-13._ |
| D11 | CORS | **Keep permissive CORS** in Axum (via `tower-http` middleware). Redirect route is called from browsers. |
| D12 | Phase 3/4 ordering | **Deployment first (Phase 3), TOML second (Phase 4)**. Ship Axum + INI through the new Bazel/Docker Hub/Jenkins pipeline, then change the config format in a subsequent release. One change at a time. |
| D13 | Rust architecture simplification | **Fold into Phase 2** — flatten hexagonal architecture (42 files → ~7) when replacing PyO3 with Axum. Remove ports layer (single-implementation traits), use_cases layer (pass-through wrappers), and consolidate error modules. Domain logic files kept as-is. |
| D14 | Implementation cadence | **One PR per phase** — each phase is a separate GitHub PR. Every merged state is production-ready, with **one bounded exception**: Phase 2 (pure Rust, cargo-built) lands without a deploy, because no container tooling is built before the Bazel bridge (D17/D18). Prod holds on the last hybrid release until the Phase 3 cutover; config keeps flowing via the existing `/reload` webhook during that window. |
| D15 | OTel export mechanism | **OTLP to a per-stack Grafana Alloy sidecar** (`deploy_alloy: true` in `ops-deploy`) that forwards to Grafana Cloud — the DaSCH house pattern on the Swarm/VM substrate, as `dpe` runs today. (The earlier "direct OTLP to Grafana Cloud, no sidecar" wording assumed Cloud Run, since dropped — D17; on the VM/Swarm substrate the sidecar is the export path.) App-side: the dpe Rust OTel stack (`init-tracing-opentelemetry` + `opentelemetry-otlp` grpc-tonic + `opentelemetry-appender-tracing`), env-driven via `OTEL_*` with a no-op local fallback. Concrete Alloy/endpoint config lifted from dpe's stack in Phase 3. _Added 2026-07-13; corrected 2026-07-17._ |
| D16 | Sentry scope | **Sentry receives only 5xx / broken-invariant errors** (unhandled exceptions, violated assumptions, panics) — the signals that indicate a bug in our code. **4xx client errors (malformed ARK, bad UUID, unknown project/resource, etc.) are NOT sent to Sentry**; they are normal operating data captured in Grafana (logs/metrics/traces). Implementation: gate the Sentry capture on `status >= 500`. Unknown-project/-resource classifies as 4xx. Keeps Sentry a high-signal channel, near-silent in steady state. _Added 2026-07-14._ |
| D17 | Deployment target — **not Cloud Run** | **Production stays on the standard DaSCH pipeline: image → Docker Hub → Jenkins → DaSCH infrastructure** (same as `dpe`/dsp-api prod). The earlier PRD's "deploy prod on GCP Cloud Run" was an inherited assumption with no recorded rationale; on inspection, DaSCH uses GCP/Cloud Run **only for ephemeral per-PR preview deployments** (dpe/Mosaic, auto-deleted on merge) — never for production, and dsp-api uses no GCP at all. For a permanent-identifier service, longevity and vendor-neutrality argue against pinning production to a managed serverless product. **Preview deployments are also skipped** for ark-resolver: its behavior is deterministic ARK→URL redirection with no UI, so the one thing a preview would add — exercising the real built image/startup — is captured instead by a per-PR container smoke test in CI (Phase 5). (DaSCH does run previews elsewhere, e.g. dsp-app and the incubator, where a live UI needs eyeballs; that rationale doesn't apply here.) Corollary: **no new containerization before Bazel** — the OCI image is Bazel-native from the first build. _Added 2026-07-15._ |
| D18 | Bazel bridge & monorepo | **Adopt Bazel now, in the ark-resolver repo**, using bzlmod + `rules_rust` (Crate Universe from `Cargo.toml`) + `rules_oci`, mirroring the Sipi Bazel migration's conventions/versions. Write BUILD files and lockfiles so the eventual **move into the DaSCH monorepo is a mechanical relocation**, not a rebuild. The monorepo move itself is a separate follow-up (out of scope here). Sequenced **after** Phase 2 so Bazel meets a clean single-language Rust crate rather than the PyO3/maturin hybrid. _Added 2026-07-15._ |
| D19 | Config format | **TOML** for the ark-resolver-data registry. KDL was considered — it reads nicely for a records-style registry and its Rust support is fine (official `kdl` crate + serde bridges `serde_kdl`/`knurdy`, or `knus` derive) — but TOML wins on serde maturity (Cargo-grade `toml` crate), tooling ubiquity, and repo consistency. **Low-stakes and reversible**: the registry is a small, flat dataset that converts between formats trivially, so revisiting KDL (or anything else) later is cheap. _Added 2026-07-15._ |

## Open Questions

All major questions have been resolved (see Resolved Decisions table). Remaining items to determine at implementation time:

1. **Jenkins/Ansible wiring**: Confirm the exact `ops-deploy` changes needed to build+deploy the Bazel-produced image and to trigger a rebuild+redeploy on ark-resolver-data pushes (cross-repo dispatch). Mirror the `dpe` CD workflow.
2. **Bazel version alignment**: Pin the same `rules_rust`/`rules_oci`/toolchain versions as the Sipi Bazel setup so the monorepo move stays mechanical (D18).
3. **Alloy sidecar / Grafana Cloud config**: Lift the per-stack Alloy config and Grafana Cloud credentials from dpe's `ops-deploy` stack during Phase 3 (D15).

---

## Phasing & Linear Project Structure

This PRD maps to the existing Linear project "Migrate Ark-Resolver to Rust" (`fc0274ef`). Each phase is tracked as a separate Linear issue:

| Phase | Description | Linear Issue | Branch | Dependency |
|-------|-------------|-------------|--------|------------|
| Phase 1 | Redirect Route Validation | [DEV-5871](https://linear.app/dasch/issue/DEV-5871) | `feature/dev-5871-phase-1-redirect-route-validation` | None |
| Phase 2 | Axum HTTP Service | [DEV-5872](https://linear.app/dasch/issue/DEV-5872) | `feature/dev-5872-phase-2-replace-sanic-with-axum-pure-rust-http-service` | DEV-5871 |
| Phase 3 | Bazel Build + Deployment | [DEV-5874](https://linear.app/dasch/issue/DEV-5874) | `feature/dev-5874-phase-3-cloud-run-deployment` | DEV-5872 |
| Phase 4 | Config Modernization (INI → TOML) | [DEV-5873](https://linear.app/dasch/issue/DEV-5873) | `feature/dev-5873-phase-4-config-modernization-ini-toml` | DEV-5874 |
| Phase 5 | Test Infrastructure | [DEV-5875](https://linear.app/dasch/issue/DEV-5875) | `feature/dev-5875-phase-5-test-infrastructure` | DEV-5873 |

Phases are numbered in execution order. **Phase 3 (DEV-5874) is re-scoped from "Cloud Run Deployment" to "Bazel build + deployment via Docker Hub + Jenkins"** — the Linear issue title/branch still say "cloud-run" and should be updated. Phase 3 may be split into two PRs (3a Bazel build, 3b deployment). Deployment (Phase 3) before TOML migration (Phase 4) — one change at a time, deploy Axum + INI first, then switch format.

**Status (2026-07-15):** Phase 1 complete — parity gate verified (see DEV-5871). Phases 2–5 in backlog; Phase 2 is unblocked. No implementation-plan docs exist yet for Phases 2–5 (only this PRD and the Phase 1 plan).

---

*Last updated: 2026-07-17 — Verified all deployment/hosting statements against `ops-deploy`/`infra`/`ops-tf` + live telemetry: current hosting is Docker Swarm on a University of Basel ITS VM (ark has **not** migrated to DaSCH-owned hardware); prod deploys automatically on release (not a manual Jenkins button); OTel export is via a per-stack Grafana Alloy sidecar as `dpe` does (not "direct, no sidecar"). Prior 2026-07-15 revision: dropped Cloud Run (production stays on the standard image → Docker Hub → Jenkins pipeline; Cloud Run/GCP and preview deployments out of scope, D17), added a monorepo-ready Bazel build bridge (D8/D18), re-scoped Phase 3 to "Bazel build + deployment", recorded the config-format decision (D19). Builds on the 2026-07-13/14 observability revisions (full OpenTelemetry to Grafana Cloud, Sentry for 5xx only; D6/D10/D15/D16). Originally authored 2026-02-14 by Ivan Subotic.*
