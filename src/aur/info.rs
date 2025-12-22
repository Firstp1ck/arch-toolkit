//! AUR package info/details functionality.

use crate::aur::utils::{arrs, s, u64_of};
use crate::cache::cache_key_info;
use crate::client::{
    ArchClient, extract_retry_after, is_archlinux_url, rate_limit_archlinux,
    reset_archlinux_backoff, retry_with_policy,
};
use crate::error::{ArchToolkitError, Result};
use crate::types::AurPackageDetails;
use reqwest::Client;
use serde_json::Value;
use tracing::{debug, warn};

/// What: Fetch detailed information for one or more AUR packages.
///
/// Inputs:
/// - `client`: `ArchClient` to use for requests.
/// - `names`: Slice of package names to fetch info for.
///
/// Output:
/// - `Result<Vec<AurPackageDetails>>` containing package details, or an error.
///
/// Details:
/// - Uses AUR RPC v5 info endpoint.
/// - Fetches info for all packages in a single request (more efficient).
/// - Returns empty vector if no packages found (not an error).
/// - Applies rate limiting for archlinux.org requests.
/// - Uses retry policy if enabled for info operations.
/// - Checks cache before making network request if caching is enabled.
///
/// # Errors
/// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
/// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
pub async fn info(client: &ArchClient, names: &[&str]) -> Result<Vec<AurPackageDetails>> {
    if names.is_empty() {
        return Ok(Vec::new());
    }

    // Check cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_info
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_info(names);
        if let Some(cached) = cache.get::<Vec<AurPackageDetails>>(&cache_key) {
            debug!(names = ?names, "cache hit for info");
            return Ok(cached);
        }
    }

    // Build URL with multiple arg parameters using array notation
    // AUR RPC v5 requires arg[]=name1&arg[]=name2 format for multiple packages
    let mut url = String::from("https://aur.archlinux.org/rpc/v5/info?");
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            url.push('&');
        }
        url.push_str("arg[]=");
        url.push_str(name);
    }

    debug!(names = ?names, url = %url, "fetching AUR package info");

    // Apply rate limiting for archlinux.org
    let _permit = if is_archlinux_url(&url) {
        rate_limit_archlinux().await
    } else {
        return Err(ArchToolkitError::InvalidInput(format!(
            "Unexpected URL domain: {url}"
        )));
    };

    let retry_policy = client.retry_policy();
    let http_client = client.http_client();

    // Wrap the request in retry logic if enabled
    let result = if retry_policy.enabled && retry_policy.retry_info {
        retry_with_policy(retry_policy, "info", &names.join(", "), || async {
            perform_info_request(http_client, &url, names).await
        })
        .await
    } else {
        perform_info_request(http_client, &url, names).await
    }?;

    // Store in cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_info
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_info(names);
        let _ = cache.set(&cache_key, &result, cache_config.info_ttl);
    }

    Ok(result)
}

/// What: Perform the actual info request without retry logic.
///
/// Inputs:
/// - `client`: HTTP client to use for requests.
/// - `url`: URL to request.
///
/// Output:
/// - `Result<Vec<AurPackageDetails>>` containing package details, or an error.
///
/// Details:
/// - Internal helper function that performs the HTTP request and parsing
/// - Used by both retry and non-retry code paths
async fn perform_info_request(
    client: &Client,
    url: &str,
    package_names: &[&str],
) -> Result<Vec<AurPackageDetails>> {
    let response = match client.get(url).send().await {
        Ok(resp) => {
            reset_archlinux_backoff();
            resp
        }
        Err(e) => {
            warn!(error = %e, packages = ?package_names, "AUR info request failed");
            return Err(ArchToolkitError::info_failed(package_names, e));
        }
    };

    // Check for Retry-After header before consuming response
    let _retry_after = extract_retry_after(&response);

    let response = match response.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            warn!(error = %e, packages = ?package_names, "AUR info returned non-success status");
            return Err(ArchToolkitError::info_failed(package_names, e));
        }
    };

    let json: Value = match response.json().await {
        Ok(json) => json,
        Err(e) => {
            warn!(error = %e, packages = ?package_names, "failed to parse AUR info JSON");
            // reqwest::Error can contain serde_json::Error, but we'll treat it as network error
            // since the JSON parsing happens inside reqwest
            return Err(ArchToolkitError::info_failed(package_names, e));
        }
    };

    let mut packages = Vec::new();

    if let Some(results) = json.get("results").and_then(Value::as_array) {
        for pkg in results {
            let name = s(pkg, "Name");
            if name.is_empty() {
                continue;
            }

            let version = s(pkg, "Version");
            let description = s(pkg, "Description");
            let url = s(pkg, "URL");

            // Extract arrays
            let licenses = arrs(pkg, &["License", "Licenses"]);
            let groups = arrs(pkg, &["Groups", "Group"]);
            let provides = arrs(pkg, &["Provides"]);
            let depends = arrs(pkg, &["Depends"]);
            let make_depends = arrs(pkg, &["MakeDepends"]);
            let opt_depends = arrs(pkg, &["OptDepends"]);
            let conflicts = arrs(pkg, &["Conflicts"]);
            let replaces = arrs(pkg, &["Replaces"]);

            // Extract maintainer
            let maintainer_str = s(pkg, "Maintainer");
            let maintainer = if maintainer_str.is_empty() {
                None
            } else {
                Some(maintainer_str)
            };

            // Extract timestamps
            let first_submitted = pkg
                .get("FirstSubmitted")
                .and_then(Value::as_i64)
                .filter(|&ts| ts > 0);
            let last_modified = pkg
                .get("LastModified")
                .and_then(Value::as_i64)
                .filter(|&ts| ts > 0);

            // Extract popularity and votes
            let popularity = pkg.get("Popularity").and_then(Value::as_f64);
            let num_votes = u64_of(pkg, &["NumVotes", "Votes"]);

            // Extract out-of-date timestamp
            let out_of_date = pkg
                .get("OutOfDate")
                .and_then(Value::as_i64)
                .and_then(|ts| u64::try_from(ts).ok())
                .filter(|&ts| ts > 0);

            let orphaned = maintainer.is_none();

            packages.push(AurPackageDetails {
                name,
                version,
                description,
                url,
                licenses,
                groups,
                provides,
                depends,
                make_depends,
                opt_depends,
                conflicts,
                replaces,
                maintainer,
                first_submitted,
                last_modified,
                popularity,
                num_votes,
                out_of_date,
                orphaned,
            });
        }
    }

    debug!(found = packages.len(), "AUR info fetch completed");

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArchToolkitError;
    use serde_json::json;

    #[test]
    fn test_info_error_includes_package_context() {
        // Test that InfoFailed error includes the package names
        let packages = &["yay", "paru"];
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
        let error = ArchToolkitError::info_failed(packages, mock_error);
        let error_msg = format!("{error}");
        assert!(
            error_msg.contains("yay"),
            "Error message should include package names: {error_msg}"
        );
        assert!(
            error_msg.contains("paru"),
            "Error message should include all package names: {error_msg}"
        );
        assert!(
            error_msg.contains("AUR info fetch failed"),
            "Error message should indicate info operation: {error_msg}"
        );
    }

    #[test]
    fn test_info_parses_valid_response() {
        let json = json!({
            "results": [
                {
                    "Name": "yay",
                    "Version": "12.3.4-1",
                    "Description": "AUR helper",
                    "URL": "https://github.com/Jguer/yay",
                    "License": ["MIT"],
                    "Groups": [],
                    "Provides": [],
                    "Depends": ["git", "go"],
                    "MakeDepends": ["git"],
                    "OptDepends": ["sudo: privilege escalation"],
                    "Conflicts": [],
                    "Replaces": [],
                    "Maintainer": "someuser",
                    "FirstSubmitted": 1_234_567_890,
                    "LastModified": 1_234_567_891,
                    "Popularity": 3.0,
                    "NumVotes": 100,
                    "OutOfDate": null
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
            let url = s(pkg, "URL");

            let licenses = arrs(pkg, &["License", "Licenses"]);
            let groups = arrs(pkg, &["Groups", "Group"]);
            let provides = arrs(pkg, &["Provides"]);
            let depends = arrs(pkg, &["Depends"]);
            let make_depends = arrs(pkg, &["MakeDepends"]);
            let opt_depends = arrs(pkg, &["OptDepends"]);
            let conflicts = arrs(pkg, &["Conflicts"]);
            let replaces = arrs(pkg, &["Replaces"]);

            let maintainer_str = s(pkg, "Maintainer");
            let maintainer = if maintainer_str.is_empty() {
                None
            } else {
                Some(maintainer_str)
            };

            let first_submitted = pkg
                .get("FirstSubmitted")
                .and_then(Value::as_i64)
                .filter(|&ts| ts > 0);
            let last_modified = pkg
                .get("LastModified")
                .and_then(Value::as_i64)
                .filter(|&ts| ts > 0);

            let popularity = pkg.get("Popularity").and_then(Value::as_f64);
            let num_votes = u64_of(pkg, &["NumVotes", "Votes"]);

            let out_of_date = pkg
                .get("OutOfDate")
                .and_then(Value::as_i64)
                .and_then(|ts| u64::try_from(ts).ok())
                .filter(|&ts| ts > 0);

            let orphaned = maintainer.is_none();

            packages.push(AurPackageDetails {
                name,
                version,
                description,
                url,
                licenses,
                groups,
                provides,
                depends,
                make_depends,
                opt_depends,
                conflicts,
                replaces,
                maintainer,
                first_submitted,
                last_modified,
                popularity,
                num_votes,
                out_of_date,
                orphaned,
            });
        }

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "yay");
        assert_eq!(packages[0].version, "12.3.4-1");
        assert_eq!(packages[0].depends, vec!["git", "go"]);
        assert_eq!(packages[0].opt_depends, vec!["sudo: privilege escalation"]);
        assert_eq!(packages[0].num_votes, Some(100));
    }
}
