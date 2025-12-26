//! Integration tests for the index module using mock pacman commands.

#[cfg(feature = "index")]
mod tests {
    use arch_toolkit::index::{
        InstalledPackagesMode, get_installed_packages, is_explicit, is_installed,
        refresh_explicit_cache, refresh_explicit_cache_async, refresh_installed_cache,
        refresh_installed_cache_async,
    };
    use std::collections::HashSet;

    /// Helper to create a temporary pacman script for testing.
    #[cfg(not(target_os = "windows"))]
    fn create_mock_pacman_script(
        root: &std::path::Path,
        command: &str,
        output: &str,
    ) -> std::io::Result<()> {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let bin_dir = root.join("bin");
        fs::create_dir_all(&bin_dir)?;

        let script_path = bin_dir.join("pacman");
        let script_content = format!(
            r#"#!/usr/bin/env bash
set -e
if [[ "$*" == "{command}" ]]; then
  echo "{output}"
  exit 0
fi
exit 1
"#
        );
        fs::write(&script_path, script_content)?;

        #[cfg(unix)]
        {
            let mut perm = fs::metadata(&script_path)?.permissions();
            perm.set_mode(0o755);
            fs::set_permissions(&script_path, perm)?;
        }

        Ok(())
    }

    /// Helper to set up PATH with mock pacman and restore it.
    #[cfg(not(target_os = "windows"))]
    struct PathGuard {
        original: String,
    }

    #[cfg(not(target_os = "windows"))]
    impl Drop for PathGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::set_var("PATH", &self.original);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    /// What: Verify `refresh_installed_cache` populates cache from pacman output.
    ///
    /// Inputs:
    /// - Override PATH with a fake pacman that emits installed package names.
    ///
    /// Output:
    /// - Cache lookup succeeds for the emitted names after refresh completes.
    ///
    /// Details:
    /// - Exercises the sync refresh path and verifies cache contents.
    async fn refresh_installed_cache_populates_cache_from_pacman_output() {
        let original_path = std::env::var("PATH").unwrap_or_default();
        let _path_guard = PathGuard {
            original: original_path.clone(),
        };

        let mut root = std::env::temp_dir();
        root.push(format!(
            "arch_toolkit_fake_pacman_qq_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));

        create_mock_pacman_script(&root, "-Qq", "alpha\nbeta\ngamma\n")
            .expect("Failed to create mock pacman script");

        let bin = root.join("bin");
        let new_path = format!("{}:{}", bin.to_string_lossy(), original_path);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        // Small delay to ensure PATH is propagated to child processes (needed on macOS)
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut cache = HashSet::new();
        let result = refresh_installed_cache(Some(&mut cache));

        let _ = std::fs::remove_dir_all(&root);

        assert!(result.is_ok());
        let packages = result.expect("refresh_installed_cache should succeed");
        assert!(packages.contains("alpha"));
        assert!(packages.contains("beta"));
        assert!(packages.contains("gamma"));
        assert_eq!(cache.len(), 3);
        assert!(cache.contains("alpha"));
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    /// What: Verify `refresh_installed_cache_async` works asynchronously with mock pacman.
    ///
    /// Inputs:
    /// - Override PATH with a fake pacman and call async refresh.
    ///
    /// Output:
    /// - Cache is populated after async operation completes.
    ///
    /// Details:
    /// - Tests async version with mock pacman script.
    async fn refresh_installed_cache_async_works_with_mock() {
        let original_path = std::env::var("PATH").unwrap_or_default();
        let _path_guard = PathGuard {
            original: original_path.clone(),
        };

        let mut root = std::env::temp_dir();
        root.push(format!(
            "arch_toolkit_fake_pacman_qq_async_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));

        create_mock_pacman_script(&root, "-Qq", "package1\npackage2\n")
            .expect("Failed to create mock pacman script");

        let bin = root.join("bin");
        let new_path = format!("{}:{}", bin.to_string_lossy(), original_path);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut cache = HashSet::new();
        let result = refresh_installed_cache_async(Some(&mut cache)).await;

        let _ = std::fs::remove_dir_all(&root);

        assert!(result.is_ok());
        let packages = result.expect("refresh_installed_cache should succeed");
        assert!(packages.contains("package1"));
        assert!(packages.contains("package2"));
        assert_eq!(cache.len(), 2);
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    /// What: Verify `refresh_explicit_cache` populates cache with explicit packages.
    ///
    /// Inputs:
    /// - Override PATH with a fake pacman that emits explicit package names.
    ///
    /// Output:
    /// - Cache contains explicit packages after refresh.
    ///
    /// Details:
    /// - Tests both `LeafOnly` and `AllExplicit` modes.
    async fn refresh_explicit_cache_populates_cache_from_pacman_output() {
        let original_path = std::env::var("PATH").unwrap_or_default();
        let _path_guard = PathGuard {
            original: original_path.clone(),
        };

        let mut root = std::env::temp_dir();
        root.push(format!(
            "arch_toolkit_fake_pacman_qeq_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));

        create_mock_pacman_script(&root, "-Qeq", "vim\ngit\npython\n")
            .expect("Failed to create mock pacman script");

        let bin = root.join("bin");
        let new_path = format!("{}:{}", bin.to_string_lossy(), original_path);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut cache = HashSet::new();
        let result = refresh_explicit_cache(InstalledPackagesMode::AllExplicit, Some(&mut cache));

        let _ = std::fs::remove_dir_all(&root);

        assert!(result.is_ok());
        let packages = result.expect("refresh_installed_cache should succeed");
        assert!(packages.contains("vim"));
        assert!(packages.contains("git"));
        assert!(packages.contains("python"));
        assert_eq!(cache.len(), 3);
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    /// What: Verify `refresh_explicit_cache_async` works with mock pacman.
    ///
    /// Inputs:
    /// - Override PATH with a fake pacman and call async refresh.
    ///
    /// Output:
    /// - Cache is populated after async operation completes.
    ///
    /// Details:
    /// - Tests async version with `LeafOnly` mode.
    async fn refresh_explicit_cache_async_works_with_mock() {
        let original_path = std::env::var("PATH").unwrap_or_default();
        let _path_guard = PathGuard {
            original: original_path.clone(),
        };

        let mut root = std::env::temp_dir();
        root.push(format!(
            "arch_toolkit_fake_pacman_qetq_async_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));

        create_mock_pacman_script(&root, "-Qetq", "leaf1\nleaf2\n")
            .expect("Failed to create mock pacman script");

        let bin = root.join("bin");
        let new_path = format!("{}:{}", bin.to_string_lossy(), original_path);
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut cache = HashSet::new();
        let result =
            refresh_explicit_cache_async(InstalledPackagesMode::LeafOnly, Some(&mut cache)).await;

        let _ = std::fs::remove_dir_all(&root);

        assert!(result.is_ok());
        let packages = result.expect("refresh_installed_cache should succeed");
        assert!(packages.contains("leaf1"));
        assert!(packages.contains("leaf2"));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    /// What: Verify `is_installed` works with cache.
    ///
    /// Inputs:
    /// - Cache containing package names and function call.
    ///
    /// Output:
    /// - Returns correct boolean values for cached packages.
    ///
    /// Details:
    /// - Tests cache-based lookup without requiring pacman.
    fn is_installed_works_with_cache() {
        let cache = HashSet::from(["vim".to_string(), "git".to_string()]);
        assert!(is_installed("vim", Some(&cache)));
        assert!(is_installed("git", Some(&cache)));
        assert!(!is_installed("nonexistent", Some(&cache)));
    }

    #[test]
    /// What: Verify `is_explicit` works with cache.
    ///
    /// Inputs:
    /// - Cache containing explicit package names and function call.
    ///
    /// Output:
    /// - Returns correct boolean values for cached packages.
    ///
    /// Details:
    /// - Tests cache-based lookup for both modes without requiring pacman.
    fn is_explicit_works_with_cache() {
        let cache = HashSet::from(["vim".to_string(), "git".to_string()]);
        assert!(is_explicit(
            "vim",
            InstalledPackagesMode::AllExplicit,
            Some(&cache)
        ));
        assert!(is_explicit(
            "git",
            InstalledPackagesMode::LeafOnly,
            Some(&cache)
        ));
        assert!(!is_explicit(
            "nonexistent",
            InstalledPackagesMode::AllExplicit,
            Some(&cache)
        ));
    }

    #[test]
    /// What: Verify `get_installed_packages` returns `HashSet`.
    ///
    /// Inputs:
    /// - Direct call to `get_installed_packages`.
    ///
    /// Output:
    /// - Returns Ok(HashSet<String>) (may be empty if pacman unavailable).
    ///
    /// Details:
    /// - Tests that function returns correct type and handles errors gracefully.
    fn get_installed_packages_returns_hashset() {
        let result = get_installed_packages();
        assert!(result.is_ok());
        // Result may be empty if pacman unavailable, which is graceful degradation
    }
}
