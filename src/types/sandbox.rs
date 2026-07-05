//! Sandbox-related data types for build-preflight dependency analysis.

use serde::{Deserialize, Serialize};

/// What: Status of one declared dependency relative to the host environment.
///
/// Inputs:
/// - Produced by `sandbox::analyze_dependencies()` and the analysis entry points.
///
/// Output:
/// - Installation and version-constraint status for a single dependency spec.
///
/// Details:
/// - `name` keeps the full spec as declared (e.g., `foo>=1.2` or
///   `bar: enables feature X` for optdepends) so callers can display it verbatim.
/// - `version_satisfied` is `false` when the package is not installed; when
///   installed without a declared constraint it is `true`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyDelta {
    /// Dependency spec as declared (may include version requirement or description).
    pub name: String,
    /// Whether this dependency is installed (or provided) on the host.
    pub is_installed: bool,
    /// Installed version when available (from `pacman -Q`).
    pub installed_version: Option<String>,
    /// Whether the installed version satisfies the declared constraint.
    pub version_satisfied: bool,
}

/// What: Build-preflight analysis result for a package.
///
/// Inputs:
/// - Produced by `sandbox::analyze_pkgbuild()` / `sandbox::analyze_srcinfo()`.
///
/// Output:
/// - Per-category dependency deltas comparing the package's declared
///   dependencies against the host.
///
/// Details:
/// - Ported from Pacsea's `SandboxInfo`; answers "what would I need to install
///   to build this AUR package?" before any build starts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxInfo {
    /// Package name the analysis belongs to.
    pub package_name: String,
    /// Runtime dependencies (`depends`).
    pub depends: Vec<DependencyDelta>,
    /// Build-time dependencies (`makedepends`).
    pub makedepends: Vec<DependencyDelta>,
    /// Test dependencies (`checkdepends`).
    pub checkdepends: Vec<DependencyDelta>,
    /// Optional dependencies (`optdepends`).
    pub optdepends: Vec<DependencyDelta>,
}

impl SandboxInfo {
    /// What: List dependency specs that are not installed on the host.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - Specs from `depends`, `makedepends`, and `checkdepends` that are missing.
    ///
    /// Details:
    /// - Optional dependencies are excluded; they do not block a build.
    #[must_use]
    pub fn missing_packages(&self) -> Vec<&str> {
        self.depends
            .iter()
            .chain(&self.makedepends)
            .chain(&self.checkdepends)
            .filter(|delta| !delta.is_installed)
            .map(|delta| delta.name.as_str())
            .collect()
    }

    /// What: Check whether all build-relevant dependencies are installed.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - `true` when every entry in `depends`, `makedepends`, and
    ///   `checkdepends` is installed on the host.
    ///
    /// Details:
    /// - Optional dependencies are excluded; version constraints are reported
    ///   per-delta but do not affect this readiness check (pacman would
    ///   upgrade them during install).
    #[must_use]
    pub fn is_ready_to_build(&self) -> bool {
        self.depends
            .iter()
            .chain(&self.makedepends)
            .chain(&self.checkdepends)
            .all(|delta| delta.is_installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(name: &str, installed: bool) -> DependencyDelta {
        DependencyDelta {
            name: name.to_string(),
            is_installed: installed,
            installed_version: installed.then(|| "1.0".to_string()),
            version_satisfied: installed,
        }
    }

    #[test]
    /// What: Verify `missing_packages` collects non-installed build deps only.
    ///
    /// Inputs:
    /// - Info with missing entries in each category including optdepends.
    ///
    /// Output:
    /// - Missing depends/makedepends/checkdepends; optdepends excluded.
    ///
    /// Details:
    /// - Optional dependencies never block a build.
    fn missing_packages_excludes_optdepends() {
        let info = SandboxInfo {
            package_name: "pkg".to_string(),
            depends: vec![delta("a", true), delta("b", false)],
            makedepends: vec![delta("c", false)],
            checkdepends: vec![delta("d", true)],
            optdepends: vec![delta("e", false)],
        };
        assert_eq!(info.missing_packages(), ["b", "c"]);
    }

    #[test]
    /// What: Verify `is_ready_to_build` requires all build deps installed.
    ///
    /// Inputs:
    /// - Info variants with and without missing build dependencies.
    ///
    /// Output:
    /// - `true` only when depends/makedepends/checkdepends are all installed.
    ///
    /// Details:
    /// - Missing optdepends must not affect readiness.
    fn readiness() {
        let ready = SandboxInfo {
            package_name: "pkg".to_string(),
            depends: vec![delta("a", true)],
            makedepends: vec![delta("b", true)],
            checkdepends: vec![],
            optdepends: vec![delta("c", false)],
        };
        assert!(ready.is_ready_to_build());

        let not_ready = SandboxInfo {
            makedepends: vec![delta("b", false)],
            ..ready
        };
        assert!(!not_ready.is_ready_to_build());
    }

    #[test]
    /// What: Verify serde roundtrip and Default for `SandboxInfo`.
    ///
    /// Inputs:
    /// - Populated info serialized to JSON and back; `SandboxInfo::default()`.
    ///
    /// Output:
    /// - Roundtrip equality; default is empty and ready to build.
    ///
    /// Details:
    /// - Supports caller-side caching of analysis results.
    fn serde_and_default() {
        let info = SandboxInfo {
            package_name: "pkg".to_string(),
            depends: vec![delta("glibc>=2.38", true)],
            ..Default::default()
        };
        let back: SandboxInfo =
            serde_json::from_str(&serde_json::to_string(&info).expect("serialize"))
                .expect("deserialize");
        assert_eq!(back, info);

        let empty = SandboxInfo::default();
        assert!(empty.missing_packages().is_empty());
        assert!(empty.is_ready_to_build());
    }
}
