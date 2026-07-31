//! Prelude module for convenient imports.
//!
//! This module re-exports commonly used types, traits, and functions from arch-toolkit,
//! allowing you to import everything you need with a single `use arch_toolkit::prelude::*;`.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let client = ArchClient::new()?;
//! let packages: Vec<AurPackage> = client.aur().search("yay").await?;
//! Ok(())
//! # }
//! # }
//! ```
//!
//! ## With Custom Configuration
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::prelude::*;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<()> {
//! let client = ArchClient::builder()
//!     .timeout(Duration::from_secs(60))
//!     .user_agent("my-app/1.0")
//!     .build()?;
//! let packages = client.aur().search("yay").await?;
//! Ok(())
//! # }
//! # }
//! ```
//!
//! ## Using Mock for Testing
//!
//! ```no_run
//! # #[cfg(feature = "aur")] mod wrap {
//! use arch_toolkit::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let mock = MockAurApi::new()
//!     .with_search_result("yay", Ok(vec![]));
//! let packages = mock.search("yay").await?;
//! Ok(())
//! # }
//! # }
//! ```
//!
//! ## Dependency Resolution
//!
//! ```ignore
//! use arch_toolkit::prelude::*;
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

// Core client types
#[cfg(feature = "aur")]
pub use crate::client::{ArchClient, ArchClientBuilder};

// Data types
pub use crate::types::{AurComment, AurPackage, AurPackageDetails};

// Error handling
pub use crate::error::{ArchToolkitError as Error, Result};

// Traits
#[cfg(feature = "aur")]
pub use crate::aur::AurApi;

// Testing
#[cfg(feature = "aur")]
pub use crate::aur::MockAurApi;

// Configuration types
#[cfg(feature = "aur")]
pub use crate::cache::{CacheConfig, CacheConfigBuilder};

#[cfg(feature = "aur")]
pub use crate::aur::validation::ValidationConfig;

#[cfg(feature = "aur")]
pub use crate::client::RetryPolicy;

#[cfg(feature = "aur")]
pub use crate::client::CacheInvalidator;

// Health types
#[cfg(feature = "aur")]
pub use crate::types::{HealthStatus, ServiceStatus};

// Dependency types and functions
#[cfg(feature = "deps")]
pub use crate::types::{
    Dependency, DependencySource, DependencySpec, DependencyStatus, PackageRef, PackageSource,
    ReverseDependencySummary, SrcinfoData,
};

#[cfg(feature = "deps")]
pub use crate::types::dependency::{
    DependencyConstraintRange, DependencyGraphConfig, DependencyGraphDiagnostic,
    DependencyGraphDiagnosticKind, DependencyGraphEdge, DependencyGraphNode,
    DependencyGraphNodeStatus, DependencyGraphResolution, DependencyMetadata,
    DependencyMetadataResponse, DependencyProvenance, DependencyVersionBound,
};

#[cfg(feature = "deps")]
pub use crate::deps::{
    DependencyMetadataProvider, DependencyResolution, DependencyResolver, ResolverConfig,
    ReverseDependencyAnalyzer, ReverseDependencyReport, get_installed_packages, parse_dep_spec,
    version_satisfies,
};

// Index types and functions
#[cfg(feature = "index")]
pub use crate::types::index::{
    IndexQueryResult, InstalledPackagesMode, MirrorDiscoveryLimits, MirrorInfo, OfficialIndex,
    OfficialPackage,
};

#[cfg(feature = "index")]
pub use crate::index::{
    IndexRefreshHandle, detect_enabled_repos, fetch_arch_mirrors, fetch_mirrors_from,
    fetch_official_index, fetch_official_index_for_repos, generate_mirrorlist, load_from_disk,
    load_from_disk_or_default, save_to_disk, search_official, spawn_index_refresh,
};

// Install types and functions
#[cfg(feature = "install")]
pub use crate::types::install::{
    AurHelper, CascadeMode, CommandSpec, InstallOptions, PrivilegeTool,
};

#[cfg(feature = "install")]
pub use crate::install::{
    InstallPlan, aur_install_shell_fallback, aur_update_shell_fallback, build_aur_install,
    build_aur_update_command, build_batch_install, build_force_sync_update_command,
    build_pacman_install, build_remove_command, build_update_command, detect_aur_helper,
    detect_privilege_tool, with_privilege,
};

// News types and functions
#[cfg(feature = "news")]
pub use crate::types::news::{AdvisorySeverity, ArchNewsItem, SecurityAdvisory};

#[cfg(feature = "news")]
pub use crate::news::{
    FeedCache, InMemoryFeedCache, extract_article_text, fetch_arch_news, fetch_arch_news_cached,
    fetch_article_text, fetch_security_advisories, fetch_security_advisories_cached,
};

// Sandbox types and functions
#[cfg(feature = "sandbox")]
pub use crate::types::sandbox::{
    DependencyDelta, SandboxAnalysisLimitation, SandboxFinding, SandboxInfo, SandboxRuleId,
    SandboxStaticAnalysis,
};

#[cfg(feature = "sandbox")]
pub use crate::sandbox::{analyze_pkgbuild, analyze_pkgbuild_security, analyze_srcinfo};

// Official metadata and mirror-health capability types/functions
#[cfg(all(feature = "aur", feature = "index"))]
pub use crate::aur::{
    ARCH_PACKAGE_SEARCH_URL, check_mirror_health, fetch_arch_package_detail,
    fetch_official_package_detail_from,
};

#[cfg(all(feature = "aur", feature = "index"))]
pub use crate::types::package::{
    MetadataFetchLimits, MirrorHealth, MirrorHealthLimits, MirrorHealthStatus,
};
