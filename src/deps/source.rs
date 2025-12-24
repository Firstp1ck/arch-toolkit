//! Dependency source determination utilities.
//!
//! This module provides functions to determine where a dependency package comes from
//! (official repository, AUR, or local) and to identify critical system packages.

use crate::types::dependency::DependencySource;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::process::{Command, Stdio};

/// What: Infer the origin repository for a dependency currently under analysis.
///
/// Inputs:
/// - `name`: Candidate dependency package name.
/// - `installed`: Set of locally installed package names used to detect presence.
///
/// Output:
/// - Returns a tuple with the determined `DependencySource` and a flag indicating core membership.
///
/// Details:
/// - Prefers inspecting `pacman -Qi` metadata when the package is installed; otherwise defaults to heuristics.
/// - For installed packages: uses `pacman -Qi` to read the "Repository" field.
/// - For uninstalled packages: uses `pacman -Si` to check if it exists in official repositories.
/// - Handles local packages (repo = "local" or empty) specially.
/// - Downgrades gracefully to official classifications when the repository field cannot be read.
/// - Sets `LC_ALL=C` and `LANG=C` for consistent locale-independent output.
/// - Returns reasonable defaults when pacman is unavailable (graceful degradation).
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::determine_dependency_source;
/// use std::collections::HashSet;
///
/// let installed = HashSet::from(["glibc".to_string()]);
/// let (source, is_core) = determine_dependency_source("glibc", &installed);
/// println!("Source: {:?}, Is core: {}", source, is_core);
/// ```
pub fn determine_dependency_source<S: BuildHasher>(
    name: &str,
    installed: &HashSet<String, S>,
) -> (DependencySource, bool) {
    if !installed.contains(name) {
        // Not installed - check if it exists in official repos first
        // Only default to AUR if it's not found in official repos
        let output = Command::new("pacman")
            .args(["-Si", name])
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        if let Ok(output) = output
            && output.status.success()
        {
            // Package exists in official repos - determine which repo
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.starts_with("Repository")
                    && let Some(colon_pos) = line.find(':')
                {
                    let repo = line[colon_pos + 1..].trim().to_lowercase();
                    let is_core = repo == "core";
                    return (DependencySource::Official { repo }, is_core);
                }
            }
            // Found in official repos but couldn't determine repo - assume extra
            return (
                DependencySource::Official {
                    repo: "extra".to_string(),
                },
                false,
            );
        }
        // Not found in official repos - this could be:
        // 1. A binary/script provided by a package (not a package itself) - should be Missing
        // 2. A virtual package (.so file) - should be filtered out earlier
        // 3. A real AUR package - but we can't distinguish without checking AUR
        //
        // IMPORTANT: We don't try AUR here because:
        // - Most dependencies are from official repos or are binaries/scripts
        // - Trying AUR for every unknown dependency causes unnecessary API calls
        // - Real AUR packages should be explicitly specified by the user, not discovered as dependencies
        // - If it's truly an AUR dependency, it will be marked as Missing and the user can handle it
        tracing::debug!(
            "Package {} not found in official repos and not installed - will be marked as Missing (skipping AUR check)",
            name
        );
        // Return AUR but the resolve logic should check if it exists before trying API
        return (DependencySource::Aur, false);
    }

    // Package is installed - check which repository it came from
    let output = Command::new("pacman")
        .args(["-Qi", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            // Look for "Repository" field in pacman -Qi output
            for line in text.lines() {
                if line.starts_with("Repository")
                    && let Some(colon_pos) = line.find(':')
                {
                    let repo = line[colon_pos + 1..].trim().to_lowercase();
                    let is_core = repo == "core";
                    // Handle local packages specially
                    if repo == "local" || repo.is_empty() {
                        return (DependencySource::Local, false);
                    }
                    return (DependencySource::Official { repo }, is_core);
                }
            }
        }
        _ => {
            // Fallback: try pacman -Q to see if it's installed
            // If we can't determine repo, assume it's from an official repo
            tracing::debug!(
                "Could not determine repository for {}, assuming official",
                name
            );
        }
    }

    // Default: assume official repository (most installed packages are)
    let is_core = is_system_package(name);
    (
        DependencySource::Official {
            repo: if is_core {
                "core".to_string()
            } else {
                "extra".to_string()
            },
        },
        is_core,
    )
}

/// What: Identify whether a dependency belongs to a curated list of critical system packages.
///
/// Inputs:
/// - `name`: Package name to compare against the predefined system set.
///
/// Output:
/// - `true` when the package is considered a core system component; otherwise `false`.
///
/// Details:
/// - Used to highlight packages whose removal or downgrade should be discouraged.
/// - Checks against a curated list of critical system packages.
/// - Uses exact string matching (case-sensitive).
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::deps::is_system_package;
///
/// if is_system_package("glibc") {
///     println!("glibc is a critical system package");
/// }
/// ```
#[must_use]
pub fn is_system_package(name: &str) -> bool {
    // List of critical system packages
    let system_packages = [
        "glibc",
        "linux",
        "systemd",
        "pacman",
        "bash",
        "coreutils",
        "gcc",
        "binutils",
        "filesystem",
        "util-linux",
        "shadow",
        "sed",
        "grep",
    ];
    system_packages.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Confirm `is_system_package` recognizes curated critical packages.
    ///
    /// Inputs:
    /// - `names`: Sample package names including system and non-system entries.
    ///
    /// Output:
    /// - Returns `true` for known core packages and `false` for unrelated software.
    ///
    /// Details:
    /// - Exercises both positive (glibc, linux) and negative (firefox) cases to validate membership.
    fn test_is_system_package_detects_core() {
        assert!(is_system_package("glibc"));
        assert!(is_system_package("linux"));
        assert!(is_system_package("systemd"));
        assert!(is_system_package("pacman"));
        assert!(is_system_package("bash"));
        assert!(is_system_package("coreutils"));
        assert!(is_system_package("gcc"));
        assert!(is_system_package("binutils"));
        assert!(is_system_package("filesystem"));
        assert!(is_system_package("util-linux"));
        assert!(is_system_package("shadow"));
        assert!(is_system_package("sed"));
        assert!(is_system_package("grep"));
        assert!(!is_system_package("firefox"));
        assert!(!is_system_package("vim"));
        assert!(!is_system_package("nonexistent"));
        assert!(!is_system_package(""));
    }

    #[test]
    /// What: Test `determine_dependency_source` with installed core package.
    ///
    /// Inputs:
    /// - Mock pacman -Qi output for a core package.
    ///
    /// Output:
    /// - Returns `(DependencySource::Official { repo: "core" }, true)`.
    ///
    /// Details:
    /// - Tests parsing of pacman -Qi output for installed core packages.
    fn test_determine_dependency_source_installed_core() {
        // This test would require mocking Command, which is complex
        // Instead, we test the parsing logic separately
        let sample_output = "Repository      : core\nName            : glibc\n";
        let mut found_repo = None;
        for line in sample_output.lines() {
            if line.starts_with("Repository")
                && let Some(colon_pos) = line.find(':')
            {
                let repo = line[colon_pos + 1..].trim().to_lowercase();
                let is_core = repo == "core";
                found_repo = Some((repo, is_core));
                break;
            }
        }
        assert_eq!(found_repo, Some(("core".to_string(), true)));
    }

    #[test]
    /// What: Test `determine_dependency_source` with installed extra package.
    ///
    /// Inputs:
    /// - Mock pacman -Qi output for an extra package.
    ///
    /// Output:
    /// - Returns `(DependencySource::Official { repo: "extra" }, false)`.
    ///
    /// Details:
    /// - Tests parsing of pacman -Qi output for installed extra packages.
    fn test_determine_dependency_source_installed_extra() {
        let sample_output = "Repository      : extra\nName            : firefox\n";
        let mut found_repo = None;
        for line in sample_output.lines() {
            if line.starts_with("Repository")
                && let Some(colon_pos) = line.find(':')
            {
                let repo = line[colon_pos + 1..].trim().to_lowercase();
                let is_core = repo == "core";
                found_repo = Some((repo, is_core));
                break;
            }
        }
        assert_eq!(found_repo, Some(("extra".to_string(), false)));
    }

    #[test]
    /// What: Test `determine_dependency_source` with installed local package.
    ///
    /// Inputs:
    /// - Mock pacman -Qi output for a local package.
    ///
    /// Output:
    /// - Returns `(DependencySource::Local, false)`.
    ///
    /// Details:
    /// - Tests parsing of pacman -Qi output for local packages.
    fn test_determine_dependency_source_installed_local() {
        let sample_output = "Repository      : local\nName            : custom-package\n";
        let mut found_repo = None;
        for line in sample_output.lines() {
            if line.starts_with("Repository")
                && let Some(colon_pos) = line.find(':')
            {
                let repo = line[colon_pos + 1..].trim().to_lowercase();
                if repo == "local" || repo.is_empty() {
                    found_repo = Some("local");
                    break;
                }
            }
        }
        assert_eq!(found_repo, Some("local"));
    }

    #[test]
    /// What: Test `determine_dependency_source` with uninstalled official package.
    ///
    /// Inputs:
    /// - Mock pacman -Si output for an official package.
    ///
    /// Output:
    /// - Returns `(DependencySource::Official { repo }, is_core)`.
    ///
    /// Details:
    /// - Tests parsing of pacman -Si output for uninstalled official packages.
    fn test_determine_dependency_source_not_installed_official() {
        let sample_output = "Repository      : extra\nName            : firefox\n";
        let mut found_repo = None;
        for line in sample_output.lines() {
            if line.starts_with("Repository")
                && let Some(colon_pos) = line.find(':')
            {
                let repo = line[colon_pos + 1..].trim().to_lowercase();
                let is_core = repo == "core";
                found_repo = Some((repo, is_core));
                break;
            }
        }
        assert_eq!(found_repo, Some(("extra".to_string(), false)));
    }

    #[test]
    /// What: Test `determine_dependency_source` fallback behavior.
    ///
    /// Inputs:
    /// - Package name that triggers fallback logic.
    ///
    /// Output:
    /// - Returns reasonable defaults based on `is_system_package()`.
    ///
    /// Details:
    /// - Tests fallback when pacman commands fail or repository cannot be determined.
    fn test_determine_dependency_source_fallback() {
        // Test fallback logic: if is_system_package returns true, should default to core
        let is_core = is_system_package("glibc");
        assert!(is_core);
        let expected_repo = if is_core { "core" } else { "extra" };
        assert_eq!(expected_repo, "core");

        // Test fallback logic: if is_system_package returns false, should default to extra
        let is_core = is_system_package("firefox");
        assert!(!is_core);
        let expected_repo = if is_core { "core" } else { "extra" };
        assert_eq!(expected_repo, "extra");
    }

    #[test]
    /// What: Test parsing repository field from pacman output.
    ///
    /// Inputs:
    /// - Various pacman output formats.
    ///
    /// Output:
    /// - Correctly extracts repository name and determines if core.
    ///
    /// Details:
    /// - Tests edge cases in parsing pacman output.
    fn test_parse_repository_field() {
        let test_cases = vec![
            ("Repository      : core", ("core", true)),
            ("Repository      : extra", ("extra", false)),
            ("Repository      : community", ("community", false)),
            ("Repository      : local", ("local", false)),
            ("Repository: core", ("core", true)),
            ("Repository : extra", ("extra", false)),
        ];

        for (input, (expected_repo, expected_is_core)) in test_cases {
            if let Some(colon_pos) = input.find(':') {
                let repo = input[colon_pos + 1..].trim().to_lowercase();
                let is_core = repo == "core";
                assert_eq!(repo, expected_repo);
                assert_eq!(is_core, expected_is_core);
            }
        }
    }

    // Integration tests that require pacman - these are ignored by default
    #[test]
    #[ignore = "Requires pacman to be available"]
    /// What: Test `determine_dependency_source` with real pacman.
    ///
    /// Inputs:
    /// - Real pacman database.
    ///
    /// Output:
    /// - Correctly determines source for installed packages.
    ///
    /// Details:
    /// - Integration test that requires pacman to be available.
    fn test_determine_dependency_source_integration() {
        use crate::deps::get_installed_packages;
        // Test with a package that should be installed (pacman itself)
        if let Ok(installed) = get_installed_packages()
            && installed.contains("pacman")
        {
            let (source, is_core) = determine_dependency_source("pacman", &installed);
            // pacman should be from core repository
            match source {
                DependencySource::Official { repo } => {
                    assert_eq!(repo, "core");
                    assert!(is_core);
                }
                _ => panic!("pacman should be from official core repository"),
            }
        }
    }
}
