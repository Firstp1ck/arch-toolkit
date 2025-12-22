# arch-toolkit

Complete Rust toolkit for Arch Linux package management. Provides a unified API for interacting with Arch Linux package management, including AUR (Arch User Repository) operations, dependency resolution, package index queries, installation command building, news feeds, and security advisories.

## Features

### Current Features

- **AUR Operations** (`aur` feature, enabled by default)
  - Package search via AUR RPC v5
  - Detailed package information retrieval
  - Package comments fetching and parsing
  - PKGBUILD content retrieval
  - Automatic rate limiting with exponential backoff
  - Configurable retry policies with per-operation control
  - Optional caching layer (memory and disk)

- **Dependency Parsing** (`deps` feature)
  - Parse dependencies from PKGBUILD files (single-line and multi-line arrays)
  - Parse dependencies from .SRCINFO files
  - Parse dependency specifications with version constraints
  - Parse pacman output for dependencies and conflicts
  - Fetch .SRCINFO from AUR (requires `aur` feature)

### Planned Features

- Package database queries
- Installation command building
- News feeds and security advisories
- PKGBUILD security analysis

## Installation

Add `arch-toolkit` to your `Cargo.toml`:

```toml
[dependencies]
arch-toolkit = "0.1.2"
```

### Feature Flags

- `aur` (default): AUR search, package info, comments, and PKGBUILD fetching
- `deps`: Dependency parsing from PKGBUILD, .SRCINFO, and pacman output
- `cache-disk`: Enable disk-based caching for persistence across restarts

To disable default features:

```toml
arch-toolkit = { version = "0.1.2", default-features = false, features = ["aur"] }
```

To enable dependency parsing:

```toml
arch-toolkit = { version = "0.1.2", features = ["deps"] }
```

To enable disk caching:

```toml
arch-toolkit = { version = "0.1.2", features = ["cache-disk"] }
```

## Quick Start

### Basic Usage

```rust
use arch_toolkit::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a client with default settings
    let client = ArchClient::new()?;
    
    // Search for packages
    let packages = client.aur().search("yay").await?;
    println!("Found {} packages", packages.len());
    
    // Get detailed package information
    let details = client.aur().info(&["yay", "paru"]).await?;
    for pkg in details {
        println!("{}: {}", pkg.name, pkg.description);
    }
    
    Ok(())
}
```

### Custom Configuration

```rust
use arch_toolkit::ArchClient;
use std::time::Duration;

let client = ArchClient::builder()
    .timeout(Duration::from_secs(60))
    .user_agent("my-app/1.0")
    .max_retries(5)
    .build()?;
```

Or configure via environment variables (perfect for CI/CD):

```bash
export ARCH_TOOLKIT_TIMEOUT=60
export ARCH_TOOLKIT_USER_AGENT="my-app/1.0"
export ARCH_TOOLKIT_MAX_RETRIES=5
```

```rust
let client = ArchClient::builder()
    .from_env()  // Load configuration from environment
    .build()?;
```

### Retry Policy Configuration

```rust
use arch_toolkit::ArchClient;
use arch_toolkit::client::RetryPolicy;

let retry_policy = RetryPolicy {
    max_retries: 5,
    initial_delay_ms: 2000,
    max_delay_ms: 60_000,
    jitter_max_ms: 1000,
    enabled: true,
    retry_search: true,
    retry_info: true,
    retry_comments: true,
    retry_pkgbuild: false,
};

let client = ArchClient::builder()
    .retry_policy(retry_policy)
    .build()?;
```

### Caching

Enable caching to reduce network requests:

```rust
use arch_toolkit::ArchClient;
use arch_toolkit::cache::CacheConfigBuilder;
use std::time::Duration;

let cache_config = CacheConfigBuilder::new()
    .enable_search(true)
    .search_ttl(Duration::from_secs(300)) // 5 minutes
    .enable_info(true)
    .info_ttl(Duration::from_secs(900)) // 15 minutes
    .enable_comments(true)
    .comments_ttl(Duration::from_secs(600)) // 10 minutes
    .memory_cache_size(200)
    .build();

let client = ArchClient::builder()
    .cache_config(cache_config)
    .build()?;
```

With disk caching (requires `cache-disk` feature):

```rust
let cache_config = CacheConfigBuilder::new()
    .enable_search(true)
    .search_ttl(Duration::from_secs(300))
    .enable_disk_cache(true) // Persist across restarts
    .build();
```

### Fetch Comments

```rust
let comments = client.aur().comments("yay").await?;
for comment in comments.iter().take(5) {
    println!("{}: {}", comment.author, comment.content);
}
```

### Fetch PKGBUILD

```rust
let pkgbuild = client.aur().pkgbuild("yay").await?;
println!("PKGBUILD:\n{}", pkgbuild);
```

### Parse Dependencies

Parse dependencies from PKGBUILD or .SRCINFO files:

```rust
use arch_toolkit::deps::{parse_pkgbuild_deps, parse_srcinfo_deps};

// Parse PKGBUILD
let pkgbuild = r"depends=('glibc' 'python>=3.10')";
let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

// Parse .SRCINFO
let srcinfo = r"depends = glibc\ndepends = python>=3.10";
let (depends, makedepends, checkdepends, optdepends) = parse_srcinfo_deps(srcinfo);
```

### Health Checks

Monitor AUR service status:

```rust
// Quick health check
let is_healthy = client.health_check().await?;

// Detailed status with latency
let status = client.health_status().await?;
println!("Status: {:?}, Latency: {:?}", status.status, status.latency);
```

## Examples

See the `examples/` directory for comprehensive examples:

- `examples/aur_example.rs`: Complete AUR operations demonstration
- `examples/with_caching.rs`: Caching layer usage
- `examples/env_config.rs`: Environment variable configuration
- `examples/health_check.rs`: Health check functionality
- `examples/pkgbuild_example.rs`: PKGBUILD dependency parsing
- `examples/srcinfo_example.rs`: .SRCINFO parsing and fetching

Run examples with:

```bash
cargo run --example aur_example
cargo run --example with_caching
cargo run --example env_config
cargo run --example health_check
cargo run --example pkgbuild_example --features deps
cargo run --example srcinfo_example --features deps
```

## API Documentation

Full API documentation is available at [docs.rs/arch-toolkit](https://docs.rs/arch-toolkit) or build locally:

```bash
cargo doc --open
```

## Rate Limiting

arch-toolkit automatically implements rate limiting for archlinux.org requests:

- Minimum 200ms delay between requests
- Exponential backoff on failures
- Serialized requests (one at a time) to prevent overwhelming the server
- Configurable retry policies

## Error Handling

All operations return `Result<T, ArchToolkitError>`. Common error types:

- `ArchToolkitError::Network`: HTTP request failures
- `ArchToolkitError::Parse`: JSON/HTML parsing errors
- `ArchToolkitError::InvalidInput`: Invalid parameters or URLs
- `ArchToolkitError::Timeout`: Request timeout
- `ArchToolkitError::EmptyInput`: Empty input provided (with input validation)
- `ArchToolkitError::InvalidPackageName`: Invalid package name format

Input validation is enabled by default and validates package names and search queries against Arch Linux standards.

## Requirements

- Rust 1.70 or later
- Tokio runtime (for async operations)

## License

MIT

## Repository

https://github.com/Firstp1ck/arch-toolkit

