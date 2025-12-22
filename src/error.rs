//! Unified error type for arch-toolkit.

use thiserror::Error;

/// Unified error type for all arch-toolkit operations.
///
/// This error type covers all possible failure modes across different modules,
/// providing clear, actionable error messages.
#[derive(Error, Debug)]
pub enum ArchToolkitError {
    /// Network or HTTP request error.
    ///
    /// Note: For AUR operations, prefer using operation-specific error variants
    /// (`SearchFailed`, `InfoFailed`, `CommentsFailed`, `PkgbuildFailed`) to preserve context.
    /// This variant is retained for client initialization and non-AUR operations.
    #[error("Network error: {0}")]
    Network(reqwest::Error),

    /// AUR search operation failed.
    #[error("AUR search failed for query '{query}': {source}")]
    SearchFailed {
        /// The search query that failed.
        query: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// AUR info fetch operation failed.
    #[error("AUR info fetch failed for packages [{packages}]: {source}")]
    InfoFailed {
        /// Comma-separated list of package names that failed.
        packages: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// AUR comments fetch operation failed.
    #[error("AUR comments fetch failed for package '{package}': {source}")]
    CommentsFailed {
        /// The package name that failed.
        package: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// PKGBUILD fetch operation failed.
    #[error("PKGBUILD fetch failed for package '{package}': {source}")]
    PkgbuildFailed {
        /// The package name that failed.
        package: String,
        /// The underlying network error.
        #[source]
        source: reqwest::Error,
    },

    /// JSON parsing error.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Custom parsing error with message.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Rate limiting error with optional retry-after information.
    #[error("Rate limited by server{0}", .retry_after.map(|s| format!(" (retry after {s}s)")).unwrap_or_default())]
    RateLimited {
        /// Optional retry-after value in seconds from server.
        retry_after: Option<u64>,
    },

    /// Package not found (enhanced with package name).
    #[error("Package '{package}' not found")]
    PackageNotFound {
        /// The package name that was not found.
        package: String,
    },

    /// Invalid input parameter.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl ArchToolkitError {
    /// What: Create a `SearchFailed` error with query context.
    ///
    /// Inputs:
    /// - `query`: The search query that failed
    /// - `source`: The underlying network error
    ///
    /// Output:
    /// - `ArchToolkitError::SearchFailed` variant
    ///
    /// Details:
    /// - Convenience constructor for search operation errors
    /// - Preserves both the query and the underlying error
    #[must_use]
    pub fn search_failed(query: impl Into<String>, source: reqwest::Error) -> Self {
        Self::SearchFailed {
            query: query.into(),
            source,
        }
    }

    /// What: Create an `InfoFailed` error with package names context.
    ///
    /// Inputs:
    /// - `packages`: Slice of package names that failed
    /// - `source`: The underlying network error
    ///
    /// Output:
    /// - `ArchToolkitError::InfoFailed` variant
    ///
    /// Details:
    /// - Convenience constructor for info operation errors
    /// - Formats package names as comma-separated string
    /// - Preserves both the package names and the underlying error
    #[must_use]
    pub fn info_failed(packages: &[&str], source: reqwest::Error) -> Self {
        Self::InfoFailed {
            packages: packages.join(", "),
            source,
        }
    }

    /// What: Create a `CommentsFailed` error with package name context.
    ///
    /// Inputs:
    /// - `package`: The package name that failed
    /// - `source`: The underlying network error
    ///
    /// Output:
    /// - `ArchToolkitError::CommentsFailed` variant
    ///
    /// Details:
    /// - Convenience constructor for comments operation errors
    /// - Preserves both the package name and the underlying error
    #[must_use]
    pub fn comments_failed(package: impl Into<String>, source: reqwest::Error) -> Self {
        Self::CommentsFailed {
            package: package.into(),
            source,
        }
    }

    /// What: Create a `PkgbuildFailed` error with package name context.
    ///
    /// Inputs:
    /// - `package`: The package name that failed
    /// - `source`: The underlying network error
    ///
    /// Output:
    /// - `ArchToolkitError::PkgbuildFailed` variant
    ///
    /// Details:
    /// - Convenience constructor for pkgbuild operation errors
    /// - Preserves both the package name and the underlying error
    #[must_use]
    pub fn pkgbuild_failed(package: impl Into<String>, source: reqwest::Error) -> Self {
        Self::PkgbuildFailed {
            package: package.into(),
            source,
        }
    }
}

/// Result type alias for arch-toolkit operations.
pub type Result<T> = std::result::Result<T, ArchToolkitError>;
