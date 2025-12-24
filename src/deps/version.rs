//! Version comparison utilities for dependency resolution.
//!
//! This module provides functions to compare version strings in a way that
//! matches pacman's version comparison algorithm, supporting dependency
//! requirement checking and version analysis.

use std::cmp::Ordering;

/// What: Normalize a version string by stripping the pkgrel suffix.
///
/// Inputs:
/// - `version`: Version string that may include pkgrel (e.g., "1.2.3-1").
///
/// Output:
/// - Returns a normalized version string with pkgrel removed.
///
/// Details:
/// - Strips everything after the last `-` if it's followed by digits only.
/// - Preserves text suffixes (e.g., "1.2.3-alpha" remains unchanged).
/// - Used internally for consistent version comparisons.
fn normalize_version(version: &str) -> String {
    // Find the last dash
    if let Some(last_dash) = version.rfind('-') {
        // Check if everything after the dash is numeric
        let suffix = &version[last_dash + 1..];
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            // It's a pkgrel, strip it
            return version[..last_dash].to_string();
        }
    }
    version.to_string()
}

/// What: Compare two version strings using pacman-compatible algorithm.
///
/// Inputs:
/// - `a`: Left-hand version string.
/// - `b`: Right-hand version string.
///
/// Output:
/// - Returns `Ordering::Less` if `a < b`.
/// - Returns `Ordering::Equal` if `a == b`.
/// - Returns `Ordering::Greater` if `a > b`.
///
/// Details:
/// - Splits versions on `.` and `-` into segments.
/// - Compares segments pairwise:
///   - If both are numeric: compares as numbers.
///   - If one is numeric and one is text: numeric < text (pacman behavior).
///   - If both are text: lexicographic comparison.
/// - Missing segments are treated as "0".
/// - First non-equal segment determines the result.
/// - This algorithm matches pacman's `alpm_version_cmp` behavior.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::compare_versions;
/// use std::cmp::Ordering;
///
/// assert_eq!(compare_versions("1.2.3", "1.2.4"), Ordering::Less);
/// assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
/// assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
/// assert_eq!(compare_versions("1.2.3alpha", "1.2.3beta"), Ordering::Less);
/// assert_eq!(compare_versions("1.2.3", "1.2.3alpha"), Ordering::Greater);
/// ```
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let a_normalized = normalize_version(a);
    let b_normalized = normalize_version(b);

    let a_parts: Vec<&str> = a_normalized.split(['.', '-']).collect();
    let b_parts: Vec<&str> = b_normalized.split(['.', '-']).collect();
    let len = a_parts.len().max(b_parts.len());

    for idx in 0..len {
        let a_seg = a_parts.get(idx).copied().unwrap_or("0");
        let b_seg = b_parts.get(idx).copied().unwrap_or("0");

        // Extract numeric prefix from each segment
        let a_num_end = a_seg
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit())
            .map_or(a_seg.len(), |(i, _)| i);
        let b_num_end = b_seg
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit())
            .map_or(b_seg.len(), |(i, _)| i);

        let a_num_str = &a_seg[..a_num_end];
        let b_num_str = &b_seg[..b_num_end];
        let a_suffix = &a_seg[a_num_end..];
        let b_suffix = &b_seg[b_num_end..];

        // Try to parse numeric prefixes
        match (a_num_str.parse::<i64>(), b_num_str.parse::<i64>()) {
            (Ok(a_num), Ok(b_num)) => {
                // Both have numeric prefixes, compare numbers first
                match a_num.cmp(&b_num) {
                    Ordering::Equal => {
                        // Numeric prefixes equal, compare suffixes
                        // Empty suffix > non-empty suffix (pacman: "3" > "3alpha")
                        match (a_suffix.is_empty(), b_suffix.is_empty()) {
                            (true, true) => {}                         // Both empty, continue
                            (true, false) => return Ordering::Greater, // "3" > "3alpha"
                            (false, true) => return Ordering::Less,    // "3alpha" < "3"
                            (false, false) => {
                                // Both have suffixes, compare lexicographically
                                match a_suffix.cmp(b_suffix) {
                                    Ordering::Equal => {}
                                    ord => return ord,
                                }
                            }
                        }
                    }
                    ord => return ord,
                }
            }
            (Ok(_), Err(_)) => {
                // a has numeric prefix, b doesn't: numeric < text (pacman behavior)
                return Ordering::Less;
            }
            (Err(_), Ok(_)) => {
                // a doesn't have numeric prefix, b does: text > numeric (pacman behavior)
                return Ordering::Greater;
            }
            (Err(_), Err(_)) => {
                // Neither has numeric prefix, compare lexicographically
                match a_seg.cmp(b_seg) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
            }
        }
    }

    Ordering::Equal
}

/// What: Check if a version satisfies a version requirement.
///
/// Inputs:
/// - `version`: Version string to check (e.g., "1.2.3").
/// - `requirement`: Version requirement with operator (e.g., ">=1.2.0", "=2.0", "<3.0").
///
/// Output:
/// - Returns `true` if the version satisfies the requirement.
/// - Returns `false` if the version does not satisfy the requirement.
/// - Returns `true` if requirement is empty or has no operator (no constraint).
///
/// Details:
/// - Supports operators: `>=`, `<=`, `=`, `>`, `<`.
/// - Uses `compare_versions()` for proper version comparison (not string comparison).
/// - Automatically normalizes versions (strips pkgrel) before comparison.
/// - Empty or invalid requirement strings default to `true` (no constraint).
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::version_satisfies;
///
/// assert!(version_satisfies("2.0", ">=1.5"));
/// assert!(!version_satisfies("1.0", ">=1.5"));
/// assert!(version_satisfies("1.5", "<=1.5"));
/// assert!(version_satisfies("1.6", ">1.5"));
/// assert!(!version_satisfies("1.4", ">1.5"));
/// assert!(version_satisfies("1.5", "=1.5"));
/// assert!(!version_satisfies("1.6", "<1.5"));
/// assert!(version_satisfies("2.0", "")); // Empty requirement = satisfied
/// ```
#[must_use]
pub fn version_satisfies(version: &str, requirement: &str) -> bool {
    // Empty requirement means no constraint
    if requirement.is_empty() {
        return true;
    }

    // Try to extract operator and version
    let (op, req_version) = if let Some(rest) = requirement.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = requirement.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = requirement.strip_prefix("=") {
        ("=", rest)
    } else if let Some(rest) = requirement.strip_prefix(">") {
        (">", rest)
    } else if let Some(rest) = requirement.strip_prefix("<") {
        ("<", rest)
    } else {
        // No operator found, assume satisfied (no constraint)
        return true;
    };

    // Use proper version comparison
    let comparison = compare_versions(version, req_version);

    match op {
        ">=" => matches!(comparison, Ordering::Equal | Ordering::Greater),
        "<=" => matches!(comparison, Ordering::Equal | Ordering::Less),
        "=" => comparison == Ordering::Equal,
        ">" => comparison == Ordering::Greater,
        "<" => comparison == Ordering::Less,
        _ => true, // Unknown operator, assume satisfied
    }
}

/// What: Extract the leading numeric component from a version string.
///
/// Inputs:
/// - `version`: Version string to parse (e.g., "1.2.3", "2.0.0-alpha").
///
/// Output:
/// - Returns `Some(u64)` for the first numeric segment.
/// - Returns `None` when the first segment cannot be parsed as a number.
///
/// Details:
/// - Splits version on `.` and `-`, treating the first token as the major component.
/// - Used by `is_major_version_bump()` to extract major version numbers.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::extract_major_component;
///
/// assert_eq!(extract_major_component("1.2.3"), Some(1));
/// assert_eq!(extract_major_component("2.0.0-alpha"), Some(2));
/// assert_eq!(extract_major_component("10.5.2"), Some(10));
/// assert_eq!(extract_major_component("alpha"), None);
/// ```
#[must_use]
pub fn extract_major_component(version: &str) -> Option<u64> {
    let normalized = normalize_version(version);
    let token = normalized.split(['.', '-']).next()?;
    token.parse::<u64>().ok()
}

/// What: Determine whether a new version constitutes a major version bump.
///
/// Inputs:
/// - `old`: Currently installed version (e.g., "1.2.3").
/// - `new`: Target version to check (e.g., "2.0.0").
///
/// Output:
/// - Returns `true` when the major component increased.
/// - Returns `false` otherwise (same major, minor/patch bump, or parsing failure).
///
/// Details:
/// - Extracts the first numeric segment from both versions.
/// - Compares major version numbers only.
/// - Returns `false` if either version cannot be parsed.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::is_major_version_bump;
///
/// assert!(is_major_version_bump("1.2.3", "2.0.0"));
/// assert!(!is_major_version_bump("1.2.3", "1.3.0"));
/// assert!(!is_major_version_bump("1.2.3", "1.2.4"));
/// assert!(!is_major_version_bump("2.0.0", "1.9.9"));
/// ```
#[must_use]
pub fn is_major_version_bump(old: &str, new: &str) -> bool {
    match (extract_major_component(old), extract_major_component(new)) {
        (Some(old_major), Some(new_major)) => new_major > old_major,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_version() {
        // Pkgrel stripping
        assert_eq!(normalize_version("1.2.3-1"), "1.2.3");
        assert_eq!(normalize_version("1.2.3-42"), "1.2.3");
        assert_eq!(normalize_version("2.0.0-1"), "2.0.0");

        // Text suffixes preserved
        assert_eq!(normalize_version("1.2.3-alpha"), "1.2.3-alpha");
        assert_eq!(normalize_version("1.2.3-beta1"), "1.2.3-beta1");

        // No dash, unchanged
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
        assert_eq!(normalize_version("2.0"), "2.0");
    }

    #[test]
    fn test_compare_versions_basic() {
        // Basic numeric comparisons
        assert_eq!(compare_versions("1.0.0", "1.0.1"), Ordering::Less);
        assert_eq!(compare_versions("1.0.1", "1.0.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.9.9", "2.0.0"), Ordering::Less);
    }

    #[test]
    fn test_compare_versions_missing_segments() {
        // Missing segments treated as "0"
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("1", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.2", "1.2.1"), Ordering::Less);
    }

    #[test]
    fn test_compare_versions_pkgrel() {
        // Pkgrel should be stripped before comparison
        assert_eq!(compare_versions("1.2.3-1", "1.2.3-2"), Ordering::Equal);
        assert_eq!(compare_versions("1.2.3-1", "1.2.3"), Ordering::Equal);
        assert_eq!(compare_versions("1.2.3-10", "1.2.4-1"), Ordering::Less);
    }

    #[test]
    fn test_compare_versions_text_segments() {
        // Numeric < text (pacman behavior)
        assert_eq!(compare_versions("1.2.3", "1.2.3alpha"), Ordering::Greater);
        assert_eq!(compare_versions("1.2.3alpha", "1.2.3"), Ordering::Less);
        assert_eq!(compare_versions("1.2.3alpha", "1.2.3beta"), Ordering::Less);
        assert_eq!(
            compare_versions("1.2.3beta", "1.2.3alpha"),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_versions_mixed() {
        // Mixed numeric and text
        assert_eq!(compare_versions("1.2.3", "1.2.4"), Ordering::Less);
        assert_eq!(compare_versions("1.2.3alpha", "1.2.3beta"), Ordering::Less);
        assert_eq!(compare_versions("1.2.3", "1.2.3alpha"), Ordering::Greater);
        assert_eq!(compare_versions("1.2.3alpha", "1.2.4"), Ordering::Less);
    }

    #[test]
    fn test_compare_versions_edge_cases() {
        // Edge cases
        assert_eq!(compare_versions("", ""), Ordering::Equal);
        assert_eq!(compare_versions("0", "0.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("10.0.0", "9.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.10.0", "1.9.9"), Ordering::Greater);
    }

    #[test]
    fn test_version_satisfies_greater_equal() {
        assert!(version_satisfies("2.0", ">=1.5"));
        assert!(version_satisfies("1.5", ">=1.5"));
        assert!(!version_satisfies("1.0", ">=1.5"));
        assert!(version_satisfies("1.5.1", ">=1.5"));
        assert!(version_satisfies("2.0.0", ">=1.5.0"));
    }

    #[test]
    fn test_version_satisfies_less_equal() {
        assert!(version_satisfies("1.0", "<=1.5"));
        assert!(version_satisfies("1.5", "<=1.5"));
        assert!(!version_satisfies("2.0", "<=1.5"));
        assert!(version_satisfies("1.4.9", "<=1.5"));
    }

    #[test]
    fn test_version_satisfies_equal() {
        assert!(version_satisfies("1.5", "=1.5"));
        assert!(!version_satisfies("1.6", "=1.5"));
        assert!(!version_satisfies("1.4", "=1.5"));
        assert!(version_satisfies("1.5.0", "=1.5"));
    }

    #[test]
    fn test_version_satisfies_greater() {
        assert!(version_satisfies("1.6", ">1.5"));
        assert!(!version_satisfies("1.5", ">1.5"));
        assert!(!version_satisfies("1.4", ">1.5"));
        assert!(version_satisfies("2.0", ">1.5"));
    }

    #[test]
    fn test_version_satisfies_less() {
        assert!(version_satisfies("1.4", "<1.5"));
        assert!(!version_satisfies("1.5", "<1.5"));
        assert!(!version_satisfies("1.6", "<1.5"));
        assert!(version_satisfies("1.0", "<1.5"));
    }

    #[test]
    fn test_version_satisfies_empty() {
        // Empty requirement = no constraint = satisfied
        assert!(version_satisfies("2.0", ""));
        assert!(version_satisfies("1.0", ""));
        assert!(version_satisfies("any-version", ""));
    }

    #[test]
    fn test_version_satisfies_no_operator() {
        // No operator = no constraint = satisfied
        assert!(version_satisfies("2.0", "n/a"));
        assert!(version_satisfies("1.0", "some-text"));
    }

    #[test]
    fn test_version_satisfies_pkgrel() {
        // Pkgrel should be normalized before comparison
        assert!(version_satisfies("1.2.3-1", ">=1.2.3"));
        assert!(version_satisfies("1.2.3-10", ">=1.2.3"));
        assert!(version_satisfies("1.2.3", ">=1.2.3-1"));
        assert!(version_satisfies("1.2.3-5", "=1.2.3-1")); // Both normalized to 1.2.3
    }

    #[test]
    fn test_extract_major_component() {
        assert_eq!(extract_major_component("1.2.3"), Some(1));
        assert_eq!(extract_major_component("2.0.0"), Some(2));
        assert_eq!(extract_major_component("10.5.2"), Some(10));
        assert_eq!(extract_major_component("2.0.0-alpha"), Some(2));
        assert_eq!(extract_major_component("1.2.3-1"), Some(1));
        assert_eq!(extract_major_component("alpha"), None);
        assert_eq!(extract_major_component(""), None);
    }

    #[test]
    fn test_is_major_version_bump() {
        // Major version increases
        assert!(is_major_version_bump("1.2.3", "2.0.0"));
        assert!(is_major_version_bump("1.0.0", "2.0.0"));
        assert!(is_major_version_bump("0.9.9", "1.0.0"));

        // Same major version
        assert!(!is_major_version_bump("1.2.3", "1.3.0"));
        assert!(!is_major_version_bump("1.2.3", "1.2.4"));
        assert!(!is_major_version_bump("1.0.0", "1.9.9"));

        // Downgrade
        assert!(!is_major_version_bump("2.0.0", "1.9.9"));
        assert!(!is_major_version_bump("2.0.0", "1.0.0"));

        // Parsing failures
        assert!(!is_major_version_bump("alpha", "1.0.0"));
        assert!(!is_major_version_bump("1.0.0", "beta"));
        assert!(!is_major_version_bump("", "1.0.0"));
    }

    #[test]
    fn test_is_major_version_bump_pkgrel() {
        // Pkgrel should not affect major version detection
        assert!(is_major_version_bump("1.2.3-1", "2.0.0-1"));
        assert!(!is_major_version_bump("1.2.3-1", "1.3.0-1"));
    }
}
