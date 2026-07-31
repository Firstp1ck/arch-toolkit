//! Environment variable configuration for arch-toolkit.
//!
//! This module provides utilities for reading configuration from environment variables,
//! allowing zero-code configuration for CI/CD pipelines, Docker containers, and runtime adjustments.

#[cfg(feature = "aur")]
use std::time::Duration;

/// What: Read timeout from `ARCH_TOOLKIT_TIMEOUT` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<Duration>` containing the timeout if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_TIMEOUT` as seconds (u64)
/// - Returns `None` if variable is not set or cannot be parsed
/// - Invalid values are silently ignored (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_timeout() -> Option<Duration> {
    std::env::var("ARCH_TOOLKIT_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
}

/// What: Read user agent from `ARCH_TOOLKIT_USER_AGENT` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<String>` containing the user agent if the variable is set, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_USER_AGENT` as a string
/// - Returns `None` if variable is not set
/// - Empty strings are treated as unset (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_user_agent() -> Option<String> {
    std::env::var("ARCH_TOOLKIT_USER_AGENT")
        .ok()
        .filter(|s| !s.is_empty())
}

/// What: Read health check timeout from `ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<Duration>` containing the timeout if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT` as seconds (u64)
/// - Returns `None` if variable is not set or cannot be parsed
/// - Invalid values are silently ignored (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_health_check_timeout() -> Option<Duration> {
    std::env::var("ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
}

/// What: Read max retries from `ARCH_TOOLKIT_MAX_RETRIES` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<u32>` containing the max retries if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_MAX_RETRIES` as u32
/// - Returns `None` if variable is not set or cannot be parsed
/// - Invalid values are silently ignored (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_max_retries() -> Option<u32> {
    std::env::var("ARCH_TOOLKIT_MAX_RETRIES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
}

/// What: Read retry enabled flag from `ARCH_TOOLKIT_RETRY_ENABLED` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<bool>` containing the flag if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_RETRY_ENABLED` as boolean
/// - Accepts: "true", "1", "yes", "on" (case-insensitive) for `true`
/// - Accepts: "false", "0", "no", "off" (case-insensitive) for `false`
/// - Returns `None` if variable is not set or cannot be parsed
#[cfg(feature = "aur")]
#[must_use]
pub fn env_retry_enabled() -> Option<bool> {
    std::env::var("ARCH_TOOLKIT_RETRY_ENABLED")
        .ok()
        .and_then(|v| {
            let lower = v.to_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            }
        })
}

/// What: Read retry initial delay from `ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<u64>` containing the delay in milliseconds if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS` as u64 (milliseconds)
/// - Returns `None` if variable is not set or cannot be parsed
/// - Invalid values are silently ignored (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_retry_initial_delay_ms() -> Option<u64> {
    std::env::var("ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
}

/// What: Read retry max delay from `ARCH_TOOLKIT_RETRY_MAX_DELAY_MS` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<u64>` containing the max delay in milliseconds if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_RETRY_MAX_DELAY_MS` as u64 (milliseconds)
/// - Returns `None` if variable is not set or cannot be parsed
/// - Invalid values are silently ignored (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_retry_max_delay_ms() -> Option<u64> {
    std::env::var("ARCH_TOOLKIT_RETRY_MAX_DELAY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
}

/// What: Read validation strict flag from `ARCH_TOOLKIT_VALIDATION_STRICT` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<bool>` containing the flag if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_VALIDATION_STRICT` as boolean
/// - Accepts: "true", "1", "yes", "on" (case-insensitive) for `true`
/// - Accepts: "false", "0", "no", "off" (case-insensitive) for `false`
/// - Returns `None` if variable is not set or cannot be parsed
#[cfg(feature = "aur")]
#[must_use]
pub fn env_validation_strict() -> Option<bool> {
    std::env::var("ARCH_TOOLKIT_VALIDATION_STRICT")
        .ok()
        .and_then(|v| {
            let lower = v.to_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            }
        })
}

/// What: Read cache size from `ARCH_TOOLKIT_CACHE_SIZE` environment variable.
///
/// Inputs: None
///
/// Output:
/// - `Option<usize>` containing the cache size if the variable is set and valid, `None` otherwise
///
/// Details:
/// - Reads `ARCH_TOOLKIT_CACHE_SIZE` as usize
/// - Returns `None` if variable is not set or cannot be parsed
/// - Invalid values are silently ignored (returns `None`)
#[cfg(feature = "aur")]
#[must_use]
pub fn env_cache_size() -> Option<usize> {
    std::env::var("ARCH_TOOLKIT_CACHE_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

/// Test-only helpers that make process-global environment mutation deterministic.
///
/// Details:
/// - `ARCH_TOOLKIT_*` variables are process-global, so concurrently running unit
///   tests would otherwise observe each other's writes. Every test that reads or
///   writes them must hold the guard returned by [`test_support::lock_env`].
#[cfg(test)]
#[allow(clippy::redundant_pub_crate)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

    /// Every `ARCH_TOOLKIT_*` variable owned by the crate configuration surface.
    ///
    /// Details:
    /// - The guard clears all of them on acquisition and restores them on drop,
    ///   so each test starts from a known-empty configuration environment.
    pub(crate) const MANAGED_VARS: &[&str] = &[
        "ARCH_TOOLKIT_TIMEOUT",
        "ARCH_TOOLKIT_USER_AGENT",
        "ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT",
        "ARCH_TOOLKIT_MAX_RETRIES",
        "ARCH_TOOLKIT_RETRY_ENABLED",
        "ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS",
        "ARCH_TOOLKIT_RETRY_MAX_DELAY_MS",
        "ARCH_TOOLKIT_VALIDATION_STRICT",
        "ARCH_TOOLKIT_CACHE_SIZE",
    ];

    /// Process-wide mutex serializing every environment-mutating test.
    static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

    /// What: RAII guard that owns exclusive access to the `ARCH_TOOLKIT_*` environment.
    ///
    /// Inputs:
    /// - Constructed only through [`lock_env`].
    ///
    /// Output:
    /// - Exclusive, restoring access to the managed environment variables.
    ///
    /// Details:
    /// - Holds the process-wide mutex for its whole lifetime.
    /// - Clears all managed variables on acquisition and restores the previously
    ///   observed values on drop, including for panicking tests.
    pub(crate) struct EnvGuard {
        /// Held lock keeping other environment tests out of the critical section.
        _lock: MutexGuard<'static, ()>,
        /// Values observed at acquisition time, restored on drop.
        saved: Vec<(&'static str, Option<String>)>,
    }

    #[allow(clippy::unused_self)]
    impl EnvGuard {
        /// What: Set a managed environment variable inside the guarded section.
        ///
        /// Inputs:
        /// - `key`: One of [`MANAGED_VARS`].
        /// - `value`: Raw value to expose to the configuration readers.
        ///
        /// Output:
        /// - The variable is visible to this process until the guard is dropped.
        ///
        /// Details:
        /// - Panics when `key` is not managed, because such a variable would not
        ///   be restored on drop and could leak into other tests.
        pub(crate) fn set(&self, key: &'static str, value: &str) {
            assert!(
                MANAGED_VARS.contains(&key),
                "{key} is not a managed ARCH_TOOLKIT_* test variable"
            );
            // SAFETY: the guard holds the process-wide environment mutex, so no
            // other test thread reads or writes the environment concurrently.
            unsafe {
                std::env::set_var(key, value);
            }
        }

        /// What: Remove a managed environment variable inside the guarded section.
        ///
        /// Inputs:
        /// - `key`: One of [`MANAGED_VARS`].
        ///
        /// Output:
        /// - The variable is unset for this process until the guard is dropped.
        ///
        /// Details:
        /// - Acquisition already clears every managed variable; this exists for
        ///   tests that unset a value in the middle of a scenario.
        pub(crate) fn remove(&self, key: &'static str) {
            assert!(
                MANAGED_VARS.contains(&key),
                "{key} is not a managed ARCH_TOOLKIT_* test variable"
            );
            // SAFETY: see `set`.
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in &self.saved {
                // SAFETY: the guard still owns the environment mutex here.
                unsafe {
                    match value {
                        Some(previous) => std::env::set_var(key, previous),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    /// What: Acquire exclusive, restoring access to the `ARCH_TOOLKIT_*` environment.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - An [`EnvGuard`] with every managed variable cleared.
    ///
    /// Details:
    /// - Recovers from mutex poisoning so one failing test cannot cascade into
    ///   unrelated failures; the guard restores state regardless.
    /// - Makes the environment tests deterministic under `cargo test` without
    ///   `--test-threads=1`.
    pub(crate) fn lock_env() -> EnvGuard {
        let lock = ENV_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let saved = MANAGED_VARS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();

        for key in MANAGED_VARS {
            // SAFETY: the mutex is held, so no other test thread touches the environment.
            unsafe {
                std::env::remove_var(key);
            }
        }

        EnvGuard { _lock: lock, saved }
    }
}

#[cfg(test)]
#[cfg(feature = "aur")]
#[allow(clippy::significant_drop_tightening)]
mod tests {
    use super::test_support::lock_env;
    use super::*;

    #[test]
    fn test_env_timeout_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_TIMEOUT", "60");
        assert_eq!(env_timeout(), Some(Duration::from_mins(1)));
    }

    #[test]
    fn test_env_timeout_invalid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_TIMEOUT", "invalid");
        assert_eq!(env_timeout(), None);
    }

    #[test]
    fn test_env_timeout_missing() {
        let _env = lock_env();
        assert_eq!(env_timeout(), None);
    }

    #[test]
    fn test_env_user_agent_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_USER_AGENT", "my-app/1.0");
        assert_eq!(env_user_agent(), Some("my-app/1.0".to_string()));
    }

    #[test]
    fn test_env_user_agent_empty() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_USER_AGENT", "");
        assert_eq!(env_user_agent(), None);
    }

    #[test]
    fn test_env_user_agent_missing() {
        let _env = lock_env();
        assert_eq!(env_user_agent(), None);
    }

    #[test]
    fn test_env_health_check_timeout_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT", "10");
        assert_eq!(env_health_check_timeout(), Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_env_health_check_timeout_missing() {
        let _env = lock_env();
        assert_eq!(env_health_check_timeout(), None);
    }

    #[test]
    fn test_env_max_retries_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_MAX_RETRIES", "5");
        assert_eq!(env_max_retries(), Some(5));
    }

    #[test]
    fn test_env_max_retries_invalid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_MAX_RETRIES", "invalid");
        assert_eq!(env_max_retries(), None);
    }

    #[test]
    fn test_env_max_retries_missing() {
        let _env = lock_env();
        assert_eq!(env_max_retries(), None);
    }

    #[test]
    fn test_env_retry_enabled_true() {
        let env = lock_env();
        for value in ["true", "TRUE", "True", "1", "yes", "YES", "on", "ON"] {
            env.set("ARCH_TOOLKIT_RETRY_ENABLED", value);
            assert_eq!(env_retry_enabled(), Some(true), "Failed for value: {value}");
        }
    }

    #[test]
    fn test_env_retry_enabled_false() {
        let env = lock_env();
        for value in ["false", "FALSE", "False", "0", "no", "NO", "off", "OFF"] {
            env.set("ARCH_TOOLKIT_RETRY_ENABLED", value);
            assert_eq!(
                env_retry_enabled(),
                Some(false),
                "Failed for value: {value}"
            );
        }
    }

    #[test]
    fn test_env_retry_enabled_invalid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_RETRY_ENABLED", "maybe");
        assert_eq!(env_retry_enabled(), None);
    }

    #[test]
    fn test_env_retry_enabled_missing() {
        let _env = lock_env();
        assert_eq!(env_retry_enabled(), None);
    }

    #[test]
    fn test_env_retry_initial_delay_ms_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS", "2000");
        assert_eq!(env_retry_initial_delay_ms(), Some(2000));
    }

    #[test]
    fn test_env_retry_initial_delay_ms_missing() {
        let _env = lock_env();
        assert_eq!(env_retry_initial_delay_ms(), None);
    }

    #[test]
    fn test_env_retry_max_delay_ms_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_RETRY_MAX_DELAY_MS", "60000");
        assert_eq!(env_retry_max_delay_ms(), Some(60000));
    }

    #[test]
    fn test_env_retry_max_delay_ms_missing() {
        let _env = lock_env();
        assert_eq!(env_retry_max_delay_ms(), None);
    }

    #[test]
    fn test_env_validation_strict_true() {
        let env = lock_env();
        for value in ["true", "TRUE", "1", "yes", "on"] {
            env.set("ARCH_TOOLKIT_VALIDATION_STRICT", value);
            assert_eq!(
                env_validation_strict(),
                Some(true),
                "Failed for value: {value}"
            );
        }
    }

    #[test]
    fn test_env_validation_strict_false() {
        let env = lock_env();
        for value in ["false", "FALSE", "0", "no", "off"] {
            env.set("ARCH_TOOLKIT_VALIDATION_STRICT", value);
            assert_eq!(
                env_validation_strict(),
                Some(false),
                "Failed for value: {value}"
            );
        }
    }

    #[test]
    fn test_env_validation_strict_missing() {
        let _env = lock_env();
        assert_eq!(env_validation_strict(), None);
    }

    #[test]
    fn test_env_cache_size_valid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_CACHE_SIZE", "200");
        assert_eq!(env_cache_size(), Some(200));
    }

    #[test]
    fn test_env_cache_size_invalid() {
        let env = lock_env();
        env.set("ARCH_TOOLKIT_CACHE_SIZE", "invalid");
        assert_eq!(env_cache_size(), None);
    }

    #[test]
    fn test_env_cache_size_missing() {
        let _env = lock_env();
        assert_eq!(env_cache_size(), None);
    }

    #[test]
    /// What: Verify the environment guard restores pre-existing values on drop.
    ///
    /// Inputs:
    /// - A managed variable set outside the guard, then overwritten inside it.
    ///
    /// Output:
    /// - The original value is observable again after the guard is dropped.
    ///
    /// Details:
    /// - Protects the isolation contract that keeps parallel test runs deterministic.
    fn test_env_guard_restores_previous_values() {
        let outer = lock_env();
        outer.set("ARCH_TOOLKIT_TIMEOUT", "111");

        {
            let inner_saved = std::env::var("ARCH_TOOLKIT_TIMEOUT").ok();
            assert_eq!(inner_saved.as_deref(), Some("111"));
        }

        outer.remove("ARCH_TOOLKIT_TIMEOUT");
        assert_eq!(env_timeout(), None);
    }
}
