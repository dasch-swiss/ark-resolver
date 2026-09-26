# ARK Resolution

The ARK resolver turns a DaSCH persistent identifier into the location of the thing it identifies, and translates between ARK identifiers and DSP resource IRIs. It exists so that citations stay stable while hosts and applications change.

## Language

### Identifiers

**ARK**:
A persistent identifier of the form `ark:/<NAAN>/<ARK scheme version>/...` that names a DaSCH project, resource or value.
_Avoid_: PID (too general), permalink

**ARK URL**:
An ARK prefixed with the resolver's external host, e.g. `https://ark.dasch.swiss/ark:/72163/1/0803`.
_Avoid_: ARK link, resolver URL

**NAAN**:
The Name Assigning Authority Number that scopes every DaSCH ARK, `72163` in production.
_Avoid_: prefix, namespace

**ARK scheme version**:
The number after the NAAN that selects how the rest of an ARK is parsed: `1` for DSP ARKs, `0` for legacy SALSAH ARKs.
_Avoid_: ARK version, URL version (both collide with **Version**)

**Project shortcode**:
The four-hex-digit identifier of a DSP project, e.g. `0803`, used as the project segment of an ARK and as the registry section name.
_Avoid_: project ID, project_id (the code's name for it), project number

**Resource ID**:
The Base64url-encoded UUID of a DSP resource, as it appears in its IRI.
_Avoid_: UUID, resource UUID

**Escaped resource ID**:
A **Resource ID** with its **Check digit** appended and every `-` replaced by `=`, which is the form that appears in an **ARK**.
_Avoid_: ARK UUID, encoded ID

**Check digit**:
One Base64url character appended to a **Resource ID** or **Value ID** that detects a mistyped **ARK**.
_Avoid_: checksum

**Value ID**:
The Base64url-encoded UUID of a value within a resource, carried in an **ARK** as a second **Escaped resource ID**-style segment.
_Avoid_: value UUID

**Version**:
A point in a resource's or value's history, named in an **ARK** by a UTC timestamp suffix (`.20180604T085622Z`).
_Avoid_: citation date, citdate (the SALSAH parameter), timestamp (the carrier, not the concept)

**Resource IRI**:
The RDF identifier of a DSP resource, `http://rdfh.ch/<Project shortcode>/<Resource ID>`.
_Avoid_: resource URI, RDF ID

**Legacy SALSAH ARK**:
An **ARK** at **ARK scheme version** `0`, minted by the PHP-based SALSAH system before migration to DSP, which names a resource by its SALSAH integer ID.
_Avoid_: v0 ARK, PHP ARK, old ARK

### Resolution

**Top-level object**:
The DaSCH home page, which the bare ARK `ark:/<NAAN>/1` resolves to.
_Avoid_: root ARK, NAA object

**Redirect target**:
The URL an **ARK** resolves to: a project landing page, a resource page, or a value within a resource page.
_Avoid_: landing URL, destination

**Registry**:
The INI file that maps each **Project shortcode** to its hosts and redirect templates; its source of truth is the `ark-resolver-data` repository.
_Avoid_: config, settings (the registry is one input to them), project list

**Redirect template**:
A registry string with `$`-placeholders from which a **Redirect target** or **Resource IRI** is built, e.g. `DSPResourceRedirectUrl`.
_Avoid_: URL pattern

**Project host**:
The host of project landing pages, today the Discovery and Presentation Environment (`repository.dasch.swiss`), set as `ProjectHost` in the **Registry**.
_Avoid_: host (collides with **Resource host**)

**Resource host**:
The per-project host of resource and value pages, today dsp-app (`app.dasch.swiss`), set as `Host` in a project's **Registry** section.
_Avoid_: host, app host, server

**Conversion**:
The translation between an **ARK** and a **Resource IRI** (and its **Version**), served at `/convert`.
_Avoid_: mapping, lookup

### Migration

**Served implementation**:
The implementation whose result the user receives, which is the Python one until Phase 2 of the migration.
_Avoid_: primary, reference implementation

**Shadow implementation**:
The Rust implementation, run on every request beside the **Served implementation** and never allowed to affect a response.
_Avoid_: new implementation, Rust path

**Shadow execution**:
The run of both implementations on the same input with the results compared and reported to Sentry and tracing.
_Avoid_: parallel execution (the code's name for it; it describes timing, not purpose), dual run

**Shadow mismatch**:
A **Shadow execution** whose two results differ, or where exactly one implementation failed.
_Avoid_: discrepancy, divergence

## Relationships

- An **ARK** has exactly one **ARK scheme version**, which decides how its remaining segments are read
- An **ARK** at scheme version `1` names the **Top-level object**, or one project by **Project shortcode**, optionally one resource by **Escaped resource ID**, optionally one value by **Value ID**, and optionally one **Version**
- An **Escaped resource ID** decodes to exactly one **Resource ID**, and only if its **Check digit** validates
- A **Project shortcode** has exactly one **Registry** section, which provides its **Resource host** and may override **Redirect templates**
- A project **ARK** resolves to a **Redirect target** on the **Project host**; resource and value **ARKs** resolve to a **Redirect target** on the project's **Resource host**
- A **Legacy SALSAH ARK** is accepted only for projects whose **Registry** section sets `AllowVersion0`, and its **Resource ID** is derived as a UUIDv5 from the SALSAH integer ID
- Every resolution and **Conversion** is one **Shadow execution**: the **Served implementation** answers, the **Shadow implementation** is compared, and a difference is a **Shadow mismatch**

## Example dialogue

> **Dev:** "A user reports that `ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh` lands on the wrong page. Where do I look?"
> **Domain expert:** "It's a resource **ARK** at **ARK scheme version** 1. The `=` is an escaped `-`, and the final `h` is the **Check digit**, so the **Resource ID** is `lklK7rVuVOmpBZYWrF8o-g`. The **Redirect target** is built from the `DSPResourceRedirectUrl` **Redirect template** on project 0803's **Resource host**."
> **Dev:** "And if I append `.20180604T085622Z`?"
> **Domain expert:** "That's a **Version**, so the resource-version template applies. Don't call it the ARK version. That term means the scheme number after the NAAN."
> **Dev:** "Sentry shows a **Shadow mismatch** for that ARK. Did the user see the wrong answer?"
> **Domain expert:** "No. The user always gets the **Served implementation**'s answer. A mismatch means the **Shadow implementation** disagrees, and that's a Rust bug to fix before Phase 2."

## Flagged ambiguities

- **"version"** was used for both the ARK's scheme number (`url_version`, `dsp_ark_version`, "version 0 ARK URLs") and a point in a resource's history (`?version=`, `.timestamp`). Resolved: **ARK scheme version** for the first, **Version** for the second.
- **"host"** was used for both the project landing-page host (`ProjectHost`, `$project_host`) and the per-project resource host (`Host`, `$host`), which are different applications in production. Resolved: **Project host** and **Resource host**.
- **"project ID"** (`project_id`, `$project_id`) names the **Project shortcode**, not the project's IRI (`DSPProjectIri`). Resolved: say **Project shortcode**; the code keeps `project_id`.
- **"UUID"** was used for the plain Base64url UUID, the same with a check digit, and its escaped ARK form. Resolved: **Resource ID** and **Escaped resource ID**.
- **"parallel execution"** (the module and class name) describes timing, but the concept is a served result plus a compared shadow. Resolved: **Shadow execution**; the code keeps `ParallelExecutor`.
- **"settings"**, **"config"** and **"registry"** were used interchangeably. Resolved: **Registry** is the INI of projects and templates; settings are the registry combined with environment configuration and are not a domain term.
