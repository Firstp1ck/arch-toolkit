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

#[cfg(test)]
#[cfg(feature = "aur")]
mod tests {
    use super::*;

    #[test]
    fn test_env_timeout_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_TIMEOUT", "60");
        }
        let result = env_timeout();
        assert_eq!(result, Some(Duration::from_secs(60)));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_TIMEOUT");
        }
    }

    #[test]
    fn test_env_timeout_invalid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_TIMEOUT", "invalid");
        }
        let result = env_timeout();
        assert_eq!(result, None);
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_TIMEOUT");
        }
    }

    #[test]
    fn test_env_timeout_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_TIMEOUT");
        }
        let result = env_timeout();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_user_agent_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_USER_AGENT", "my-app/1.0");
        }
        let result = env_user_agent();
        assert_eq!(result, Some("my-app/1.0".to_string()));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_USER_AGENT");
        }
    }

    #[test]
    fn test_env_user_agent_empty() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_USER_AGENT", "");
        }
        let result = env_user_agent();
        assert_eq!(result, None);
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_USER_AGENT");
        }
    }

    #[test]
    fn test_env_user_agent_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_USER_AGENT");
        }
        let result = env_user_agent();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_health_check_timeout_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT", "10");
        }
        let result = env_health_check_timeout();
        assert_eq!(result, Some(Duration::from_secs(10)));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT");
        }
    }

    #[test]
    fn test_env_health_check_timeout_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT");
        }
        let result = env_health_check_timeout();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_max_retries_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_MAX_RETRIES", "5");
        }
        let result = env_max_retries();
        assert_eq!(result, Some(5));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_MAX_RETRIES");
        }
    }

    #[test]
    fn test_env_max_retries_invalid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_MAX_RETRIES", "invalid");
        }
        let result = env_max_retries();
        assert_eq!(result, None);
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_MAX_RETRIES");
        }
    }

    #[test]
    fn test_env_max_retries_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_MAX_RETRIES");
        }
        let result = env_max_retries();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_retry_enabled_true() {
        for value in ["true", "TRUE", "True", "1", "yes", "YES", "on", "ON"] {
            unsafe {
                std::env::set_var("ARCH_TOOLKIT_RETRY_ENABLED", value);
            }
            let result = env_retry_enabled();
            assert_eq!(result, Some(true), "Failed for value: {value}");
            unsafe {
                std::env::remove_var("ARCH_TOOLKIT_RETRY_ENABLED");
            }
        }
    }

    #[test]
    fn test_env_retry_enabled_false() {
        for value in ["false", "FALSE", "False", "0", "no", "NO", "off", "OFF"] {
            unsafe {
                std::env::set_var("ARCH_TOOLKIT_RETRY_ENABLED", value);
            }
            let result = env_retry_enabled();
            assert_eq!(result, Some(false), "Failed for value: {value}");
            unsafe {
                std::env::remove_var("ARCH_TOOLKIT_RETRY_ENABLED");
            }
        }
    }

    #[test]
    fn test_env_retry_enabled_invalid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_RETRY_ENABLED", "maybe");
        }
        let result = env_retry_enabled();
        assert_eq!(result, None);
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_RETRY_ENABLED");
        }
    }

    #[test]
    fn test_env_retry_enabled_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_RETRY_ENABLED");
        }
        let result = env_retry_enabled();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_retry_initial_delay_ms_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS", "2000");
        }
        let result = env_retry_initial_delay_ms();
        assert_eq!(result, Some(2000));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS");
        }
    }

    #[test]
    fn test_env_retry_initial_delay_ms_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS");
        }
        let result = env_retry_initial_delay_ms();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_retry_max_delay_ms_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_RETRY_MAX_DELAY_MS", "60000");
        }
        let result = env_retry_max_delay_ms();
        assert_eq!(result, Some(60000));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_RETRY_MAX_DELAY_MS");
        }
    }

    #[test]
    fn test_env_retry_max_delay_ms_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_RETRY_MAX_DELAY_MS");
        }
        let result = env_retry_max_delay_ms();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_validation_strict_true() {
        for value in ["true", "TRUE", "1", "yes", "on"] {
            unsafe {
                std::env::set_var("ARCH_TOOLKIT_VALIDATION_STRICT", value);
            }
            let result = env_validation_strict();
            assert_eq!(result, Some(true), "Failed for value: {value}");
            unsafe {
                std::env::remove_var("ARCH_TOOLKIT_VALIDATION_STRICT");
            }
        }
    }

    #[test]
    fn test_env_validation_strict_false() {
        for value in ["false", "FALSE", "0", "no", "off"] {
            unsafe {
                std::env::set_var("ARCH_TOOLKIT_VALIDATION_STRICT", value);
            }
            let result = env_validation_strict();
            assert_eq!(result, Some(false), "Failed for value: {value}");
            unsafe {
                std::env::remove_var("ARCH_TOOLKIT_VALIDATION_STRICT");
            }
        }
    }

    #[test]
    fn test_env_validation_strict_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_VALIDATION_STRICT");
        }
        let result = env_validation_strict();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_cache_size_valid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_CACHE_SIZE", "200");
        }
        let result = env_cache_size();
        assert_eq!(result, Some(200));
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_CACHE_SIZE");
        }
    }

    #[test]
    fn test_env_cache_size_invalid() {
        unsafe {
            std::env::set_var("ARCH_TOOLKIT_CACHE_SIZE", "invalid");
        }
        let result = env_cache_size();
        assert_eq!(result, None);
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_CACHE_SIZE");
        }
    }

    #[test]
    fn test_env_cache_size_missing() {
        unsafe {
            std::env::remove_var("ARCH_TOOLKIT_CACHE_SIZE");
        }
        let result = env_cache_size();
        assert_eq!(result, None);
    }
}
