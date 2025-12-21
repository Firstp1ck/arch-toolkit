# Changelog

All notable changes to arch-toolkit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

