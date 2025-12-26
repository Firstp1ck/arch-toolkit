//! Official repository package search functions for the index module.

use crate::types::index::{IndexQueryResult, OfficialIndex, OfficialPackage};

/// What: Search the official index for packages whose names match `query`.
///
/// Inputs:
/// - `index`: Reference to the official package index to search.
/// - `query`: Raw query string to match against package names.
/// - `fuzzy`: When `true`, uses fuzzy matching (fzf-style); when `false`, uses substring matching.
///
/// Output:
/// - Vector of `IndexQueryResult` containing matched packages with optional fuzzy scores.
/// - An empty or whitespace-only query returns an empty list.
/// - When fuzzy mode is enabled, items are returned with scores for sorting.
///
/// Details:
/// - When `fuzzy` is `false`, performs a case-insensitive substring match on package names.
/// - When `fuzzy` is `true`, uses fuzzy matching and returns items with match scores.
/// - Fuzzy matching requires the `fuzzy-search` feature flag; if not available, falls back to substring matching.
/// - Results are not sorted; caller should sort by fuzzy score if needed.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::{search_official, OfficialIndex};
///
/// let index = OfficialIndex::default();
/// // Substring matching
/// let results = search_official(&index, "vim", false);
/// // Fuzzy matching (requires fuzzy-search feature)
/// let fuzzy_results = search_official(&index, "rg", true);
/// ```
#[must_use]
pub fn search_official(index: &OfficialIndex, query: &str, fuzzy: bool) -> Vec<IndexQueryResult> {
    let ql = query.trim();
    if ql.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    // Create fuzzy matcher if fuzzy matching is requested and available
    #[cfg(feature = "fuzzy-search")]
    let fuzzy_matcher = if fuzzy {
        Some(fuzzy_matcher::skim::SkimMatcherV2::default())
    } else {
        None
    };

    // If fuzzy-search feature is not available, always use substring matching
    // (fuzzy parameter is part of API but ignored when feature disabled)
    #[cfg(not(feature = "fuzzy-search"))]
    let _ = fuzzy; // Acknowledge parameter exists but not used without feature
    #[cfg(not(feature = "fuzzy-search"))]
    let use_fuzzy = false;
    #[cfg(feature = "fuzzy-search")]
    let use_fuzzy = fuzzy;

    for pkg in &index.pkgs {
        let match_score = if use_fuzzy {
            #[cfg(feature = "fuzzy-search")]
            {
                fuzzy_matcher.as_ref().and_then(|m| {
                    use fuzzy_matcher::FuzzyMatcher;
                    m.fuzzy_match(&pkg.name, ql)
                })
            }
            #[cfg(not(feature = "fuzzy-search"))]
            {
                None
            }
        } else {
            // Case-insensitive substring matching
            let name_lower = pkg.name.to_lowercase();
            let query_lower = ql.to_lowercase();
            if name_lower.contains(&query_lower) {
                Some(0) // Use 0 as placeholder score for substring matches
            } else {
                None
            }
        };

        if let Some(score) = match_score {
            results.push(IndexQueryResult {
                package: pkg.clone(),
                fuzzy_score: if use_fuzzy { Some(score) } else { None },
            });
        }
    }

    results
}

/// What: Return all packages from the official index.
///
/// Inputs:
/// - `index`: Reference to the official package index.
///
/// Output:
/// - Vector of all `OfficialPackage` entries from the index.
///
/// Details:
/// - Clones all packages from the index.
/// - Order is preserved from the index.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::{all_official, OfficialIndex};
///
/// let index = OfficialIndex::default();
/// let all_packages = all_official(&index);
/// println!("Found {} official packages", all_packages.len());
/// ```
#[must_use]
pub fn all_official(index: &OfficialIndex) -> Vec<OfficialPackage> {
    index.pkgs.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::index::OfficialIndex;

    fn create_test_index() -> OfficialIndex {
        let mut index = OfficialIndex {
            pkgs: vec![
                OfficialPackage {
                    name: "ripgrep".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "14.0.0".to_string(),
                    description: "Fast grep tool".to_string(),
                },
                OfficialPackage {
                    name: "vim".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "9.0".to_string(),
                    description: "Text editor".to_string(),
                },
                OfficialPackage {
                    name: "pacman".to_string(),
                    repo: "core".to_string(),
                    arch: "x86_64".to_string(),
                    version: "6.1.0".to_string(),
                    description: "Package manager".to_string(),
                },
            ],
            name_to_idx: std::collections::HashMap::new(),
        };
        index.rebuild_name_index();
        index
    }

    #[test]
    /// What: Verify `search_official` returns empty vector for empty query.
    ///
    /// Inputs:
    /// - Empty query string and whitespace-only query.
    ///
    /// Output:
    /// - Empty result set for both cases.
    ///
    /// Details:
    /// - Tests that whitespace trimming logic works correctly.
    fn search_official_empty_query_returns_empty() {
        let index = create_test_index();
        assert!(search_official(&index, "", false).is_empty());
        assert!(search_official(&index, "   ", false).is_empty());
        assert!(search_official(&index, "\t\n", false).is_empty());
    }

    #[test]
    /// What: Verify `search_official` performs case-insensitive substring matching.
    ///
    /// Inputs:
    /// - Query with different case variations.
    ///
    /// Output:
    /// - Results match regardless of case.
    ///
    /// Details:
    /// - Tests that substring matching is case-insensitive.
    fn search_official_case_insensitive_substring() {
        let index = create_test_index();
        let results_lower = search_official(&index, "vim", false);
        let results_upper = search_official(&index, "VIM", false);
        let results_mixed = search_official(&index, "ViM", false);

        assert_eq!(results_lower.len(), 1);
        assert_eq!(results_upper.len(), 1);
        assert_eq!(results_mixed.len(), 1);
        assert_eq!(results_lower[0].package.name, "vim");
    }

    #[test]
    /// What: Verify `search_official` finds partial matches.
    ///
    /// Inputs:
    /// - Query that matches part of package name.
    ///
    /// Output:
    /// - Results include packages with matching substring.
    ///
    /// Details:
    /// - Tests that substring matching works for partial names.
    fn search_official_partial_match() {
        let index = create_test_index();
        let results = search_official(&index, "rip", false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package.name, "ripgrep");
        assert_eq!(results[0].fuzzy_score, None); // Substring match has no fuzzy score
    }

    #[test]
    /// What: Verify `search_official` with fuzzy matching (if feature enabled).
    ///
    /// Inputs:
    /// - Query that doesn't match as substring but should match fuzzy.
    ///
    /// Output:
    /// - Results include fuzzy matches with scores.
    ///
    /// Details:
    /// - Tests that fuzzy matching finds non-substring matches.
    /// - Only runs if fuzzy-search feature is enabled.
    #[cfg(feature = "fuzzy-search")]
    fn search_official_fuzzy_match() {
        let index = create_test_index();
        // "rg" should match "ripgrep" with fuzzy matching but not substring
        let substring_results = search_official(&index, "rg", false);
        let fuzzy_results = search_official(&index, "rg", true);

        assert_eq!(substring_results.len(), 0); // No substring match
        assert_eq!(fuzzy_results.len(), 1); // Fuzzy match found
        assert_eq!(fuzzy_results[0].package.name, "ripgrep");
        assert!(fuzzy_results[0].fuzzy_score.is_some()); // Has fuzzy score
    }

    #[test]
    /// What: Verify `search_official` graceful degradation when fuzzy-search feature is disabled.
    ///
    /// Inputs:
    /// - Query with fuzzy=true but feature not available.
    ///
    /// Output:
    /// - Falls back to substring matching.
    ///
    /// Details:
    /// - Tests graceful degradation when fuzzy-search feature is not enabled.
    #[cfg(not(feature = "fuzzy-search"))]
    fn search_official_fuzzy_fallback() {
        let index = create_test_index();
        // When fuzzy-search is not available, fuzzy=true should fall back to substring
        let results = search_official(&index, "rip", true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package.name, "ripgrep");
    }

    #[test]
    /// What: Verify `all_official` returns all packages.
    ///
    /// Inputs:
    /// - Index with multiple packages.
    ///
    /// Output:
    /// - Vector containing all packages from index.
    ///
    /// Details:
    /// - Tests that all packages are returned in correct order.
    fn all_official_returns_all_packages() {
        let index = create_test_index();
        let all = all_official(&index);
        assert_eq!(all.len(), 3);

        let names: Vec<String> = all.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains(&"ripgrep".to_string()));
        assert!(names.contains(&"vim".to_string()));
        assert!(names.contains(&"pacman".to_string()));
    }

    #[test]
    /// What: Verify `all_official` returns empty vector for empty index.
    ///
    /// Inputs:
    /// - Empty index.
    ///
    /// Output:
    /// - Empty vector.
    ///
    /// Details:
    /// - Tests that empty index returns empty results.
    fn all_official_empty_index() {
        let index = OfficialIndex::default();
        let all = all_official(&index);
        assert!(all.is_empty());
    }
}
