//! Complete Rust toolkit for Arch Linux package management.
//!
//! This crate provides a unified API for interacting with Arch Linux package management,
//! including AUR (Arch User Repository) operations, dependency resolution, package
//! index queries, installation command building, news feeds, and security advisories.
//!
//! # Features
//!
//! - `aur`: AUR search, package info, comments, and PKGBUILD fetching
//! - `deps`: Dependency resolution, parsing, and reverse dependency analysis
//! - `index`: Package database queries (installed and explicit package tracking)
//! - `install`: Installation command building (pacman, AUR helpers, batch planning)
//! - `news`: Arch news RSS and security advisories
//! - `sandbox`: Build-preflight dependency analysis (PKGBUILD/.SRCINFO vs host)
//!
//! # Examples
//!
//! ## Basic AUR Search
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::ArchClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::new()?;
//! let packages = client.aur().search("yay").await?;
//! println!("Found {} packages", packages.len());
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Fetch Package Details
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::ArchClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::new()?;
//! let details = client.aur().info(&["yay", "paru"]).await?;
//! for pkg in details {
//!     println!("{}: {}", pkg.name, pkg.description);
//! }
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Custom Configuration
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::ArchClient;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::builder()
//!     .timeout(Duration::from_secs(60))
//!     .user_agent("my-app/1.0")
//!     .build()?;
//! let packages = client.aur().search("yay").await?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Fetch Comments
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::ArchClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::new()?;
//! let comments = client.aur().comments("yay").await?;
//! for comment in comments.iter().take(5) {
//!     println!("{}: {}", comment.author, comment.content);
//! }
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Fetch PKGBUILD
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::ArchClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::new()?;
//! let pkgbuild = client.aur().pkgbuild("yay").await?;
//! println!("PKGBUILD:\n{}", pkgbuild);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Dependency Resolution
//!
//! ```ignore
//! use arch_toolkit::deps::DependencyResolver;
//! use arch_toolkit::{PackageRef, PackageSource};
//!
//! let resolver = DependencyResolver::new();
//! let packages = vec![
//!     PackageRef {
//!         name: "firefox".into(),
//!         version: "121.0".into(),
//!         source: PackageSource::Official {
//!             repo: "extra".into(),
//!             arch: "x86_64".into(),
//!         },
//!     },
//! ];
//!
//! let result = resolver.resolve(&packages).unwrap();
//! println!("Found {} dependencies", result.dependencies.len());
//! ```
//!
//! ## Parse Dependency Specifications
//!
//! ```ignore
//! use arch_toolkit::deps::parse_dep_spec;
//!
//! let spec = parse_dep_spec("python>=3.12");
//! println!("Package: {}, Version: {}", spec.name, spec.version_req);
//! ```
//!
//! ## Query Installed Packages
//!
//! ```ignore
//! use arch_toolkit::deps::get_installed_packages;
//!
//! let installed = get_installed_packages().unwrap();
//! println!("Found {} installed packages", installed.len());
//! ```

pub mod error;
pub mod types;

#[cfg(feature = "aur")]
pub mod aur;

#[cfg(feature = "aur")]
pub mod client;

#[cfg(feature = "aur")]
pub mod cache;

#[cfg(feature = "aur")]
pub mod health;

#[cfg(feature = "aur")]
mod env;

#[cfg(feature = "aur")]
mod http;

#[cfg(feature = "deps")]
pub mod deps;

#[cfg(feature = "index")]
pub mod index;

#[cfg(feature = "install")]
pub mod install;

#[cfg(feature = "news")]
pub mod news;

#[cfg(feature = "sandbox")]
pub mod sandbox;

/// Prelude module for convenient imports.
///
/// This module re-exports commonly used types, traits, and functions,
/// allowing you to import everything you need with a single `use arch_toolkit::prelude::*;`.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "aur")] mod wrap {
/// use arch_toolkit::prelude::*;
///
/// # async fn example() -> Result<()> {
/// let client = ArchClient::new()?;
/// let packages: Vec<AurPackage> = client.aur().search("yay").await?;
/// Ok(())
/// # }
/// # }
/// ```
pub mod prelude;

// Re-export commonly used types
pub use error::{ArchToolkitError as Error, Result};
pub use types::{AurComment, AurPackage, AurPackageDetails};

#[cfg(feature = "aur")]
pub use types::{HealthStatus, ServiceStatus};

#[cfg(feature = "deps")]
pub use types::{
    Dependency, DependencySource, DependencySpec, DependencyStatus, PackageRef, PackageSource,
    ReverseDependencySummary, SrcinfoData,
};

#[cfg(feature = "deps")]
pub use types::dependency::{
    DependencyConstraintRange, DependencyGraphConfig, DependencyGraphDiagnostic,
    DependencyGraphDiagnosticKind, DependencyGraphEdge, DependencyGraphNode,
    DependencyGraphNodeStatus, DependencyGraphResolution, DependencyMetadata,
    DependencyMetadataResponse, DependencyProvenance, DependencyVersionBound,
};

#[cfg(feature = "index")]
pub use types::index::{
    IndexQueryResult, InstalledPackagesMode, MirrorDiscoveryLimits, MirrorInfo, OfficialIndex,
    OfficialPackage,
};

#[cfg(feature = "install")]
pub use types::install::{AurHelper, CascadeMode, CommandSpec, InstallOptions, PrivilegeTool};

#[cfg(feature = "news")]
pub use types::news::{AdvisorySeverity, ArchNewsItem, SecurityAdvisory};

#[cfg(feature = "news")]
pub use news::{FeedCache, InMemoryFeedCache};

#[cfg(feature = "sandbox")]
pub use types::sandbox::{
    DependencyDelta, SandboxAnalysisLimitation, SandboxFinding, SandboxInfo, SandboxRuleId,
    SandboxStaticAnalysis,
};

#[cfg(all(feature = "aur", feature = "index"))]
pub use aur::{
    ARCH_PACKAGE_SEARCH_URL, check_mirror_health, fetch_arch_package_detail,
    fetch_official_package_detail_from,
};

#[cfg(all(feature = "aur", feature = "index"))]
pub use types::package::{
    MetadataFetchLimits, MirrorHealth, MirrorHealthLimits, MirrorHealthStatus,
};

#[cfg(feature = "deps")]
pub use deps::{
    DependencyMetadataProvider, DependencyResolution, DependencyResolver, ResolverConfig,
    ReverseDependencyAnalyzer, ReverseDependencyReport,
};

#[cfg(feature = "aur")]
pub use aur::{AurApi, MockAurApi};

#[cfg(feature = "aur")]
pub use client::{ArchClient, ArchClientBuilder, CacheInvalidator, RetryPolicy};

#[cfg(feature = "aur")]
pub use cache::{CacheConfig, CacheConfigBuilder};

#[cfg(feature = "aur")]
pub use aur::validation::ValidationConfig;
