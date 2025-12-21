//! Caching layer for arch-toolkit operations.
//!
//! Provides optional in-memory and disk caching for AUR operations to reduce
//! network requests and improve performance.

#[cfg(feature = "aur")]
mod config;
#[cfg(feature = "cache-disk")]
#[cfg(feature = "aur")]
mod disk;
#[cfg(feature = "aur")]
mod memory;

#[cfg(feature = "aur")]
pub use config::{CacheConfig, CacheConfigBuilder};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// What: Trait for cache implementations.
///
/// Inputs:
/// - `K`: Cache key type (must be `AsRef<str>`)
/// - `V`: Value type (must be `Clone + Serialize + Deserialize`)
///
/// Output:
/// - Various methods return `Option<V>` for cache hits or `Result` for operations
///
/// Details:
/// - Provides abstract interface for cache operations
/// - Supports get, set, invalidate, and clear operations
/// - Generic over value type to support different cached data
#[cfg(feature = "aur")]
pub trait Cache<K, V>
where
    K: AsRef<str>,
    V: Clone + Serialize + for<'de> Deserialize<'de>,
{
    /// What: Get a value from the cache.
    ///
    /// Inputs:
    /// - `key`: Cache key to look up
    ///
    /// Output:
    /// - `Option<V>` containing cached value if found and not expired, `None` otherwise
    ///
    /// Details:
    /// - Returns `None` if key not found or entry has expired
    /// - Should check TTL before returning value
    fn get(&self, key: &K) -> Option<V>;

    /// What: Store a value in the cache.
    ///
    /// Inputs:
    /// - `key`: Cache key
    /// - `value`: Value to cache
    /// - `ttl`: Time-to-live duration
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Stores value with specified TTL
    /// - May evict entries if cache is full (LRU)
    /// - Should not block the main operation
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Serialization)` if value serialization fails
    /// - Returns `Err(CacheError::Io)` if disk cache write fails (disk cache only)
    fn set(&self, key: &K, value: &V, ttl: Duration) -> Result<(), CacheError>;

    /// What: Invalidate a specific cache entry.
    ///
    /// Inputs:
    /// - `key`: Cache key to invalidate
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Removes the entry from cache
    /// - Safe to call if key doesn't exist
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Io)` if disk cache file removal fails (disk cache only)
    fn invalidate(&self, key: &K) -> Result<(), CacheError>;

    /// What: Clear all entries from the cache.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Removes all entries from cache
    /// - Useful for cache invalidation operations
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Io)` if disk cache cleanup fails (disk cache only)
    fn clear(&self) -> Result<(), CacheError>;
}

/// What: Error type for cache operations.
///
/// Inputs: None
///
/// Output:
/// - `CacheError` enum with various error variants
///
/// Details:
/// - Represents errors that can occur during cache operations
/// - Includes serialization, I/O, and other errors
#[cfg(feature = "aur")]
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Serialization error when storing cache entry.
    #[error("Cache serialization error: {0}")]
    Serialization(String),

    /// Deserialization error when loading cache entry.
    #[error("Cache deserialization error: {0}")]
    Deserialization(String),

    /// I/O error when accessing disk cache.
    #[error("Cache I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Other cache-related errors.
    #[error("Cache error: {0}")]
    Other(String),
}

/// What: Generate cache key for search operation.
///
/// Inputs:
/// - `query`: Search query string
///
/// Output:
/// - `String` containing normalized cache key
///
/// Details:
/// - Normalizes query by trimming whitespace
/// - Format: `"search:{query}"`
#[cfg(feature = "aur")]
#[must_use]
pub fn cache_key_search(query: &str) -> String {
    let trimmed = query.trim();
    format!("search:{trimmed}")
}

/// What: Generate cache key for info operation.
///
/// Inputs:
/// - `names`: Slice of package names
///
/// Output:
/// - `String` containing normalized cache key
///
/// Details:
/// - Sorts package names for consistent keys
/// - Format: `"info:{sorted_names}"`
#[cfg(feature = "aur")]
#[must_use]
pub fn cache_key_info(names: &[&str]) -> String {
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    format!("info:{}", sorted.join(","))
}

/// What: Generate cache key for comments operation.
///
/// Inputs:
/// - `pkgname`: Package name
///
/// Output:
/// - `String` containing normalized cache key
///
/// Details:
/// - Format: `"comments:{pkgname}"`
#[cfg(feature = "aur")]
#[must_use]
pub fn cache_key_comments(pkgname: &str) -> String {
    format!("comments:{pkgname}")
}

/// What: Generate cache key for pkgbuild operation.
///
/// Inputs:
/// - `package`: Package name
///
/// Output:
/// - `String` containing normalized cache key
///
/// Details:
/// - Format: `"pkgbuild:{package}"`
#[cfg(feature = "aur")]
#[must_use]
pub fn cache_key_pkgbuild(package: &str) -> String {
    format!("pkgbuild:{package}")
}

#[cfg(feature = "aur")]
use memory::MemoryCache;

#[cfg(feature = "cache-disk")]
#[cfg(feature = "aur")]
use disk::DiskCache;

/// What: Combined cache wrapper that uses memory and optionally disk cache.
///
/// Inputs: None (created via `CacheWrapper::new()`)
///
/// Output:
/// - `CacheWrapper` instance ready for use
///
/// Details:
/// - Always uses in-memory LRU cache
/// - Optionally uses disk cache if enabled in config
/// - Checks memory cache first, then disk cache
/// - Writes to both caches when storing
#[cfg(feature = "aur")]
#[derive(Debug)]
pub struct CacheWrapper {
    /// In-memory LRU cache.
    memory: MemoryCache,
    /// Optional disk cache.
    #[cfg(feature = "cache-disk")]
    disk: Option<DiskCache>,
}

#[cfg(feature = "aur")]
impl CacheWrapper {
    /// What: Create a new cache wrapper with memory and optional disk cache.
    ///
    /// Inputs:
    /// - `config`: Cache configuration
    ///
    /// Output:
    /// - `Result<CacheWrapper>` with configured caches, or error if initialization fails
    ///
    /// Details:
    /// - Always creates memory cache
    /// - Creates disk cache if enabled in config
    /// - Returns error if disk cache creation fails
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Io)` if disk cache directory creation fails
    pub fn new(config: &CacheConfig) -> Result<Self, CacheError> {
        let memory = MemoryCache::new(config.memory_cache_size);
        #[cfg(feature = "cache-disk")]
        {
            let disk = if config.enable_disk_cache {
                Some(DiskCache::new().map_err(CacheError::Io)?)
            } else {
                None
            };
            Ok(Self { memory, disk })
        }
        #[cfg(not(feature = "cache-disk"))]
        {
            Ok(Self { memory })
        }
    }

    /// What: Get a value from cache (checks memory first, then disk).
    ///
    /// Inputs:
    /// - `key`: Cache key
    ///
    /// Output:
    /// - `Option<V>` containing cached value if found, `None` otherwise
    ///
    /// Details:
    /// - Checks memory cache first (fastest)
    /// - Falls back to disk cache if memory miss
    /// - Promotes disk cache hits to memory cache
    #[must_use]
    pub fn get<V>(&self, key: &str) -> Option<V>
    where
        V: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        let key_str = key.to_string();
        // Try memory cache first
        if let Some(value) = <MemoryCache as Cache<String, V>>::get(&self.memory, &key_str) {
            return Some(value);
        }

        // Try disk cache if available
        #[cfg(feature = "cache-disk")]
        if let Some(ref disk) = self.disk
            && let Some(value) = <DiskCache as Cache<String, V>>::get(disk, &key_str)
        {
            // Promote to memory cache
            let _ = <MemoryCache as Cache<String, V>>::set(
                &self.memory,
                &key_str,
                &value,
                Duration::from_secs(300),
            );
            return Some(value);
        }

        None
    }

    /// What: Store a value in cache (writes to both memory and disk if enabled).
    ///
    /// Inputs:
    /// - `key`: Cache key
    /// - `value`: Value to cache
    /// - `ttl`: Time-to-live duration
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Writes to memory cache (always)
    /// - Writes to disk cache if enabled
    /// - Errors in disk cache don't prevent memory cache write
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Serialization)` if value serialization fails
    pub fn set<V>(&self, key: &str, value: &V, ttl: Duration) -> Result<(), CacheError>
    where
        V: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        let key_str = key.to_string();
        // Always write to memory cache
        <MemoryCache as Cache<String, V>>::set(&self.memory, &key_str, value, ttl)?;

        // Write to disk cache if enabled
        #[cfg(feature = "cache-disk")]
        if let Some(ref disk) = self.disk {
            let _ = <DiskCache as Cache<String, V>>::set(disk, &key_str, value, ttl);
        }

        Ok(())
    }

    /// What: Invalidate a cache entry (removes from both memory and disk).
    ///
    /// Inputs:
    /// - `key`: Cache key to invalidate
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Removes from memory cache
    /// - Removes from disk cache if enabled
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Io)` if disk cache file removal fails (disk cache only)
    pub fn invalidate(&self, key: &str) -> Result<(), CacheError> {
        let key_str = key.to_string();
        <MemoryCache as Cache<String, ()>>::invalidate(&self.memory, &key_str)?;
        #[cfg(feature = "cache-disk")]
        if let Some(ref disk) = self.disk {
            let _ = <DiskCache as Cache<String, ()>>::invalidate(disk, &key_str);
        }
        Ok(())
    }

    /// What: Clear all cache entries (both memory and disk).
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Clears memory cache
    /// - Clears disk cache if enabled
    ///
    /// # Errors
    /// - Returns `Err(CacheError::Io)` if disk cache cleanup fails (disk cache only)
    pub fn clear(&self) -> Result<(), CacheError> {
        <MemoryCache as Cache<String, ()>>::clear(&self.memory)?;
        #[cfg(feature = "cache-disk")]
        if let Some(ref disk) = self.disk {
            let _ = <DiskCache as Cache<String, ()>>::clear(disk);
        }
        Ok(())
    }
}
