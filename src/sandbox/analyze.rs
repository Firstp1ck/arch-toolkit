//! Dependency delta analysis: compare declared dependencies against the host.

use std::collections::HashSet;
use std::hash::BuildHasher;
use std::process::{Command, Stdio};

use crate::deps::{
    get_installed_version, is_package_installed_or_provided, parse_dep_spec, parse_pkgbuild_deps,
    parse_srcinfo_deps, version_satisfies,
};
use crate::types::sandbox::{DependencyDelta, SandboxInfo};

/// What: Analyze a package's dependencies from PKGBUILD content.
///
/// Inputs:
/// - `package_name`: Package name for the report.
/// - `pkgbuild_text`: PKGBUILD content.
/// - `installed`: Set of installed package names (see `deps::get_installed_packages`).
/// - `provided`: Set of names provided by installed packages (see `deps::get_provided_packages`).
///
/// Output:
/// - `SandboxInfo` with per-category dependency deltas.
///
/// Details:
/// - Parses dependency arrays via `deps::parse_pkgbuild_deps` (Phase 2 parser)
///   and delegates to [`analyze_dependencies`] per category.
///
/// # Example
///
/// ```
/// use arch_toolkit::sandbox::analyze_pkgbuild;
/// use std::collections::HashSet;
///
/// let pkgbuild = "depends=('glibc')\nmakedepends=('rust>=1.70')";
/// let installed: HashSet<String> = HashSet::from(["glibc".to_string()]);
/// let provided = HashSet::new();
/// let info = analyze_pkgbuild("demo", pkgbuild, &installed, &provided);
/// assert_eq!(info.missing_packages(), ["rust>=1.70"]);
/// ```
#[must_use]
pub fn analyze_pkgbuild<S: BuildHasher>(
    package_name: &str,
    pkgbuild_text: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
) -> SandboxInfo {
    let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild_text);
    build_info(
        package_name,
        &depends,
        &makedepends,
        &checkdepends,
        &optdepends,
        installed,
        provided,
    )
}

/// What: Analyze a package's dependencies from .SRCINFO content.
///
/// Inputs:
/// - `package_name`: Package name for the report.
/// - `srcinfo_text`: .SRCINFO content.
/// - `installed`: Set of installed package names.
/// - `provided`: Set of names provided by installed packages.
///
/// Output:
/// - `SandboxInfo` with per-category dependency deltas.
///
/// Details:
/// - Parses dependency fields via `deps::parse_srcinfo_deps` (Phase 2 parser)
///   and delegates to [`analyze_dependencies`] per category.
/// - .SRCINFO is preferred over PKGBUILD when both are available (it is
///   machine-generated and unambiguous); fetch it via `deps::fetch_srcinfo`
///   when the `aur` feature is enabled.
///
/// # Example
///
/// ```
/// use arch_toolkit::sandbox::analyze_srcinfo;
/// use std::collections::HashSet;
///
/// let srcinfo = "pkgbase = demo\n\tdepends = glibc\n\tmakedepends = cmake";
/// let installed: HashSet<String> = HashSet::from(["glibc".to_string(), "cmake".to_string()]);
/// let provided = HashSet::new();
/// let info = analyze_srcinfo("demo", srcinfo, &installed, &provided);
/// assert!(info.is_ready_to_build());
/// ```
#[must_use]
pub fn analyze_srcinfo<S: BuildHasher>(
    package_name: &str,
    srcinfo_text: &str,
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
) -> SandboxInfo {
    let (depends, makedepends, checkdepends, optdepends) = parse_srcinfo_deps(srcinfo_text);
    build_info(
        package_name,
        &depends,
        &makedepends,
        &checkdepends,
        &optdepends,
        installed,
        provided,
    )
}

/// What: Assemble a `SandboxInfo` from parsed dependency arrays.
///
/// Inputs:
/// - `package_name`: Package name for the report.
/// - Parsed dependency arrays per category.
/// - `installed` / `provided`: Host package sets.
///
/// Output:
/// - `SandboxInfo` with all categories analyzed.
///
/// Details:
/// - Shared tail of the PKGBUILD and .SRCINFO entry points.
fn build_info<S: BuildHasher>(
    package_name: &str,
    depends: &[String],
    makedepends: &[String],
    checkdepends: &[String],
    optdepends: &[String],
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
) -> SandboxInfo {
    SandboxInfo {
        package_name: package_name.to_string(),
        depends: analyze_dependencies(depends, installed, provided),
        makedepends: analyze_dependencies(makedepends, installed, provided),
        checkdepends: analyze_dependencies(checkdepends, installed, provided),
        optdepends: analyze_dependencies(optdepends, installed, provided),
    }
}

/// What: Analyze dependency specs against the host environment.
///
/// Inputs:
/// - `deps`: Dependency specs as declared (may include version requirements
///   or optdepends `pkg: description` annotations).
/// - `installed`: Set of installed package names.
/// - `provided`: Set of names provided by installed packages.
///
/// Output:
/// - One `DependencyDelta` per spec (local packages are skipped).
///
/// Details:
/// - Membership is checked via `deps::is_package_installed_or_provided`.
/// - Version constraints are parsed with `deps::parse_dep_spec` and checked
///   with `deps::version_satisfies` against the `pacman -Q` version —
///   an improvement over Pacsea, which passed the full spec as the
///   requirement (never failing the check).
/// - Installed local packages are filtered out, matching Pacsea (they are
///   not relevant for build-preflight analysis).
/// - Version lookup shells out to `pacman -Q` per installed dependency and
///   degrades gracefully when pacman is unavailable.
#[must_use]
pub fn analyze_dependencies<S: BuildHasher>(
    deps: &[String],
    installed: &HashSet<String, S>,
    provided: &HashSet<String, S>,
) -> Vec<DependencyDelta> {
    deps.iter()
        .filter_map(|dep_spec| {
            let pkg_name = extract_package_name(dep_spec);
            let is_installed = is_package_installed_or_provided(&pkg_name, installed, provided);

            // Skip local packages — not relevant for sandbox analysis
            if is_installed && is_local_package(&pkg_name) {
                return None;
            }

            let installed_version = if is_installed {
                get_installed_version(&pkg_name).ok()
            } else {
                None
            };

            // Check the declared constraint against the installed version.
            // Strip any optdepends description before parsing the spec.
            let spec_only = dep_spec
                .split_once(": ")
                .map_or(dep_spec.as_str(), |(spec, _desc)| spec);
            let version_req = parse_dep_spec(spec_only).version_req;
            let version_satisfied = installed_version
                .as_ref()
                .is_some_and(|version| version_satisfies(version, &version_req));

            Some(DependencyDelta {
                name: dep_spec.clone(),
                is_installed,
                installed_version,
                version_satisfied,
            })
        })
        .collect()
}

/// What: Extract the bare package name from a dependency specification.
///
/// Inputs:
/// - `dep_spec`: Spec like `foo>=1.2`, `bar`, or `baz: enables feature X`.
///
/// Output:
/// - Package name without version requirements or optdepends description.
///
/// Details:
/// - Handles the optdepends `package: description` form first, then strips
///   version operators via `deps::parse_dep_spec`.
///
/// # Example
///
/// ```
/// use arch_toolkit::sandbox::extract_package_name;
///
/// assert_eq!(extract_package_name("python>=3.12"), "python");
/// assert_eq!(extract_package_name("cups: printing support"), "cups");
/// assert_eq!(extract_package_name("glibc"), "glibc");
/// ```
#[must_use]
pub fn extract_package_name(dep_spec: &str) -> String {
    let spec_only = dep_spec
        .split_once(':')
        .map_or(dep_spec, |(before, _)| before);
    parse_dep_spec(spec_only.trim()).name
}

/// What: Check whether an installed package comes from the `local` repository.
///
/// Inputs:
/// - `name`: Installed package name.
///
/// Output:
/// - `true` when `pacman -Qi` reports the Repository field as `local` or empty.
///
/// Details:
/// - Returns `false` when pacman is unavailable or the field cannot be read
///   (graceful degradation, matching Pacsea).
fn is_local_package(name: &str) -> bool {
    let output = Command::new("pacman")
        .args(["-Qi", name])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.starts_with("Repository")
                && let Some(colon_pos) = line.find(':')
            {
                let repo = line[colon_pos + 1..].trim().to_lowercase();
                return repo == "local" || repo.is_empty();
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sets(installed: &[&str]) -> (HashSet<String>, HashSet<String>) {
        (
            installed.iter().map(ToString::to_string).collect(),
            HashSet::new(),
        )
    }

    #[test]
    /// What: Verify deltas report installed and missing dependencies.
    ///
    /// Inputs:
    /// - Specs with one installed and one missing package.
    ///
    /// Output:
    /// - Correct `is_installed` flags; missing entries have no version.
    ///
    /// Details:
    /// - Membership comes from the caller-provided installed set.
    fn membership() {
        let (installed, provided) = sets(&["glibc"]);
        let deps = vec![
            "glibc".to_string(),
            "definitely-not-installed-xyz".to_string(),
        ];
        let deltas = analyze_dependencies(&deps, &installed, &provided);
        assert_eq!(deltas.len(), 2);
        assert!(deltas[0].is_installed);
        assert!(!deltas[1].is_installed);
        assert!(deltas[1].installed_version.is_none());
        assert!(!deltas[1].version_satisfied);
    }

    #[test]
    /// What: Verify provided packages count as installed.
    ///
    /// Inputs:
    /// - Package present only in the provided set.
    ///
    /// Output:
    /// - Delta marked installed.
    ///
    /// Details:
    /// - Virtual packages (e.g., `sh` provided by `bash`) must not appear missing.
    fn provided_counts_as_installed() {
        let installed: HashSet<String> = HashSet::new();
        let provided: HashSet<String> = HashSet::from(["sh".to_string()]);
        let deltas = analyze_dependencies(&["sh".to_string()], &installed, &provided);
        assert!(deltas[0].is_installed);
    }

    #[test]
    /// What: Verify name extraction across spec forms.
    ///
    /// Inputs:
    /// - Version-constrained, plain, and optdepends-annotated specs.
    ///
    /// Output:
    /// - Bare package names.
    ///
    /// Details:
    /// - Mirrors Pacsea's `extract_package_name` behavior.
    fn name_extraction() {
        assert_eq!(extract_package_name("python>=3.12"), "python");
        assert_eq!(extract_package_name("qt6-base<7"), "qt6-base");
        assert_eq!(extract_package_name("libfoo=1.0"), "libfoo");
        assert_eq!(extract_package_name("cups: printing support"), "cups");
        assert_eq!(extract_package_name("  glibc  "), "glibc");
    }

    #[test]
    /// What: Verify full analysis from PKGBUILD text.
    ///
    /// Inputs:
    /// - PKGBUILD with depends/makedepends/optdepends and a partial installed set.
    ///
    /// Output:
    /// - Correct categories, missing list, and readiness flag.
    ///
    /// Details:
    /// - Exercises the Phase 2 parser integration end to end.
    fn pkgbuild_analysis() {
        let pkgbuild = r"
depends=('glibc' 'missing-dep-xyz')
makedepends=('cmake')
optdepends=('cups: printing support')
";
        let (installed, provided) = sets(&["glibc", "cmake"]);
        let info = analyze_pkgbuild("demo", pkgbuild, &installed, &provided);
        assert_eq!(info.package_name, "demo");
        assert_eq!(info.depends.len(), 2);
        assert_eq!(info.makedepends.len(), 1);
        assert_eq!(info.optdepends.len(), 1);
        assert_eq!(info.missing_packages(), ["missing-dep-xyz"]);
        assert!(!info.is_ready_to_build());
    }

    #[test]
    /// What: Verify full analysis from .SRCINFO text.
    ///
    /// Inputs:
    /// - .SRCINFO with all build deps present in the installed set.
    ///
    /// Output:
    /// - Ready-to-build report with no missing packages.
    ///
    /// Details:
    /// - Missing optdepends must not affect readiness.
    fn srcinfo_analysis() {
        let srcinfo = "pkgbase = demo\n\tdepends = glibc\n\tmakedepends = rust\n\toptdepends = cups: printing";
        let (installed, provided) = sets(&["glibc", "rust"]);
        let info = analyze_srcinfo("demo", srcinfo, &installed, &provided);
        assert!(info.is_ready_to_build());
        assert!(info.missing_packages().is_empty());
        assert_eq!(info.optdepends.len(), 1);
    }

    #[test]
    /// What: Verify empty input produces an empty, ready report.
    ///
    /// Inputs:
    /// - Empty PKGBUILD text.
    ///
    /// Output:
    /// - No deltas in any category; ready to build.
    ///
    /// Details:
    /// - Degenerate inputs must not panic.
    fn empty_input() {
        let (installed, provided) = sets(&[]);
        let info = analyze_pkgbuild("empty", "", &installed, &provided);
        assert!(info.depends.is_empty());
        assert!(info.is_ready_to_build());
    }
}
