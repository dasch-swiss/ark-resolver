# ADR-0001: Adopt Hexagonal Architecture for Python-to-Rust Migration

## Status

**Accepted** - 2025-01-15

## Context

The DSP ARK Resolver is currently in Phase 1 of a Python-to-Rust migration. The existing Rust code is tightly coupled to PyO3 throughout, which creates several problems:

- **Testing Limitation**: Cannot run pure Rust unit tests (`cargo test --lib` fails due to PyO3 runtime dependencies)
- **Development Velocity**: Slow feedback loop for business logic changes due to PyO3 integration requirements
- **Architectural Coupling**: Business logic mixed with PyO3 infrastructure code
- **Future Constraints**: Difficult to add new interfaces (planned HTTP service, CLI tools)

### Current Architecture Problems
- Rust business logic contains PyO3 dependencies throughout
- Core algorithms cannot be tested independently of Python runtime
- Framework concerns mixed with domain logic
- Limited extensibility for future interface requirements

### Future Requirements
- Pure Rust HTTP service (Axum) to replace Python server
- CLI interface for ARK resolution operations
- Performance-critical operations running without PyO3 overhead
- Maintainable architecture that supports independent evolution of components

## Decision

We will refactor the codebase using **Hexagonal Architecture** (Ports & Adapters pattern) with a clear separation of concerns:

### Architecture Layers

1. **Domain Layer** (`src/core/domain/`)
   - Pure business logic and mathematical operations
   - Zero external dependencies beyond Rust std library
   - Contains core algorithms and business rules

2. **Use Cases Layer** (`src/core/use_cases/`)
   - Application business logic orchestration
   - Coordinates domain operations
   - Grouped by business capability (e.g., `CheckDigitValidator`, `ArkUuidProcessor`)

3. **Ports Layer** (`src/core/ports/`)
   - Abstract interfaces (traits) for external world interaction
   - Defines contracts that adapters must implement

4. **Adapters Layer** (`src/adapters/`)
   - Concrete implementations for external frameworks
   - Currently: PyO3 adapter maintaining existing API
   - Future: HTTP (Axum), CLI adapters

### Error Handling Strategy
- **Simplified domain-specific errors**: Clear indication of what failed without overly granular implementation details
- **Error conversion at adapter boundaries**: Domain errors converted to appropriate adapter-specific errors (PyO3, HTTP status codes, etc.)

### Migration Strategy
- **Incremental migration**: One module at a time, starting with check digit (most isolated)
- **API compatibility**: Maintain existing PyO3 function signatures during migration
- **Parallel validation**: Existing Python test suite continues to validate functionality

## Consequences

### Positive
- **Pure Rust Testing**: Enables `cargo test --lib` with fast feedback loops
- **Framework Independence**: Core business logic not tied to any specific framework
- **Multiple Interfaces**: Easy to add HTTP, CLI, or other adapters in the future
- **Performance Potential**: Pure Rust core eliminates PyO3 overhead for critical operations
- **Clear Boundaries**: Well-defined separation between business logic and infrastructure
- **Migration Path**: Smooth transition strategy to eventual pure Rust service

### Negative
- **Initial Complexity**: More architectural overhead than simple refactoring
- **Learning Curve**: Team needs to understand hexagonal architecture principles
- **File Count**: More files and indirection initially (though with clear purpose)

### Neutral
- **API Stability**: Existing Python integration continues to work unchanged
- **Gradual Migration**: No big-bang changes, reduces deployment risk
- **Testing Strategy**: Maintains existing Python integration tests while adding pure Rust unit tests

## Implementation Notes

### Migration Order
1. **Phase 1**: Check digit module (most isolated, validates approach)
2. **Phase 2**: UUID processing module (depends on check digit)  
3. **Phase 3**: Settings and configuration management
4. **Phase 4**: ARK URL parsing and formatting
5. **Phase 5**: HTTP service adapter (future)

### Success Criteria
- `cargo test --lib` runs successfully with comprehensive coverage
- Existing Python integration tests continue to pass
- Clear separation between domain logic and PyO3 adapter code
- New features can be implemented in pure Rust with adapter wrappers

### File Structure
```
src/
├── core/
│   ├── domain/          # Pure business logic
│   ├── use_cases/       # Application orchestration
│   ├── ports/           # Abstract interfaces
│   └── errors/          # Domain-specific errors
├── adapters/
│   └── pyo3/            # PyO3 framework adapter
└── lib.rs               # PyO3 module definition
```

## References

- [Hexagonal Architecture (Ports & Adapters)](https://alistair.cockburn.us/hexagonal-architecture/)
- [Clean Architecture by Robert C. Martin](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Project Migration Plan](../todos.md)
- [Current Architecture Issues](https://github.com/dasch-swiss/ark-resolver/issues)