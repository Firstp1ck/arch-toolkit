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
//! use arch_toolkit::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let client = ArchClient::new()?;
//! let packages: Vec<AurPackage> = client.aur().search("yay").await?;
//! Ok(())
//! # }
//! ```
//!
//! ## With Custom Configuration
//!
//! ```no_run
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
//! ```
//!
//! ## Using Mock for Testing
//!
//! ```no_run
//! use arch_toolkit::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let mock = MockAurApi::new()
//!     .with_search_result("yay", Ok(vec![]));
//! let packages = mock.search("yay").await?;
//! Ok(())
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
pub use crate::deps::{
    DependencyResolution, DependencyResolver, ResolverConfig, ReverseDependencyAnalyzer,
    ReverseDependencyReport, get_installed_packages, parse_dep_spec, version_satisfies,
};
