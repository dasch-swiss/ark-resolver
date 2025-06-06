# Structured Logging and Observability Upgrade for ARK Resolver Migration

### TL;DR

ARK Resolver is transitioning from Sentry spans to a unified
observability model using structured JSON logs (for failures and
discrepancies only) and aggregate metrics, both shipped via
OpenTelemetry to Grafana Cloud (Loki). This centralizes monitoring for
ARK Resolver with all other DaSCH services, enables robust surface area
for top issues and migration validation, and provides privacy-safe
aggregation of user impact while minimizing operational cost and log
volume.

------------------------------------------------------------------------

## Goals

### Business Goals

- Centralize all observability, monitoring, and analytics for DaSCH
  products—including ARK Resolver—within Grafana Cloud for unified
  analysis, integration, and operational efficiency.

- Support a safe, verifiable migration from Python to Rust by
  automatically surfacing discrepancies without logging routine
  successes.

- Enable deep operational analytics and health dashboards for internal
  use (failure rates, top issues, user impact) without unnecessary log
  bloat.

- Maintain adaptability for future, in-house/self-hosted deployments.

- Fully comply with privacy requirements by ensuring no raw user IPs are
  included in persistent storage.

### User Goals

- Grant DaSCH developers and ops real-time insight into ARK request
  failure rates and the nature of those failures.

- Make ARK validation and migration issues highly discoverable and
  traceable using targeted logs.

- Surface the most common request issues, enabling proactive debugging
  and faster support.

- Quantify unique users impacted by issues, without exposing their
  identities.

### Non-Goals

- No external alerting, paging, or fireside escalation processes.

- No logging of successful requests at individual level.

- No expansion of user tracking beyond unique-user hashing.

- No external-facing dashboards or analytics for third parties.

------------------------------------------------------------------------

## User Stories

**User Persona: DaSCH Developer / Operator**

- As a DaSCH developer, I want to view failures, error types, and
  discrepancies for ARK requests so I can quickly debug and resolve
  urgent issues.

- As a DaSCH operator, I want to validate that the new Rust service
  matches current Python behavior, so we can confidently migrate.

- As a DaSCH developer, I want to visualize metrics in Grafana so I can
  monitor system health and usage without relying on high log volumes.

- As a DaSCH developer, I want to identify the ARKs and projects most
  frequently failing, so effort can be focused on systemic problems.

- As a DaSCH operator, I want to see how many unique users experience
  issues so we can quantify and address real-world impact.

- As a DaSCH developer, I want to see hourly/daily/weekly request rates,
  so that unusual activity or usage trends are immediately visible.

------------------------------------------------------------------------

## Functional Requirements

- **Logging Features (Priority: High)**

  - Log only requests resulting in:

    - Failure (invalid input, unresolved ARK/resource, etc.)

    - Discrepancy between Python and Rust handler results

  - Each relevant log shall capture:

    - timestamp (ISO format)

    - ark (full ARK string requested)

    - project (parsed from path segment if present)

    - error_type (categorize: invalid input, unresolved ARK,
      discrepancy)

    - user_hash (one-way hashed source IP)

    - python_result (success/error or output detail from Python handler)

    - rust_result (success/error or output detail from Rust handler)

    - discrepancy (flag if Python and Rust results differ)

  - Format logs as structured JSON sent via OpenTelemetry to Grafana
    Cloud Loki.

  - Immediately transform and hash user IP addresses before logging.

- **Metrics & Observability (Priority: High)**

  - Emit metrics via OpenTelemetry for:

    - Total requests (aggregate)

    - Successful requests

    - Failed requests (by failure type)

    - Requests with discrepancy between Python and Rust outputs

  - Metrics dimensions should include time bucket, project, and (where
    feasible) ARK.

  - All dashboards and failure rate calculations to use metrics for
    volume/rate and percent computations, not logs.

- **Migration Support & Implementation Parity (Priority: High)**

  - Process every request through both Python and Rust handlers.

  - On discrepancies, log details of both results and flag as such.

  - Default ARK resolution for the user is Python-based until migration
    completes.

  - Maintain clear taxonomy of error types.

- **Dashboarding Features (Priority: Medium)**

  - Enable Grafana dashboards/widgets to:

    - Display top N most recent/frequent failing ARKs (from logs).

    - Show overall and period-based (hour/day/week) failure/discrepancy
      rates (from metrics).

    - Present unique user (user_hash) counts for failed/discrepant
      requests.

    - Visualize total and error request volumes and trends over time
      (from metrics).

- **Privacy and Retention (Priority: High)**

  - Never log or retain raw IP addresses; store only hash values with
    secure, secret salt.

  - Logs default to Grafana Cloud’s standard retention; revisit if
    retention requirements change.

------------------------------------------------------------------------

## User Experience

**Entry Point & First-Time User Experience**

- DaSCH developers and ops access Grafana dashboards through internal
  authentication.

- No onboarding required; dashboards and log queries are immediately
  usable upon rollout.

**Core Experience**

- **Step 1:** User opens the Grafana dashboard for ARK Resolver logs and
  metrics.

  - Instantly visible: recent failed/discrepant ARKs, top issues, and
    aggregate request volumes/rates.

  - Logs searchable by ARK, project, error type, or user_hash for
    failures only.

- **Step 2:** User investigates a spike in failed/discrepant requests.

  - Dashboards surface top failing ARKs and affected projects.

  - User drills down to error types and requests showing Python/Rust
    discrepancies.

- **Step 3:** User measures impact.

  - Widget shows unique users affected by failures/discrepancies.

  - Time-series charts display all request, error, and discrepancy rates
    over time.

- **Step 4:** User exports log slices for deeper forensic analysis where
  needed.

  - All queries and exports remain internal and privacy-compliant.

**Advanced Features & Edge Cases**

- Power users can construct ad hoc Grafana queries over failure logs and
  metrics.

- Discrepancy flag supports immediate surfacing of migration mismatches.

- Logs handle and note missing fields gracefully (e.g., missing project
  path).

**UI/UX Highlights**

- Grafana dashboards feature sortable tables, time pickers, top-N
  widgets, and color-coded error/discrepancy rates.

- Widgets are optimized for low-latency query response and real-time
  updating.

- All logs and dashboards are strictly internal and user-anonymized.

------------------------------------------------------------------------

## Narrative

A DaSCH developer, midway through the ARK Resolver’s migration to Rust,
notices a rise in failure metrics for a key project. Instead of wading
through a sea of logs, she opens the new Grafana dashboard to find
immediate metrics: top failing ARKs, error rate over time, and a split
between user error and migration-induced discrepancies.

Drilling into failure logs, she pinpoints the requests where the Python
and Rust handlers diverged—surfaced automatically by the discrepancy
flag. With an accurate count of unique users impacted (through
anonymized hashes), she can triage urgency and collaborate with the Rust
migration lead to resolve the edge-case bug.

Thanks to this targeted, privacy-respecting observability setup, the
team can rapidly respond to outages, validate migration correctness, and
provide robust reporting to product management—all from a unified,
cost-effective Grafana environment.

------------------------------------------------------------------------

## Success Metrics

**User-Centric Metrics**

- Number of DaSCH developers/ops regularly using Grafana dashboards for
  ARK monitoring.

- Rate of successful issue identification and remediation using
  metrics/logs.

- Frequency of dashboard searches/queries.

**Business Metrics**

- Consistency of ARK Resolver observability within the unified Grafana
  Cloud monitoring suite.

- Migration completeness as evidenced by declining discrepancy rates and
  increased confidence.

**Technical Metrics**

- 100% of failing/discrepant ARK requests logged with all required
  fields and privacy controls.

- Accurate, timely emission of all observability metrics as specified.

- No raw IP addresses present in any persisted or queried logs.

**Tracking Plan**

- Metric-based tracking of total, successful, failed, and discrepant
  requests per time period.

- Log-based breakdown of error/discrepancy types and frequencies.

- Count of unique user_hash values for failed/discrepant requests.

- Percentages and trends of requests with discrepancies between
  implementations.

- Volume and types of Grafana dashboard queries performed.

------------------------------------------------------------------------

## Technical Considerations

### Logging and Metrics Patterns

- **Structured logging:** Only failed or discrepant ARK requests are
  logged, with all required context and user-hash. Successes are not
  individually logged.

- **Metrics coverage:** All requests (successful and otherwise) emit
  metrics for total, success, failure (by type), and discrepancy; these
  metrics drive dashboards and rate calculations.

- **User privacy:** User IP addresses are transformed into salted
  one-way hashes before any logging/aggregation. No raw IPs are ever
  written or stored.

- **Data retention:** Logs and metrics follow Grafana Cloud’s default
  retention settings (adjustable as needed).

- **OpenTelemetry integration:** Ensures consistency and ease of
  transport across Python and Rust implementations.

- **Log schema:** Maintain a defined, documented schema for all logged
  fields.

### System Architecture

- Implement logging/metric emission in both Python and Rust ARK Resolver
  codebases.

- On migration: every ARK request is processed by both handlers;
  discrepancies between handlers are logged as special cases.

- Logs and metrics are shipped to Grafana Cloud’s Loki and metrics
  backends, supporting seamless migration to self-hosting if required.

### Integration Points

- Sentry spans deprecated post-migration/validation.

- Grafana dashboards serve as the sole observability surface for ARK
  Resolver and all DaSCH internal users.

### Scalability & Performance

- Logging only errors/discrepancies keeps log volume—and
  cost—manageable, even at high throughput.

- Metric emission is lightweight and suitable for very high cardinality
  and event rates.

- Performance impact on request handling remains minimal due to
  selectivity in logging.

### Privacy & Compliance

- Compliant with GDPR and Swiss law for operational logs.

- Hashed user IDs used only for aggregate stats; no user identification
  or cross-service matching.

- Internal documentation to make rationales and guarantees on privacy
  explicit.

### Challenges & Risks

- Ensuring true functional parity and rapid discrepancy flagging in
  migration phase.

- Maintaining appropriate log/metric cardinality to avoid unneeded cost
  spikes.

- Training team on where to use metrics vs. logs for subsequent
  observability needs.

------------------------------------------------------------------------

## Milestones & Sequencing

### Project Estimate

- 1–2 weeks for core implementation and rollout, assuming focus by a
  small team.

### Team Size & Composition

- 1–2 developers familiar with both Python and Rust, plus
  observability/logging concepts.

- (Optional) 1 Site Reliability Engineer or DevOps for OpenTelemetry,
  Grafana, and Loki integrations.

### Suggested Phases

**Phase 1: Metrics and Logging Foundation (3–4 days)**

- Define log schema.

- Implement failure/discrepancy logging and user_hash logic in Python
  and Rust.

- Integrate OpenTelemetry metric emission: total, success, failure (by
  type), discrepancy.

**Phase 2: Dual Processing & Discrepancy Handling (2–3 days)**

- Ensure every ARK request is processed and compared in both Python and
  Rust handlers.

- Log and tag discrepancies.

- Default to Python result for now.

**Phase 3: Grafana Dashboard Rollout (2–3 days)**

- Build widgets: top N failing ARKs, failure/discrepancy rates, unique
  users, traffic trends.

- Document internal usage for team.

**Phase 4: Parallel Run & Baseline Validation (4–5 days)**

- Team reviews logs and metrics.

- Live operational validation; iterate as needed.

**Phase 5: Sentry Sunset & Futureproofing (2 days)**

- Decommission Sentry for ARK Resolver logging.

- Complete documentation—internal privacy, observability patterns, and
  migration report.

------------------------------------------------------------------------
