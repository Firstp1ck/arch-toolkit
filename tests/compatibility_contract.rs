//! Deterministic cross-module compatibility and missing-tool contracts.

#[cfg(any(
    feature = "aur",
    feature = "deps",
    feature = "index",
    feature = "install",
    feature = "news",
    feature = "sandbox"
))]
mod tests {
    #[cfg(all(any(feature = "install", feature = "deps"), unix))]
    use std::path::Path;
    #[cfg(all(feature = "install", unix))]
    use std::path::PathBuf;
    #[cfg(all(any(feature = "install", feature = "deps"), unix))]
    use std::sync::{Mutex, MutexGuard, PoisonError};

    /// Process-local serialization for PATH mutation in this integration binary.
    #[cfg(all(any(feature = "install", feature = "deps"), unix))]
    static PATH_LOCK: Mutex<()> = Mutex::new(());

    /// What: Restore PATH after a deterministic command-discovery scenario.
    ///
    /// Inputs:
    /// - Constructed by [`PathGuard::replace`].
    ///
    /// Output:
    /// - PATH remains replaced until this guard is dropped.
    ///
    /// Details:
    /// - The mutex prevents sibling tests in this binary from observing temporary tool fixtures.
    #[cfg(all(any(feature = "install", feature = "deps"), unix))]
    struct PathGuard {
        /// Held process-local PATH lock.
        _lock: MutexGuard<'static, ()>,
        /// Original PATH value.
        original: Option<String>,
    }

    #[cfg(all(any(feature = "install", feature = "deps"), unix))]
    impl PathGuard {
        /// What: Replace PATH with one deterministic directory.
        ///
        /// Inputs:
        /// - `directory`: Directory containing only the tools required by the scenario.
        ///
        /// Output:
        /// - A restoring guard.
        ///
        /// Details:
        /// - Child command lookup cannot fall through to host pacman/helper/privilege binaries.
        fn replace(directory: &Path) -> Self {
            let lock = PATH_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            let original = std::env::var("PATH").ok();
            unsafe { std::env::set_var("PATH", directory) };
            Self {
                _lock: lock,
                original,
            }
        }
    }

    #[cfg(all(any(feature = "install", feature = "deps"), unix))]
    impl Drop for PathGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var("PATH", value),
                    None => std::env::remove_var("PATH"),
                }
            }
        }
    }

    /// What: Create an executable no-op tool fixture.
    ///
    /// Inputs:
    /// - `directory`: Fixture bin directory.
    /// - `name`: Tool binary name.
    ///
    /// Output:
    /// - Path to the executable fixture.
    ///
    /// Details:
    /// - Unix-only because production Arch Linux command discovery is Unix-oriented.
    #[cfg(all(feature = "install", unix))]
    fn create_tool(directory: &Path, name: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        std::fs::create_dir_all(directory).expect("create fixture bin");
        let path = directory.join(name);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("write fixture tool");
        let mut permissions = std::fs::metadata(&path)
            .expect("read fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).expect("make fixture executable");
        path
    }

    #[cfg(feature = "aur")]
    #[test]
    fn aur_percent_encoding_contract() {
        assert_eq!(
            arch_toolkit::aur::utils::percent_encode("name+space ü"),
            "name%2Bspace%20%C3%BC"
        );
    }

    #[cfg(all(feature = "install", unix))]
    #[test]
    fn tool_detection_and_planning_are_deterministic_and_side_effect_free() {
        use arch_toolkit::install::{
            build_pacman_install, detect_aur_helper, detect_privilege_tool,
        };
        use arch_toolkit::types::install::{AurHelper, InstallOptions, PrivilegeTool};

        let temp = tempfile::tempdir().expect("temporary fixture directory");
        let bin = temp.path().join("bin");
        create_tool(&bin, "doas");
        create_tool(&bin, "sudo");
        create_tool(&bin, "paru");
        create_tool(&bin, "yay");
        let _path = PathGuard::replace(&bin);

        assert_eq!(detect_privilege_tool(), Some(PrivilegeTool::Doas));
        assert_eq!(detect_aur_helper(), Some(AurHelper::Paru));
        let marker = temp.path().join("executed");
        let spec = build_pacman_install(&["ripgrep"], &InstallOptions::default())
            .expect("build command plan");
        assert_eq!(
            spec.to_shell_string(),
            "pacman -S --needed --noconfirm -- ripgrep"
        );
        assert!(!marker.exists(), "building a command must never execute it");
    }

    #[cfg(all(feature = "install", unix))]
    #[test]
    fn missing_helpers_and_privilege_tools_return_none() {
        let temp = tempfile::tempdir().expect("temporary fixture directory");
        let _path = PathGuard::replace(temp.path());
        assert_eq!(arch_toolkit::install::detect_aur_helper(), None);
        assert_eq!(arch_toolkit::install::detect_privilege_tool(), None);
    }

    #[cfg(all(feature = "deps", unix))]
    #[test]
    fn missing_pacman_queries_follow_documented_empty_behavior() {
        let temp = tempfile::tempdir().expect("temporary fixture directory");
        let _path = PathGuard::replace(temp.path());
        assert!(
            arch_toolkit::deps::get_installed_packages()
                .expect("missing pacman degrades to an empty set")
                .is_empty()
        );
        assert!(
            arch_toolkit::deps::get_upgradable_packages()
                .expect("missing pacman degrades to an empty set")
                .is_empty()
        );
    }

    #[cfg(feature = "index")]
    #[test]
    fn tolerant_index_loading_contract() {
        let temp = tempfile::tempdir().expect("temporary fixture directory");
        let path = temp.path().join("corrupt.json");
        std::fs::write(&path, "not json").expect("write corrupt index");
        assert!(
            arch_toolkit::index::load_from_disk_or_default(&path)
                .pkgs
                .is_empty()
        );
    }

    #[cfg(feature = "aur")]
    #[test]
    /// What: Freeze the AUR model fields consumed by Pacsea.
    ///
    /// Inputs:
    /// - Representative search, detail, and comment values.
    ///
    /// Output:
    /// - Direct field access and serialization retain the consumer-facing data.
    ///
    /// Details:
    /// - This is an aggregate compatibility fixture, not a network test.
    fn pacsea_aur_model_contract() {
        use arch_toolkit::types::package::{AurComment, AurPackage, AurPackageDetails};

        let package = AurPackage {
            name: "paru".to_string(),
            version: "2.1.0-1".to_string(),
            description: "AUR helper".to_string(),
            popularity: Some(9.5),
            out_of_date: Some(1_700_000_000),
            orphaned: false,
            maintainer: Some("maintainer".to_string()),
        };
        assert_eq!(package.name, "paru");
        assert_eq!(package.maintainer.as_deref(), Some("maintainer"));

        let details = AurPackageDetails {
            name: "paru".to_string(),
            version: "2.1.0-1".to_string(),
            depends: vec!["pacman".to_string()],
            make_depends: vec!["cargo".to_string()],
            opt_depends: vec!["bat: colored output".to_string()],
            provides: vec!["aur-helper".to_string()],
            conflicts: vec!["paru-bin".to_string()],
            num_votes: Some(42),
            ..Default::default()
        };
        assert_eq!(details.depends, ["pacman"]);
        assert_eq!(details.provides, ["aur-helper"]);
        assert_eq!(details.num_votes, Some(42));

        let comment = AurComment {
            id: Some("123".to_string()),
            author: "alice".to_string(),
            date: "2026-08-08".to_string(),
            date_timestamp: Some(1_786_147_200),
            date_url: Some("https://aur.archlinux.org/packages/paru#comment-123".to_string()),
            content: "Pinned guidance".to_string(),
            pinned: true,
        };
        let serialized = serde_json::to_value(comment).expect("serialize AUR comment");
        assert_eq!(serialized["id"], "123");
        assert_eq!(serialized["pinned"], true);
    }

    #[cfg(feature = "news")]
    #[test]
    /// What: Freeze advisory identity, severity, package extraction, and serde fields for Pacsea.
    ///
    /// Inputs:
    /// - A representative security advisory.
    ///
    /// Output:
    /// - Stable ID, rank, package list, and serialized representation.
    ///
    /// Details:
    /// - Caller read-state remains outside the library; this fixture covers only shared data.
    fn pacsea_news_model_contract() {
        use arch_toolkit::types::news::{AdvisorySeverity, SecurityAdvisory};

        let advisory = SecurityAdvisory {
            id: "ASA-202608-1".to_string(),
            date: "2026-08-08".to_string(),
            title: "ASA-202608-1: openssl: multiple issues".to_string(),
            summary: Some("Multiple issues".to_string()),
            url: Some("https://security.archlinux.org/ASA-202608-1".to_string()),
            severity: AdvisorySeverity::High,
            packages: vec!["openssl".to_string()],
        };
        assert_eq!(advisory.id, "ASA-202608-1");
        assert_eq!(advisory.severity.rank(), 4);
        assert_eq!(advisory.packages, ["openssl"]);
        let serialized = serde_json::to_value(advisory).expect("serialize advisory");
        assert_eq!(serialized["severity"], "High");
    }

    #[cfg(feature = "sandbox")]
    #[test]
    /// What: Freeze sandbox dependency-delta serde and version-state behavior for Pacsea.
    ///
    /// Inputs:
    /// - One installed but version-unsatisfied dependency and one missing dependency.
    ///
    /// Output:
    /// - Stable roundtrip data, missing-package list, and readiness behavior.
    ///
    /// Details:
    /// - Pacsea remains responsible for combining installation and version satisfaction in its adapter.
    fn pacsea_sandbox_model_contract() {
        use arch_toolkit::types::sandbox::{DependencyDelta, SandboxInfo};

        let info = SandboxInfo {
            package_name: "demo".to_string(),
            depends: vec![
                DependencyDelta {
                    name: "openssl>=3.5".to_string(),
                    is_installed: true,
                    installed_version: Some("3.4".to_string()),
                    version_satisfied: false,
                },
                DependencyDelta {
                    name: "missing-runtime".to_string(),
                    is_installed: false,
                    installed_version: None,
                    version_satisfied: false,
                },
            ],
            ..Default::default()
        };
        assert_eq!(info.missing_packages(), ["missing-runtime"]);
        assert!(!info.is_ready_to_build());
        assert!(!info.depends[0].version_satisfied);

        let serialized = serde_json::to_string(&info).expect("serialize sandbox info");
        let roundtrip: SandboxInfo =
            serde_json::from_str(&serialized).expect("deserialize sandbox info");
        assert_eq!(roundtrip, info);
    }
}
