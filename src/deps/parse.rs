//! Parsing utilities for dependency specifications.
//!
//! This module provides functions to parse:
//! - Dependency specification strings (e.g., "python>=3.12")
//! - Pacman -Si output for dependencies and conflicts

use std::collections::HashSet;

use crate::types::DependencySpec;

/// English labels that indicate the "Depends On" field in pacman output.
/// Hardcoded to avoid i18n dependencies.
const DEPENDS_LABELS: &[&str] = &["Depends On"];

/// Labels that indicate "None" (no dependencies/conflicts).
const NONE_LABELS: &[&str] = &["None"];

/// Labels that indicate the "Conflicts With" field in pacman output.
const CONFLICTS_LABELS: &[&str] = &["Conflicts With"];

/// Common English words to filter out from dependency parsing.
/// These are not valid package names and appear in description text.
const COMMON_WORDS: &[&str] = &[
    "for", "to", "with", "is", "that", "using", "usually", "bundled", "bindings", "tooling", "the",
    "and", "or", "in", "on", "at", "by", "from", "as", "if", "when", "where", "which", "what",
    "how", "why",
];

/// What: Split a dependency specification into name and version requirement.
///
/// Inputs:
/// - `spec`: Dependency string from pacman (e.g., "python>=3.12", "glibc").
///
/// Output:
/// - Returns `DependencySpec` with name and `version_req` fields.
///
/// Details:
/// - Searches for version operators in precedence order: <=, >=, =, <, >
/// - Returns empty `version_req` when no operator is present.
/// - Trims whitespace from both name and version.
///
/// # Examples
///
/// ```
/// use arch_toolkit::deps::parse_dep_spec;
///
/// let spec = parse_dep_spec("python>=3.12");
/// assert_eq!(spec.name, "python");
/// assert_eq!(spec.version_req, ">=3.12");
///
/// let spec = parse_dep_spec("glibc");
/// assert_eq!(spec.name, "glibc");
/// assert!(spec.version_req.is_empty());
/// ```
#[must_use]
pub fn parse_dep_spec(spec: &str) -> DependencySpec {
    // Check operators in precedence order (multi-char before single-char)
    for op in ["<=", ">=", "=", "<", ">"] {
        if let Some(pos) = spec.find(op) {
            return DependencySpec {
                name: spec[..pos].trim().to_string(),
                version_req: spec[pos..].trim().to_string(),
            };
        }
    }
    DependencySpec::new(spec.trim())
}

/// What: Check if a token looks like a valid package name.
///
/// Inputs:
/// - `token`: A whitespace-separated token from pacman output.
///
/// Output:
/// - Returns `true` if token appears to be a valid package name.
///
/// Details:
/// - Filters out .so files (virtual packages)
/// - Filters out common English words
/// - Filters out tokens shorter than 2 characters
/// - Filters out tokens starting with non-alphanumeric (except - and _)
/// - Filters out tokens ending with colons
/// - Requires at least one alphanumeric character
fn is_valid_package_token(token: &str) -> bool {
    if token.is_empty() || token.len() < 2 {
        return false;
    }

    // Filter out .so files (virtual packages)
    // Note: We already convert to lowercase, so case-sensitive comparison is safe here
    let lower = token.to_lowercase();
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    {
        if lower.ends_with(".so") || lower.contains(".so.") || lower.contains(".so=") {
            return false;
        }
    }

    // Filter out common words
    if COMMON_WORDS.contains(&lower.as_str()) {
        return false;
    }

    // Check first character
    let Some(first_char) = token.chars().next() else {
        return false;
    };
    if !first_char.is_alphanumeric() && first_char != '-' && first_char != '_' {
        return false;
    }

    // Filter out tokens ending with colons
    if token.ends_with(':') {
        return false;
    }

    // Must contain at least one alphanumeric character
    token.chars().any(char::is_alphanumeric)
}

/// What: Collect continuation lines for a pacman -Si field.
///
/// Inputs:
/// - `lines`: All lines from the pacman output
/// - `start_index`: Index of the field line (e.g., "Depends On")
/// - `field_value`: Initial value after the colon on the field line
///
/// Output:
/// - Returns concatenated string with all continuation lines appended
///
/// Details:
/// - Continuation lines are indented and don't have field names
/// - Stops when hitting a new field, empty line, or non-continuation line
fn collect_continuation_lines(lines: &[&str], start_index: usize, field_value: &str) -> String {
    let mut result = field_value.to_string();

    for continuation_line in lines.iter().skip(start_index + 1) {
        // Stop if we hit an empty line
        if continuation_line.trim().is_empty() {
            break;
        }

        // Check if this is a continuation line (starts with whitespace, no field name)
        let trimmed = continuation_line.trim_start();
        if continuation_line.starts_with(char::is_whitespace) {
            // This is a continuation line - append it
            result.push(' ');
            result.push_str(trimmed);
        } else if trimmed.contains(':') && !trimmed.starts_with(char::is_whitespace) {
            // This is a new field, stop collecting
            break;
        } else {
            // Not a continuation, stop
            break;
        }
    }

    result
}

/// What: Extract dependency specifications from pacman -Si "Depends On" field.
///
/// Inputs:
/// - `text`: Raw stdout from `pacman -Si` for a package.
///
/// Output:
/// - Returns vector of dependency specification strings (without .so virtual packages).
///
/// Details:
/// - Scans for lines starting with "Depends On"
/// - Handles multi-line dependencies (continuation lines are indented)
/// - Splits dependencies on whitespace
/// - Filters out .so virtual packages
/// - Filters out common words and invalid tokens
/// - Deduplicates dependencies (returns unique list)
/// - Returns empty vector if no dependencies or "None"
///
/// # Examples
///
/// ```
/// use arch_toolkit::deps::parse_pacman_si_deps;
///
/// let output = "Name            : firefox\nDepends On      : glibc gtk3 python>=3.10\n";
/// let deps = parse_pacman_si_deps(output);
/// assert_eq!(deps, vec!["glibc", "gtk3", "python>=3.10"]);
///
/// let output = "Name            : base\nDepends On      : None\n";
/// let deps = parse_pacman_si_deps(output);
/// assert!(deps.is_empty());
/// ```
#[must_use]
pub fn parse_pacman_si_deps(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        // Check if line starts with any known "Depends On" label
        let is_depends_line = DEPENDS_LABELS.iter().any(|label| line.starts_with(label))
            || (line.contains("Depends") && line.contains("On"));

        if is_depends_line && let Some(colon_pos) = line.find(':') {
            // Start with text after colon on the "Depends On" line
            let initial_value = line[colon_pos + 1..].trim();

            // Collect continuation lines (indented lines without field names)
            let deps_str = collect_continuation_lines(&lines, i, initial_value);

            // Check for "None" equivalent
            let deps_str_trimmed = deps_str.trim();
            if deps_str_trimmed.is_empty()
                || NONE_LABELS
                    .iter()
                    .any(|label| deps_str_trimmed.eq_ignore_ascii_case(label))
            {
                return Vec::new();
            }

            // Split, filter, deduplicate, and collect
            let mut seen = HashSet::new();
            return deps_str_trimmed
                .split_whitespace()
                .map(str::trim)
                .filter(|s| is_valid_package_token(s))
                .filter_map(|s| {
                    // Deduplicate: only add if not seen before
                    if seen.insert(s) {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect();
        }
    }
    Vec::new()
}

/// What: Extract conflict specifications from pacman -Si "Conflicts With" field.
///
/// Inputs:
/// - `text`: Raw stdout from `pacman -Si` for a package.
///
/// Output:
/// - Returns vector of package names that conflict (without version constraints).
///
/// Details:
/// - Scans for lines starting with "Conflicts With"
/// - Handles multi-line conflicts (continuation lines are indented)
/// - Splits conflicts on whitespace
/// - Removes version constraints from package names
/// - Filters out .so virtual packages and invalid tokens
/// - Deduplicates conflicts (returns unique list)
/// - Returns empty vector if no conflicts or "None"
///
/// # Examples
///
/// ```
/// use arch_toolkit::deps::parse_pacman_si_conflicts;
///
/// let output = "Name            : vim\nConflicts With : gvim vi\n";
/// let conflicts = parse_pacman_si_conflicts(output);
/// assert_eq!(conflicts, vec!["gvim", "vi"]);
///
/// let output = "Name            : base\nConflicts With : None\n";
/// let conflicts = parse_pacman_si_conflicts(output);
/// assert!(conflicts.is_empty());
/// ```
#[must_use]
pub fn parse_pacman_si_conflicts(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        // Check if line starts with any known "Conflicts With" label
        let is_conflicts_line = CONFLICTS_LABELS.iter().any(|label| line.starts_with(label))
            || (line.contains("Conflicts") && line.contains("With"));

        if is_conflicts_line && let Some(colon_pos) = line.find(':') {
            // Start with text after colon on the "Conflicts With" line
            let initial_value = line[colon_pos + 1..].trim();

            // Collect continuation lines (indented lines without field names)
            let conflicts_str = collect_continuation_lines(&lines, i, initial_value);

            // Check for "None" equivalent
            let conflicts_str_trimmed = conflicts_str.trim();
            if conflicts_str_trimmed.is_empty()
                || NONE_LABELS
                    .iter()
                    .any(|label| conflicts_str_trimmed.eq_ignore_ascii_case(label))
            {
                return Vec::new();
            }

            // Split, filter, extract package name (remove version), deduplicate, and collect
            let mut seen = HashSet::new();
            return conflicts_str_trimmed
                .split_whitespace()
                .map(str::trim)
                .filter(|s| is_valid_package_token(s))
                .map(|s| parse_dep_spec(s).name)
                .filter_map(|name| {
                    // Deduplicate: only add if not seen before
                    if seen.insert(name.clone()) {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    // === parse_dep_spec tests ===

    #[test]
    fn parse_dep_spec_no_version() {
        let spec = parse_dep_spec("glibc");
        assert_eq!(spec.name, "glibc");
        assert!(spec.version_req.is_empty());
        assert!(!spec.has_version_req());
    }

    #[test]
    fn parse_dep_spec_greater_equal() {
        let spec = parse_dep_spec("python>=3.12");
        assert_eq!(spec.name, "python");
        assert_eq!(spec.version_req, ">=3.12");
    }

    #[test]
    fn parse_dep_spec_less_equal() {
        let spec = parse_dep_spec("openssl<=1.1.1");
        assert_eq!(spec.name, "openssl");
        assert_eq!(spec.version_req, "<=1.1.1");
    }

    #[test]
    fn parse_dep_spec_equal() {
        let spec = parse_dep_spec("firefox=121.0");
        assert_eq!(spec.name, "firefox");
        assert_eq!(spec.version_req, "=121.0");
    }

    #[test]
    fn parse_dep_spec_greater() {
        let spec = parse_dep_spec("rust>1.70");
        assert_eq!(spec.name, "rust");
        assert_eq!(spec.version_req, ">1.70");
    }

    #[test]
    fn parse_dep_spec_less() {
        let spec = parse_dep_spec("cmake<4.0");
        assert_eq!(spec.name, "cmake");
        assert_eq!(spec.version_req, "<4.0");
    }

    #[test]
    fn parse_dep_spec_with_whitespace() {
        let spec = parse_dep_spec("  python >= 3.12  ");
        assert_eq!(spec.name, "python");
        assert_eq!(spec.version_req, ">= 3.12");
    }

    #[test]
    fn parse_dep_spec_complex_version() {
        let spec = parse_dep_spec("qt5-base>=5.15.10-1");
        assert_eq!(spec.name, "qt5-base");
        assert_eq!(spec.version_req, ">=5.15.10-1");
    }

    // === is_valid_package_token tests ===

    #[test]
    fn is_valid_package_token_valid() {
        assert!(is_valid_package_token("glibc"));
        assert!(is_valid_package_token("qt5-base"));
        assert!(is_valid_package_token("python3"));
        assert!(is_valid_package_token("lib32-glibc"));
    }

    #[test]
    fn is_valid_package_token_so_files() {
        assert!(!is_valid_package_token("libedit.so"));
        assert!(!is_valid_package_token("libgit2.so.1"));
        assert!(!is_valid_package_token("libfoo.so=0-64"));
    }

    #[test]
    fn is_valid_package_token_common_words() {
        assert!(!is_valid_package_token("for"));
        assert!(!is_valid_package_token("with"));
        assert!(!is_valid_package_token("the"));
    }

    #[test]
    fn is_valid_package_token_short() {
        assert!(!is_valid_package_token("a"));
        assert!(!is_valid_package_token(""));
    }

    #[test]
    fn is_valid_package_token_invalid_start() {
        assert!(!is_valid_package_token("(test)"));
        assert!(!is_valid_package_token("[optional]"));
    }

    #[test]
    fn is_valid_package_token_colon_ending() {
        assert!(!is_valid_package_token("error:"));
    }

    // === parse_pacman_si_deps tests ===

    #[test]
    fn parse_pacman_si_deps_basic() {
        let text = "Name            : firefox\nDepends On      : glibc gtk3 nss\n";
        let deps = parse_pacman_si_deps(text);
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&"glibc".to_string()));
        assert!(deps.contains(&"gtk3".to_string()));
        assert!(deps.contains(&"nss".to_string()));
    }

    #[test]
    fn parse_pacman_si_deps_with_versions() {
        let text = "Depends On      : python>=3.10 rust>=1.70\n";
        let deps = parse_pacman_si_deps(text);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"python>=3.10".to_string()));
        assert!(deps.contains(&"rust>=1.70".to_string()));
    }

    #[test]
    fn parse_pacman_si_deps_none() {
        let text = "Depends On      : None\n";
        let deps = parse_pacman_si_deps(text);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_pacman_si_deps_empty() {
        let text = "Depends On      :\n";
        let deps = parse_pacman_si_deps(text);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_pacman_si_deps_filters_so() {
        let text = "Depends On      : glibc libedit.so libgit2.so.1 nss\n";
        let deps = parse_pacman_si_deps(text);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"glibc".to_string()));
        assert!(deps.contains(&"nss".to_string()));
    }

    #[test]
    fn parse_pacman_si_deps_no_depends_line() {
        let text = "Name            : firefox\nVersion         : 121.0\n";
        let deps = parse_pacman_si_deps(text);
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_pacman_si_deps_deduplicates() {
        let text = "Depends On      : glibc gtk3 glibc nss gtk3\n";
        let deps = parse_pacman_si_deps(text);
        assert_eq!(deps.len(), 3, "Should deduplicate dependencies");
        assert!(deps.contains(&"glibc".to_string()));
        assert!(deps.contains(&"gtk3".to_string()));
        assert!(deps.contains(&"nss".to_string()));
    }

    #[test]
    fn parse_pacman_si_deps_multiline() {
        let text = "Name            : firefox\nDepends On      : glibc gtk3 libpulse nss\n                  libxt libxss libxcomposite\n                  libx11 libxcb\n";
        let deps = parse_pacman_si_deps(text);
        assert_eq!(deps.len(), 9);
        assert!(deps.contains(&"glibc".to_string()));
        assert!(deps.contains(&"gtk3".to_string()));
        assert!(deps.contains(&"libpulse".to_string()));
        assert!(deps.contains(&"nss".to_string()));
        assert!(deps.contains(&"libxt".to_string()));
        assert!(deps.contains(&"libxss".to_string()));
        assert!(deps.contains(&"libxcomposite".to_string()));
        assert!(deps.contains(&"libx11".to_string()));
        assert!(deps.contains(&"libxcb".to_string()));
    }

    // === parse_pacman_si_conflicts tests ===

    #[test]
    fn parse_pacman_si_conflicts_basic() {
        let text = "Conflicts With  : conflicting-pkg1 conflicting-pkg2\n";
        let conflicts = parse_pacman_si_conflicts(text);
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.contains(&"conflicting-pkg1".to_string()));
        assert!(conflicts.contains(&"conflicting-pkg2".to_string()));
    }

    #[test]
    fn parse_pacman_si_conflicts_with_versions() {
        let text = "Conflicts With  : old-pkg<2.0 new-pkg>=3.0\n";
        let conflicts = parse_pacman_si_conflicts(text);
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.contains(&"old-pkg".to_string()));
        assert!(conflicts.contains(&"new-pkg".to_string()));
    }

    #[test]
    fn parse_pacman_si_conflicts_none() {
        let text = "Conflicts With  : None\n";
        let conflicts = parse_pacman_si_conflicts(text);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_pacman_si_conflicts_empty() {
        let text = "Conflicts With  :\n";
        let conflicts = parse_pacman_si_conflicts(text);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_pacman_si_conflicts_no_conflicts_line() {
        let text = "Name            : firefox\nVersion         : 121.0\n";
        let conflicts = parse_pacman_si_conflicts(text);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn parse_pacman_si_conflicts_deduplicates() {
        let text = "Conflicts With  : pkg1 pkg2 pkg1 pkg3\n";
        let conflicts = parse_pacman_si_conflicts(text);
        assert_eq!(conflicts.len(), 3, "Should deduplicate conflicts");
        assert!(conflicts.contains(&"pkg1".to_string()));
        assert!(conflicts.contains(&"pkg2".to_string()));
        assert!(conflicts.contains(&"pkg3".to_string()));
    }
}
