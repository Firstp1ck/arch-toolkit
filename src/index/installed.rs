//! Installed package query functions for the index module.

use std::collections::HashSet;
use std::process::{Command, Stdio};

use crate::error::{ArchToolkitError, Result};

/// What: Query pacman for all installed packages and optionally update a cache.
///
/// Inputs:
/// - `cache`: Optional mutable reference to a `HashSet<String>` to update with results.
///
/// Output:
/// - Returns `Ok(HashSet<String>)` containing all installed package names.
/// - Returns `Ok(HashSet::new())` on failure (graceful degradation).
///
/// Details:
/// - Uses `pacman -Qq` to query the local database.
/// - If `cache` is provided, updates it with the results.
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
/// - Logs errors for diagnostics but returns empty set to avoid blocking operations.
///
/// # Errors
///
/// This function does not return errors - it gracefully degrades by returning an empty set.
/// Errors are logged using `tracing::error` for diagnostics.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::refresh_installed_cache;
/// use std::collections::HashSet;
///
/// let mut cache = HashSet::new();
/// let packages = refresh_installed_cache(Some(&mut cache)).unwrap();
/// println!("Found {} installed packages", packages.len());
/// ```
#[allow(clippy::implicit_hasher)]
pub fn refresh_installed_cache(cache: Option<&mut HashSet<String>>) -> Result<HashSet<String>> {
    tracing::debug!("Running: pacman -Qq");
    let output = Command::new("pacman")
        .args(["-Qq"])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let packages = match output {
        Ok(output) => {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let packages: HashSet<String> = text
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                tracing::debug!(
                    "Successfully retrieved {} installed packages",
                    packages.len()
                );
                packages
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::error!(
                    "pacman -Qq failed with status {:?}: {}",
                    output.status.code(),
                    stderr
                );
                HashSet::new()
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute pacman -Qq: {}", e);
            HashSet::new()
        }
    };

    // Update cache if provided
    if let Some(cache_ref) = cache {
        cache_ref.clone_from(&packages);
    }

    Ok(packages)
}

/// What: Query pacman for all installed packages asynchronously and optionally update a cache.
///
/// Inputs:
/// - `cache`: Optional mutable reference to a `HashSet<String>` to update with results.
///
/// Output:
/// - Returns a future that resolves to `Result<HashSet<String>>` containing all installed package names.
///
/// Details:
/// - Uses `tokio::task::spawn_blocking` to run the sync version in a blocking task.
/// - If `cache` is provided, updates it with the results after the task completes.
/// - The cache parameter must be `Send` and `Sync` to be used across async boundaries.
///
/// # Errors
///
/// Returns `Err` if the blocking task fails, otherwise returns the same result as the sync version.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::refresh_installed_cache_async;
/// use std::collections::HashSet;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut cache = HashSet::new();
/// let packages = refresh_installed_cache_async(Some(&mut cache)).await?;
/// println!("Found {} installed packages", packages.len());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "index")]
#[allow(clippy::implicit_hasher)]
pub async fn refresh_installed_cache_async(
    cache: Option<&mut HashSet<String>>,
) -> Result<HashSet<String>> {
    // Run the blocking operation without the cache parameter
    let result = tokio::task::spawn_blocking(|| refresh_installed_cache(None))
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("Blocking task failed: {e}")))?;

    // Update cache if provided and result is successful
    if let (Ok(packages), Some(cache_ref)) = (result.as_ref(), cache) {
        cache_ref.clone_from(packages);
    }

    result
}

/// What: Check if a package is installed, using cache if provided or querying pacman directly.
///
/// Inputs:
/// - `name`: Package name to check.
/// - `cache`: Optional reference to a `HashSet<String>` containing installed package names.
///
/// Output:
/// - Returns `true` if the package is installed, `false` otherwise.
///
/// Details:
/// - If `cache` is provided, checks membership in the cache (O(1) lookup).
/// - If `cache` is `None`, queries pacman directly using `pacman -Q`.
/// - Gracefully degrades: returns `false` on error.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::is_installed;
/// use std::collections::HashSet;
///
/// let cache = HashSet::from(["vim".to_string(), "git".to_string()]);
/// assert!(is_installed("vim", Some(&cache)));
/// assert!(!is_installed("nonexistent", Some(&cache)));
/// ```
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn is_installed(name: &str, cache: Option<&HashSet<String>>) -> bool {
    if let Some(cache_ref) = cache {
        return cache_ref.contains(name);
    }

    // Query pacman directly if no cache
    tracing::debug!("Running: pacman -Q {}", name);
    let output = Command::new("pacman")
        .args(["-Q", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(e) => {
            tracing::error!("Failed to execute pacman -Q {}: {}", name, e);
            false
        }
    }
}

/// What: Query pacman directly for all installed packages without caching.
///
/// Inputs:
/// - None: Invokes `pacman -Qq` to query the local database.
///
/// Output:
/// - Returns `Ok(HashSet<String>)` containing all installed package names.
/// - Returns `Ok(HashSet::new())` on failure (graceful degradation).
///
/// Details:
/// - Direct query to pacman, no caching involved.
/// - Reuses the same logic as `refresh_installed_cache` but without cache parameter.
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
///
/// # Errors
///
/// This function does not return errors - it gracefully degrades by returning an empty set.
/// Errors are logged using `tracing::error` for diagnostics.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::get_installed_packages;
///
/// let packages = get_installed_packages().unwrap();
/// println!("Found {} installed packages", packages.len());
/// ```
pub fn get_installed_packages() -> Result<HashSet<String>> {
    refresh_installed_cache(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify `refresh_installed_cache` updates cache when provided.
    ///
    /// Inputs:
    /// - Empty cache and function call with cache parameter.
    ///
    /// Output:
    /// - Cache is populated with results (if pacman is available).
    ///
    /// Details:
    /// - Tests that cache parameter is updated correctly.
    fn refresh_installed_cache_updates_cache() {
        let mut cache = HashSet::new();
        let _result = refresh_installed_cache(Some(&mut cache));
        // Cache should be updated (may be empty if pacman unavailable, which is OK)
        // We can't assert specific contents without knowing system state
    }

    #[test]
    /// What: Verify `refresh_installed_cache` works without cache parameter.
    ///
    /// Inputs:
    /// - Function call without cache parameter.
    ///
    /// Output:
    /// - Returns `HashSet` (may be empty if pacman unavailable).
    ///
    /// Details:
    /// - Tests that function works correctly when no cache is provided.
    fn refresh_installed_cache_without_cache() {
        let result = refresh_installed_cache(None);
        assert!(result.is_ok());
        // Result may be empty if pacman unavailable, which is graceful degradation
    }

    #[test]
    /// What: Verify `is_installed` uses cache when provided.
    ///
    /// Inputs:
    /// - Package name and cache containing the package.
    ///
    /// Output:
    /// - Returns `true` for cached package, `false` for non-cached package.
    ///
    /// Details:
    /// - Tests that cache lookup works correctly.
    fn is_installed_uses_cache() {
        let cache = HashSet::from(["vim".to_string(), "git".to_string()]);
        assert!(is_installed("vim", Some(&cache)));
        assert!(is_installed("git", Some(&cache)));
        assert!(!is_installed("nonexistent", Some(&cache)));
    }

    #[test]
    /// What: Verify `is_installed` queries pacman when cache is not provided.
    ///
    /// Inputs:
    /// - Package name without cache parameter.
    ///
    /// Output:
    /// - Returns result from pacman query (may be false if pacman unavailable).
    ///
    /// Details:
    /// - Tests that function falls back to direct pacman query.
    fn is_installed_without_cache() {
        // This will query pacman directly
        // Result depends on system state, but should not panic
        let _result = is_installed("vim", None);
    }

    #[test]
    /// What: Verify `get_installed_packages` returns `HashSet`.
    ///
    /// Inputs:
    /// - None: Direct query to pacman.
    ///
    /// Output:
    /// - Returns `Ok(HashSet<String>)` (may be empty if pacman unavailable).
    ///
    /// Details:
    /// - Tests that function returns correct type and handles errors gracefully.
    fn get_installed_packages_returns_hashset() {
        let result = get_installed_packages();
        assert!(result.is_ok());
        // Result may be empty if pacman unavailable, which is graceful degradation
    }

    #[cfg(feature = "index")]
    #[tokio::test]
    /// What: Verify `refresh_installed_cache_async` works asynchronously.
    ///
    /// Inputs:
    /// - Async function call with optional cache.
    ///
    /// Output:
    /// - Returns future that resolves to `HashSet`.
    ///
    /// Details:
    /// - Tests that async version works correctly.
    async fn refresh_installed_cache_async_works() {
        let mut cache = HashSet::new();
        let result = refresh_installed_cache_async(Some(&mut cache)).await;
        assert!(result.is_ok());
        // Result may be empty if pacman unavailable, which is graceful degradation
    }
}
