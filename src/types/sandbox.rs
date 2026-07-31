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

/// What: Identify one stable static PKGBUILD threat-model rule.
///
/// Inputs:
/// - Produced by [`crate::sandbox::analyze_pkgbuild_security`] when matching
///   text is found in a PKGBUILD.
///
/// Output:
/// - A stable serialized `SB00x` identifier suitable for callers to filter or
///   present without relying on an opaque aggregate score.
///
/// Details:
/// - Rules describe potentially risky shell constructs, not proof of malicious
///   intent. They are intentionally deterministic and text-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxRuleId {
    /// `SB001`: Command substitution can execute a dynamically constructed command.
    #[serde(rename = "SB001")]
    CommandSubstitution,
    /// `SB002`: A download command can retrieve unreviewed remote content.
    #[serde(rename = "SB002")]
    RemoteDownload,
    /// `SB003`: A privilege escalation command expands the impact of a build step.
    #[serde(rename = "SB003")]
    PrivilegedCommand,
    /// `SB004`: Recursive forced removal can destroy files outside a package build tree.
    #[serde(rename = "SB004")]
    DestructiveRemoval,
    /// `SB005`: Dynamic evaluation obscures the command text that will run.
    #[serde(rename = "SB005")]
    DynamicEvaluation,
}

impl SandboxRuleId {
    /// What: Return the stable textual identifier for this static-analysis rule.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - One of `SB001` through `SB005`.
    ///
    /// Details:
    /// - The value matches the enum's serde representation and is stable for
    ///   caller-side policy, fixture, and display code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CommandSubstitution => "SB001",
            Self::RemoteDownload => "SB002",
            Self::PrivilegedCommand => "SB003",
            Self::DestructiveRemoval => "SB004",
            Self::DynamicEvaluation => "SB005",
        }
    }
}

/// What: Record evidence for one deterministic static PKGBUILD finding.
///
/// Inputs:
/// - Produced from one matched source line during text-only analysis.
///
/// Output:
/// - Stable rule ID, one-based line number, and a bounded source excerpt.
///
/// Details:
/// - Evidence is not executed, expanded, or parsed as full shell syntax.
/// - A finding flags review-worthy text, not a proven exploit or reputation score.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxFinding {
    /// Stable rule identifier describing the matched construct.
    pub rule_id: SandboxRuleId,
    /// One-based PKGBUILD line containing the evidence.
    pub line: usize,
    /// Bounded source excerpt retained exactly for caller review.
    pub evidence: String,
}

/// What: State a known limitation of deterministic static PKGBUILD analysis.
///
/// Inputs:
/// - Included in every [`SandboxStaticAnalysis`] result.
///
/// Output:
/// - Structured, explicit scope information rather than an implied guarantee.
///
/// Details:
/// - Limitations are stable categories so callers can present or persist the
///   analysis boundary alongside findings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxAnalysisLimitation {
    /// Analysis reads raw text only and never executes PKGBUILD content.
    #[serde(rename = "text-only-no-execution")]
    TextOnlyNoExecution,
    /// The scanner does not implement a complete Bash parser or expansion model.
    #[serde(rename = "not-a-full-shell-parser")]
    NotFullShellParser,
    /// Remote reputation, signatures, and external scanner results are not included.
    #[serde(rename = "no-external-reputation-or-scanner")]
    NoExternalReputationOrScanner,
    /// Findings identify review signals and can include false negatives or positives.
    #[serde(rename = "not-proof-of-malicious-intent")]
    NotProofOfMaliciousIntent,
}

/// What: Hold a text-only PKGBUILD threat-model analysis result.
///
/// Inputs:
/// - Produced by [`crate::sandbox::analyze_pkgbuild_security`] from a package
///   name and unexecuted PKGBUILD text.
///
/// Output:
/// - Structured stable-rule findings and explicit scanner limitations.
///
/// Details:
/// - No aggregate risk score is calculated.
/// - The report can be serialized for caller-owned review workflows without
///   granting the library authority to execute or build the PKGBUILD.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxStaticAnalysis {
    /// Caller-provided package name associated with the analyzed text.
    pub package_name: String,
    /// Findings in source-line and stable-rule order.
    pub findings: Vec<SandboxFinding>,
    /// Explicit scope and correctness limitations of this text-only result.
    pub limitations: Vec<SandboxAnalysisLimitation>,
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
