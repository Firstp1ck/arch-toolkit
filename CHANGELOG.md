# Changelog

All notable changes to arch-toolkit will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-XX

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

#### Core Infrastructure
- **Unified Error Type**: `ArchToolkitError` enum with comprehensive error variants
  - Network errors (HTTP/network failures)
  - JSON parsing errors
  - Custom parsing errors
  - Rate limiting errors with retry-after information
  - Not found errors
  - Invalid input errors
- **HTTP Client with Rate Limiting**: Shared client with built-in rate limiting
  - Exponential backoff for archlinux.org requests (500ms base, max 60s)
  - Semaphore-based request serialization
  - Random jitter to prevent thundering herd
  - Automatic backoff reset on successful requests
- **Type System**: Comprehensive data types for AUR operations
  - `AurPackage`: Minimal package info for search results
  - `AurPackageDetails`: Full package details with all metadata
  - `AurComment`: Comment structure with author, date, content, pinned status
- **Utility Functions**: JSON parsing and URL encoding helpers
  - `percent_encode()`: RFC 3986-compliant URL encoding
  - `s()`, `ss()`, `arrs()`, `u64_of()`: JSON extraction helpers

#### Documentation
- Comprehensive rustdoc comments for all public APIs
  - What/Inputs/Output/Details format
  - Usage examples in documentation
- Crate-level documentation with examples for all AUR operations
- README with usage examples

#### Testing
- Unit tests for search and info parsing
- Test coverage for JSON parsing edge cases
- Integration test structure in place

### Technical Details
- **Async-first design**: All I/O operations use `tokio` async/await
- **Feature flags**: Modular design with `aur` feature flag
- **Zero dependencies by default**: Minimal core dependencies (serde, thiserror, tracing)
- **Optional dependencies**: HTTP client, HTML parsing, date handling only when `aur` feature is enabled
- **Strict code quality**: Clippy pedantic and nursery rules enabled
- **Complexity thresholds**: Cyclomatic and data flow complexity < 25

[0.1.0]: https://github.com/Firstp1ck/arch-toolkit/releases/tag/v0.1.0

