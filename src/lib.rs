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
//! use arch_toolkit::aur;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let packages = aur::search(&client, "yay").await?;
//! println!("Found {} packages", packages.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Fetch Package Details
//!
//! ```no_run
//! use arch_toolkit::aur;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let details = aur::info(&client, &["yay", "paru"]).await?;
//! for pkg in details {
//!     println!("{}: {}", pkg.name, pkg.description);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Fetch Comments
//!
//! ```no_run
//! use arch_toolkit::aur;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let comments = aur::comments(&client, "yay").await?;
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
//! use arch_toolkit::aur;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let pkgbuild = aur::pkgbuild(&client, "yay").await?;
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

// Re-export commonly used types
pub use error::{ArchToolkitError as Error, Result};
pub use types::{AurComment, AurPackage, AurPackageDetails};
