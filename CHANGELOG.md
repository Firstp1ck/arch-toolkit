# Changelog

All notable changes to arch-toolkit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2025-12-22

# Release v0.1.2 - Dependency Parsing

**Release Date**: 2025-12-22

## 🚀 What's New

This release introduces comprehensive dependency parsing capabilities, allowing you to extract and analyze dependencies from PKGBUILD files, .SRCINFO files, and pacman output.

### ✨ New Features

**Dependency Parsing Module (`deps` feature)**
- Parse dependencies from PKGBUILD files
  - Supports single-line and multi-line bash array syntax
  - Handles append syntax (`depends+=`) in PKGBUILD functions
  - Extracts `depends`, `makedepends`, `checkdepends`, and `optdepends`
  - Automatic filtering of virtual packages (.so files)
  - Automatic deduplication of dependencies
- Parse dependencies from .SRCINFO files
  - Extract structured data from .SRCINFO content
  - Handle architecture-specific dependencies
  - Support for split packages
  - Fetch .SRCINFO from AUR (requires `aur` feature)
- Parse dependency specifications
  - Parse dependency specs with version constraints (e.g., `package>=1.2.3`)
  - Parse pacman `-Si` output for dependencies and conflicts
  - Handle multi-line dependencies and deduplication

**Dependency Types**
- Comprehensive type system for dependency management
- Support for dependency status, sources, and specifications
- Ready for future dependency resolution features

### 📚 Examples

Two new example files demonstrate the parsing capabilities:
- `examples/pkgbuild_example.rs` - 16 usage examples for PKGBUILD parsing
- `examples/srcinfo_example.rs` - Comprehensive .SRCINFO parsing examples

## Quick Start

### Parsing PKGBUILD Files

```rust
use arch_toolkit::deps::parse_pkgbuild_deps;

let pkgbuild = r"
depends=('glibc' 'python>=3.10')
makedepends=('make' 'gcc')
";

let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);
println!("Runtime dependencies: {:?}", depends);
```

### Parsing .SRCINFO Files

```rust
use arch_toolkit::deps::{parse_srcinfo, parse_srcinfo_deps};

let srcinfo = r"
pkgbase = example-package
depends = glibc
depends = python>=3.10
makedepends = make
";

// Parse full .SRCINFO into structured data
let data = parse_srcinfo(srcinfo)?;

// Or just extract dependencies
let (depends, makedepends, checkdepends, optdepends) = parse_srcinfo_deps(srcinfo);
```

### Fetching .SRCINFO from AUR

```rust
use arch_toolkit::deps::fetch_srcinfo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let srcinfo = fetch_srcinfo("yay").await?;
    let data = parse_srcinfo(&srcinfo)?;
    println!("Package: {}", data.pkgbase);
    Ok(())
}
```

### Parsing Dependency Specifications

```rust
use arch_toolkit::deps::parse_dep_spec;

let spec = "python>=3.10";
let dep = parse_dep_spec(spec)?;
println!("Package: {}, Version: {:?}", dep.name, dep.version);
```

## Installation

```bash
cargo add arch-toolkit@0.1.2 --features deps
```

Or update your `Cargo.toml`:

```toml
[dependencies]
arch-toolkit = { version = "0.1.2", features = ["deps"] }
```

For AUR integration (fetching .SRCINFO):

```toml
[dependencies]
arch-toolkit = { version = "0.1.2", features = ["deps", "aur"] }
```

## Migration from v0.1.1

No breaking changes! This is a backward-compatible release. All existing code will continue to work.

**New capabilities:**
- Enable the `deps` feature to access dependency parsing functions
- Use `parse_pkgbuild_deps()` to extract dependencies from PKGBUILD files
- Use `parse_srcinfo()` or `fetch_srcinfo()` for .SRCINFO parsing
- Check out the example files for comprehensive usage patterns

## Documentation

- [Full API Documentation](https://docs.rs/arch-toolkit/0.1.2)
- [GitHub Repository](https://github.com/Firstp1ck/arch-toolkit)
- [Examples](https://github.com/Firstp1ck/arch-toolkit/tree/main/examples)

## Feedback

Found a bug or have a feature request? Open an issue on [GitHub](https://github.com/Firstp1ck/arch-toolkit/issues)!

---

## [0.1.1] - 2025-12-22

# Release v0.1.1 - Developer Experience & Robustness

**Release Date**: 2025-12-21

## 🚀 What's New

This release focuses on improving developer experience, adding configuration flexibility, and enhancing robustness with input validation and health checks.

### ✨ New Features

**Environment Variable Configuration**
- Configure the client entirely via environment variables
- Support for `ARCH_TOOLKIT_*` variables:
  - `ARCH_TOOLKIT_TIMEOUT` - Request timeout
  - `ARCH_TOOLKIT_USER_AGENT` - Custom user agent
  - `ARCH_TOOLKIT_MAX_RETRIES` - Maximum retry attempts
  - `ARCH_TOOLKIT_RETRY_ENABLED` - Enable/disable retries
  - `ARCH_TOOLKIT_VALIDATION_STRICT` - Strict validation mode
  - `ARCH_TOOLKIT_CACHE_SIZE` - Cache size configuration
- Perfect for CI/CD and Docker environments
- Use `ArchClientBuilder::from_env()` for pure environment-based configuration
- Use `ArchClientBuilder::with_env()` to merge env vars into existing builder

**Health Check Functionality**
- Check AUR service status and latency
- `client.health_check()` - Quick health check
- `client.health_status()` - Detailed health status with latency
- Automatic status classification:
  - `Healthy` - Service responding quickly (< 2s)
  - `Degraded` - Service responding slowly (≥ 2s)
  - `Unreachable` - Service not responding
  - `Timeout` - Request timed out
- Configurable health check timeout

**Input Validation**
- Automatic validation of package names and search queries
- Validates against Arch Linux packaging standards
- Configurable validation behavior:
  - Strict mode: Returns errors for empty/invalid inputs
  - Lenient mode: Returns empty results for invalid inputs
- New error types:
  - `EmptyInput` - Empty input provided
  - `InvalidPackageName` - Invalid package name format
  - `InvalidSearchQuery` - Invalid search query
  - `InputTooLong` - Input exceeds maximum length

**Prelude Module**
- Convenient single-import module: `use arch_toolkit::prelude::*;`
- Re-exports all commonly used types and functions
- Cleaner imports for common use cases

**Rich Error Context**
- Enhanced error messages with more context
- Better error classification and reporting
- Improved debugging experience

**Trait-Based Design**
- `AurApi` trait for better testability
- Easier to mock and test AUR operations
- More flexible architecture

### 🔧 Improvements

- Better error messages with more context
- Improved testability with trait-based design
- Enhanced robustness with input validation
- MIT License added to project
- Various dev usability improvements

## Quick Start

### Using Environment Variables

```bash
export ARCH_TOOLKIT_TIMEOUT=60
export ARCH_TOOLKIT_USER_AGENT="my-app/1.0"
export ARCH_TOOLKIT_MAX_RETRIES=3
```

```rust
use arch_toolkit::ArchClient;

// Create client from environment variables
let client = ArchClient::builder()
    .from_env()
    .build()?;
```

### Health Checks

```rust
use arch_toolkit::ArchClient;

let client = ArchClient::new()?;

// Quick health check
let is_healthy = client.health_check().await?;
println!("AUR is healthy: {}", is_healthy);

// Detailed health status
let status = client.health_status().await?;
println!("Status: {:?}, Latency: {:?}", status.status, status.latency);
```

### Input Validation

```rust
use arch_toolkit::ArchClient;
use arch_toolkit::validation::ValidationConfig;

// Strict validation (default)
let client = ArchClient::new()?;
// Empty search will return an error
let result = client.aur().search("").await; // Returns Err(EmptyInput)

// Lenient validation
let config = ValidationConfig {
    strict_empty: false,
    ..Default::default()
};
let client = ArchClient::builder()
    .validation_config(config)
    .build()?;
// Empty search will return empty results
let result = client.aur().search("").await?; // Returns Ok(vec![])
```

### Using Prelude

```rust
use arch_toolkit::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = ArchClient::new()?;
    let packages = client.aur().search("yay").await?;
    Ok(())
}
```

## Migration from v0.1.0

No breaking changes! This is a backward-compatible release. All existing code will continue to work.

**Optional upgrades:**
- Consider using `prelude::*` for cleaner imports
- Add health checks to monitor AUR service status
- Use environment variables for configuration in CI/CD environments
- Enable input validation for better error handling

## Installation

```bash
cargo add arch-toolkit@0.1.1 --features aur
```

Or update your `Cargo.toml`:

```toml
[dependencies]
arch-toolkit = { version = "0.1.1", features = ["aur"] }
```

## Documentation

- [Full API Documentation](https://docs.rs/arch-toolkit/0.1.1)
- [GitHub Repository](https://github.com/Firstp1ck/arch-toolkit)
- [Examples](https://github.com/Firstp1ck/arch-toolkit/tree/main/examples)

## Feedback

Found a bug or have a feature request? Open an issue on [GitHub](https://github.com/Firstp1ck/arch-toolkit/issues)!

---

## [0.1.0] - 2025-12-21

### Added

#### AUR Module (`feature = "aur"`)
- **AUR Search**: Search for packages in the Arch User Repository by name
  - Uses AUR RPC v5 search endpoint
  - Returns up to 200 results
  - Includes popularity, out-of-date status, and maintainer information
- **AUR Package Info**: Fetch detailed information for one or more AUR packages
  - Batch query support (fetch multiple packages in a single request)
  - Comprehensive package details including dependencies, licenses, groups, provides, conflicts, replaces
  - Timestamps for first submission and last modification
  - Vote counts and popularity scores
- **AUR Comments**: Scrape and parse comments from AUR package pages
  - HTML parsing with `scraper` crate
  - Date parsing and timezone conversion
  - Pinned comment detection
  - Markdown-like formatting for comment content
  - Sorted by date (latest first)
- **PKGBUILD Fetching**: Retrieve PKGBUILD content for AUR packages
  - Fetches from AUR cgit repository
  - Dual-level rate limiting (200ms minimum interval + exponential backoff)
  - 10-second timeout
- **Optional Caching Layer**: In-memory and disk caching for AUR operations
  - In-memory LRU cache with configurable TTL per operation
  - Optional disk cache with JSON serialization for persistence
  - Per-operation cache enable/disable flags (search, info, comments, pkgbuild)
  - Configurable TTLs for each operation type
  - Cache promotion from disk to memory on hit
  - Thread-safe implementation with Arc and Mutex
  - Standardized cache key generation functions for consistent key formatting
  - Generic `Cache<K, V>` trait for extensible cache implementations

#### Core Infrastructure
- **Unified Error Type**: `ArchToolkitError` enum with comprehensive error variants
  - Network errors (HTTP/network failures)
  - JSON parsing errors
  - Custom parsing errors
  - Rate limiting errors with retry-after information
  - Not found errors
  - Invalid input errors
  - Cache errors (serialization, I/O, expiration)
- **HTTP Client with Rate Limiting**: Shared client with built-in rate limiting
  - Exponential backoff for archlinux.org requests (500ms base, max 60s)
  - Semaphore-based request serialization
  - Random jitter to prevent thundering herd
  - Automatic backoff reset on successful requests
- **Client Builder Pattern**: `ArchClientBuilder` for flexible client configuration
  - Custom timeout configuration
  - User agent customization
  - Retry policy configuration with exponential backoff
  - Cache configuration support
- **Retry Policy**: Configurable retry behavior for transient network failures
  - Exponential backoff with configurable initial and max delays
  - Random jitter to prevent thundering herd
  - Per-operation retry enable/disable flags (search, info, comments, pkgbuild)
  - Automatic retry-after header handling
  - Error classification (timeouts, 5xx, 429 are retryable)
- **Cache Invalidation API**: `CacheInvalidator` builder for manual cache management
  - Invalidate specific search queries
  - Invalidate info cache for specific packages
  - Invalidate comments cache for specific packages
  - Invalidate pkgbuild cache for specific packages
  - Invalidate all caches for a package
  - Clear all caches
- **AUR Operations Wrapper**: `Aur` struct providing fluent API for AUR operations
  - Method chaining: `client.aur().search()`, `client.aur().info()`, etc.
  - Automatic rate limiting and retry handling
  - Integrated caching when configured
- **Type System**: Comprehensive data types for AUR operations
  - `AurPackage`: Minimal package info for search results
  - `AurPackageDetails`: Full package details with all metadata
  - `AurComment`: Comment structure with author, date, content, pinned status
- **Utility Functions**: JSON parsing and URL encoding helpers
  - `percent_encode()`: RFC 3986-compliant URL encoding
  - `s()`, `ss()`, `arrs()`, `u64_of()`: JSON extraction helpers
  - `is_retryable_error()`: Classify network errors as retryable or not
  - `extract_retry_after()`: Extract Retry-After header from HTTP responses
  - `cache_key_*()`: Standardized cache key generation for all operation types

#### Documentation
- Comprehensive rustdoc comments for all public APIs
  - What/Inputs/Output/Details format
  - Usage examples in documentation
  - Error documentation with `# Errors` sections
- Crate-level documentation with examples for all AUR operations
- README with usage examples
- Example programs demonstrating AUR operations (`examples/aur_example.rs`)
- Example program demonstrating caching layer (`examples/with_caching.rs`)

#### Testing
- Unit tests for search and info parsing
- Test coverage for JSON parsing edge cases
- Unit tests for cache implementations (memory and disk)
- Integration tests for caching layer (`tests/cache_integration.rs`)
- Test coverage for cache TTL expiration and LRU eviction
- Thread safety tests for concurrent cache access

#### Development Infrastructure
- GitHub Pull Request template for standardized PRs
- GitHub issue templates for bug reports and feature requests
- GitHub Actions workflows for CI/CD
  - Rust build and test workflow
  - Documentation deployment workflow
  - Release automation workflow
  - CodeQL security analysis workflow
- Cursor IDE commands for development workflow

### Technical Details
- **Async-first design**: All I/O operations use `tokio` async/await
- **Feature flags**: Modular design with `aur` and `cache-disk` feature flags
- **Zero dependencies by default**: Minimal core dependencies (serde, thiserror, tracing)
- **Optional dependencies**: HTTP client, HTML parsing, date handling only when `aur` feature is enabled
- **Caching dependencies**: LRU cache and directory utilities only when caching is used
- **Strict code quality**: Clippy pedantic and nursery rules enabled
- **Complexity thresholds**: Cyclomatic and data flow complexity < 25
- **Cache design**: Generic `Cache<K, V>` trait for extensibility

[0.1.0]: https://github.com/Firstp1ck/arch-toolkit/releases/tag/v0.1.0
