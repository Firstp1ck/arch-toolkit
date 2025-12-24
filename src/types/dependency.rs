//! Dependency-related data types for dependency resolution operations.

use serde::{Deserialize, Serialize};

// === Enums ===

/// Status of a dependency relative to the current system state.
///
/// This enum represents the installation status and requirements for a dependency,
/// used throughout the dependency resolution process to track what actions are needed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyStatus {
    /// Already installed and version matches requirement.
    Installed {
        /// Installed version of the package.
        version: String,
    },
    /// Not installed, needs to be installed.
    ToInstall,
    /// Installed but outdated, needs upgrade.
    ToUpgrade {
        /// Current installed version.
        current: String,
        /// Required version for upgrade.
        required: String,
    },
    /// Conflicts with existing packages.
    Conflict {
        /// Reason for the conflict.
        reason: String,
    },
    /// Cannot be found in configured repositories or AUR.
    Missing,
}

impl DependencyStatus {
    /// What: Check if the dependency is already installed.
    ///
    /// Inputs:
    /// - `self`: The dependency status to check.
    ///
    /// Output:
    /// - Returns `true` if the dependency is installed (regardless of version).
    ///
    /// Details:
    /// - Returns `true` for both `Installed` and `ToUpgrade` variants.
    #[must_use]
    pub const fn is_installed(&self) -> bool {
        matches!(self, Self::Installed { .. } | Self::ToUpgrade { .. })
    }

    /// What: Check if the dependency needs action (install or upgrade).
    ///
    /// Inputs:
    /// - `self`: The dependency status to check.
    ///
    /// Output:
    /// - Returns `true` if the dependency needs to be installed or upgraded.
    ///
    /// Details:
    /// - Returns `true` for `ToInstall` and `ToUpgrade` variants.
    #[must_use]
    pub const fn needs_action(&self) -> bool {
        matches!(self, Self::ToInstall | Self::ToUpgrade { .. })
    }

    /// What: Check if there's a conflict with this dependency.
    ///
    /// Inputs:
    /// - `self`: The dependency status to check.
    ///
    /// Output:
    /// - Returns `true` if the dependency has a conflict.
    ///
    /// Details:
    /// - Returns `true` only for the `Conflict` variant.
    #[must_use]
    pub const fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict { .. })
    }

    /// What: Get a priority value for sorting (lower = more urgent).
    ///
    /// Inputs:
    /// - `self`: The dependency status to get priority for.
    ///
    /// Output:
    /// - Returns a numeric priority where lower numbers indicate higher urgency.
    ///
    /// Details:
    /// - Priority order: Conflict (0) < Missing (1) < `ToInstall` (2) < `ToUpgrade` (3) < Installed (4).
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Conflict { .. } => 0,
            Self::Missing => 1,
            Self::ToInstall => 2,
            Self::ToUpgrade { .. } => 3,
            Self::Installed { .. } => 4,
        }
    }
}

impl std::fmt::Display for DependencyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Installed { version } => write!(f, "Installed ({version})"),
            Self::ToInstall => write!(f, "To Install"),
            Self::ToUpgrade { current, required } => {
                write!(f, "To Upgrade ({current} -> {required})")
            }
            Self::Conflict { reason } => write!(f, "Conflict: {reason}"),
            Self::Missing => write!(f, "Missing"),
        }
    }
}

/// Source of a dependency package.
///
/// Indicates where a dependency package comes from, which affects how it's resolved
/// and installed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencySource {
    /// Official repository package.
    Official {
        /// Repository name (e.g., "core", "extra", "community").
        repo: String,
    },
    /// AUR package.
    Aur,
    /// Local package (not in repos).
    Local,
}

impl std::fmt::Display for DependencySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Official { repo } => write!(f, "Official ({repo})"),
            Self::Aur => write!(f, "AUR"),
            Self::Local => write!(f, "Local"),
        }
    }
}

/// Package source for dependency resolution input.
///
/// Used when specifying packages to resolve dependencies for, indicating whether
/// the package is from an official repository or AUR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageSource {
    /// Official repository.
    Official {
        /// Repository name (e.g., "core", "extra", "community").
        repo: String,
        /// Target architecture (e.g., `"x86_64"`).
        arch: String,
    },
    /// AUR package.
    Aur,
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Official { repo, arch } => write!(f, "Official ({repo}/{arch})"),
            Self::Aur => write!(f, "AUR"),
        }
    }
}

// === Core Structs ===

/// Information about a single dependency.
///
/// Contains all metadata about a dependency including its status, source, and
/// relationships to other packages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    /// Package name.
    pub name: String,
    /// Required version constraint (e.g., ">=1.2.3" or empty if no constraint).
    pub version_req: String,
    /// Current status of this dependency.
    pub status: DependencyStatus,
    /// Source repository or origin.
    pub source: DependencySource,
    /// Packages that require this dependency.
    pub required_by: Vec<String>,
    /// Packages that this dependency depends on (transitive dependencies).
    pub depends_on: Vec<String>,
    /// Whether this is a core repository package.
    pub is_core: bool,
    /// Whether this is a critical system package.
    pub is_system: bool,
}

/// Package reference for dependency resolution input.
///
/// Used to specify packages for which dependencies should be resolved.
/// This is a simplified representation compared to full package details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRef {
    /// Package name.
    pub name: String,
    /// Package version.
    pub version: String,
    /// Package source (official or AUR).
    pub source: PackageSource,
}

/// Parsed dependency specification (name with optional version requirement).
///
/// Result of parsing a dependency string like "python>=3.12" or "glibc".
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DependencySpec {
    /// Package name.
    pub name: String,
    /// Version constraint (may be empty if no constraint specified).
    pub version_req: String,
}

impl DependencySpec {
    /// What: Create a new dependency spec with just a name.
    ///
    /// Inputs:
    /// - `name`: Package name (will be converted to String).
    ///
    /// Output:
    /// - Returns a new `DependencySpec` with empty version requirement.
    ///
    /// Details:
    /// - Convenience constructor for dependencies without version constraints.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version_req: String::new(),
        }
    }

    /// What: Create a new dependency spec with name and version requirement.
    ///
    /// Inputs:
    /// - `name`: Package name (will be converted to String).
    /// - `version_req`: Version requirement string (e.g., ">=1.2.3").
    ///
    /// Output:
    /// - Returns a new `DependencySpec` with both name and version requirement.
    ///
    /// Details:
    /// - Convenience constructor for dependencies with version constraints.
    #[must_use]
    pub fn with_version(name: impl Into<String>, version_req: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version_req: version_req.into(),
        }
    }

    /// What: Check if this spec has a version requirement.
    ///
    /// Inputs:
    /// - `self`: The dependency spec to check.
    ///
    /// Output:
    /// - Returns `true` if a version requirement is specified.
    ///
    /// Details:
    /// - Checks if `version_req` is non-empty.
    #[must_use]
    pub const fn has_version_req(&self) -> bool {
        !self.version_req.is_empty()
    }
}

impl std::fmt::Display for DependencySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.version_req.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}{}", self.name, self.version_req)
        }
    }
}

/// Reverse dependency analysis result.
///
/// Contains the list of packages that depend on the target packages, along with
/// summary statistics for each target package.
#[derive(Clone, Debug, Default)]
pub struct ReverseDependencyReport {
    /// Packages that depend on the target packages.
    pub dependents: Vec<Dependency>,
    /// Per-package summary statistics.
    pub summaries: Vec<ReverseDependencySummary>,
}

/// Summary statistics for a single package's reverse dependencies.
///
/// Used in reverse dependency analysis to summarize how many packages depend
/// on a given package, broken down by direct and transitive dependents.
#[derive(Clone, Debug, Default)]
pub struct ReverseDependencySummary {
    /// Package name.
    pub package: String,
    /// Number of packages that directly depend on this package (depth 1).
    pub direct_dependents: usize,
    /// Number of packages that depend on this package through other packages (depth ≥ 2).
    pub transitive_dependents: usize,
    /// Total number of dependents (direct + transitive).
    pub total_dependents: usize,
}

/// Parsed .SRCINFO file data.
///
/// Contains all dependency-related fields extracted from a .SRCINFO file,
/// which is the machine-readable format generated from PKGBUILD files.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SrcinfoData {
    /// Package base name (may differ from pkgname for split packages).
    pub pkgbase: String,
    /// Package name (may differ from pkgbase for split packages).
    pub pkgname: String,
    /// Package version.
    pub pkgver: String,
    /// Package release number.
    pub pkgrel: String,
    /// Runtime dependencies.
    pub depends: Vec<String>,
    /// Build-time dependencies.
    pub makedepends: Vec<String>,
    /// Test dependencies.
    pub checkdepends: Vec<String>,
    /// Optional dependencies.
    pub optdepends: Vec<String>,
    /// Conflicting packages.
    pub conflicts: Vec<String>,
    /// Packages this package provides.
    pub provides: Vec<String>,
    /// Packages this package replaces.
    pub replaces: Vec<String>,
}

/// Result of dependency resolution operation.
///
/// Contains all resolved dependencies along with any conflicts or missing packages
/// discovered during the resolution process.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DependencyResolution {
    /// Resolved dependencies with status.
    pub dependencies: Vec<Dependency>,
    /// Packages that have conflicts.
    pub conflicts: Vec<String>,
    /// Packages that are missing.
    pub missing: Vec<String>,
}

/// Configuration for dependency resolution.
///
/// Controls various aspects of how dependencies are resolved, including which
/// types of dependencies to include and how deep to traverse the dependency tree.
///
/// Note: This struct does not implement `Clone` or `Debug` because it contains
/// a function pointer (`pkgbuild_cache`) that cannot be cloned or debugged.
#[allow(clippy::struct_excessive_bools, clippy::type_complexity)]
pub struct ResolverConfig {
    /// Whether to include optional dependencies.
    pub include_optdepends: bool,
    /// Whether to include make dependencies.
    pub include_makedepends: bool,
    /// Whether to include check dependencies.
    pub include_checkdepends: bool,
    /// Maximum depth for transitive dependency resolution (0 = direct only).
    pub max_depth: usize,
    /// Custom callback for fetching PKGBUILD from cache (optional).
    pub pkgbuild_cache: Option<Box<dyn Fn(&str) -> Option<String> + Send + Sync>>,
    /// Whether to check AUR for missing dependencies.
    pub check_aur: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            include_optdepends: false,
            include_makedepends: false,
            include_checkdepends: false,
            max_depth: 0, // Direct dependencies only
            pkgbuild_cache: None,
            check_aur: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_status_priority_ordering() {
        let conflict = DependencyStatus::Conflict {
            reason: "test".to_string(),
        };
        let missing = DependencyStatus::Missing;
        let to_install = DependencyStatus::ToInstall;
        let to_upgrade = DependencyStatus::ToUpgrade {
            current: "1.0".to_string(),
            required: "2.0".to_string(),
        };
        let installed = DependencyStatus::Installed {
            version: "1.0".to_string(),
        };

        assert!(conflict.priority() < missing.priority());
        assert!(missing.priority() < to_install.priority());
        assert!(to_install.priority() < to_upgrade.priority());
        assert!(to_upgrade.priority() < installed.priority());
    }

    #[test]
    fn dependency_status_helper_methods() {
        let installed = DependencyStatus::Installed {
            version: "1.0".to_string(),
        };
        assert!(installed.is_installed());
        assert!(!installed.needs_action());
        assert!(!installed.is_conflict());

        let to_install = DependencyStatus::ToInstall;
        assert!(!to_install.is_installed());
        assert!(to_install.needs_action());
        assert!(!to_install.is_conflict());

        let conflict = DependencyStatus::Conflict {
            reason: "test".to_string(),
        };
        assert!(!conflict.is_installed());
        assert!(!conflict.needs_action());
        assert!(conflict.is_conflict());
    }

    #[test]
    fn dependency_spec_constructors() {
        let spec1 = DependencySpec::new("glibc");
        assert_eq!(spec1.name, "glibc");
        assert!(spec1.version_req.is_empty());
        assert!(!spec1.has_version_req());

        let spec2 = DependencySpec::with_version("python", ">=3.12");
        assert_eq!(spec2.name, "python");
        assert_eq!(spec2.version_req, ">=3.12");
        assert!(spec2.has_version_req());
    }

    #[test]
    fn dependency_spec_display() {
        let spec1 = DependencySpec::new("glibc");
        assert_eq!(spec1.to_string(), "glibc");

        let spec2 = DependencySpec::with_version("python", ">=3.12");
        assert_eq!(spec2.to_string(), "python>=3.12");
    }

    #[test]
    fn dependency_status_display() {
        let installed = DependencyStatus::Installed {
            version: "1.0".to_string(),
        };
        assert!(installed.to_string().contains("Installed"));
        assert!(installed.to_string().contains("1.0"));

        let to_install = DependencyStatus::ToInstall;
        assert_eq!(to_install.to_string(), "To Install");

        let to_upgrade = DependencyStatus::ToUpgrade {
            current: "1.0".to_string(),
            required: "2.0".to_string(),
        };
        assert!(to_upgrade.to_string().contains("To Upgrade"));
        assert!(to_upgrade.to_string().contains("1.0"));
        assert!(to_upgrade.to_string().contains("2.0"));

        let conflict = DependencyStatus::Conflict {
            reason: "test reason".to_string(),
        };
        assert!(conflict.to_string().contains("Conflict"));
        assert!(conflict.to_string().contains("test reason"));

        let missing = DependencyStatus::Missing;
        assert_eq!(missing.to_string(), "Missing");
    }

    #[test]
    fn dependency_source_display() {
        let official = DependencySource::Official {
            repo: "core".to_string(),
        };
        assert!(official.to_string().contains("Official"));
        assert!(official.to_string().contains("core"));

        let aur = DependencySource::Aur;
        assert_eq!(aur.to_string(), "AUR");

        let local = DependencySource::Local;
        assert_eq!(local.to_string(), "Local");
    }

    #[test]
    fn package_source_display() {
        let official = PackageSource::Official {
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
        };
        assert!(official.to_string().contains("Official"));
        assert!(official.to_string().contains("extra"));
        assert!(official.to_string().contains("x86_64"));

        let aur = PackageSource::Aur;
        assert_eq!(aur.to_string(), "AUR");
    }

    #[test]
    fn serde_roundtrip_dependency_status() {
        let statuses = vec![
            DependencyStatus::Installed {
                version: "1.0.0".to_string(),
            },
            DependencyStatus::ToInstall,
            DependencyStatus::ToUpgrade {
                current: "1.0.0".to_string(),
                required: "2.0.0".to_string(),
            },
            DependencyStatus::Conflict {
                reason: "test conflict".to_string(),
            },
            DependencyStatus::Missing,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialization should succeed");
            let deserialized: DependencyStatus =
                serde_json::from_str(&json).expect("deserialization should succeed");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn serde_roundtrip_dependency_source() {
        let sources = vec![
            DependencySource::Official {
                repo: "core".to_string(),
            },
            DependencySource::Aur,
            DependencySource::Local,
        ];

        for source in sources {
            let json = serde_json::to_string(&source).expect("serialization should succeed");
            let deserialized: DependencySource =
                serde_json::from_str(&json).expect("deserialization should succeed");
            assert_eq!(source, deserialized);
        }
    }

    #[test]
    fn serde_roundtrip_dependency() {
        let dep = Dependency {
            name: "glibc".to_string(),
            version_req: ">=2.35".to_string(),
            status: DependencyStatus::Installed {
                version: "2.35".to_string(),
            },
            source: DependencySource::Official {
                repo: "core".to_string(),
            },
            required_by: vec!["firefox".to_string(), "chromium".to_string()],
            depends_on: vec!["linux-api-headers".to_string()],
            is_core: true,
            is_system: true,
        };

        let json = serde_json::to_string(&dep).expect("serialization should succeed");
        let deserialized: Dependency =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(dep.name, deserialized.name);
        assert_eq!(dep.version_req, deserialized.version_req);
        assert_eq!(dep.status, deserialized.status);
        assert_eq!(dep.source, deserialized.source);
        assert_eq!(dep.required_by, deserialized.required_by);
        assert_eq!(dep.depends_on, deserialized.depends_on);
        assert_eq!(dep.is_core, deserialized.is_core);
        assert_eq!(dep.is_system, deserialized.is_system);
    }

    #[test]
    fn serde_roundtrip_srcinfo_data() {
        let srcinfo = SrcinfoData {
            pkgbase: "test-package".to_string(),
            pkgname: "test-package".to_string(),
            pkgver: "1.0.0".to_string(),
            pkgrel: "1".to_string(),
            depends: vec!["glibc".to_string(), "python>=3.12".to_string()],
            makedepends: vec!["make".to_string(), "gcc".to_string()],
            checkdepends: vec!["check".to_string()],
            optdepends: vec!["optional: optional-package".to_string()],
            conflicts: vec!["conflicting-pkg".to_string()],
            provides: vec!["provided-pkg".to_string()],
            replaces: vec!["replaced-pkg".to_string()],
        };

        let json = serde_json::to_string(&srcinfo).expect("serialization should succeed");
        let deserialized: SrcinfoData =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(srcinfo.pkgbase, deserialized.pkgbase);
        assert_eq!(srcinfo.pkgname, deserialized.pkgname);
        assert_eq!(srcinfo.pkgver, deserialized.pkgver);
        assert_eq!(srcinfo.pkgrel, deserialized.pkgrel);
        assert_eq!(srcinfo.depends, deserialized.depends);
        assert_eq!(srcinfo.makedepends, deserialized.makedepends);
        assert_eq!(srcinfo.checkdepends, deserialized.checkdepends);
        assert_eq!(srcinfo.optdepends, deserialized.optdepends);
        assert_eq!(srcinfo.conflicts, deserialized.conflicts);
        assert_eq!(srcinfo.provides, deserialized.provides);
        assert_eq!(srcinfo.replaces, deserialized.replaces);
    }

    #[test]
    fn serde_roundtrip_package_ref() {
        let pkg_ref = PackageRef {
            name: "firefox".to_string(),
            version: "121.0".to_string(),
            source: PackageSource::Official {
                repo: "extra".to_string(),
                arch: "x86_64".to_string(),
            },
        };

        let json = serde_json::to_string(&pkg_ref).expect("serialization should succeed");
        let deserialized: PackageRef =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(pkg_ref, deserialized);
    }
}
