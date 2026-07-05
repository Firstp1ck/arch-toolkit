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
}
