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
| **Status** | ✅ Complete - All tasks 2.1.1 through 2.6.3 complete |

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

- **Task 2.1.4: Port PKGBUILD Parsing** - Complete
  - Created `src/deps/pkgbuild.rs` with parsing functions
  - Ported `parse_pkgbuild_deps()` and `parse_pkgbuild_conflicts()` from Pacsea
  - Handles single-line and multi-line bash array syntax
  - Handles append syntax (depends+=) in PKGBUILD functions
  - Filters .so virtual packages and invalid package names
  - Automatic deduplication of dependencies and conflicts
  - Comprehensive unit tests (21 tests, ported from Pacsea + additional edge cases)
  - Example file `examples/pkgbuild_example.rs` with 16 usage examples
  - Module exports updated in `src/deps/mod.rs`

- **Task 2.2.1: Port Version Comparison** - Complete
  - Created `src/deps/version.rs` with version comparison utilities
  - Implemented `compare_versions()` with pacman-compatible algorithm
  - Ported and improved `version_satisfies()` function (uses proper version comparison instead of string comparison)
  - Implemented `is_major_version_bump()` and `extract_major_component()` functions
  - Added `normalize_version()` helper for pkgrel suffix stripping
  - Handles numeric vs text segments correctly (numeric < text, versions without suffixes > versions with suffixes)
  - Comprehensive unit tests (18 tests covering all edge cases)
  - All functions exported in `src/deps/mod.rs`
  - Code quality checks pass (fmt, clippy, tests)

- **Task 2.3.1: Port Package Queries** - Complete
  - Created `src/deps/query.rs` with package querying functions
  - Implemented `get_installed_packages()` using `pacman -Qq`
  - Implemented `get_upgradable_packages()` using `pacman -Qu` with parsing logic
  - Implemented `get_provided_packages()` (returns empty set, lazy checking for performance)
  - Implemented `is_package_installed_or_provided()` with lazy provided checking using `pacman -Qqo`
  - Implemented `get_installed_version()` using `pacman -Q` with version normalization
  - Implemented `get_available_version()` using `pacman -Si` for repository queries
  - All functions gracefully degrade when pacman is unavailable (return empty sets/None)
  - Generic over `BuildHasher` for flexibility with different HashSet implementations
  - Comprehensive unit tests (10 tests: 6 parsing logic tests, 4 integration tests)
  - All functions exported in `src/deps/mod.rs`
  - Code quality checks pass (fmt, clippy, tests)

- **Task 2.3.2: Port Source Determination** - Complete
  - Created `src/deps/source.rs` with source determination functions
  - Implemented `determine_dependency_source()` for installed and uninstalled packages
  - Implemented `is_system_package()` for critical package detection
  - Handles official vs AUR vs local source detection
  - Generic over `BuildHasher` for flexibility
  - Comprehensive unit tests (8 tests)
  - All functions exported in `src/deps/mod.rs`
  - Code quality checks pass (fmt, clippy, tests)

- **Task 2.4.1: Create DependencyResolver** - Complete
  - Created `src/deps/resolve.rs` with dependency resolution functions (~1,300 lines)
  - Implemented `DependencyResolver` struct with `new()`, `with_config()`, and `resolve()` methods
  - Ported `determine_status()` for dependency status determination
  - Ported `batch_fetch_official_deps()` for efficient batched pacman queries
  - Ported `resolve_package_deps()` for single package resolution (official, local, AUR)
  - Ported `fetch_package_conflicts()` for conflict detection
  - Implemented helper functions: `should_filter_dependency()`, `process_dependency_spec()`, `merge_dependency()`, etc.
  - Handles conflict detection and processing
  - PKGBUILD cache lookup via optional callback in `ResolverConfig`
  - AUR integration (feature-gated, with limitations for async .SRCINFO fetching)
  - Added `DependencyResolution` and `ResolverConfig` types to `src/types/dependency.rs`
  - Comprehensive unit tests (7 tests)
  - All functions exported in `src/deps/mod.rs`
  - Code quality checks pass (fmt, clippy, tests)

- **Task 2.4.2: Port Reverse Dependency Analysis** - Complete
  - Created `src/deps/reverse.rs` with reverse dependency analysis functions (~850 lines)
  - Implemented `ReverseDependencyAnalyzer` struct with `new()` and `analyze()` methods
  - Ported BFS traversal logic from `resolve_reverse_dependencies()` using `pacman -Qi` queries
  - Ported `fetch_pkg_info()` for pacman -Qi queries with key-value parsing
  - Ported `parse_key_value_output()` helper for parsing pacman output with wrapped lines
  - Ported `split_ws_or_none()` helper for whitespace-separated field parsing
  - Implemented aggregation and summary logic with per-root relationship tracking
  - Added `has_installed_required_by()` helper for checking installed dependents
  - Added `get_installed_required_by()` helper for getting list of installed dependents
  - Internal types: `PkgInfo`, `AggregatedEntry`, `RootRelation`, `ReverseResolverState`
  - Handles direct vs transitive dependents (depth tracking)
  - Conflict status generation with detailed reason strings
  - Source determination (official, AUR, local) based on repository information
  - System/core package detection based on groups and repository
  - Added `ReverseDependencyReport` type to `src/types/dependency.rs`
  - Comprehensive unit tests (5 tests)
  - All functions exported in `src/deps/mod.rs`
  - Code quality checks pass (fmt, clippy, tests)

- **Task 2.5.1: Create Module Entry Point** - Complete
  - Enhanced `src/deps/mod.rs` with comprehensive module-level documentation
  - Added usage examples for all major functionality (parsing, version comparison, querying, resolution, reverse analysis)
  - Documented feature flag requirements (deps, optional aur)
  - Added links to example programs
  - Verified all public types and functions are properly exported
  - Updated `src/lib.rs` to reflect deps module is complete (not "planned")
  - Added deps module examples to crate-level documentation
  - Added all deps types to crate-level re-exports
  - Updated `src/prelude.rs` with deps exports for convenience
  - Added commonly used deps types and functions to prelude
  - Code quality checks pass (fmt, clippy)

### ✅ Completed
- **Task 2.5.2: Update Main Library** - Already complete as part of 2.5.1
- **Task 2.6.1: Unit Tests** - Comprehensive unit tests verified and complete
- **Task 2.6.2: Integration Tests** - Created `tests/deps_integration.rs` with comprehensive tests
- **Task 2.6.3: Documentation** - Added rustdoc examples, updated README, created deps_example.rs

### 📋 Planned
- Phase 2 complete - Module ready for use

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
- [x] Create `src/deps/pkgbuild.rs`
- [x] Port `parse_pkgbuild_deps()` from Pacsea's sandbox/parse.rs
- [x] Port `parse_pkgbuild_conflicts()` function
- [x] Handle bash array syntax (single-line and multi-line)
- [x] Filter .so virtual packages
- [x] Add unit tests with sample PKGBUILD content
- [x] Add example file `examples/pkgbuild_example.rs`
- [x] Update module exports in `src/deps/mod.rs`

### Phase 2.2: Version Utilities (Est: 4-6 hours)

#### Task 2.2.1: Port Version Comparison
- [x] Create `src/deps/version.rs`
- [x] Port `version_satisfies()` function
- [x] Implement proper version comparison (>=, <=, =, >, <)
- [x] Handle pkgrel suffix stripping
- [x] Add comprehensive unit tests for version edge cases
- [x] Implement pacman-compatible version comparison algorithm (no external crate needed)

### Phase 2.3: Package Querying (Est: 6-8 hours)

#### Task 2.3.1: Port Package Queries
- [x] Create `src/deps/query.rs`
- [x] Port `get_installed_packages()` (pacman -Qq)
- [x] Port `get_upgradable_packages()` (pacman -Qu)
- [x] Port `get_provided_packages()` (lazy checking)
- [x] Port `is_package_installed_or_provided()`
- [x] Port `get_installed_version()` (pacman -Q)
- [x] Port `get_available_version()` (pacman -Si)
- [x] Add graceful degradation when pacman is unavailable
- [x] Add unit tests (parsing logic) and integration tests (command execution)

#### Task 2.3.2: Port Source Determination
- [x] Create logic for `determine_dependency_source()`
- [x] Port `is_system_package()` for critical package detection
- [x] Handle official vs AUR vs local source detection
- [x] Add tests for source determination

### Phase 2.4: Dependency Resolution (Est: 10-12 hours)

#### Task 2.4.1: Create DependencyResolver
- [x] Create `src/deps/resolve.rs`
- [x] Implement `DependencyResolver` struct
- [x] Port `resolve_package_deps()` for single package
- [x] Port `batch_fetch_official_deps()` for batched queries
- [x] Port `fetch_package_conflicts()` function
- [x] Implement `resolve()` method for batch resolution
- [x] Handle status determination (`determine_status()`)
- [x] Implement conflict detection and processing
- [x] Make PKGBUILD cache lookup optional via callback
- [x] Add integration with AUR module (optional, feature-gated)

#### Task 2.4.2: Port Reverse Dependency Analysis
- [x] Create `src/deps/reverse.rs`
- [x] Implement `ReverseDependencyAnalyzer` struct
- [x] Port BFS traversal logic from `resolve_reverse_dependencies()`
- [x] Port `fetch_pkg_info()` for pacman -Qi queries
- [x] Port `parse_key_value_output()` helper
- [x] Implement aggregation and summary logic
- [x] Add `has_installed_required_by()` helper
- [x] Add `get_installed_required_by()` helper

### Phase 2.5: Module Integration (Est: 4-6 hours)

#### Task 2.5.1: Create Module Entry Point
- [x] Create `src/deps/mod.rs`
- [x] Re-export all public types and functions
- [x] Add crate-level documentation with examples
- [x] Add feature flag support in `Cargo.toml`

#### Task 2.5.2: Update Main Library
- [x] Add `deps` feature flag to `Cargo.toml`
- [x] Add conditional module compilation in `src/lib.rs`
- [x] Update `src/prelude.rs` with deps exports
- [x] Update crate documentation

### Phase 2.6: Testing and Documentation (Est: 6-8 hours)

#### Task 2.6.1: Unit Tests
- [x] Test all parsing functions with edge cases - ✅ Comprehensive unit tests already exist
- [x] Test version comparison edge cases - ✅ Comprehensive unit tests already exist
- [x] Test status determination logic - ✅ Unit tests exist in resolve.rs
- [x] Test conflict detection - ✅ Unit tests exist in resolve.rs

#### Task 2.6.2: Integration Tests
- [x] Create `tests/deps_integration.rs` - ✅ Created with comprehensive integration tests
- [x] Test dependency resolution with mock commands - ✅ Integration tests created
- [x] Test reverse dependency analysis - ✅ Integration tests created
- [x] Test AUR integration (if enabled) - ✅ Integration tests created with feature-gated AUR tests

#### Task 2.6.3: Documentation
- [x] Add rustdoc examples for all public APIs - ✅ Examples added to resolve.rs, reverse.rs, source.rs
- [x] Add module-level documentation - ✅ Already complete in mod.rs
- [x] Update README with deps module usage - ✅ Comprehensive deps section added
- [x] Add example program `examples/deps_example.rs` - ✅ Created comprehensive example

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
| Sandbox parse functions | Include in deps module | ✅ Resolved (Task 2.1.4) |
| Index module coupling | Accept parameters instead | ✅ Resolved (Task 2.3.1) |
| System command execution | Direct std::process::Command | ✅ Resolved (Task 2.3.1) |

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
- [x] Can parse PKGBUILD files and extract dependency arrays - ✅ Task 2.1.4
- [x] Can query installed packages from pacman database - ✅ Task 2.3.1
- [x] Can determine dependency source (official, AUR, local) - ✅ Task 2.3.2
- [x] Can resolve dependencies for a list of packages - ✅ Task 2.4.1
- [x] Can analyze reverse dependencies for removal operations - ✅ Task 2.4.2
- [x] Graceful degradation when pacman is unavailable - ✅ Task 2.3.1, 2.3.2
- [x] Works without i18n (English-only output) - ✅ Task 2.1.2

### Code Quality
- [x] All functions have rustdoc comments (What/Inputs/Output/Details) - ✅ Tasks 2.1.1, 2.1.2, 2.1.3, 2.1.4, 2.2.1, 2.3.1, 2.3.2, 2.4.1, 2.4.2
- [ ] Cyclomatic complexity < 25 for all functions
- [x] cargo fmt produces no changes - ✅ Tasks 2.1.1, 2.1.2, 2.1.3, 2.1.4, 2.2.1, 2.3.1, 2.3.2, 2.4.1, 2.4.2
- [x] cargo clippy produces no warnings - ✅ Tasks 2.1.1, 2.1.2, 2.1.3, 2.1.4, 2.2.1, 2.3.1, 2.3.2, 2.4.1, 2.4.2
- [x] All tests pass (cargo test -- --test-threads=1) - ✅ Tasks 2.1.1, 2.1.2, 2.1.3, 2.1.4, 2.2.1, 2.3.1, 2.3.2, 2.4.1, 2.4.2

### Testing
- [x] Unit tests for all parsing functions - ✅ Tasks 2.1.2, 2.1.3, 2.1.4
- [x] Unit tests for version comparison - ✅ Task 2.2.1
- [x] Unit tests for package querying (parsing logic) - ✅ Task 2.3.1
- [x] Integration tests for package querying (command execution) - ✅ Task 2.3.1
- [x] Unit tests for source determination - ✅ Task 2.3.2
- [x] Unit tests for dependency resolution - ✅ Task 2.4.1
- [x] Unit tests for reverse dependency analysis - ✅ Task 2.4.2
- [x] Example program demonstrating usage - ✅ Tasks 2.1.1, 2.1.3, 2.1.4

### Documentation
- [x] Module-level documentation with examples - ✅ Tasks 2.1.1, 2.1.2, 2.1.3, 2.1.4, 2.2.1, 2.3.1, 2.3.2, 2.4.1, 2.4.2
- [x] README updated with deps module usage - ✅ Task 2.6.3
- [x] Feature flags documented - ✅ Tasks 2.1.1, 2.1.3 (deps feature, conditional aur feature)
- [x] Rustdoc examples for all public APIs - ✅ Task 2.6.3
- [x] Comprehensive example program - ✅ Task 2.6.3 (deps_example.rs)

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
4. Replace `src/logic/sandbox/parse.rs` PKGBUILD functions with `arch_toolkit::deps::parse_pkgbuild_*`
5. Replace dependency resolution with `arch_toolkit::deps::DependencyResolver`
6. Replace reverse deps with `arch_toolkit::deps::ReverseDependencyAnalyzer`
7. Remove duplicated code

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

