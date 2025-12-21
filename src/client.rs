//! HTTP client with rate limiting for arch-toolkit.

#[cfg(feature = "aur")]
use std::sync::{LazyLock, Mutex};
#[cfg(feature = "aur")]
use std::time::{Duration, Instant};

#[cfg(feature = "aur")]
use rand::Rng;
#[cfg(feature = "aur")]
use tracing::{debug, warn};

#[cfg(feature = "aur")]
/// Rate limiter state for archlinux.org with exponential backoff.
struct ArchLinuxRateLimiter {
    /// Last request timestamp.
    last_request: Instant,
    /// Current backoff delay in milliseconds (starts at base delay, increases exponentially).
    current_backoff_ms: u64,
    /// Number of consecutive failures/rate limits.
    consecutive_failures: u32,
}

#[cfg(feature = "aur")]
/// Rate limiter for archlinux.org requests with exponential backoff.
/// Tracks last request time and implements progressive delays on failures.
static ARCHLINUX_RATE_LIMITER: LazyLock<Mutex<ArchLinuxRateLimiter>> = LazyLock::new(|| {
    Mutex::new(ArchLinuxRateLimiter {
        last_request: Instant::now(),
        current_backoff_ms: 500, // Start with 500ms base delay
        consecutive_failures: 0,
    })
});

#[cfg(feature = "aur")]
/// Semaphore to serialize archlinux.org requests (only 1 concurrent request allowed).
/// This prevents multiple async tasks from overwhelming the server even when rate limiting
/// is applied, because the rate limiter alone doesn't prevent concurrent requests that
/// start at nearly the same time from all proceeding simultaneously.
static ARCHLINUX_REQUEST_SEMAPHORE: LazyLock<std::sync::Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| std::sync::Arc::new(tokio::sync::Semaphore::new(1)));

#[cfg(feature = "aur")]
/// Base delay for archlinux.org requests (500ms).
const ARCHLINUX_BASE_DELAY_MS: u64 = 500;
#[cfg(feature = "aur")]
/// Maximum backoff delay (60 seconds).
const ARCHLINUX_MAX_BACKOFF_MS: u64 = 60_000;
#[cfg(feature = "aur")]
/// Maximum jitter in milliseconds to add to rate limiting delays (prevents thundering herd).
const JITTER_MAX_MS: u64 = 500;

/// What: Apply rate limiting specifically for archlinux.org requests with exponential backoff.
///
/// Inputs: None
///
/// Output: `OwnedSemaphorePermit` that the caller MUST hold during the request.
///
/// # Panics
/// - Panics if the archlinux.org request semaphore is closed (should never happen in practice).
///
/// Details:
/// - Acquires a semaphore permit to serialize archlinux.org requests (only 1 at a time).
/// - Uses base delay (500ms) for archlinux.org to reduce request frequency.
/// - Implements exponential backoff: increases delay on consecutive failures (500ms → 1s → 2s → 4s, max 60s).
/// - Adds random jitter (0-500ms) to prevent thundering herd when multiple clients retry simultaneously.
/// - Resets backoff after successful requests.
/// - Thread-safe via mutex guarding the rate limiter state.
/// - The returned permit MUST be held until the HTTP request completes to ensure serialization.
#[cfg(feature = "aur")]
pub async fn rate_limit_archlinux() -> tokio::sync::OwnedSemaphorePermit {
    // 1. Acquire semaphore to serialize requests (waits if another request is in progress)
    let permit = ARCHLINUX_REQUEST_SEMAPHORE
        .clone()
        .acquire_owned()
        .await
        .expect("archlinux.org request semaphore should never be closed");

    // 2. Now that we have exclusive access, compute and apply the rate limiting delay
    let delay_needed = {
        let mut limiter = match ARCHLINUX_RATE_LIMITER.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let elapsed = limiter.last_request.elapsed();
        let min_delay = Duration::from_millis(limiter.current_backoff_ms);
        let delay = if elapsed < min_delay {
            min_delay.checked_sub(elapsed).unwrap_or(Duration::ZERO)
        } else {
            Duration::ZERO
        };
        limiter.last_request = Instant::now();
        delay
    };

    if !delay_needed.is_zero() {
        // Add random jitter to prevent thundering herd when multiple clients retry simultaneously
        let jitter_ms = rand::rng().random_range(0..=JITTER_MAX_MS);
        let delay_with_jitter = delay_needed + Duration::from_millis(jitter_ms);
        #[allow(clippy::cast_possible_truncation)] // Delay will be small (max 60s = 60000ms)
        let delay_ms = delay_needed.as_millis() as u64;
        debug!(
            delay_ms,
            jitter_ms,
            total_ms = delay_with_jitter.as_millis(),
            "rate limiting archlinux.org request with jitter"
        );
        tokio::time::sleep(delay_with_jitter).await;
    }

    // 3. Return the permit - caller MUST hold it during the request
    permit
}

/// What: Increase backoff delay for archlinux.org after a failure or rate limit.
///
/// Inputs:
/// - `retry_after_seconds`: Optional retry-after value from server (in seconds).
///
/// Output: None
///
/// Details:
/// - If `retry_after_seconds` is provided, uses that value (capped at maximum).
/// - Otherwise, doubles the current backoff delay (exponential backoff).
/// - Caps backoff at maximum delay (60 seconds).
/// - Increments consecutive failure counter.
#[cfg(feature = "aur")]
pub fn increase_archlinux_backoff(retry_after_seconds: Option<u64>) {
    let mut limiter = match ARCHLINUX_RATE_LIMITER.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    limiter.consecutive_failures += 1;
    // Use Retry-After value if provided, otherwise use exponential backoff
    if let Some(retry_after) = retry_after_seconds {
        // Convert seconds to milliseconds, cap at maximum
        let retry_after_ms = (retry_after * 1000).min(ARCHLINUX_MAX_BACKOFF_MS);
        limiter.current_backoff_ms = retry_after_ms;
        warn!(
            consecutive_failures = limiter.consecutive_failures,
            retry_after_seconds = retry_after,
            backoff_ms = limiter.current_backoff_ms,
            "increased archlinux.org backoff delay using Retry-After header"
        );
    } else {
        // Double the backoff delay, capped at maximum
        limiter.current_backoff_ms = (limiter.current_backoff_ms * 2).min(ARCHLINUX_MAX_BACKOFF_MS);
        warn!(
            consecutive_failures = limiter.consecutive_failures,
            backoff_ms = limiter.current_backoff_ms,
            "increased archlinux.org backoff delay"
        );
    }
}

/// What: Reset backoff delay for archlinux.org after a successful request.
///
/// Inputs: None
///
/// Output: None
///
/// Details:
/// - Resets backoff to base delay (500ms).
/// - Resets consecutive failure counter.
#[cfg(feature = "aur")]
pub fn reset_archlinux_backoff() {
    let mut limiter = match ARCHLINUX_RATE_LIMITER.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if limiter.consecutive_failures > 0 {
        debug!(
            previous_failures = limiter.consecutive_failures,
            previous_backoff_ms = limiter.current_backoff_ms,
            "resetting archlinux.org backoff after successful request"
        );
    }
    limiter.current_backoff_ms = ARCHLINUX_BASE_DELAY_MS;
    limiter.consecutive_failures = 0;
}

/// What: Check if a URL belongs to archlinux.org domain.
///
/// Inputs:
/// - `url`: URL string to check.
///
/// Output:
/// - `true` if URL is from archlinux.org, `false` otherwise.
///
/// Details:
/// - Checks if URL contains "archlinux.org" domain.
#[cfg(feature = "aur")]
#[must_use]
pub fn is_archlinux_url(url: &str) -> bool {
    url.contains("archlinux.org")
}
