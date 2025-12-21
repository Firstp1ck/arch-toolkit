//! PKGBUILD fetching functionality.

use crate::aur::utils::percent_encode;
use crate::client::{is_archlinux_url, rate_limit_archlinux, reset_archlinux_backoff};
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
/// - `client`: HTTP client to use for requests.
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
///
/// # Errors
/// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
/// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
/// - Returns `Err(ArchToolkitError::Parse)` if rate limiter mutex is poisoned
pub async fn pkgbuild(client: &Client, package: &str) -> Result<String> {
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

    // Fetch with timeout
    let response = match client
        .get(&url)
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

    debug!(package = %package, len = text.len(), "PKGBUILD fetched successfully");

    Ok(text)
}
