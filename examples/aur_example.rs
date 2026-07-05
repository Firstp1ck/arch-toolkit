//! Comprehensive AUR (Arch User Repository) usage example for arch-toolkit.
//!
//! This example demonstrates all AUR-related operations:
//! - Creating an `ArchClient` with default and custom settings
//! - Using the builder pattern for configuration
//! - Configuring retry policies with exponential backoff
//! - AUR package search, info, comments, and PKGBUILD fetching
//! - Error handling and edge cases
//! - Working with AUR package data structures
//! - Formatting and displaying results
//!
//! Note: This example focuses on AUR operations. For other arch-toolkit features
//! (dependency resolution, package index queries, installation commands, etc.),
//! see other example files.

use arch_toolkit::ArchClient;
use arch_toolkit::client::RetryPolicy;
use arch_toolkit::error::Result;
use std::time::Duration;

#[tokio::main]
#[allow(clippy::too_many_lines)] // Example file - comprehensive demonstration
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: AUR (Arch User Repository) Example         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Default Client Configuration
    // ========================================================================
    println!("┌─ Example 1: Default Client Configuration ────────────────────┐");
    println!("│ Creating client with sensible defaults (30s timeout)         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let client = ArchClient::new()?;
    println!("✓ Client created successfully\n");

    // ========================================================================
    // Example 2: Search with Detailed Results
    // ========================================================================
    println!("┌─ Example 2: AUR Package Search ─────────────────────────────┐");
    println!("│ Searching for packages matching 'yay'                        │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let packages = client.aur().search("yay").await?;
    println!("Found {} packages matching 'yay'\n", packages.len());

    // Display first 5 results with all available fields
    for (i, pkg) in packages.iter().take(5).enumerate() {
        println!("  {}. {}", i + 1, pkg.name);
        println!("     Version: {}", pkg.version);
        println!("     Description: {}", pkg.description);
        if let Some(pop) = pkg.popularity {
            println!("     Popularity: {pop:.2}");
        }
        if let Some(maintainer) = &pkg.maintainer {
            println!("     Maintainer: {maintainer}");
        } else {
            println!("     Status: ⚠️  ORPHANED (no maintainer)");
        }
        if pkg.out_of_date.is_some() {
            println!("     Status: ⚠️  OUT OF DATE");
        }
        println!();
    }

    // ========================================================================
    // Example 3: Custom Client Configuration
    // ========================================================================
    println!("┌─ Example 3: Custom Client Configuration ───────────────────┐");
    println!("│ Using builder pattern for custom timeout and user agent     │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let custom_client = ArchClient::builder()
        .timeout(Duration::from_mins(1))
        .user_agent("my-arch-tool/1.0")
        .build()?;
    println!("✓ Custom client created with 60s timeout\n");

    // ========================================================================
    // Example 4: Retry Policy Configuration
    // ========================================================================
    println!("┌─ Example 4: Retry Policy Configuration ───────────────────────┐");
    println!("│ Configuring retry policies for resilient network operations   │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // Example 4a: Default retry policy (already enabled)
    println!("4a. Default retry policy:");
    println!("   The default client has retries enabled with:");
    println!("   - Max retries: 3");
    println!("   - Initial delay: 1000ms");
    println!("   - Max delay: 30000ms");
    println!("   - All operations retryable by default");
    let _default_client = ArchClient::new()?;
    println!("   ✓ Default client created with retry policy enabled\n");

    // Example 4b: Custom retry policy with more retries
    println!("4b. Custom retry policy (5 retries):");
    let _retry_client = ArchClient::builder().max_retries(5).build()?;
    println!("   ✓ Client created with 5 max retries (instead of default 3)");
    println!("   This provides more resilience for transient network failures\n");

    // Example 4c: Full retry policy configuration
    println!("4c. Full retry policy configuration:");
    let full_retry_policy = RetryPolicy {
        max_retries: 5,
        initial_delay_ms: 2000,
        max_delay_ms: 60_000,
        jitter_max_ms: 1000,
        enabled: true,
        retry_search: true,
        retry_info: true,
        retry_comments: true,
        retry_pkgbuild: false, // Disable retries for PKGBUILD
    };
    let _full_retry_client = ArchClient::builder()
        .retry_policy(full_retry_policy)
        .build()?;
    println!("   Configured with:");
    println!("   - Max retries: 5");
    println!("   - Initial delay: 2000ms");
    println!("   - Max delay: 60000ms");
    println!("   - PKGBUILD retries: disabled");
    println!("   ✓ Client created with full retry policy configuration\n");

    // Example 4d: Per-operation retry configuration
    println!("4d. Per-operation retry configuration:");
    let _selective_client = ArchClient::builder()
        .retry_operation("pkgbuild", false) // Disable retries for PKGBUILD only
        .retry_operation("comments", false) // Disable retries for comments
        .build()?;
    println!("   Configured to disable retries for:");
    println!("   - PKGBUILD operations");
    println!("   - Comments operations");
    println!("   Search and info operations will still retry");
    println!("   ✓ Client created with selective retry configuration\n");

    // Example 4e: Disable retries globally
    println!("4e. Disable retries globally:");
    let _no_retry_client = ArchClient::builder().retry_enabled(false).build()?;
    println!("   ✓ Client created with retries disabled");
    println!("   All operations will fail immediately on network errors\n");

    // ========================================================================
    // Example 5: Fetch Detailed Package Information
    // ========================================================================
    println!("┌─ Example 4: Detailed Package Information ───────────────────┐");
    println!("│ Fetching full details for multiple packages                  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let details = custom_client.aur().info(&["yay", "paru"]).await?;
    println!("Fetched details for {} packages:\n", details.len());

    for pkg in &details {
        println!("  📦 {}", pkg.name);
        println!("     Version: {}", pkg.version);
        println!("     Description: {}", pkg.description);

        if !pkg.url.is_empty() {
            println!("     URL: {}", pkg.url);
        }

        if !pkg.licenses.is_empty() {
            println!("     Licenses: {}", pkg.licenses.join(", "));
        }

        if let Some(maintainer) = &pkg.maintainer {
            println!("     Maintainer: {maintainer}");
        } else {
            println!("     Status: ⚠️  ORPHANED");
        }

        if let Some(pop) = pkg.popularity {
            println!("     Popularity: {pop:.2}");
        }

        if let Some(votes) = pkg.num_votes {
            println!("     Votes: {votes}");
        }

        if !pkg.depends.is_empty() {
            let deps_preview: Vec<&str> = pkg.depends.iter().map(String::as_str).take(5).collect();
            let deps_count = pkg.depends.len();
            println!(
                "     Dependencies ({deps_count}): {}",
                deps_preview.join(", ")
            );
            if deps_count > 5 {
                let remaining = deps_count - 5;
                println!("       ... and {remaining} more");
            }
        }

        if !pkg.opt_depends.is_empty() {
            let opt_count = pkg.opt_depends.len();
            println!("     Optional Dependencies ({opt_count}):");
            for opt_dep in pkg.opt_depends.iter().take(3) {
                println!("       - {opt_dep}");
            }
            if opt_count > 3 {
                let remaining = opt_count - 3;
                println!("       ... and {remaining} more");
            }
        }

        if let Some(submitted) = pkg.first_submitted {
            let date = chrono::DateTime::from_timestamp(submitted, 0).map_or_else(
                || "unknown".to_string(),
                |dt| dt.format("%Y-%m-%d").to_string(),
            );
            println!("     First Submitted: {date}");
        }

        if let Some(modified) = pkg.last_modified {
            let date = chrono::DateTime::from_timestamp(modified, 0).map_or_else(
                || "unknown".to_string(),
                |dt| dt.format("%Y-%m-%d").to_string(),
            );
            println!("     Last Modified: {date}");
        }

        if pkg.out_of_date.is_some() {
            println!("     Status: ⚠️  OUT OF DATE");
        }

        println!();
    }

    // ========================================================================
    // Example 6: Error Handling - Invalid Package
    // ========================================================================
    println!("┌─ Example 5: Error Handling ─────────────────────────────────┐");
    println!("│ Demonstrating graceful handling of non-existent packages    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match custom_client
        .aur()
        .info(&["this-package-definitely-does-not-exist-12345"])
        .await
    {
        Ok(packages) => {
            if packages.is_empty() {
                println!("✓ Package not found (empty result, not an error)\n");
            } else {
                println!("Unexpected: found packages\n");
            }
        }
        Err(e) => {
            println!("✗ Error: {e}\n");
        }
    }

    // ========================================================================
    // Example 7: Empty Search Query
    // ========================================================================
    println!("┌─ Example 6: Edge Case - Empty Search ───────────────────────┐");
    println!("│ Testing behavior with empty search query                     │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let empty_results = client.aur().search("").await?;
    println!(
        "Empty query returned {} results (expected: 0)\n",
        empty_results.len()
    );

    // ========================================================================
    // Example 8: Fetch Package Comments
    // ========================================================================
    println!("┌─ Example 7: Package Comments ───────────────────────────────┐");
    println!("│ Fetching and displaying comments from AUR package page       │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let comments = client.aur().comments("yay").await?;
    println!("Found {} comments for 'yay'\n", comments.len());

    // Display first 5 comments with full details
    for (i, comment) in comments.iter().take(5).enumerate() {
        println!("  💬 Comment #{}", i + 1);
        println!("     Author: {}", comment.author);
        println!("     Date: {}", comment.date);
        if comment.pinned {
            println!("     Status: 📌 PINNED");
        }

        // Show first 3 lines of content
        let content_lines: Vec<&str> = comment.content.lines().take(3).collect();
        if !content_lines.is_empty() {
            println!("     Content:");
            for line in content_lines {
                if !line.trim().is_empty() {
                    println!("       {}", line.trim());
                }
            }
            if comment.content.lines().count() > 3 {
                println!(
                    "       ... ({} more lines)",
                    comment.content.lines().count() - 3
                );
            }
        }
        println!();
    }

    // ========================================================================
    // Example 9: Fetch PKGBUILD
    // ========================================================================
    println!("┌─ Example 8: PKGBUILD Content ────────────────────────────────┐");
    println!("│ Fetching and displaying PKGBUILD source                      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let pkgbuild = client.aur().pkgbuild("yay").await?;
    let total_lines = pkgbuild.lines().count();
    println!("PKGBUILD fetched successfully ({total_lines} lines total)\n");

    // Display first 15 lines
    let lines: Vec<&str> = pkgbuild.lines().take(15).collect();
    println!("First 15 lines:");
    for (i, line) in lines.iter().enumerate() {
        println!("  {:2}: {}", i + 1, line);
    }
    if total_lines > 15 {
        println!("  ... ({} more lines)", total_lines - 15);
    }
    println!();

    // ========================================================================
    // Example 10: Working with Search Results
    // ========================================================================
    println!("┌─ Example 9: Analyzing Search Results ───────────────────────┐");
    println!("│ Demonstrating data analysis on search results                 │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let search_results = client.aur().search("aur").await?;
    println!(
        "Search for 'aur' returned {} packages\n",
        search_results.len()
    );

    // Analyze results
    let orphaned_count = search_results.iter().filter(|p| p.orphaned).count();
    let out_of_date_count = search_results
        .iter()
        .filter(|p| p.out_of_date.is_some())
        .count();
    let with_maintainer = search_results
        .iter()
        .filter(|p| p.maintainer.is_some())
        .count();

    let total = search_results.len();
    let orphaned_pct = if total == 0 {
        0.0
    } else {
        (f64::from(u32::try_from(orphaned_count).unwrap_or(0))
            / f64::from(u32::try_from(total).unwrap_or(1)))
            * 100.0
    };
    let out_of_date_pct = if total == 0 {
        0.0
    } else {
        (f64::from(u32::try_from(out_of_date_count).unwrap_or(0))
            / f64::from(u32::try_from(total).unwrap_or(1)))
            * 100.0
    };

    println!("  Statistics:");
    println!("    Total packages: {total}");
    println!("    With maintainer: {with_maintainer}");
    println!("    Orphaned: {orphaned_count} ({orphaned_pct:.1}%)");
    println!("    Out of date: {out_of_date_count} ({out_of_date_pct:.1}%)");

    // Find most popular package
    if let Some(most_popular) = search_results
        .iter()
        .filter_map(|p| p.popularity.map(|pop| (p, pop)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    {
        println!(
            "    Most popular: {} (popularity: {:.2})",
            most_popular.0.name, most_popular.1
        );
    }
    println!();

    // ========================================================================
    // Example 11: Batch Operations
    // ========================================================================
    println!("┌─ Example 10: Batch Package Information ─────────────────────┐");
    println!("│ Fetching details for multiple packages in one request        │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let batch_packages = ["yay", "paru", "pacman"];
    let batch_details = client.aur().info(&batch_packages).await?;
    println!(
        "Requested {} packages, received {} results\n",
        batch_packages.len(),
        batch_details.len()
    );

    for pkg in &batch_details {
        println!("  ✓ {}", pkg.name);
    }

    // Note which packages weren't found
    let found_names: Vec<&str> = batch_details.iter().map(|p| p.name.as_str()).collect();
    let missing: Vec<&str> = batch_packages
        .iter()
        .filter(|name| !found_names.contains(name))
        .copied()
        .collect();

    if !missing.is_empty() {
        println!("\n  Packages not found in AUR:");
        for name in missing {
            println!("    ✗ {name} (likely in official repos, not AUR)");
        }
    }
    println!();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nAll operations completed successfully!");
    println!("This example demonstrated:");
    println!("  • Client creation (default and custom)");
    println!("  • Retry policy configuration (default, custom, per-operation)");
    println!("  • Package search with detailed results");
    println!("  • Fetching full package information");
    println!("  • Error handling and edge cases");
    println!("  • Comment fetching and parsing");
    println!("  • PKGBUILD retrieval");
    println!("  • Data analysis on search results");
    println!("  • Batch operations");

    Ok(())
}
