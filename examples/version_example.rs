//! Comprehensive version comparison example for arch-toolkit.
//!
//! This example demonstrates:
//! - Comparing package versions using pacman-compatible algorithm
//! - Checking if versions satisfy requirements (>=, <=, =, >, <)
//! - Extracting major version components
//! - Detecting major version bumps
//! - Handling edge cases (pkgrel suffixes, alpha/beta versions, etc.)
//!
//! Run with:
//!   `cargo run --example version_example --features deps`

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example version_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args
)] // Example file - comprehensive demonstration
fn main() {
    use arch_toolkit::deps::{
        compare_versions, extract_major_component, is_major_version_bump, version_satisfies,
    };
    use std::cmp::Ordering;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Version Comparison Example                 ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic Version Comparison
    // ========================================================================
    println!("┌─ Example 1: Basic Version Comparison ──────────────────────────┐");
    println!("│ Compare two version strings using pacman-compatible algorithm │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let test_cases = vec![
        ("1.2.3", "1.2.4", Ordering::Less),
        ("2.0.0", "1.9.9", Ordering::Greater),
        ("1.0", "1.0.0", Ordering::Equal),
        ("1.2.3", "1.2.3", Ordering::Equal),
        ("10.0.0", "9.99.99", Ordering::Greater),
        ("0.1.0", "0.0.9", Ordering::Greater),
    ];

    for (a, b, expected) in test_cases {
        let result = compare_versions(a, b);
        let symbol = match result {
            Ordering::Less => "<",
            Ordering::Equal => "==",
            Ordering::Greater => ">",
        };
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} {} {} {} (expected: {:?})",
            status, a, symbol, b, expected
        );
    }
    println!();

    // ========================================================================
    // Example 2: Version Comparison with Pkgrel Suffixes
    // ========================================================================
    println!("┌─ Example 2: Version Comparison with Pkgrel Suffixes ───────────┐");
    println!("│ Pkgrel suffixes (e.g., -1, -2) are normalized for comparison│");
    println!("└──────────────────────────────────────────────────────────────┘");

    let pkgrel_cases = vec![
        ("1.2.3-1", "1.2.3-2", Ordering::Equal), // Pkgrel is normalized, so versions are equal
        ("1.2.3-10", "1.2.3-2", Ordering::Equal), // Pkgrel is normalized, so versions are equal
        ("1.2.3-1", "1.2.3", Ordering::Equal),   // Pkgrel is normalized
        ("2.0.0-1", "1.9.9-5", Ordering::Greater), // Base versions differ
    ];

    for (a, b, expected) in pkgrel_cases {
        let result = compare_versions(a, b);
        let symbol = match result {
            Ordering::Less => "<",
            Ordering::Equal => "==",
            Ordering::Greater => ">",
        };
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} {} {} {} (expected: {:?})",
            status, a, symbol, b, expected
        );
    }
    println!();

    // ========================================================================
    // Example 3: Version Comparison with Text Suffixes
    // ========================================================================
    println!("┌─ Example 3: Version Comparison with Text Suffixes ────────────┐");
    println!("│ Versions with text suffixes (alpha, beta, rc) are handled     │");
    println!("│ Numeric versions are considered greater than text versions    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let text_suffix_cases = vec![
        ("1.2.3", "1.2.3alpha", Ordering::Greater), // Numeric > text
        ("1.2.3alpha", "1.2.3beta", Ordering::Less), // Lexicographic
        ("1.2.3alpha", "1.2.3beta", Ordering::Less),
        ("1.2.3rc1", "1.2.3rc2", Ordering::Less),
        ("1.2.3", "1.2.3-1", Ordering::Equal), // Pkgrel is normalized, so versions are equal
    ];

    for (a, b, expected) in text_suffix_cases {
        let result = compare_versions(a, b);
        let symbol = match result {
            Ordering::Less => "<",
            Ordering::Equal => "==",
            Ordering::Greater => ">",
        };
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} {} {} {} (expected: {:?})",
            status, a, symbol, b, expected
        );
    }
    println!();

    // ========================================================================
    // Example 4: Version Requirement Satisfaction
    // ========================================================================
    println!("┌─ Example 4: Version Requirement Satisfaction ─────────────────┐");
    println!("│ Check if a version satisfies a requirement (>=, <=, =, >, <)    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let requirement_cases = vec![
        ("2.0", ">=1.5", true),
        ("1.0", ">=1.5", false),
        ("1.5", "=1.5", true),
        ("1.6", "=1.5", false),
        ("1.4", "<1.5", true),
        ("1.5", "<1.5", false),
        ("1.6", ">1.5", true),
        ("1.5", ">1.5", false),
        ("1.5", "<=1.5", true),
        ("1.6", "<=1.5", false),
        ("2.0.0", ">=1.0.0", true),
        ("0.9.0", ">=1.0.0", false),
        ("1.2.3-1", ">=1.2.3", true), // Pkgrel normalized
        ("1.2.2", ">=1.2.3", false),  // Note: this is a string comparison edge case
    ];

    for (version, requirement, expected) in requirement_cases {
        let result = version_satisfies(version, requirement);
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} version_satisfies(\"{}\", \"{}\"): {} (expected: {})",
            status, version, requirement, result, expected
        );
    }
    println!();

    // ========================================================================
    // Example 5: Extract Major Version Component
    // ========================================================================
    println!("┌─ Example 5: Extract Major Version Component ───────────────────┐");
    println!("│ Extract the major version number from a version string       │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let major_cases = vec![
        ("2.0.0", Some(2)),
        ("10.5.2", Some(10)),
        ("1.2.3-alpha", Some(1)),
        ("0.1.0", Some(0)),
        ("100.0.0", Some(100)),
        ("1.2.3-1", Some(1)), // Pkgrel doesn't affect major component
    ];

    for (version, expected) in major_cases {
        let result = extract_major_component(version);
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} extract_major_component(\"{}\"): {:?} (expected: {:?})",
            status, version, result, expected
        );
    }
    println!();

    // ========================================================================
    // Example 6: Detect Major Version Bumps
    // ========================================================================
    println!("┌─ Example 6: Detect Major Version Bumps ───────────────────────┐");
    println!("│ Check if an upgrade represents a major version bump          │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let bump_cases = vec![
        ("1.2.3", "2.0.0", true),       // Major bump
        ("1.2.3", "1.3.0", false),      // Minor bump
        ("1.2.3", "1.2.4", false),      // Patch bump
        ("1.2.3-1", "2.0.0-1", true),   // Major bump with pkgrel
        ("10.0.0", "11.0.0", true),     // Major bump (double digit)
        ("0.9.0", "1.0.0", true),       // Major bump from 0.x to 1.x
        ("2.0.0", "2.1.0", false),      // Minor bump
        ("1.2.3-alpha", "2.0.0", true), // Major bump from alpha
    ];

    for (old, new, expected) in bump_cases {
        let result = is_major_version_bump(old, new);
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} is_major_version_bump(\"{}\", \"{}\"): {} (expected: {})",
            status, old, new, result, expected
        );
    }
    println!();

    // ========================================================================
    // Example 7: Real-World Package Version Scenarios
    // ========================================================================
    println!("┌─ Example 7: Real-World Package Version Scenarios ─────────────┐");
    println!("│ Examples using actual Arch Linux package version patterns    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // Simulate checking if installed version satisfies dependency requirement
    let scenarios = vec![
        ("6.1.0-1", ">=6.0.0", "pacman upgrade check"),
        ("121.0-1", ">=120.0", "firefox version check"),
        ("2.38-1", ">=2.35", "glibc compatibility check"),
        ("5.15.10-1", ">=5.15.0", "qt5-base version check"),
    ];

    for (installed, requirement, description) in scenarios {
        let satisfies = version_satisfies(installed, requirement);
        let status = if satisfies {
            "✓ Satisfies"
        } else {
            "✗ Does not satisfy"
        };
        println!(
            "  {}: {} {} requirement {}",
            description, installed, status, requirement
        );
    }
    println!();

    // ========================================================================
    // Example 8: Edge Cases and Special Versions
    // ========================================================================
    println!("┌─ Example 8: Edge Cases and Special Versions ─────────────────┐");
    println!("│ Handling edge cases in version comparison                     │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let edge_cases = vec![
        ("1", "1.0", Ordering::Equal),            // Missing segments
        ("1.0", "1.0.0.0", Ordering::Equal),      // Extra segments
        ("1.2.3-10", "1.2.3-2", Ordering::Equal), // Pkgrel is normalized
    ];

    for (a, b, expected) in edge_cases {
        let result = compare_versions(a, b);
        let symbol = match result {
            Ordering::Less => "<",
            Ordering::Equal => "==",
            Ordering::Greater => ">",
        };
        let status = if result == expected { "✓" } else { "✗" };
        println!(
            "  {} compare_versions(\"{}\", \"{}\"): {} (expected: {:?})",
            status, a, b, symbol, expected
        );
    }
    println!();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nThis example demonstrated:");
    println!("  • compare_versions() - Pacman-compatible version comparison");
    println!("  • version_satisfies() - Check version requirement satisfaction");
    println!("  • extract_major_component() - Extract major version number");
    println!("  • is_major_version_bump() - Detect major version upgrades");
    println!("  • Handling of pkgrel suffixes, text suffixes, and edge cases");
    println!("\nAll version comparison functions use pacman's algorithm for");
    println!("consistency with Arch Linux package management.");
}
