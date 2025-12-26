# Index Module - Phase 3 Implementation Plan

This document provides a detailed structured plan for implementing the Index Module (`feature = "index"`) in arch-toolkit. This is Phase 3 of the extraction plan from Pacsea.

---

## Executive Summary

| Aspect | Details |
|--------|---------|
| **Module** | `index` (feature = "index") |
| **Source** | `Pacsea/src/index/` (10 files, ~3,000 lines) |
| **Estimated Effort** | 20-30 hours |
| **Complexity** | Medium (system command execution, data persistence, async operations) |
| **Dependencies** | `types` module, optional `aur` module for enrichment |
| **Status** | ⏳ Planned - Not yet started |

## Overview

The Index Module provides functionality for querying and managing package database information:

1. **Installed Package Queries** - Query installed packages using `pacman -Q*` commands
2. **Official Repository Queries** - Search and query official Arch Linux repositories
3. **Mirror Management** - Fetch and manage Arch Linux mirror lists (Windows-specific)
4. **Index Persistence** - Load and save package index data to disk

This module is foundational and used by other modules (deps, install) for package information.

---

## Source Analysis

### Files to Extract

| File | Lines | Functionality | Dependencies |
|------|-------|---------------|--------------|
| `installed.rs` | ~175 | Installed package cache, `is_installed()`, `refresh_installed_cache()` | `crate::util::pacman`, `crate::state::PackageItem` |
| `query.rs` | ~322 | Official repo search (`search_official()`, `all_official()`), fuzzy matching | `crate::state::{PackageItem, Source}`, `fuzzy-matcher` |
| `fetch.rs` | ~200 | Fetch official index from Arch API | `crate::util::curl`, `crate::sources::rate_limit_archlinux` |
| `persist.rs` | ~150 | Load/save index to disk (JSON) | `serde`, file I/O |
| `mirrors.rs` | ~1082 | Mirror list fetching (Windows-only) | `crate::util::curl`, `crate::sources::*` |
| `enrich.rs` | ~100 | Package enrichment (optional) | `crate::state::PackageItem` |
| `explicit.rs` | ~80 | Explicit package tracking | `crate::util::pacman` |
| `update.rs` | ~150 | Background index updates | `tokio`, async operations |
| `distro.rs` | ~50 | Distribution detection | System queries |
| `mod.rs` | ~175 | Module structure, shared state | All above |

### Key Dependencies to Remove

1. **Pacsea State Types** - Replace `PackageItem`, `Source` with arch-toolkit types
2. **Curl Wrapper** - Replace `crate::util::curl` with `reqwest` (already in arch-toolkit)
3. **Rate Limiting** - Use arch-toolkit's shared rate limiting
4. **Global State** - Replace `OnceLock`/`RwLock` with caller-provided state or return values

---

## Implementation Tasks

### Task 3.1: Define Standalone Types

**Goal**: Create framework-agnostic types for package index operations.

#### Task 3.1.1: Create Index Types

**File**: `src/types/index.rs`

**Types to Create**:
- `OfficialPackage` - Represents an official repository package
  ```rust
  pub struct OfficialPackage {
      pub name: String,
      pub version: String,
      pub description: String,
      pub repo: String,
      pub arch: String,
  }
  ```
- `OfficialIndex` - Collection of official packages with name lookup
  ```rust
  pub struct OfficialIndex {
      pub packages: Vec<OfficialPackage>,
      // Internal: name_to_idx for O(1) lookups
  }
  ```
- `IndexQueryResult` - Search result with optional fuzzy score
  ```rust
  pub struct IndexQueryResult {
      pub package: OfficialPackage,
      pub fuzzy_score: Option<i64>,
  }
  ```

**Estimated Effort**: 2-3 hours

**Acceptance Criteria**:
- [ ] All types have rustdoc comments (What/Inputs/Output/Details)
- [ ] Types are serializable with serde
- [ ] `OfficialIndex` has `rebuild_name_index()` method
- [ ] Unit tests for type operations
- [ ] Code passes `cargo fmt`, `cargo clippy`, `cargo check`

---

### Task 3.2: Port Installed Package Queries

**Goal**: Extract installed package query functionality.

#### Task 3.2.1: Port Installed Package Cache

**File**: `src/index/installed.rs`

**Functions to Port**:
- `refresh_installed_cache()` - Refresh installed package cache using `pacman -Qq`
- `is_installed(name: &str) -> bool` - Check if package is installed
- `get_installed_packages() -> HashSet<String>` - Get all installed package names

**Changes Required**:
- Remove dependency on `crate::util::pacman` - use direct `Command` execution
- Remove global state (`OnceLock`/`RwLock`) - return values or accept state parameter
- Add graceful degradation when pacman is unavailable
- Use `tokio::task::spawn_blocking` for async operations

**Estimated Effort**: 3-4 hours

**Acceptance Criteria**:
- [ ] Functions work without global state
- [ ] Graceful degradation when pacman unavailable
- [ ] Async operations use `spawn_blocking` correctly
- [ ] Unit tests for installed package queries
- [ ] Integration tests with mock pacman
- [ ] Code passes quality checks

#### Task 3.2.2: Port Explicit Package Tracking

**File**: `src/index/explicit.rs`

**Functions to Port**:
- `refresh_explicit_cache()` - Refresh explicitly installed packages (`pacman -Qe`)
- `is_explicit(name: &str) -> bool` - Check if package is explicitly installed

**Estimated Effort**: 1-2 hours

**Acceptance Criteria**:
- [ ] Functions work without global state
- [ ] Graceful degradation when pacman unavailable
- [ ] Unit tests
- [ ] Code passes quality checks

---

### Task 3.3: Port Official Repository Queries

**Goal**: Extract official repository search and query functionality.

#### Task 3.3.1: Port Official Index Search

**File**: `src/index/query.rs`

**Functions to Port**:
- `search_official(query: &str, fuzzy: bool) -> Vec<IndexQueryResult>` - Search official packages
- `all_official(index: &OfficialIndex) -> Vec<OfficialPackage>` - Get all official packages
- `find_package_by_name(index: &OfficialIndex, name: &str) -> Option<OfficialPackage>` - Find package by name

**Changes Required**:
- Replace `PackageItem` with `OfficialPackage`
- Replace `Source::Official` with direct `OfficialPackage` fields
- Remove dependency on global `idx()` - accept `&OfficialIndex` parameter
- Make fuzzy matching optional (feature flag or parameter)
- Remove dependency on `crate::util::fuzzy_match_rank_with_matcher` - use `fuzzy-matcher` directly

**Estimated Effort**: 4-5 hours

**Acceptance Criteria**:
- [ ] Functions accept `&OfficialIndex` parameter (no global state)
- [ ] Fuzzy matching works correctly
- [ ] Case-insensitive substring matching works
- [ ] Unit tests for search functionality
- [ ] Code passes quality checks

#### Task 3.3.2: Port Official Index Fetching

**File**: `src/index/fetch.rs`

**Functions to Port**:
- `fetch_official_index() -> Result<OfficialIndex>` - Fetch official index from Arch API
- `fetch_official_index_async() -> Result<OfficialIndex>` - Async version

**Changes Required**:
- Replace `crate::util::curl` with `reqwest` (use arch-toolkit's `ArchClient`)
- Replace `crate::sources::rate_limit_archlinux` with arch-toolkit rate limiting
- Parse Arch Packages API JSON response
- Handle rate limiting and retries via arch-toolkit infrastructure

**Estimated Effort**: 3-4 hours

**Acceptance Criteria**:
- [ ] Uses `reqwest` via arch-toolkit client
- [ ] Respects rate limiting
- [ ] Handles errors gracefully
- [ ] Unit tests with mock HTTP responses
- [ ] Integration tests (optional, requires network)
- [ ] Code passes quality checks

---

### Task 3.4: Port Index Persistence

**Goal**: Extract index persistence functionality.

#### Task 3.4.1: Port Index Load/Save

**File**: `src/index/persist.rs`

**Functions to Port**:
- `load_from_disk(path: &Path) -> Result<OfficialIndex>` - Load index from JSON file
- `save_to_disk(index: &OfficialIndex, path: &Path) -> Result<()>` - Save index to JSON file

**Changes Required**:
- Use `OfficialIndex` type instead of Pacsea types
- Handle serde serialization/deserialization
- Rebuild name index after deserialization

**Estimated Effort**: 2-3 hours

**Acceptance Criteria**:
- [ ] Load/save works correctly
- [ ] Name index rebuilt after load
- [ ] Handles file errors gracefully
- [ ] Unit tests for persistence
- [ ] Code passes quality checks

---

### Task 3.5: Port Mirror Management (Optional)

**Goal**: Extract mirror list fetching (Windows-specific, optional).

#### Task 3.5.1: Port Mirror Fetching

**File**: `src/index/mirrors.rs`

**Functions to Port**:
- `fetch_mirrors() -> Result<Vec<MirrorInfo>>` - Fetch mirror list from Arch API
- `generate_mirrorlist(mirrors: &[MirrorInfo]) -> String` - Generate mirrorlist format

**Changes Required**:
- Replace `crate::util::curl` with `reqwest`
- Replace rate limiting with arch-toolkit infrastructure
- Make Windows-specific code optional (feature flag or cfg)
- Define `MirrorInfo` type

**Estimated Effort**: 2-3 hours

**Acceptance Criteria**:
- [ ] Mirror fetching works
- [ ] Windows-specific code is optional
- [ ] Unit tests
- [ ] Code passes quality checks

**Note**: This may be low priority if Windows support is not needed initially.

---

### Task 3.6: Port Background Updates (Optional)

**Goal**: Extract background index update functionality.

#### Task 3.6.1: Port Background Update

**File**: `src/index/update.rs`

**Functions to Port**:
- `update_in_background(path: &Path) -> tokio::task::JoinHandle<Result<()>>` - Spawn background update task

**Changes Required**:
- Remove dependency on Pacsea notification channels
- Use callback or return future instead
- Make optional (feature flag)

**Estimated Effort**: 2-3 hours

**Acceptance Criteria**:
- [ ] Background updates work
- [ ] No dependency on Pacsea channels
- [ ] Unit tests
- [ ] Code passes quality checks

**Note**: This may be low priority if background updates are not needed initially.

---

### Task 3.7: Module Entry Point

**Goal**: Create module entry point with comprehensive documentation.

#### Task 3.7.1: Create Module Entry Point

**File**: `src/index/mod.rs`

**Tasks**:
- Export all public types and functions
- Add module-level documentation with usage examples
- Document feature flag requirements
- Add convenience re-exports

**Estimated Effort**: 1-2 hours

**Acceptance Criteria**:
- [ ] All public APIs exported
- [ ] Comprehensive module documentation
- [ ] Usage examples in rustdoc
- [ ] Feature flags documented
- [ ] Code passes quality checks

---

### Task 3.8: Testing and Documentation

**Goal**: Ensure comprehensive testing and documentation.

#### Task 3.8.1: Unit Tests

**Tasks**:
- Port existing unit tests from Pacsea
- Add tests for new standalone API
- Test error cases and edge conditions
- Test graceful degradation

**Estimated Effort**: 3-4 hours

**Acceptance Criteria**:
- [ ] All functions have unit tests
- [ ] Error cases covered
- [ ] Edge cases covered
- [ ] Tests pass with `cargo test -- --test-threads=1`

#### Task 3.8.2: Integration Tests

**File**: `tests/index_integration.rs`

**Tasks**:
- Create integration tests for index operations
- Test with mock pacman commands
- Test persistence operations
- Test search functionality

**Estimated Effort**: 2-3 hours

**Acceptance Criteria**:
- [ ] Integration tests cover main workflows
- [ ] Tests use mock commands where possible
- [ ] Tests pass with `cargo test -- --test-threads=1`

#### Task 3.8.3: Documentation and Examples

**Tasks**:
- Add rustdoc examples to all public APIs
- Create example program `examples/index_example.rs`
- Update README with index module documentation
- Document feature flag requirements

**Estimated Effort**: 2-3 hours

**Acceptance Criteria**:
- [ ] All public APIs have rustdoc examples
- [ ] Example program demonstrates usage
- [ ] README updated
- [ ] Feature flags documented

---

## Feature Flag Configuration

### Cargo.toml Updates

```toml
[features]
default = ["aur"]
index = []  # No additional dependencies, uses std and serde

[dependencies]
# Index module dependencies (always included)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Optional dependencies
fuzzy-matcher = { version = "0.3", optional = true }  # For fuzzy search
tokio = { version = "1", features = ["rt", "time"], optional = true }  # For async operations
```

**Note**: The index module should work without async by default, but async operations are available when `tokio` feature is enabled.

---

## API Design

### Example Usage

```rust
use arch_toolkit::index::{OfficialIndex, search_official, refresh_installed_cache};
use arch_toolkit::types::OfficialPackage;

// Load or fetch official index
let index = OfficialIndex::load_from_disk("index.json")
    .or_else(|_| fetch_official_index())?;

// Search official packages
let results = search_official(&index, "ripgrep", true);  // fuzzy = true
for result in results {
    println!("{}: {}", result.package.name, result.package.version);
}

// Check installed packages
refresh_installed_cache().await?;
let installed = get_installed_packages()?;
if installed.contains("ripgrep") {
    println!("ripgrep is installed");
}
```

---

## Migration Strategy

### For Pacsea

Once the index module is complete, Pacsea can:

1. Add `arch-toolkit` dependency with `features = ["index"]`
2. Gradually replace `src/index/` calls with `arch_toolkit::index::*`
3. Remove duplicated index code from Pacsea
4. Focus development on TUI-specific features

### Backward Compatibility

- The module should provide similar functionality to Pacsea's index module
- API may differ slightly (no global state, explicit parameters)
- Migration guide should be provided

---

## Risk Assessment

### High Risk Areas

1. **Global State Removal** - Removing `OnceLock`/`RwLock` may require API changes
   - **Mitigation**: Design API to accept state explicitly or return owned values

2. **Pacman Command Execution** - System command execution can fail
   - **Mitigation**: Graceful degradation, clear error messages

3. **Async Operations** - Mixing sync and async operations
   - **Mitigation**: Make async optional, provide both sync and async APIs

### Medium Risk Areas

1. **Fuzzy Matching** - Dependency on `fuzzy-matcher` crate
   - **Mitigation**: Make optional feature, provide fallback to substring matching

2. **Windows Support** - Mirror management is Windows-specific
   - **Mitigation**: Make optional, use cfg attributes

---

## Success Criteria

The Index Module is complete when:

- [ ] All core functionality ported (installed queries, official queries, persistence)
- [ ] No dependencies on Pacsea internals
- [ ] All functions have rustdoc documentation
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Example program works
- [ ] Code passes `cargo fmt`, `cargo clippy`, `cargo check`
- [ ] README updated with index module documentation
- [ ] Module entry point complete

---

## Timeline

| Task | Estimated Hours | Priority |
|------|----------------|----------|
| 3.1: Define Types | 2-3 | High |
| 3.2: Installed Queries | 4-6 | High |
| 3.3: Official Queries | 7-9 | High |
| 3.4: Persistence | 2-3 | High |
| 3.5: Mirror Management | 2-3 | Medium (Optional) |
| 3.6: Background Updates | 2-3 | Low (Optional) |
| 3.7: Module Entry Point | 1-2 | High |
| 3.8: Testing & Docs | 7-10 | High |
| **Total** | **27-39 hours** | |

**Recommended Approach**: Start with high-priority tasks (3.1-3.4, 3.7-3.8), then add optional features (3.5-3.6) as needed.

---

## Next Steps

1. Review and approve this plan
2. Start with Task 3.1 (Define Types)
3. Proceed incrementally through tasks
4. Update this document as tasks are completed
5. Publish v0.2.0 when complete

---

## References

- [AUR_TOOLKIT_CRATE_PREPARATION.md](./AUR_TOOLKIT_CRATE_PREPARATION.md) - Overall extraction plan
- [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md) - Phase 2 implementation plan (reference)
- Pacsea source: `src/index/` - Source code to extract from

