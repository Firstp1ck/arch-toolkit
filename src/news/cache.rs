//! Generic cache boundary for news feed payloads.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::error::{ArchToolkitError, Result};

/// What: Define caller-owned storage for raw news and advisory feed payloads.
///
/// Inputs:
/// - `key`: Stable feed key supplied by the news fetch helper.
/// - `value`: Raw successful feed payload to store.
///
/// Output:
/// - Cached payloads on hits and explicit errors from a caller's storage backend.
///
/// Details:
/// - The boundary is intentionally independent from the AUR cache and makes no
///   assumptions about persistence, expiration, encryption, or eviction.
/// - Callers own freshness policy; a cache implementation can decline a stale
///   entry by returning `Ok(None)`.
pub trait FeedCache: Send + Sync {
    /// What: Look up a raw feed payload by a stable key.
    ///
    /// Inputs:
    /// - `key`: Namespaced key generated from the feed kind and URL.
    ///
    /// Output:
    /// - `Ok(Some(payload))` for a cache hit, `Ok(None)` for a miss, or an
    ///   error when the backing store cannot be read.
    ///
    /// Details:
    /// - Implementations should enforce their own expiration policy before
    ///   returning a payload.
    ///
    /// # Errors
    ///
    /// Returns a backing-store error when the requested payload cannot be read.
    fn get(&self, key: &str) -> Result<Option<String>>;

    /// What: Store a raw feed payload under a stable key.
    ///
    /// Inputs:
    /// - `key`: Namespaced key generated from the feed kind and URL.
    /// - `value`: Successful bounded response body to store.
    ///
    /// Output:
    /// - `Ok(())` when the value is stored, otherwise a backing-store error.
    ///
    /// Details:
    /// - Feed fetches surface write errors explicitly so callers do not mistake
    ///   an unavailable requested cache for a successful durable cache write.
    ///
    /// # Errors
    ///
    /// Returns a backing-store error when the payload cannot be stored.
    fn put(&self, key: &str, value: &str) -> Result<()>;
}

/// What: Provide a small deterministic in-memory implementation of [`FeedCache`].
///
/// Inputs:
/// - Constructed with [`InMemoryFeedCache::new`] and a maximum entry count.
///
/// Output:
/// - A thread-safe cache suitable for short-lived applications and tests.
///
/// Details:
/// - This implementation has no time-to-live policy; use a caller-provided
///   cache when persistence or expiration is required.
/// - On capacity pressure it removes the lexically first key, keeping eviction
///   deterministic without adding an AUR-coupled cache dependency.
#[derive(Debug)]
pub struct InMemoryFeedCache {
    /// Maximum number of payloads retained by this cache.
    capacity: usize,
    /// Payloads ordered by key for deterministic eviction.
    entries: Mutex<BTreeMap<String, String>>,
}

impl InMemoryFeedCache {
    /// What: Create an empty in-memory feed cache with an explicit capacity.
    ///
    /// Inputs:
    /// - `capacity`: Maximum number of feed payloads that may be retained.
    ///
    /// Output:
    /// - A ready-to-use cache, or `InvalidInput` when capacity is zero.
    ///
    /// Details:
    /// - The explicit non-zero bound prevents unbounded cache construction and
    ///   keeps memory ownership visible to callers.
    ///
    /// # Errors
    ///
    /// Returns `ArchToolkitError::InvalidInput` when `capacity` is zero.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(ArchToolkitError::InvalidInput(
                "feed cache capacity must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            capacity,
            entries: Mutex::new(BTreeMap::new()),
        })
    }
}

impl FeedCache for InMemoryFeedCache {
    /// What: Return a cloned payload for the requested feed key.
    ///
    /// Inputs:
    /// - `key`: Feed cache key to look up.
    ///
    /// Output:
    /// - The stored payload when present, otherwise `None`.
    ///
    /// Details:
    /// - Recovers the in-memory cache after a poisoned mutex because payloads
    ///   are independent values and the map remains usable.
    fn get(&self, key: &str) -> Result<Option<String>> {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(entries.get(key).cloned())
    }

    /// What: Store a payload and evict deterministically when full.
    ///
    /// Inputs:
    /// - `key`: Feed cache key to write.
    /// - `value`: Raw successful feed payload.
    ///
    /// Output:
    /// - `Ok(())` after the in-memory map is updated.
    ///
    /// Details:
    /// - Updating an existing key does not evict another entry.
    /// - A new key evicts the lexically first existing key only at capacity.
    fn put(&self, key: &str, value: &str) -> Result<()> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !entries.contains_key(key) && entries.len() == self.capacity {
            let _ = entries.pop_first();
        }
        entries.insert(key.to_string(), value.to_string());
        drop(entries);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FeedCache, InMemoryFeedCache};

    #[test]
    /// What: Verify bounded cache storage and deterministic eviction.
    ///
    /// Inputs:
    /// - A capacity-one cache and two lexically ordered feed keys.
    ///
    /// Output:
    /// - The first key is evicted and the newer key remains accessible.
    ///
    /// Details:
    /// - Demonstrates that this generic cache has no dependency on AUR cache
    ///   configuration or types.
    fn cache_evicts_deterministically() {
        let cache = InMemoryFeedCache::new(1).expect("valid cache capacity");
        cache.put("arch-news:a", "first").expect("store first");
        cache.put("arch-news:b", "second").expect("store second");

        assert_eq!(cache.get("arch-news:a").expect("read first"), None);
        assert_eq!(
            cache.get("arch-news:b").expect("read second"),
            Some("second".to_string())
        );
    }

    #[test]
    /// What: Verify zero-capacity caches are rejected explicitly.
    ///
    /// Inputs:
    /// - A zero entry capacity.
    ///
    /// Output:
    /// - An invalid-input error rather than an unbounded or unusable cache.
    ///
    /// Details:
    /// - Keeps the cache capacity bound enforceable at construction time.
    fn cache_rejects_zero_capacity() {
        assert!(InMemoryFeedCache::new(0).is_err());
    }
}
