//! Index module for package database queries and management.
//!
//! This module provides functionality for querying and managing package database information:
//!
//! - **Installed Package Queries** - Query installed packages using `pacman -Q*` commands
//! - **Explicit Package Tracking** - Track explicitly installed packages with different modes
//!
//! # Features
//!
//! This module requires the `index` feature flag to be enabled:
//!
//! ```toml
//! [dependencies]
//! arch-toolkit = { version = "0.2", features = ["index"] }
//! ```
//!
//! # Examples
//!
//! ## Query Installed Packages
//!
//! ```no_run
//! use arch_toolkit::index::{get_installed_packages, is_installed};
//! use std::collections::HashSet;
//!
//! // Direct query without caching
//! let packages = get_installed_packages().unwrap();
//! println!("Found {} installed packages", packages.len());
//!
//! // Check if a package is installed
//! if is_installed("vim", Some(&packages)) {
//!     println!("vim is installed");
//! }
//! ```
//!
//! ## Use Cache for Multiple Queries
//!
//! ```no_run
//! use arch_toolkit::index::{refresh_installed_cache, is_installed};
//! use std::collections::HashSet;
//!
//! let mut cache = HashSet::new();
//! refresh_installed_cache(Some(&mut cache)).unwrap();
//!
//! // Now use cache for fast lookups
//! for package in ["vim", "git", "python"] {
//!     if is_installed(package, Some(&cache)) {
//!         println!("{} is installed", package);
//!     }
//! }
//! ```
//!
//! ## Async Cache Refresh
//!
//! ```no_run
//! use arch_toolkit::index::refresh_installed_cache_async;
//! use std::collections::HashSet;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut cache = HashSet::new();
//! let packages = refresh_installed_cache_async(Some(&mut cache)).await?;
//! println!("Refreshed cache with {} packages", packages.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Query Explicit Packages
//!
//! ```no_run
//! use arch_toolkit::index::{refresh_explicit_cache, is_explicit, InstalledPackagesMode};
//! use std::collections::HashSet;
//!
//! let mut cache = HashSet::new();
//! // Get all explicitly installed packages
//! refresh_explicit_cache(InstalledPackagesMode::AllExplicit, Some(&mut cache)).unwrap();
//!
//! // Check if a package is explicitly installed
//! if is_explicit("vim", InstalledPackagesMode::AllExplicit, Some(&cache)) {
//!     println!("vim is explicitly installed");
//! }
//!
//! // Get only leaf packages (not required by others)
//! let leaf_packages = refresh_explicit_cache(InstalledPackagesMode::LeafOnly, None).unwrap();
//! println!("Found {} leaf packages", leaf_packages.len());
//! ```

mod explicit;
mod installed;

// Re-export types from types module
pub use crate::types::index::InstalledPackagesMode;

// Re-export installed functions
pub use installed::{
    get_installed_packages, is_installed, refresh_installed_cache, refresh_installed_cache_async,
};

// Re-export explicit functions
pub use explicit::{is_explicit, refresh_explicit_cache, refresh_explicit_cache_async};
