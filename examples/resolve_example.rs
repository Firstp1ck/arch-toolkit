//! Comprehensive dependency resolution example for arch-toolkit.
//!
//! This example demonstrates:
//! - Resolving dependencies for packages from official repos, AUR, or local
//! - Configuring dependency resolution with ResolverConfig
//! - Determining dependency status (installed, to install, to upgrade, conflict, missing)
//! - Batch fetching dependencies for multiple packages
//! - Fetching package conflicts
//! - Handling different dependency types (depends, makedepends, optdepends, checkdepends)
//!
//! Run with:
//!   `cargo run --example resolve_example --features deps`
//!   `cargo run --example resolve_example --features deps,aur`  # For AUR examples

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example resolve_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args
)] // Example file - comprehensive demonstration
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::deps::{
        DependencyResolver, batch_fetch_official_deps, determine_status, fetch_package_conflicts,
    };
    use arch_toolkit::deps::{
        get_installed_packages, get_provided_packages, get_upgradable_packages,
    };
    use arch_toolkit::types::dependency::ResolverConfig;
    use arch_toolkit::{PackageRef, PackageSource};

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Dependency Resolution Example             ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic Dependency Resolution
    // ========================================================================
    println!("┌─ Example 1: Basic Dependency Resolution ─────────────────────┐");
    println!("│ Resolve dependencies for a single package                   │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let resolver = DependencyResolver::new();
    let packages = vec![PackageRef {
        name: "pacman".to_string(),
        version: "6.1.0".to_string(),
        source: PackageSource::Official {
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    match resolver.resolve(&packages) {
        Ok(result) => {
            println!("  Resolved dependencies for: pacman");
            println!("  Found {} dependencies", result.dependencies.len());
            println!("  Found {} conflicts", result.conflicts.len());
            println!("  Found {} missing packages", result.missing.len());

            if !result.dependencies.is_empty() {
                println!("\n  Sample dependencies:");
                for dep in result.dependencies.iter().take(5) {
                    println!("    • {} - {:?}", dep.name, dep.status);
                }
            }
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }
    println!();

    // ========================================================================
    // Example 2: Dependency Resolution with Configuration
    // ========================================================================
    println!("┌─ Example 2: Dependency Resolution with Configuration ─────────┐");
    println!("│ Configure resolver to include optional/make/check deps      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // Default config (no optional deps)
    let config_default = ResolverConfig::default();
    let resolver_default = DependencyResolver::with_config(config_default);
    let packages = vec![PackageRef {
        name: "firefox".to_string(),
        version: "121.0".to_string(),
        source: PackageSource::Official {
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    match resolver_default.resolve(&packages) {
        Ok(result) => {
            println!("  Default config (no optional deps):");
            println!("    Dependencies: {}", result.dependencies.len());
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Config with optional dependencies
    let config_with_opt = ResolverConfig {
        include_optdepends: true,
        ..Default::default()
    };
    let resolver_with_opt = DependencyResolver::with_config(config_with_opt);

    match resolver_with_opt.resolve(&packages) {
        Ok(result) => {
            println!("  Config with optional deps:");
            println!("    Dependencies: {}", result.dependencies.len());
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Config with make dependencies
    let config_with_make = ResolverConfig {
        include_makedepends: true,
        ..Default::default()
    };
    let resolver_with_make = DependencyResolver::with_config(config_with_make);

    match resolver_with_make.resolve(&packages) {
        Ok(result) => {
            println!("  Config with make deps:");
            println!("    Dependencies: {}", result.dependencies.len());
        }
        Err(e) => println!("  Error: {}", e),
    }

    // Config with all dependency types
    let config_all = ResolverConfig {
        include_optdepends: true,
        include_makedepends: true,
        include_checkdepends: true,
        ..Default::default()
    };
    let resolver_all = DependencyResolver::with_config(config_all);

    match resolver_all.resolve(&packages) {
        Ok(result) => {
            println!("  Config with all dependency types:");
            println!("    Dependencies: {}", result.dependencies.len());
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // ========================================================================
    // Example 3: Determine Dependency Status
    // ========================================================================
    println!("┌─ Example 3: Determine Dependency Status ────────────────────────┐");
    println!("│ Check the status of individual dependencies                  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let provided = get_provided_packages(&installed);
            let upgradable = get_upgradable_packages().unwrap_or_default();

            let test_cases = vec![
                ("glibc", ""),               // Usually installed
                ("nonexistent-package", ""), // Not installed
                ("python", ">=3.10"),        // Version requirement
                ("bash", ""),                // Usually installed
            ];

            println!("  Checking dependency status:");
            for (name, version_req) in test_cases {
                let status =
                    determine_status(name, version_req, &installed, &provided, &upgradable);
                println!(
                    "    {} (req: {}): {:?}",
                    name,
                    if version_req.is_empty() {
                        "none"
                    } else {
                        version_req
                    },
                    status
                );
            }
        }
        Err(_) => println!("  Skipping (error getting installed packages)"),
    }
    println!();

    // ========================================================================
    // Example 4: Batch Fetching Dependencies
    // ========================================================================
    println!("┌─ Example 4: Batch Fetching Dependencies ─────────────────────┐");
    println!("│ Efficiently fetch dependencies for multiple packages         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let package_names = ["pacman", "glibc", "bash", "systemd"];
    let batch_deps = batch_fetch_official_deps(&package_names);

    println!(
        "  Batch fetched dependencies for {} packages:",
        package_names.len()
    );
    for pkg_name in &package_names {
        if let Some(deps) = batch_deps.get(*pkg_name) {
            println!("    {}: {} dependencies", pkg_name, deps.len());
            if !deps.is_empty() {
                let sample: Vec<&str> = deps.iter().map(|s| s.as_str()).take(3).collect();
                println!("      Sample: {}", sample.join(", "));
                if deps.len() > 3 {
                    println!("      ... and {} more", deps.len() - 3);
                }
            }
        } else {
            println!("    {}: No dependencies found", pkg_name);
        }
    }
    println!();

    // ========================================================================
    // Example 5: Fetch Package Conflicts
    // ========================================================================
    println!("┌─ Example 5: Fetch Package Conflicts ─────────────────────────┐");
    println!("│ Get list of packages that conflict with a given package      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let test_packages = vec![
        (
            "firefox",
            PackageSource::Official {
                repo: "extra".to_string(),
                arch: "x86_64".to_string(),
            },
        ),
        (
            "pacman",
            PackageSource::Official {
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
            },
        ),
    ];

    for (name, source) in test_packages {
        let conflicts = fetch_package_conflicts(name, &source);
        println!("  {} conflicts with {} package(s):", name, conflicts.len());
        if !conflicts.is_empty() {
            for conflict in conflicts.iter().take(5) {
                println!("    • {}", conflict);
            }
            if conflicts.len() > 5 {
                println!("    ... and {} more", conflicts.len() - 5);
            }
        }
    }
    println!();

    // ========================================================================
    // Example 6: Resolving Multiple Packages
    // ========================================================================
    println!("┌─ Example 6: Resolving Multiple Packages ─────────────────────┐");
    println!("│ Resolve dependencies for multiple packages at once          │");
    println!("└──────────────────────────────────────────────────────────────┘");

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

    match resolver.resolve(&packages) {
        Ok(result) => {
            println!("  Resolved dependencies for {} packages", packages.len());
            println!("  Total dependencies: {}", result.dependencies.len());
            println!("  Total conflicts: {}", result.conflicts.len());
            println!("  Total missing: {}", result.missing.len());

            // Group dependencies by status
            let mut by_status: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for dep in &result.dependencies {
                let status_key = format!("{:?}", dep.status);
                *by_status.entry(status_key).or_insert(0) += 1;
            }

            if !by_status.is_empty() {
                println!("\n  Dependencies by status:");
                for (status, count) in by_status {
                    println!("    {}: {}", status, count);
                }
            }
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }
    println!();

    // ========================================================================
    // Example 7: Dependency Resolution Details
    // ========================================================================
    println!("┌─ Example 7: Dependency Resolution Details ───────────────────┐");
    println!("│ Examine detailed information about resolved dependencies    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let resolver = DependencyResolver::new();
    let packages = vec![PackageRef {
        name: "bash".to_string(),
        version: "5.2".to_string(),
        source: PackageSource::Official {
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    match resolver.resolve(&packages) {
        Ok(result) => {
            if !result.dependencies.is_empty() {
                println!("  Sample dependency details:");
                for dep in result.dependencies.iter().take(3) {
                    println!("\n    Package: {}", dep.name);
                    println!("      Status: {:?}", dep.status);
                    println!("      Source: {:?}", dep.source);
                    println!(
                        "      Version requirement: {}",
                        if dep.version_req.is_empty() {
                            "none"
                        } else {
                            &dep.version_req
                        }
                    );
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
    println!();

    // ========================================================================
    // Example 8: Handling Missing Dependencies
    // ========================================================================
    println!("┌─ Example 8: Handling Missing Dependencies ───────────────────┐");
    println!("│ Identify dependencies that cannot be found                  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let resolver = DependencyResolver::new();
    // Use a package that might have unusual dependencies
    let packages = vec![PackageRef {
        name: "pacman".to_string(),
        version: "6.1.0".to_string(),
        source: PackageSource::Official {
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    match resolver.resolve(&packages) {
        Ok(result) => {
            if !result.missing.is_empty() {
                println!("  Missing dependencies found:");
                for missing in &result.missing {
                    println!("    • {}", missing);
                }
            } else {
                println!("  No missing dependencies (all dependencies found)");
            }

            if !result.conflicts.is_empty() {
                println!("\n  Conflicts found:");
                for conflict in &result.conflicts {
                    println!("    • {}", conflict);
                }
            } else {
                println!("\n  No conflicts found");
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    #[cfg(feature = "aur")]
    {
        // ========================================================================
        // Example 9: AUR Package Resolution (if enabled)
        // ========================================================================
        println!("┌─ Example 9: AUR Package Resolution ────────────────────────┐");
        println!("│ Resolve dependencies for AUR packages (if aur feature)   │");
        println!("└──────────────────────────────────────────────────────────────┘");

        println!("  AUR feature is enabled!");
        println!("  Note: AUR dependency resolution has limitations due to");
        println!("  async .SRCINFO fetching. See documentation for details.");
        println!();
    }

    #[cfg(not(feature = "aur"))]
    {
        println!("┌─ Example 9: AUR Package Resolution ────────────────────────┐");
        println!("│ AUR feature is not enabled                                 │");
        println!("└──────────────────────────────────────────────────────────────┘");
        println!("  Enable with: cargo run --example resolve_example --features \"deps,aur\"");
        println!();
    }

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!("\nThis example demonstrated:");
    println!("  • DependencyResolver - Main resolver struct");
    println!("  • ResolverConfig - Configuration for dependency resolution");
    println!("  • determine_status() - Check individual dependency status");
    println!("  • batch_fetch_official_deps() - Efficient batch queries");
    println!("  • fetch_package_conflicts() - Get package conflicts");
    println!("  • Handling different dependency types and statuses");
    println!("  • Resolving dependencies for multiple packages");
    println!("\nAll resolution functions work with official repositories,");
    println!("AUR packages (if aur feature enabled), and local packages.");

    Ok(())
}
