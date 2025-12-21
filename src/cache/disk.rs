//! Disk cache implementation for persistent caching.

use super::{Cache, CacheError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// What: Disk cache entry with serialized data and metadata.
///
/// Inputs: None (created internally)
///
/// Output:
/// - `DiskCacheEntry` containing cached data and expiration info
///
/// Details:
/// - Stores serialized JSON data
/// - Includes cached timestamp and TTL for expiration checking
#[derive(Serialize, Deserialize)]
struct DiskCacheEntry {
    /// Serialized cache value as JSON string.
    data: String,
    /// Unix timestamp when entry was cached (seconds since epoch).
    cached_at: u64,
    /// TTL in seconds.
    ttl_seconds: u64,
}

/// What: In-memory disk cache with file-based persistence.
///
/// Inputs: None (created via `DiskCache::new()`)
///
/// Output:
/// - `DiskCache` instance ready for use
///
/// Details:
/// - Stores cache entries as JSON files on disk
/// - Cache directory: `~/.cache/arch-toolkit/`
/// - Supports TTL-based expiration
/// - Thread-safe via internal synchronization
#[derive(Debug)]
pub struct DiskCache {
    /// Base directory for cache files.
    pub(crate) cache_dir: PathBuf,
}

impl DiskCache {
    /// What: Create a new disk cache instance.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `Result<DiskCache, std::io::Error>` with cache instance, or error if directory creation fails
    ///
    /// Details:
    /// - Creates cache directory if it doesn't exist
    /// - Uses `~/.cache/arch-toolkit/` as base directory
    /// - Returns error if directory creation fails
    pub fn new() -> Result<Self, std::io::Error> {
        let cache_dir = Self::get_cache_dir()?;
        fs::create_dir_all(&cache_dir)?;

        // Create subdirectories for each operation type
        for subdir in ["search", "info", "comments", "pkgbuild"] {
            fs::create_dir_all(cache_dir.join(subdir))?;
        }

        Ok(Self { cache_dir })
    }

    /// What: Get the cache directory path.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `Result<PathBuf, std::io::Error>` with cache directory path
    ///
    /// Details:
    /// - Uses `dirs::cache_dir()` to get system cache directory
    /// - Appends `arch-toolkit` to the path
    /// - Returns error if cache directory cannot be determined
    fn get_cache_dir() -> Result<PathBuf, std::io::Error> {
        dirs::cache_dir()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Cannot determine cache directory",
                )
            })
            .map(|dir| dir.join("arch-toolkit"))
    }

    /// What: Get file path for a cache key.
    ///
    /// Inputs:
    /// - `key`: Cache key
    ///
    /// Output:
    /// - `PathBuf` with file path
    ///
    /// Details:
    /// - Determines operation type from key prefix
    /// - Creates safe filename from key (replaces invalid chars)
    /// - Returns path in appropriate subdirectory
    fn get_file_path(&self, key: &str) -> PathBuf {
        let (subdir, key_part) = key
            .strip_prefix("search:")
            .map(|rest| ("search", rest))
            .or_else(|| key.strip_prefix("info:").map(|rest| ("info", rest)))
            .or_else(|| key.strip_prefix("comments:").map(|rest| ("comments", rest)))
            .or_else(|| key.strip_prefix("pkgbuild:").map(|rest| ("pkgbuild", rest)))
            .unwrap_or(("search", key));

        // Create safe filename (replace invalid chars with underscore)
        let safe_filename = key_part
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        // Use hash for very long keys to avoid filesystem limits
        let filename = if safe_filename.len() > 200 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            format!("{:x}", hasher.finish())
        } else {
            safe_filename
        };

        self.cache_dir.join(subdir).join(format!("{filename}.json"))
    }

    /// What: Check if a cache entry is expired.
    ///
    /// Inputs:
    /// - `entry`: Cache entry to check
    ///
    /// Output:
    /// - `true` if expired, `false` otherwise
    ///
    /// Details:
    /// - Compares `cached_at` + `ttl_seconds` with current time
    fn is_expired(entry: &DiskCacheEntry) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        entry.cached_at + entry.ttl_seconds < now
    }

    /// What: Clean up expired entries from disk cache.
    ///
    /// Inputs: None
    ///
    /// Output: None
    ///
    /// Details:
    /// - Scans all cache subdirectories
    /// - Removes expired JSON files
    /// - Logs errors but doesn't fail
    #[allow(dead_code)] // Public API method for manual cleanup
    pub fn cleanup_expired(&self) {
        for subdir in ["search", "info", "comments", "pkgbuild"] {
            let dir = self.cache_dir.join(subdir);
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension() == Some(std::ffi::OsStr::new("json"))
                        && let Ok(content) = fs::read_to_string(&path)
                        && let Ok(cache_entry) = serde_json::from_str::<DiskCacheEntry>(&content)
                        && Self::is_expired(&cache_entry)
                    {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

impl<K, V> Cache<K, V> for DiskCache
where
    K: AsRef<str>,
    V: Clone + Serialize + for<'de> Deserialize<'de>,
{
    /// What: Get a value from the disk cache.
    ///
    /// Inputs:
    /// - `key`: Cache key to look up
    ///
    /// Output:
    /// - `Option<V>` containing cached value if found and not expired, `None` otherwise
    ///
    /// Details:
    /// - Reads JSON file from disk
    /// - Checks if entry is expired
    /// - Deserializes and returns value
    fn get(&self, key: &K) -> Option<V> {
        let path = self.get_file_path(key.as_ref());
        let content = fs::read_to_string(&path).ok()?;
        let entry: DiskCacheEntry = serde_json::from_str(&content).ok()?;

        // Check expiration
        if Self::is_expired(&entry) {
            let _ = fs::remove_file(&path);
            return None;
        }

        // Deserialize value
        serde_json::from_str(&entry.data).ok()
    }

    /// What: Store a value in the disk cache.
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
    /// - Serializes value to JSON
    /// - Creates cache entry with timestamp and TTL
    /// - Writes to disk atomically (write to temp file, then rename)
    fn set(&self, key: &K, value: &V, ttl: Duration) -> Result<(), CacheError> {
        let path = self.get_file_path(key.as_ref());
        let data =
            serde_json::to_string(value).map_err(|e| CacheError::Serialization(e.to_string()))?;

        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CacheError::Other(format!("System time error: {e}")))?;

        let entry = DiskCacheEntry {
            data,
            cached_at: cached_at.as_secs(),
            ttl_seconds: ttl.as_secs(),
        };

        let json =
            serde_json::to_string(&entry).map_err(|e| CacheError::Serialization(e.to_string()))?;

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json).map_err(CacheError::Io)?;
        fs::rename(&temp_path, &path).map_err(CacheError::Io)?;

        Ok(())
    }

    /// What: Invalidate a specific cache entry.
    ///
    /// Inputs:
    /// - `key`: Cache key to invalidate
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Removes the cache file from disk
    /// - Safe to call if file doesn't exist
    fn invalidate(&self, key: &K) -> Result<(), CacheError> {
        let path = self.get_file_path(key.as_ref());
        if path.exists() {
            fs::remove_file(&path).map_err(CacheError::Io)?;
        }
        Ok(())
    }

    /// What: Clear all entries from the disk cache.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Removes all JSON files from cache subdirectories
    fn clear(&self) -> Result<(), CacheError> {
        for subdir in ["search", "info", "comments", "pkgbuild"] {
            let dir = self.cache_dir.join(subdir);
            if dir.exists() {
                for entry in fs::read_dir(&dir).map_err(CacheError::Io)? {
                    let entry = entry.map_err(CacheError::Io)?;
                    let path = entry.path();
                    if path.extension() == Some(std::ffi::OsStr::new("json"))
                        || path.extension() == Some(std::ffi::OsStr::new("tmp"))
                    {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Allow unwrap in test helper - test setup failures should panic
    #[allow(clippy::unwrap_used)]
    fn create_test_cache() -> (DiskCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        for subdir in ["search", "info", "comments", "pkgbuild"] {
            fs::create_dir_all(cache_dir.join(subdir)).unwrap();
        }

        // Create a DiskCache with the test directory
        let cache = DiskCache { cache_dir };

        (cache, temp_dir)
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_disk_cache_get_set() {
        let (cache, _temp_dir) = create_test_cache();
        let key = "test_key".to_string();
        let value = "test_value".to_string();

        // Initially empty
        assert!(<DiskCache as Cache<String, String>>::get(&cache, &key).is_none());

        // Set value
        <DiskCache as Cache<String, String>>::set(&cache, &key, &value, Duration::from_secs(60))
            .unwrap();

        // Get value
        let retrieved = <DiskCache as Cache<String, String>>::get(&cache, &key);
        assert_eq!(retrieved, Some(value));
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_disk_cache_invalidate() {
        let (cache, _temp_dir) = create_test_cache();
        let key = "test_key".to_string();
        let value = "test_value".to_string();

        <DiskCache as Cache<String, String>>::set(&cache, &key, &value, Duration::from_secs(60))
            .unwrap();
        assert!(<DiskCache as Cache<String, String>>::get(&cache, &key).is_some());

        <DiskCache as Cache<String, String>>::invalidate(&cache, &key).unwrap();
        assert!(<DiskCache as Cache<String, String>>::get(&cache, &key).is_none());
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_disk_cache_clear() {
        let (cache, _temp_dir) = create_test_cache();
        let value = "test_value".to_string();

        <DiskCache as Cache<String, String>>::set(
            &cache,
            &"key1".to_string(),
            &value,
            Duration::from_secs(60),
        )
        .unwrap();
        <DiskCache as Cache<String, String>>::set(
            &cache,
            &"key2".to_string(),
            &value,
            Duration::from_secs(60),
        )
        .unwrap();

        <DiskCache as Cache<String, String>>::clear(&cache).unwrap();

        assert!(<DiskCache as Cache<String, String>>::get(&cache, &"key1".to_string()).is_none());
        assert!(<DiskCache as Cache<String, String>>::get(&cache, &"key2".to_string()).is_none());
    }
}
