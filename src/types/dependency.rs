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

impl PackageRef {
    /// What: Create a reference to an official repository package.
    ///
    /// Inputs:
    /// - `name`: Package name.
    /// - `version`: Package version.
    /// - `repo`: Repository name (e.g., "core", "extra").
    /// - `arch`: Target architecture (e.g., `x86_64`, `any`).
    ///
    /// Output:
    /// - `PackageRef` with `PackageSource::Official`.
    ///
    /// Details:
    /// - Convenience constructor for resolution and install-planning inputs.
    #[must_use]
    pub fn official(
        name: impl Into<String>,
        version: impl Into<String>,
        repo: impl Into<String>,
        arch: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            source: PackageSource::Official {
                repo: repo.into(),
                arch: arch.into(),
            },
        }
    }

    /// What: Create a reference to an AUR package.
    ///
    /// Inputs:
    /// - `name`: Package name.
    /// - `version`: Package version.
    ///
    /// Output:
    /// - `PackageRef` with `PackageSource::Aur`.
    ///
    /// Details:
    /// - Convenience constructor for resolution and install-planning inputs.
    #[must_use]
    pub fn aur(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            source: PackageSource::Aur,
        }
    }
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

/// What: Carry raw `.SRCINFO` metadata returned by an injected graph metadata provider.
///
/// Inputs:
/// - Requested name, selected actual package, verified source, and raw `.SRCINFO` text.
///
/// Output:
/// - Supplies enough information to resolve direct and virtual dependencies without a crate-level
///   AUR or HTTP dependency.
///
/// Details:
/// - `package_name` must name the selected split-package output. If it differs from
///   `requested_name`, the resolver verifies that the selected output provides the requested name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyMetadata {
    /// Package or virtual name queried from the provider.
    pub requested_name: String,
    /// Actual selected package name, including a split-package output when applicable.
    pub package_name: String,
    /// Verified source supplied by the metadata provider.
    pub source: DependencySource,
    /// Raw `.SRCINFO` text for the containing package base.
    pub srcinfo: String,
}

impl DependencyMetadata {
    /// What: Construct injected raw metadata for one requested package or provider.
    ///
    /// Inputs:
    /// - `requested_name`: Queried package or virtual name.
    /// - `package_name`: Selected actual package output.
    /// - `source`: Verified source of the selected package.
    /// - `srcinfo`: Raw `.SRCINFO` package-base metadata.
    ///
    /// Output:
    /// - Returns a metadata record suitable for `DependencyMetadataProvider`.
    ///
    /// Details:
    /// - This constructor performs no I/O or parsing so deterministic fixtures can use it directly.
    #[must_use]
    pub fn new(
        requested_name: impl Into<String>,
        package_name: impl Into<String>,
        source: DependencySource,
        srcinfo: impl Into<String>,
    ) -> Self {
        Self {
            requested_name: requested_name.into(),
            package_name: package_name.into(),
            source,
            srcinfo: srcinfo.into(),
        }
    }
}

/// What: Describe a batched injected metadata-provider result.
///
/// Inputs:
/// - A requested name and either returned metadata, an absence reason, or a retrieval failure.
///
/// Output:
/// - Lets graph resolution retain partial results and report actionable diagnostics.
///
/// Details:
/// - Provider failures are non-fatal for sibling branches and never cause fallback AUR inference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencyMetadataResponse {
    /// Verified metadata was returned for a requested package or virtual dependency.
    Found(DependencyMetadata),
    /// No verified metadata exists for the requested name.
    Missing {
        /// Requested package or virtual name.
        requested_name: String,
        /// Actionable absence reason.
        reason: String,
    },
    /// Metadata retrieval failed through a network, helper, or provider-specific error.
    Failure {
        /// Requested package or virtual name.
        requested_name: String,
        /// Actionable provider error message.
        message: String,
    },
}

/// What: Identify the source and requested/provider identity behind a graph node.
///
/// Inputs:
/// - The requested dependency name, optional verified source, and optional selected provider.
///
/// Output:
/// - Lets callers distinguish direct packages, virtual providers, and unresolved names.
///
/// Details:
/// - `source` is `None` only when metadata is absent or failed. The resolver never infers AUR
///   provenance merely because an unknown name was not found elsewhere.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyProvenance {
    /// Original dependency name requested by the parent package.
    pub requested_name: String,
    /// Verified source reported by the metadata provider, if metadata was available.
    pub source: Option<DependencySource>,
    /// Actual package selected to satisfy a virtual request, if different from the request.
    pub provider: Option<String>,
}

/// What: Represent one inclusive or exclusive edge of an intersected version range.
///
/// Inputs:
/// - A version string and whether equality is permitted at that edge.
///
/// Output:
/// - Supplies a lower or upper bound for `DependencyConstraintRange`.
///
/// Details:
/// - Version ordering uses the dependency resolver's epoch/pkgver/pkgrel comparator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyVersionBound {
    /// Bound version including epoch and pkgrel when present.
    pub version: String,
    /// Whether the bound includes its version value.
    pub inclusive: bool,
}

/// What: Store the deterministic intersection of dependency version requirements.
///
/// Inputs:
/// - Zero or more `=`, `>`, `>=`, `<`, or `<=` requirements for one resolved package.
///
/// Output:
/// - Exposes the most restrictive compatible lower and upper bounds.
///
/// Details:
/// - Equal requirements are represented by equal inclusive lower and upper bounds. An absent
///   bound means no requirement on that side of the interval.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyConstraintRange {
    /// Most restrictive compatible lower bound, if any.
    pub lower: Option<DependencyVersionBound>,
    /// Most restrictive compatible upper bound, if any.
    pub upper: Option<DependencyVersionBound>,
}

/// What: Describe the resolution state of a graph node.
///
/// Inputs:
/// - Metadata, provider, and conflict observations made during one graph run.
///
/// Output:
/// - Lets callers identify resolved, missing, and conflicting graph nodes.
///
/// Details:
/// - Missing nodes retain a `DependencyProvenance` with no source rather than being labelled AUR.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyGraphNodeStatus {
    /// Metadata was parsed and the package can participate in dependency traversal.
    #[default]
    Resolved,
    /// Metadata was unavailable or failed validation for the requested package.
    Missing,
    /// The package conflicts with another resolved graph node.
    Conflicting,
}

/// What: Represent one deterministic dependency graph node.
///
/// Inputs:
/// - Verified metadata and merged requirements for one actual package.
///
/// Output:
/// - Stores stable node identity, source provenance, split-package base, and selected metadata.
///
/// Details:
/// - `name` is the actual selected package. `provenance.requested_name` retains the virtual or
///   direct dependency name that selected it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyGraphNode {
    /// Stable actual package identity used by graph edges.
    pub name: String,
    /// Package-base name retained from `.SRCINFO`.
    pub pkgbase: Option<String>,
    /// Combined `epoch:pkgver-pkgrel` version, if metadata was parsed.
    pub version: Option<String>,
    /// Verified source and provider provenance.
    pub provenance: DependencyProvenance,
    /// Current graph node state.
    pub status: DependencyGraphNodeStatus,
    /// Intersected requirements targeting this node.
    pub constraints: DependencyConstraintRange,
    /// Virtual packages supplied by this node.
    pub provides: Vec<String>,
    /// Declared package or virtual conflicts for this node.
    pub conflicts: Vec<String>,
    /// Minimum lexical traversal depth at which this node was encountered.
    pub depth: usize,
}

/// What: Represent one directed dependency requirement between graph nodes.
///
/// Inputs:
/// - Parent and selected child package names, requested dependency name, and version requirement.
///
/// Output:
/// - Preserves dependency and virtual-provider provenance independently of rendering.
///
/// Details:
/// - Edges are sorted lexically by the resolver and can be rendered without triggering metadata
///   lookup or changing the resolution result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyGraphEdge {
    /// Actual package that declares the dependency.
    pub from: String,
    /// Actual selected package, or the missing requested name when metadata is absent.
    pub to: String,
    /// Dependency or virtual package name written by the parent.
    pub requested_name: String,
    /// Version requirement written by the parent, if present.
    pub version_req: String,
}

/// What: Categorize non-fatal graph-resolution diagnostics.
///
/// Inputs:
/// - Metadata, bound, graph, and conflict events observed during resolution.
///
/// Output:
/// - Provides stable categories for actionable caller diagnostics.
///
/// Details:
/// - Diagnostics preserve partial graph results instead of silently omitting failed branches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyGraphDiagnosticKind {
    /// Metadata was not available for a requested package or virtual dependency.
    MissingMetadata,
    /// The injected provider returned a transport, helper, or other retrieval failure.
    MetadataFailure,
    /// Returned `.SRCINFO` text did not contain the selected requested package output.
    MalformedSrcinfo,
    /// A dependency path returned to a package already in the active traversal path.
    Cycle,
    /// A child exceeded the configured transitive depth.
    DepthLimit,
    /// Adding a node exceeded the configured per-run node limit.
    NodeLimit,
    /// The provider exceeded the configured metadata timeout.
    Timeout,
    /// A dependency requirement used an unsupported operator or omitted its version.
    MalformedConstraint,
    /// Multiple valid requirements for one selected package have an empty intersection.
    IncompatibleConstraints,
    /// A declared package or virtual conflict matched another resolved node.
    Conflict,
    /// A provider returned no response or a response for an unrequested package.
    MetadataProtocol,
}

/// What: Record one actionable non-fatal graph-resolution event.
///
/// Inputs:
/// - A diagnostic kind, affected package, optional related package, and message.
///
/// Output:
/// - Lets callers surface partial-resolution limitations without parsing log output.
///
/// Details:
/// - Entries are sorted deterministically by kind, package, related package, and message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyGraphDiagnostic {
    /// Stable category for the event.
    pub kind: DependencyGraphDiagnosticKind,
    /// Affected package or requested dependency name.
    pub package: String,
    /// Related package when the event concerns an edge, cycle, or conflict.
    pub related_package: Option<String>,
    /// Actionable detail suitable for caller display.
    pub message: String,
}

/// What: Return the bounded, deterministic output of one metadata graph-resolution run.
///
/// Inputs:
/// - Root package references, injected metadata, and graph resolution bounds.
///
/// Output:
/// - Contains lexical roots, nodes, edges, and structured diagnostics.
///
/// Details:
/// - The graph is independent from tree rendering and remains useful when metadata failures leave
///   partial branches unresolved.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyGraphResolution {
    /// Requested root package names in lexical order.
    pub roots: Vec<String>,
    /// Resolved and missing nodes in lexical order by actual package name.
    pub nodes: Vec<DependencyGraphNode>,
    /// Directed edges in lexical order.
    pub edges: Vec<DependencyGraphEdge>,
    /// Non-fatal diagnostic events in lexical order.
    pub diagnostics: Vec<DependencyGraphDiagnostic>,
}

/// What: Configure bounded metadata graph resolution.
///
/// Inputs:
/// - Maximum transitive depth, graph-node count, metadata timeout, and provider batch concurrency.
///
/// Output:
/// - Limits resource use for one graph-resolution run.
///
/// Details:
/// - Defaults are depth 8, 256 nodes, 10-second metadata timeout, and one provider batch at a
///   time. The synchronous resolver passes the timeout to the injected provider and keeps only one
///   batch in flight; providers must honor the timeout for preemptive I/O cancellation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DependencyGraphConfig {
    /// Maximum edges from a root to a traversed child; zero resolves root metadata only.
    pub max_depth: usize,
    /// Maximum unique graph nodes, including roots and missing nodes.
    pub max_nodes: usize,
    /// Maximum duration supplied to each metadata provider batch.
    pub metadata_timeout: std::time::Duration,
    /// Maximum names supplied in one provider batch; the synchronous resolver runs batches serially.
    pub max_concurrency: usize,
}

impl Default for DependencyGraphConfig {
    /// What: Construct conservative bounds for graph resolution.
    ///
    /// Inputs:
    /// - None.
    ///
    /// Output:
    /// - Returns depth 8, 256 nodes, a 10-second timeout, and serial provider batching.
    ///
    /// Details:
    /// - These defaults constrain fixture and production providers without changing the legacy
    ///   direct-only `DependencyResolver::resolve` entry point.
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_nodes: 256,
            metadata_timeout: std::time::Duration::from_secs(10),
            max_concurrency: 1,
        }
    }
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
