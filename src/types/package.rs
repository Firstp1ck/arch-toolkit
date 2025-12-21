//! Package-related data types for AUR operations.

use serde::{Deserialize, Serialize};

/// Basic AUR package information from search results.
///
/// This is a lightweight representation suitable for lists and search results.
/// For full package details, see [`AurPackageDetails`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AurPackage {
    /// Canonical package name.
    pub name: String,
    /// Version string as reported by AUR.
    pub version: String,
    /// One-line description suitable for list display.
    pub description: String,
    /// AUR popularity score when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popularity: Option<f64>,
    /// Timestamp when package was flagged out-of-date (Unix timestamp in seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_date: Option<u64>,
    /// Whether package is orphaned (no active maintainer).
    #[serde(default)]
    pub orphaned: bool,
    /// Package maintainer username (None if orphaned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintainer: Option<String>,
}

/// Full AUR package details from the info endpoint.
///
/// Contains comprehensive information about a package, including all dependencies,
/// metadata, and AUR-specific fields.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AurPackageDetails {
    /// Package name.
    pub name: String,
    /// Full version string.
    pub version: String,
    /// Long description.
    pub description: String,
    /// Upstream project URL (may be empty if unknown).
    pub url: String,
    /// SPDX or human-readable license identifiers.
    pub licenses: Vec<String>,
    /// Group memberships.
    pub groups: Vec<String>,
    /// Virtual provisions supplied by this package.
    pub provides: Vec<String>,
    /// Required dependencies.
    pub depends: Vec<String>,
    /// Build dependencies.
    pub make_depends: Vec<String>,
    /// Optional dependencies with annotations.
    pub opt_depends: Vec<String>,
    /// Conflicting packages.
    pub conflicts: Vec<String>,
    /// Packages that this package replaces.
    pub replaces: Vec<String>,
    /// Package maintainer username (None if orphaned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintainer: Option<String>,
    /// First submission timestamp (Unix timestamp in seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_submitted: Option<i64>,
    /// Last modification timestamp (Unix timestamp in seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<i64>,
    /// AUR popularity score when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popularity: Option<f64>,
    /// Number of votes on AUR.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_votes: Option<u64>,
    /// Timestamp when package was flagged out-of-date (Unix timestamp in seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_date: Option<u64>,
    /// Whether package is orphaned (no active maintainer).
    #[serde(default)]
    pub orphaned: bool,
}

/// AUR comment from a package page.
///
/// Contains author, date, and content of a comment, with optional timestamp
/// for reliable chronological sorting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AurComment {
    /// Stable comment identifier parsed from DOM when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Comment author username.
    pub author: String,
    /// Human-readable date string.
    pub date: String,
    /// Unix timestamp for sorting (None if parsing failed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_timestamp: Option<i64>,
    /// URL from the date link (None if not available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_url: Option<String>,
    /// Comment content text (formatted as markdown-like syntax).
    pub content: String,
    /// Whether this comment is pinned (shown at the top).
    #[serde(default)]
    pub pinned: bool,
}
