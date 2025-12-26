//! Index-related data types for official repository package operations.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// What: Capture the minimal metadata about an official package entry.
///
/// Inputs:
/// - Populated primarily from `pacman -Sl`/API responses with optional enrichment.
///
/// Output:
/// - Serves as the source of truth for official repository package information.
///
/// Details:
/// - Represents a package from official Arch Linux repositories.
/// - Non-name fields may be empty initially; enrichment routines fill them lazily.
/// - Serializable via Serde to allow saving and restoring across sessions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialPackage {
    /// Package name.
    pub name: String,
    /// Repository name (e.g., "core", "extra", "community").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repo: String,
    /// Target architecture (e.g., `x86_64`, `any`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub arch: String,
    /// Package version.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    /// Package description.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// What: Represent the full collection of official packages maintained in memory.
///
/// Inputs:
/// - Populated by fetch and enrichment routines before being persisted or queried.
///
/// Output:
/// - Exposed through API helpers that clone or iterate the package list.
///
/// Details:
/// - Serializable via Serde to allow saving and restoring across sessions.
/// - The `name_to_idx` field is derived from `pkgs` and skipped during serialization.
/// - Provides O(1) lookup via `find_package_by_name()` when `name_to_idx` is populated.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OfficialIndex {
    /// All known official packages in the index.
    pub pkgs: Vec<OfficialPackage>,
    /// Index mapping lowercase package names to their position in `pkgs` for O(1) lookups.
    /// Skipped during serialization; rebuilt after deserialization via `rebuild_name_index()`.
    #[serde(skip)]
    pub name_to_idx: HashMap<String, usize>,
}

impl OfficialIndex {
    /// What: Rebuild the `name_to_idx` `HashMap` from the current `pkgs` Vec.
    ///
    /// Inputs:
    /// - None (operates on `self.pkgs`)
    ///
    /// Output:
    /// - Populates `self.name_to_idx` with lowercase package names mapped to indices.
    ///
    /// Details:
    /// - Should be called after deserialization or when `pkgs` is modified.
    /// - Uses lowercase names for case-insensitive lookups.
    /// - Clears existing index before rebuilding to ensure consistency.
    pub fn rebuild_name_index(&mut self) {
        self.name_to_idx.clear();
        self.name_to_idx.reserve(self.pkgs.len());
        for (i, pkg) in self.pkgs.iter().enumerate() {
            self.name_to_idx.insert(pkg.name.to_lowercase(), i);
        }
    }

    /// What: Find a package by name in the official index using O(1) lookup.
    ///
    /// Inputs:
    /// - `name`: Package name to search for (case-insensitive)
    ///
    /// Output:
    /// - `Some(&OfficialPackage)` if the package is found, `None` otherwise.
    ///
    /// Details:
    /// - Uses the `name_to_idx` `HashMap` for O(1) lookup by lowercase name.
    /// - Falls back to linear scan if `HashMap` is empty (e.g., before rebuild).
    /// - Case-insensitive matching is performed.
    #[must_use]
    pub fn find_package_by_name(&self, name: &str) -> Option<&OfficialPackage> {
        // Try O(1) HashMap lookup first
        let name_lower = name.to_lowercase();
        if let Some(&idx) = self.name_to_idx.get(&name_lower) {
            return self.pkgs.get(idx);
        }
        // Fallback to linear scan if HashMap is empty or index mismatch
        self.pkgs.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

/// What: Search result with optional fuzzy matching score.
///
/// Inputs:
/// - Created by search functions that match packages against queries.
///
/// Output:
/// - Contains the matched package and its fuzzy score (if fuzzy matching was used).
///
/// Details:
/// - Used to return search results with relevance scores for sorting.
/// - `fuzzy_score` is `None` for exact or substring matches.
/// - `fuzzy_score` is `Some(i64)` when fuzzy matching is enabled, with higher scores indicating better matches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexQueryResult {
    /// The matched package.
    pub package: OfficialPackage,
    /// Fuzzy matching score, if fuzzy matching was used.
    pub fuzzy_score: Option<i64>,
}

/// What: Filter mode for querying explicitly installed packages.
///
/// Inputs:
/// - Used as parameter to explicit package query functions.
///
/// Output:
/// - Determines which pacman command arguments are used.
///
/// Details:
/// - `LeafOnly`: Uses `pacman -Qetq` (explicitly installed AND not required by other packages).
/// - `AllExplicit`: Uses `pacman -Qeq` (all explicitly installed packages, including dependencies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstalledPackagesMode {
    /// Query only leaf packages (explicitly installed and not required).
    LeafOnly,
    /// Query all explicitly installed packages.
    AllExplicit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify `rebuild_name_index` populates `HashMap` correctly.
    ///
    /// Inputs:
    /// - `OfficialIndex` with two packages.
    ///
    /// Output:
    /// - `HashMap` contains lowercase names mapped to correct indices.
    ///
    /// Details:
    /// - Tests that the `HashMap` is built correctly and supports case-insensitive lookups.
    fn rebuild_name_index_populates_hashmap() {
        let mut index = OfficialIndex {
            pkgs: vec![
                OfficialPackage {
                    name: "PackageA".to_string(),
                    repo: "core".to_string(),
                    arch: "x86_64".to_string(),
                    version: "1.0".to_string(),
                    description: "Desc A".to_string(),
                },
                OfficialPackage {
                    name: "PackageB".to_string(),
                    repo: "extra".to_string(),
                    arch: "any".to_string(),
                    version: "2.0".to_string(),
                    description: "Desc B".to_string(),
                },
            ],
            name_to_idx: HashMap::new(),
        };

        index.rebuild_name_index();

        assert_eq!(index.name_to_idx.len(), 2);
        assert_eq!(index.name_to_idx.get("packagea"), Some(&0));
        assert_eq!(index.name_to_idx.get("packageb"), Some(&1));
        // Original case should not be found
        assert_eq!(index.name_to_idx.get("PackageA"), None);
    }

    #[test]
    /// What: Verify `find_package_by_name` uses `HashMap` for O(1) lookup.
    ///
    /// Inputs:
    /// - Seed index with packages and rebuilt `HashMap`.
    ///
    /// Output:
    /// - Package found via case-insensitive name lookup.
    ///
    /// Details:
    /// - Tests that find works with different case variations.
    fn find_package_by_name_uses_hashmap() {
        let mut index = OfficialIndex {
            pkgs: vec![
                OfficialPackage {
                    name: "ripgrep".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "14.0.0".to_string(),
                    description: "Fast grep".to_string(),
                },
                OfficialPackage {
                    name: "vim".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "9.0".to_string(),
                    description: "Text editor".to_string(),
                },
            ],
            name_to_idx: HashMap::new(),
        };

        index.rebuild_name_index();

        // Test exact case
        let result = index.find_package_by_name("ripgrep");
        assert!(result.is_some());
        assert_eq!(result.map(|p| p.name.as_str()), Some("ripgrep"));

        // Test different case (HashMap uses lowercase)
        let result_upper = index.find_package_by_name("RIPGREP");
        assert!(result_upper.is_some());
        assert_eq!(result_upper.map(|p| p.name.as_str()), Some("ripgrep"));

        // Test non-existent package
        let not_found = index.find_package_by_name("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    /// What: Verify `find_package_by_name` falls back to linear scan when `HashMap` is empty.
    ///
    /// Inputs:
    /// - `OfficialIndex` with packages but empty `name_to_idx`.
    ///
    /// Output:
    /// - Package found via linear scan fallback.
    ///
    /// Details:
    /// - Tests that the fallback mechanism works when index is not rebuilt.
    fn find_package_by_name_fallback_to_linear_scan() {
        let index = OfficialIndex {
            pkgs: vec![OfficialPackage {
                name: "test-package".to_string(),
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
                version: "1.0".to_string(),
                description: "Test".to_string(),
            }],
            name_to_idx: HashMap::new(),
        };

        // Should still find package via linear scan
        let result = index.find_package_by_name("test-package");
        assert!(result.is_some());
        assert_eq!(result.map(|p| p.name.as_str()), Some("test-package"));

        // Case-insensitive fallback
        let result_upper = index.find_package_by_name("TEST-PACKAGE");
        assert!(result_upper.is_some());
    }

    #[test]
    /// What: Verify serialization and deserialization of `OfficialIndex`.
    ///
    /// Inputs:
    /// - `OfficialIndex` with packages and rebuilt name index.
    ///
    /// Output:
    /// - Deserialized index matches original, and name index can be rebuilt.
    ///
    /// Details:
    /// - Tests that `name_to_idx` is skipped during serialization.
    /// - Verifies that index can be rebuilt after deserialization.
    fn serialization_deserialization() {
        let mut index = OfficialIndex {
            pkgs: vec![
                OfficialPackage {
                    name: "package1".to_string(),
                    repo: "core".to_string(),
                    arch: "x86_64".to_string(),
                    version: "1.0".to_string(),
                    description: "Package 1".to_string(),
                },
                OfficialPackage {
                    name: "package2".to_string(),
                    repo: "extra".to_string(),
                    arch: "any".to_string(),
                    version: "2.0".to_string(),
                    description: "Package 2".to_string(),
                },
            ],
            name_to_idx: HashMap::new(),
        };

        index.rebuild_name_index();

        // Serialize
        let json = serde_json::to_string(&index).expect("Serialization should succeed");
        assert!(!json.contains("name_to_idx")); // Should be skipped

        // Deserialize
        let mut deserialized: OfficialIndex =
            serde_json::from_str(&json).expect("Deserialization should succeed");
        assert_eq!(deserialized.pkgs.len(), 2);
        assert!(deserialized.name_to_idx.is_empty()); // Should be empty after deserialization

        // Rebuild index
        deserialized.rebuild_name_index();
        assert_eq!(deserialized.name_to_idx.len(), 2);
        assert_eq!(deserialized.name_to_idx.get("package1"), Some(&0));
        assert_eq!(deserialized.name_to_idx.get("package2"), Some(&1));

        // Verify find works after rebuild
        let found = deserialized.find_package_by_name("package1");
        assert!(found.is_some());
        assert_eq!(found.map(|p| p.name.as_str()), Some("package1"));
    }

    #[test]
    /// What: Verify `IndexQueryResult` creation and serialization.
    ///
    /// Inputs:
    /// - `IndexQueryResult` with package and optional fuzzy score.
    ///
    /// Output:
    /// - Result can be created and serialized correctly.
    ///
    /// Details:
    /// - Tests both with and without fuzzy score.
    fn index_query_result_creation() {
        let package = OfficialPackage {
            name: "test".to_string(),
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
            version: "1.0".to_string(),
            description: "Test package".to_string(),
        };

        // With fuzzy score
        let result_with_score = IndexQueryResult {
            package: package.clone(),
            fuzzy_score: Some(100),
        };
        assert_eq!(result_with_score.fuzzy_score, Some(100));

        // Without fuzzy score
        let result_without_score = IndexQueryResult {
            package,
            fuzzy_score: None,
        };
        assert_eq!(result_without_score.fuzzy_score, None);

        // Serialization test
        let json = serde_json::to_string(&result_with_score).expect("Should serialize");
        let deserialized: IndexQueryResult =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.fuzzy_score, Some(100));
        assert_eq!(deserialized.package.name, "test");
    }
}
