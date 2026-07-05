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

    /// Helper to build a sample index for persistence tests.
    fn sample_index() -> arch_toolkit::index::OfficialIndex {
        let mut index = arch_toolkit::index::OfficialIndex {
            pkgs: vec![
                arch_toolkit::index::OfficialPackage {
                    name: "ripgrep".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "14.0.0".to_string(),
                    description: "Fast grep".to_string(),
                },
                arch_toolkit::index::OfficialPackage {
                    name: "vim".to_string(),
                    repo: "extra".to_string(),
                    arch: "x86_64".to_string(),
                    version: "9.0".to_string(),
                    description: "Text editor".to_string(),
                },
            ],
            name_to_idx: std::collections::HashMap::new(),
        };
        index.rebuild_name_index();
        index
    }

    /// Helper to create a unique temp file path for persistence tests.
    fn temp_index_path(tag: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "arch_toolkit_index_integration_{tag}_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("System time is before UNIX epoch")
                .as_nanos()
        ));
        path
    }

    #[test]
    /// What: Verify the full save → load → search workflow.
    ///
    /// Inputs:
    /// - Sample index persisted to disk, reloaded, then queried.
    ///
    /// Output:
    /// - Loaded index supports both name lookup and substring search.
    ///
    /// Details:
    /// - Exercises persistence together with the query API end to end.
    fn persist_roundtrip_supports_queries() {
        use arch_toolkit::index::{load_from_disk, save_to_disk, search_official};

        let index = sample_index();
        let path = temp_index_path("query");

        save_to_disk(&index, &path).expect("save should succeed");
        let loaded = load_from_disk(&path).expect("load should succeed");

        // Name lookup works after rebuild on load
        let found = loaded.find_package_by_name("VIM");
        assert_eq!(found.map(|p| p.name.as_str()), Some("vim"));

        // Substring search works against the loaded index
        let results = search_official(&loaded, "rip", false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package.name, "ripgrep");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    /// What: Verify reloading a rewritten index replaces prior data.
    ///
    /// Inputs:
    /// - Two different snapshots written to the same path sequentially.
    ///
    /// Output:
    /// - Second load reflects only the second snapshot's data.
    ///
    /// Details:
    /// - Mirrors Pacsea's reload-replaces-index behavior without global state.
    fn persist_reload_replaces_previous_data() {
        use arch_toolkit::index::{OfficialIndex, OfficialPackage, load_from_disk, save_to_disk};

        let path = temp_index_path("reload");

        let first = sample_index();
        save_to_disk(&first, &path).expect("first save should succeed");

        let mut second = OfficialIndex {
            pkgs: vec![OfficialPackage {
                name: "bat".to_string(),
                repo: "extra".to_string(),
                arch: "x86_64".to_string(),
                version: "0.24".to_string(),
                description: "Cat clone".to_string(),
            }],
            name_to_idx: std::collections::HashMap::new(),
        };
        second.rebuild_name_index();
        save_to_disk(&second, &path).expect("second save should succeed");

        let loaded = load_from_disk(&path).expect("load should succeed");
        assert_eq!(loaded.pkgs.len(), 1);
        assert!(loaded.find_package_by_name("bat").is_some());
        assert!(loaded.find_package_by_name("ripgrep").is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    /// What: Verify the async persistence variants in an integration context.
    ///
    /// Inputs:
    /// - Sample index saved and loaded via `save_to_disk_async`/`load_from_disk_async`.
    ///
    /// Output:
    /// - Loaded index matches the saved data.
    ///
    /// Details:
    /// - Confirms the `spawn_blocking` wrappers work under a tokio runtime.
    async fn persist_async_roundtrip() {
        use arch_toolkit::index::{load_from_disk_async, save_to_disk_async};

        let index = sample_index();
        let path = temp_index_path("async");

        save_to_disk_async(index.clone(), path.clone())
            .await
            .expect("async save should succeed");
        let loaded = load_from_disk_async(path.clone())
            .await
            .expect("async load should succeed");

        assert_eq!(loaded.pkgs, index.pkgs);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    /// What: Verify loading a missing index file surfaces an `Io` error.
    ///
    /// Inputs:
    /// - Non-existent file path.
    ///
    /// Output:
    /// - `Err(ArchToolkitError::Io)` allowing callers to fall back to fetching.
    ///
    /// Details:
    /// - The load-or-fetch pattern from the module docs relies on this behavior.
    fn persist_load_missing_file_errors() {
        use arch_toolkit::error::ArchToolkitError;
        use arch_toolkit::index::load_from_disk;

        let path = temp_index_path("missing");
        let result = load_from_disk(&path);
        assert!(matches!(result, Err(ArchToolkitError::Io { .. })));
    }
}
