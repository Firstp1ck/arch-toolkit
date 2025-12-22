# Arch Toolkit Crate Preparation

This document analyzes framework-agnostic modules in Pacsea (`src/sources/`, `src/logic/`, `src/index/`, `src/install/`) for extraction into a unified `arch-toolkit` crate with feature flags. This single crate approach is recommended over multiple separate crates for better maintainability, shared types, and user experience.

## Current Status

**Phase 1 (MVP) - AUR Module: ✅ COMPLETED**

- **Version**: v0.1.0 (published 2025-12-21)
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

**Phase 2+ - Remaining Modules: ⏳ PLANNED**

- Dependencies module (SRCINFO parsing, dependency resolution)
- Index module (package database queries)
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
- ✅ Dependency resolution (SRCINFO parsing, tree building)
- ✅ Package index queries (installed, official repos)
- ✅ Installation command building
- ✅ News feeds and advisories
- ✅ PKGBUILD security analysis

**Gaps in existing crates:**
1. **Fragmented** - Need multiple crates for complete functionality
2. **No dependency resolution** - Missing SRCINFO parsing and dep tree building
3. **No news/advisories** - No Arch news RSS or security advisory support
4. **No comments** - AUR comment scraping not available
5. **No sandbox analysis** - PKGBUILD security analysis missing
6. **No unified error types** - Each crate has its own error handling
7. **No rate limiting** - Missing built-in rate limiting for archlinux.org
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
   - ✅ SRCINFO parsing (`alpm-pkgbuild` exists but different focus)
   - ✅ Dependency tree building (unique)
   - ✅ Reverse dependency analysis (unique)
   - ✅ Version constraint parsing

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

The AUR module has been successfully extracted and published as v0.1.0:

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

### ⏳ Remaining Work (Future Phases)

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

- [ ] **Port dependency parsing** - From `src/logic/deps/parse.rs`
- [ ] **Port dependency resolution** - From `src/logic/deps/resolve.rs`
- [ ] **Port reverse deps** - From `src/logic/deps/reverse.rs`
- [ ] **Port SRCINFO parsing** - From `src/logic/deps/srcinfo.rs`
- [ ] **Port AUR dependency queries** - From `src/logic/deps/aur.rs`

#### Index Module (`feature = "index"`)
- [ ] **Port installed package queries** - From `src/index/installed.rs`
- [ ] **Port official repo queries** - From `src/index/query.rs`
- [ ] **Port mirror management** - From `src/index/mirrors.rs`
- [ ] **Remove Pacsea-specific caching** - Let callers handle persistence

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
| Port dependency logic | 6-8 hours | High |
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
version = "0.1.0"
edition = "2024"
description = "Complete Rust toolkit for Arch Linux package management"
license = "MIT"
repository = "https://github.com/Firstp1ck/arch-toolkit"
keywords = ["archlinux", "aur", "pacman", "package-manager"]
categories = ["api-bindings", "command-line-utilities"]

[features]
default = ["aur", "deps"]
aur = ["reqwest", "tokio", "scraper"]           # AUR RPC, comments, PKGBUILD
deps = ["fuzzy-matcher"]                        # Dependency resolution
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
arch-toolkit = { version = "0.1", default-features = false, features = ["aur"] }

# Dependency resolution only
[dependencies]
arch-toolkit = { version = "0.1", default-features = false, features = ["deps"] }

# Full TUI app (like Pacsea)
[dependencies]
arch-toolkit = { version = "0.1", features = ["full"] }

# CLI tool for package queries
[dependencies]
arch-toolkit = { version = "0.1", features = ["aur", "index"] }
```

### Benefits for Pacsea

Pacsea could then depend on `arch-toolkit` internally:

```toml
# Pacsea's Cargo.toml
[dependencies]
arch-toolkit = { version = "0.1", features = ["full"] }
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

The AUR module has been successfully extracted from Pacsea and published as `arch-toolkit v0.1.0`. All blockers have been resolved:

1. ✅ **Decoupled from Pacsea types** - Created standalone types (`AurPackage`, `AurPackageDetails`, `AurComment`)
2. ✅ **Replaced curl with reqwest** - All HTTP operations use standard `reqwest` client
3. ✅ **Removed i18n dependency** - All operations return English-only data
4. ✅ **Optional caching** - Caching is optional via `CacheConfig`, no hard dependencies

### Remaining Modules

The following modules from Pacsea still need extraction (future phases):

1. **Dependencies Module** - SRCINFO parsing, dependency resolution, reverse deps
2. **Index Module** - Installed package queries, official repo queries, mirror management
3. **Install Module** - Pacman command building, AUR helper detection, batch operations
4. **News Module** - Arch news RSS, security advisories
5. **Sandbox Module** - PKGBUILD security analysis

These modules may still have blockers similar to what the AUR module had:

### Recommended Approach

**Create a unified `arch-toolkit` crate** with feature flags, starting fresh with a clean API design:

1. **Phase 1 (MVP)**: Extract AUR module only (~20-30 hours) ✅ **COMPLETED**
   - Most reusable and independent
   - Can be published and used immediately
   - Validates the approach
   - **Status**: Published as v0.1.0 on 2025-12-21

2. **Phase 2**: Add dependencies module (~30-40 hours) ⏳ **PLANNED**
   - High reuse value
   - Complements AUR module
   - **Status**: Detailed plan created
   - **Plan Document**: [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md)

3. **Phase 3**: Add remaining modules incrementally ⏳ **PLANNED**
   - Index, install, news, sandbox as needed
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

Now that `arch-toolkit v0.1.0` is published, Pacsea can:

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

