//! PKGBUILD fetching functionality.

use crate::aur::utils::percent_encode;
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
pub async fn pkgbuild(client: &ArchClient, package: &str) -> Result<String> {
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
        retry_with_policy(retry_policy, "pkgbuild", || async {
            perform_pkgbuild_request(http_client, &url).await
        })
        .await?
    } else {
        perform_pkgbuild_request(http_client, &url).await?
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
///
/// Output:
/// - `Result<String>` containing PKGBUILD text, or an error.
///
/// Details:
/// - Internal helper function that performs the HTTP request
/// - Used by both retry and non-retry code paths
async fn perform_pkgbuild_request(client: &Client, url: &str) -> Result<String> {
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
            debug!(error = %e, "PKGBUILD request failed");
            return Err(ArchToolkitError::Network(e));
        }
    };

    // Check for Retry-After header before consuming response
    let _retry_after = extract_retry_after(&response);

    let response = match response.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            debug!(error = %e, "PKGBUILD returned non-success status");
            return Err(ArchToolkitError::Network(e));
        }
    };

    let text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            debug!(error = %e, "failed to read PKGBUILD response");
            return Err(ArchToolkitError::Network(e));
        }
    };

    Ok(text)
}
