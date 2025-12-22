//! Comprehensive example demonstrating Rich Error Context in arch-toolkit.
//!
//! This example showcases the enhanced error handling system that preserves
//! operation context (query strings, package names) in error messages, making
//! debugging and user-facing error messages much more actionable.
//!
//! Key features demonstrated:
//! - Operation-specific error variants with context
//! - Extracting context from errors
//! - Error source chain traversal
//! - Practical error handling patterns
//! - Comparison of error messages with and without context
//!
//! Run this example with:
//! ```bash
//! cargo run --example rich_error_context
//! ```

use arch_toolkit::ArchClient;
use arch_toolkit::error::{ArchToolkitError, Result};
use std::error::Error;

/// Helper function to create a mock reqwest::Error for examples.
fn create_mock_error() -> reqwest::Error {
    let cert_result = reqwest::Certificate::from_pem(b"invalid cert");
    match cert_result {
        Ok(cert) => reqwest::Client::builder()
            .add_root_certificate(cert)
            .build()
            .expect_err("Should fail to build client with invalid cert"),
        Err(e) => e,
    }
}

#[tokio::main]
#[allow(
    clippy::too_many_lines, // Example file - comprehensive demonstration
    clippy::unused_variables, // Example file - variables used for demonstration
    clippy::doc_markdown, // Example file - documentation style
    clippy::items_after_statements, // Example file - helper functions
    clippy::unnecessary_wraps, // Example file - return values for consistency
)]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║     arch-toolkit: Rich Error Context Example                   ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let client = ArchClient::new()?;

    // ========================================================================
    // Example 1: Understanding Error Context
    // ========================================================================
    println!("┌─ Example 1: Understanding Error Context ─────────────────────┐");
    println!("│ Rich errors preserve operation context in error messages      │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    demonstrate_error_context(&client).await?;

    // ========================================================================
    // Example 2: Extracting Context from Errors
    // ========================================================================
    println!("\n┌─ Example 2: Extracting Context from Errors ──────────────────┐");
    println!("│ Extract query/package names from error variants              │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    let _ = demonstrate_context_extraction();

    // ========================================================================
    // Example 3: Error Source Chain
    // ========================================================================
    println!("\n┌─ Example 3: Error Source Chain ─────────────────────────────┐");
    println!("│ Traverse the error source chain for detailed diagnostics     │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    let _ = demonstrate_error_source_chain();

    // ========================================================================
    // Example 4: Practical Error Handling Patterns
    // ========================================================================
    println!("\n┌─ Example 4: Practical Error Handling Patterns ──────────────┐");
    println!("│ Real-world patterns for handling contextual errors           │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    demonstrate_practical_patterns(&client).await?;

    // ========================================================================
    // Example 5: Error Message Comparison
    // ========================================================================
    println!("\n┌─ Example 5: Error Message Comparison ────────────────────────┐");
    println!("│ See the difference: with vs without context                  │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    let _ = demonstrate_error_comparison();

    println!("\n✓ All examples completed successfully!");
    Ok(())
}

/// What: Demonstrate how error context is preserved in error messages.
///
/// Inputs:
/// - `client`: ArchClient instance
///
/// Output:
/// - `Result<()>` indicating success or failure
///
/// Details:
/// - Shows how different operations preserve their context in errors
/// - Demonstrates error message formatting with context
async fn demonstrate_error_context(_client: &ArchClient) -> Result<()> {
    println!("When an AUR operation fails, the error message includes:");
    println!("  • The operation type (search, info, comments, pkgbuild)");
    println!("  • The specific query or package name that failed");
    println!("  • The underlying network error\n");

    // Simulate different error scenarios
    println!("Error Variants with Context:");
    println!("─────────────────────────────\n");

    // SearchFailed example
    println!("1. SearchFailed Error:");
    println!("   Operation: AUR search");
    println!("   Context: Query string is preserved");
    println!("   Example: \"AUR search failed for query 'yay': connection timed out\"\n");

    // InfoFailed example
    println!("2. InfoFailed Error:");
    println!("   Operation: AUR info fetch");
    println!("   Context: Package names are preserved");
    println!(
        "   Example: \"AUR info fetch failed for packages [yay, paru]: connection refused\"\n"
    );

    // CommentsFailed example
    println!("3. CommentsFailed Error:");
    println!("   Operation: AUR comments fetch");
    println!("   Context: Package name is preserved");
    println!("   Example: \"AUR comments fetch failed for package 'yay': network unreachable\"\n");

    // PkgbuildFailed example
    println!("4. PkgbuildFailed Error:");
    println!("   Operation: PKGBUILD fetch");
    println!("   Context: Package name is preserved");
    println!("   Example: \"PKGBUILD fetch failed for package 'yay': request timeout\"\n");

    // PackageNotFound example
    println!("5. PackageNotFound Error:");
    println!("   Operation: Package lookup");
    println!("   Context: Package name is preserved");
    println!("   Example: \"Package 'nonexistent-package' not found\"\n");

    Ok(())
}

/// What: Demonstrate how to extract context from error variants.
///
/// Inputs: None
///
/// Output:
/// - `Result<()>` indicating success or failure
///
/// Details:
/// - Shows pattern matching on error variants
/// - Extracts query/package names from errors
/// - Demonstrates helper methods for context extraction
fn demonstrate_context_extraction() -> Result<()> {
    println!("Extracting Context from Errors:");
    println!("────────────────────────────────\n");

    // Example 1: Extract query from SearchFailed
    println!("1. Extracting Query from SearchFailed:");
    let search_error = ArchToolkitError::search_failed("yay", create_mock_error());
    match &search_error {
        ArchToolkitError::SearchFailed { query, .. } => {
            println!("   Query: {query}");
            println!("   Full error: {search_error}\n");
        }
        _ => unreachable!(),
    }

    // Example 2: Extract packages from InfoFailed
    println!("2. Extracting Packages from InfoFailed:");
    let info_error = ArchToolkitError::info_failed(&["yay", "paru"], create_mock_error());
    match &info_error {
        ArchToolkitError::InfoFailed { packages, .. } => {
            println!("   Packages: {packages}");
            println!("   Full error: {info_error}\n");
        }
        _ => unreachable!(),
    }

    // Example 3: Extract package from CommentsFailed
    println!("3. Extracting Package from CommentsFailed:");
    let comments_error = ArchToolkitError::comments_failed("yay", create_mock_error());
    match &comments_error {
        ArchToolkitError::CommentsFailed { package, .. } => {
            println!("   Package: {package}");
            println!("   Full error: {comments_error}\n");
        }
        _ => unreachable!(),
    }

    // Example 4: Extract package from PkgbuildFailed
    println!("4. Extracting Package from PkgbuildFailed:");
    let pkgbuild_error = ArchToolkitError::pkgbuild_failed("yay", create_mock_error());
    match &pkgbuild_error {
        ArchToolkitError::PkgbuildFailed { package, .. } => {
            println!("   Package: {package}");
            println!("   Full error: {pkgbuild_error}\n");
        }
        _ => unreachable!(),
    }

    // Example 5: Extract package from PackageNotFound
    println!("5. Extracting Package from PackageNotFound:");
    let not_found_error = ArchToolkitError::PackageNotFound {
        package: "nonexistent-package".to_string(),
    };
    match &not_found_error {
        ArchToolkitError::PackageNotFound { package } => {
            println!("   Package: {package}");
            println!("   Full error: {not_found_error}\n");
        }
        _ => unreachable!(),
    }

    // Example 6: Generic context extraction function
    println!("6. Generic Context Extraction Function:");
    fn extract_context(error: &ArchToolkitError) -> Option<String> {
        match error {
            ArchToolkitError::SearchFailed { query, .. } => Some(format!("query: {query}")),
            ArchToolkitError::InfoFailed { packages, .. } => Some(format!("packages: {packages}")),
            ArchToolkitError::CommentsFailed { package, .. } => Some(format!("package: {package}")),
            ArchToolkitError::PkgbuildFailed { package, .. } => Some(format!("package: {package}")),
            ArchToolkitError::PackageNotFound { package } => Some(format!("package: {package}")),
            _ => None,
        }
    }

    let contexts = vec![
        extract_context(&search_error),
        extract_context(&info_error),
        extract_context(&comments_error),
        extract_context(&pkgbuild_error),
        extract_context(&not_found_error),
    ];

    for (i, ctx) in contexts.iter().enumerate() {
        if let Some(context) = ctx {
            println!("   Error {} context: {context}", i + 1);
        }
    }
    println!();

    Ok(())
}

/// What: Demonstrate error source chain traversal.
///
/// Inputs: None
///
/// Output:
/// - `Result<()>` indicating success or failure
///
/// Details:
/// - Shows how to traverse the error source chain
/// - Demonstrates accessing underlying reqwest::Error
/// - Shows error chain formatting
fn demonstrate_error_source_chain() -> Result<()> {
    println!("Error Source Chain:");
    println!("───────────────────\n");

    // Create an error with a source chain
    let search_error = ArchToolkitError::search_failed("yay", create_mock_error());

    println!("1. Error Display (includes context):");
    println!("   {search_error}\n");

    println!("2. Error Source Chain:");
    let mut current: Option<&dyn Error> = Some(&search_error);
    let mut depth = 0;
    while let Some(err) = current {
        let indent = "   ".repeat(depth);
        println!("{indent}└─ {}", err);
        current = err.source();
        depth += 1;
        if depth > 5 {
            // Prevent infinite loops
            break;
        }
    }
    println!();

    println!("3. Accessing Underlying Error:");
    match &search_error {
        ArchToolkitError::SearchFailed { source, .. } => {
            println!(
                "   Source error type: {}",
                std::any::type_name_of_val(source)
            );
            println!("   Source error: {source}\n");
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// What: Demonstrate practical error handling patterns.
///
/// Inputs:
/// - `client`: ArchClient instance
///
/// Output:
/// - `Result<()>` indicating success or failure
///
/// Details:
/// - Shows real-world error handling patterns
/// - Demonstrates user-friendly error messages
/// - Shows retry logic with context preservation
async fn demonstrate_practical_patterns(client: &ArchClient) -> Result<()> {
    println!("Practical Error Handling Patterns:");
    println!("───────────────────────────────────\n");

    // Pattern 1: User-friendly error messages
    println!("1. User-Friendly Error Messages:");
    async fn search_with_friendly_error(
        client: &ArchClient,
        query: &str,
    ) -> Result<Vec<arch_toolkit::AurPackage>> {
        client.aur().search(query).await.map_err(|e| {
            match &e {
                ArchToolkitError::SearchFailed { query, .. } => {
                    eprintln!("❌ Search failed for query: '{query}'");
                    eprintln!("   Please check your internet connection and try again.");
                }
                _ => {
                    eprintln!("❌ Unexpected error: {e}");
                }
            }
            e
        })
    }

    // Try a search (will succeed or show friendly error)
    match search_with_friendly_error(client, "yay").await {
        Ok(packages) => {
            println!("   ✓ Search succeeded: found {} packages", packages.len());
        }
        Err(e) => {
            println!("   ✗ Search failed with context: {e}");
        }
    }
    println!();

    // Pattern 2: Context-aware retry logic
    println!("2. Context-Aware Retry Logic:");
    async fn search_with_retry(
        client: &ArchClient,
        query: &str,
        max_retries: u32,
    ) -> Result<Vec<arch_toolkit::AurPackage>> {
        let mut last_error = None;
        for attempt in 1..=max_retries {
            match client.aur().search(query).await {
                Ok(packages) => {
                    if attempt > 1 {
                        println!("   ✓ Search succeeded after {attempt} attempts");
                    }
                    return Ok(packages);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        match &last_error {
                            Some(ArchToolkitError::SearchFailed { query, .. }) => {
                                println!(
                                    "   ⚠ Attempt {attempt} failed for query '{query}', retrying..."
                                );
                            }
                            _ => {
                                println!("   ⚠ Attempt {attempt} failed, retrying...");
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
            }
        }
        Err(last_error.unwrap())
    }

    // Try search with retry (will succeed or show retry attempts)
    match search_with_retry(client, "yay", 3).await {
        Ok(packages) => {
            println!("   ✓ Search succeeded: found {} packages", packages.len());
        }
        Err(e) => {
            println!("   ✗ Search failed after retries: {e}");
        }
    }
    println!();

    // Pattern 3: Batch operation with per-item error handling
    println!("3. Batch Operation with Per-Item Error Handling:");
    async fn fetch_multiple_packages(
        client: &ArchClient,
        packages: &[&str],
    ) -> Vec<(String, Result<arch_toolkit::AurPackageDetails>)> {
        let mut results = Vec::new();
        match client.aur().info(packages).await {
            Ok(details) => {
                // Success case - all packages found
                let found_names: Vec<String> = details.iter().map(|d| d.name.clone()).collect();
                for detail in details {
                    results.push((detail.name.clone(), Ok(detail)));
                }
                // Check for missing packages
                for package in packages {
                    if !found_names.contains(&package.to_string()) {
                        results.push((
                            (*package).to_string(),
                            Err(ArchToolkitError::PackageNotFound {
                                package: (*package).to_string(),
                            }),
                        ));
                    }
                }
            }
            Err(e) => {
                // Extract package names from error if possible
                match &e {
                    ArchToolkitError::InfoFailed { packages, .. } => {
                        println!("   ⚠ Batch fetch failed for packages: {packages}");
                        // For demonstration, create individual errors for each package
                        for package in packages.split(", ") {
                            results.push((
                                package.to_string(),
                                Err(ArchToolkitError::InfoFailed {
                                    packages: packages.clone(),
                                    source: create_mock_error(),
                                }),
                            ));
                        }
                    }
                    _ => {
                        // Fallback: mark all as failed
                        for package in packages {
                            results.push((
                                (*package).to_string(),
                                Err(ArchToolkitError::InfoFailed {
                                    packages: packages.join(", "),
                                    source: create_mock_error(),
                                }),
                            ));
                        }
                    }
                }
            }
        }
        results
    }

    let results = fetch_multiple_packages(client, &["yay", "paru"]).await;
    for (package, result) in results {
        match result {
            Ok(detail) => {
                println!("   ✓ {package}: {}", detail.version);
            }
            Err(e) => {
                println!("   ✗ {package}: {e}");
            }
        }
    }
    println!();

    // Pattern 4: Error categorization
    println!("4. Error Categorization:");
    fn categorize_error(error: &ArchToolkitError) -> &'static str {
        match error {
            ArchToolkitError::SearchFailed { .. }
            | ArchToolkitError::InfoFailed { .. }
            | ArchToolkitError::CommentsFailed { .. }
            | ArchToolkitError::PkgbuildFailed { .. }
            | ArchToolkitError::Network(_) => "Network Error",
            ArchToolkitError::Json(_) | ArchToolkitError::Parse(_) => "Parsing Error",
            ArchToolkitError::RateLimited { .. } => "Rate Limit Error",
            ArchToolkitError::PackageNotFound { .. } => "Not Found Error",
            ArchToolkitError::InvalidInput(_) => "Input Error",
        }
    }

    let errors = vec![
        ArchToolkitError::search_failed("yay", create_mock_error()),
        ArchToolkitError::PackageNotFound {
            package: "nonexistent".to_string(),
        },
        ArchToolkitError::RateLimited {
            retry_after: Some(60),
        },
    ];

    for error in &errors {
        let category = categorize_error(error);
        println!("   {category}: {error}");
    }
    println!();

    Ok(())
}

/// What: Demonstrate error message comparison (with vs without context).
///
/// Inputs: None
///
/// Output:
/// - `Result<()>` indicating success or failure
///
/// Details:
/// - Shows the difference between generic and contextual errors
/// - Demonstrates the value of rich error context
fn demonstrate_error_comparison() -> Result<()> {
    println!("Error Message Comparison:");
    println!("──────────────────────────\n");

    println!("Scenario: AUR search for 'yay' fails due to network error\n");

    println!("❌ WITHOUT Context (Generic Error):");
    let generic_error = ArchToolkitError::Network(create_mock_error());
    println!("   {generic_error}\n");
    println!("   Problems:");
    println!("   • No indication of which query failed");
    println!("   • No indication of which operation failed");
    println!("   • Difficult to debug in batch operations\n");

    println!("✅ WITH Context (Rich Error):");
    let contextual_error = ArchToolkitError::search_failed("yay", create_mock_error());
    println!("   {contextual_error}\n");
    println!("   Benefits:");
    println!("   • Query 'yay' is preserved in error message");
    println!("   • Operation type (search) is clear");
    println!("   • Easy to identify which operation failed");
    println!("   • Better user experience with actionable errors\n");

    println!("Example: Batch Operations");
    println!("─────────────────────────\n");

    println!("Without context:");
    println!("  Error 1: Network error: connection timed out");
    println!("  Error 2: Network error: connection timed out");
    println!("  Error 3: Network error: connection timed out");
    println!("  ❓ Which queries failed? Unknown!\n");

    println!("With context:");
    println!("  Error 1: AUR search failed for query 'yay': connection timed out");
    println!("  Error 2: AUR search failed for query 'paru': connection timed out");
    println!("  Error 3: AUR search failed for query 'pacman': connection timed out");
    println!("  ✓ Clear: All three queries failed, but we know which ones!\n");

    Ok(())
}
