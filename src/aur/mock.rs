//! Mock implementation of `AurApi` for testing purposes.

use super::traits::AurApi;
use crate::error::{ArchToolkitError, Result};
use crate::types::{AurComment, AurPackage, AurPackageDetails};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// What: Mock implementation of `AurApi` for testing.
///
/// Inputs: None (created via `MockAurApi::new()` or builder methods)
///
/// Output:
/// - `MockAurApi` instance that can be configured with predefined responses
///
/// Details:
/// - Allows setting predefined results for each operation type
/// - Supports both success and error responses
/// - Thread-safe via `Arc<Mutex<>>` for internal state
/// - Builder pattern for easy configuration
/// - Useful for unit testing without hitting real AUR endpoints
#[derive(Debug)]
pub struct MockAurApi {
    /// Predefined search results, keyed by query string.
    search_results: Arc<Mutex<HashMap<String, Result<Vec<AurPackage>>>>>,
    /// Predefined info results, keyed by sorted package names (comma-separated).
    info_results: Arc<Mutex<HashMap<String, Result<Vec<AurPackageDetails>>>>>,
    /// Predefined comments results, keyed by package name.
    comments_results: Arc<Mutex<HashMap<String, Result<Vec<AurComment>>>>>,
    /// Predefined pkgbuild results, keyed by package name.
    pkgbuild_results: Arc<Mutex<HashMap<String, Result<String>>>>,
    /// Default search result if no specific query match is found.
    default_search_result: Option<Result<Vec<AurPackage>>>,
    /// Default info result if no specific package match is found.
    default_info_result: Option<Result<Vec<AurPackageDetails>>>,
    /// Default comments result if no specific package match is found.
    default_comments_result: Option<Result<Vec<AurComment>>>,
    /// Default pkgbuild result if no specific package match is found.
    default_pkgbuild_result: Option<Result<String>>,
}

impl Default for MockAurApi {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAurApi {
    /// What: Clone a Result containing clonable success values.
    ///
    /// Inputs:
    /// - `result`: Result to clone
    ///
    /// Output:
    /// - Cloned result (errors converted to Parse errors if not clonable)
    ///
    /// Details:
    /// - Clones Ok values directly
    /// - Converts non-clonable errors (Network) to Parse errors
    fn clone_result<T: Clone>(result: &Result<T>) -> Result<T> {
        match result {
            Ok(ok) => Ok(ok.clone()),
            Err(e) => Err(match e {
                ArchToolkitError::Network(_) => {
                    ArchToolkitError::Parse("Mock network error".to_string())
                }
                ArchToolkitError::Json(_) => {
                    ArchToolkitError::Parse("Mock JSON error".to_string())
                }
                ArchToolkitError::Parse(s) => ArchToolkitError::Parse(s.clone()),
                ArchToolkitError::RateLimited { retry_after } => {
                    ArchToolkitError::RateLimited {
                        retry_after: *retry_after,
                    }
                }
                ArchToolkitError::NotFound => ArchToolkitError::NotFound,
                ArchToolkitError::InvalidInput(s) => {
                    ArchToolkitError::InvalidInput(s.clone())
                }
            }),
        }
    }

    /// What: Create a new `MockAurApi` with empty configuration.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `MockAurApi` instance ready for configuration
    ///
    /// Details:
    /// - Starts with no predefined results
    /// - Use builder methods to configure responses
    #[must_use]
    pub fn new() -> Self {
        Self {
            search_results: Arc::new(Mutex::new(HashMap::new())),
            info_results: Arc::new(Mutex::new(HashMap::new())),
            comments_results: Arc::new(Mutex::new(HashMap::new())),
            pkgbuild_results: Arc::new(Mutex::new(HashMap::new())),
            default_search_result: None,
            default_info_result: None,
            default_comments_result: None,
            default_pkgbuild_result: None,
        }
    }

    /// What: Set a search result for a specific query.
    ///
    /// Inputs:
    /// - `query`: Query string to match
    /// - `results`: Result containing search results
    ///
    /// Output:
    /// - `Self` for method chaining
    ///
    /// Details:
    /// - Stores the result for the exact query string
    /// - Overwrites any existing result for this query
    ///
    /// # Panics
    /// - Panics if the internal mutex is poisoned (should never happen in practice)
    #[must_use]
    pub fn with_search_result(self, query: &str, result: Result<Vec<AurPackage>>) -> Self {
        {
            let mut results = self
                .search_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.insert(query.to_string(), result);
        }
        self
    }

    /// What: Set a default search result for queries without specific matches.
    ///
    /// Inputs:
    /// - `result`: Default result to return
    ///
    /// Output:
    /// - `Self` for method chaining
    ///
    /// Details:
    /// - Used when a query doesn't have a specific match
    /// - If not set, returns an error for unmatched queries
    #[must_use]
    pub fn with_default_search_result(self, result: Result<Vec<AurPackage>>) -> Self {
        Self {
            default_search_result: Some(result),
            ..self
        }
    }

    /// What: Set an info result for specific package names.
    ///
    /// Inputs:
    /// - `names`: Slice of package names
    /// - `result`: Result containing package details
    ///
    /// Output:
    /// - `Self` for method chaining
    ///
    /// Details:
    /// - Stores the result keyed by sorted, comma-separated package names
    /// - Overwrites any existing result for these packages
    ///
    /// # Panics
    /// - Panics if the internal mutex is poisoned (should never happen in practice)
    #[must_use]
    pub fn with_info_result(
        self,
        names: &[&str],
        result: Result<Vec<AurPackageDetails>>,
    ) -> Self {
        let mut sorted_names = names.to_vec();
        sorted_names.sort_unstable();
        let key = sorted_names.join(",");
        {
            let mut results = self
                .info_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.insert(key, result);
        }
        self
    }

    /// What: Set a default info result for packages without specific matches.
    ///
    /// Inputs:
    /// - `result`: Default result to return
    ///
    /// Output:
    /// - `Self` for method chaining
    #[must_use]
    pub fn with_default_info_result(self, result: Result<Vec<AurPackageDetails>>) -> Self {
        Self {
            default_info_result: Some(result),
            ..self
        }
    }

    /// What: Set a comments result for a specific package.
    ///
    /// Inputs:
    /// - `pkgname`: Package name
    /// - `result`: Result containing comments
    ///
    /// Output:
    /// - `Self` for method chaining
    ///
    /// # Panics
    /// - Panics if the internal mutex is poisoned (should never happen in practice)
    #[must_use]
    pub fn with_comments_result(self, pkgname: &str, result: Result<Vec<AurComment>>) -> Self {
        {
            let mut results = self
                .comments_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.insert(pkgname.to_string(), result);
        }
        self
    }

    /// What: Set a default comments result for packages without specific matches.
    ///
    /// Inputs:
    /// - `result`: Default result to return
    ///
    /// Output:
    /// - `Self` for method chaining
    #[must_use]
    pub fn with_default_comments_result(self, result: Result<Vec<AurComment>>) -> Self {
        Self {
            default_comments_result: Some(result),
            ..self
        }
    }

    /// What: Set a pkgbuild result for a specific package.
    ///
    /// Inputs:
    /// - `package`: Package name
    /// - `result`: Result containing PKGBUILD content
    ///
    /// Output:
    /// - `Self` for method chaining
    ///
    /// # Panics
    /// - Panics if the internal mutex is poisoned (should never happen in practice)
    #[must_use]
    pub fn with_pkgbuild_result(self, package: &str, result: Result<String>) -> Self {
        {
            let mut results = self
                .pkgbuild_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.insert(package.to_string(), result);
        }
        self
    }

    /// What: Set a default pkgbuild result for packages without specific matches.
    ///
    /// Inputs:
    /// - `result`: Default result to return
    ///
    /// Output:
    /// - `Self` for method chaining
    #[must_use]
    pub fn with_default_pkgbuild_result(self, result: Result<String>) -> Self {
        Self {
            default_pkgbuild_result: Some(result),
            ..self
        }
    }
}

#[async_trait]
impl AurApi for MockAurApi {
    /// What: Search for packages in the AUR by name (mock implementation).
    ///
    /// Inputs:
    /// - `query`: Search query string
    ///
    /// Output:
    /// - `Result<Vec<AurPackage>>` containing predefined search results, or an error
    ///
    /// Details:
    /// - Returns predefined result for the query if available
    /// - Falls back to default search result if set
    /// - Returns error if no match found and no default is set
    async fn search(&self, query: &str) -> Result<Vec<AurPackage>> {
        let result = {
            let results = self
                .search_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.get(query).map(Self::clone_result)
        };

        if let Some(result) = result {
            return result;
        }

        if let Some(ref default) = self.default_search_result {
            return Self::clone_result(default);
        }

        Err(ArchToolkitError::Parse(format!(
            "MockAurApi: No search result configured for query '{query}'"
        )))
    }

    /// What: Fetch detailed information for one or more AUR packages (mock implementation).
    ///
    /// Inputs:
    /// - `names`: Slice of package names to fetch info for
    ///
    /// Output:
    /// - `Result<Vec<AurPackageDetails>>` containing predefined package details, or an error
    ///
    /// Details:
    /// - Returns predefined result for the sorted package names if available
    /// - Falls back to default info result if set
    /// - Returns error if no match found and no default is set
    async fn info(&self, names: &[&str]) -> Result<Vec<AurPackageDetails>> {
        let mut sorted_names = names.to_vec();
        sorted_names.sort_unstable();
        let key = sorted_names.join(",");

        let result = {
            let results = self
                .info_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.get(&key).map(Self::clone_result)
        };

        if let Some(result) = result {
            return result;
        }

        if let Some(ref default) = self.default_info_result {
            return Self::clone_result(default);
        }

        Err(ArchToolkitError::Parse(format!(
            "MockAurApi: No info result configured for packages '{key}'"
        )))
    }

    /// What: Fetch AUR package comments (mock implementation).
    ///
    /// Inputs:
    /// - `pkgname`: Package name to fetch comments for
    ///
    /// Output:
    /// - `Result<Vec<AurComment>>` containing predefined comments, or an error
    ///
    /// Details:
    /// - Returns predefined result for the package if available
    /// - Falls back to default comments result if set
    /// - Returns error if no match found and no default is set
    async fn comments(&self, pkgname: &str) -> Result<Vec<AurComment>> {
        let result = {
            let results = self
                .comments_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.get(pkgname).map(Self::clone_result)
        };

        if let Some(result) = result {
            return result;
        }

        if let Some(ref default) = self.default_comments_result {
            return Self::clone_result(default);
        }

        Err(ArchToolkitError::Parse(format!(
            "MockAurApi: No comments result configured for package '{pkgname}'"
        )))
    }

    /// What: Fetch PKGBUILD content for an AUR package (mock implementation).
    ///
    /// Inputs:
    /// - `package`: Package name to fetch PKGBUILD for
    ///
    /// Output:
    /// - `Result<String>` containing predefined PKGBUILD content, or an error
    ///
    /// Details:
    /// - Returns predefined result for the package if available
    /// - Falls back to default pkgbuild result if set
    /// - Returns error if no match found and no default is set
    async fn pkgbuild(&self, package: &str) -> Result<String> {
        let result = {
            let results = self
                .pkgbuild_results
                .lock()
                .expect("MockAurApi mutex should not be poisoned");
            results.get(package).map(Self::clone_result)
        };

        if let Some(result) = result {
            return result;
        }

        if let Some(ref default) = self.default_pkgbuild_result {
            return Self::clone_result(default);
        }

        Err(ArchToolkitError::Parse(format!(
            "MockAurApi: No pkgbuild result configured for package '{package}'"
        )))
    }
}

#[cfg(test)]
mod tests {
    // Allow unwrap in tests - these are intentional panics for test failures
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::types::{AurComment, AurPackage, AurPackageDetails};

    #[tokio::test]
    async fn test_mock_search_success() {
        let mock = MockAurApi::new().with_search_result(
            "yay",
            Ok(vec![AurPackage {
                name: "yay".to_string(),
                version: "12.0.0".to_string(),
                description: "AUR helper".to_string(),
                popularity: Some(100.0),
                out_of_date: None,
                orphaned: false,
                maintainer: Some("user".to_string()),
            }]),
        );

        let result = mock.search("yay").await;
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "yay");
    }

    #[tokio::test]
    async fn test_mock_search_error() {
        let mock = MockAurApi::new().with_search_result(
            "error",
            Err(ArchToolkitError::Parse("test error".to_string())),
        );

        let result = mock.search("error").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_search_not_found() {
        let mock = MockAurApi::new();
        let result = mock.search("unknown").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No search result configured")
        );
    }

    #[tokio::test]
    async fn test_mock_search_default() {
        let mock = MockAurApi::new().with_default_search_result(Ok(vec![AurPackage {
            name: "default".to_string(),
            version: "1.0.0".to_string(),
            description: "Default package".to_string(),
            popularity: None,
            out_of_date: None,
            orphaned: false,
            maintainer: None,
        }]));

        let result = mock.search("any-query").await;
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "default");
    }

    #[tokio::test]
    async fn test_mock_info_success() {
        let mock = MockAurApi::new().with_info_result(
            &["yay"],
            Ok(vec![AurPackageDetails {
                name: "yay".to_string(),
                version: "12.0.0".to_string(),
                description: "AUR helper".to_string(),
                url: "https://github.com/Jguer/yay".to_string(),
                licenses: vec!["MIT".to_string()],
                groups: vec![],
                provides: vec![],
                depends: vec![],
                make_depends: vec![],
                opt_depends: vec![],
                conflicts: vec![],
                replaces: vec![],
                maintainer: Some("user".to_string()),
                first_submitted: None,
                last_modified: None,
                popularity: Some(100.0),
                num_votes: Some(1000),
                out_of_date: None,
                orphaned: false,
            }]),
        );

        let result = mock.info(&["yay"]).await;
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "yay");
    }

    #[tokio::test]
    async fn test_mock_info_sorted() {
        let mock = MockAurApi::new().with_info_result(
            &["yay", "paru"],
            Ok(vec![AurPackageDetails {
                name: "yay".to_string(),
                version: "12.0.0".to_string(),
                description: "AUR helper".to_string(),
                url: String::new(),
                licenses: vec![],
                groups: vec![],
                provides: vec![],
                depends: vec![],
                make_depends: vec![],
                opt_depends: vec![],
                conflicts: vec![],
                replaces: vec![],
                maintainer: None,
                first_submitted: None,
                last_modified: None,
                popularity: None,
                num_votes: None,
                out_of_date: None,
                orphaned: false,
            }]),
        );

        // Should work with different order
        let result1 = mock.info(&["yay", "paru"]).await;
        assert!(result1.is_ok());

        let result2 = mock.info(&["paru", "yay"]).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_mock_comments_success() {
        let mock = MockAurApi::new().with_comments_result(
            "yay",
            Ok(vec![AurComment {
                id: Some("1".to_string()),
                author: "user".to_string(),
                date: "2024-01-01".to_string(),
                date_timestamp: Some(1_704_067_200),
                date_url: None,
                content: "Great package!".to_string(),
                pinned: false,
            }]),
        );

        let result = mock.comments("yay").await;
        assert!(result.is_ok());
        let comments = result.unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].author, "user");
    }

    #[tokio::test]
    async fn test_mock_pkgbuild_success() {
        let mock = MockAurApi::new()
            .with_pkgbuild_result("yay", Ok("pkgname=yay\npkgver=12.0.0".to_string()));

        let result = mock.pkgbuild("yay").await;
        assert!(result.is_ok());
        let pkgbuild = result.unwrap();
        assert!(pkgbuild.contains("yay"));
    }
}
