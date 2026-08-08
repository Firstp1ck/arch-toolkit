//! PKGBUILD fetching functionality.

use crate::aur::utils::percent_encode;
use crate::aur::validation::validate_package_name;
use crate::cache::cache_key_pkgbuild;
use crate::client::{
    ArchClient, extract_retry_after, is_archlinux_url, rate_limit_archlinux,
    reset_archlinux_backoff, retry_with_policy,
};
use crate::error::{ArchToolkitError, Result};
use reqwest::Client;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::debug;

/// Rate limiter for PKGBUILD requests to avoid overwhelming AUR servers.
///
/// Tracks the timestamp of the last PKGBUILD request to enforce minimum intervals.
static PKGBUILD_RATE_LIMITER: Mutex<Option<Instant>> = Mutex::new(None);
/// Minimum interval between PKGBUILD requests in milliseconds.
const PKGBUILD_MIN_INTERVAL_MS: u64 = 200;
/// Maximum accepted PKGBUILD response body size in bytes.
const MAX_AUR_PKGBUILD_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

/// What: Fetch PKGBUILD content for an AUR package.
///
/// Inputs:
/// - `client`: `ArchClient` to use for requests.
/// - `package`: Package name to fetch PKGBUILD for.
///
/// Output:
/// - `Result<String>` with PKGBUILD text when available; `Err` on network or lookup failure.
///
/// Details:
/// - Fetches from `https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={package}`
/// - Applies rate limiting (200ms minimum interval between requests)
/// - Uses timeout (10 seconds)
/// - Returns raw PKGBUILD text
/// - Uses retry policy if enabled for pkgbuild operations.
/// - Checks cache before making network request if caching is enabled.
///
/// # Errors
/// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
/// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
/// - Returns `Err(ArchToolkitError::Parse)` if rate limiter mutex is poisoned
/// - Returns `Err(ArchToolkitError::EmptyInput)` if package name is empty and strict mode is enabled
/// - Returns `Err(ArchToolkitError::InvalidPackageName)` if package name is invalid
/// - Returns `Err(ArchToolkitError::InputTooLong)` if package name exceeds maximum length
pub async fn pkgbuild(client: &ArchClient, package: &str) -> Result<String> {
    // Validate input
    let validation_config = client.validation_config();
    validate_package_name(package, Some(validation_config))?;
    // Check cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_pkgbuild
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_pkgbuild(package);
        if let Some(cached) = cache.get::<String>(&cache_key) {
            debug!(package = %package, "cache hit for pkgbuild");
            return Ok(cached);
        }
    }

    let url = format!(
        "https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}",
        percent_encode(package)
    );

    debug!(package = %package, url = %url, "fetching PKGBUILD");

    // Rate limiting: ensure minimum interval between requests
    let delay = {
        let mut last_request = PKGBUILD_RATE_LIMITER.lock().map_err(|_| {
            ArchToolkitError::Parse("PKGBUILD rate limiter mutex poisoned".to_string())
        })?;
        if let Some(last) = *last_request {
            let elapsed = last.elapsed();
            if elapsed < Duration::from_millis(PKGBUILD_MIN_INTERVAL_MS) {
                let delay = Duration::from_millis(PKGBUILD_MIN_INTERVAL_MS)
                    .checked_sub(elapsed)
                    .ok_or_else(|| {
                        ArchToolkitError::Parse("Invalid delay calculation".to_string())
                    })?;
                debug!(
                    package = %package,
                    delay_ms = delay.as_millis(),
                    "Rate limiting PKGBUILD request"
                );
                *last_request = Some(Instant::now());
                Some(delay)
            } else {
                *last_request = Some(Instant::now());
                None
            }
        } else {
            *last_request = Some(Instant::now());
            None
        }
    };
    if let Some(delay) = delay {
        tokio::time::sleep(delay).await;
    }

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
    let text = if retry_policy.enabled && retry_policy.retry_pkgbuild {
        retry_with_policy(retry_policy, "pkgbuild", package, || async {
            perform_pkgbuild_request(http_client, &url, package).await
        })
        .await?
    } else {
        perform_pkgbuild_request(http_client, &url, package).await?
    };

    debug!(package = %package, len = text.len(), "PKGBUILD fetched successfully");

    // Store in cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_pkgbuild
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_pkgbuild(package);
        let _ = cache.set(&cache_key, &text, cache_config.pkgbuild_ttl);
    }

    Ok(text)
}

/// What: Perform the actual PKGBUILD request without retry logic.
///
/// Inputs:
/// - `client`: HTTP client to use for requests.
/// - `url`: URL to request.
/// - `package`: Package name retained in every operation error.
///
/// Output:
/// - `Result<String>` containing PKGBUILD text, or an error.
///
/// Details:
/// - Internal helper function that performs the HTTP request
/// - Used by both retry and non-retry code paths
async fn perform_pkgbuild_request(client: &Client, url: &str, package: &str) -> Result<String> {
    // Fetch with timeout
    let response = match client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            reset_archlinux_backoff();
            resp
        }
        Err(e) => {
            debug!(error = %e, package = %package, "PKGBUILD request failed");
            return Err(ArchToolkitError::pkgbuild_failed(package, e));
        }
    };

    // Check for Retry-After header before consuming response
    let _retry_after = extract_retry_after(&response);

    let response = match response.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            debug!(error = %e, package = %package, "PKGBUILD returned non-success status");
            return Err(ArchToolkitError::pkgbuild_failed(package, e));
        }
    };

    let resource_label = format!("AUR PKGBUILD for package '{package}'");
    crate::http::read_bounded_response_text(
        response,
        MAX_AUR_PKGBUILD_RESPONSE_BYTES,
        &resource_label,
        |error| ArchToolkitError::pkgbuild_failed(package, error),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArchToolkitError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_pkgbuild_error_includes_package_context() {
        // Test that PkgbuildFailed error includes the package name
        let package = "yay";
        let mock_error = crate::aur::utils::mock_reqwest_error();
        let error = ArchToolkitError::pkgbuild_failed(package, mock_error);
        let error_msg = format!("{error}");
        assert!(
            error_msg.contains(package),
            "Error message should include package name: {error_msg}"
        );
        assert!(
            error_msg.contains("PKGBUILD fetch failed"),
            "Error message should indicate pkgbuild operation: {error_msg}"
        );
    }

    #[tokio::test]
    /// What: Verify an oversized PKGBUILD response is rejected while reading.
    ///
    /// Inputs:
    /// - A local response one byte above the approved 10 MiB ceiling.
    ///
    /// Output:
    /// - `InputTooLong` retaining PKGBUILD-operation and package context.
    ///
    /// Details:
    /// - The response is inert bytes and is never sourced or executed.
    async fn oversized_aur_pkgbuild_response_is_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/PKGBUILD"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![
                b'x';
                MAX_AUR_PKGBUILD_RESPONSE_BYTES
                    + 1
            ]))
            .mount(&server)
            .await;

        let error =
            perform_pkgbuild_request(&Client::new(), &format!("{}/PKGBUILD", server.uri()), "yay")
                .await
                .expect_err("oversized PKGBUILD response must fail");
        let message = error.to_string();

        assert!(matches!(
            error,
            ArchToolkitError::InputTooLong {
                max_length: MAX_AUR_PKGBUILD_RESPONSE_BYTES,
                ..
            }
        ));
        assert!(message.contains("PKGBUILD"));
        assert!(message.contains("yay"));
    }

    #[tokio::test]
    /// What: Preserve PKGBUILD operation context for status and UTF-8 failures.
    ///
    /// Inputs:
    /// - Local HTTP 404 and invalid UTF-8 responses for package `yay`.
    ///
    /// Output:
    /// - Contextual errors identifying PKGBUILD and the package.
    ///
    /// Details:
    /// - Neither response body is logged, interpreted, sourced, or executed.
    async fn aur_pkgbuild_status_and_utf8_errors_are_contextual() {
        for (path_value, template) in [
            ("/status", ResponseTemplate::new(404)),
            (
                "/utf8",
                ResponseTemplate::new(200).set_body_bytes([0xf0, 0x28, 0x8c]),
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path(path_value))
                .respond_with(template)
                .mount(&server)
                .await;

            let error = perform_pkgbuild_request(
                &Client::new(),
                &format!("{}{path_value}", server.uri()),
                "yay",
            )
            .await
            .expect_err("invalid PKGBUILD response must fail");
            let message = error.to_string();

            assert!(message.contains("PKGBUILD"));
            assert!(message.contains("yay"));
        }
    }

    #[tokio::test]
    /// What: Return a normal bounded PKGBUILD fixture unchanged.
    ///
    /// Inputs:
    /// - An inert local PKGBUILD string.
    ///
    /// Output:
    /// - Exact UTF-8 content returned to the caller.
    ///
    /// Details:
    /// - Fetching remains data-only and does not evaluate shell text.
    async fn normal_aur_pkgbuild_fixture_is_read() {
        let server = MockServer::start().await;
        let pkgbuild = "pkgname=yay\npkgver=1\n";
        Mock::given(method("GET"))
            .and(path("/PKGBUILD"))
            .respond_with(ResponseTemplate::new(200).set_body_string(pkgbuild))
            .mount(&server)
            .await;

        let body =
            perform_pkgbuild_request(&Client::new(), &format!("{}/PKGBUILD", server.uri()), "yay")
                .await
                .expect("normal PKGBUILD fixture");

        assert_eq!(body, pkgbuild);
    }
}
