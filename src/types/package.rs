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

/// What: Bound one official-package metadata response and candidate scan.
///
/// Inputs:
/// - Constructed by callers before fetching a package detail response.
///
/// Output:
/// - Maximum bytes read and result candidates considered for one request.
///
/// Details:
/// - The response bound is enforced before JSON parsing.
/// - The candidate bound prevents a broad endpoint response from expanding a
///   single-package detail lookup into unbounded parsing work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataFetchLimits {
    /// Maximum response-body bytes accepted before JSON parsing.
    pub max_response_bytes: usize,
    /// Maximum API result rows considered while selecting an exact package.
    pub max_candidates: usize,
}

impl Default for MetadataFetchLimits {
    /// What: Provide conservative bounds for an official package detail request.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - A 512 KiB response bound and 16 candidate rows.
    ///
    /// Details:
    /// - Callers can choose tighter or explicitly reviewed larger limits.
    fn default() -> Self {
        Self {
            max_response_bytes: 512 * 1024,
            max_candidates: 16,
        }
    }
}

/// What: Bound sequential mirror health probes made by one caller.
///
/// Inputs:
/// - Constructed by callers before checking mirror probe URLs.
///
/// Output:
/// - Maximum mirror rows probed in input order.
///
/// Details:
/// - Transport timeout, proxy, TLS, redirect, and retry policy remain owned by
///   the caller-provided reqwest client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorHealthLimits {
    /// Maximum input mirror rows probed sequentially.
    pub max_mirrors: usize,
}

impl Default for MirrorHealthLimits {
    /// What: Provide a conservative default mirror-health probe bound.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - A maximum of 16 sequential probes.
    ///
    /// Details:
    /// - Sequential checks avoid an implicit concurrent request burst.
    fn default() -> Self {
        Self { max_mirrors: 16 }
    }
}

/// What: Represent the reachability classification of one mirror probe.
///
/// Inputs:
/// - Produced from a single caller-selected HTTP(S) probe response or error.
///
/// Output:
/// - A stable status without a latency ranking or aggregate score.
///
/// Details:
/// - A success status means the final response status was 2xx under the
///   caller's reqwest redirect policy; it is not a broad performance claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirrorHealthStatus {
    /// The probe completed with a successful 2xx HTTP status.
    Reachable,
    /// The probe returned a non-success status or transport error.
    Unreachable,
    /// The source mirror URL or caller probe path was invalid for this request.
    Invalid,
}

/// What: Record bounded evidence from one mirror health probe.
///
/// Inputs:
/// - Produced for each selected `MirrorInfo` row in input order.
///
/// Output:
/// - Mirror URL, stable classification, optional final status code, and an
///   actionable error detail when no successful response was received.
///
/// Details:
/// - This is deliberately not a performance ranking and never changes mirror
///   configuration or executes a system command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorHealth {
    /// Source mirror URL that was selected for the probe.
    pub mirror_url: String,
    /// Stable reachability classification for this one probe.
    pub status: MirrorHealthStatus,
    /// Final HTTP status code when a response was received.
    pub status_code: Option<u16>,
    /// Bounded transport or validation detail when the probe was not reachable.
    pub detail: Option<String>,
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
