//! Integration tests for the deps module.
//!
//! These tests verify that dependency resolution, reverse dependency analysis,
//! and package querying work correctly with real pacman commands (when available).
//! Note: Tests requiring pacman or network access are marked with `#[ignore]`.

#![cfg(feature = "deps")]

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use arch_toolkit::deps::{
    DependencyGraphConfig, DependencyGraphDiagnosticKind, DependencyMetadata,
    DependencyMetadataProvider, DependencyMetadataResponse, DependencyResolver,
    ReverseDependencyAnalyzer, get_installed_packages, get_upgradable_packages,
};
use arch_toolkit::error::Result;
use arch_toolkit::{DependencySource, PackageRef, PackageSource};

/// Fixture-only metadata outcome returned by `FixtureMetadataProvider`.
#[derive(Clone)]
enum FixtureMetadataOutcome {
    /// Parsed metadata response.
    Found {
        /// Selected package name.
        package_name: String,
        /// Package provenance.
        source: DependencySource,
        /// Raw `.SRCINFO` payload.
        srcinfo: String,
    },
    /// Absent metadata response.
    Missing {
        /// Actionable absence reason.
        reason: String,
    },
    /// Deterministic transport or helper failure.
    Failure {
        /// Actionable failure message.
        message: String,
    },
}

/// Fixture-only batched metadata provider for public graph resolver tests.
struct FixtureMetadataProvider {
    /// Responses indexed by the requested package or virtual name.
    outcomes: BTreeMap<String, FixtureMetadataOutcome>,
    /// Per-run request log used to assert caching behavior.
    requests: Mutex<Vec<String>>,
}

impl FixtureMetadataProvider {
    /// What: Construct a metadata provider from deterministic fixture responses.
    ///
    /// Inputs:
    /// - `outcomes`: Metadata outcomes indexed by requested dependency name.
    ///
    /// Output:
    /// - Returns a provider with an empty request log.
    ///
    /// Details:
    /// - The provider does not execute commands or access the network.
    const fn new(outcomes: BTreeMap<String, FixtureMetadataOutcome>) -> Self {
        Self {
            outcomes,
            requests: Mutex::new(Vec::new()),
        }
    }

    /// What: Count metadata fetches for one requested package.
    ///
    /// Inputs:
    /// - `package`: Requested package or virtual dependency name.
    ///
    /// Output:
    /// - Returns the number of requests recorded for the name.
    ///
    /// Details:
    /// - A poisoned test mutex is treated as an empty log because the test must not panic.
    fn request_count(&self, package: &str) -> usize {
        self.requests.lock().map_or(0, |requests| {
            requests
                .iter()
                .filter(|name| name.as_str() == package)
                .count()
        })
    }
}

impl DependencyMetadataProvider for FixtureMetadataProvider {
    fn fetch_metadata(
        &self,
        requested_names: &[String],
        _timeout: Duration,
    ) -> Vec<DependencyMetadataResponse> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.extend(requested_names.iter().cloned());
        }

        requested_names
            .iter()
            .map(|requested_name| match self.outcomes.get(requested_name) {
                Some(FixtureMetadataOutcome::Found {
                    package_name,
                    source,
                    srcinfo,
                }) => DependencyMetadataResponse::Found(DependencyMetadata::new(
                    requested_name,
                    package_name,
                    source.clone(),
                    srcinfo,
                )),
                Some(FixtureMetadataOutcome::Missing { reason }) => {
                    DependencyMetadataResponse::Missing {
                        requested_name: requested_name.clone(),
                        reason: reason.clone(),
                    }
                }
                Some(FixtureMetadataOutcome::Failure { message }) => {
                    DependencyMetadataResponse::Failure {
                        requested_name: requested_name.clone(),
                        message: message.clone(),
                    }
                }
                None => DependencyMetadataResponse::Missing {
                    requested_name: requested_name.clone(),
                    reason: "fixture has no metadata".to_string(),
                },
            })
            .collect()
    }
}

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

/// Test graph resolution with direct, transitive, duplicate, and provided dependencies.
#[test]
fn test_graph_resolver_fixture_transitive_provider_and_cache() -> Result<()> {
    let outcomes = BTreeMap::from([
        (
            "aur-root".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "aur-root".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = aur-root\npkgname = aur-root\npkgver = 1\npkgrel = 1\ndepends = shared>=1:2.0-3\ndepends = virtual-lib>=1\n".to_string(),
            },
        ),
        (
            "other-root".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "other-root".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = other-root\npkgname = other-root\npkgver = 1\npkgrel = 1\ndepends = shared<=1:3.0-1\n".to_string(),
            },
        ),
        (
            "shared".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "shared".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = shared\npkgname = shared\npkgver = 1:2.5\npkgrel = 4\ndepends = leaf\n".to_string(),
            },
        ),
        (
            "virtual-lib".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "real-provider".to_string(),
                source: DependencySource::Official {
                    repo: "extra".to_string(),
                },
                srcinfo: "pkgbase = real-provider\npkgname = real-provider\npkgver = 1\npkgrel = 1\nprovides = virtual-lib=1\n".to_string(),
            },
        ),
        (
            "leaf".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "leaf".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = leaf\npkgname = leaf\npkgver = 1\npkgrel = 1\n".to_string(),
            },
        ),
    ]);
    let provider = FixtureMetadataProvider::new(outcomes);
    let resolver = DependencyResolver::new();
    let roots = vec![
        PackageRef::aur("other-root", "1"),
        PackageRef::aur("aur-root", "1"),
    ];

    let graph = resolver.resolve_graph(
        &roots,
        &provider,
        DependencyGraphConfig {
            max_depth: 4,
            max_nodes: 16,
            metadata_timeout: Duration::from_secs(1),
            max_concurrency: 2,
        },
    )?;

    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|node| node.name.as_str())
            .collect::<Vec<_>>(),
        vec!["aur-root", "leaf", "other-root", "real-provider", "shared"]
    );
    assert_eq!(provider.request_count("shared"), 1);
    assert!(graph.nodes.iter().any(|node| {
        node.name == "real-provider"
            && node.provenance.provider.as_deref() == Some("real-provider")
            && node.provenance.requested_name == "virtual-lib"
    }));
    assert!(graph.diagnostics.is_empty());
    assert_eq!(graph.render_tree(), graph.render_tree());
    assert!(graph.render_tree().contains("real-provider"));
    Ok(())
}

/// Test structured fixture diagnostics for malformed, missing, failed, cyclic, and incompatible metadata.
#[test]
fn test_graph_resolver_fixture_diagnostics_and_bounds() -> Result<()> {
    let outcomes = BTreeMap::from([
        (
            "cycle-root".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "cycle-root".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = cycle-root\npkgname = cycle-root\npkgver = 1\npkgrel = 1\ndepends = cycle-child\ndepends = shared>=1:3\ndepends = missing\ndepends = malformed\ndepends = network-failure\n".to_string(),
            },
        ),
        (
            "cycle-child".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "cycle-child".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = cycle-child\npkgname = cycle-child\npkgver = 1\npkgrel = 1\ndepends = cycle-root\ndepends = shared<1:3\n".to_string(),
            },
        ),
        (
            "shared".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "shared".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = shared\npkgname = shared\npkgver = 1:3\npkgrel = 1\n".to_string(),
            },
        ),
        (
            "missing".to_string(),
            FixtureMetadataOutcome::Missing {
                reason: "not indexed".to_string(),
            },
        ),
        (
            "malformed".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "malformed".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = malformed\npkgver = 1\n".to_string(),
            },
        ),
        (
            "network-failure".to_string(),
            FixtureMetadataOutcome::Failure {
                message: "network helper failed".to_string(),
            },
        ),
    ]);
    let provider = FixtureMetadataProvider::new(outcomes);
    let graph = DependencyResolver::new().resolve_graph(
        &[PackageRef::aur("cycle-root", "1")],
        &provider,
        DependencyGraphConfig {
            max_depth: 4,
            max_nodes: 16,
            metadata_timeout: Duration::from_secs(1),
            max_concurrency: 1,
        },
    )?;

    let kinds = graph
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&DependencyGraphDiagnosticKind::Cycle));
    assert!(kinds.contains(&DependencyGraphDiagnosticKind::MissingMetadata));
    assert!(kinds.contains(&DependencyGraphDiagnosticKind::MalformedSrcinfo));
    assert!(kinds.contains(&DependencyGraphDiagnosticKind::MetadataFailure));
    assert!(kinds.contains(&DependencyGraphDiagnosticKind::IncompatibleConstraints));
    Ok(())
}

/// Test split-package selection, conflicts, and graph safety bounds through public fixtures.
#[test]
fn test_graph_resolver_fixture_split_conflicts_and_limits() -> Result<()> {
    let outcomes = BTreeMap::from([
        (
            "root".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "root".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = root\npkgname = root\npkgver = 1\npkgrel = 1\nconflicts = conflict-target\ndepends = split-output\ndepends = conflict-target\n".to_string(),
            },
        ),
        (
            "split-output".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "split-output".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = split-base\ndepends = base-shared\npkgname = split-other\ndepends = other-only\npkgname = split-output\ndepends = output-only\npkgver = 1\npkgrel = 1\n".to_string(),
            },
        ),
        (
            "conflict-target".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "conflict-target".to_string(),
                source: DependencySource::Local,
                srcinfo: "pkgbase = conflict-target\npkgname = conflict-target\npkgver = 1\npkgrel = 1\n".to_string(),
            },
        ),
        (
            "base-shared".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "base-shared".to_string(),
                source: DependencySource::Official {
                    repo: "core".to_string(),
                },
                srcinfo: "pkgbase = base-shared\npkgname = base-shared\npkgver = 1\npkgrel = 1\n".to_string(),
            },
        ),
        (
            "output-only".to_string(),
            FixtureMetadataOutcome::Found {
                package_name: "output-only".to_string(),
                source: DependencySource::Aur,
                srcinfo: "pkgbase = output-only\npkgname = output-only\npkgver = 1\npkgrel = 1\n".to_string(),
            },
        ),
    ]);
    let provider = FixtureMetadataProvider::new(outcomes);
    let resolver = DependencyResolver::new();
    let roots = [PackageRef::aur("root", "1")];

    let graph = resolver.resolve_graph(&roots, &provider, DependencyGraphConfig::default())?;
    assert!(graph.nodes.iter().any(|node| {
        node.name == "split-output"
            && node.pkgbase.as_deref() == Some("split-base")
            && node.status == arch_toolkit::deps::DependencyGraphNodeStatus::Resolved
    }));
    assert!(!graph.nodes.iter().any(|node| node.name == "other-only"));
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DependencyGraphDiagnosticKind::Conflict)
    );
    assert!(graph.nodes.iter().any(|node| {
        node.name == "root"
            && node.status == arch_toolkit::deps::DependencyGraphNodeStatus::Conflicting
    }));
    assert!(graph.nodes.iter().any(|node| {
        node.name == "conflict-target"
            && node.status == arch_toolkit::deps::DependencyGraphNodeStatus::Conflicting
    }));

    let depth_limited = resolver.resolve_graph(
        &roots,
        &provider,
        DependencyGraphConfig {
            max_depth: 0,
            max_nodes: 8,
            metadata_timeout: Duration::from_secs(1),
            max_concurrency: 1,
        },
    )?;
    assert!(
        depth_limited
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DependencyGraphDiagnosticKind::DepthLimit)
    );

    let node_limited = resolver.resolve_graph(
        &roots,
        &provider,
        DependencyGraphConfig {
            max_depth: 4,
            max_nodes: 1,
            metadata_timeout: Duration::from_secs(1),
            max_concurrency: 1,
        },
    )?;
    assert!(
        node_limited
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DependencyGraphDiagnosticKind::NodeLimit)
    );
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
