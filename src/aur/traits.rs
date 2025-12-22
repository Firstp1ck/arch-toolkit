//! Traits for AUR operations, enabling testability via mock implementations.

use crate::error::Result;
use crate::types::{AurComment, AurPackage, AurPackageDetails};
use async_trait::async_trait;

/// What: Trait for AUR operations, enabling testability via mock implementations.
///
/// Inputs: None (trait definition)
///
/// Output: Trait that defines the interface for AUR operations
///
/// Details:
/// - Defines the core AUR operations: search, info, comments, and pkgbuild
/// - Allows users to create mock implementations for unit testing
/// - The `Aur<'a>` struct implements this trait for real AUR operations
/// - Mock implementations can be used to test code without hitting real APIs
#[async_trait]
pub trait AurApi: Send + Sync {
    /// What: Search for packages in the AUR by name.
    ///
    /// Inputs:
    /// - `query`: Search query string
    ///
    /// Output:
    /// - `Result<Vec<AurPackage>>` containing search results, or an error
    ///
    /// Details:
    /// - Searches the AUR for packages matching the query
    /// - Returns empty vector if no results found (not an error)
    async fn search(&self, query: &str) -> Result<Vec<AurPackage>>;

    /// What: Fetch detailed information for one or more AUR packages.
    ///
    /// Inputs:
    /// - `names`: Slice of package names to fetch info for
    ///
    /// Output:
    /// - `Result<Vec<AurPackageDetails>>` containing package details, or an error
    ///
    /// Details:
    /// - Fetches comprehensive information for the specified packages
    /// - Returns empty vector if no packages found (not an error)
    async fn info(&self, names: &[&str]) -> Result<Vec<AurPackageDetails>>;

    /// What: Fetch AUR package comments.
    ///
    /// Inputs:
    /// - `pkgname`: Package name to fetch comments for
    ///
    /// Output:
    /// - `Result<Vec<AurComment>>` with parsed comments, or an error
    ///
    /// Details:
    /// - Fetches comments from the AUR package page
    /// - Comments are sorted by date (latest first)
    async fn comments(&self, pkgname: &str) -> Result<Vec<AurComment>>;

    /// What: Fetch PKGBUILD content for an AUR package.
    ///
    /// Inputs:
    /// - `package`: Package name to fetch PKGBUILD for
    ///
    /// Output:
    /// - `Result<String>` with PKGBUILD text, or an error
    ///
    /// Details:
    /// - Fetches the raw PKGBUILD content for the specified package
    async fn pkgbuild(&self, package: &str) -> Result<String>;
}
