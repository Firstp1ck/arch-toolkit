//! Complete Rust toolkit for Arch Linux package management.
//!
//! This crate provides a unified API for interacting with Arch Linux package management,
//! including AUR (Arch User Repository) operations, dependency resolution, package
//! index queries, installation command building, news feeds, and security advisories.
//!
//! # Features
//!
//! - `aur`: AUR search, package info, comments, and PKGBUILD fetching
//! - `deps`: Dependency resolution and SRCINFO parsing (planned)
//! - `index`: Package database queries (planned)
//! - `install`: Installation command building (planned)
//! - `news`: News feeds and security advisories (planned)
//! - `sandbox`: PKGBUILD security analysis (planned)
//!
//! # Examples
//!
//! ## Basic AUR Search
//!
//! ```no_run
//! use arch_toolkit::ArchClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::new()?;
//! let packages = client.aur().search("yay").await?;
//! println!("Found {} packages", packages.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Fetch Package Details
//!
//! ```no_run
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
//! ```
//!
//! ## Custom Configuration
//!
//! ```no_run
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
//! ```
//!
//! ## Fetch Comments
//!
//! ```no_run
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
//! ```
//!
//! ## Fetch PKGBUILD
//!
//! ```no_run
//! use arch_toolkit::ArchClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ArchClient::new()?;
//! let pkgbuild = client.aur().pkgbuild("yay").await?;
//! println!("PKGBUILD:\n{}", pkgbuild);
//! # Ok(())
//! # }
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

/// Prelude module for convenient imports.
///
/// This module re-exports commonly used types, traits, and functions,
/// allowing you to import everything you need with a single `use arch_toolkit::prelude::*;`.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::prelude::*;
///
/// # async fn example() -> Result<()> {
/// let client = ArchClient::new()?;
/// let packages: Vec<AurPackage> = client.aur().search("yay").await?;
/// Ok(())
/// # }
/// ```
pub mod prelude;

// Re-export commonly used types
pub use error::{ArchToolkitError as Error, Result};
pub use types::{AurComment, AurPackage, AurPackageDetails};

#[cfg(feature = "aur")]
pub use types::{HealthStatus, ServiceStatus};

#[cfg(feature = "aur")]
pub use aur::{AurApi, MockAurApi};

#[cfg(feature = "aur")]
pub use client::{ArchClient, ArchClientBuilder, CacheInvalidator, RetryPolicy};

#[cfg(feature = "aur")]
pub use cache::{CacheConfig, CacheConfigBuilder};

#[cfg(feature = "aur")]
pub use aur::validation::ValidationConfig;
