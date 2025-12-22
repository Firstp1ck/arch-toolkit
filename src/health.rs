//! Health check functionality for archlinux.org services.

use crate::error::Result;
use crate::types::{HealthStatus, ServiceStatus};
use reqwest::Client;
use std::time::{Duration, Instant};
use tracing::debug;

/// Health check endpoint (minimal AUR RPC request).
const HEALTH_CHECK_URL: &str = "https://aur.archlinux.org/rpc/v5/info";

/// Default timeout for health checks (shorter than regular operations).
const DEFAULT_HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Latency threshold for "degraded" status (2 seconds).
const DEGRADED_LATENCY_THRESHOLD: Duration = Duration::from_secs(2);

/// What: Perform health check against AUR API.
///
/// Inputs:
/// - `client`: HTTP client to use for the request
/// - `timeout`: Optional timeout override (uses default if None)
///
/// Output:
/// - `Result<HealthStatus>` with service status and latency
///
/// Details:
/// - Uses minimal RPC request (info with no packages)
/// - Validates response is valid JSON with expected structure
/// - Measures round-trip latency
/// - Returns `HealthStatus` with appropriate `ServiceStatus` based on:
///   - Success + latency < 2s = `Healthy`
///   - Success + latency >= 2s = `Degraded`
///   - HTTP error = `Unreachable`
///   - Timeout = `Timeout`
///
/// # Errors
/// - Never returns an error - always returns `Ok(HealthStatus)` with appropriate status
/// - Network errors are represented as `ServiceStatus::Unreachable` or `ServiceStatus::Timeout`
pub async fn check_health(client: &Client, timeout: Option<Duration>) -> Result<HealthStatus> {
    let start = Instant::now();
    let checked_at = start;

    let timeout_duration = timeout.unwrap_or(DEFAULT_HEALTH_CHECK_TIMEOUT);

    // Create a request with health-check-specific timeout
    let result = client
        .get(HEALTH_CHECK_URL)
        .timeout(timeout_duration)
        .send()
        .await;

    let latency = start.elapsed();

    match result {
        Ok(response) => {
            // Check HTTP status
            if !response.status().is_success() {
                debug!(
                    status = %response.status(),
                    latency_ms = latency.as_millis(),
                    "health check returned non-success status"
                );
                return Ok(HealthStatus {
                    aur_api: ServiceStatus::Unreachable,
                    latency: Some(latency),
                    checked_at,
                });
            }

            // Validate response body is valid JSON
            match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    // Verify it's a valid AUR RPC response
                    let is_valid = json.get("version").is_some() && json.get("type").is_some();

                    if !is_valid {
                        debug!(
                            latency_ms = latency.as_millis(),
                            "health check response missing expected fields"
                        );
                        return Ok(HealthStatus {
                            aur_api: ServiceStatus::Degraded,
                            latency: Some(latency),
                            checked_at,
                        });
                    }

                    // Determine status based on latency
                    let status = if latency > DEGRADED_LATENCY_THRESHOLD {
                        ServiceStatus::Degraded
                    } else {
                        ServiceStatus::Healthy
                    };

                    debug!(
                        latency_ms = latency.as_millis(),
                        ?status,
                        "health check completed"
                    );

                    Ok(HealthStatus {
                        aur_api: status,
                        latency: Some(latency),
                        checked_at,
                    })
                }
                Err(e) => {
                    debug!(
                        error = %e,
                        latency_ms = latency.as_millis(),
                        "health check failed to parse response"
                    );
                    Ok(HealthStatus {
                        aur_api: ServiceStatus::Degraded,
                        latency: Some(latency),
                        checked_at,
                    })
                }
            }
        }
        Err(e) => {
            let status = if e.is_timeout() {
                ServiceStatus::Timeout
            } else {
                ServiceStatus::Unreachable
            };

            debug!(
                error = %e,
                ?status,
                latency_ms = latency.as_millis(),
                "health check failed"
            );

            Ok(HealthStatus {
                aur_api: status,
                latency: Some(latency),
                checked_at,
            })
        }
    }
}
