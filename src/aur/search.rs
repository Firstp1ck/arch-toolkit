//! AUR search functionality.

use crate::aur::utils::{percent_encode, s};
use crate::cache::cache_key_search;
use crate::client::{
    ArchClient, extract_retry_after, is_archlinux_url, rate_limit_archlinux,
    reset_archlinux_backoff, retry_with_policy,
};
use crate::error::{ArchToolkitError, Result};
use crate::types::AurPackage;
use reqwest::Client;
use serde_json::Value;
use tracing::{debug, warn};

/// What: Search for packages in the AUR by name.
///
/// Inputs:
/// - `client`: `ArchClient` to use for requests.
/// - `query`: Search query string.
///
/// Output:
/// - `Result<Vec<AurPackage>>` containing search results, or an error.
///
/// Details:
/// - Uses AUR RPC v5 search endpoint.
/// - Limits results to 200 packages (AUR default).
/// - Percent-encodes the query string for URL safety.
/// - Applies rate limiting for archlinux.org requests.
/// - Returns empty vector if no results found (not an error).
/// - Uses retry policy if enabled for search operations.
/// - Checks cache before making network request if caching is enabled.
///
/// # Errors
/// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
/// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
pub async fn search(client: &ArchClient, query: &str) -> Result<Vec<AurPackage>> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return Ok(Vec::new());
    }

    // Check cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_search
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_search(trimmed_query);
        if let Some(cached) = cache.get::<Vec<AurPackage>>(&cache_key) {
            debug!(query = trimmed_query, "cache hit for search");
            return Ok(cached);
        }
    }

    let encoded_query = percent_encode(trimmed_query);
    let url = format!("https://aur.archlinux.org/rpc/v5/search?by=name&arg={encoded_query}");

    debug!(query = trimmed_query, url = %url, "searching AUR");

    // Apply rate limiting for archlinux.org
    let _permit = if is_archlinux_url(&url) {
        rate_limit_archlinux().await
    } else {
        // For non-archlinux.org URLs, we don't need rate limiting
        // This shouldn't happen for AUR search, but handle gracefully
        return Err(ArchToolkitError::InvalidInput(format!(
            "Unexpected URL domain: {url}"
        )));
    };

    let retry_policy = client.retry_policy();
    let http_client = client.http_client();

    // Wrap the request in retry logic if enabled
    let result = if retry_policy.enabled && retry_policy.retry_search {
        retry_with_policy(retry_policy, "search", trimmed_query, || async {
            perform_search_request(http_client, &url, trimmed_query).await
        })
        .await
    } else {
        perform_search_request(http_client, &url, trimmed_query).await
    }?;

    // Store in cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_search
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_search(trimmed_query);
        let _ = cache.set(&cache_key, &result, cache_config.search_ttl);
    }

    Ok(result)
}

/// What: Perform the actual search request without retry logic.
///
/// Inputs:
/// - `client`: HTTP client to use for requests.
/// - `url`: URL to request.
/// - `query`: Search query for error context.
///
/// Output:
/// - `Result<Vec<AurPackage>>` containing search results, or an error.
///
/// Details:
/// - Internal helper function that performs the HTTP request and parsing
/// - Used by both retry and non-retry code paths
async fn perform_search_request(
    client: &Client,
    url: &str,
    query: &str,
) -> Result<Vec<AurPackage>> {
    let response = match client.get(url).send().await {
        Ok(resp) => {
            reset_archlinux_backoff();
            resp
        }
        Err(e) => {
            warn!(error = %e, query = %query, "AUR search request failed");
            return Err(ArchToolkitError::search_failed(query, e));
        }
    };

    // Check for Retry-After header before consuming response
    let _retry_after = extract_retry_after(&response);

    let response = match response.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            warn!(error = %e, query = %query, "AUR search returned non-success status");
            // If we have retry_after, we could use it, but error_for_status consumes the response
            // For now, the retry logic will handle exponential backoff
            return Err(ArchToolkitError::search_failed(query, e));
        }
    };

    let json: Value = match response.json().await {
        Ok(json) => json,
        Err(e) => {
            warn!(error = %e, query = %query, "failed to parse AUR search JSON");
            // reqwest::Error can contain serde_json::Error, but we'll treat it as network error
            // since the JSON parsing happens inside reqwest
            return Err(ArchToolkitError::search_failed(query, e));
        }
    };

    let mut packages = Vec::new();

    if let Some(results) = json.get("results").and_then(Value::as_array) {
        for pkg in results.iter().take(200) {
            let name = s(pkg, "Name");
            if name.is_empty() {
                continue;
            }

            let version = s(pkg, "Version");
            let description = s(pkg, "Description");
            let popularity = pkg.get("Popularity").and_then(Value::as_f64);

            // Extract OutOfDate timestamp (i64 or null)
            let out_of_date = pkg
                .get("OutOfDate")
                .and_then(Value::as_i64)
                .and_then(|ts| u64::try_from(ts).ok())
                .filter(|&ts| ts > 0);

            // Extract Maintainer and determine if orphaned (empty or null means orphaned)
            let maintainer_str = s(pkg, "Maintainer");
            let maintainer = if maintainer_str.is_empty() {
                None
            } else {
                Some(maintainer_str)
            };
            let orphaned = maintainer.is_none();

            packages.push(AurPackage {
                name,
                version,
                description,
                popularity,
                out_of_date,
                orphaned,
                maintainer,
            });
        }
    }

    debug!(count = packages.len(), "AUR search completed");

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArchToolkitError;
    use serde_json::json;

    #[test]
    fn test_search_error_includes_query_context() {
        // Test that SearchFailed error includes the query
        let query = "test-package";
        // Create a reqwest::Error by using an invalid CA certificate
        // This is safe in tests as we're intentionally creating an error
        #[allow(clippy::unwrap_used)]
        let cert_result = reqwest::Certificate::from_pem(b"invalid cert");
        let mock_error = match cert_result {
            Ok(cert) => reqwest::Client::builder()
                .add_root_certificate(cert)
                .build()
                .expect_err("Should fail to build client with invalid cert"),
            Err(e) => e,
        };
        let error = ArchToolkitError::search_failed(query, mock_error);
        let error_msg = format!("{error}");
        assert!(
            error_msg.contains(query),
            "Error message should include query: {error_msg}"
        );
        assert!(
            error_msg.contains("AUR search failed"),
            "Error message should indicate search operation: {error_msg}"
        );
    }

    #[test]
    fn test_search_parses_valid_response() {
        let json = json!({
            "results": [
                {
                    "Name": "yay",
                    "Version": "12.3.4",
                    "Description": "AUR helper",
                    "Popularity": 3.0,
                    "OutOfDate": null,
                    "Maintainer": "someuser"
                },
                {
                    "Name": "paru",
                    "Version": "1.2.3",
                    "Description": "Another AUR helper",
                    "Popularity": 2.5,
                    "OutOfDate": 1_234_567_890,
                    "Maintainer": ""
                }
            ]
        });

        let results = json
            .get("results")
            .and_then(Value::as_array)
            .expect("test JSON should have results array");
        let mut packages = Vec::new();

        for pkg in results {
            let name = s(pkg, "Name");
            if name.is_empty() {
                continue;
            }

            let version = s(pkg, "Version");
            let description = s(pkg, "Description");
            let popularity = pkg.get("Popularity").and_then(Value::as_f64);

            let out_of_date = pkg
                .get("OutOfDate")
                .and_then(Value::as_i64)
                .and_then(|ts| u64::try_from(ts).ok())
                .filter(|&ts| ts > 0);

            let maintainer_str = s(pkg, "Maintainer");
            let maintainer = if maintainer_str.is_empty() {
                None
            } else {
                Some(maintainer_str)
            };
            let orphaned = maintainer.is_none();

            packages.push(AurPackage {
                name,
                version,
                description,
                popularity,
                out_of_date,
                orphaned,
                maintainer,
            });
        }

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "yay");
        assert_eq!(packages[0].version, "12.3.4");
        assert!(!packages[0].orphaned);
        assert_eq!(packages[1].name, "paru");
        assert!(packages[1].orphaned);
        assert_eq!(packages[1].out_of_date, Some(1_234_567_890));
    }
}
