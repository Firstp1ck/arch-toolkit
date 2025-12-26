# Arch Toolkit Crate Preparation

This document analyzes framework-agnostic modules in Pacsea (`src/sources/`, `src/logic/`, `src/index/`, `src/install/`) for extraction into a unified `arch-toolkit` crate with feature flags. This single crate approach is recommended over multiple separate crates for better maintainability, shared types, and user experience.

## Current Status

**Phase 1 (MVP) - AUR Module: ✅ COMPLETED**

- **Version**: v0.1.2 (latest published 2025-12-22)
- **Status**: Published to crates.io and ready for use
- **Completed Features**:
  - ✅ Core infrastructure (types, error handling, HTTP client, builder pattern)
  - ✅ AUR search, info, comments, PKGBUILD fetching
  - ✅ Rate limiting with exponential backoff
  - ✅ Retry policy with configurable backoff
  - ✅ Optional caching layer (memory + disk)
  - ✅ Mock API for testing
  - ✅ Comprehensive documentation and tests
  - ✅ CI/CD workflows
  - ✅ Health check functionality
  - ✅ Environment variable configuration
  - ✅ Input validation and prelude module (v0.1.1)
  - ✅ Rich error context and trait-based design (v0.1.1)

**Phase 2 - Dependencies Module: ✅ COMPLETE**

- **Version**: v0.1.2 (published 2025-12-22)
- **Status**: Complete - all core functionality implemented, module entry point complete, testing and documentation complete (Tasks 2.1.1 through 2.6.3)
- **Completed Features**:
  - ✅ Dependency type system (`Dependency`, `DependencySpec`, `DependencyStatus`, etc.)
  - ✅ Dependency spec parsing (`parse_dep_spec`)
  - ✅ Pacman output parsing (`parse_pacman_si_deps`, `parse_pacman_si_conflicts`)
  - ✅ .SRCINFO parsing (`parse_srcinfo`, `parse_srcinfo_deps`, `parse_srcinfo_conflicts`)
  - ✅ .SRCINFO fetching from AUR (`fetch_srcinfo` - requires `aur` feature)
  - ✅ PKGBUILD parsing (`parse_pkgbuild_deps`, `parse_pkgbuild_conflicts`)
  - ✅ Version comparison utilities (`compare_versions`, `version_satisfies`, `is_major_version_bump`, `extract_major_component`)
  - ✅ Package querying (`get_installed_packages`, `get_upgradable_packages`, `get_installed_version`, `get_available_version`, `is_package_installed_or_provided`)
  - ✅ Source determination (`determine_dependency_source`, `is_system_package`)
  - ✅ Dependency resolution (`DependencyResolver`, `determine_status`, `batch_fetch_official_deps`, `fetch_package_conflicts`)
  - ✅ Reverse dependency analysis (`ReverseDependencyAnalyzer`, `has_installed_required_by`, `get_installed_required_by`)
  - ✅ Comprehensive examples and documentation
- **Completed Work**:
  - ✅ All core functionality implemented
  - ✅ Module entry point complete (Task 2.5.1)
  - ✅ Integration tests created (Task 2.6.2)
  - ✅ Documentation complete (Task 2.6.3)
  - ✅ Comprehensive examples created (Task 2.6.3)
  - ⏳ AUR dependency queries (async .SRCINFO fetching limitation noted - future enhancement)
  - **Detailed Plan**: [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md)

**Phase 3 - Index Module: 🚧 IN PROGRESS**

- Index module (package database queries)
  - ✅ Installed package queries (Task 3.2.1 - complete)
  - ✅ Explicit package tracking (Task 3.2.2 - complete)
  - ✅ Index types (Task 3.1.1 - complete)
  - ✅ Official repository queries (Task 3.3 - complete)
  - ⏳ Mirror management (Task 3.5 - optional, pending)
  - ⏳ Index persistence (Task 3.4 - pending)

**Phase 4+ - Remaining Modules: ⏳ PLANNED**

- Install module (command building)
- News module (RSS feeds, advisories)
- Sandbox module (PKGBUILD security analysis)

## Existing Crates on crates.io

Before proceeding, here's a comprehensive analysis of what already exists in the Rust ecosystem:

### AUR-Specific Crates

| Crate | Features | Status | Notes |
|-------|----------|--------|-------|
| **`aur`** | AUR RPC client (async/sync) | Active | Supports both `hyper` and `reqwest` |
| **`aur-client`** | AUR search, clone | Active | Basic AUR operations |
| **`aur-rpc`** | AUR RPC abstractions | Active | Search and info functions |
| **`aur_rs`** | AUR package info | Active | `search_package`, `package_info` |

### ALPM/Pacman Crates

| Crate | Features | Status | Notes |
|-------|----------|--------|-------|
| **`alpm-rs`** | libalpm bindings | Active | **Important** - Direct pacman backend access |
| **`alpm-types`** | ALPM type definitions | Active | Shared types for ALPM |
| **`alpm-pkgbuild`** | PKGBUILD parsing | Active | Extract metadata, convert to SRCINFO |
| **`alpm-parsers`** | ALPM spec parsers | Active | Custom INI parser, duplicate keys |
| **`alpm-package`** | Low-level package creation | Active | Create ALPM packages from directories |

### Full-Featured Package Managers (Applications)

| Crate/Project | Features | Status | Notes |
|----------------|----------|--------|-------|
| **`aura`** | Full AUR manager (Rust port) | Active | Complete package manager, not a library |
| **`archlink`** | CLI tool with fuzzy search | Active | Application, not reusable library |
| **`pacdef`** | Declarative package manager | Active | Multi-backend, group file management |
| **`arch`** | CLI utility for Arch systems | Active | System management tool |

### What's Missing

**No unified library crate exists** that combines:
- ✅ AUR operations (search, info, comments, PKGBUILD)
- 🚧 Dependency parsing (SRCINFO parsing ✅, tree building ⏳)
- ⏳ Package index queries (installed, official repos)
- ⏳ Installation command building
- ⏳ News feeds and advisories
- ⏳ PKGBUILD security analysis

**Gaps in existing crates:**
1. **Fragmented** - Need multiple crates for complete functionality
2. **Partial dependency support** - SRCINFO parsing available ✅, but missing dependency tree building ⏳
3. **No news/advisories** - No Arch news RSS or security advisory support
4. **No comments** - AUR comment scraping not available (except arch-toolkit ✅)
5. **No sandbox analysis** - PKGBUILD security analysis missing
6. **No unified error types** - Each crate has its own error handling (arch-toolkit provides unified errors ✅)
7. **No rate limiting** - Missing built-in rate limiting for archlinux.org (arch-toolkit provides this ✅)
8. **Applications vs Libraries** - Most are CLI tools, not reusable libraries

### Recommended Features to Include

Based on analysis of existing crates, here's what `arch-toolkit` should include:

#### ✅ **Must Include** (Core Functionality)

1. **AUR Operations** (from Pacsea `src/sources/`)
   - ✅ Search packages (`aur`, `aur-rpc` exist but basic)
   - ✅ Package info/details
   - ✅ **AUR comments scraping** (unique - not in other crates)
   - ✅ PKGBUILD fetching
   - ✅ Rate limiting (unique - not in other crates)

2. **Dependency Resolution** (from Pacsea `src/logic/deps/`)
   - ✅ SRCINFO parsing (`alpm-pkgbuild` exists but different focus) - **IMPLEMENTED in v0.1.2**
   - ✅ Dependency spec parsing - **IMPLEMENTED in v0.1.2**
   - ✅ PKGBUILD dependency parsing - **IMPLEMENTED in v0.1.2**
   - ⏳ Dependency tree building (unique) - **IN PROGRESS**
   - ⏳ Reverse dependency analysis (unique) - **PLANNED**
   - ✅ Version constraint parsing - **IMPLEMENTED in v0.1.2**

3. **Package Index** (from Pacsea `src/index/`)
   - ✅ Installed package queries
   - ✅ Official repo queries
   - ✅ Mirror management

4. **Installation Commands** (from Pacsea `src/install/`)
   - ✅ Pacman command building
   - ✅ AUR helper detection
   - ✅ Batch operations

5. **News & Advisories** (from Pacsea `src/sources/news/`)
   - ✅ Arch news RSS (unique)
   - ✅ Security advisories (unique)

6. **Sandbox Analysis** (from Pacsea `src/logic/sandbox/`)
   - ✅ PKGBUILD security analysis (unique)

#### 🤔 **Consider Including** (From Other Crates)

1. **Fuzzy Search** (from `archlink`)
   - ✅ Already in Pacsea (`fuzzy-matcher`)
   - **Recommendation**: Include in search functions

2. **PKGBUILD Parsing** (from `alpm-pkgbuild`)
   - ✅ Already in Pacsea (`src/logic/files/pkgbuild_parse.rs`)
   - **Recommendation**: Include but ensure compatibility with `alpm-pkgbuild` types

3. **libalpm Integration** (from `alpm-rs`)
   - ⚠️ Requires C library dependency
   - **Recommendation**: Make optional feature `feature = "alpm"` that wraps `alpm-rs`
   - Allows pure-Rust fallback for environments without libalpm

4. **Declarative Package Management** (from `pacdef`)
   - ⚠️ Different use case (multi-machine sync)
   - **Recommendation**: Not included - different scope

#### ❌ **Don't Include** (Out of Scope)

1. **Full package manager** - That's what `aura` does
2. **CLI interface** - Applications should use the library
3. **System snapshots** - Too specialized
4. **Multi-distro support** - Focus on Arch Linux only

### Competitive Advantage

The proposed `arch-toolkit` would offer:

1. **Unified API** - Single crate with feature flags
2. **Complete feature set** - All Arch Linux operations in one place
3. **Battle-tested code** - Extracted from Pacsea (production use)
4. **Rate limiting** - Built-in backoff and circuit breaker logic
5. **Comprehensive** - AUR, deps, index, install, news, sandbox
6. **Well-documented** - Rustdoc comments with What/Inputs/Output/Details format
7. **Async-first** - Modern async/await design
8. **Pure Rust** - No C dependencies by default (optional `alpm` feature)
9. **Unique features** - Comments, news, advisories, sandbox analysis

---

## Current State Assessment

### ✅ Strong Points

1. **Well-documented code** - All functions have rustdoc comments with What/Inputs/Output/Details format
2. **Comprehensive AUR functionality** - Covers AUR search, details, comments, PKGBUILD fetching
3. **Rate limiting built-in** - Has exponential backoff and semaphore-based serialization for `archlinux.org`
4. **Good test coverage** - Unit tests and integration tests for core functionality
5. **Async-first design** - Uses `tokio` for async operations
6. **Optional caching layer** - Memory and disk caching with configurable TTLs
7. **Retry policy** - Configurable retry with exponential backoff and error classification
8. **Mock testing support** - `MockAurApi` trait for dependency injection in tests
9. **Environment variable configuration** - Support for configuring client via environment variables
10. **Health check functionality** - Service status checking for archlinux.org endpoints

### ✅ Completed (Phase 1 - MVP)

The AUR module has been successfully extracted and published as v0.1.0 (updated to v0.1.2):

1. **Core Infrastructure** ✅
   - Standalone types (`AurPackage`, `AurPackageDetails`, `AurComment`, `HealthStatus`, `ServiceStatus`)
   - Unified error type (`ArchToolkitError` with operation-specific variants)
   - Replaced curl with reqwest
   - Shared HTTP client with rate limiting (exponential backoff + semaphore serialization)
   - Builder pattern (`ArchClientBuilder` with environment variable support)

2. **AUR Module** ✅
   - AUR search (RPC v5, up to 200 results)
   - AUR info (batch queries, comprehensive package details)
   - Comments scraping (HTML parsing, date parsing, pinned comment detection)
   - PKGBUILD fetching (cgit with dual-level rate limiting)
   - Rate limiting (exponential backoff with jitter, semaphore-based serialization)
   - Retry policy (configurable per-operation, exponential backoff, retry-after header support)
   - Caching layer (memory LRU + disk cache with JSON serialization, cache promotion)
   - Mock API for testing (`MockAurApi` trait implementation)
   - Validation config (package name validation, search query validation)

3. **Documentation & Testing** ✅
   - Comprehensive rustdoc comments (What/Inputs/Output/Details format)
   - Feature flag documentation (README and Cargo.toml)
   - Unit and integration tests (cache integration tests)
   - Example programs (`examples/aur_example.rs`, `examples/with_caching.rs`)

4. **Additional Features** ✅ (Beyond original plan)
   - Health check functionality (`health.rs` - service status checking)
   - Environment variable configuration (`env.rs` - config via env vars)
   - Cache invalidation API (`CacheInvalidator` - manual cache management)
   - Utility functions (URL encoding, JSON parsing helpers)
   - Prelude module for convenient imports

### 🚧 In Progress (Phase 2 - Dependencies Module)

The dependencies module is partially complete in v0.1.2:

1. **Dependency Parsing** ✅
   - Dependency spec parsing (`parse_dep_spec`)
   - Pacman output parsing (`parse_pacman_si_deps`, `parse_pacman_si_conflicts`)
   - .SRCINFO parsing (`parse_srcinfo`, `parse_srcinfo_deps`, `parse_srcinfo_conflicts`)
   - .SRCINFO fetching from AUR (`fetch_srcinfo` - requires `aur` feature)
   - PKGBUILD parsing (`parse_pkgbuild_deps`, `parse_pkgbuild_conflicts`)

2. **Dependency Types** ✅
   - Comprehensive type system in `src/types/dependency.rs`
   - `Dependency`, `DependencySpec`, `DependencyStatus`, `DependencySource`, etc.
   - Helper methods and Display implementations

3. **Version Comparison Utilities** ✅
   - Version comparison (`compare_versions`) with pacman-compatible algorithm
   - Version requirement checking (`version_satisfies`) with proper comparison (improved from Pacsea)
   - Major version bump detection (`is_major_version_bump`)
   - Major component extraction (`extract_major_component`)
   - Pkgrel suffix normalization
   - Comprehensive unit tests (18 tests)

4. **Package Querying** ✅
   - Installed packages query (`get_installed_packages`) using `pacman -Qq`
   - Upgradable packages query (`get_upgradable_packages`) using `pacman -Qu`
   - Installed version query (`get_installed_version`) using `pacman -Q`
   - Available version query (`get_available_version`) using `pacman -Si`
   - Provided packages check (`is_package_installed_or_provided`) with lazy checking
   - Graceful degradation when pacman is unavailable
   - Generic over `BuildHasher` for flexibility
   - Comprehensive unit tests (10 tests: 6 parsing logic, 4 integration)

5. **Source Determination** ✅
   - Source determination (`determine_dependency_source`) for installed and uninstalled packages
   - Critical system package detection (`is_system_package`)
   - Handles official repositories, AUR, and local packages
   - Uses `pacman -Qi` for installed packages and `pacman -Si` for uninstalled packages
   - Graceful degradation when pacman is unavailable
   - Generic over `BuildHasher` for flexibility
   - Comprehensive unit tests (8 tests)

6. **Dependency Resolution** ✅
   - Dependency resolver (`DependencyResolver`) with `new()`, `with_config()`, and `resolve()` methods
   - Status determination (`determine_status`) for dependency status checking
   - Batch fetching (`batch_fetch_official_deps`) for efficient pacman queries
   - Single package resolution (`resolve_package_deps`) for official, local, and AUR packages
   - Conflict detection (`fetch_package_conflicts`) for package conflicts
   - Dependency merging with status priority handling
   - PKGBUILD cache callback support via `ResolverConfig`
   - AUR integration (feature-gated, with limitations for async .SRCINFO fetching)
   - Comprehensive unit tests (7 tests)
   - Added `DependencyResolution` and `ResolverConfig` types

7. **Reverse Dependency Analysis** ✅
   - Reverse dependency analyzer (`ReverseDependencyAnalyzer`) with `new()` and `analyze()` methods
   - BFS traversal using `pacman -Qi` queries to find all packages that depend on removal targets
   - Per-root relationship tracking to distinguish direct vs transitive dependents
   - Package information caching to avoid redundant pacman calls
   - Conflict status generation with detailed reason strings
   - Source determination (official, AUR, local) based on repository information
   - System/core package detection based on groups and repository
   - Helper functions: `has_installed_required_by()`, `get_installed_required_by()`
   - Comprehensive unit tests (5 tests)
   - Added `ReverseDependencyReport` type

8. **Examples & Documentation** ✅
   - `examples/pkgbuild_example.rs` - 16 usage examples
   - `examples/srcinfo_example.rs` - Comprehensive .SRCINFO examples
   - Comprehensive unit tests

9. **Module Entry Point** ✅ (Task 2.5.1)
   - Enhanced `src/deps/mod.rs` with comprehensive module-level documentation
   - Added usage examples for all major functionality
   - Documented feature flag requirements
   - Updated `src/lib.rs` to reflect deps module is complete
   - Added deps exports to `src/prelude.rs` for convenience
   - All types and functions properly exported

10. **Testing and Documentation** ✅ (Task 2.6.1, 2.6.2, 2.6.3)
   - Comprehensive unit tests verified for all modules
   - Integration tests created in `tests/deps_integration.rs`
   - Rustdoc examples added to all public APIs
   - README updated with comprehensive deps module documentation
   - Comprehensive example program `examples/deps_example.rs` created

**Phase 2 Status: ✅ COMPLETE**
- All planned tasks (2.1.1 through 2.6.3) are complete
- Module is ready for production use
- Future enhancement: AUR dependency queries (async .SRCINFO fetching limitation noted)

**Detailed Plan**: [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md)

### ⏳ Remaining Work (Future Phases)

**Phase 3 - Index Module** (Next Priority)
- Installed package queries
- Official repository queries
- Mirror management
- Index persistence
- **Detailed Plan**: [INDEX_MODULE_PHASE.md](./INDEX_MODULE_PHASE.md)

**Phase 4+ - Remaining Modules** (Future)
The following modules are planned but not yet implemented:

### ❌ Blockers for Remaining Modules

#### 1. **Heavy Internal Dependencies**

The module depends on many Pacsea-internal types and functions:

| Import | File | Issue |
|--------|------|-------|
| `crate::state::PackageItem` | search.rs, details.rs, pkgbuild.rs | Core data type |
| `crate::state::PackageDetails` | details.rs | Core data type |
| `crate::state::Source` | search.rs, details.rs, pkgbuild.rs | Core enum |
| `crate::state::NewsItem` | news/fetch.rs | News data type |
| `crate::state::types::*` | multiple files | Many types (NewsFeedItem, AurComment, etc.) |
| `crate::state::AppState` | status/translate.rs | **Major blocker** - full app state |
| `crate::state::ArchStatusColor` | status/*.rs | UI color enum |
| `crate::util::*` | search.rs, details.rs | Utility functions |
| `crate::util::curl::*` | most files | HTTP client implementation |
| `crate::i18n` | status/translate.rs | Translation system |
| `crate::index::*` | details.rs, feeds/updates.rs | Package index queries |
| `crate::logic::files::get_pkgbuild_from_cache` | pkgbuild.rs | Cache access |

#### 2. **Curl-based HTTP Client**

The module uses a custom curl wrapper (`crate::util::curl`) instead of `reqwest` directly:
- `curl_json()` - JSON fetching
- `curl_text()` - Plain text fetching  
- `curl_text_with_args()` - With custom arguments

This is tightly coupled to Pacsea's infrastructure and not suitable for a library.

#### 3. **Translation System Coupling**

`src/sources/status/translate.rs` requires the full `AppState` for i18n translations, making it impossible to extract cleanly.

#### 4. **Index Coupling**

Several files depend on `crate::index::*`:
- `details.rs` uses `search_official()` to fill missing package fields
- `feeds/updates.rs` uses `find_package_by_name()` for update detection

---

## Why a Single Unified Crate?

After analyzing all framework-agnostic modules, a **single crate with feature flags** (`arch-toolkit`) is recommended over multiple separate crates:

| Aspect | Multiple Crates | Single Crate + Features |
|--------|----------------|------------------------|
| **Shared types** | Duplicate or depend on each other | Single source of truth |
| **HTTP client** | Each crate has its own | Shared, configured once |
| **Error handling** | Different error types | Unified error type |
| **Maintenance** | 5 repos, 5 CI pipelines | 1 repo, 1 pipeline |
| **User experience** | `cargo add` 5 times | `cargo add arch-toolkit -F aur,deps` |
| **Versioning** | Coordination nightmare | Single version |

### Proposed Modules

Based on framework-agnostic analysis, the unified crate should include:

1. **`aur`** - AUR RPC, comments, PKGBUILD fetching (from `src/sources/`)
2. **`deps`** - Dependency resolution and SRCINFO parsing (from `src/logic/deps/`)
3. **`index`** - Package database queries (from `src/index/`)
4. **`install`** - Installation command building (from `src/install/`)
5. **`news`** - Arch news RSS and security advisories (from `src/sources/news/`, `src/sources/advisories.rs`)
6. **`sandbox`** - PKGBUILD security analysis (from `src/logic/sandbox/`)

---

## Extraction Plan

### Phase 1: Define Independent Types

Create standalone types that don't depend on Pacsea internals:

```rust
// aur_client/src/types.rs

/// Package source (AUR or official repository)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageSource {
    Aur,
    Official { repo: String, arch: String },
}

/// Basic package item from search results
#[derive(Clone, Debug)]
pub struct AurPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub popularity: Option<f64>,
    pub out_of_date: Option<u64>,
    pub orphaned: bool,
    pub maintainer: Option<String>,
}

/// Full package details from info endpoint
#[derive(Clone, Debug)]
pub struct AurPackageDetails {
    pub name: String,
    pub version: String,
    pub description: String,
    pub url: Option<String>,
    pub licenses: Vec<String>,
    pub depends: Vec<String>,
    pub make_depends: Vec<String>,
    pub opt_depends: Vec<String>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub maintainer: Option<String>,
    pub first_submitted: Option<i64>,
    pub last_modified: Option<i64>,
    pub popularity: Option<f64>,
    pub num_votes: Option<u64>,
    pub out_of_date: Option<u64>,
}

/// AUR comment from package page
#[derive(Clone, Debug)]
pub struct AurComment {
    pub id: Option<String>,
    pub author: String,
    pub date: String,
    pub date_timestamp: Option<i64>,
    pub content: String,
    pub pinned: bool,
}

/// News item from Arch news RSS
#[derive(Clone, Debug)]
pub struct ArchNewsItem {
    pub title: String,
    pub date: String,
    pub url: String,
    pub summary: Option<String>,
}

/// Security advisory from security.archlinux.org
#[derive(Clone, Debug)]
pub struct SecurityAdvisory {
    pub id: String,
    pub title: String,
    pub date: String,
    pub url: Option<String>,
    pub severity: AdvisorySeverity,
    pub packages: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdvisorySeverity {
    Unknown,
    Low,
    Medium,
    High,
    Critical,
}
```

### Phase 2: Replace HTTP Client

Replace curl wrapper with direct `reqwest` usage:

```rust
// aur_client/src/client.rs

pub struct AurClient {
    http: reqwest::Client,
    rate_limiter: RateLimiter,
}

impl AurClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(format!("aur-client/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            http,
            rate_limiter: RateLimiter::new(),
        }
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<AurPackage>> { ... }
    pub async fn info(&self, names: &[&str]) -> Result<Vec<AurPackageDetails>> { ... }
    pub async fn comments(&self, package: &str) -> Result<Vec<AurComment>> { ... }
    pub async fn pkgbuild(&self, package: &str) -> Result<String> { ... }
}
```

### Phase 3: Unified Crate Structure

Create a single crate with feature-flagged modules:

```
arch-toolkit/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Re-exports based on features
│   ├── types/              # Shared data types
│   │   ├── mod.rs
│   │   ├── package.rs      # PackageInfo, PackageSource
│   │   ├── dependency.rs   # Dependency, DepTree
│   │   ├── news.rs         # NewsItem, Advisory
│   │   └── error.rs        # Unified error type
│   ├── client.rs           # Shared HTTP client + rate limiting
│   ├── aur/                # feature = "aur"
│   │   ├── mod.rs
│   │   ├── search.rs       # AUR RPC search
│   │   ├── info.rs         # AUR RPC info
│   │   ├── comments.rs     # Comment scraping
│   │   └── pkgbuild.rs     # PKGBUILD fetching
│   ├── deps/               # feature = "deps"
│   │   ├── mod.rs
│   │   ├── parse.rs        # Dependency string parsing
│   │   ├── resolve.rs      # Dependency resolution
│   │   ├── reverse.rs      # Reverse dependency analysis
│   │   └── srcinfo.rs      # SRCINFO parsing
│   ├── index/              # feature = "index"
│   │   ├── mod.rs
│   │   ├── installed.rs    # Installed package queries
│   │   ├── official.rs     # Official repo queries
│   │   └── mirrors.rs      # Mirror management
│   ├── install/            # feature = "install"
│   │   ├── mod.rs
│   │   ├── pacman.rs       # Pacman command building
│   │   ├── aur_helper.rs   # AUR helper detection
│   │   └── batch.rs        # Batch operations
│   ├── news/               # feature = "news"
│   │   ├── mod.rs
│   │   ├── arch.rs         # Arch news RSS
│   │   └── advisories.rs   # Security advisories
│   └── sandbox/            # feature = "sandbox"
│       ├── mod.rs
│       ├── analyze.rs      # PKGBUILD analysis
│       └── risk.rs         # Risk categorization
```

---

## Detailed Task List

### High Priority (Required for Publication)

#### Core Infrastructure
- [x] **Define standalone types** - Create `types/` module with all data structures independent of Pacsea
  - ✅ Implemented: `AurPackage`, `AurPackageDetails`, `AurComment` in `src/types/package.rs`
  - ✅ Implemented: `HealthStatus`, `ServiceStatus` in `src/types/health.rs`
- [x] **Create unified error type** - Define `ArchToolkitError` enum using `thiserror` instead of `Box<dyn Error>`
  - ✅ Implemented: Comprehensive error enum in `src/error.rs` with operation-specific variants
- [x] **Replace curl with reqwest** - Remove dependency on `crate::util::curl`, use `reqwest` directly
  - ✅ Implemented: All HTTP operations use `reqwest` directly
- [x] **Shared HTTP client** - Create `client.rs` with rate limiting and circuit breaker logic
  - ✅ Implemented: `ArchClient` with exponential backoff rate limiting, semaphore-based serialization
  - ✅ Implemented: Dual-level rate limiting (base delay + exponential backoff with jitter)
- [x] **Add builder pattern** - Allow configuring timeouts, user agent, rate limits per module
  - ✅ Implemented: `ArchClientBuilder` with timeout, user agent, retry policy, cache config, validation config
  - ✅ Implemented: Environment variable support via `from_env()` and `with_env()`

#### AUR Module (`feature = "aur"`)
- [x] **Remove state dependency** - Extract only stateless API functions
  - ✅ Implemented: All AUR operations are stateless, no dependency on Pacsea's `AppState`
- [x] **Remove i18n dependency** - Either return English-only or accept translation function as parameter
  - ✅ Implemented: All operations return English-only data, no i18n coupling
- [x] **Remove index dependency** - Don't call `crate::index::*` functions, let callers handle enrichment
  - ✅ Implemented: AUR operations are independent, no index queries
- [x] **Remove logic dependency** - Don't call `get_pkgbuild_from_cache`, let callers provide caching
  - ✅ Implemented: Optional caching layer via `CacheConfig`, no hard dependency on Pacsea's cache
- [x] **Port AUR search** - From `src/sources/search.rs`
  - ✅ Implemented: `Aur::search()` using AUR RPC v5, returns up to 200 results
- [x] **Port AUR info** - From `src/sources/details.rs` (AUR parts only)
  - ✅ Implemented: `Aur::info()` with batch query support, comprehensive package details
- [x] **Port comments scraping** - From `src/sources/comments.rs`
  - ✅ Implemented: `Aur::comments()` with HTML parsing, date parsing, pinned comment detection
- [x] **Port PKGBUILD fetching** - From `src/sources/pkgbuild.rs`
  - ✅ Implemented: `Aur::pkgbuild()` fetching from AUR cgit with rate limiting

#### Dependencies Module (`feature = "deps"`)

**Detailed Plan**: [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md)

- [x] **Port dependency parsing** - From `src/logic/deps/parse.rs`
  - ✅ Implemented: `parse_dep_spec()`, `parse_pacman_si_deps()`, `parse_pacman_si_conflicts()` in `src/deps/parse.rs`
- [x] **Port SRCINFO parsing** - From `src/logic/deps/srcinfo.rs`
  - ✅ Implemented: `parse_srcinfo()`, `parse_srcinfo_deps()`, `parse_srcinfo_conflicts()` in `src/deps/srcinfo.rs`
  - ✅ Implemented: `fetch_srcinfo()` for AUR integration (requires `aur` feature)
- [x] **Port PKGBUILD parsing** - From `src/logic/deps/pkgbuild.rs` (via sandbox module)
  - ✅ Implemented: `parse_pkgbuild_deps()`, `parse_pkgbuild_conflicts()` in `src/deps/pkgbuild.rs`
- [x] **Define dependency types** - Create standalone types
  - ✅ Implemented: Comprehensive type system in `src/types/dependency.rs` (9 types, 580 lines)
- [x] **Port version comparison utilities** - From `src/logic/deps/utils.rs` and `src/logic/preflight/version.rs`
  - ✅ Implemented: `compare_versions()`, `version_satisfies()`, `is_major_version_bump()`, `extract_major_component()` in `src/deps/version.rs`
  - ✅ Improved `version_satisfies()` to use proper version comparison instead of string comparison
  - ✅ Comprehensive unit tests (18 tests)
- [x] **Port package querying** - From `src/logic/deps/query.rs` and `src/logic/deps/status.rs`
  - ✅ Implemented: `get_installed_packages()`, `get_upgradable_packages()`, `get_provided_packages()`, `is_package_installed_or_provided()`, `get_installed_version()`, `get_available_version()` in `src/deps/query.rs`
  - ✅ Graceful degradation when pacman is unavailable
  - ✅ Generic over `BuildHasher` for flexibility
  - ✅ Comprehensive unit tests (10 tests)
- [x] **Port source determination** - From `src/logic/deps/source.rs`
  - ✅ Implemented: `determine_dependency_source()`, `is_system_package()` in `src/deps/source.rs`
  - ✅ Handles official repositories, AUR, and local packages
  - ✅ Uses `pacman -Qi` for installed packages and `pacman -Si` for uninstalled packages
  - ✅ Graceful degradation when pacman is unavailable
  - ✅ Generic over `BuildHasher` for flexibility
  - ✅ Comprehensive unit tests (8 tests)
- [x] **Port dependency resolution** - From `src/logic/deps/resolve.rs`
  - ✅ Implemented: `DependencyResolver` struct with `new()`, `with_config()`, and `resolve()` methods in `src/deps/resolve.rs`
  - ✅ Ported `determine_status()`, `batch_fetch_official_deps()`, `resolve_package_deps()`, `fetch_package_conflicts()`
  - ✅ Handles official, local, and AUR package resolution
  - ✅ Conflict detection and processing
  - ✅ Dependency merging with status priority
  - ✅ PKGBUILD cache callback support via `ResolverConfig`
  - ✅ AUR integration (feature-gated, with limitations for async .SRCINFO fetching)
  - ✅ Added `DependencyResolution` and `ResolverConfig` types
  - ✅ Comprehensive unit tests (7 tests)
- [x] **Port reverse deps** - From `src/logic/deps/reverse.rs`
  - ✅ Implemented: `ReverseDependencyAnalyzer` struct with `new()` and `analyze()` methods in `src/deps/reverse.rs`
  - ✅ Ported BFS traversal logic using `pacman -Qi` queries
  - ✅ Ported `fetch_pkg_info()`, `parse_key_value_output()`, `split_ws_or_none()`, `convert_entry()`
  - ✅ Implemented `has_installed_required_by()` and `get_installed_required_by()` helper functions
  - ✅ Handles direct vs transitive dependents with depth tracking
  - ✅ Conflict status generation with detailed reason strings
  - ✅ Source determination and system/core package detection
  - ✅ Added `ReverseDependencyReport` type
  - ✅ Comprehensive unit tests (5 tests)
- [ ] **Port AUR dependency queries** - From `src/logic/deps/aur.rs`

#### Index Module (`feature = "index"`)
- [x] **Create index types** - `OfficialPackage`, `OfficialIndex`, `IndexQueryResult`, `InstalledPackagesMode` (Task 3.1.1)
- [x] **Port installed package queries** - From `src/index/installed.rs` (Task 3.2.1 - complete)
- [x] **Port explicit package tracking** - From `src/index/explicit.rs` (Task 3.2.2 - complete)
- [x] **Port official repo queries** - From `src/index/query.rs` (Task 3.3 - complete)
- [ ] **Port index persistence** - From `src/index/persist.rs` (Task 3.4 - pending)
- [ ] **Port mirror management** - From `src/index/mirrors.rs` (Task 3.5 - optional, Windows-specific, pending)
- [x] **Remove Pacsea-specific caching** - Let callers handle persistence (for completed tasks)
- **Detailed Plan**: [INDEX_MODULE_PHASE.md](./INDEX_MODULE_PHASE.md)

#### Install Module (`feature = "install"`)
- [ ] **Port pacman command building** - From `src/install/command.rs`
- [ ] **Port AUR helper detection** - From `src/install/executor.rs`
- [ ] **Port batch operations** - From `src/install/batch.rs`
- [ ] **Remove dry-run coupling** - Accept as parameter instead of global state

#### News Module (`feature = "news"`)
- [ ] **Port Arch news RSS** - From `src/sources/news/`
- [ ] **Port security advisories** - From `src/sources/advisories.rs`
- [ ] **Remove status translation** - Drop `src/sources/status/translate.rs` (i18n coupling)

#### Sandbox Module (`feature = "sandbox"`)
- [ ] **Port PKGBUILD analysis** - From `src/logic/sandbox/analyze.rs`
- [ ] **Port risk categorization** - From `src/logic/sandbox/types.rs`
- [ ] **Port sandbox parsing** - From `src/logic/sandbox/parse.rs`

#### Documentation & Testing
- [x] **Write comprehensive docs** - Add crate-level documentation with examples for each module
  - ✅ Implemented: Comprehensive rustdoc comments with What/Inputs/Output/Details format
  - ✅ Implemented: Crate-level documentation in `src/lib.rs` with usage examples
  - ✅ Implemented: README with quick start examples
- [x] **Add feature flag documentation** - Document which features enable which modules
  - ✅ Implemented: Feature flags documented in README and Cargo.toml
- [x] **Port existing tests** - Adapt Pacsea's tests to work with new API
  - ✅ Implemented: Unit tests for search, info, comments, pkgbuild parsing
  - ✅ Implemented: Cache integration tests in `tests/cache_integration.rs`
- [x] **Add integration tests** - Test feature combinations
  - ✅ Implemented: Integration tests for caching layer with memory and disk backends

### Medium Priority (Nice to Have)

- [x] **Add retry logic** - Configurable retry with exponential backoff
  - ✅ Implemented: `RetryPolicy` with per-operation enable/disable flags
  - ✅ Implemented: Exponential backoff with configurable initial/max delays and jitter
  - ✅ Implemented: Automatic retry-after header handling
  - ✅ Implemented: Error classification (timeouts, 5xx, 429 are retryable)
- [x] **Add caching layer** - Optional caching trait for callers to implement
  - ✅ Implemented: Generic `Cache<K, V>` trait for extensibility
  - ✅ Implemented: `MemoryCache` (LRU) and `DiskCache` implementations
  - ✅ Implemented: `CacheConfig` with per-operation TTL configuration
  - ✅ Implemented: Cache promotion from disk to memory on hit
  - ✅ Implemented: `CacheInvalidator` API for manual cache management
- [ ] **Add pagination support** - Handle large result sets
  - ⏳ Not yet implemented (AUR RPC returns up to 200 results, which is usually sufficient)
- [ ] **Add streaming support** - Return streams for large responses
  - ⏳ Not yet implemented (current API returns complete results)
- [x] **Add mock testing support** - Mockable HTTP client for testing
  - ✅ Implemented: `MockAurApi` trait implementation for testing
  - ✅ Implemented: `AurApi` trait for dependency injection
- [x] **Add CI/CD setup** - GitHub Actions for testing and publishing
  - ✅ Implemented: GitHub Actions workflows for build, test, docs, release, and security analysis

### Low Priority (Future)

- [ ] **Add official repo support** - Fetch details from archlinux.org
- [ ] **Add mirror status** - Fetch mirror health information
- [ ] **Add WebSocket support** - Real-time updates (if AUR ever supports it)

---

## Estimated Effort

| Task | Effort | Complexity |
|------|--------|------------|
| **Core Infrastructure** | | |
| Define standalone types | 4-6 hours | Medium |
| Unified error type | 2-3 hours | Low |
| Replace curl with reqwest | 4-6 hours | Medium |
| Shared HTTP client | 3-4 hours | Medium |
| Builder pattern | 2-3 hours | Low |
| **AUR Module** | | |
| Port AUR search/info | 4-6 hours | Medium |
| Port comments/PKGBUILD | 3-4 hours | Medium |
| Remove dependencies | 2-3 hours | Low |
| **Dependencies Module** | | |
| Port dependency parsing | 4-6 hours | Medium | ✅ Complete (v0.1.2) |
| Port dependency resolution | 6-8 hours | High | ⏳ Pending |
| **Index Module** | | |
| Port index queries | 4-6 hours | Medium |
| **Install Module** | | |
| Port install commands | 3-4 hours | Medium |
| **News Module** | | |
| Port news/advisories | 3-4 hours | Medium |
| **Sandbox Module** | | |
| Port sandbox analysis | 2-3 hours | Low |
| **Documentation & Testing** | | |
| Documentation | 4-6 hours | Low |
| Testing | 6-8 hours | Medium |
| **Total** | **54-78 hours** | |

**Note**: This is a significant refactoring effort. Consider doing it incrementally:
1. Start with AUR module only (20-30 hours)
2. Add dependencies module (6-8 hours)
3. Add remaining modules as needed

---

## Recommended Crate Structure

```toml
# Cargo.toml
[package]
name = "arch-toolkit"
version = "0.1.2"
edition = "2024"
description = "Complete Rust toolkit for Arch Linux package management"
license = "MIT"
repository = "https://github.com/Firstp1ck/arch-toolkit"
keywords = ["archlinux", "aur", "pacman", "package-manager"]
categories = ["api-bindings", "command-line-utilities"]

[features]
default = ["aur"]
aur = ["dep:reqwest", "dep:tokio", "dep:scraper", "dep:chrono", "dep:rand", "dep:lru", "dep:async-trait"]  # AUR RPC, comments, PKGBUILD
deps = []                                       # Dependency parsing (types only, no additional deps)
cache-disk = ["dep:dirs"]                      # Disk-based caching
index = []                                      # Package database queries
install = ["deps"]                              # Installation commands (requires deps)
news = ["reqwest", "tokio", "chrono"]           # News feeds and advisories
sandbox = []                                    # PKGBUILD security analysis
alpm = ["alpm-rs"]                              # Optional libalpm integration
full = ["aur", "deps", "index", "install", "news", "sandbox"]

[dependencies]
# Always included (minimal)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"

# HTTP client (for aur, news features)
reqwest = { version = "0.12", features = ["json"], optional = true }
tokio = { version = "1", features = ["rt", "time"], optional = true }

# HTML parsing (for aur comments, news)
scraper = { version = "0.25", optional = true }

# Date handling
chrono = { version = "0.4", optional = true }

# Fuzzy matching (for deps)
fuzzy-matcher = { version = "0.3", optional = true }

# Optional libalpm integration (for advanced pacman operations)
alpm-rs = { version = "0.1", optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
wiremock = "0.6"

[package.metadata.docs.rs]
all-features = true
```

### Optional Integration with Existing Crates

The toolkit can optionally integrate with `alpm-rs` for advanced pacman operations:

```rust
// When feature = "alpm" is enabled
use arch_toolkit::alpm;

// Use libalpm for low-level pacman operations
let handle = alpm::initialize("/", "/var/lib/pacman")?;
let db = handle.register_syncdb("core", alpm::SigLevel::NONE)?;
let pkg = db.pkg("pacman")?;
```

**Benefits:**
- Pure Rust by default (no C dependencies)
- Optional libalpm access for advanced use cases
- Best of both worlds: convenience + power

---

## API Design Sketch

### Complete Example Using All Features

```rust
use arch_toolkit::{AurClient, PackageInfo, DependencyTree};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), arch_toolkit::Error> {
    // Create client with default settings
    let client = AurClient::new();
    
    // Or customize
    let client = AurClient::builder()
        .timeout(Duration::from_secs(30))
        .rate_limit(Duration::from_millis(100))
        .user_agent("my-app/1.0")
        .build()?;
    
    // AUR operations
    let packages = client.search("yay").await?;
    let details = client.info(&["yay", "paru"]).await?;
    let comments = client.comments("yay").await?;
    let pkgbuild = client.pkgbuild("yay").await?;
    
    // Dependency resolution
    let tree = DependencyTree::resolve(&["firefox", "chromium"])?;
    for dep in tree.iter() {
        println!("{} -> {:?}", dep.name, dep.source);
    }
    
    // Package index
    let installed = arch_toolkit::index::installed_packages()?;
    let official = arch_toolkit::index::search_official("ripgrep")?;
    
    // Installation commands (dry-run by default)
    let cmd = arch_toolkit::install::build_command(&["yay"], true)?;
    println!("Would run: {}", cmd);
    
    // News
    let news = arch_toolkit::news::fetch_arch_news(10).await?;
    let advisories = arch_toolkit::news::fetch_advisories(10).await?;
    
    // Sandbox analysis
    let risks = arch_toolkit::sandbox::analyze(&pkgbuild)?;
    
    Ok(())
}
```

### Usage Patterns

```toml
# Minimal - just AUR search
[dependencies]
arch-toolkit = { version = "0.1.2", default-features = false, features = ["aur"] }

# Dependency parsing only
[dependencies]
arch-toolkit = { version = "0.1.2", default-features = false, features = ["deps"] }

# AUR + Dependency parsing
[dependencies]
arch-toolkit = { version = "0.1.2", features = ["aur", "deps"] }

# Full TUI app (like Pacsea) - when all modules are complete
[dependencies]
arch-toolkit = { version = "0.1.2", features = ["full"] }

# CLI tool for package queries - when index module is complete
[dependencies]
arch-toolkit = { version = "0.1.2", features = ["aur", "index"] }
```

### Benefits for Pacsea

Pacsea could then depend on `arch-toolkit` internally:

```toml
# Pacsea's Cargo.toml
[dependencies]
arch-toolkit = { version = "0.1.2", features = ["aur", "deps"] }
ratatui = "0.29"
crossterm = "0.29"
# ... UI-specific deps
```

This would:
1. **Reduce Pacsea's codebase** - Move ~4000+ lines of framework-agnostic code to the toolkit
2. **Enable others to build on it** - GTK, web, CLI tools can all use the same backend
3. **Improve testing** - Core logic tested independently from UI
4. **Attract contributors** - Lower barrier for non-TUI contributions

---

## Conclusion

### Phase 1 Status: ✅ COMPLETED

The AUR module has been successfully extracted from Pacsea and published as `arch-toolkit v0.1.0` (current version: v0.1.2). All blockers have been resolved:

1. ✅ **Decoupled from Pacsea types** - Created standalone types (`AurPackage`, `AurPackageDetails`, `AurComment`)
2. ✅ **Replaced curl with reqwest** - All HTTP operations use standard `reqwest` client
3. ✅ **Removed i18n dependency** - All operations return English-only data
4. ✅ **Optional caching** - Caching is optional via `CacheConfig`, no hard dependencies

### Phase 2 Status: ✅ COMPLETED

The Dependencies Module is complete in v0.1.2:

1. **Dependencies Module** - ✅ Complete (all tasks 2.1.1 through 2.6.3)
   - ✅ Parsing functions (SRCINFO, PKGBUILD, dependency specs)
   - ✅ Version comparison utilities
   - ✅ Package querying
   - ✅ Source determination
   - ✅ Dependency resolution
   - ✅ Reverse dependency analysis
   - ✅ Module entry point
   - **Plan Document**: [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md)

### Phase 3 Status: 🚧 IN PROGRESS

The Index Module is partially complete:

1. **Index Module** - 🚧 In Progress
   - ✅ Index types (`OfficialPackage`, `OfficialIndex`, `IndexQueryResult`, `InstalledPackagesMode`) - Task 3.1.1 complete
   - ✅ Installed package queries (`refresh_installed_cache`, `is_installed`, `get_installed_packages`) - Task 3.2.1 complete
   - ✅ Explicit package tracking (`refresh_explicit_cache`, `is_explicit`) - Task 3.2.2 complete
   - ✅ Official repo queries (`search_official`, `all_official`, `fetch_official_index`, `fetch_official_index_async`) - Task 3.3 complete
   - ✅ Module entry point updated with query/fetch modules - Task 3.7 partial
   - ⏳ Index persistence (Task 3.4 - pending)
   - ⏳ Mirror management (Task 3.5 - optional, pending)
   - **Plan Document**: [INDEX_MODULE_PHASE.md](./INDEX_MODULE_PHASE.md)

### Phase 4+ Status: ⏳ PLANNED

The following modules are planned but not yet started:

1. **Install Module** - ⏳ Pacman command building, AUR helper detection, batch operations
2. **News Module** - ⏳ Arch news RSS, security advisories
3. **Sandbox Module** - ⏳ PKGBUILD security analysis

These modules may still have blockers similar to what the AUR module had:

### Recommended Approach

**Create a unified `arch-toolkit` crate** with feature flags, starting fresh with a clean API design:

1. **Phase 1 (MVP)**: Extract AUR module only (~20-30 hours) ✅ **COMPLETED**
   - Most reusable and independent
   - Can be published and used immediately
   - Validates the approach
   - **Status**: Published as v0.1.0 on 2025-12-21, updated to v0.1.2 on 2025-12-22

2. **Phase 2**: Add dependencies module (~30-40 hours) ✅ **COMPLETED**
   - High reuse value
   - Complements AUR module
   - **Status**: Complete - all core functionality implemented (v0.1.2)
   - **Completed**: Dependency types, parsing (specs, SRCINFO, PKGBUILD, pacman output), version comparison utilities, package querying, source determination, dependency resolution, reverse dependency analysis, module entry point
   - **Remaining**: AUR dependency queries (async .SRCINFO fetching limitation noted - future enhancement)
   - **Plan Document**: [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md)

3. **Phase 3**: Add index module (~20-30 hours) 🚧 **IN PROGRESS**
   - ✅ Index types (Task 3.1.1 - complete)
   - ✅ Installed package queries (Task 3.2.1 - complete)
   - ✅ Explicit package tracking (Task 3.2.2 - complete)
   - ✅ Official repository queries (Task 3.3 - complete)
   - ⏳ Index persistence (Task 3.4 - pending)
   - ⏳ Mirror management (Task 3.5 - optional, pending)
   - **Status**: Tasks 3.1, 3.2, and 3.3 complete, remaining tasks pending
   - **Plan Document**: [INDEX_MODULE_PHASE.md](./INDEX_MODULE_PHASE.md)

4. **Phase 4+**: Add remaining modules incrementally ⏳ **PLANNED**
   - Install, news, sandbox as needed
   - Each can be added independently
   - **Status**: Not yet started

### Benefits of Unified Crate

- **Single dependency** for users: `cargo add arch-toolkit -F aur,deps`
- **Shared types** across all modules (no duplication)
- **Unified error handling** with `ArchToolkitError`
- **Easier maintenance** - one repo, one CI pipeline
- **Better for Pacsea** - can migrate incrementally, reducing codebase size
- **Enables other projects** - GTK apps, web frontends, CLI tools can all use the same backend

### Migration Strategy for Pacsea

Now that `arch-toolkit v0.1.2` is published, Pacsea can:

1. ✅ Add `arch-toolkit` as dependency with `features = ["aur"]`
2. ⏳ Gradually replace AUR-related modules with toolkit calls
   - Replace `src/sources/search.rs` with `arch_toolkit::ArchClient::aur().search()`
   - Replace `src/sources/details.rs` (AUR parts) with `arch_toolkit::ArchClient::aur().info()`
   - Replace `src/sources/comments.rs` with `arch_toolkit::ArchClient::aur().comments()`
   - Replace `src/sources/pkgbuild.rs` with `arch_toolkit::ArchClient::aur().pkgbuild()`
3. ⏳ Remove duplicated AUR code from Pacsea
4. ⏳ Focus development on TUI-specific features

**Next Steps for Remaining Modules:**
- Once dependencies, index, install, news, and sandbox modules are added to arch-toolkit, Pacsea can migrate those as well
- This will further reduce Pacsea's codebase size and maintenance burden

This approach benefits both the toolkit (real-world usage and testing) and Pacsea (reduced maintenance burden).

