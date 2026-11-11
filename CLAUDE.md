# CLAUDE.md

Orientation for coding agents in this repository. The rules live in the files listed under Start Here; this file routes to them.

## Notes
The coding agent MUST first present plan. Only after confirmation, start implementing plan and changing code.
The coding agent MUST ask questions about code coverage and if adding new unit tests or extending existing tests is required.

## Start Here

- [`ARCH-MAP.md`](ARCH-MAP.md): components, their paths and local-context kits, dependency direction, banned constructs. Read the entry for the component you change.
- [`CONTEXT.md`](CONTEXT.md): domain vocabulary (ARK parts, Served and Shadow implementation). Use its terms.
- [`CONVENTIONS.md`](CONVENTIONS.md): how to write code here, imported below.
- [`REVIEW.md`](REVIEW.md): what a review checks and skips.
- [`docs/adr/`](docs/adr/) for decisions, [`docs/learnings/`](docs/learnings/) for solved problems.
- [`.env.example`](.env.example): every environment variable with its default.

## Conventions

Coding conventions (business-rule comments, the served/shadow implementation rules, testing, commits) are in CONVENTIONS.md, imported here so every session loads them:

@CONVENTIONS.md

## Specs

ark-resolver specs (PRDs, implementation plans, design docs) live directly in [`docs/specs/`](docs/specs/) as flat, date-prefixed files. Platform-wide specs that span several repos stay in [`dasch-specs`](https://github.com/dasch-swiss/dasch-specs); a spec whose primary subject is ark-resolver belongs here even when it touches `ark-resolver-data`, deploy config or another repo.

- **File:** each spec is a flat file directly in `docs/specs/`, named `YYYY-MM-DD-NN-{topic}-{type}.md`. `NN` is a 2-digit daily sequence (`01`, `02`, …) so more than one spec can share a date. `{topic}` is lowercase alphanumeric + hyphens (≤60 chars). `{type}` is one of `PRD` / `plan` / `design` / `journal`. The artifacts of one piece of work share the same `YYYY-MM-DD-NN-{topic}-` stem (e.g. `…-plan.md` alongside its `…-journal.md`).
- **Assets:** any images go in a sibling `docs/specs/YYYY-MM-DD-NN-{topic}-assets/` directory, referenced relatively.
- **Frontmatter** (YAML, all spec files): `title`, `date`, `author`, `status: draft | reviewed | approved | implemented`, and `repositories:` listing the *other* code repos the feature modifies. Never list `ark-resolver` itself.
- **Reference direction is one-way. Specs are a sink.** A spec may reference out (code, docs, ADRs, `CONTEXT.md`), but nothing outside `docs/specs/` may reference into a spec, and a new spec should reference durable artifacts (an ADR, code, the glossary), not another spec. This lets any spec be renamed or deleted without breaking the tree. Flatten test: `grep -rnE "docs/specs/[0-9]{4}" --exclude-dir=specs --exclude-dir=.git .` returns nothing. Enforcement: `review`.

## Project Overview

The DSP ARK Resolver resolves ARK URLs that refer to resources in DSP (DaSCH Service Platform) repositories. As an HTTP server it redirects an ARK to the resource's location; as a command-line tool it converts between resource IRIs and ARK URLs. Redirect targets come from an INI registry named by `ARK_REGISTRY` (`tests/ark-registry.ini` locally).

It is a Python (Sanic) service with a Rust extension, `ark_resolver._rust`, built by Bazel's `pyo3_extension` rule (`//:_rust`, `src/lib.rs`) as described in `docs/adr/0002-build-with-bazel.md`. An earlier plan to rewrite the Rust side into a standalone Axum service and remove Python/PyO3 entirely is not being pursued. Logic continues to migrate from Python to Rust inside that hybrid; the migration phases and the rules for the served and shadow implementations are in CONVENTIONS.md, the module layout in ARCH-MAP.md.

## Development Commands

`nix develop` (or `direnv allow`) enters the dev shell with bazel, just, uv, cargo-audit and commitlint; `just install` installs the Python dependencies; `just setup`, once per clone, sets `blame.ignoreRevsFile` so `git blame` skips the reformat commits listed in `.git-blame-ignore-revs`. `just --list` shows every recipe with its description.

```bash
just check        # ruff format check, ruff, pyright; rustfmt and clippy with -Dwarnings
just test         # Rust unit tests (//src:core_unit_tests, //src:unit_tests)
just pytest       # Python tests against the Bazel-built extension (//tests/...)
just fmt          # format Python and Rust
just run          # serve locally with tests/ark-registry.ini
just build        # copy the Bazel-built _rust.so into ark_resolver/ for pyright and IDEs; `uv run` cannot load it
just smoke-test   # build the image, verify it, run the Bazel smoke test
just commit-lint  # check this branch's commit messages, as the commit-lint CI job does
```

The command-line modes need `ARK_REGISTRY` set to an absolute path:

```bash
bazel run //:ark_resolver_bin -- -i http://rdfh.ch/0002/70aWaB2kWsuiN6ujYgM0ZQ  # IRI to ARK
bazel run //:ark_resolver_bin -- -a http://ark.example.org/ark:/00000/0002-751e0b8a-6  # ARK redirect
```

Images go to `daschswiss/ark-resolver` (linux/amd64) from CI only (`just docker-publish`); `just docker-build` and `just image-check` build and verify one locally.
