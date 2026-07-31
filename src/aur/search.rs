//! AUR search functionality.

use crate::aur::utils::{percent_encode, s};
use crate::aur::validation::validate_search_query;
use crate::cache::cache_key_search;
use crate::client::{
    ArchClient, extract_retry_after, is_archlinux_url, rate_limit_archlinux,
    reset_archlinux_backoff, retry_with_policy,
};
use crate::error::{ArchToolkitError, Result};
use crate::types::AurPackage;
use std::num::NonZeroUsize;

use reqwest::Client;
use serde_json::Value;
use tracing::{debug, warn};

/// Maximum bytes accepted from one AUR RPC search response.
const MAX_AUR_SEARCH_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

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
/// - Returns every package row supplied by the single AUR RPC response; it
///   neither assumes an upstream result cap nor requests unsupported pagination.
/// - Use [`search_with_limit`] for an explicit caller-selected client-side cap.
/// - Percent-encodes the query string for URL safety.
/// - Applies rate limiting for archlinux.org requests.
/// - Returns empty vector if no results found (not an error).
/// - Uses retry policy if enabled for search operations.
/// - Checks cache before making network request if caching is enabled.
///
/// # Errors
/// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
/// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
/// - Returns `Err(ArchToolkitError::EmptyInput)` if query is empty and strict mode is enabled
/// - Returns `Err(ArchToolkitError::InputTooLong)` if query exceeds maximum length
pub async fn search(client: &ArchClient, query: &str) -> Result<Vec<AurPackage>> {
    // Validate input
    let validation_config = client.validation_config();
    let trimmed_query = validate_search_query(query, Some(validation_config))?;

    // In lenient mode, empty queries return empty results
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

/// What: Search AUR packages with an explicit client-side result limit.
///
/// Inputs:
/// - `client`: `ArchClient` used for the normal AUR RPC request.
/// - `query`: Search query string.
/// - `maximum_results`: Non-zero maximum number of rows returned to the caller.
///
/// Output:
/// - AUR search rows in RPC order, truncated to `maximum_results` after fetch.
///
/// Details:
/// - Delegates to [`search`] so validation, retry, rate limiting, and caching
///   are identical to an uncapped request.
/// - The cap is entirely local. It does not infer an upstream numeric cap,
///   invent pagination, or make cache entries depend on a caller's limit.
///
/// # Errors
/// - Returns the same errors as [`search`].
pub async fn search_with_limit(
    client: &ArchClient,
    query: &str,
    maximum_results: NonZeroUsize,
) -> Result<Vec<AurPackage>> {
    let packages = search(client, query).await?;
    Ok(limit_search_results(packages, maximum_results))
}

/// What: Apply an explicit caller-selected cap to fetched search rows.
///
/// Inputs:
/// - `packages`: Complete parsed result array from one AUR RPC response.
/// - `maximum_results`: Non-zero maximum number of rows to retain.
///
/// Output:
/// - The first `maximum_results` rows in existing response order.
///
/// Details:
/// - This helper has no network or cache side effects, making the cap semantics
///   deterministic and independent from unsupported server pagination.
fn limit_search_results(
    mut packages: Vec<AurPackage>,
    maximum_results: NonZeroUsize,
) -> Vec<AurPackage> {
    packages.truncate(maximum_results.get());
    packages
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

    let json = read_bounded_search_json(response, query).await?;

    let mut packages = Vec::new();

    if let Some(results) = json.get("results").and_then(Value::as_array) {
        for pkg in results {
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

/// What: Read and parse one bounded AUR RPC search response.
///
/// Inputs:
/// - `response`: Successful HTTP response.
/// - `query`: Search query retained for actionable parse/transport context.
///
/// Output:
/// - Parsed JSON document within [`MAX_AUR_SEARCH_RESPONSE_BYTES`].
///
/// Details:
/// - Checks both `Content-Length` and streamed chunks so omitted or inaccurate headers cannot
///   bypass the memory bound. The result array remains uncapped; only response bytes are bounded.
async fn read_bounded_search_json(mut response: reqwest::Response, query: &str) -> Result<Value> {
    if let Some(length) = response.content_length()
        && length > MAX_AUR_SEARCH_RESPONSE_BYTES as u64
    {
        return Err(ArchToolkitError::InputTooLong {
            field: "aur_search_response".to_string(),
            max_length: MAX_AUR_SEARCH_RESPONSE_BYTES,
            actual_length: usize::try_from(length).unwrap_or(usize::MAX),
        });
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ArchToolkitError::search_failed(query, error))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_AUR_SEARCH_RESPONSE_BYTES {
            return Err(ArchToolkitError::InputTooLong {
                field: "aur_search_response".to_string(),
                max_length: MAX_AUR_SEARCH_RESPONSE_BYTES,
                actual_length: body.len().saturating_add(chunk.len()),
            });
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|error| {
        ArchToolkitError::Parse(format!(
            "failed to parse AUR search JSON for '{query}': {error}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArchToolkitError;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_search_error_includes_query_context() {
        // Test that SearchFailed error includes the query
        let query = "test-package";
        let mock_error = crate::aur::utils::mock_reqwest_error();
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
    /// What: Verify result limits are client-selected rather than assumed upstream caps.
    ///
    /// Inputs:
    /// - Three already-parsed AUR package rows and a limit of two.
    ///
    /// Output:
    /// - The first two rows in response order.
    ///
    /// Details:
    /// - The pure helper proves result limiting neither fetches a second page nor
    ///   changes the uncapped parser behavior.
    fn search_limit_is_explicit_and_ordered() {
        let package = |name: &str| AurPackage {
            name: name.to_string(),
            version: "1.0".to_string(),
            description: String::new(),
            popularity: None,
            out_of_date: None,
            orphaned: false,
            maintainer: None,
        };
        let limited = limit_search_results(
            vec![package("first"), package("second"), package("third")],
            NonZeroUsize::new(2).expect("non-zero test limit"),
        );

        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].name, "first");
        assert_eq!(limited[1].name, "second");
    }

    #[tokio::test]
    /// What: Verify oversized AUR search responses are rejected before JSON parsing.
    ///
    /// Inputs:
    /// - A deterministic local HTTP response one byte above the 4 MiB limit.
    ///
    /// Output:
    /// - An [`ArchToolkitError::InputTooLong`] with the configured response bound.
    ///
    /// Details:
    /// - Uses wiremock only; no live AUR endpoint or external state is involved.
    /// - Directly exercises the bounded response reader used by every search request.
    async fn oversized_search_response_is_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rpc/v5/search"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![
                b'x';
                MAX_AUR_SEARCH_RESPONSE_BYTES
                    + 1
            ]))
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .get(format!("{}/rpc/v5/search", server.uri()))
            .send()
            .await
            .expect("local oversized search response");
        let error = read_bounded_search_json(response, "fixture")
            .await
            .expect_err("oversized response must fail");

        assert!(matches!(
            error,
            ArchToolkitError::InputTooLong {
                max_length: MAX_AUR_SEARCH_RESPONSE_BYTES,
                actual_length,
                ..
            } if actual_length > MAX_AUR_SEARCH_RESPONSE_BYTES
        ));
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
