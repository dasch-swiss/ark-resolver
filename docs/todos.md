# ARK Resolver Migration Plan - Phase 1

This document tracks the migration of Python code to Rust for the DSP ARK Resolver project.

## Phase 1 Overview

Phase 1 focuses on migrating core functions to Rust while maintaining Python compatibility. The convention is to create `_rust.py` files that run in parallel with Python implementations for production verification.

## Migration Status

### ✅ Completed
- [x] **Settings/Configuration System** - Fully migrated to Rust (`ArkUrlSettings`)
- [x] **Basic Rust Infrastructure** - PyO3 setup, build system, basic functions
- [x] **ARK URL Parsing** - Regex-based parsing moved to Rust
- [x] **Initial Test Coverage** - Parallel test files created
- [x] **Check Digit Migration** - Complete migration of `check_digit.py` to Rust with full test parity
- [x] **UUID Processing Functions** - Complete migration with Python parity testing

### ✅ Completed  
- [x] **Hexagonal Architecture Migration - Phase 2.1** - ✅ COMPLETED! Check digit module successfully migrated to hexagonal architecture
  - [x] **Pure Rust Testing Enabled** - `just test` now works without PyO3 dependencies
  - [x] **Clean Architecture Established** - Domain → Use Cases → Ports → Adapters pattern
  - [x] **API Compatibility Maintained** - All Python code continues to work unchanged

### 🚧 In Progress

### 📋 Planned (Phase 1)

#### High Priority
- [x] **Complete Check Digit Module Migration**
  - [x] Migrate `is_valid()` function to Rust
  - [x] Migrate `calculate_check_digit()` function to Rust  
  - [x] Migrate `calculate_modulus()` function to Rust
  - [x] Migrate helper functions (`weighted_value`, `to_int`, `to_check_digit`) to Rust
  - [x] Expose complete API to Python via PyO3
  - [x] Update `ark_url_rust.py` to use Rust check digit functions
  - [x] Add comprehensive Rust unit tests for check digit logic

- [x] **UUID Processing Migration** - ✅ COMPLETED
  - [x] Implement `add_check_digit_and_escape()` in Rust
  - [x] Implement `unescape_and_validate_uuid()` in Rust
  - [x] Remove TODO comments in `ark_url_rust.py:54` (completed - now uses Rust check digit functions)
  - [x] Update Python code to use Rust implementations
  - [x] Add comprehensive Rust unit tests with Python parity validation
  - [x] Fix error handling compatibility (PyValueError → ArkUrlException)
  - [x] Full test suite validation (27/27 tests passing)

#### Medium Priority  
- [x] **ARK URL Formatter Migration** - ✅ COMPLETED!
  - [x] Migrate `resource_iri_to_ark_id()` method to Rust
  - [x] Migrate `resource_iri_to_ark_url()` method to Rust
  - [x] Migrate `format_ark_url()` method to Rust
  - [x] Create `ArkUrlFormatter` Rust struct with PyO3 bindings
  - [x] Implement hexagonal architecture pattern
  - [x] Full test parity validation (27/27 tests passing)

- [ ] **ARK URL Info Processing Migration**
  - [ ] Migrate `ArkUrlInfo.__init__()` parsing logic to Rust
  - [ ] Migrate `to_redirect_url()` method to Rust
  - [ ] Migrate `to_resource_iri()` method to Rust
  - [ ] Migrate `to_dsp_redirect_url()` method to Rust

#### Testing & Validation
- [ ] **Expand Test Coverage**
  - [ ] Add comprehensive Rust unit tests for all migrated functions
  - [ ] Ensure test parity between Python and Rust implementations
  - [ ] Add integration tests for Python-Rust interop

- [ ] **Performance Validation**
  - [ ] Add benchmarking to measure Rust performance gains
  - [ ] Validate production parallel execution works correctly
  - [ ] Performance regression testing

#### Code Cleanup
- [ ] **Type Safety Improvements**
  - [ ] Address TODO in `ark_url.py:194` about ConfigParser types
  - [ ] Address TODO in `ark_url_rust.py:14` about Rust module typing
  - [ ] Improve error handling consistency

### 🎯 Success Criteria for Phase 1

- [x] Core UUID processing functions have Rust implementations
- [x] Python and Rust implementations run in parallel in production  
- [x] Comprehensive test coverage for both Python and Rust code
- [ ] Performance benchmarks show expected improvements
- [x] No regressions in functionality (all tests passing)
- [x] UUID processing TODO comments resolved

## Notes

- Each migration should maintain backward compatibility
- All changes must include corresponding test updates
- Performance should be measured before and after migration
- Code should follow existing Rust and Python conventions in the codebase

## Phase 2: Hexagonal Architecture Migration

### 🎯 Strategic Goal
Migrate to hexagonal architecture (Ports & Adapters pattern) as documented in [ADR-0001](adr/0001-adopt-hexagonal-architecture.md). This enables:
- Pure Rust unit testing (no Python runtime dependencies)
- Framework-independent business logic
- Easy addition of new interfaces (HTTP, CLI)
- Clear separation of concerns

### 🏗️ Target Architecture
```
Adapters (PyO3, HTTP, CLI) → Ports (Traits) → Use Cases → Domain (Pure Rust)
```

### 📋 Detailed Migration Plan

#### Phase 2.1: Check Digit Module (Start Here)
- [ ] **Domain Layer** (`src/core/domain/check_digit.rs`)
  - [ ] Pure mathematical check digit functions
  - [ ] Zero external dependencies
  - [ ] Comprehensive unit tests

- [ ] **Error Layer** (`src/core/errors/check_digit.rs`)
  - [ ] Simplified domain-specific errors
  - [ ] Clear indication of failure without implementation details

- [ ] **Use Case Layer** (`src/core/use_cases/check_digit_validator.rs`)
  - [ ] `CheckDigitValidator` struct with business logic orchestration
  - [ ] Grouped functionality approach

- [ ] **Port Layer** (`src/core/ports/check_digit.rs`)
  - [ ] `CheckDigitPort` trait defining abstract interface

- [ ] **Adapter Layer** (`src/adapters/pyo3/check_digit.rs`)
  - [ ] PyO3 wrappers maintaining exact API compatibility
  - [ ] Error conversion from domain to PyO3 errors

- [ ] **Integration** (`src/lib.rs`)
  - [ ] Update to use new architecture
  - [ ] Enable `cargo test --lib` for check digit module

#### Phase 2.2: UUID Processing Module
- [ ] **Domain Layer** (`src/core/domain/uuid_processing.rs`)
  - [ ] Pure UUID transformation logic
  - [ ] Dependencies on check digit domain functions

- [ ] **Use Case Layer** (`src/core/use_cases/ark_uuid_processor.rs`)
  - [ ] `ArkUuidProcessor` with orchestration logic
  - [ ] Integration with check digit use cases

- [ ] **Port and Adapter Layers**
  - [ ] Following same pattern as check digit module

#### Phase 2.3: Settings and Configuration
- [ ] **Domain Layer** - Configuration parsing and validation
- [ ] **Use Case Layer** - Settings management use cases
- [ ] **Port Layer** - Settings provider interfaces
- [ ] **Adapter Layer** - File system and environment variable adapters

#### Phase 2.4: ARK URL Processing
- [ ] **Domain Layer** - ARK URL parsing and formatting logic
- [ ] **Use Case Layer** - ARK resolution and conversion use cases
- [ ] **Integration** - Complete core business logic migration

#### Phase 2.5: Future Adapters
- [ ] **HTTP Adapter** - Axum-based REST API (future Phase 3)
- [ ] **CLI Adapter** - Command-line interface (future)
- [ ] **Service Migration** - Replace Python server with pure Rust

### ✅ Limitation Resolved
**Issue**: `just test` fails due to PyO3 runtime dependencies in Rust unit tests  
**Solution**: Hexagonal architecture with pure domain layer enables pure Rust testing  
**Status**: ✅ RESOLVED! `just test` now runs successfully with integration tests

### 📝 Implementation Context
- **Architecture Decision**: [ADR-0001](adr/0001-adopt-hexagonal-architecture.md)
- **Migration Strategy**: Incremental, one module at a time
- **API Compatibility**: Maintained during entire migration
- **Testing**: Pure Rust unit tests + existing Python integration tests

### 🎯 Success Criteria for Phase 2
- [x] ✅ `just test` runs successfully with comprehensive test coverage
- [x] ✅ Clear separation: Domain → Use Cases → Ports → Adapters
- [x] ✅ PyO3 adapter maintains exact API compatibility
- [ ] Framework-independent core business logic
- [ ] Foundation for future HTTP and CLI adapters

---

*Last updated: 2025-01-15*