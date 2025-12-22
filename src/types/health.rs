//! Health check types for archlinux.org services.

use std::time::{Duration, Instant};

/// What: Health status for archlinux.org services.
///
/// Inputs: None (created by health check operations)
///
/// Output: Struct containing service status and latency information
///
/// Details:
/// - Provides detailed health information for connection status UIs
/// - Includes latency measurements for performance monitoring
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Whether the AUR RPC API is reachable.
    pub aur_api: ServiceStatus,
    /// Measured latency to the AUR API (if reachable).
    pub latency: Option<Duration>,
    /// Timestamp when health check was performed.
    pub checked_at: Instant,
}

/// What: Status of a single service endpoint.
///
/// Inputs: None (enum variant)
///
/// Output: Enum representing service health state
///
/// Details:
/// - Healthy: Service is responding normally
/// - Degraded: Service is slow but functional
/// - Unreachable: Service returned an error
/// - Timeout: Request timed out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Service is healthy and responding.
    Healthy,
    /// Service is degraded (slow response, partial functionality).
    Degraded,
    /// Service is unreachable.
    Unreachable,
    /// Health check timed out.
    Timeout,
}

impl ServiceStatus {
    /// What: Check if the service is operational.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `true` if the service is operational (Healthy or Degraded), `false` otherwise
    ///
    /// Details:
    /// - Returns `true` for `Healthy` and `Degraded` statuses
    /// - Returns `false` for `Unreachable` and `Timeout` statuses
    #[must_use]
    pub const fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }
}

impl HealthStatus {
    /// What: Check if all services are healthy.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `true` if all services are operational, `false` otherwise
    ///
    /// Details:
    /// - Delegates to `ServiceStatus::is_operational()` for the AUR API
    #[must_use]
    pub const fn is_healthy(&self) -> bool {
        self.aur_api.is_operational()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_status_is_operational() {
        assert!(ServiceStatus::Healthy.is_operational());
        assert!(ServiceStatus::Degraded.is_operational());
        assert!(!ServiceStatus::Unreachable.is_operational());
        assert!(!ServiceStatus::Timeout.is_operational());
    }

    #[test]
    fn test_health_status_is_healthy() {
        let healthy = HealthStatus {
            aur_api: ServiceStatus::Healthy,
            latency: Some(Duration::from_millis(100)),
            checked_at: Instant::now(),
        };
        assert!(healthy.is_healthy());

        let degraded = HealthStatus {
            aur_api: ServiceStatus::Degraded,
            latency: Some(Duration::from_secs(3)),
            checked_at: Instant::now(),
        };
        assert!(degraded.is_healthy()); // Degraded is still operational

        let unreachable = HealthStatus {
            aur_api: ServiceStatus::Unreachable,
            latency: None,
            checked_at: Instant::now(),
        };
        assert!(!unreachable.is_healthy());

        let timeout = HealthStatus {
            aur_api: ServiceStatus::Timeout,
            latency: None,
            checked_at: Instant::now(),
        };
        assert!(!timeout.is_healthy());
    }
}
