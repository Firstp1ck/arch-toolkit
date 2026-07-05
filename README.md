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

- **Dependency Management** (`deps` feature)
  - Parse dependencies from PKGBUILD files (single-line and multi-line arrays)
  - Parse dependencies from .SRCINFO files
  - Parse dependency specifications with version constraints
  - Parse pacman output for dependencies and conflicts
  - Fetch .SRCINFO from AUR (requires `aur` feature)
  - Dependency resolution for official, AUR, and local packages
  - Reverse dependency analysis for safe package removal
  - Version comparison using pacman-compatible algorithm
  - Package querying (installed, upgradable, versions)
  - Source determination (official, AUR, local)

- **Package Index Queries** (`index` feature)
  - Installed package queries (`pacman -Qq`) with optional caching
  - Explicit package tracking (all explicit or leaf-only packages)
  - Official repository search (substring and optional fuzzy matching)
  - Official index fetching (from `pacman -Sl` or the Arch Packages API)
  - Index persistence (save/load the official index as JSON)
  - Sync and async APIs (async via `tokio::spawn_blocking`)

- **Install Command Building** (`install` feature)
  - Pacman install/remove/update command construction (build, never execute)
  - AUR helper commands with paru/yay detection and preference
  - Privilege tool detection (sudo/doas) and command wrapping
  - Batch planning: split mixed target lists between pacman and an AUR helper
  - Removal cascade modes (`-R`, `-Rs`, `-Rns`)
  - Strict package-name validation and POSIX shell quoting

- **News & Security Advisories** (`news` feature)
  - Arch Linux news RSS fetching with date normalization
  - Security advisory Atom feed with severity and package extraction
  - Pure parse functions (testable offline against recorded feeds)
  - Cutoff-date filtering for incremental fetches

### Planned Features

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
- `index`: Package database queries (installed, explicit, official repositories) and index persistence
- `install`: Installation command building (pacman, AUR helpers, batch planning; enables `deps`)
- `news`: Arch news RSS and security advisories
- `fuzzy-search`: Fuzzy matching for official index search (used with `index`)
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

To enable package index queries:

```toml
arch-toolkit = { version = "0.2", features = ["index"] }
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

### Dependency Parsing and Resolution

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

### Dependency Resolution

Resolve dependencies for packages:

```rust
use arch_toolkit::deps::{DependencyResolver, PackageRef, PackageSource};

let resolver = DependencyResolver::new();
let packages = vec![
    PackageRef {
        name: "firefox".into(),
        version: "121.0".into(),
        source: PackageSource::Official {
            repo: "extra".into(),
            arch: "x86_64".into(),
        },
    },
];

let result = resolver.resolve(&packages)?;
println!("Found {} dependencies", result.dependencies.len());
for dep in result.dependencies {
    println!("  {}: {:?}", dep.name, dep.status);
}
```

### Reverse Dependency Analysis

Find all packages that depend on packages being removed:

```rust
use arch_toolkit::deps::{ReverseDependencyAnalyzer, PackageRef, PackageSource};

let analyzer = ReverseDependencyAnalyzer::new();
let packages = vec![
    PackageRef {
        name: "qt5-base".into(),
        version: "5.15.10".into(),
        source: PackageSource::Official {
            repo: "extra".into(),
            arch: "x86_64".into(),
        },
    },
];

let report = analyzer.analyze(&packages)?;
println!("{} packages would be affected", report.dependents.len());
```

### Version Comparison

Compare package versions:

```rust
use arch_toolkit::deps::{compare_versions, version_satisfies};

// Compare versions
use std::cmp::Ordering;
assert_eq!(compare_versions("1.2.3", "1.2.4"), Ordering::Less);

// Check if version satisfies requirement
assert!(version_satisfies("2.0", ">=1.5"));
assert!(!version_satisfies("1.0", ">=1.5"));
```

### Package Querying

Query installed and upgradable packages:

```rust
use arch_toolkit::deps::{
    get_installed_packages, get_upgradable_packages,
    get_installed_version, get_available_version,
};

// Get installed packages
let installed = get_installed_packages()?;
println!("Found {} installed packages", installed.len());

// Get upgradable packages
let upgradable = get_upgradable_packages()?;
println!("Found {} upgradable packages", upgradable.len());

// Get installed version
if let Ok(version) = get_installed_version("pacman") {
    println!("Installed pacman version: {}", version);
}

// Get available version
if let Some(version) = get_available_version("pacman") {
    println!("Available pacman version: {}", version);
}
```

### Source Determination

Determine where a package comes from:

```rust
use arch_toolkit::deps::{determine_dependency_source, is_system_package};
use std::collections::HashSet;

let installed = get_installed_packages()?;
let (source, is_core) = determine_dependency_source("glibc", &installed);
println!("Source: {:?}, Is core: {}", source, is_core);

if is_system_package("glibc") {
    println!("glibc is a critical system package");
}
```

### Package Index Queries

Query installed packages and search official repositories (requires `index` feature):

```rust
use arch_toolkit::index::{
    fetch_official_index, get_installed_packages, is_installed, load_from_disk, save_to_disk,
    search_official,
};
use std::path::Path;

// Query installed packages
let installed = get_installed_packages()?;
if is_installed("vim", Some(&installed)) {
    println!("vim is installed");
}

// Load a cached official index, falling back to a fresh fetch
let path = Path::new("official_index.json");
let index = load_from_disk(path).or_else(|_| fetch_official_index())?;

// Search the official index
for result in search_official(&index, "ripgrep", false) {
    println!("{}/{} {}", result.package.repo, result.package.name, result.package.version);
}

// Persist the index for the next session
save_to_disk(&index, path)?;
```

### Install Command Building

Build (never execute) pacman and AUR helper commands (requires `install` feature):

```rust
use arch_toolkit::install::{
    build_batch_install, build_remove_command, detect_aur_helper, detect_privilege_tool,
    with_privilege,
};
use arch_toolkit::types::install::{CascadeMode, InstallOptions};
use arch_toolkit::PackageRef;

// Plan a mixed batch: official packages via pacman, AUR packages via paru/yay
let targets = vec![
    PackageRef::official("ripgrep", "14.0.0", "extra", "x86_64"),
    PackageRef::aur("yay-bin", "12.0.0"),
];
let plan = build_batch_install(
    &targets,
    detect_aur_helper(),
    detect_privilege_tool(),
    &InstallOptions::default(),
    None::<&std::collections::HashSet<String>>,
)?;
for command in &plan.commands {
    println!("Would run: {command}");          // dry run = display, not execute
    // command.to_command().status()?;         // or actually run it (argv, no shell)
}

// Removal with cascade control
let remove = with_privilege(
    detect_privilege_tool().expect("sudo or doas required"),
    build_remove_command(&["old-package"], CascadeMode::CascadeWithConfigs, true)?,
);
println!("{remove}"); // sudo pacman -Rns --noconfirm old-package
```

### News and Security Advisories

Fetch Arch news and advisories (requires `news` feature):

```rust
use arch_toolkit::news::{fetch_arch_news, fetch_security_advisories};

let client = reqwest::Client::new();

// Latest news, dates normalized to YYYY-MM-DD
for item in fetch_arch_news(&client, 10, None).await? {
    println!("{} {}", item.date, item.title);
}

// Advisories since a date, with severity and affected packages
for advisory in fetch_security_advisories(&client, 20, Some("2026-01-01")).await? {
    println!("{} [{}] {:?}", advisory.date, advisory.severity, advisory.packages);
}
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
- `examples/deps_example.rs`: Comprehensive dependency module examples
- `examples/parse_example.rs`: Dependency specification parsing
- `examples/query_example.rs`: Package querying examples
- `examples/resolve_example.rs`: Dependency resolution examples
- `examples/reverse_example.rs`: Reverse dependency analysis examples
- `examples/source_example.rs`: Source determination examples
- `examples/version_example.rs`: Version comparison examples
- `examples/index_example.rs`: Package index queries and persistence examples
- `examples/install_example.rs`: Install command building and batch planning examples
- `examples/news_example.rs`: Arch news and security advisory examples

Run examples with:

```bash
cargo run --example aur_example
cargo run --example with_caching
cargo run --example env_config
cargo run --example health_check
cargo run --example pkgbuild_example --features deps
cargo run --example srcinfo_example --features deps
cargo run --example deps_example --features deps
cargo run --example parse_example --features deps
cargo run --example query_example --features deps
cargo run --example resolve_example --features deps
cargo run --example reverse_example --features deps
cargo run --example source_example --features deps
cargo run --example version_example --features deps
cargo run --example index_example --features index
cargo run --example install_example --features install
cargo run --example news_example --features news
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

