//! AUR (Arch User Repository) operations.

#[cfg(feature = "aur")]
mod comments;
#[cfg(feature = "aur")]
mod info;
#[cfg(feature = "aur")]
mod mock;
#[cfg(feature = "aur")]
mod pkgbuild;
#[cfg(feature = "aur")]
mod search;
#[cfg(feature = "aur")]
mod traits;
#[cfg(feature = "aur")]
pub mod utils;
#[cfg(feature = "aur")]
pub mod validation;

#[cfg(feature = "aur")]
use crate::client::ArchClient;
#[cfg(feature = "aur")]
use crate::error::Result;
#[cfg(feature = "aur")]
use crate::types::{AurComment, AurPackage, AurPackageDetails};

#[cfg(feature = "aur")]
pub use mock::MockAurApi;
#[cfg(feature = "aur")]
pub use traits::AurApi;

/// What: Wrapper for AUR operations using an `ArchClient`.
///
/// Inputs: None (created via `ArchClient::aur()`)
///
/// Output: `Aur` instance that provides AUR operation methods
///
/// Details:
/// - Holds a reference to `ArchClient` to access HTTP client and configuration
/// - Provides methods: `search()`, `info()`, `comments()`, `pkgbuild()`
/// - All operations use the client's configured timeout and user agent
/// - Rate limiting is handled automatically
#[cfg(feature = "aur")]
#[derive(Debug)]
pub struct Aur<'a> {
    /// Reference to the parent `ArchClient`.
    client: &'a ArchClient,
}

#[cfg(feature = "aur")]
impl<'a> Aur<'a> {
    /// What: Create a new `Aur` wrapper for the given client.
    ///
    /// Inputs:
    /// - `client`: Reference to `ArchClient` to use for operations
    ///
    /// Output:
    /// - `Aur` wrapper instance
    ///
    /// Details:
    /// - Internal constructor, typically called via `ArchClient::aur()`
    /// - The wrapper uses the client's HTTP client and configuration
    pub(crate) const fn new(client: &'a ArchClient) -> Self {
        Self { client }
    }

    /// What: Search for packages in the AUR by name.
    ///
    /// Inputs:
    /// - `query`: Search query string.
    ///
    /// Output:
    /// - `Result<Vec<AurPackage>>` containing search results, or an error.
    ///
    /// Details:
    /// - Uses AUR RPC v5 search endpoint.
    /// - Limits results to 200 packages (AUR default).
    /// - Percent-encodes the query string for URL safety.
    /// - Applies rate limiting for archlinux.org requests.
    /// - Returns empty vector if no results found (not an error).
    ///
    /// # Errors
    /// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
    /// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
    pub async fn search(&self, query: &str) -> Result<Vec<AurPackage>> {
        search::search(self.client, query).await
    }

    /// What: Fetch detailed information for one or more AUR packages.
    ///
    /// Inputs:
    /// - `names`: Slice of package names to fetch info for.
    ///
    /// Output:
    /// - `Result<Vec<AurPackageDetails>>` containing package details, or an error.
    ///
    /// Details:
    /// - Uses AUR RPC v5 info endpoint.
    /// - Fetches info for all packages in a single request (more efficient).
    /// - Returns empty vector if no packages found (not an error).
    /// - Applies rate limiting for archlinux.org requests.
    ///
    /// # Errors
    /// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
    /// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
    pub async fn info(&self, names: &[&str]) -> Result<Vec<AurPackageDetails>> {
        info::info(self.client, names).await
    }

    /// What: Fetch AUR package comments by scraping the AUR package page.
    ///
    /// Inputs:
    /// - `pkgname`: Package name to fetch comments for.
    ///
    /// Output:
    /// - `Result<Vec<AurComment>>` with parsed comments sorted by date (latest first); `Err` on failure.
    ///
    /// Details:
    /// - Fetches HTML from `https://aur.archlinux.org/packages/<pkgname>`
    /// - Uses `scraper` to parse HTML and extract comment elements
    /// - Parses dates to Unix timestamps for sorting
    /// - Sorts comments by date descending (latest first)
    /// - Handles pinned comments (appear before "Latest Comments" heading)
    ///
    /// # Errors
    /// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
    /// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
    /// - Returns `Err(ArchToolkitError::Parse)` if HTML parsing fails
    pub async fn comments(&self, pkgname: &str) -> Result<Vec<AurComment>> {
        comments::comments(self.client, pkgname).await
    }

    /// What: Fetch PKGBUILD content for an AUR package.
    ///
    /// Inputs:
    /// - `package`: Package name to fetch PKGBUILD for.
    ///
    /// Output:
    /// - `Result<String>` with PKGBUILD text when available; `Err` on network or lookup failure.
    ///
    /// Details:
    /// - Fetches from `https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={package}`
    /// - Applies rate limiting (200ms minimum interval between requests)
    /// - Uses timeout from client configuration
    /// - Returns raw PKGBUILD text
    ///
    /// # Errors
    /// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
    /// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
    /// - Returns `Err(ArchToolkitError::Parse)` if rate limiter mutex is poisoned
    pub async fn pkgbuild(&self, package: &str) -> Result<String> {
        pkgbuild::pkgbuild(self.client, package).await
    }
}

#[cfg(feature = "aur")]
use async_trait::async_trait;

#[cfg(feature = "aur")]
#[async_trait]
impl AurApi for Aur<'_> {
    /// What: Search for packages in the AUR by name.
    ///
    /// Inputs:
    /// - `query`: Search query string
    ///
    /// Output:
    /// - `Result<Vec<AurPackage>>` containing search results, or an error
    ///
    /// Details:
    /// - Delegates to the underlying search module function
    async fn search(&self, query: &str) -> Result<Vec<AurPackage>> {
        search::search(self.client, query).await
    }

    /// What: Fetch detailed information for one or more AUR packages.
    ///
    /// Inputs:
    /// - `names`: Slice of package names to fetch info for
    ///
    /// Output:
    /// - `Result<Vec<AurPackageDetails>>` containing package details, or an error
    ///
    /// Details:
    /// - Delegates to the underlying info module function
    async fn info(&self, names: &[&str]) -> Result<Vec<AurPackageDetails>> {
        info::info(self.client, names).await
    }

    /// What: Fetch AUR package comments.
    ///
    /// Inputs:
    /// - `pkgname`: Package name to fetch comments for
    ///
    /// Output:
    /// - `Result<Vec<AurComment>>` with parsed comments, or an error
    ///
    /// Details:
    /// - Delegates to the underlying comments module function
    async fn comments(&self, pkgname: &str) -> Result<Vec<AurComment>> {
        comments::comments(self.client, pkgname).await
    }

    /// What: Fetch PKGBUILD content for an AUR package.
    ///
    /// Inputs:
    /// - `package`: Package name to fetch PKGBUILD for
    ///
    /// Output:
    /// - `Result<String>` with PKGBUILD text, or an error
    ///
    /// Details:
    /// - Delegates to the underlying pkgbuild module function
    async fn pkgbuild(&self, package: &str) -> Result<String> {
        pkgbuild::pkgbuild(self.client, package).await
    }
}
