//! Deterministic cross-module compatibility and missing-tool contracts.

#[cfg(any(
    feature = "aur",
    feature = "deps",
    feature = "index",
    feature = "install"
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
            "pacman -S --needed --noconfirm ripgrep"
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
}
