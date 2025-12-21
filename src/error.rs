//! Unified error type for arch-toolkit.

use thiserror::Error;

/// Unified error type for all arch-toolkit operations.
///
/// This error type covers all possible failure modes across different modules,
/// providing clear, actionable error messages.
#[derive(Error, Debug)]
pub enum ArchToolkitError {
    /// Network or HTTP request error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

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

    /// Package not found.
    #[error("Package not found")]
    NotFound,

    /// Invalid input parameter.
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Result type alias for arch-toolkit operations.
pub type Result<T> = std::result::Result<T, ArchToolkitError>;
