//! Parser for AUR .SRCINFO files.
//!
//! This module provides functions for parsing .SRCINFO files, which are
//! machine-readable metadata files generated from PKGBUILD files for AUR packages.

use std::collections::HashSet;

use crate::deps::parse::parse_dep_spec;
#[cfg(feature = "aur")]
use crate::error::Result;
use crate::types::dependency::SrcinfoData;

#[cfg(feature = "aur")]
use crate::aur::utils::percent_encode;

/// Maximum accepted AUR `.SRCINFO` response body size in bytes.
#[cfg(feature = "aur")]
const MAX_AUR_SRCINFO_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

/// What: Store one split-package output for graph-only `.SRCINFO` resolution.
///
/// Inputs:
/// - Package-output dependency, provider, conflict, and replacement fields.
///
/// Output:
/// - Retains a selected split package's metadata for the injected graph resolver.
///
/// Details:
/// - This internal projection keeps the legacy public `SrcinfoData` struct source-compatible while
///   preserving exact package-output ownership for graph traversal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SrcinfoPackage {
    /// Selected package output name.
    pub(super) name: String,
    /// Runtime dependency specifications.
    pub(super) depends: Vec<String>,
    /// Build dependency specifications.
    pub(super) makedepends: Vec<String>,
    /// Check dependency specifications.
    pub(super) checkdepends: Vec<String>,
    /// Optional dependency specifications.
    pub(super) optdepends: Vec<String>,
    /// Package and virtual conflict specifications.
    pub(super) conflicts: Vec<String>,
    /// Virtual provider specifications.
    pub(super) provides: Vec<String>,
    /// Replacement specifications.
    pub(super) replaces: Vec<String>,
}

/// What: Store graph-specific header and split-package `.SRCINFO` metadata.
///
/// Inputs:
/// - Package-base header fields and selected package-output projections.
///
/// Output:
/// - Supplies epoch/pkgver/pkgrel and split package metadata to graph resolution.
///
/// Details:
/// - This internal representation is separate from the legacy public aggregate parser output to
///   avoid breaking callers that construct `SrcinfoData` with a struct literal.
#[derive(Clone, Debug, Default)]
pub(super) struct GraphSrcinfoData {
    /// Package-base name.
    pub(super) pkgbase: String,
    /// Package epoch, if declared.
    pub(super) epoch: String,
    /// Package version.
    pub(super) pkgver: String,
    /// Package release.
    pub(super) pkgrel: String,
    /// Individual split-package outputs.
    pub(super) packages: Vec<SrcinfoPackage>,
}

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
                "depends" if seen_depends.insert(value.to_string()) => {
                    depends.push(value.to_string());
                }
                "makedepends" if seen_makedepends.insert(value.to_string()) => {
                    makedepends.push(value.to_string());
                }
                "checkdepends" if seen_checkdepends.insert(value.to_string()) => {
                    checkdepends.push(value.to_string());
                }
                "optdepends" if seen_optdepends.insert(value.to_string()) => {
                    optdepends.push(value.to_string());
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

/// What: Normalize a `.SRCINFO` key by removing an architecture suffix.
///
/// Inputs:
/// - `key`: A raw `.SRCINFO` key such as `depends_x86_64`.
///
/// Output:
/// - Returns the key family such as `depends`.
///
/// Details:
/// - `.SRCINFO` uses underscore suffixes for architecture-specific dependency fields.
fn srcinfo_base_key(key: &str) -> &str {
    key.find('_').map_or(key, |position| &key[..position])
}

/// What: Add a metadata value once while retaining first-seen source order.
///
/// Inputs:
/// - `values`: Destination metadata values.
/// - `value`: Metadata value to insert.
///
/// Output:
/// - Updates `values` only when the value was not present.
///
/// Details:
/// - Retaining source order keeps split-package fixture results deterministic before graph sorting.
fn push_srcinfo_value(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

/// What: Add one dependency-related `.SRCINFO` field to a package projection.
///
/// Inputs:
/// - `package`: Package-base or package-output projection to update.
/// - `key`: Normalized `.SRCINFO` field family.
/// - `value`: Trimmed field value.
///
/// Output:
/// - Updates the matching dependency, provider, conflict, or replacement collection.
///
/// Details:
/// - Values are intentionally not filtered: graph resolution must retain virtual `.so` provides
///   and dependencies even though legacy flat parser helpers retain their existing filtering.
fn apply_package_field(package: &mut SrcinfoPackage, key: &str, value: &str) {
    match key {
        "depends" => push_srcinfo_value(&mut package.depends, value),
        "makedepends" => push_srcinfo_value(&mut package.makedepends, value),
        "checkdepends" => push_srcinfo_value(&mut package.checkdepends, value),
        "optdepends" => push_srcinfo_value(&mut package.optdepends, value),
        "conflicts" => push_srcinfo_value(&mut package.conflicts, value),
        "provides" => push_srcinfo_value(&mut package.provides, value),
        "replaces" => push_srcinfo_value(&mut package.replaces, value),
        _ => {}
    }
}

/// What: Merge shared package-base fields into one split-package output.
///
/// Inputs:
/// - `package`: Split-package output to enrich.
/// - `base`: Shared package-base dependency metadata.
///
/// Output:
/// - Updates `package` with every unique base-level field.
///
/// Details:
/// - Package-output fields retain their values and base fields are appended only when absent.
fn merge_package_base(package: &mut SrcinfoPackage, base: &SrcinfoPackage) {
    for (target, shared) in [
        (&mut package.depends, &base.depends),
        (&mut package.makedepends, &base.makedepends),
        (&mut package.checkdepends, &base.checkdepends),
        (&mut package.optdepends, &base.optdepends),
        (&mut package.conflicts, &base.conflicts),
        (&mut package.provides, &base.provides),
        (&mut package.replaces, &base.replaces),
    ] {
        for value in shared {
            push_srcinfo_value(target, value);
        }
    }
}

/// What: Parse lossless package-output dependency metadata from a `.SRCINFO` document.
///
/// Inputs:
/// - `content`: Raw `.SRCINFO` text.
///
/// Output:
/// - Returns one package projection for every `pkgname` section.
///
/// Details:
/// - Package-base metadata is merged into each output. Unlike legacy helpers, virtual entries are
///   retained so a graph provider can verify provider identity and conflicts.
fn parse_srcinfo_packages(content: &str) -> Vec<SrcinfoPackage> {
    let mut base = SrcinfoPackage::default();
    let mut packages = Vec::new();
    let mut current_package = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = srcinfo_base_key(raw_key.trim());
        let value = raw_value.trim();
        if key == "pkgname" {
            packages.push(SrcinfoPackage {
                name: value.to_string(),
                ..SrcinfoPackage::default()
            });
            current_package = Some(packages.len() - 1);
            continue;
        }
        if let Some(index) = current_package {
            apply_package_field(&mut packages[index], key, value);
        } else {
            apply_package_field(&mut base, key, value);
        }
    }

    for package in &mut packages {
        merge_package_base(package, &base);
    }
    packages
}

/// What: Parse graph-specific package-base and split-package `.SRCINFO` metadata.
///
/// Inputs:
/// - `content`: Raw `.SRCINFO` text.
///
/// Output:
/// - Returns package base, epoch/pkgver/pkgrel, and lossless split-package projections.
///
/// Details:
/// - The graph resolver uses this internal parser while legacy callers retain the existing
///   aggregate `parse_srcinfo` contract and its public `SrcinfoData` shape.
pub(super) fn parse_srcinfo_graph(content: &str) -> GraphSrcinfoData {
    let mut data = GraphSrcinfoData {
        packages: parse_srcinfo_packages(content),
        ..GraphSrcinfoData::default()
    };
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = srcinfo_base_key(raw_key.trim());
        let value = raw_value.trim();
        match key {
            "pkgbase" if data.pkgbase.is_empty() => data.pkgbase = value.to_string(),
            "epoch" if data.epoch.is_empty() => data.epoch = value.to_string(),
            "pkgver" if data.pkgver.is_empty() => data.pkgver = value.to_string(),
            "pkgrel" if data.pkgrel.is_empty() => data.pkgrel = value.to_string(),
            _ => {}
        }
    }
    data
}

/// What: Parse full .SRCINFO content into structured data.
///
/// Inputs:
/// - `content`: Raw .SRCINFO file content.
///
/// Output:
/// - Returns `SrcinfoData` with aggregate fields populated.
///
/// Details:
/// - Parses all fields including pkgbase, pkgname, pkgver, pkgrel and package arrays.
/// - Existing aggregate fields retain their historical first-name/merged-array behavior.
/// - Graph-only split-package selection is kept internal to preserve public struct-literal compatibility.
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
                "pkgbase"
                    if data.pkgbase.is_empty() => {
                        data.pkgbase = value.to_string();
                    }
                "pkgname"
                    // For split packages, use the first pkgname found
                    if !pkgname_found => {
                        data.pkgname = value.to_string();
                        pkgname_found = true;
                    }
                "pkgver"
                    if data.pkgver.is_empty() => {
                        data.pkgver = value.to_string();
                    }
                "pkgrel"
                    if data.pkgrel.is_empty() => {
                        data.pkgrel = value.to_string();
                    }
                "provides"
                    if seen_provides.insert(value.to_string()) => {
                        data.provides.push(value.to_string());
                    }
                "replaces"
                    if seen_replaces.insert(value.to_string()) => {
                        data.replaces.push(value.to_string());
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
    let url = format!(
        "https://aur.archlinux.org/cgit/aur.git/plain/.SRCINFO?h={}",
        percent_encode(name)
    );
    fetch_srcinfo_from_url(client, name, &url).await
}

/// What: Fetch and validate one bounded `.SRCINFO` document from a selected URL.
///
/// Inputs:
/// - `client`: Reqwest HTTP client retaining caller timeout and transport policy.
/// - `name`: AUR package name retained in every status, body, and parse error.
/// - `url`: Request URL selected by the public AUR endpoint wrapper or a local test.
///
/// Output:
/// - Validated `.SRCINFO` text within [`MAX_AUR_SRCINFO_RESPONSE_BYTES`].
///
/// Details:
/// - Streams without executing, sourcing, expanding, or logging response content.
/// - The URL remains private to avoid logging a full untrusted value.
#[cfg(feature = "aur")]
async fn fetch_srcinfo_from_url(client: &reqwest::Client, name: &str, url: &str) -> Result<String> {
    use crate::error::ArchToolkitError;

    tracing::debug!(package = %name, "fetching AUR .SRCINFO");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(ArchToolkitError::Network)?;
    let status = response.status();
    if !status.is_success() {
        return Err(ArchToolkitError::InvalidInput(format!(
            "AUR .SRCINFO fetch failed for package '{name}' with status {status}"
        )));
    }

    let resource_label = format!("AUR .SRCINFO for package '{name}'");
    let text = crate::http::read_bounded_response_text(
        response,
        MAX_AUR_SRCINFO_RESPONSE_BYTES,
        &resource_label,
        |error| {
            ArchToolkitError::Parse(format!(
                "{resource_label} response body read failed: {error}"
            ))
        },
    )
    .await?;

    if text.trim().is_empty() {
        return Err(ArchToolkitError::EmptyInput {
            field: format!("AUR .SRCINFO response for package '{name}'"),
            message: "response body was empty".to_string(),
        });
    }
    if text.trim_start().starts_with("<html") || text.trim_start().starts_with("<!DOCTYPE") {
        return Err(ArchToolkitError::Parse(format!(
            "AUR .SRCINFO fetch for package '{name}' received an HTML error page"
        )));
    }
    if !text.contains("pkgbase =") && !text.contains("pkgname =") {
        return Err(ArchToolkitError::Parse(format!(
            "AUR .SRCINFO response for package '{name}' is not valid .SRCINFO format"
        )));
    }

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "aur")]
    use crate::error::ArchToolkitError;
    #[cfg(feature = "aur")]
    use wiremock::matchers::{method, path};
    #[cfg(feature = "aur")]
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

    /// What: Verify split package outputs retain selected and inherited metadata.
    ///
    /// Inputs:
    /// - A fixture with package-base dependencies and two split package outputs.
    ///
    /// Output:
    /// - Confirms legacy first-package fields remain while `packages` preserves both outputs.
    ///
    /// Details:
    /// - Shared base dependencies must be inherited without leaking one output's dependencies into
    ///   another selected split package.
    #[test]
    fn test_parse_srcinfo_split_packages() {
        let srcinfo = r"
pkgbase = split-package
depends = shared-base
pkgname = split-package-base
depends = base-only
pkgname = split-package-gui
depends = gui-only
provides = virtual-gui=1
pkgver = 1.0.0
pkgrel = 1
";

        let data = parse_srcinfo(srcinfo);
        assert_eq!(data.pkgname, "split-package-base");
        assert_eq!(data.pkgbase, "split-package");
        let graph_data = parse_srcinfo_graph(srcinfo);
        assert_eq!(graph_data.packages.len(), 2);
        let base = graph_data
            .packages
            .iter()
            .find(|package| package.name == "split-package-base");
        let gui = graph_data
            .packages
            .iter()
            .find(|package| package.name == "split-package-gui");
        assert!(base.is_some_and(|package| {
            package.depends == vec!["base-only".to_string(), "shared-base".to_string()]
        }));
        assert!(gui.is_some_and(|package| {
            package.depends == vec!["gui-only".to_string(), "shared-base".to_string()]
                && package.provides == vec!["virtual-gui=1".to_string()]
        }));
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

    #[cfg(feature = "aur")]
    #[tokio::test]
    /// What: Reject an oversized AUR `.SRCINFO` response.
    ///
    /// Inputs:
    /// - A local body one byte above the named 10 MiB ceiling.
    ///
    /// Output:
    /// - Contextual `InputTooLong` identifying `.SRCINFO` and package `yay`.
    ///
    /// Details:
    /// - The inert bytes are bounded before metadata format validation.
    async fn oversized_aur_srcinfo_response_is_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.SRCINFO"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![
                b'x';
                MAX_AUR_SRCINFO_RESPONSE_BYTES
                    + 1
            ]))
            .mount(&server)
            .await;

        let error = fetch_srcinfo_from_url(
            &reqwest::Client::new(),
            "yay",
            &format!("{}/.SRCINFO", server.uri()),
        )
        .await
        .expect_err("oversized .SRCINFO response must fail");
        let message = error.to_string();

        assert!(matches!(
            error,
            ArchToolkitError::InputTooLong {
                max_length: MAX_AUR_SRCINFO_RESPONSE_BYTES,
                ..
            }
        ));
        assert!(message.contains(".SRCINFO"));
        assert!(message.contains("yay"));
    }

    #[cfg(feature = "aur")]
    #[tokio::test]
    /// What: Preserve `.SRCINFO` package context for status and invalid bodies.
    ///
    /// Inputs:
    /// - Local 404, empty, malformed text, and HTML responses.
    ///
    /// Output:
    /// - An actionable error naming `.SRCINFO` and package `yay` for each response.
    ///
    /// Details:
    /// - Empty and format checks remain after the bounded strict UTF-8 read.
    async fn aur_srcinfo_status_empty_and_malformed_errors_are_contextual() {
        for (path_value, template) in [
            ("/status", ResponseTemplate::new(404)),
            ("/empty", ResponseTemplate::new(200)),
            (
                "/malformed",
                ResponseTemplate::new(200).set_body_string("not metadata"),
            ),
            (
                "/html",
                ResponseTemplate::new(200).set_body_string("<!DOCTYPE html>error"),
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(path_value))
                .respond_with(template)
                .mount(&server)
                .await;

            let error = fetch_srcinfo_from_url(
                &reqwest::Client::new(),
                "yay",
                &format!("{}{path_value}", server.uri()),
            )
            .await
            .expect_err("invalid .SRCINFO response must fail");
            let message = error.to_string();

            assert!(message.contains(".SRCINFO"));
            assert!(message.contains("yay"));
        }
    }

    #[cfg(feature = "aur")]
    #[tokio::test]
    /// What: Return a normal bounded `.SRCINFO` fixture unchanged.
    ///
    /// Inputs:
    /// - A local valid metadata document for package `yay`.
    ///
    /// Output:
    /// - Exact source text ready for caller-controlled parsing.
    ///
    /// Details:
    /// - The fetch path validates markers but never executes metadata content.
    async fn normal_aur_srcinfo_fixture_is_read() {
        let server = MockServer::start().await;
        let srcinfo = "pkgbase = yay\npkgname = yay\npkgver = 1\n";
        Mock::given(method("GET"))
            .and(path("/.SRCINFO"))
            .respond_with(ResponseTemplate::new(200).set_body_string(srcinfo))
            .mount(&server)
            .await;

        let body = fetch_srcinfo_from_url(
            &reqwest::Client::new(),
            "yay",
            &format!("{}/.SRCINFO", server.uri()),
        )
        .await
        .expect("normal .SRCINFO fixture");

        assert_eq!(body, srcinfo);
    }
}
