//! Comprehensive reverse dependency analysis example for arch-toolkit.
//!
//! This example demonstrates:
//! - Analyzing reverse dependencies for packages being removed
//! - Finding all packages that depend on removal targets
//! - Distinguishing direct vs transitive dependents
//! - Getting summary statistics for reverse dependencies
//! - Using helper functions to check for installed dependents
//! - Understanding conflict status and source determination
//!
//! Run with:
//!   `cargo run --example reverse_example --features deps`

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example reverse_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_not
)] // Example file - comprehensive demonstration
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::deps::get_installed_packages;
    use arch_toolkit::deps::{
        ReverseDependencyAnalyzer, get_installed_required_by, has_installed_required_by,
    };
    use arch_toolkit::{PackageRef, PackageSource};

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Reverse Dependency Analysis Example       ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic Reverse Dependency Analysis
    // ========================================================================
    println!("┌─ Example 1: Basic Reverse Dependency Analysis ─────────────────┐");
    println!("│ Find all packages that depend on a package being removed    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    // Find a package with a reasonable number of direct dependents (< 50)
    // Note: Total dependents (including transitive) may be higher
    let test_packages = ["wget", "nano", "vim", "htop", "curl", "git"];
    let mut selected_package: Option<&str> = None;

    println!("  Finding a suitable test package (< 50 direct dependents)...");
    println!("  (Note: Total dependents including transitive may be higher)");
    for pkg_name in &test_packages {
        if has_installed_required_by(pkg_name) {
            let count = get_installed_required_by(pkg_name).len();
            println!("    {}: {} direct dependents", pkg_name, count);
            if count < 50 && selected_package.is_none() {
                selected_package = Some(pkg_name);
                println!("    → Selected {} for analysis", pkg_name);
            }
        } else {
            println!("    {}: 0 dependents (good for quick test)", pkg_name);
            if selected_package.is_none() {
                selected_package = Some(pkg_name);
                println!("    → Selected {} for analysis", pkg_name);
            }
        }
    }

    if let Some(pkg_name) = selected_package {
        match get_installed_packages() {
            Ok(installed) if installed.contains(pkg_name) => {
                let packages = vec![PackageRef {
                    name: pkg_name.to_string(),
                    version: "1.0".to_string(),
                    source: PackageSource::Official {
                        repo: "extra".to_string(),
                        arch: "x86_64".to_string(),
                    },
                }];

                println!("\n  Analyzing reverse dependencies for: {}", pkg_name);
                println!("  Progress: Querying pacman database...");
                println!("  (This may take a moment for packages with many transitive dependents)");
                match analyzer.analyze(&packages) {
                    Ok(report) => {
                        println!("  ✓ Analysis complete!");
                        println!("  Found {} dependents", report.dependents.len());
                        println!("  Found {} summary entries", report.summaries.len());

                        if !report.dependents.is_empty() {
                            println!("\n  Sample dependents (first 5):");
                            for (i, dep) in report.dependents.iter().take(5).enumerate() {
                                println!("    {}. {} - {:?}", i + 1, dep.name, dep.status);
                            }
                        }
                    }
                    Err(e) => println!("  ✗ Error analyzing reverse dependencies: {}", e),
                }
            }
            _ => println!("  Skipping ({} not installed)", pkg_name),
        }
    } else {
        println!("  No suitable package found with < 100 dependents");
        println!("  (All test packages have too many or no dependents)");
    }
    println!();

    // ========================================================================
    // Example 2: Reverse Dependency Summary Statistics
    // ========================================================================
    println!("┌─ Example 2: Reverse Dependency Summary Statistics ─────────────┐");
    println!("│ Get detailed statistics about direct vs transitive deps     │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    // Find a package with very few dependents for quick demonstration
    // Prefer packages with 0-10 direct dependents to keep total dependents low
    let test_packages = ["wget", "git", "nano", "vim", "htop"];
    let mut selected_package: Option<&str> = None;

    println!("  Finding a package with very few dependents for quick demo...");
    for pkg_name in &test_packages {
        if has_installed_required_by(pkg_name) {
            let count = get_installed_required_by(pkg_name).len();
            println!("    {}: {} direct dependents", pkg_name, count);
            // Prefer packages with 0-10 direct dependents
            if count <= 10 && selected_package.is_none() {
                selected_package = Some(pkg_name);
                println!(
                    "    → Selected {} (has {} direct dependents)",
                    pkg_name, count
                );
            }
        } else {
            println!("    {}: 0 dependents (good for quick test)", pkg_name);
            if selected_package.is_none() {
                selected_package = Some(pkg_name);
                println!("    → Selected {} (no dependents)", pkg_name);
            }
        }
    }

    if let Some(pkg_name) = selected_package {
        match get_installed_packages() {
            Ok(installed) if installed.contains(pkg_name) => {
                let packages = vec![PackageRef {
                    name: pkg_name.to_string(),
                    version: "1.0".to_string(),
                    source: PackageSource::Official {
                        repo: "extra".to_string(),
                        arch: "x86_64".to_string(),
                    },
                }];

                println!("\n  Analyzing: {}...", pkg_name);
                println!("  Progress: Querying pacman database...");
                match analyzer.analyze(&packages) {
                    Ok(report) => {
                        println!("  ✓ Analysis complete!");
                        if !report.summaries.is_empty() {
                            println!("  Summary statistics for removal targets:");
                            for summary in report.summaries.iter().take(5) {
                                println!("\n    Package: {}", summary.package);
                                println!("      Direct dependents: {}", summary.direct_dependents);
                                println!(
                                    "      Transitive dependents: {}",
                                    summary.transitive_dependents
                                );
                                println!("      Total dependents: {}", summary.total_dependents);
                            }
                        } else {
                            println!("  No summary statistics available");
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            }
            _ => println!("  Skipping ({} not installed)", pkg_name),
        }
    } else {
        println!("  No suitable package found");
    }
    println!();

    // ========================================================================
    // Example 3: Helper Functions - Check for Dependents
    // ========================================================================
    println!("┌─ Example 3: Helper Functions - Check for Dependents ─────────┐");
    println!("│ Quick check if a package has installed dependents           │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let test_packages = ["glibc", "bash", "pacman", "systemd"];

    println!("  Checking for installed dependents:");
    for pkg_name in &test_packages {
        if has_installed_required_by(pkg_name) {
            println!("    ✓ {} has installed dependents", pkg_name);
            let dependents = get_installed_required_by(pkg_name);
            println!("      Found {} dependents", dependents.len());
            if !dependents.is_empty() {
                let sample: Vec<&str> = dependents.iter().map(|s| s.as_str()).take(3).collect();
                println!("      Sample: {}", sample.join(", "));
                if dependents.len() > 3 {
                    println!("      ... and {} more", dependents.len() - 3);
                }
            }
        } else {
            println!("    ✗ {} has no installed dependents", pkg_name);
        }
    }
    println!();

    // ========================================================================
    // Example 4: Detailed Dependent Information
    // ========================================================================
    println!("┌─ Example 4: Detailed Dependent Information ───────────────────┐");
    println!("│ Examine detailed information about dependents              │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    // Find a package with reasonable dependents (< 50 direct)
    let test_packages = ["wget", "nano", "vim", "htop"];
    let mut selected_package: Option<&str> = None;

    for pkg_name in &test_packages {
        if has_installed_required_by(pkg_name) {
            let count = get_installed_required_by(pkg_name).len();
            if count < 50 && selected_package.is_none() {
                selected_package = Some(pkg_name);
            }
        } else if selected_package.is_none() {
            selected_package = Some(pkg_name);
        }
    }

    if let Some(pkg_name) = selected_package {
        match get_installed_packages() {
            Ok(installed) if installed.contains(pkg_name) => {
                let packages = vec![PackageRef {
                    name: pkg_name.to_string(),
                    version: "1.0".to_string(),
                    source: PackageSource::Official {
                        repo: "extra".to_string(),
                        arch: "x86_64".to_string(),
                    },
                }];

                println!("  Analyzing: {}...", pkg_name);
                println!("  Progress: Querying pacman database...");
                match analyzer.analyze(&packages) {
                    Ok(report) => {
                        println!("  ✓ Analysis complete!");
                        if !report.dependents.is_empty() {
                            println!("  Sample dependent details:");
                            for dep in report.dependents.iter().take(3) {
                                println!("\n    Package: {}", dep.name);
                                println!("      Status: {:?}", dep.status);
                                println!("      Source: {:?}", dep.source);
                                println!("      Required by: {} package(s)", dep.required_by.len());
                                if !dep.required_by.is_empty() {
                                    println!("        {}", dep.required_by.join(", "));
                                }
                                println!("      Depends on: {} package(s)", dep.depends_on.len());
                                println!("      Is core repository: {}", dep.is_core);
                                println!("      Is system package: {}", dep.is_system);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            }
            _ => println!("  Skipping ({} not installed)", pkg_name),
        }
    } else {
        println!("  No suitable package found");
    }
    println!();

    // ========================================================================
    // Example 5: Analyzing Multiple Packages
    // ========================================================================
    println!("┌─ Example 5: Analyzing Multiple Packages ─────────────────────┐");
    println!("│ Analyze reverse dependencies for multiple packages         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    match get_installed_packages() {
        Ok(installed) => {
            let mut packages_to_analyze = Vec::new();

            // Find packages with reasonable dependents (< 50 direct)
            let test_packages = ["wget", "nano", "vim"];
            for pkg_name in &test_packages {
                if installed.contains(*pkg_name) {
                    let count = if has_installed_required_by(pkg_name) {
                        get_installed_required_by(pkg_name).len()
                    } else {
                        0
                    };
                    if count < 50 && packages_to_analyze.len() < 2 {
                        packages_to_analyze.push(PackageRef {
                            name: pkg_name.to_string(),
                            version: "1.0".to_string(),
                            source: PackageSource::Official {
                                repo: "extra".to_string(),
                                arch: "x86_64".to_string(),
                            },
                        });
                    }
                }
            }

            if !packages_to_analyze.is_empty() {
                println!("  Analyzing {} packages...", packages_to_analyze.len());
                match analyzer.analyze(&packages_to_analyze) {
                    Ok(report) => {
                        println!("  ✓ Analysis complete!");
                        println!("  Found {} total dependents", report.dependents.len());
                        println!("  Found {} summary entries", report.summaries.len());

                        if !report.summaries.is_empty() {
                            println!("\n  Per-package summary:");
                            for summary in &report.summaries {
                                println!(
                                    "    {}: {} total dependents",
                                    summary.package, summary.total_dependents
                                );
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            } else {
                println!("  No suitable packages found (< 100 dependents each)");
            }
        }
        Err(_) => println!("  Skipping (error getting installed packages)"),
    }
    println!();

    // ========================================================================
    // Example 6: Conflict Status in Reverse Dependencies
    // ========================================================================
    println!("┌─ Example 6: Conflict Status in Reverse Dependencies ─────────┐");
    println!("│ Understand conflict status for removal operations           │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    // Find a package with reasonable dependents (< 50 direct)
    let test_packages = ["wget", "nano", "vim", "htop"];
    let mut selected_package: Option<&str> = None;

    for pkg_name in &test_packages {
        if has_installed_required_by(pkg_name) {
            let count = get_installed_required_by(pkg_name).len();
            if count < 50 && selected_package.is_none() {
                selected_package = Some(pkg_name);
            }
        } else if selected_package.is_none() {
            selected_package = Some(pkg_name);
        }
    }

    if let Some(pkg_name) = selected_package {
        match get_installed_packages() {
            Ok(installed) if installed.contains(pkg_name) => {
                let packages = vec![PackageRef {
                    name: pkg_name.to_string(),
                    version: "1.0".to_string(),
                    source: PackageSource::Official {
                        repo: "extra".to_string(),
                        arch: "x86_64".to_string(),
                    },
                }];

                println!("  Analyzing: {}...", pkg_name);
                println!("  Progress: Querying pacman database...");
                match analyzer.analyze(&packages) {
                    Ok(report) => {
                        println!("  ✓ Analysis complete!");
                        // Count dependents by status
                        let mut status_counts: std::collections::HashMap<String, usize> =
                            std::collections::HashMap::new();
                        for dep in &report.dependents {
                            let status_key = format!("{:?}", dep.status);
                            *status_counts.entry(status_key).or_insert(0) += 1;
                        }

                        if !status_counts.is_empty() {
                            println!("  Dependents by status:");
                            for (status, count) in status_counts {
                                println!("    {}: {}", status, count);
                            }
                        }

                        // Show examples of conflict status
                        let conflicts: Vec<_> = report
                            .dependents
                            .iter()
                            .filter(|dep| {
                                matches!(
                                    dep.status,
                                    arch_toolkit::types::dependency::DependencyStatus::Conflict { .. }
                                )
                            })
                            .take(3)
                            .collect();

                        if !conflicts.is_empty() {
                            println!("\n  Sample conflict statuses:");
                            for dep in conflicts {
                                println!("    {}: {:?}", dep.name, dep.status);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            }
            _ => println!("  Skipping ({} not installed)", pkg_name),
        }
    } else {
        println!("  No suitable package found");
    }
    println!();

    // ========================================================================
    // Example 7: Source Determination in Reverse Dependencies
    // ========================================================================
    println!("┌─ Example 7: Source Determination in Reverse Dependencies ───┐");
    println!("│ See where dependents come from (official, AUR, local)       │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    // Find a package with reasonable dependents (< 50 direct)
    let test_packages = ["wget", "nano", "vim", "htop"];
    let mut selected_package: Option<&str> = None;

    for pkg_name in &test_packages {
        if has_installed_required_by(pkg_name) {
            let count = get_installed_required_by(pkg_name).len();
            if count < 50 && selected_package.is_none() {
                selected_package = Some(pkg_name);
            }
        } else if selected_package.is_none() {
            selected_package = Some(pkg_name);
        }
    }

    if let Some(pkg_name) = selected_package {
        match get_installed_packages() {
            Ok(installed) if installed.contains(pkg_name) => {
                let packages = vec![PackageRef {
                    name: pkg_name.to_string(),
                    version: "1.0".to_string(),
                    source: PackageSource::Official {
                        repo: "extra".to_string(),
                        arch: "x86_64".to_string(),
                    },
                }];

                println!("  Analyzing: {}...", pkg_name);
                println!("  Progress: Querying pacman database...");
                match analyzer.analyze(&packages) {
                    Ok(report) => {
                        println!("  ✓ Analysis complete!");
                        // Count dependents by source
                        let mut source_counts: std::collections::HashMap<String, usize> =
                            std::collections::HashMap::new();
                        for dep in &report.dependents {
                            let source_key = format!("{:?}", dep.source);
                            *source_counts.entry(source_key).or_insert(0) += 1;
                        }

                        if !source_counts.is_empty() {
                            println!("  Dependents by source:");
                            for (source, count) in source_counts {
                                println!("    {}: {}", source, count);
                            }
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            }
            _ => println!("  Skipping ({} not installed)", pkg_name),
        }
    } else {
        println!("  No suitable package found");
    }
    println!();

    // ========================================================================
    // Example 8: Empty Analysis (No Dependents)
    // ========================================================================
    println!("┌─ Example 8: Empty Analysis (No Dependents) ─────────────────┐");
    println!("│ Handle cases where packages have no dependents              │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let analyzer = ReverseDependencyAnalyzer::new();

    // Try with a package that likely has no dependents
    match get_installed_packages() {
        Ok(installed) => {
            // Find a package that might have no dependents
            let test_packages = ["linux", "bash", "pacman"];
            let mut found_package = None;

            for pkg_name in &test_packages {
                if installed.contains(*pkg_name) && !has_installed_required_by(pkg_name) {
                    found_package = Some(*pkg_name);
                    break;
                }
            }

            if let Some(pkg_name) = found_package {
                let packages = vec![PackageRef {
                    name: pkg_name.to_string(),
                    version: "1.0".to_string(),
                    source: PackageSource::Official {
                        repo: "core".to_string(),
                        arch: "x86_64".to_string(),
                    },
                }];

                match analyzer.analyze(&packages) {
                    Ok(report) => {
                        println!("  Analyzing: {}", pkg_name);
                        println!("  Dependents: {}", report.dependents.len());
                        println!("  Summaries: {}", report.summaries.len());

                        if report.dependents.is_empty() {
                            println!("  ✓ No dependents found - safe to remove");
                        } else {
                            println!("  ⚠ Found {} dependents", report.dependents.len());
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            } else {
                println!("  All test packages have dependents or are not installed");
            }
        }
        Err(_) => println!("  Skipping (error getting installed packages)"),
    }
    println!();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nThis example demonstrated:");
    println!("  • ReverseDependencyAnalyzer - Main analyzer struct");
    println!("  • analyze() - Perform reverse dependency analysis");
    println!("  • has_installed_required_by() - Quick check for dependents");
    println!("  • get_installed_required_by() - Get list of dependents");
    println!("  • ReverseDependencyReport - Analysis results with summaries");
    println!("  • Distinguishing direct vs transitive dependents");
    println!("  • Conflict status and source determination");
    println!("\nReverse dependency analysis is essential for safe package");
    println!("removal operations, helping identify what will be affected.");

    Ok(())
}
