# Dependencies Module - Phase 2 Implementation Plan

This document provides a detailed structured plan for implementing the Dependencies Module (`feature = "deps"`) in arch-toolkit. This is Phase 2 of the extraction plan from Pacsea.

---

## Executive Summary

| Aspect | Details |
|--------|---------|
| **Module** | `deps` (feature = "deps") |
| **Source** | `Pacsea/src/logic/deps/` (9 files, ~2,800 lines) |
| **Estimated Effort** | 30-40 hours |
| **Complexity** | High (system command execution, complex data structures) |
| **Dependencies** | `types` module, optional `aur` module for AUR package resolution |
| **Status** | ⏳ In Progress - Tasks 2.1.1, 2.1.2, 2.1.3 Complete ✅ |

## Current Progress

### ✅ Completed
- **Task 2.1.1: Define Standalone Types** - All core dependency types implemented
  - Created `src/types/dependency.rs` with 9 types (580 lines)
  - Added helper methods and Display implementations
  - Comprehensive unit tests (12 tests)
  - Example file demonstrating usage
  - Feature flag and exports configured

- **Task 2.1.2: Port Dependency Spec Parsing** - Complete
  - Created `src/deps/parse.rs` with parsing functions
  - Ported `parse_dep_spec()`, `parse_pacman_si_deps()`, `parse_pacman_si_conflicts()`
  - Removed i18n dependency, using English-only labels
  - Handles multi-line dependencies and deduplication
  - Comprehensive unit tests

- **Task 2.1.3: Port .SRCINFO Parsing** - Complete
  - Created `src/deps/srcinfo.rs` with parsing functions
  - Ported `parse_srcinfo_deps()`, `parse_srcinfo_conflicts()`, `parse_srcinfo()`
  - Implemented `fetch_srcinfo()` async function with reqwest (requires `aur` feature)
  - Handles architecture-specific dependencies, split packages, .so filtering
  - Comprehensive unit tests (11 tests)
  - Example file `examples/srcinfo_example.rs` demonstrating all features

### ⏳ In Progress
- **Task 2.1.4: Port PKGBUILD Parsing** - Next up

### 📋 Planned
- Phase 2.2-2.6 (Version utils, Query, Resolution, Integration, Testing)

---

## Current State Analysis

### Source Files in Pacsea

| File | Lines | Purpose | Extraction Complexity |
|------|-------|---------|----------------------|
| `deps.rs` (mod) | ~520 | Main entry point, resolve_dependencies | High |
| `parse.rs` | ~434 | Dependency spec parsing, pacman output parsing | Medium |
| `srcinfo.rs` | ~203 | .SRCINFO parsing | Low |
| `resolve.rs` | ~927 | Core dependency resolution logic | High |
| `reverse.rs` | ~819 | Reverse dependency analysis | Medium |
| `query.rs` | ~203 | Package querying (installed, upgradable) | Low |
| `status.rs` | ~260 | Status determination, version checking | Low |
| `source.rs` | ~180 | Dependency source determination | Low |
| `utils.rs` | ~60 | Utility functions | Low |
| `aur.rs` | ~1 | AUR-specific (currently empty placeholder) | N/A |

### Internal Dependencies (Blockers)

| Import | Files | Issue | Resolution |
|--------|-------|-------|------------|
| `crate::state::modal::DependencyInfo` | deps.rs, resolve.rs, reverse.rs | Core data type | Create standalone type |
| `crate::state::modal::DependencyStatus` | status.rs, utils.rs | Status enum | Create standalone type |
| `crate::state::modal::DependencySource` | source.rs, reverse.rs | Source enum | Create standalone type |
| `crate::state::modal::ReverseRootSummary` | reverse.rs | Summary type | Create standalone type |
| `crate::state::types::PackageItem` | deps.rs, reverse.rs | Package type | Create simplified type |
| `crate::state::types::Source` | resolve.rs | Source enum | Create standalone type |
| `crate::i18n::*` | parse.rs | Localized labels | Remove i18n, English-only |
| `crate::logic::files::get_pkgbuild_from_cache` | resolve.rs | Cache access | Make optional callback |
| `crate::logic::sandbox::parse_pkgbuild_deps` | resolve.rs | PKGBUILD parsing | Include in module |
| `crate::util::curl` | srcinfo.rs | HTTP client | Use reqwest |
| `crate::index::is_installed` | reverse.rs | Package index | Accept as parameter |

---

## Proposed API Design

### Module Structure

```
arch-toolkit/src/
├── deps/                           # feature = "deps"
│   ├── mod.rs                      # Public API re-exports
│   ├── types.rs                    # Standalone data types
│   ├── parse.rs                    # Dependency spec parsing
│   ├── srcinfo.rs                  # .SRCINFO parsing
│   ├── pkgbuild.rs                 # PKGBUILD parsing
│   ├── version.rs                  # Version comparison utilities
│   ├── query.rs                    # Package querying (installed, upgradable)
│   ├── resolve.rs                  # Dependency resolution
│   └── reverse.rs                  # Reverse dependency analysis
└── types/
    └── dependency.rs               # Shared dependency types
```

### Core Types

```rust
// arch-toolkit/src/types/dependency.rs

/// Status of a dependency relative to the current system state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencyStatus {
    /// Already installed and version matches requirement.
    Installed { version: String },
    /// Not installed, needs to be installed.
    ToInstall,
    /// Installed but outdated, needs upgrade.
    ToUpgrade { current: String, required: String },
    /// Conflicts with existing packages.
    Conflict { reason: String },
    /// Cannot be found in configured repositories or AUR.
    Missing,
}

/// Source of a dependency package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencySource {
    /// Official repository package.
    Official { repo: String },
    /// AUR package.
    Aur,
    /// Local package (not in repos).
    Local,
}

/// Information about a single dependency.
#[derive(Clone, Debug)]
pub struct Dependency {
    /// Package name.
    pub name: String,
    /// Required version constraint (e.g., ">=1.2.3").
    pub version_req: String,
    /// Current status of this dependency.
    pub status: DependencyStatus,
    /// Source repository or origin.
    pub source: DependencySource,
    /// Packages that require this dependency.
    pub required_by: Vec<String>,
    /// Packages that this dependency depends on.
    pub depends_on: Vec<String>,
    /// Whether this is a core repository package.
    pub is_core: bool,
    /// Whether this is a critical system package.
    pub is_system: bool,
}

/// Package information for dependency resolution.
#[derive(Clone, Debug)]
pub struct PackageRef {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Package source (official or AUR).
    pub source: PackageSource,
}

/// Package source enum for dependency resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageSource {
    /// Official repository.
    Official { repo: String, arch: String },
    /// AUR package.
    Aur,
}

/// Parsed dependency specification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencySpec {
    /// Package name.
    pub name: String,
    /// Version constraint (may be empty).
    pub version_req: String,
}

/// Parsed .SRCINFO dependencies.
#[derive(Clone, Debug, Default)]
pub struct SrcinfoData {
    /// Package base name.
    pub pkgbase: String,
    /// Package name (may differ from pkgbase for split packages).
    pub pkgname: String,
    /// Package version.
    pub pkgver: String,
    /// Package release number.
    pub pkgrel: String,
    /// Runtime dependencies.
    pub depends: Vec<String>,
    /// Build-time dependencies.
    pub makedepends: Vec<String>,
    /// Test dependencies.
    pub checkdepends: Vec<String>,
    /// Optional dependencies.
    pub optdepends: Vec<String>,
    /// Conflicting packages.
    pub conflicts: Vec<String>,
    /// Packages this package provides.
    pub provides: Vec<String>,
    /// Packages this package replaces.
    pub replaces: Vec<String>,
}

/// Dependency resolution result.
#[derive(Clone, Debug, Default)]
pub struct DependencyResolution {
    /// Resolved dependencies with status.
    pub dependencies: Vec<Dependency>,
    /// Packages that have conflicts.
    pub conflicts: Vec<String>,
    /// Packages that are missing.
    pub missing: Vec<String>,
}

/// Reverse dependency analysis result.
#[derive(Clone, Debug, Default)]
pub struct ReverseDependencyReport {
    /// Packages that depend on the target packages.
    pub dependents: Vec<Dependency>,
    /// Per-package summary statistics.
    pub summaries: Vec<ReverseDependencySummary>,
}

/// Summary statistics for a single package's reverse dependencies.
#[derive(Clone, Debug, Default)]
pub struct ReverseDependencySummary {
    /// Package name.
    pub package: String,
    /// Number of direct dependents (depth 1).
    pub direct_dependents: usize,
    /// Number of transitive dependents (depth ≥ 2).
    pub transitive_dependents: usize,
    /// Total number of dependents.
    pub total_dependents: usize,
}
```

### Public API

```rust
// arch-toolkit/src/deps/mod.rs

/// Parse a dependency specification string.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::parse_dep_spec;
///
/// let spec = parse_dep_spec("python>=3.12");
/// assert_eq!(spec.name, "python");
/// assert_eq!(spec.version_req, ">=3.12");
/// ```
pub fn parse_dep_spec(spec: &str) -> DependencySpec;

/// Parse .SRCINFO content into structured data.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::parse_srcinfo;
///
/// let srcinfo_content = "pkgbase = my-package\npkgname = my-package\ndepends = glibc";
/// let data = parse_srcinfo(srcinfo_content)?;
/// assert!(data.depends.contains(&"glibc".to_string()));
/// ```
pub fn parse_srcinfo(content: &str) -> Result<SrcinfoData>;

/// Parse PKGBUILD content for dependency arrays.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::parse_pkgbuild_deps;
///
/// let pkgbuild = "depends=('glibc' 'python>=3.10')";
/// let (deps, makedeps, checkdeps, optdeps) = parse_pkgbuild_deps(pkgbuild);
/// assert!(deps.contains(&"glibc".to_string()));
/// ```
pub fn parse_pkgbuild_deps(pkgbuild: &str) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>);

/// Check if a version satisfies a version requirement.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::version_satisfies;
///
/// assert!(version_satisfies("2.0", ">=1.5"));
/// assert!(!version_satisfies("1.0", ">=1.5"));
/// ```
pub fn version_satisfies(installed: &str, requirement: &str) -> bool;

/// Query installed packages from pacman database.
///
/// Returns a set of package names currently installed on the system.
pub fn get_installed_packages() -> HashSet<String>;

/// Query packages that have upgrades available.
///
/// Returns a set of package names that pacman reports as upgradable.
pub fn get_upgradable_packages() -> HashSet<String>;

/// Get the installed version of a package.
///
/// # Errors
///
/// Returns an error if the package is not installed or version cannot be parsed.
pub fn get_installed_version(name: &str) -> Result<String>;

/// Dependency resolver for batch package operations.
pub struct DependencyResolver {
    // Configuration and state
}

impl DependencyResolver {
    /// Create a new dependency resolver.
    pub fn new() -> Self;
    
    /// Create a resolver with custom configuration.
    pub fn with_config(config: ResolverConfig) -> Self;
    
    /// Resolve dependencies for a list of packages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::{DependencyResolver, PackageRef, PackageSource};
    ///
    /// let resolver = DependencyResolver::new();
    /// let packages = vec![
    ///     PackageRef {
    ///         name: "firefox".into(),
    ///         version: "121.0".into(),
    ///         source: PackageSource::Official { repo: "extra".into(), arch: "x86_64".into() },
    ///     },
    /// ];
    /// let result = resolver.resolve(&packages)?;
    /// println!("Found {} dependencies", result.dependencies.len());
    /// ```
    pub fn resolve(&self, packages: &[PackageRef]) -> Result<DependencyResolution>;
}

/// Reverse dependency analyzer for removal operations.
pub struct ReverseDependencyAnalyzer {
    // Configuration and state
}

impl ReverseDependencyAnalyzer {
    /// Create a new reverse dependency analyzer.
    pub fn new() -> Self;
    
    /// Analyze reverse dependencies for packages being removed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::{ReverseDependencyAnalyzer, PackageRef, PackageSource};
    ///
    /// let analyzer = ReverseDependencyAnalyzer::new();
    /// let packages = vec![
    ///     PackageRef {
    ///         name: "qt5-base".into(),
    ///         version: "5.15.10".into(),
    ///         source: PackageSource::Official { repo: "extra".into(), arch: "x86_64".into() },
    ///     },
    /// ];
    /// let report = analyzer.analyze(&packages)?;
    /// println!("{} packages would be affected", report.dependents.len());
    /// ```
    pub fn analyze(&self, packages: &[PackageRef]) -> Result<ReverseDependencyReport>;
}
```

### Configuration

```rust
/// Configuration for dependency resolution.
#[derive(Clone, Debug)]
pub struct ResolverConfig {
    /// Whether to include optional dependencies.
    pub include_optdepends: bool,
    /// Whether to include make dependencies.
    pub include_makedepends: bool,
    /// Whether to include check dependencies.
    pub include_checkdepends: bool,
    /// Maximum depth for transitive dependency resolution (0 = direct only).
    pub max_depth: usize,
    /// Custom callback for fetching PKGBUILD from cache (optional).
    pub pkgbuild_cache: Option<Box<dyn Fn(&str) -> Option<String> + Send + Sync>>,
    /// Whether to check AUR for missing dependencies.
    pub check_aur: bool,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            include_optdepends: false,
            include_makedepends: false,
            include_checkdepends: false,
            max_depth: 0, // Direct dependencies only
            pkgbuild_cache: None,
            check_aur: false,
        }
    }
}
```

---

## Implementation Tasks

### Phase 2.1: Types and Parsing (Est: 8-10 hours)

#### Task 2.1.1: Define Standalone Types
- [x] Create `src/types/dependency.rs` with all dependency-related types
- [x] Add `DependencyStatus`, `DependencySource`, `Dependency`, `PackageRef`
- [x] Add `DependencySpec`, `SrcinfoData`, `ReverseDependencySummary`
- [x] Add `PackageSource` enum for resolution input
- [x] Export types in `src/types/mod.rs` and `src/lib.rs`
- [x] Add Serde derives for serialization support
- [x] Add comprehensive rustdoc documentation
- [x] Add helper methods on `DependencyStatus` (is_installed, needs_action, is_conflict, priority)
- [x] Add `Display` trait implementations for all types
- [x] Add `DependencySpec` constructors (new, with_version, has_version_req)
- [x] Add comprehensive unit tests with serde roundtrips
- [x] Add `deps` feature flag to `Cargo.toml`
- [x] Create example file `examples/deps_types_example.rs`

**Note:** `ResolverConfig`, `DependencyResolution`, and `ReverseDependencyReport` will be added in later tasks when implementing the actual resolver and analyzer structs.

#### Task 2.1.2: Port Dependency Spec Parsing
- [x] Create `src/deps/parse.rs`
- [x] Port `parse_dep_spec()` from Pacsea
- [x] Remove i18n dependency - use hardcoded English labels
- [x] Port `parse_pacman_si_deps()` for parsing pacman -Si output
- [x] Port `parse_pacman_si_conflicts()` for parsing conflicts
- [x] Add unit tests for all parsing functions
- [x] Document edge cases (version operators, .so filtering)
- [x] Handle multi-line dependencies (continuation lines in pacman -Si output)
- [x] Deduplicate dependencies in `parse_pacman_si_deps()` and `parse_pacman_si_conflicts()` (return unique list)

#### Task 2.1.3: Port .SRCINFO Parsing
- [x] Create `src/deps/srcinfo.rs`
- [x] Port `parse_srcinfo_deps()` function
- [x] Port `parse_srcinfo_conflicts()` function
- [x] Add new `parse_srcinfo()` function returning `SrcinfoData`
- [x] Add async `fetch_srcinfo()` using reqwest (not curl)
- [x] Add unit tests with sample .SRCINFO content
- [x] Handle edge cases (split packages, architecture-specific deps)
- [x] Add example file `examples/srcinfo_example.rs` demonstrating all features

#### Task 2.1.4: Port PKGBUILD Parsing
- [ ] Create `src/deps/pkgbuild.rs`
- [ ] Port `parse_pkgbuild_deps()` from Pacsea's sandbox/parse.rs
- [ ] Port `parse_pkgbuild_conflicts()` function
- [ ] Handle bash array syntax (single-line and multi-line)
- [ ] Filter .so virtual packages
- [ ] Add unit tests with sample PKGBUILD content

### Phase 2.2: Version Utilities (Est: 4-6 hours)

#### Task 2.2.1: Port Version Comparison
- [ ] Create `src/deps/version.rs`
- [ ] Port `version_satisfies()` function
- [ ] Implement proper version comparison (>=, <=, =, >, <)
- [ ] Handle pkgrel suffix stripping
- [ ] Add comprehensive unit tests for version edge cases
- [ ] Consider using `alpm_version_cmp` logic or `version-compare` crate

### Phase 2.3: Package Querying (Est: 6-8 hours)

#### Task 2.3.1: Port Package Queries
- [ ] Create `src/deps/query.rs`
- [ ] Port `get_installed_packages()` (pacman -Qq)
- [ ] Port `get_upgradable_packages()` (pacman -Qu)
- [ ] Port `get_provided_packages()` (lazy checking)
- [ ] Port `is_package_installed_or_provided()`
- [ ] Port `get_installed_version()` (pacman -Q)
- [ ] Port `get_available_version()` (pacman -Si)
- [ ] Add graceful degradation when pacman is unavailable
- [ ] Add unit tests with mock commands

#### Task 2.3.2: Port Source Determination
- [ ] Create logic for `determine_dependency_source()`
- [ ] Port `is_system_package()` for critical package detection
- [ ] Handle official vs AUR vs local source detection
- [ ] Add tests for source determination

### Phase 2.4: Dependency Resolution (Est: 10-12 hours)

#### Task 2.4.1: Create DependencyResolver
- [ ] Create `src/deps/resolve.rs`
- [ ] Implement `DependencyResolver` struct
- [ ] Port `resolve_package_deps()` for single package
- [ ] Port `batch_fetch_official_deps()` for batched queries
- [ ] Port `fetch_package_conflicts()` function
- [ ] Implement `resolve()` method for batch resolution
- [ ] Handle status determination (`determine_status()`)
- [ ] Implement conflict detection and processing
- [ ] Make PKGBUILD cache lookup optional via callback
- [ ] Add integration with AUR module (optional, feature-gated)

#### Task 2.4.2: Port Reverse Dependency Analysis
- [ ] Create `src/deps/reverse.rs`
- [ ] Implement `ReverseDependencyAnalyzer` struct
- [ ] Port BFS traversal logic from `resolve_reverse_dependencies()`
- [ ] Port `fetch_pkg_info()` for pacman -Qi queries
- [ ] Port `parse_key_value_output()` helper
- [ ] Implement aggregation and summary logic
- [ ] Add `has_installed_required_by()` helper
- [ ] Add `get_installed_required_by()` helper

### Phase 2.5: Module Integration (Est: 4-6 hours)

#### Task 2.5.1: Create Module Entry Point
- [ ] Create `src/deps/mod.rs`
- [ ] Re-export all public types and functions
- [ ] Add crate-level documentation with examples
- [ ] Add feature flag support in `Cargo.toml`

#### Task 2.5.2: Update Main Library
- [ ] Add `deps` feature flag to `Cargo.toml`
- [ ] Add conditional module compilation in `src/lib.rs`
- [ ] Update `src/prelude.rs` with deps exports
- [ ] Update crate documentation

### Phase 2.6: Testing and Documentation (Est: 6-8 hours)

#### Task 2.6.1: Unit Tests
- [ ] Test all parsing functions with edge cases
- [ ] Test version comparison edge cases
- [ ] Test status determination logic
- [ ] Test conflict detection

#### Task 2.6.2: Integration Tests
- [ ] Create `tests/deps_integration.rs`
- [ ] Test dependency resolution with mock commands
- [ ] Test reverse dependency analysis
- [ ] Test AUR integration (if enabled)

#### Task 2.6.3: Documentation
- [ ] Add rustdoc examples for all public APIs
- [ ] Add module-level documentation
- [ ] Update README with deps module usage
- [ ] Add example program `examples/deps_example.rs`

---

## Feature Flags

```toml
# Cargo.toml additions
[features]
default = ["aur", "deps"]
deps = []                              # Dependency resolution
full = ["aur", "deps", "index", "install", "news", "sandbox"]

# Optional AUR integration for deps
deps-aur = ["deps", "aur"]             # Dependency resolution with AUR lookup
```

---

## Dependencies

### Required Dependencies

```toml
# Already in Cargo.toml
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
tracing = "0.1"

# New for deps module
# (none - uses system commands)
```

### Optional Dependencies

```toml
# For async .SRCINFO fetching (already available via aur feature)
reqwest = { version = "0.12", features = ["json"], optional = true }
tokio = { version = "1", features = ["rt", "time", "process"], optional = true }
```

---

## Blockers and Resolutions

### Resolved Blockers (from AUR module)

| Blocker | Resolution | Status |
|---------|------------|--------|
| HTTP client (curl) | Use reqwest | ✅ Done in AUR module |
| Unified error type | ArchToolkitError | ✅ Done |
| Rate limiting | Built into ArchClient | ✅ Done |

### New Blockers for Deps Module

| Blocker | Resolution | Status |
|---------|------------|--------|
| DependencyInfo type coupling | Create standalone type | ✅ Resolved (Task 2.1.1) |
| DependencyStatus type coupling | Create standalone type | ✅ Resolved (Task 2.1.1) |
| DependencySource type coupling | Create standalone type | ✅ Resolved (Task 2.1.1) |
| PackageItem type coupling | Create simplified PackageRef | ✅ Resolved (Task 2.1.1) |
| ReverseRootSummary type coupling | Create standalone type | ✅ Resolved (Task 2.1.1) |
| i18n dependency in parse.rs | Remove, use English-only labels | ✅ Resolved (Task 2.1.2) |
| PKGBUILD cache access | Accept optional callback | ⏳ Pending (Task 2.4.1) |
| Sandbox parse functions | Include in deps module | ⏳ Pending (Task 2.1.4) |
| Index module coupling | Accept parameters instead | ⏳ Pending (Task 2.3.1) |
| System command execution | Direct std::process::Command | ⏳ Pending (Task 2.3.1) |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| System command failures | Medium | Medium | Graceful degradation, clear errors |
| Pacman version differences | Low | Medium | Test on multiple Arch versions |
| Performance (many packages) | Medium | Low | Batch queries, caching |
| Edge cases in version comparison | Medium | Low | Comprehensive tests |
| Complex PKGBUILD parsing | Medium | Medium | Extensive test cases |

---

## Acceptance Criteria

### Functionality
- [x] Can parse dependency specifications with version constraints - ✅ Task 2.1.2
- [x] Can parse .SRCINFO files and extract all dependency types - ✅ Task 2.1.3
- [ ] Can parse PKGBUILD files and extract dependency arrays
- [ ] Can query installed packages from pacman database
- [ ] Can resolve dependencies for a list of packages
- [ ] Can analyze reverse dependencies for removal operations
- [ ] Graceful degradation when pacman is unavailable
- [x] Works without i18n (English-only output) - ✅ Task 2.1.2

### Code Quality
- [x] All functions have rustdoc comments (What/Inputs/Output/Details) - ✅ Tasks 2.1.1, 2.1.2, 2.1.3
- [ ] Cyclomatic complexity < 25 for all functions
- [x] cargo fmt produces no changes - ✅ Tasks 2.1.1, 2.1.2, 2.1.3
- [x] cargo clippy produces no warnings - ✅ Tasks 2.1.1, 2.1.2, 2.1.3
- [x] All tests pass (cargo test -- --test-threads=1) - ✅ Tasks 2.1.1, 2.1.2, 2.1.3

### Testing
- [x] Unit tests for all parsing functions - ✅ Tasks 2.1.2, 2.1.3
- [ ] Unit tests for version comparison
- [ ] Integration tests with mock commands
- [x] Example program demonstrating usage - ✅ Tasks 2.1.1, 2.1.3

### Documentation
- [x] Module-level documentation with examples - ✅ Tasks 2.1.1, 2.1.2, 2.1.3
- [ ] README updated with deps module usage
- [x] Feature flags documented - ✅ Tasks 2.1.1, 2.1.3 (deps feature, conditional aur feature)

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Test coverage | > 80% for parsing functions |
| API surface | < 20 public functions |
| Dependencies | 0 new required deps |
| Breaking changes | 0 to existing API |
| Documentation | 100% public items documented |

---

## Timeline

| Week | Tasks | Deliverable |
|------|-------|-------------|
| Week 1 | Phase 2.1 (Types, Parsing) | Core types and parsing functions |
| Week 2 | Phase 2.2-2.3 (Version, Query) | Version utils and package queries |
| Week 3 | Phase 2.4 (Resolution) | DependencyResolver, Reverse analysis |
| Week 4 | Phase 2.5-2.6 (Integration, Testing) | Complete module with tests |

**Total Estimated Time: 30-40 hours**

---

## Post-Implementation

### Migration Path for Pacsea

Once the deps module is complete, Pacsea can:

1. Add `arch-toolkit` with `features = ["aur", "deps"]`
2. Replace `src/logic/deps/parse.rs` with `arch_toolkit::deps::parse_*`
3. Replace `src/logic/deps/srcinfo.rs` with `arch_toolkit::deps::parse_srcinfo`
4. Replace dependency resolution with `arch_toolkit::deps::DependencyResolver`
5. Replace reverse deps with `arch_toolkit::deps::ReverseDependencyAnalyzer`
6. Remove duplicated code

### Future Enhancements

- [ ] **Dependency tree visualization** - Tree structure for deps
- [ ] **Cycle detection** - Detect circular dependencies
- [ ] **Version range merging** - Combine version requirements
- [ ] **AUR deep resolution** - Full AUR dependency tree
- [ ] **Provides/Conflicts resolution** - Full virtual package handling
- [ ] **Parallel resolution** - Async batch queries

---

## References

- [Pacsea src/logic/deps/](https://github.com/Firstp1ck/Pacsea/tree/main/src/logic/deps)
- [Arch Linux .SRCINFO](https://wiki.archlinux.org/title/.SRCINFO)
- [Arch Linux PKGBUILD](https://wiki.archlinux.org/title/PKGBUILD)
- [pacman man page](https://man.archlinux.org/man/pacman.8)
- [alpm-pkgbuild crate](https://crates.io/crates/alpm-pkgbuild) (for reference)

