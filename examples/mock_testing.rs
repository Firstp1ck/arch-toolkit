//! Example demonstrating how to use `MockAurApi` for testing.
//!
//! This example shows how to:
//! - Create a mock AUR API with predefined results
//! - Use the mock in place of the real `Aur` implementation
//! - Test different scenarios including success and error cases
//! - Use the trait-based design to write testable code
//!
//! This is particularly useful for:
//! - Unit testing code that depends on AUR operations
//! - Testing error handling without hitting real APIs
//! - Fast, deterministic tests that don't require network access

use arch_toolkit::error::{ArchToolkitError, Result};
use arch_toolkit::types::{AurComment, AurPackage, AurPackageDetails};
use arch_toolkit::{AurApi, MockAurApi};

/// What: Example function that uses `AurApi` trait.
///
/// Inputs:
/// - `api`: Any type implementing `AurApi` (real or mock)
/// - `query`: Search query
///
/// Output:
/// - `Result<usize>` with count of packages found
///
/// Details:
/// - This function works with both real and mock implementations
/// - Demonstrates how trait-based design enables testability
async fn count_packages(api: &dyn AurApi, query: &str) -> Result<usize> {
    let packages = api.search(query).await?;
    Ok(packages.len())
}

/// What: Example function that processes package information.
///
/// Inputs:
/// - `api`: Any type implementing `AurApi`
/// - `package_names`: Names of packages to process
///
/// Output:
/// - `Result<Vec<String>>` with package descriptions
///
/// Details:
/// - Fetches package info and extracts descriptions
/// - Works with any `AurApi` implementation
async fn get_package_descriptions(api: &dyn AurApi, package_names: &[&str]) -> Result<Vec<String>> {
    let details = api.info(package_names).await?;
    Ok(details.into_iter().map(|p| p.description).collect())
}

#[tokio::main]
#[allow(clippy::too_many_lines, clippy::uninlined_format_args)] // Example file - comprehensive demonstration
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Mock Testing Example                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic Mock Setup
    // ========================================================================
    println!("┌─ Example 1: Basic Mock Setup ───────────────────────────────┐");
    println!("│ Creating a mock with predefined search results               │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let mock = MockAurApi::new()
        .with_search_result(
            "yay",
            Ok(vec![
                AurPackage {
                    name: "yay".to_string(),
                    version: "12.0.0".to_string(),
                    description: "Yet Another Yaourt".to_string(),
                    popularity: Some(100.0),
                    out_of_date: None,
                    orphaned: false,
                    maintainer: Some("Jguer".to_string()),
                },
                AurPackage {
                    name: "yay-bin".to_string(),
                    version: "12.0.0".to_string(),
                    description: "Yet Another Yaourt (binary)".to_string(),
                    popularity: Some(50.0),
                    out_of_date: None,
                    orphaned: false,
                    maintainer: Some("Jguer".to_string()),
                },
            ]),
        )
        .with_search_result(
            "paru",
            Ok(vec![AurPackage {
                name: "paru".to_string(),
                version: "2.0.0".to_string(),
                description: "Feature packed AUR helper".to_string(),
                popularity: Some(95.0),
                out_of_date: None,
                orphaned: false,
                maintainer: Some("Morganamilo".to_string()),
            }]),
        );

    // Use the mock as a trait object
    let count = count_packages(&mock, "yay").await?;
    println!("✓ Found {count} packages matching 'yay'\n");

    // ========================================================================
    // Example 2: Testing Error Scenarios
    // ========================================================================
    println!("┌─ Example 2: Testing Error Scenarios ────────────────────────┐");
    println!("│ Configuring mock to return errors for testing error handling  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let error_mock = MockAurApi::new().with_search_result(
        "error",
        Err(ArchToolkitError::Parse("Network error for testing".to_string())),
    );

    match count_packages(&error_mock, "error").await {
        Ok(_) => println!("✗ Unexpected success"),
        Err(e) => println!("✓ Correctly handled error: {e}\n"),
    }

    // ========================================================================
    // Example 3: Default Results
    // ========================================================================
    println!("┌─ Example 3: Default Results ─────────────────────────────────┐");
    println!("│ Using default results for unmatched queries                   │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let default_mock = MockAurApi::new().with_default_search_result(Ok(vec![AurPackage {
        name: "default-package".to_string(),
        version: "1.0.0".to_string(),
        description: "Default result".to_string(),
        popularity: None,
        out_of_date: None,
        orphaned: false,
        maintainer: None,
    }]));

    let count = count_packages(&default_mock, "any-query").await?;
    println!("✓ Default result returned: {count} packages\n");

    // ========================================================================
    // Example 4: Info Operation Mocking
    // ========================================================================
    println!("┌─ Example 4: Info Operation Mocking ─────────────────────────┐");
    println!("│ Mocking package info responses                                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let info_mock = MockAurApi::new().with_info_result(
        &["yay"],
        Ok(vec![AurPackageDetails {
            name: "yay".to_string(),
            version: "12.0.0".to_string(),
            description: "Yet Another Yaourt - A simple AUR helper written in Go".to_string(),
            url: "https://github.com/Jguer/yay".to_string(),
            licenses: vec!["GPL".to_string(), "MIT".to_string()],
            groups: vec![],
            provides: vec![],
            depends: vec!["git".to_string(), "go".to_string()],
            make_depends: vec![],
            opt_depends: vec![],
            conflicts: vec![],
            replaces: vec![],
            maintainer: Some("Jguer".to_string()),
            first_submitted: Some(1_500_000_000),
            last_modified: Some(1_700_000_000),
            popularity: Some(100.0),
            num_votes: Some(5000),
            out_of_date: None,
            orphaned: false,
        }]),
    );

    let descriptions = get_package_descriptions(&info_mock, &["yay"]).await?;
    println!("✓ Got description: {}\n", descriptions[0]);

    // ========================================================================
    // Example 5: Comments Mocking
    // ========================================================================
    println!("┌─ Example 5: Comments Mocking ────────────────────────────────┐");
    println!("│ Mocking package comments                                      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let comments_mock = MockAurApi::new().with_comments_result(
        "yay",
        Ok(vec![
            AurComment {
                id: Some("1".to_string()),
                author: "user1".to_string(),
                date: "2024-01-15".to_string(),
                date_timestamp: Some(1_705_132_800),
                date_url: None,
                content: "Great package! Works perfectly.".to_string(),
                pinned: true,
            },
            AurComment {
                id: Some("2".to_string()),
                author: "user2".to_string(),
                date: "2024-01-20".to_string(),
                date_timestamp: Some(1_705_564_800),
                date_url: None,
                content: "Thanks for maintaining this!".to_string(),
                pinned: false,
            },
        ]),
    );

    let comments = comments_mock.comments("yay").await?;
    println!("✓ Found {} comments", comments.len());
    for comment in &comments {
        println!("  - {}: {}", comment.author, comment.content);
    }
    println!();

    // ========================================================================
    // Example 6: PKGBUILD Mocking
    // ========================================================================
    println!("┌─ Example 6: PKGBUILD Mocking ───────────────────────────────┐");
    println!("│ Mocking PKGBUILD content                                      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let pkgbuild_mock = MockAurApi::new().with_pkgbuild_result(
        "yay",
        Ok(r#"pkgname=yay
pkgver=12.0.0
pkgrel=1
pkgdesc="Yet Another Yaourt - A simple AUR helper written in Go"
arch=('x86_64')
url="https://github.com/Jguer/yay"
license=('GPL' 'MIT')
depends=('git' 'go')
makedepends=()
"#
        .to_string()),
    );

    let pkgbuild = pkgbuild_mock.pkgbuild("yay").await?;
    println!("✓ PKGBUILD content (first 100 chars):");
    println!("  {}\n", &pkgbuild.chars().take(100).collect::<String>());

    // ========================================================================
    // Example 7: Complex Mock Configuration
    // ========================================================================
    println!("┌─ Example 7: Complex Mock Configuration ─────────────────────┐");
    println!("│ Setting up multiple operations with different results         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let complex_mock = MockAurApi::new()
        .with_search_result(
            "yay",
            Ok(vec![AurPackage {
                name: "yay".to_string(),
                version: "12.0.0".to_string(),
                description: "AUR helper".to_string(),
                popularity: Some(100.0),
                out_of_date: None,
                orphaned: false,
                maintainer: Some("Jguer".to_string()),
            }]),
        )
        .with_info_result(
            &["yay"],
            Ok(vec![AurPackageDetails {
                name: "yay".to_string(),
                version: "12.0.0".to_string(),
                description: "AUR helper".to_string(),
                url: String::new(),
                licenses: vec![],
                groups: vec![],
                provides: vec![],
                depends: vec![],
                make_depends: vec![],
                opt_depends: vec![],
                conflicts: vec![],
                replaces: vec![],
                maintainer: Some("Jguer".to_string()),
                first_submitted: None,
                last_modified: None,
                popularity: Some(100.0),
                num_votes: None,
                out_of_date: None,
                orphaned: false,
            }]),
        )
        .with_comments_result("yay", Ok(vec![]))
        .with_pkgbuild_result("yay", Ok("pkgname=yay".to_string()));

    // Test all operations
    let _packages = complex_mock.search("yay").await?;
    let _info = complex_mock.info(&["yay"]).await?;
    let _comments = complex_mock.comments("yay").await?;
    let _pkgbuild = complex_mock.pkgbuild("yay").await?;

    println!("✓ All operations completed successfully\n");

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nThis example demonstrated:");
    println!("  • Creating mocks with predefined results");
    println!("  • Testing error scenarios");
    println!("  • Using default results for unmatched queries");
    println!("  • Mocking all AUR operations (search, info, comments, pkgbuild)");
    println!("  • Writing testable code using the AurApi trait");
    println!("\nKey benefits:");
    println!("  • Fast tests (no network requests)");
    println!("  • Deterministic behavior");
    println!("  • Easy error scenario testing");
    println!("  • No external dependencies for tests");

    Ok(())
}
