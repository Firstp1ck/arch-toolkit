//! Parser for AUR .SRCINFO files.
//!
//! This module provides functions for parsing .SRCINFO files, which are
//! machine-readable metadata files generated from PKGBUILD files for AUR packages.

use std::collections::HashSet;

use crate::deps::parse::parse_dep_spec;
use crate::error::Result;
use crate::types::SrcinfoData;

#[cfg(feature = "aur")]
use crate::aur::utils::percent_encode;

/// What: Parse dependencies from .SRCINFO content.
///
/// Inputs:
/// - `srcinfo`: Raw .SRCINFO file content.
///
/// Output:
/// - Returns a tuple of (depends, makedepends, checkdepends, optdepends) vectors.
///
/// Details:
/// - Parses key-value pairs from .SRCINFO format.
/// - Handles array fields that can appear multiple times.
/// - Filters out virtual packages (.so files).
/// - Deduplicates dependencies (returns unique list).
/// - Handles architecture-specific dependencies (e.g., `depends_x86_64`).
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[must_use]
pub fn parse_srcinfo_deps(srcinfo: &str) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut depends = Vec::new();
    let mut makedepends = Vec::new();
    let mut checkdepends = Vec::new();
    let mut optdepends = Vec::new();

    // Use HashSet for deduplication
    let mut seen_depends = HashSet::new();
    let mut seen_makedepends = HashSet::new();
    let mut seen_checkdepends = HashSet::new();
    let mut seen_optdepends = HashSet::new();

    for line in srcinfo.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // .SRCINFO format: key = value (tab-indented)
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Filter out virtual packages (.so files)
            let value_lower = value.to_lowercase();
            if value_lower.ends_with(".so")
                || value_lower.contains(".so.")
                || value_lower.contains(".so=")
            {
                continue;
            }

            // Handle architecture-specific dependencies by merging into main arrays
            let base_key = key
                .find('_')
                .map_or(key, |underscore_pos| &key[..underscore_pos]);

            match base_key {
                "depends" => {
                    if seen_depends.insert(value.to_string()) {
                        depends.push(value.to_string());
                    }
                }
                "makedepends" => {
                    if seen_makedepends.insert(value.to_string()) {
                        makedepends.push(value.to_string());
                    }
                }
                "checkdepends" => {
                    if seen_checkdepends.insert(value.to_string()) {
                        checkdepends.push(value.to_string());
                    }
                }
                "optdepends" => {
                    if seen_optdepends.insert(value.to_string()) {
                        optdepends.push(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    (depends, makedepends, checkdepends, optdepends)
}

/// What: Parse conflicts from .SRCINFO content.
///
/// Inputs:
/// - `srcinfo`: Raw .SRCINFO file content.
///
/// Output:
/// - Returns a vector of conflicting package names (without version constraints).
///
/// Details:
/// - Parses "conflicts" key-value pairs from .SRCINFO format.
/// - Handles array fields that can appear multiple times.
/// - Filters out virtual packages (.so files) and extracts package names from version constraints.
/// - Deduplicates conflicts (returns unique list).
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[must_use]
pub fn parse_srcinfo_conflicts(srcinfo: &str) -> Vec<String> {
    let mut conflicts = Vec::new();
    let mut seen = HashSet::new();

    for line in srcinfo.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // .SRCINFO format: key = value
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Handle architecture-specific conflicts
            let base_key = key
                .find('_')
                .map_or(key, |underscore_pos| &key[..underscore_pos]);

            if base_key == "conflicts" {
                // Filter out virtual packages (.so files)
                let value_lower = value.to_lowercase();
                if value_lower.ends_with(".so")
                    || value_lower.contains(".so.")
                    || value_lower.contains(".so=")
                {
                    continue;
                }
                // Extract package name (remove version constraints if present)
                let spec = parse_dep_spec(value);
                if !spec.name.is_empty() && seen.insert(spec.name.clone()) {
                    conflicts.push(spec.name);
                }
            }
        }
    }

    conflicts
}

/// What: Parse full .SRCINFO content into structured data.
///
/// Inputs:
/// - `content`: Raw .SRCINFO file content.
///
/// Output:
/// - Returns `SrcinfoData` with all parsed fields populated.
///
/// Details:
/// - Parses all fields from .SRCINFO format including pkgbase, pkgname, pkgver, pkgrel.
/// - Extracts all dependency types (depends, makedepends, checkdepends, optdepends).
/// - Extracts conflicts, provides, and replaces arrays.
/// - For split packages (multiple pkgname), uses the first pkgname found.
/// - Handles architecture-specific dependencies by merging into main arrays.
/// - Returns default `SrcinfoData` with empty fields if content is malformed.
#[must_use]
pub fn parse_srcinfo(content: &str) -> SrcinfoData {
    let mut data = SrcinfoData::default();
    let mut pkgname_found = false;

    // Parse dependencies and conflicts
    let (depends, makedepends, checkdepends, optdepends) = parse_srcinfo_deps(content);
    data.depends = depends;
    data.makedepends = makedepends;
    data.checkdepends = checkdepends;
    data.optdepends = optdepends;
    data.conflicts = parse_srcinfo_conflicts(content);

    // Parse other fields
    let mut seen_provides = HashSet::new();
    let mut seen_replaces = HashSet::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Handle architecture-specific fields by stripping suffix
            let base_key = key
                .find('_')
                .map_or(key, |underscore_pos| &key[..underscore_pos]);

            match base_key {
                "pkgbase" => {
                    if data.pkgbase.is_empty() {
                        data.pkgbase = value.to_string();
                    }
                }
                "pkgname" => {
                    // For split packages, use the first pkgname found
                    if !pkgname_found {
                        data.pkgname = value.to_string();
                        pkgname_found = true;
                    }
                }
                "pkgver" => {
                    if data.pkgver.is_empty() {
                        data.pkgver = value.to_string();
                    }
                }
                "pkgrel" => {
                    if data.pkgrel.is_empty() {
                        data.pkgrel = value.to_string();
                    }
                }
                "provides" => {
                    if seen_provides.insert(value.to_string()) {
                        data.provides.push(value.to_string());
                    }
                }
                "replaces" => {
                    if seen_replaces.insert(value.to_string()) {
                        data.replaces.push(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    data
}

/// What: Fetch .SRCINFO content for an AUR package using async HTTP.
///
/// Inputs:
/// - `client`: Reqwest HTTP client.
/// - `name`: AUR package name.
///
/// Output:
/// - Returns .SRCINFO content as a string, or an error if fetch fails.
///
/// # Errors
/// - Returns `Err` when HTTP request fails (network error or client error)
/// - Returns `Err` when HTTP response status is not successful
/// - Returns `Err` when response body cannot be read
/// - Returns `Err` when response is empty or contains HTML error page
/// - Returns `Err` when response does not appear to be valid .SRCINFO format
///
/// Details:
/// - Uses reqwest for async fetching with built-in timeout handling.
/// - Validates that the response is not empty, not HTML, and contains .SRCINFO format markers.
/// - Requires the `aur` feature to be enabled.
#[cfg(feature = "aur")]
pub async fn fetch_srcinfo(client: &reqwest::Client, name: &str) -> Result<String> {
    use crate::error::ArchToolkitError;

    let url = format!(
        "https://aur.archlinux.org/cgit/aur.git/plain/.SRCINFO?h={}",
        percent_encode(name)
    );
    tracing::debug!("Fetching .SRCINFO from: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(ArchToolkitError::Network)?;

    if !response.status().is_success() {
        return Err(ArchToolkitError::InvalidInput(format!(
            "HTTP request failed with status: {}",
            response.status()
        )));
    }

    let text = response.text().await.map_err(ArchToolkitError::Network)?;

    if text.trim().is_empty() {
        return Err(ArchToolkitError::EmptyInput {
            field: "srcinfo_content".to_string(),
            message: "Empty .SRCINFO content".to_string(),
        });
    }

    // Check if we got an HTML error page instead of .SRCINFO content
    if text.trim_start().starts_with("<html") || text.trim_start().starts_with("<!DOCTYPE") {
        return Err(ArchToolkitError::Parse(
            "Received HTML error page instead of .SRCINFO".to_string(),
        ));
    }

    // Validate that it looks like .SRCINFO format (should have pkgbase or pkgname)
    if !text.contains("pkgbase =") && !text.contains("pkgname =") {
        return Err(ArchToolkitError::Parse(
            "Response does not appear to be valid .SRCINFO format".to_string(),
        ));
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_srcinfo_deps() {
        let srcinfo = r"
pkgbase = test-package
pkgname = test-package
pkgver = 1.0.0
pkgrel = 1
depends = foo
depends = bar>=1.2.3
makedepends = make
makedepends = gcc
checkdepends = check
optdepends = optional: optional-package
depends = libfoo.so=1-64
";

        let (depends, makedepends, checkdepends, optdepends) = parse_srcinfo_deps(srcinfo);

        // Should have 2 depends (foo and bar>=1.2.3), libfoo.so should be filtered
        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar>=1.2.3".to_string()));

        // Should have 2 makedepends
        assert_eq!(makedepends.len(), 2);
        assert!(makedepends.contains(&"make".to_string()));
        assert!(makedepends.contains(&"gcc".to_string()));

        // Should have 1 checkdepends
        assert_eq!(checkdepends.len(), 1);
        assert!(checkdepends.contains(&"check".to_string()));

        // Should have 1 optdepends (with "optional:" prefix)
        assert_eq!(optdepends.len(), 1);
        assert!(optdepends.contains(&"optional: optional-package".to_string()));
    }

    #[test]
    fn test_parse_srcinfo_deps_deduplicates() {
        let srcinfo = r"
depends = glibc
depends = gtk3
depends = glibc
depends = nss
";

        let (depends, _, _, _) = parse_srcinfo_deps(srcinfo);
        assert_eq!(depends.len(), 3, "Should deduplicate dependencies");
        assert!(depends.contains(&"glibc".to_string()));
        assert!(depends.contains(&"gtk3".to_string()));
        assert!(depends.contains(&"nss".to_string()));
    }

    #[test]
    fn test_parse_srcinfo_deps_arch_specific() {
        let srcinfo = r"
depends = common-dep
depends_x86_64 = arch-specific-dep
depends_aarch64 = arm-dep
";

        let (depends, _, _, _) = parse_srcinfo_deps(srcinfo);
        // All architecture-specific deps should be merged
        assert!(depends.contains(&"common-dep".to_string()));
        assert!(depends.contains(&"arch-specific-dep".to_string()));
        assert!(depends.contains(&"arm-dep".to_string()));
    }

    #[test]
    fn test_parse_srcinfo_conflicts() {
        let srcinfo = r"
pkgbase = test-package
pkgname = test-package
pkgver = 1.0.0
pkgrel = 1
conflicts = conflicting-pkg1
conflicts = conflicting-pkg2>=2.0
conflicts = libfoo.so=1-64
";

        let conflicts = parse_srcinfo_conflicts(srcinfo);

        // Should have 2 conflicts (conflicting-pkg1 and conflicting-pkg2), libfoo.so should be filtered
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.contains(&"conflicting-pkg1".to_string()));
        assert!(conflicts.contains(&"conflicting-pkg2".to_string()));
    }

    #[test]
    fn test_parse_srcinfo_conflicts_empty() {
        let srcinfo = r"
pkgbase = test-package
pkgname = test-package
pkgver = 1.0.0
";

        let conflicts = parse_srcinfo_conflicts(srcinfo);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_parse_srcinfo_conflicts_deduplicates() {
        let srcinfo = r"
conflicts = pkg1
conflicts = pkg2
conflicts = pkg1
conflicts = pkg3
";

        let conflicts = parse_srcinfo_conflicts(srcinfo);
        assert_eq!(conflicts.len(), 3, "Should deduplicate conflicts");
        assert!(conflicts.contains(&"pkg1".to_string()));
        assert!(conflicts.contains(&"pkg2".to_string()));
        assert!(conflicts.contains(&"pkg3".to_string()));
    }

    #[test]
    fn test_parse_srcinfo_full() {
        let srcinfo = r"
pkgbase = test-package
pkgname = test-package
pkgver = 1.0.0
pkgrel = 1
depends = glibc
depends = python>=3.12
makedepends = make
checkdepends = check
optdepends = optional: optional-package
conflicts = conflicting-pkg
provides = provided-pkg
replaces = replaced-pkg
";

        let data = parse_srcinfo(srcinfo);

        assert_eq!(data.pkgbase, "test-package");
        assert_eq!(data.pkgname, "test-package");
        assert_eq!(data.pkgver, "1.0.0");
        assert_eq!(data.pkgrel, "1");
        assert_eq!(data.depends.len(), 2);
        assert!(data.depends.contains(&"glibc".to_string()));
        assert!(data.depends.contains(&"python>=3.12".to_string()));
        assert_eq!(data.makedepends.len(), 1);
        assert!(data.makedepends.contains(&"make".to_string()));
        assert_eq!(data.checkdepends.len(), 1);
        assert!(data.checkdepends.contains(&"check".to_string()));
        assert_eq!(data.optdepends.len(), 1);
        assert!(
            data.optdepends
                .contains(&"optional: optional-package".to_string())
        );
        assert_eq!(data.conflicts.len(), 1);
        assert!(data.conflicts.contains(&"conflicting-pkg".to_string()));
        assert_eq!(data.provides.len(), 1);
        assert!(data.provides.contains(&"provided-pkg".to_string()));
        assert_eq!(data.replaces.len(), 1);
        assert!(data.replaces.contains(&"replaced-pkg".to_string()));
    }

    #[test]
    fn test_parse_srcinfo_split_packages() {
        let srcinfo = r"
pkgbase = split-package
pkgname = split-package-base
pkgname = split-package-gui
pkgver = 1.0.0
pkgrel = 1
";

        let data = parse_srcinfo(srcinfo);
        // Should use first pkgname found
        assert_eq!(data.pkgname, "split-package-base");
        assert_eq!(data.pkgbase, "split-package");
    }

    #[test]
    fn test_parse_srcinfo_comments_and_blank_lines() {
        let srcinfo = r"
# This is a comment
pkgbase = test-package

pkgname = test-package
# Another comment
pkgver = 1.0.0
";

        let data = parse_srcinfo(srcinfo);
        assert_eq!(data.pkgbase, "test-package");
        assert_eq!(data.pkgname, "test-package");
        assert_eq!(data.pkgver, "1.0.0");
    }

    #[test]
    fn test_parse_srcinfo_empty() {
        let data = parse_srcinfo("");
        assert_eq!(data.pkgbase, "");
        assert_eq!(data.pkgname, "");
        assert_eq!(data.pkgver, "");
        assert_eq!(data.pkgrel, "");
        assert!(data.depends.is_empty());
        assert!(data.makedepends.is_empty());
        assert!(data.checkdepends.is_empty());
        assert!(data.optdepends.is_empty());
        assert!(data.conflicts.is_empty());
        assert!(data.provides.is_empty());
        assert!(data.replaces.is_empty());
    }

    #[test]
    fn test_parse_srcinfo_malformed() {
        // Missing equals signs, invalid format
        let srcinfo = r"
pkgbase test-package
invalid line
";

        let data = parse_srcinfo(srcinfo);
        // Should handle gracefully, pkgbase won't be set
        assert_eq!(data.pkgbase, "");
    }
}
