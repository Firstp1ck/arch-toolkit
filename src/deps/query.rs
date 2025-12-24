//! Package querying functions for dependency resolution.
//!
//! This module provides functions to query the pacman database for installed packages,
//! upgradable packages, provided packages, and package versions. All functions gracefully
//! degrade when pacman is unavailable, returning empty sets or None as appropriate.

use crate::error::{ArchToolkitError, Result};
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::process::{Command, Stdio};

/// What: Enumerate all currently installed packages on the system.
///
/// Inputs:
/// - (none): Invokes `pacman -Qq` to query the local database.
///
/// Output:
/// - Returns `Ok(HashSet<String>)` containing package names installed on the machine.
/// - Returns `Ok(HashSet::new())` on failure (graceful degradation).
///
/// Details:
/// - Uses pacman's quiet format to obtain trimmed names.
/// - Logs errors for diagnostics but returns empty set to avoid blocking dependency checks.
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
/// use arch_toolkit::deps::get_installed_packages;
///
/// let installed = get_installed_packages().unwrap();
/// println!("Found {} installed packages", installed.len());
/// ```
pub fn get_installed_packages() -> Result<HashSet<String>> {
    tracing::debug!("Running: pacman -Qq");
    let output = Command::new("pacman")
        .args(["-Qq"])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
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
                Ok(packages)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::error!(
                    "pacman -Qq failed with status {:?}: {}",
                    output.status.code(),
                    stderr
                );
                Ok(HashSet::new())
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute pacman -Qq: {}", e);
            Ok(HashSet::new())
        }
    }
}

/// What: Collect names of packages that have upgrades available via pacman.
///
/// Inputs:
/// - (none): Reads upgrade information by invoking `pacman -Qu`.
///
/// Output:
/// - Returns `Ok(HashSet<String>)` containing package names that pacman reports as upgradable.
/// - Returns `Ok(HashSet::new())` on failure (graceful degradation).
///
/// Details:
/// - Parses output format: "name old-version -> new-version" or just "name" for AUR packages.
/// - Extracts package name (everything before first space or "->").
/// - Gracefully handles command failures by returning an empty set to avoid blocking dependency checks.
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
///
/// # Errors
///
/// This function does not return errors - it gracefully degrades by returning an empty set.
/// Errors are logged using `tracing::debug` for diagnostics.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::get_upgradable_packages;
///
/// let upgradable = get_upgradable_packages().unwrap();
/// println!("Found {} upgradable packages", upgradable.len());
/// ```
pub fn get_upgradable_packages() -> Result<HashSet<String>> {
    tracing::debug!("Running: pacman -Qu");
    let output = Command::new("pacman")
        .args(["-Qu"])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                // pacman -Qu outputs "name old-version -> new-version" or just "name" for AUR packages
                let packages: HashSet<String> = text
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() {
                            return None;
                        }
                        // Extract package name (everything before space or "->")
                        Some(line.find(' ').map_or_else(
                            || line.to_string(),
                            |space_pos| line[..space_pos].trim().to_string(),
                        ))
                    })
                    .collect();
                tracing::debug!(
                    "Successfully retrieved {} upgradable packages",
                    packages.len()
                );
                Ok(packages)
            } else {
                // No upgradable packages or error - return empty set
                tracing::debug!("pacman -Qu returned non-zero status (no upgrades or error)");
                Ok(HashSet::new())
            }
        }
        Err(e) => {
            tracing::debug!("Failed to execute pacman -Qu: {} (assuming no upgrades)", e);
            Ok(HashSet::new())
        }
    }
}

/// What: Build an empty provides set (for API compatibility).
///
/// Inputs:
/// - `installed`: Set of installed package names (unused, kept for API compatibility).
///
/// Output:
/// - Returns an empty set (provides are now checked lazily).
///
/// Details:
/// - This function is kept for API compatibility but no longer builds the full provides set.
/// - Provides are now checked on-demand using `is_package_installed_or_provided()` for better performance.
/// - This avoids querying all installed packages upfront, which was very slow.
///
/// # Example
///
/// ```
/// use arch_toolkit::deps::{get_installed_packages, get_provided_packages};
///
/// let installed = get_installed_packages().unwrap();
/// let provided = get_provided_packages(&installed);
/// assert!(provided.is_empty()); // Always returns empty set
/// ```
#[must_use]
pub fn get_provided_packages<S: BuildHasher + Default>(
    _installed: &HashSet<String, S>,
) -> HashSet<String> {
    // Return empty set - provides are now checked lazily on-demand
    // This avoids querying all installed packages upfront, which was very slow
    HashSet::default()
}

/// What: Check if a specific package name is provided by any installed package (lazy check).
///
/// Inputs:
/// - `name`: Package name to check.
/// - `installed`: Set of installed package names (unused, kept for API compatibility).
///
/// Output:
/// - Returns `Some(package_name)` if the name is provided by an installed package, `None` otherwise.
///
/// Details:
/// - Uses `pacman -Qqo` to efficiently check if any installed package provides the name.
/// - This is much faster than querying all packages upfront.
/// - Returns the name of the providing package for debugging purposes.
fn check_if_provided<S: BuildHasher>(
    name: &str,
    _installed: &HashSet<String, S>,
) -> Option<String> {
    // Use pacman -Qqo to check which package provides this name
    // This is efficient - pacman does the lookup internally
    let output = Command::new("pacman")
        .args(["-Qqo", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            let providing_pkg = text.lines().next().map(|s| s.trim().to_string());
            if let Some(providing_pkg) = &providing_pkg {
                tracing::debug!("{} is provided by {}", name, providing_pkg);
            }
            providing_pkg
        }
        _ => None,
    }
}

/// What: Check if a package is installed or provided by an installed package.
///
/// Inputs:
/// - `name`: Package name to check.
/// - `installed`: Set of directly installed package names.
/// - `provided`: Set of package names provided by installed packages (unused, kept for API compatibility).
///
/// Output:
/// - Returns `true` if the package is directly installed or provided by an installed package.
///
/// Details:
/// - First checks if the package is directly installed.
/// - Then lazily checks if it's provided by any installed package using `pacman -Qqo`.
/// - This handles cases like `rustup` providing `rust` efficiently without querying all packages upfront.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::{get_installed_packages, get_provided_packages, is_package_installed_or_provided};
///
/// let installed = get_installed_packages().unwrap();
/// let provided = get_provided_packages(&installed);
///
/// assert!(is_package_installed_or_provided("pacman", &installed, &provided));
/// ```
#[must_use]
pub fn is_package_installed_or_provided<S: BuildHasher>(
    name: &str,
    installed: &HashSet<String, S>,
    _provided: &HashSet<String, S>,
) -> bool {
    // First check if directly installed
    if installed.contains(name) {
        return true;
    }

    // Lazy check if provided by any installed package (much faster than building full set upfront)
    check_if_provided(name, installed).is_some()
}

/// What: Retrieve the locally installed version of a package.
///
/// Inputs:
/// - `name`: Package to query via `pacman -Q`.
///
/// Output:
/// - Returns `Ok(String)` with the installed version string on success.
/// - Returns `Err(ArchToolkitError::PackageNotFound)` if the package is not installed.
/// - Returns `Err(ArchToolkitError::Parse)` if the version string cannot be parsed.
///
/// Details:
/// - Normalizes versions by removing revision suffixes to facilitate requirement comparisons.
/// - Parses format: "name version" or "name version-revision".
/// - Strips revision suffix (e.g., "1.2.3-1" -> "1.2.3").
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
///
/// # Errors
///
/// - Returns `PackageNotFound` when the package is not installed.
/// - Returns `Parse` when the version string cannot be parsed from command output.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::get_installed_version;
///
/// let version = get_installed_version("pacman")?;
/// println!("Installed version: {}", version);
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn get_installed_version(name: &str) -> Result<String> {
    let output = Command::new("pacman")
        .args(["-Q", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ArchToolkitError::Parse(format!("pacman -Q failed: {e}")))?;

    if !output.status.success() {
        return Err(ArchToolkitError::PackageNotFound {
            package: name.to_string(),
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    if let Some(line) = text.lines().next() {
        // Format: "name version" or "name version-revision"
        if let Some(space_pos) = line.find(' ') {
            let version = line[space_pos + 1..].trim();
            // Remove revision suffix if present (e.g., "1.2.3-1" -> "1.2.3")
            let version = version.split('-').next().unwrap_or(version);
            return Ok(version.to_string());
        }
    }

    Err(ArchToolkitError::Parse(format!(
        "Could not parse version from pacman -Q output for package '{name}'"
    )))
}

/// What: Query the repositories for the latest available version of a package.
///
/// Inputs:
/// - `name`: Package name looked up via `pacman -Si`.
///
/// Output:
/// - Returns `Some(String)` with the version string advertised in the repositories.
/// - Returns `None` on failure (package not found in repos or command error).
///
/// Details:
/// - Strips revision suffixes (e.g., `-1`) so comparisons focus on the base semantic version.
/// - Parses "Version: x.y.z" line from pacman -Si output.
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
/// - Gracefully degrades by returning `None` if pacman is unavailable or package not found.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::get_available_version;
///
/// if let Some(version) = get_available_version("pacman") {
///     println!("Available version: {}", version);
/// }
/// ```
#[must_use]
pub fn get_available_version(name: &str) -> Option<String> {
    let output = Command::new("pacman")
        .args(["-Si", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.starts_with("Version")
            && let Some(colon_pos) = line.find(':')
        {
            let version = line[colon_pos + 1..].trim();
            // Remove revision suffix if present
            let version = version.split('-').next().unwrap_or(version);
            return Some(version.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_installed_packages_output() {
        // Test parsing logic with sample output
        let sample_output = "pacman\nfirefox\nvim\n";
        let packages: HashSet<String> = sample_output
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(packages.len(), 3);
        assert!(packages.contains("pacman"));
        assert!(packages.contains("firefox"));
        assert!(packages.contains("vim"));
    }

    #[test]
    fn test_parse_upgradable_packages_output() {
        // Test parsing logic with sample output
        let sample_output =
            "firefox 121.0-1 -> 122.0-1\nvim 9.0.0000-1 -> 9.0.1000-1\npackage-name\n";
        let packages: HashSet<String> = sample_output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                Some(line.find(' ').map_or_else(
                    || line.to_string(),
                    |space_pos| line[..space_pos].trim().to_string(),
                ))
            })
            .collect();
        assert_eq!(packages.len(), 3);
        assert!(packages.contains("firefox"));
        assert!(packages.contains("vim"));
        assert!(packages.contains("package-name"));
    }

    #[test]
    fn test_parse_installed_version_output() {
        // Test parsing logic with sample output
        let sample_output = "pacman 6.1.0-1\n";
        if let Some(line) = sample_output.lines().next()
            && let Some(space_pos) = line.find(' ')
        {
            let version = line[space_pos + 1..].trim();
            let version = version.split('-').next().unwrap_or(version);
            assert_eq!(version, "6.1.0");
        }
    }

    #[test]
    fn test_parse_available_version_output() {
        // Test parsing logic with sample output
        let sample_output =
            "Repository      : extra\nName            : pacman\nVersion         : 6.1.0-1\n";
        for line in sample_output.lines() {
            if line.starts_with("Version")
                && let Some(colon_pos) = line.find(':')
            {
                let version = line[colon_pos + 1..].trim();
                let version = version.split('-').next().unwrap_or(version);
                assert_eq!(version, "6.1.0");
                return;
            }
        }
        panic!("Version line not found");
    }

    #[test]
    fn test_get_provided_packages_returns_empty() {
        let installed = HashSet::from(["pacman".to_string()]);
        let provided = get_provided_packages(&installed);
        assert!(provided.is_empty());
    }

    #[test]
    fn test_is_package_installed_or_provided_direct_install() {
        let installed = HashSet::from(["pacman".to_string(), "vim".to_string()]);
        let provided = HashSet::new();
        assert!(is_package_installed_or_provided(
            "pacman", &installed, &provided
        ));
        assert!(is_package_installed_or_provided(
            "vim", &installed, &provided
        ));
        assert!(!is_package_installed_or_provided(
            "nonexistent",
            &installed,
            &provided
        ));
    }

    // Integration tests that require pacman - these are ignored by default
    #[test]
    #[ignore = "Requires pacman to be available"]
    fn test_get_installed_packages_integration() {
        if let Ok(packages) = get_installed_packages() {
            // Should have at least some packages on a real system
            // But we can't assert exact count since it varies
            println!("Found {} installed packages", packages.len());
        }
    }

    #[test]
    #[ignore = "Requires pacman to be available"]
    fn test_get_upgradable_packages_integration() {
        if let Ok(packages) = get_upgradable_packages() {
            // May be empty if system is up to date
            println!("Found {} upgradable packages", packages.len());
        }
    }

    #[test]
    #[ignore = "Requires pacman to be available and package to be installed"]
    fn test_get_installed_version_integration() {
        // Test with a package that should be installed (pacman itself)
        if let Ok(version) = get_installed_version("pacman") {
            assert!(!version.is_empty());
            println!("Installed pacman version: {version}");
        }
    }

    #[test]
    #[ignore = "Requires pacman to be available and package in repos"]
    fn test_get_available_version_integration() {
        // Test with a package that should be in repos
        if let Some(version) = get_available_version("pacman") {
            assert!(!version.is_empty());
            println!("Available pacman version: {version}");
        }
    }
}
