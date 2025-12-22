## 1. **Client Builder Pattern with Configuration**

Currently, functions accept a raw `reqwest::Client`. A builder pattern would be much more ergonomic:

```rust
// Current (verbose for users)
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("my-app/1.0")
    .build()?;
let results = aur::search(&client, "yay").await?;

// Improved (simple defaults, customizable)
let client = ArchClient::builder()
    .timeout(Duration::from_secs(30))
    .user_agent("my-app/1.0")
    .build()?;
let results = client.aur().search("yay").await?;
```

## 2. **Retry Policies with Configurable Strategy**

Beyond rate limiting, transient network failures need automatic retries:
- Configurable max retries (default: 3)
- Exponential backoff with jitter
- Retry only on specific error codes (5xx, timeouts)

## 3. **Optional Caching Layer**

For repeated queries (common in TUI apps like Pacsea):
- In-memory LRU cache with TTL
- Optional disk cache for PKGBUILDs
- Cache invalidation API

## 4. **Testability: Trait-based Design**

Make it easy to mock in tests:
```rust
#[async_trait]
pub trait AurApi {
    async fn search(&self, query: &str) -> Result<Vec<AurPackage>>;
    async fn info(&self, names: &[&str]) -> Result<Vec<AurPackageDetails>>;
    // ...
}
```

This allows users to create mock implementations for unit tests without hitting real APIs.

## 5. **Rich Error Context**

Current errors lose context. Improve with:
```rust
#[error("AUR search failed for query '{query}': {source}")]
SearchFailed {
    query: String,
    #[source]
    source: reqwest::Error,
}
```

## 6. **Input Validation Layer**

Validate inputs before making network requests:
- Package name format validation (no special chars)
- Query length limits
- Empty input handling with clear errors

## 7. **Health Check / Connectivity Test**

```rust
// Quick check if AUR is reachable
client.health_check().await?;
```

Useful for apps to show connection status.

## 8. **Prelude Module**

Simplify imports:
```rust
use arch_toolkit::prelude::*;
// Imports: ArchClient, AurPackage, AurPackageDetails, Error, Result
```

## 9. **Examples Directory**

Runnable examples demonstrating common patterns:
- `examples/search.rs` - Basic search
- `examples/batch_info.rs` - Fetching multiple packages
- `examples/with_caching.rs` - Using the cache layer
- `examples/custom_client.rs` - Custom configuration

## 10. **Environment Variable Configuration**

Support `ARCH_TOOLKIT_TIMEOUT`, `ARCH_TOOLKIT_USER_AGENT`, etc. for CI/CD and containerized environments.

---

Would you like me to create a detailed plan for implementing some or all of these improvements? If so, which ones are most important to you? I'd suggest prioritizing:

1. **Client Builder Pattern** (most impactful for DX)
2. **Testability Traits** (critical for library adoption)
3. **Rich Error Context** (debugging ease)
4. **Examples Directory** (documentation by example)

---

## Implementation Status

### ✅ Implemented

1. **Client Builder Pattern with Configuration** - ✅ DONE
   - `ArchClient::builder()` with `timeout()`, `user_agent()`, `retry_policy()`, `cache_config()` methods
   - `ArchClient::new()` for default configuration
   - Method chaining API: `client.aur().search()`, `client.aur().info()`, etc.

2. **Retry Policies with Configurable Strategy** - ✅ DONE
   - `RetryPolicy` struct with configurable max retries (default: 3)
   - Exponential backoff with jitter implemented
   - Retry only on specific error codes (5xx, timeouts) via `is_retryable_error()`
   - Per-operation retry configuration

3. **Optional Caching Layer** - ✅ DONE
   - In-memory LRU cache with TTL (`MemoryCache`)
   - Optional disk cache for PKGBUILDs (via `cache-disk` feature)
   - Cache invalidation API (`CacheInvalidator` with per-operation invalidation methods)
   - `CacheConfig` with per-operation enable/disable and TTL configuration

4. **Examples Directory** - ✅ DONE
   - `examples/aur_example.rs` - Comprehensive AUR operations example
   - `examples/with_caching.rs` - Caching layer usage example
   - Examples demonstrate builder pattern, retry policies, and caching

### ❌ Not Yet Implemented

4. **Testability: Trait-based Design** - ❌ TODO
   - No `AurApi` trait exists
   - Current implementation uses concrete types, making mocking difficult

5. **Rich Error Context** - ⚠️ PARTIAL
   - Basic error types exist (`ArchToolkitError::Network`, `InvalidInput`, etc.)
   - Missing operation-specific error variants with context (e.g., `SearchFailed { query, source }`)
   - Errors don't preserve query/package name context in error messages

6. **Input Validation Layer** - ❌ TODO
   - No package name format validation
   - No query length limits
   - Empty input handling exists but could be more explicit

7. **Health Check / Connectivity Test** - ❌ TODO
   - No `health_check()` method on `ArchClient`
   - No connectivity test functionality

8. **Prelude Module** - ❌ TODO
   - No `prelude` module in `src/lib.rs`
   - Common types are re-exported at crate root but not via `prelude::*`

10. **Environment Variable Configuration** - ❌ TODO
   - No support for `ARCH_TOOLKIT_TIMEOUT`, `ARCH_TOOLKIT_USER_AGENT`, etc.
   - Builder doesn't read from environment variables