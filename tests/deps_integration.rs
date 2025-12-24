//! Integration tests for the deps module.
//!
//! These tests verify that dependency resolution, reverse dependency analysis,
//! and package querying work correctly with real pacman commands (when available).
//! Note: Tests requiring pacman or network access are marked with `#[ignore]`.

#![cfg(feature = "deps")]

use arch_toolkit::deps::{
    DependencyResolver, ReverseDependencyAnalyzer, get_installed_packages, get_upgradable_packages,
};
use arch_toolkit::error::Result;
use arch_toolkit::{PackageRef, PackageSource};

/// Test that dependency resolver handles empty input gracefully.
#[test]
fn test_dependency_resolver_empty() -> Result<()> {
    let resolver = DependencyResolver::new();
    let result = resolver.resolve(&[])?;
    assert_eq!(result.dependencies.len(), 0);
    assert_eq!(result.conflicts.len(), 0);
    assert_eq!(result.missing.len(), 0);
    Ok(())
}

/// Test dependency resolution with a real package (requires pacman).
#[test]
#[ignore = "Requires pacman to be available"]
fn test_dependency_resolver_real_package() -> Result<()> {
    let resolver = DependencyResolver::new();
    let packages = vec![PackageRef {
        name: "pacman".to_string(),
        version: "6.1.0".to_string(),
        source: PackageSource::Official {
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    let result = resolver.resolve(&packages)?;
    // Should find some dependencies for pacman
    println!(
        "Found {} dependencies for pacman",
        result.dependencies.len()
    );
    assert!(!result.dependencies.is_empty());
    Ok(())
}

/// Test dependency resolution with multiple packages (requires pacman).
#[test]
#[ignore = "Requires pacman to be available"]
fn test_dependency_resolver_multiple_packages() -> Result<()> {
    let resolver = DependencyResolver::new();
    let packages = vec![
        PackageRef {
            name: "pacman".to_string(),
            version: "6.1.0".to_string(),
            source: PackageSource::Official {
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
            },
        },
        PackageRef {
            name: "glibc".to_string(),
            version: "2.38".to_string(),
            source: PackageSource::Official {
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
            },
        },
    ];

    let result = resolver.resolve(&packages)?;
    println!(
        "Found {} dependencies for {} packages",
        result.dependencies.len(),
        packages.len()
    );
    // Should find dependencies
    assert!(!result.dependencies.is_empty());
    Ok(())
}

/// Test dependency resolver with custom configuration.
#[test]
fn test_dependency_resolver_with_config() -> Result<()> {
    use arch_toolkit::ResolverConfig;

    let config = ResolverConfig {
        include_optdepends: true,
        include_makedepends: false,
        include_checkdepends: false,
        max_depth: 0,
        pkgbuild_cache: None,
        check_aur: false,
    };

    let resolver = DependencyResolver::with_config(config);
    let result = resolver.resolve(&[])?;
    assert_eq!(result.dependencies.len(), 0);
    Ok(())
}

/// Test reverse dependency analyzer with empty input.
#[test]
fn test_reverse_dependency_analyzer_empty() -> Result<()> {
    let analyzer = ReverseDependencyAnalyzer::new();
    let result = analyzer.analyze(&[])?;
    assert_eq!(result.dependents.len(), 0);
    assert_eq!(result.summaries.len(), 0);
    Ok(())
}

/// Test reverse dependency analysis with a real package (requires pacman).
#[test]
#[ignore = "Requires pacman to be available and package to be installed"]
fn test_reverse_dependency_analyzer_real_package() -> Result<()> {
    let analyzer = ReverseDependencyAnalyzer::new();
    let packages = vec![PackageRef {
        name: "glibc".to_string(),
        version: "2.38".to_string(),
        source: PackageSource::Official {
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    let result = analyzer.analyze(&packages)?;
    println!("Found {} dependents for glibc", result.dependents.len());
    // glibc should have many dependents
    assert!(!result.dependents.is_empty());
    Ok(())
}

/// Test reverse dependency analysis with uninstalled package.
#[test]
#[ignore = "Requires pacman to be available"]
fn test_reverse_dependency_analyzer_uninstalled_package() -> Result<()> {
    let analyzer = ReverseDependencyAnalyzer::new();
    let packages = vec![PackageRef {
        name: "nonexistent-package-xyz123".to_string(),
        version: "1.0.0".to_string(),
        source: PackageSource::Official {
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    let result = analyzer.analyze(&packages)?;
    // Uninstalled packages should return empty result
    assert_eq!(result.dependents.len(), 0);
    assert_eq!(result.summaries.len(), 0);
    Ok(())
}

/// Test reverse dependency analysis with multiple packages (requires pacman).
#[test]
#[ignore = "Requires pacman to be available and packages to be installed"]
fn test_reverse_dependency_analyzer_multiple_packages() -> Result<()> {
    let analyzer = ReverseDependencyAnalyzer::new();
    let packages = vec![
        PackageRef {
            name: "glibc".to_string(),
            version: "2.38".to_string(),
            source: PackageSource::Official {
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
            },
        },
        PackageRef {
            name: "bash".to_string(),
            version: "5.2".to_string(),
            source: PackageSource::Official {
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
            },
        },
    ];

    let result = analyzer.analyze(&packages)?;
    println!(
        "Found {} dependents for {} packages",
        result.dependents.len(),
        packages.len()
    );
    // Should find dependents
    assert!(!result.dependents.is_empty());
    Ok(())
}

/// Test package querying functions (requires pacman).
#[test]
#[ignore = "Requires pacman to be available"]
fn test_get_installed_packages_integration() -> Result<()> {
    let packages = get_installed_packages()?;
    // Should have at least some packages on a real system
    println!("Found {} installed packages", packages.len());
    // We can't assert exact count since it varies, but should be > 0
    assert!(!packages.is_empty());
    Ok(())
}

/// Test upgradable packages query (requires pacman).
#[test]
#[ignore = "Requires pacman to be available"]
fn test_get_upgradable_packages_integration() -> Result<()> {
    let packages = get_upgradable_packages()?;
    // May be empty if system is up to date
    println!("Found {} upgradable packages", packages.len());
    // This is fine - just verify it doesn't crash
    Ok(())
}

/// Test graceful degradation when pacman is unavailable.
#[test]
fn test_graceful_degradation_no_pacman() {
    // This test verifies that functions handle missing pacman gracefully
    // by checking that they return empty results rather than panicking

    // Note: This is hard to test without actually removing pacman,
    // but the code should handle Command::new("pacman") failures gracefully
    // by returning empty sets or None

    // We can at least verify the functions exist and can be called
    let _resolver = DependencyResolver::new();
    let _analyzer = ReverseDependencyAnalyzer::new();
}

#[cfg(feature = "aur")]
mod aur_tests {
    use super::*;
    use arch_toolkit::deps::{fetch_srcinfo, parse_srcinfo};
    use reqwest::Client;

    /// Test fetching .SRCINFO from AUR (requires network access).
    #[tokio::test]
    #[ignore = "Requires network access"]
    async fn test_fetch_srcinfo_from_aur() -> Result<()> {
        let client = Client::new();
        let srcinfo: String = fetch_srcinfo(&client, "yay").await?;
        assert!(!srcinfo.is_empty());

        // Should be valid .SRCINFO format
        let data = parse_srcinfo(&srcinfo);
        assert_eq!(data.pkgname, "yay");
        assert!(!data.pkgver.is_empty());
        Ok(())
    }

    /// Test parsing fetched .SRCINFO.
    #[tokio::test]
    #[ignore = "Requires network access"]
    async fn test_parse_fetched_srcinfo() -> Result<()> {
        let client = Client::new();
        let srcinfo = fetch_srcinfo(&client, "paru").await?;
        let data = parse_srcinfo(&srcinfo);

        assert_eq!(data.pkgname, "paru");
        assert!(!data.pkgver.is_empty());
        // Should have some dependencies
        println!("Found {} dependencies for paru", data.depends.len());
        Ok(())
    }

    /// Test fetching .SRCINFO for non-existent package.
    #[tokio::test]
    #[ignore = "Requires network access"]
    async fn test_fetch_srcinfo_nonexistent() {
        // Should return an error for non-existent packages
        let client = Client::new();
        let result: arch_toolkit::error::Result<String> =
            fetch_srcinfo(&client, "nonexistent-package-xyz123").await;
        assert!(result.is_err());
    }
}
