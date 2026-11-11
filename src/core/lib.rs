//! The hexagonal core: domain, errors, ports and use cases.
//!
//! I/O, environment and HTTP access go through `ports/` and are implemented in `src/adapters`.
//! The crates the core may use are the `deps` of `//src:core` in `src/BUILD.bazel`; adding one is a
//! reviewed change to the dependency rule in ADR-0001.

pub mod domain;
pub mod errors;
pub mod ports;
pub mod use_cases;
