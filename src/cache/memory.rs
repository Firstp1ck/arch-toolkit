//! In-memory LRU cache implementation.

use super::{Cache, CacheError};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What: Cache entry with value and expiration timestamp.
///
/// Inputs: None (created internally)
///
/// Output:
/// - `CacheEntry<V>` containing cached value and expiration time
///
/// Details:
/// - Stores the cached value along with when it expires
/// - Used internally by `MemoryCache`
struct CacheEntry<V> {
    /// Cached value.
    value: V,
    /// Expiration timestamp.
    expires_at: Instant,
}

/// What: In-memory LRU cache with TTL expiration.
///
/// Inputs: None (created via `MemoryCache::new()`)
///
/// Output:
/// - `MemoryCache` instance ready for use
///
/// Details:
/// - Uses LRU eviction when cache is full
/// - Supports TTL-based expiration
/// - Thread-safe via `Arc<Mutex<>>`
/// - Generic over value type
#[derive(Debug)]
pub struct MemoryCache {
    /// Internal LRU cache wrapped in mutex for thread safety.
    cache: Arc<Mutex<LruCache<String, CacheEntry<Vec<u8>>>>>,
}

impl MemoryCache {
    /// What: Create a new memory cache with specified capacity.
    ///
    /// Inputs:
    /// - `capacity`: Maximum number of entries in cache
    ///
    /// Output:
    /// - `MemoryCache` instance
    ///
    /// Details:
    /// - Creates LRU cache with specified capacity
    /// - Wraps in `Arc<Mutex<>>` for thread safety
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity.max(1))
                    .expect("capacity.max(1) should always be >= 1"),
            ))),
        }
    }

    /// What: Clean up expired entries from cache.
    ///
    /// Inputs: None
    ///
    /// Output: None
    ///
    /// Details:
    /// - Removes all expired entries
    /// - Called automatically during get operations
    /// - May be called manually for cleanup
    fn cleanup_expired(&self) {
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let now = Instant::now();
        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.expires_at < now)
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            cache.pop(&key);
        }
    }
}

impl<K, V> Cache<K, V> for MemoryCache
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
    /// - Checks if entry exists and is not expired
    /// - Returns `None` if expired or not found
    /// - Automatically cleans up expired entries
    fn get(&self, key: &K) -> Option<V> {
        self.cleanup_expired();

        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let key_str = key.as_ref();
        let entry = cache.get(key_str)?;

        // Check if expired
        if entry.expires_at < Instant::now() {
            cache.pop(key_str);
            return None;
        }

        // Clone the serialized value before dropping the guard
        let value = entry.value.clone();
        drop(cache); // Early drop of guard

        // Deserialize value
        serde_json::from_slice(&value).ok()
    }

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
    /// - Serializes value to JSON bytes
    /// - Stores with expiration timestamp
    /// - May evict least recently used entry if cache is full
    fn set(&self, key: &K, value: &V, ttl: Duration) -> Result<(), CacheError> {
        let serialized =
            serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))?;

        let expires_at = Instant::now() + ttl;
        let entry = CacheEntry {
            value: serialized,
            expires_at,
        };
        let key_string = key.as_ref().to_string();

        {
            let mut cache = match self.cache.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            cache.put(key_string, entry);
        }
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
    /// - Removes the entry from cache
    /// - Safe to call if key doesn't exist
    fn invalidate(&self, key: &K) -> Result<(), CacheError> {
        let key_str = key.as_ref();
        {
            let mut cache = match self.cache.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            cache.pop(key_str);
        }
        Ok(())
    }

    /// What: Clear all entries from the cache.
    ///
    /// Inputs: None
    ///
    /// Output:
    /// - `Result<(), CacheError>` indicating success or failure
    ///
    /// Details:
    /// - Removes all entries from cache
    fn clear(&self) -> Result<(), CacheError> {
        {
            let mut cache = match self.cache.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            cache.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_memory_cache_get_set() {
        let cache = MemoryCache::new(10);
        let key = "test_key".to_string();
        let value = "test_value".to_string();

        // Initially empty
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &key).is_none());

        // Set value
        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &key,
            &value,
            StdDuration::from_secs(60),
        )
        .unwrap();

        // Get value
        let retrieved = <MemoryCache as Cache<String, String>>::get(&cache, &key);
        assert_eq!(retrieved, Some(value));
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_memory_cache_ttl_expiration() {
        let cache = MemoryCache::new(10);
        let key = "test_key".to_string();
        let value = "test_value".to_string();

        // Set value with short TTL
        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &key,
            &value,
            StdDuration::from_millis(100),
        )
        .unwrap();

        // Should be available immediately
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &key).is_some());

        // Wait for expiration
        thread::sleep(StdDuration::from_millis(150));

        // Should be expired
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &key).is_none());
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_memory_cache_lru_eviction() {
        let cache = MemoryCache::new(2);
        let value1 = "value1".to_string();
        let value2 = "value2".to_string();
        let value3 = "value3".to_string();

        // Fill cache to capacity
        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &"key1".to_string(),
            &value1,
            StdDuration::from_secs(60),
        )
        .unwrap();
        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &"key2".to_string(),
            &value2,
            StdDuration::from_secs(60),
        )
        .unwrap();

        // Access key1 to make it more recently used
        <MemoryCache as Cache<String, String>>::get(&cache, &"key1".to_string());

        // Add third entry - should evict key2 (least recently used)
        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &"key3".to_string(),
            &value3,
            StdDuration::from_secs(60),
        )
        .unwrap();

        // key1 and key3 should be present, key2 should be evicted
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &"key1".to_string()).is_some());
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &"key2".to_string()).is_none());
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &"key3".to_string()).is_some());
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_memory_cache_invalidate() {
        let cache = MemoryCache::new(10);
        let key = "test_key".to_string();
        let value = "test_value".to_string();

        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &key,
            &value,
            StdDuration::from_secs(60),
        )
        .unwrap();
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &key).is_some());

        <MemoryCache as Cache<String, String>>::invalidate(&cache, &key).unwrap();
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &key).is_none());
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_memory_cache_clear() {
        let cache = MemoryCache::new(10);
        let value = "test_value".to_string();

        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &"key1".to_string(),
            &value,
            StdDuration::from_secs(60),
        )
        .unwrap();
        <MemoryCache as Cache<String, String>>::set(
            &cache,
            &"key2".to_string(),
            &value,
            StdDuration::from_secs(60),
        )
        .unwrap();

        <MemoryCache as Cache<String, String>>::clear(&cache).unwrap();

        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &"key1".to_string()).is_none());
        assert!(<MemoryCache as Cache<String, String>>::get(&cache, &"key2".to_string()).is_none());
    }

    // Allow unwrap in tests - test failures should panic
    #[allow(clippy::unwrap_used)]
    #[test]
    fn test_memory_cache_thread_safety() {
        let cache = Arc::new(MemoryCache::new(100));
        let mut handles = vec![];

        // Spawn multiple threads writing to cache
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let key = format!("key_{i}_{j}");
                    let value = format!("value_{i}_{j}");
                    <MemoryCache as Cache<String, String>>::set(
                        &cache_clone,
                        &key,
                        &value,
                        StdDuration::from_secs(60),
                    )
                    .unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all entries are present
        for i in 0..10 {
            for j in 0..10 {
                let key = format!("key_{i}_{j}");
                let expected = format!("value_{i}_{j}");
                let retrieved = <MemoryCache as Cache<String, String>>::get(&cache, &key);
                assert_eq!(retrieved, Some(expected));
            }
        }
    }
}
