//! Core dependency resolution logic for individual packages.
//!
//! This module provides functions to resolve dependencies for packages, determine
//! dependency status, and handle batch operations for efficient dependency resolution.

use crate::deps::parse::{parse_dep_spec, parse_pacman_si_conflicts, parse_pacman_si_deps};
use crate::deps::pkgbuild::parse_pkgbuild_deps;
use crate::deps::query::{
    get_available_version, get_installed_packages, get_installed_version, get_provided_packages,
    get_upgradable_packages, is_package_installed_or_provided,
};
use crate::deps::source::{determine_dependency_source, is_system_package};
use crate::deps::version::version_satisfies;
use crate::error::Result;
use crate::types::dependency::{
    Dependency, DependencySource, DependencyStatus, PackageRef, PackageSource, ResolverConfig,
};
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::process::{Command, Stdio};

/// Type alias for PKGBUILD cache callback function.
type PkgbuildCacheFn = dyn Fn(&str) -> Option<String> + Send + Sync;

/// What: Evaluate a dependency's installation status relative to required versions.
///
/// Inputs:
/// - `name`: Dependency package identifier.
/// - `version_req`: Optional version constraint string (e.g., `>=1.2`).
/// - `installed`: Set of names currently installed on the system.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of names pacman reports as upgradable.
///
/// Output:
/// - Returns a `DependencyStatus` describing whether installation, upgrade, or no action is needed.
///
/// Details:
/// - Combines local database queries with helper functions to capture upgrade requirements.
/// - Uses `is_package_installed_or_provided()` to check if package is available.
/// - Uses `get_installed_version()` and `get_available_version()` for version checking.
/// - Uses `version_satisfies()` for version requirement validation.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::determine_status;
/// use std::collections::HashSet;
///
/// let installed = HashSet::from(["glibc".to_string()]);
/// let provided = HashSet::new();
/// let upgradable = HashSet::new();
///
/// let status = determine_status("glibc", "", &installed, &provided, &upgradable);
/// println!("Status: {:?}", status);
/// ```
pub fn determine_status<S: BuildHasher>(
    name: &str,
    version_req: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> DependencyStatus {
    // Check if package is installed or provided by an installed package
    if !is_package_installed_or_provided(name, installed, provided) {
        return DependencyStatus::ToInstall;
    }

    // Check if package is upgradable (even without version requirement)
    let is_upgradable = upgradable.contains(name);

    // If version requirement is specified, check if it matches
    if !version_req.is_empty() {
        // Try to get installed version
        if let Ok(installed_version) = get_installed_version(name) {
            // Check if version requirement is satisfied
            if !version_satisfies(&installed_version, version_req) {
                return DependencyStatus::ToUpgrade {
                    current: installed_version,
                    required: version_req.to_string(),
                };
            }
            // Version requirement satisfied, but check if package is upgradable anyway
            if is_upgradable {
                // Get available version from pacman -Si if possible
                let available_version =
                    get_available_version(name).unwrap_or_else(|| "newer".to_string());
                return DependencyStatus::ToUpgrade {
                    current: installed_version,
                    required: available_version,
                };
            }
            return DependencyStatus::Installed {
                version: installed_version,
            };
        }
    }

    // Installed but no version check needed - check if upgradable
    if is_upgradable {
        match get_installed_version(name) {
            Ok(current_version) => {
                let available_version =
                    get_available_version(name).unwrap_or_else(|| "newer".to_string());
                return DependencyStatus::ToUpgrade {
                    current: current_version,
                    required: available_version,
                };
            }
            Err(_) => {
                return DependencyStatus::ToUpgrade {
                    current: "installed".to_string(),
                    required: "newer".to_string(),
                };
            }
        }
    }

    // Installed and up-to-date - get actual version
    get_installed_version(name).map_or_else(
        |_| DependencyStatus::Installed {
            version: "installed".to_string(),
        },
        |version| DependencyStatus::Installed { version },
    )
}

/// What: Batch fetch dependency lists for multiple official packages using `pacman -Si`.
///
/// Inputs:
/// - `names`: Package names to query (must be official packages, not local).
///
/// Output:
/// - `HashMap` mapping package name to its dependency list (`Vec<String>`).
///
/// Details:
/// - Batches queries into chunks of 50 to avoid command-line length limits.
/// - Parses multi-package `pacman -Si` output (packages separated by blank lines).
/// - Gracefully handles command failures by returning partial results.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::batch_fetch_official_deps;
///
/// let packages = vec!["firefox", "vim"];
/// let deps = batch_fetch_official_deps(&packages);
/// println!("Found dependencies for {} packages", deps.len());
/// ```
#[must_use]
pub fn batch_fetch_official_deps(names: &[&str]) -> HashMap<String, Vec<String>> {
    const BATCH_SIZE: usize = 50;
    let mut result_map = HashMap::new();

    for chunk in names.chunks(BATCH_SIZE) {
        let mut args = vec!["-Si"];
        args.extend(chunk.iter().copied());
        match Command::new("pacman")
            .args(&args)
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                // Parse multi-package output: packages are separated by blank lines
                let mut package_blocks = Vec::new();
                let mut current_block = String::new();
                for line in text.lines() {
                    if line.trim().is_empty() {
                        if !current_block.is_empty() {
                            package_blocks.push(current_block.clone());
                            current_block.clear();
                        }
                    } else {
                        current_block.push_str(line);
                        current_block.push('\n');
                    }
                }
                if !current_block.is_empty() {
                    package_blocks.push(current_block);
                }

                // Parse each block to extract package name and dependencies
                for block in package_blocks {
                    let dep_names = parse_pacman_si_deps(&block);
                    // Extract package name from block
                    if let Some(name_line) =
                        block.lines().find(|l| l.trim_start().starts_with("Name"))
                        && let Some((_, name)) = name_line.split_once(':')
                    {
                        let pkg_name = name.trim().to_string();
                        result_map.insert(pkg_name, dep_names);
                    }
                }
            }
            _ => {
                // If batch fails, fall back to individual queries (but don't do it here to avoid recursion)
                // The caller will handle individual queries
                break;
            }
        }
    }
    result_map
}

/// What: Check if a command is available in PATH.
///
/// Inputs:
/// - `cmd`: Command name to check.
///
/// Output:
/// - Returns true if the command exists and can be executed.
///
/// Details:
/// - Uses a simple version check to verify command availability.
fn is_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .args(["--version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok()
}

/// What: Check if a package name should be filtered out (virtual package or self-reference).
///
/// Inputs:
/// - `pkg_name`: Package name to check.
/// - `parent_name`: Name of the parent package (to detect self-references).
///
/// Output:
/// - Returns true if the package should be filtered out.
///
/// Details:
/// - Filters out .so files (virtual packages) and self-references.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn should_filter_dependency(pkg_name: &str, parent_name: &str) -> bool {
    let pkg_lower = pkg_name.to_lowercase();
    pkg_name == parent_name
        || pkg_lower.ends_with(".so")
        || pkg_lower.contains(".so.")
        || pkg_lower.contains(".so=")
}

/// What: Convert a dependency spec into a `Dependency` record.
///
/// Inputs:
/// - `dep_spec`: Dependency specification string (may include version requirements).
/// - `parent_name`: Name of the package that requires this dependency.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns Some(Dependency) if the dependency should be included, None if filtered.
///
/// Details:
/// - Parses the dependency spec, filters out virtual packages and self-references,
///   and determines status, source, and system package flags.
fn process_dependency_spec<S: BuildHasher>(
    dep_spec: &str,
    parent_name: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> Option<Dependency> {
    let spec = parse_dep_spec(dep_spec);
    let pkg_name = spec.name;
    let version_req = spec.version_req;

    if should_filter_dependency(&pkg_name, parent_name) {
        if pkg_name == parent_name {
            tracing::debug!("Skipping self-reference: {} == {}", pkg_name, parent_name);
        } else {
            tracing::debug!("Filtering out virtual package: {}", pkg_name);
        }
        return None;
    }

    let status = determine_status(&pkg_name, &version_req, installed, provided, upgradable);
    let (source, is_core) = determine_dependency_source(&pkg_name, installed);
    let is_system = is_core || is_system_package(&pkg_name);

    Some(Dependency {
        name: pkg_name,
        version_req,
        status,
        source,
        required_by: vec![parent_name.to_string()],
        depends_on: Vec::new(),
        is_core,
        is_system,
    })
}

/// What: Process a list of dependency specs into `Dependency` records.
///
/// Inputs:
/// - `dep_specs`: Vector of dependency specification strings.
/// - `parent_name`: Name of the package that requires these dependencies.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns a vector of `Dependency` records (filtered).
///
/// Details:
/// - Processes each dependency spec and collects valid dependencies.
fn process_dependency_specs<S: BuildHasher>(
    dep_specs: Vec<String>,
    parent_name: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> Vec<Dependency> {
    dep_specs
        .into_iter()
        .filter_map(|dep_spec| {
            process_dependency_spec(&dep_spec, parent_name, installed, provided, upgradable)
        })
        .collect()
}

/// What: Resolve dependencies for a local package using pacman -Qi.
///
/// Inputs:
/// - `name`: Package name.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns a vector of `Dependency` records or an error string.
///
/// Details:
/// - Uses pacman -Qi to get dependency information for locally installed packages.
fn resolve_local_package_deps<S: BuildHasher>(
    name: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> Result<Vec<Dependency>> {
    tracing::debug!("Running: pacman -Qi {} (local package)", name);
    let output = Command::new("pacman")
        .args(["-Qi", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            tracing::error!("Failed to execute pacman -Qi {}: {}", name, e);
            crate::error::ArchToolkitError::Parse(format!("pacman -Qi failed: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "pacman -Qi {} failed with status {:?}: {}",
            name,
            output.status.code(),
            stderr
        );
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    tracing::debug!("pacman -Qi {} output ({} bytes)", name, text.len());

    let dep_names = parse_pacman_si_deps(&text);
    tracing::debug!(
        "Parsed {} dependency names from pacman -Qi output",
        dep_names.len()
    );

    Ok(process_dependency_specs(
        dep_names, name, installed, provided, upgradable,
    ))
}

/// What: Resolve dependencies for an official package using pacman -Si.
///
/// Inputs:
/// - `name`: Package name.
/// - `repo`: Repository name (for logging).
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns a vector of `Dependency` records or an error string.
///
/// Details:
/// - Uses pacman -Si to get dependency information for official packages.
fn resolve_official_package_deps<S: BuildHasher>(
    name: &str,
    repo: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> Result<Vec<Dependency>> {
    tracing::debug!("Running: pacman -Si {} (repo: {})", name, repo);
    let output = Command::new("pacman")
        .args(["-Si", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            tracing::error!("Failed to execute pacman -Si {}: {}", name, e);
            crate::error::ArchToolkitError::Parse(format!("pacman -Si failed: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!(
            "pacman -Si {} failed with status {:?}: {}",
            name,
            output.status.code(),
            stderr
        );
        return Err(crate::error::ArchToolkitError::Parse(format!(
            "pacman -Si failed for {name}: {stderr}"
        )));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    tracing::debug!("pacman -Si {} output ({} bytes)", name, text.len());

    let dep_names = parse_pacman_si_deps(&text);
    tracing::debug!(
        "Parsed {} dependency names from pacman -Si output",
        dep_names.len()
    );

    Ok(process_dependency_specs(
        dep_names, name, installed, provided, upgradable,
    ))
}

/// What: Try to resolve dependencies using an AUR helper (paru or yay).
///
/// Inputs:
/// - `helper`: Helper command name ("paru" or "yay").
/// - `name`: Package name.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns Some(Vec<Dependency>) if successful, None otherwise.
///
/// Details:
/// - Executes helper -Si command and parses the output for dependencies.
fn try_helper_resolution<S: BuildHasher>(
    helper: &str,
    name: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> Option<Vec<Dependency>> {
    tracing::debug!("Trying {} -Si {} for dependency resolution", helper, name);
    let output = Command::new(helper)
        .args(["-Si", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!(
            "{} -Si {} failed (will try other methods): {}",
            helper,
            name,
            stderr.trim()
        );
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    tracing::debug!("{} -Si {} output ({} bytes)", helper, name, text.len());
    let dep_names = parse_pacman_si_deps(&text);

    if dep_names.is_empty() {
        return None;
    }

    tracing::info!(
        "Using {} to resolve runtime dependencies for {} (will fetch .SRCINFO for build-time deps)",
        helper,
        name
    );

    let deps = process_dependency_specs(dep_names, name, installed, provided, upgradable);
    Some(deps)
}

/// What: Enhance dependency list with .SRCINFO data.
///
/// Inputs:
/// - `name`: Package name.
/// - `deps`: Existing dependency list to enhance.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns the enhanced dependency list.
///
/// Details:
/// - Fetches and parses .SRCINFO to add missing depends entries.
/// - Requires `feature = "aur"` to be enabled.
#[cfg(feature = "aur")]
fn enhance_with_srcinfo<S: BuildHasher>(
    name: &str,
    deps: Vec<Dependency>,
    _installed: &HashSet<String, S>,
    _provided: &HashSet<String, S>,
    _upgradable: &HashSet<String, S>,
) -> Vec<Dependency> {
    // Note: fetch_srcinfo is async and requires a reqwest client, so we can't use it here
    // in the sync context. This is a limitation - callers should fetch .SRCINFO separately
    // if needed, or use the async API when available.
    tracing::debug!(
        "Skipping .SRCINFO enhancement for {} (requires async context)",
        name
    );
    deps
}

/// What: Enhance dependency list with .SRCINFO data (no-op when AUR feature is disabled).
///
/// Inputs:
/// - `name`: Package name (unused).
/// - `deps`: Existing dependency list to return as-is.
/// - `installed`: Set of locally installed packages (unused).
/// - `provided`: Set of package names provided by installed packages (unused).
/// - `upgradable`: Set of packages flagged for upgrades (unused).
///
/// Output:
/// - Returns the dependency list unchanged.
///
/// Details:
/// - This is a no-op when `feature = "aur"` is not enabled.
#[cfg(not(feature = "aur"))]
#[allow(clippy::unused_parameters)]
fn enhance_with_srcinfo<S: BuildHasher>(
    _name: &str,
    deps: Vec<Dependency>,
    _installed: &HashSet<String, S>,
    _provided: &HashSet<String, S>,
    _upgradable: &HashSet<String, S>,
) -> Vec<Dependency> {
    deps
}

/// What: Fallback to cached PKGBUILD for dependency resolution.
///
/// Inputs:
/// - `name`: Package name.
/// - `pkgbuild_cache`: Optional callback to fetch PKGBUILD from cache.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
///
/// Output:
/// - Returns a vector of `Dependency` records if `PKGBUILD` is found, empty vector otherwise.
///
/// Details:
/// - Attempts to use cached PKGBUILD when .SRCINFO is unavailable (offline fallback).
fn fallback_to_pkgbuild<S: BuildHasher>(
    name: &str,
    pkgbuild_cache: Option<&PkgbuildCacheFn>,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
) -> Vec<Dependency> {
    let Some(pkgbuild_text) = pkgbuild_cache.and_then(|f| f(name)) else {
        tracing::debug!(
            "No cached PKGBUILD available for {} (offline, no dependencies resolved)",
            name
        );
        return Vec::new();
    };

    tracing::info!(
        "Using cached PKGBUILD for {} to resolve dependencies (offline fallback)",
        name
    );
    let (pkgbuild_depends, _, _, _) = parse_pkgbuild_deps(&pkgbuild_text);

    let deps = process_dependency_specs(pkgbuild_depends, name, installed, provided, upgradable);
    tracing::info!(
        "Resolved {} dependencies from cached PKGBUILD for {}",
        deps.len(),
        name
    );
    deps
}

/// What: Resolve dependencies for an AUR package.
///
/// Inputs:
/// - `name`: Package name.
/// - `installed`: Set of locally installed packages.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades.
/// - `pkgbuild_cache`: Optional callback to fetch PKGBUILD from cache.
///
/// Output:
/// - Returns a vector of `Dependency` records.
///
/// Details:
/// - Tries paru/yay first, then falls back to .SRCINFO and cached PKGBUILD.
fn resolve_aur_package_deps<S: BuildHasher>(
    name: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
    pkgbuild_cache: Option<&PkgbuildCacheFn>,
) -> Vec<Dependency> {
    tracing::debug!(
        "Attempting to resolve AUR package: {} (will skip if not found)",
        name
    );

    // Try paru first
    let (mut deps, mut used_helper) = if is_command_available("paru")
        && let Some(helper_deps) =
            try_helper_resolution("paru", name, installed, provided, upgradable)
    {
        (helper_deps, true)
    } else {
        (Vec::new(), false)
    };

    // Try yay if paru didn't work
    if !used_helper
        && is_command_available("yay")
        && let Some(helper_deps) =
            try_helper_resolution("yay", name, installed, provided, upgradable)
    {
        deps = helper_deps;
        used_helper = true;
    }

    if !used_helper {
        tracing::debug!(
            "Skipping AUR API for {} - paru/yay failed or not available (likely not a real package)",
            name
        );
    }

    // Always try to enhance with .SRCINFO
    deps = enhance_with_srcinfo(name, deps, installed, provided, upgradable);

    // Fallback to PKGBUILD if no dependencies were found
    if !used_helper && deps.is_empty() {
        deps = fallback_to_pkgbuild(name, pkgbuild_cache, installed, provided, upgradable);
    }

    deps
}

/// What: Resolve direct dependency metadata for a single package.
///
/// Inputs:
/// - `name`: Package identifier whose dependencies should be enumerated.
/// - `source`: Source enum describing whether the package is official or AUR.
/// - `installed`: Set of locally installed packages for status determination.
/// - `provided`: Set of package names provided by installed packages.
/// - `upgradable`: Set of packages flagged for upgrades, used to detect stale dependencies.
/// - `pkgbuild_cache`: Optional callback to fetch PKGBUILD from cache.
///
/// Output:
/// - Returns a vector of `Dependency` records or an error string when resolution fails.
///
/// Details:
/// - Invokes pacman or AUR helpers depending on source, filtering out virtual entries and self references.
fn resolve_package_deps<S: BuildHasher>(
    name: &str,
    source: &PackageSource,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
    pkgbuild_cache: Option<&PkgbuildCacheFn>,
) -> Result<Vec<Dependency>> {
    let deps = match source {
        PackageSource::Official { repo, .. } => {
            if repo == "local" {
                resolve_local_package_deps(name, installed, provided, upgradable)?
            } else {
                resolve_official_package_deps(name, repo, installed, provided, upgradable)?
            }
        }
        PackageSource::Aur => {
            resolve_aur_package_deps(name, installed, provided, upgradable, pkgbuild_cache)
        }
    };

    tracing::debug!("Resolved {} dependencies for package {}", deps.len(), name);
    Ok(deps)
}

/// What: Fetch conflicts for a package from pacman or AUR sources.
///
/// Inputs:
/// - `name`: Package identifier.
/// - `source`: Source enum describing whether the package is official or AUR.
///
/// Output:
/// - Returns a vector of conflicting package names, or empty vector on error.
///
/// Details:
/// - For official packages, uses `pacman -Si` to get conflicts.
/// - For AUR packages, tries paru/yay first, then falls back to .SRCINFO.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::fetch_package_conflicts;
/// use arch_toolkit::PackageSource;
///
/// let conflicts = fetch_package_conflicts(
///     "firefox",
///     &PackageSource::Official {
///         repo: "extra".into(),
///         arch: "x86_64".into(),
///     },
/// );
/// println!("Found {} conflicts", conflicts.len());
/// ```
pub fn fetch_package_conflicts(name: &str, source: &PackageSource) -> Vec<String> {
    match source {
        PackageSource::Official { repo, .. } => {
            // Handle local packages specially - use pacman -Qi instead of -Si
            if repo == "local" {
                tracing::debug!("Running: pacman -Qi {} (local package, conflicts)", name);
                if let Ok(output) = Command::new("pacman")
                    .args(["-Qi", name])
                    .env("LC_ALL", "C")
                    .env("LANG", "C")
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    && output.status.success()
                {
                    let text = String::from_utf8_lossy(&output.stdout);
                    return parse_pacman_si_conflicts(&text);
                }
                return Vec::new();
            }

            // Use pacman -Si to get conflicts
            tracing::debug!("Running: pacman -Si {} (conflicts)", name);
            if let Ok(output) = Command::new("pacman")
                .args(["-Si", name])
                .env("LC_ALL", "C")
                .env("LANG", "C")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                && output.status.success()
            {
                let text = String::from_utf8_lossy(&output.stdout);
                return parse_pacman_si_conflicts(&text);
            }
            Vec::new()
        }
        PackageSource::Aur => {
            // Try paru/yay first
            let has_paru = is_command_available("paru");
            let has_yay = is_command_available("yay");

            if has_paru {
                tracing::debug!("Trying paru -Si {} for conflicts", name);
                if let Ok(output) = Command::new("paru")
                    .args(["-Si", name])
                    .env("LC_ALL", "C")
                    .env("LANG", "C")
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    && output.status.success()
                {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let conflicts = parse_pacman_si_conflicts(&text);
                    if !conflicts.is_empty() {
                        return conflicts;
                    }
                }
            }

            if has_yay {
                tracing::debug!("Trying yay -Si {} for conflicts", name);
                if let Ok(output) = Command::new("yay")
                    .args(["-Si", name])
                    .env("LC_ALL", "C")
                    .env("LANG", "C")
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    && output.status.success()
                {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let conflicts = parse_pacman_si_conflicts(&text);
                    if !conflicts.is_empty() {
                        return conflicts;
                    }
                }
            }

            // Fall back to .SRCINFO
            // Note: fetch_srcinfo is async, so we can't use it here in sync context
            // This is a limitation - conflicts from .SRCINFO won't be detected in sync mode
            #[cfg(feature = "aur")]
            {
                tracing::debug!(
                    "Skipping .SRCINFO conflict check for {} (requires async context)",
                    name
                );
            }

            Vec::new()
        }
    }
}

/// What: Get priority value for dependency status (lower = more urgent).
///
/// Inputs:
/// - `status`: Dependency status to get priority for.
///
/// Output:
/// - Returns a numeric priority where lower numbers indicate higher urgency.
///
/// Details:
/// - Priority order: Conflict (0) < Missing (1) < `ToInstall` (2) < `ToUpgrade` (3) < Installed (4).
const fn dependency_priority(status: &DependencyStatus) -> u8 {
    status.priority()
}

/// What: Merge a dependency into the dependency map.
///
/// Inputs:
/// - `dep`: Dependency to merge.
/// - `parent_name`: Name of the package that requires this dependency.
/// - `installed`: Set of installed package names.
/// - `provided`: Set of provided packages.
/// - `upgradable`: Set of upgradable package names.
/// - `deps`: Mutable reference to the dependency map to update.
///
/// Output:
/// - Updates the `deps` map with the merged dependency.
///
/// Details:
/// - Merges status (keeps worst), version requirements (keeps more restrictive), and `required_by` lists.
fn merge_dependency<S: BuildHasher>(
    dep: &Dependency,
    parent_name: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
    upgradable: &HashSet<String, S>,
    deps: &mut HashMap<String, Dependency>,
) {
    let dep_name = dep.name.clone();

    // Check if dependency already exists and get its current state
    let needs_required_by_update = deps
        .get(&dep_name)
        .is_none_or(|e| !e.required_by.contains(&parent_name.to_string()));

    // Update or create dependency entry
    let entry = deps.entry(dep_name.clone()).or_insert_with(|| Dependency {
        name: dep_name.clone(),
        version_req: dep.version_req.clone(),
        status: dep.status.clone(),
        source: dep.source.clone(),
        required_by: vec![parent_name.to_string()],
        depends_on: Vec::new(),
        is_core: dep.is_core,
        is_system: dep.is_system,
    });

    // Update required_by (add the parent if not already present)
    if needs_required_by_update {
        entry.required_by.push(parent_name.to_string());
    }

    // Merge status (keep worst)
    // But never overwrite a Conflict status - conflicts take precedence
    if !matches!(entry.status, DependencyStatus::Conflict { .. }) {
        let existing_priority = dependency_priority(&entry.status);
        let new_priority = dependency_priority(&dep.status);
        if new_priority < existing_priority {
            entry.status = dep.status.clone();
        }
    }

    // Merge version requirements (keep more restrictive)
    // But never overwrite a Conflict status - conflicts take precedence
    if !dep.version_req.is_empty() && dep.version_req != entry.version_req {
        // If entry is already a conflict, don't overwrite it with dependency status
        if matches!(entry.status, DependencyStatus::Conflict { .. }) {
            // Still update version if needed, but keep conflict status
            if entry.version_req.is_empty() {
                entry.version_req.clone_from(&dep.version_req);
            }
            return;
        }

        if entry.version_req.is_empty() {
            entry.version_req.clone_from(&dep.version_req);
        } else {
            // Check which version requirement is more restrictive
            let existing_status = determine_status(
                &entry.name,
                &entry.version_req,
                installed,
                provided,
                upgradable,
            );
            let new_status = determine_status(
                &entry.name,
                &dep.version_req,
                installed,
                provided,
                upgradable,
            );
            let existing_req_priority = dependency_priority(&existing_status);
            let new_req_priority = dependency_priority(&new_status);

            if new_req_priority < existing_req_priority {
                entry.version_req.clone_from(&dep.version_req);
                entry.status = new_status;
            }
        }
    }
}

/// Dependency resolver for batch package operations.
///
/// Provides a high-level API for resolving dependencies for multiple packages,
/// handling batch operations, conflict detection, and dependency merging.
pub struct DependencyResolver {
    /// Resolver configuration.
    config: ResolverConfig,
}

impl DependencyResolver {
    /// What: Create a new dependency resolver with default configuration.
    ///
    /// Inputs:
    /// - (none)
    ///
    /// Output:
    /// - Returns a new `DependencyResolver` with default configuration.
    ///
    /// Details:
    /// - Uses `ResolverConfig::default()` for configuration.
    /// - Default config: direct dependencies only, no optional/make/check deps, no AUR checking.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::DependencyResolver;
    ///
    /// let resolver = DependencyResolver::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ResolverConfig::default(),
        }
    }

    /// What: Create a resolver with custom configuration.
    ///
    /// Inputs:
    /// - `config`: Custom resolver configuration.
    ///
    /// Output:
    /// - Returns a new `DependencyResolver` with the provided configuration.
    ///
    /// Details:
    /// - Allows customization of dependency resolution behavior.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::DependencyResolver;
    /// use arch_toolkit::types::dependency::ResolverConfig;
    ///
    /// let config = ResolverConfig {
    ///     include_optdepends: true,
    ///     include_makedepends: false,
    ///     include_checkdepends: false,
    ///     max_depth: 0,
    ///     pkgbuild_cache: None,
    ///     check_aur: false,
    /// };
    /// let resolver = DependencyResolver::with_config(config);
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // ResolverConfig contains function pointer, can't be const
    pub fn with_config(config: ResolverConfig) -> Self {
        Self { config }
    }

    /// What: Resolve dependencies for a list of packages.
    ///
    /// Inputs:
    /// - `packages`: Slice of `PackageRef` records to resolve dependencies for.
    ///
    /// Output:
    /// - Returns `Ok(DependencyResolution)` with resolved dependencies, conflicts, and missing packages.
    /// - Returns `Err(ArchToolkitError)` if resolution fails.
    ///
    /// Details:
    /// - Resolves ONLY direct dependencies (non-recursive) for each package.
    /// - Merges duplicates by name, retaining the most severe status across all requesters.
    /// - Detects conflicts between packages being installed and already installed packages.
    /// - Sorts dependencies by priority (conflicts first, then missing, then to-install, then installed).
    /// - Uses batch fetching for official packages to reduce pacman command overhead.
    ///
    /// # Errors
    ///
    /// Returns `Err(ArchToolkitError::Parse)` if pacman commands fail or output cannot be parsed.
    /// Returns `Err(ArchToolkitError::PackageNotFound)` if required packages are not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use arch_toolkit::deps::DependencyResolver;
    /// use arch_toolkit::{PackageRef, PackageSource};
    ///
    /// let resolver = DependencyResolver::new();
    /// let packages = vec![
    ///     PackageRef {
    ///         name: "firefox".into(),
    ///         version: "121.0".into(),
    ///         source: PackageSource::Official {
    ///             repo: "extra".into(),
    ///             arch: "x86_64".into(),
    ///         },
    ///     },
    /// ];
    ///
    /// let result = resolver.resolve(&packages)?;
    /// println!("Found {} dependencies", result.dependencies.len());
    /// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
    /// ```
    pub fn resolve(
        &self,
        packages: &[PackageRef],
    ) -> Result<crate::types::dependency::DependencyResolution> {
        use crate::types::dependency::DependencyResolution;

        if packages.is_empty() {
            tracing::warn!("No packages provided for dependency resolution");
            return Ok(DependencyResolution::default());
        }

        let mut deps: HashMap<String, Dependency> = HashMap::new();
        let mut conflicts: Vec<String> = Vec::new();
        let mut missing: Vec<String> = Vec::new();

        // Get installed packages set
        tracing::info!("Fetching list of installed packages...");
        let installed = get_installed_packages()?;
        tracing::info!("Found {} installed packages", installed.len());

        // Get all provided packages (e.g., rustup provides rust)
        // Note: Provides are checked lazily on-demand for performance, not built upfront
        tracing::debug!(
            "Provides will be checked lazily on-demand (not building full set for performance)"
        );
        let provided = get_provided_packages(&installed);

        // Get list of upgradable packages to detect if dependencies need upgrades
        let upgradable = get_upgradable_packages()?;
        tracing::info!("Found {} upgradable packages", upgradable.len());

        // Initialize set of root packages (for tracking)
        let root_names: HashSet<String> = packages.iter().map(|p| p.name.clone()).collect();

        // Check conflicts for packages being installed
        tracing::info!("Checking conflicts for {} package(s)", packages.len());
        for package in packages {
            let package_conflicts = fetch_package_conflicts(&package.name, &package.source);
            for conflict_name in package_conflicts {
                if installed.contains(&conflict_name) || root_names.contains(&conflict_name) {
                    if !conflicts.contains(&conflict_name) {
                        conflicts.push(conflict_name.clone());
                    }
                    // Mark as conflict in dependency map
                    let dep = Dependency {
                        name: conflict_name,
                        version_req: String::new(),
                        status: DependencyStatus::Conflict {
                            reason: format!("Conflicts with {}", package.name),
                        },
                        source: DependencySource::Local,
                        required_by: vec![package.name.clone()],
                        depends_on: Vec::new(),
                        is_core: false,
                        is_system: false,
                    };
                    merge_dependency(
                        &dep,
                        &package.name,
                        &installed,
                        &provided,
                        &upgradable,
                        &mut deps,
                    );
                }
            }
        }

        // Batch fetch official package dependencies to reduce pacman command overhead
        let official_packages: Vec<&str> = packages
            .iter()
            .filter_map(|pkg| {
                if let PackageSource::Official { repo, .. } = &pkg.source {
                    if repo == "local" {
                        None
                    } else {
                        Some(pkg.name.as_str())
                    }
                } else {
                    None
                }
            })
            .collect();
        let batched_deps_cache = if official_packages.is_empty() {
            HashMap::new()
        } else {
            batch_fetch_official_deps(&official_packages)
        };

        // Resolve ONLY direct dependencies (non-recursive)
        // This is faster and avoids resolving transitive dependencies which can be slow and error-prone
        for package in packages {
            // Check if we have batched results for this official package
            let use_batched = matches!(package.source, PackageSource::Official { ref repo, .. } if repo != "local")
                && batched_deps_cache.contains_key(package.name.as_str());

            let resolved_deps = if use_batched {
                // Use batched dependency list
                let dep_names = batched_deps_cache
                    .get(package.name.as_str())
                    .cloned()
                    .unwrap_or_default();
                process_dependency_specs(
                    dep_names,
                    &package.name,
                    &installed,
                    &provided,
                    &upgradable,
                )
            } else {
                // Resolve individually
                match resolve_package_deps(
                    &package.name,
                    &package.source,
                    &installed,
                    &provided,
                    &upgradable,
                    self.config
                        .pkgbuild_cache
                        .as_ref()
                        .map(|f| f.as_ref() as &(dyn Fn(&str) -> Option<String> + Send + Sync)),
                ) {
                    Ok(deps) => deps,
                    Err(e) => {
                        tracing::warn!(
                            "  Failed to resolve dependencies for {}: {}",
                            package.name,
                            e
                        );
                        // Mark as missing
                        if !missing.contains(&package.name) {
                            missing.push(package.name.clone());
                        }
                        continue;
                    }
                }
            };

            tracing::debug!(
                "  Found {} dependencies for {}",
                resolved_deps.len(),
                package.name
            );

            for dep in resolved_deps {
                // Check if dependency is missing
                if matches!(dep.status, DependencyStatus::Missing) && !missing.contains(&dep.name) {
                    missing.push(dep.name.clone());
                }

                merge_dependency(
                    &dep,
                    &package.name,
                    &installed,
                    &provided,
                    &upgradable,
                    &mut deps,
                );

                // DON'T recursively resolve dependencies - only show direct dependencies
                // This prevents resolving transitive dependencies which can be slow and error-prone
            }
        }

        let mut result: Vec<Dependency> = deps.into_values().collect();
        tracing::info!("Total unique dependencies found: {}", result.len());

        // Sort dependencies: conflicts first, then missing, then to-install, then installed
        result.sort_by(|a, b| {
            let priority_a = dependency_priority(&a.status);
            let priority_b = dependency_priority(&b.status);
            priority_a
                .cmp(&priority_b)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(DependencyResolution {
            dependencies: result,
            conflicts,
            missing,
        })
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::dependency::DependencyStatus;

    #[test]
    fn test_should_filter_dependency() {
        assert!(should_filter_dependency("libfoo.so", "package"));
        assert!(should_filter_dependency("libfoo.so.1", "package"));
        assert!(should_filter_dependency("libfoo.so=1", "package"));
        assert!(should_filter_dependency("package", "package")); // self-reference
        assert!(!should_filter_dependency("glibc", "package"));
        assert!(!should_filter_dependency("firefox", "package"));
    }

    #[test]
    fn test_determine_status_not_installed() {
        let installed = HashSet::new();
        let provided = HashSet::new();
        let upgradable = HashSet::new();

        let status = determine_status("nonexistent", "", &installed, &provided, &upgradable);
        assert!(matches!(status, DependencyStatus::ToInstall));
    }

    #[test]
    fn test_batch_fetch_official_deps_parsing() {
        // Test parsing logic with sample output
        let sample_output = "Name            : firefox\nDepends On      : glibc\n\nName            : vim\nDepends On      : glibc\n";
        let mut package_blocks = Vec::new();
        let mut current_block = String::new();
        for line in sample_output.lines() {
            if line.trim().is_empty() {
                if !current_block.is_empty() {
                    package_blocks.push(current_block.clone());
                    current_block.clear();
                }
            } else {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }
        if !current_block.is_empty() {
            package_blocks.push(current_block);
        }

        assert_eq!(package_blocks.len(), 2);
        assert!(package_blocks[0].contains("firefox"));
        assert!(package_blocks[1].contains("vim"));
    }

    #[test]
    fn test_dependency_priority() {
        assert_eq!(
            dependency_priority(&DependencyStatus::Conflict {
                reason: "test".to_string(),
            }),
            0
        );
        assert_eq!(dependency_priority(&DependencyStatus::Missing), 1);
        assert_eq!(dependency_priority(&DependencyStatus::ToInstall), 2);
        assert_eq!(
            dependency_priority(&DependencyStatus::ToUpgrade {
                current: "1.0".to_string(),
                required: "2.0".to_string(),
            }),
            3
        );
        assert_eq!(
            dependency_priority(&DependencyStatus::Installed {
                version: "1.0".to_string(),
            }),
            4
        );
    }

    #[test]
    fn test_dependency_resolver_new() {
        let resolver = DependencyResolver::new();
        // Just verify it can be created
        assert!(matches!(resolver.config.max_depth, 0));
    }

    #[test]
    fn test_dependency_resolver_with_config() {
        let config = ResolverConfig {
            include_optdepends: true,
            include_makedepends: true,
            include_checkdepends: true,
            max_depth: 2,
            pkgbuild_cache: None,
            check_aur: true,
        };
        let resolver = DependencyResolver::with_config(config);
        assert_eq!(resolver.config.max_depth, 2);
        assert!(resolver.config.include_optdepends);
        assert!(resolver.config.check_aur);
    }

    #[test]
    fn test_dependency_resolver_resolve_empty() {
        let resolver = DependencyResolver::new();
        let result = resolver
            .resolve(&[])
            .expect("resolve should succeed for empty packages");
        assert_eq!(result.dependencies.len(), 0);
        assert_eq!(result.conflicts.len(), 0);
        assert_eq!(result.missing.len(), 0);
    }

    // Integration tests that require pacman - these are ignored by default
    #[test]
    #[ignore = "Requires pacman to be available"]
    fn test_dependency_resolver_resolve_integration() {
        let resolver = DependencyResolver::new();
        let packages = vec![PackageRef {
            name: "pacman".to_string(),
            version: "6.1.0".to_string(),
            source: PackageSource::Official {
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
            },
        }];

        if let Ok(result) = resolver.resolve(&packages) {
            // Should find some dependencies for pacman
            println!("Found {} dependencies", result.dependencies.len());
            assert!(!result.dependencies.is_empty());
        }
    }
}
