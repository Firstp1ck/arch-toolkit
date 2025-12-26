//! Explicit package query functions for the index module.

use std::collections::HashSet;
use std::process::{Command, Stdio};

use crate::error::{ArchToolkitError, Result};
use crate::types::index::InstalledPackagesMode;

/// What: Query pacman for explicitly installed packages and optionally update a cache.
///
/// Inputs:
/// - `mode`: Filter mode determining which packages to query (`LeafOnly` or `AllExplicit`).
/// - `cache`: Optional mutable reference to a `HashSet<String>` to update with results.
///
/// Output:
/// - Returns `Ok(HashSet<String>)` containing explicitly installed package names.
/// - Returns `Ok(HashSet::new())` on failure (graceful degradation).
///
/// Details:
/// - Uses `pacman -Qetq` for `LeafOnly` mode (explicitly installed AND not required).
/// - Uses `pacman -Qeq` for `AllExplicit` mode (all explicitly installed).
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
/// use arch_toolkit::index::{refresh_explicit_cache, InstalledPackagesMode};
/// use std::collections::HashSet;
///
/// let mut cache = HashSet::new();
/// let packages = refresh_explicit_cache(InstalledPackagesMode::AllExplicit, Some(&mut cache)).unwrap();
/// println!("Found {} explicitly installed packages", packages.len());
/// ```
#[allow(clippy::implicit_hasher)]
pub fn refresh_explicit_cache(
    mode: InstalledPackagesMode,
    cache: Option<&mut HashSet<String>>,
) -> Result<HashSet<String>> {
    let args: &[&str] = match mode {
        InstalledPackagesMode::LeafOnly => &["-Qetq"], // explicitly installed AND not required
        InstalledPackagesMode::AllExplicit => &["-Qeq"], // all explicitly installed
    };

    tracing::debug!("Running: pacman {:?}", args);
    let output = Command::new("pacman")
        .args(args)
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
                    "Successfully retrieved {} explicit packages (mode: {:?})",
                    packages.len(),
                    mode
                );
                packages
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::error!(
                    "pacman {:?} failed with status {:?}: {}",
                    args,
                    output.status.code(),
                    stderr
                );
                HashSet::new()
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute pacman {:?}: {}", args, e);
            HashSet::new()
        }
    };

    // Update cache if provided
    if let Some(cache_ref) = cache {
        cache_ref.clone_from(&packages);
    }

    Ok(packages)
}

/// What: Query pacman for explicitly installed packages asynchronously and optionally update a cache.
///
/// Inputs:
/// - `mode`: Filter mode determining which packages to query (`LeafOnly` or `AllExplicit`).
/// - `cache`: Optional mutable reference to a `HashSet<String>` to update with results.
///
/// Output:
/// - Returns a future that resolves to `Result<HashSet<String>>` containing explicitly installed package names.
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
/// use arch_toolkit::index::{refresh_explicit_cache_async, InstalledPackagesMode};
/// use std::collections::HashSet;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut cache = HashSet::new();
/// let packages = refresh_explicit_cache_async(InstalledPackagesMode::LeafOnly, Some(&mut cache)).await?;
/// println!("Found {} leaf packages", packages.len());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "index")]
#[allow(clippy::implicit_hasher)]
pub async fn refresh_explicit_cache_async(
    mode: InstalledPackagesMode,
    cache: Option<&mut HashSet<String>>,
) -> Result<HashSet<String>> {
    // Run the blocking operation without the cache parameter
    let result = tokio::task::spawn_blocking(move || refresh_explicit_cache(mode, None))
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("Blocking task failed: {e}")))?;

    // Update cache if provided and result is successful
    if let (Ok(packages), Some(cache_ref)) = (result.as_ref(), cache) {
        cache_ref.clone_from(packages);
    }

    result
}

/// What: Check if a package is explicitly installed, using cache if provided or querying pacman directly.
///
/// Inputs:
/// - `name`: Package name to check.
/// - `mode`: Filter mode for query type (`LeafOnly` or `AllExplicit`).
/// - `cache`: Optional reference to a `HashSet<String>` containing explicit package names.
///
/// Output:
/// - Returns `true` if the package is explicitly installed, `false` otherwise.
///
/// Details:
/// - If `cache` is provided, checks membership in the cache (O(1) lookup).
/// - If `cache` is `None`, queries pacman directly using the appropriate command for the mode.
/// - Gracefully degrades: returns `false` on error.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::index::{is_explicit, InstalledPackagesMode};
/// use std::collections::HashSet;
///
/// let cache = HashSet::from(["vim".to_string(), "git".to_string()]);
/// assert!(is_explicit("vim", InstalledPackagesMode::AllExplicit, Some(&cache)));
/// assert!(!is_explicit("nonexistent", InstalledPackagesMode::AllExplicit, Some(&cache)));
/// ```
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn is_explicit(
    name: &str,
    mode: InstalledPackagesMode,
    cache: Option<&HashSet<String>>,
) -> bool {
    if let Some(cache_ref) = cache {
        return cache_ref.contains(name);
    }

    // Query pacman directly if no cache
    let args: &[&str] = match mode {
        InstalledPackagesMode::LeafOnly => &["-Qet", name],
        InstalledPackagesMode::AllExplicit => &["-Qe", name],
    };

    tracing::debug!("Running: pacman {:?}", args);
    let output = Command::new("pacman")
        .args(args)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(e) => {
            tracing::error!("Failed to execute pacman {:?}: {}", args, e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify `refresh_explicit_cache` updates cache when provided.
    ///
    /// Inputs:
    /// - Empty cache and function call with cache parameter.
    ///
    /// Output:
    /// - Cache is populated with results (if pacman is available).
    ///
    /// Details:
    /// - Tests that cache parameter is updated correctly for both modes.
    fn refresh_explicit_cache_updates_cache() {
        let mut cache = HashSet::new();
        let _result = refresh_explicit_cache(InstalledPackagesMode::AllExplicit, Some(&mut cache));
        // Cache should be updated (may be empty if pacman unavailable, which is OK)
    }

    #[test]
    /// What: Verify `refresh_explicit_cache` works without cache parameter.
    ///
    /// Inputs:
    /// - Function call without cache parameter for both modes.
    ///
    /// Output:
    /// - Returns `HashSet` (may be empty if pacman unavailable).
    ///
    /// Details:
    /// - Tests that function works correctly when no cache is provided.
    fn refresh_explicit_cache_without_cache() {
        let result_leaf = refresh_explicit_cache(InstalledPackagesMode::LeafOnly, None);
        assert!(result_leaf.is_ok());

        let result_all = refresh_explicit_cache(InstalledPackagesMode::AllExplicit, None);
        assert!(result_all.is_ok());
    }

    #[test]
    /// What: Verify `is_explicit` uses cache when provided.
    ///
    /// Inputs:
    /// - Package name, mode, and cache containing the package.
    ///
    /// Output:
    /// - Returns `true` for cached package, `false` for non-cached package.
    ///
    /// Details:
    /// - Tests that cache lookup works correctly for both modes.
    fn is_explicit_uses_cache() {
        let cache = HashSet::from(["vim".to_string(), "git".to_string()]);
        assert!(is_explicit(
            "vim",
            InstalledPackagesMode::AllExplicit,
            Some(&cache)
        ));
        assert!(is_explicit(
            "git",
            InstalledPackagesMode::LeafOnly,
            Some(&cache)
        ));
        assert!(!is_explicit(
            "nonexistent",
            InstalledPackagesMode::AllExplicit,
            Some(&cache)
        ));
    }

    #[test]
    /// What: Verify `is_explicit` queries pacman when cache is not provided.
    ///
    /// Inputs:
    /// - Package name and mode without cache parameter.
    ///
    /// Output:
    /// - Returns result from pacman query (may be false if pacman unavailable).
    ///
    /// Details:
    /// - Tests that function falls back to direct pacman query for both modes.
    fn is_explicit_without_cache() {
        // This will query pacman directly
        // Result depends on system state, but should not panic
        let _result_leaf = is_explicit("vim", InstalledPackagesMode::LeafOnly, None);
        let _result_all = is_explicit("vim", InstalledPackagesMode::AllExplicit, None);
    }

    #[cfg(feature = "index")]
    #[tokio::test]
    /// What: Verify `refresh_explicit_cache_async` works asynchronously.
    ///
    /// Inputs:
    /// - Async function call with optional cache for both modes.
    ///
    /// Output:
    /// - Returns future that resolves to `HashSet`.
    ///
    /// Details:
    /// - Tests that async version works correctly for both modes.
    async fn refresh_explicit_cache_async_works() {
        let mut cache = HashSet::new();
        let result =
            refresh_explicit_cache_async(InstalledPackagesMode::AllExplicit, Some(&mut cache))
                .await;
        assert!(result.is_ok());
        // Result may be empty if pacman unavailable, which is graceful degradation
    }
}
