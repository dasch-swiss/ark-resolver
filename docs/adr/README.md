# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records for the DSP ARK Resolver project.

## What are ADRs?

Architecture Decision Records (ADRs) are documents that capture important architectural decisions made during the development of this project, along with their context and consequences.

## Format

Each ADR follows this structure:
- **Title**: Brief description of the decision
- **Status**: Current status (Proposed, Accepted, Deprecated, Superseded)
- **Context**: Background and problem being solved
- **Decision**: What we decided to do
- **Consequences**: Expected positive, negative, and neutral outcomes

## Naming Convention

ADRs are named using the pattern: `NNNN-title-with-dashes.md`

Where `NNNN` is a zero-padded sequential number (e.g., `0001`, `0002`).

## Index

| ADR | Status | Title |
|-----|--------|-------|
| [0001](0001-adopt-hexagonal-architecture.md) | Accepted | Adopt Hexagonal Architecture for Python-to-Rust Migration |

## Creating New ADRs

1. Copy `template.md` to create a new ADR
2. Use the next sequential number
3. Fill in all sections
4. Update this README index
5. Submit for review before marking as "Accepted"

## References

- [ADR GitHub Repository](https://adr.github.io/)
- [Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)