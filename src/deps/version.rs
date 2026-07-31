//! Version comparison utilities for dependency resolution.
//!
//! This module provides epoch/pkgver/pkgrel comparison for dependency
//! requirement checking and version analysis. It matches libalpm's conditional
//! pkgrel behavior while preserving the resolver's existing pkgver segmentation.

use std::cmp::Ordering;

/// What: Split an Arch package version into epoch and remaining version text.
///
/// Inputs:
/// - `version`: A version that may begin with a numeric `epoch:` prefix.
///
/// Output:
/// - Returns the parsed epoch and the remaining `pkgver-pkgrel` text.
///
/// Details:
/// - Missing or malformed epochs are treated as epoch zero without discarding version text.
fn split_epoch(version: &str) -> (u64, &str) {
    version
        .split_once(':')
        .and_then(|(epoch, rest)| {
            (!rest.is_empty())
                .then(|| epoch.parse::<u64>().ok().map(|epoch| (epoch, rest)))
                .flatten()
        })
        .unwrap_or((0, version))
}

/// What: Split an Arch package version into pkgver and numeric pkgrel.
///
/// Inputs:
/// - `version`: Version text without an epoch prefix.
///
/// Output:
/// - Returns pkgver and an optional numeric pkgrel.
///
/// Details:
/// - Only a final numeric `-pkgrel` suffix is split; textual prerelease suffixes remain pkgver.
/// - A missing pkgrel remains `None` because libalpm compares pkgrel only when both versions
///   declare one.
fn split_pkgrel(version: &str) -> (&str, Option<&str>) {
    version
        .rsplit_once('-')
        .and_then(|(pkgver, pkgrel)| {
            (!pkgver.is_empty()
                && !pkgrel.is_empty()
                && pkgrel.chars().all(|character| character.is_ascii_digit()))
            .then_some((pkgver, Some(pkgrel)))
        })
        .unwrap_or((version, None))
}

/// What: Normalize a version string to its pkgver component.
///
/// Inputs:
/// - `version`: A version that may contain epoch and pkgrel components.
///
/// Output:
/// - Returns pkgver without epoch or numeric pkgrel.
///
/// Details:
/// - This helper is only used for major-version presentation logic; full comparison retains epoch
///   and pkgrel through `compare_versions`.
fn normalize_version(version: &str) -> String {
    let (_, without_epoch) = split_epoch(version);
    split_pkgrel(without_epoch).0.to_string()
}

/// What: Compare two pkgver-like strings by Arch-compatible numeric and text segments.
///
/// Inputs:
/// - `left`: Left pkgver or pkgrel string.
/// - `right`: Right pkgver or pkgrel string.
///
/// Output:
/// - Returns lexical/numeric ordering for the first different segment.
///
/// Details:
/// - Missing segments are zero and an empty text suffix sorts after a non-empty suffix, matching
///   the existing resolver's prerelease behavior.
fn compare_version_components(left: &str, right: &str) -> Ordering {
    let left_parts = left.split(['.', '-']).collect::<Vec<_>>();
    let right_parts = right.split(['.', '-']).collect::<Vec<_>>();
    for index in 0..left_parts.len().max(right_parts.len()) {
        let left_segment = left_parts.get(index).copied().unwrap_or("0");
        let right_segment = right_parts.get(index).copied().unwrap_or("0");
        let left_end = left_segment
            .char_indices()
            .find(|(_, character)| !character.is_ascii_digit())
            .map_or(left_segment.len(), |(index, _)| index);
        let right_end = right_segment
            .char_indices()
            .find(|(_, character)| !character.is_ascii_digit())
            .map_or(right_segment.len(), |(index, _)| index);
        let (left_number, left_suffix) = (&left_segment[..left_end], &left_segment[left_end..]);
        let (right_number, right_suffix) =
            (&right_segment[..right_end], &right_segment[right_end..]);
        let ordering = match (left_number.parse::<u64>(), right_number.parse::<u64>()) {
            (Ok(left_number), Ok(right_number)) => left_number.cmp(&right_number),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => left_segment.cmp(right_segment),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
        match (left_suffix.is_empty(), right_suffix.is_empty()) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) if left_suffix != right_suffix => return left_suffix.cmp(right_suffix),
            _ => {}
        }
    }
    Ordering::Equal
}

/// What: Compare two Arch package versions including epoch, pkgver, and pkgrel.
///
/// Inputs:
/// - `a`: Left-hand package version.
/// - `b`: Right-hand package version.
///
/// Output:
/// - Returns `Ordering::Less`, `Ordering::Equal`, or `Ordering::Greater`.
///
/// Details:
/// - Numeric epoch takes precedence, followed by pkgver. Numeric pkgrel is compared only when
///   both operands declare one, matching libalpm/pacman dependency semantics.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::compare_versions;
/// use std::cmp::Ordering;
///
/// assert_eq!(compare_versions("1:1.2.3-2", "1:1.2.3-1"), Ordering::Greater);
/// assert_eq!(compare_versions("2:1.0-1", "1:99.0-9"), Ordering::Greater);
/// ```
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (a_epoch, a_without_epoch) = split_epoch(a);
    let (b_epoch, b_without_epoch) = split_epoch(b);
    let epoch_ordering = a_epoch.cmp(&b_epoch);
    if epoch_ordering != Ordering::Equal {
        return epoch_ordering;
    }
    let (a_pkgver, a_pkgrel) = split_pkgrel(a_without_epoch);
    let (b_pkgver, b_pkgrel) = split_pkgrel(b_without_epoch);
    let pkgver_ordering = compare_version_components(a_pkgver, b_pkgver);
    if pkgver_ordering != Ordering::Equal {
        return pkgver_ordering;
    }
    match (a_pkgrel, b_pkgrel) {
        (Some(a_pkgrel), Some(b_pkgrel)) => compare_version_components(a_pkgrel, b_pkgrel),
        _ => Ordering::Equal,
    }
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
/// - Matches libalpm by comparing pkgrel only when both operands declare one.
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

    /// What: Verify pkgrel participates in full Arch package version ordering.
    ///
    /// Inputs:
    /// - Fixed versions with equal pkgver and different numeric pkgrel values.
    ///
    /// Output:
    /// - Confirms pkgrel breaks pkgver ties without overriding pkgver ordering.
    ///
    /// Details:
    /// - Libalpm compares pkgrel only when both operands declare one.
    #[test]
    fn test_compare_versions_pkgrel() {
        assert_eq!(compare_versions("1.2.3-1", "1.2.3-2"), Ordering::Less);
        assert_eq!(compare_versions("1.2.3-1", "1.2.3"), Ordering::Equal);
        assert_eq!(compare_versions("1.2.3", "1.2.3-1"), Ordering::Equal);
        assert_eq!(compare_versions("1.2.3-10", "1.2.4-1"), Ordering::Less);
    }

    /// What: Verify epoch precedes pkgver and pkgrel ordering.
    ///
    /// Inputs:
    /// - Fixed versions with different epochs and release values.
    ///
    /// Output:
    /// - Confirms epoch-aware comparisons and requirements are deterministic.
    ///
    /// Details:
    /// - A higher epoch wins even when its pkgver is lexically lower.
    #[test]
    fn test_compare_versions_epoch() {
        assert_eq!(compare_versions("2:1.0-1", "1:99.0-9"), Ordering::Greater);
        assert_eq!(compare_versions("1:1.0-1", "1.0-99"), Ordering::Greater);
        assert!(version_satisfies("1:2.0-3", ">=1:2.0-3"));
        assert!(!version_satisfies("1:2.0-2", ">=1:2.0-3"));
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

    /// What: Verify pkgrel-aware dependency requirement checks.
    ///
    /// Inputs:
    /// - Fixed package versions and requirements containing numeric release suffixes.
    ///
    /// Output:
    /// - Confirms requirements retain release precision.
    ///
    /// Details:
    /// - An absent pkgrel compares equal to a matching pkgver, while two explicit releases retain
    ///   their ordering.
    #[test]
    fn test_version_satisfies_pkgrel() {
        assert!(version_satisfies("1.2.3-1", "=1.2.3"));
        assert!(version_satisfies("1.2.3", "<=1.2.3-1"));
        assert!(version_satisfies("1.2.3-10", ">=1.2.3"));
        assert!(!version_satisfies("1.2.3-5", "=1.2.3-1"));
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
