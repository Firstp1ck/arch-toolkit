//! Cache configuration for arch-toolkit operations.

use std::time::Duration;

/// What: Configuration for cache behavior.
///
/// Inputs: None (created via `CacheConfig::builder()` or `default()`)
///
/// Output:
/// - `CacheConfig` instance with configurable cache settings
///
/// Details:
/// - Controls per-operation cache enable/disable flags
/// - Configures TTL for each operation type
/// - Sets memory cache size limits
/// - Controls disk cache enable/disable
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // Per-operation flags are necessary for fine-grained control
pub struct CacheConfig {
    /// Whether search operation caching is enabled (default: false).
    pub enable_search: bool,
    /// TTL for search cache entries (default: 5 minutes).
    pub search_ttl: Duration,
    /// Whether info operation caching is enabled (default: false).
    pub enable_info: bool,
    /// TTL for info cache entries (default: 15 minutes).
    pub info_ttl: Duration,
    /// Whether comments operation caching is enabled (default: false).
    pub enable_comments: bool,
    /// TTL for comments cache entries (default: 10 minutes).
    pub comments_ttl: Duration,
    /// Whether pkgbuild operation caching is enabled (default: false).
    pub enable_pkgbuild: bool,
    /// TTL for pkgbuild cache entries (default: 1 hour).
    pub pkgbuild_ttl: Duration,
    /// Maximum number of entries in memory cache per operation (default: 100).
    pub memory_cache_size: usize,
    /// Whether disk cache is enabled (default: false).
    pub enable_disk_cache: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_search: false,
            search_ttl: Duration::from_secs(300), // 5 minutes
            enable_info: false,
            info_ttl: Duration::from_secs(900), // 15 minutes
            enable_comments: false,
            comments_ttl: Duration::from_secs(600), // 10 minutes
            enable_pkgbuild: false,
            pkgbuild_ttl: Duration::from_secs(3600), // 1 hour
            memory_cache_size: 100,
            enable_disk_cache: false,
        }
    }
}

/// What: Builder for creating `CacheConfig` with custom settings.
///
/// Inputs: None (created via `CacheConfig::builder()`)
///
/// Output:
/// - `CacheConfigBuilder` that can be configured and built
///
/// Details:
/// - Allows customization of all cache settings
/// - All methods return `&mut Self` for method chaining
/// - Call `build()` to create the `CacheConfig`
#[derive(Debug, Clone)]
pub struct CacheConfigBuilder {
    /// Internal cache configuration being built.
    config: CacheConfig,
}

impl CacheConfigBuilder {
    /// What: Create a new builder with default values.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `CacheConfigBuilder` with default configuration
    ///
    /// Details:
    /// - Starts with all caches disabled
    /// - Uses default TTL values
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: uses Default::default()
    pub fn new() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }

    /// What: Enable or disable search operation caching.
    ///
    /// Inputs:
    /// - `enable`: Whether to enable search caching
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: false
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self
    pub fn enable_search(mut self, enable: bool) -> Self {
        self.config.enable_search = enable;
        self
    }

    /// What: Set TTL for search cache entries.
    ///
    /// Inputs:
    /// - `ttl`: Time-to-live duration
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: 5 minutes
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self and uses Duration
    pub fn search_ttl(mut self, ttl: Duration) -> Self {
        self.config.search_ttl = ttl;
        self
    }

    /// What: Enable or disable info operation caching.
    ///
    /// Inputs:
    /// - `enable`: Whether to enable info caching
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: false
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self
    pub fn enable_info(mut self, enable: bool) -> Self {
        self.config.enable_info = enable;
        self
    }

    /// What: Set TTL for info cache entries.
    ///
    /// Inputs:
    /// - `ttl`: Time-to-live duration
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: 15 minutes
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self and uses Duration
    pub fn info_ttl(mut self, ttl: Duration) -> Self {
        self.config.info_ttl = ttl;
        self
    }

    /// What: Enable or disable comments operation caching.
    ///
    /// Inputs:
    /// - `enable`: Whether to enable comments caching
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: false
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self
    pub fn enable_comments(mut self, enable: bool) -> Self {
        self.config.enable_comments = enable;
        self
    }

    /// What: Set TTL for comments cache entries.
    ///
    /// Inputs:
    /// - `ttl`: Time-to-live duration
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: 10 minutes
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self and uses Duration
    pub fn comments_ttl(mut self, ttl: Duration) -> Self {
        self.config.comments_ttl = ttl;
        self
    }

    /// What: Enable or disable pkgbuild operation caching.
    ///
    /// Inputs:
    /// - `enable`: Whether to enable pkgbuild caching
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: false
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self
    pub fn enable_pkgbuild(mut self, enable: bool) -> Self {
        self.config.enable_pkgbuild = enable;
        self
    }

    /// What: Set TTL for pkgbuild cache entries.
    ///
    /// Inputs:
    /// - `ttl`: Time-to-live duration
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: 1 hour
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self and uses Duration
    pub fn pkgbuild_ttl(mut self, ttl: Duration) -> Self {
        self.config.pkgbuild_ttl = ttl;
        self
    }

    /// What: Set maximum number of entries in memory cache per operation.
    ///
    /// Inputs:
    /// - `size`: Maximum cache size
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: 100
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self
    pub fn memory_cache_size(mut self, size: usize) -> Self {
        self.config.memory_cache_size = size;
        self
    }

    /// What: Enable or disable disk cache.
    ///
    /// Inputs:
    /// - `enable`: Whether to enable disk cache
    ///
    /// Output:
    /// - `&mut Self` for method chaining
    ///
    /// Details:
    /// - Default: false
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: mutates self
    pub fn enable_disk_cache(mut self, enable: bool) -> Self {
        self.config.enable_disk_cache = enable;
        self
    }

    /// What: Build the `CacheConfig` with configured settings.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `CacheConfig` with configured values
    ///
    /// Details:
    /// - Consumes the builder
    /// - Returns the configured `CacheConfig`
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Cannot be const: consumes self
    pub fn build(self) -> CacheConfig {
        self.config
    }
}

impl Default for CacheConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(!config.enable_search);
        assert!(!config.enable_info);
        assert!(!config.enable_comments);
        assert!(!config.enable_pkgbuild);
        assert!(!config.enable_disk_cache);
        assert_eq!(config.memory_cache_size, 100);
        assert_eq!(config.search_ttl, Duration::from_secs(300));
        assert_eq!(config.info_ttl, Duration::from_secs(900));
        assert_eq!(config.comments_ttl, Duration::from_secs(600));
        assert_eq!(config.pkgbuild_ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfigBuilder::new()
            .enable_search(true)
            .search_ttl(Duration::from_secs(600))
            .enable_info(true)
            .info_ttl(Duration::from_secs(1800))
            .memory_cache_size(200)
            .enable_disk_cache(true)
            .build();

        assert!(config.enable_search);
        assert!(config.enable_info);
        assert_eq!(config.search_ttl, Duration::from_secs(600));
        assert_eq!(config.info_ttl, Duration::from_secs(1800));
        assert_eq!(config.memory_cache_size, 200);
        assert!(config.enable_disk_cache);
    }
}
