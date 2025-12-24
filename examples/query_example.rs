//! Comprehensive package querying example for arch-toolkit.
//!
//! This example demonstrates:
//! - Querying installed packages from pacman database
//! - Finding packages with available upgrades
//! - Checking provided packages (lazy checking)
//! - Determining if packages are installed or provided
//! - Getting installed and available package versions
//! - Graceful degradation when pacman is unavailable
//!
//! Run with:
//!   `cargo run --example query_example --features deps`

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example query_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // Example file - comprehensive demonstration
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::deps::{
        get_available_version, get_installed_packages, get_installed_version,
        get_provided_packages, get_upgradable_packages, is_package_installed_or_provided,
    };

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Package Querying Example                    ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Get All Installed Packages
    // ========================================================================
    println!("┌─ Example 1: Get All Installed Packages ────────────────────────┐");
    println!("│ Query the complete list of installed packages                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            println!("Found {} installed packages", installed.len());
            if !installed.is_empty() {
                println!("\nSample packages (first 10):");
                for (i, pkg) in installed.iter().take(10).enumerate() {
                    println!("  {}. {}", i + 1, pkg);
                }
                if installed.len() > 10 {
                    println!("  ... and {} more packages", installed.len() - 10);
                }

                // Check for common packages
                let common_packages = ["pacman", "glibc", "bash", "systemd", "linux"];
                println!("\nChecking for common packages:");
                for pkg in &common_packages {
                    if installed.contains(*pkg) {
                        println!("  ✓ {} is installed", pkg);
                    } else {
                        println!("  ✗ {} is not installed", pkg);
                    }
                }
            } else {
                println!("No packages found (pacman may not be available)");
            }
        }
        Err(e) => {
            println!("Error querying installed packages: {}", e);
            println!("Note: This function gracefully degrades when pacman is unavailable");
        }
    }

    // ========================================================================
    // Example 2: Get Upgradable Packages
    // ========================================================================
    println!("\n┌─ Example 2: Get Upgradable Packages ──────────────────────────┐");
    println!("│ Find packages that have upgrades available                    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_upgradable_packages() {
        Ok(upgradable) => {
            println!("Found {} upgradable packages", upgradable.len());
            if !upgradable.is_empty() {
                println!("\nPackages with available upgrades:");
                for (i, pkg) in upgradable.iter().take(10).enumerate() {
                    println!("  {}. {}", i + 1, pkg);
                }
                if upgradable.len() > 10 {
                    println!("  ... and {} more packages", upgradable.len() - 10);
                }
            } else {
                println!("No upgradable packages found");
                println!("Note: This could mean:");
                println!("  - System is up to date");
                println!("  - pacman is not available");
                println!("  - No packages need updating");
            }
        }
        Err(e) => {
            println!("Error querying upgradable packages: {}", e);
            println!("Note: This function gracefully degrades when pacman is unavailable");
        }
    }

    // ========================================================================
    // Example 3: Get Installed Version
    // ========================================================================
    println!("\n┌─ Example 3: Get Installed Version ────────────────────────────┐");
    println!("│ Query the installed version of a specific package            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let test_packages = ["pacman", "glibc", "bash", "nonexistent-package"];

    for pkg_name in &test_packages {
        match get_installed_version(pkg_name) {
            Ok(version) => {
                println!("  ✓ {}: version {}", pkg_name, version);
            }
            Err(e) => {
                println!("  ✗ {}: {}", pkg_name, e);
            }
        }
    }

    println!("\nNote: Version strings are normalized (revision suffixes removed)");
    println!("      Format: 'name version' or 'name version-revision' -> 'version'");

    // ========================================================================
    // Example 4: Get Available Version
    // ========================================================================
    println!("\n┌─ Example 4: Get Available Version ────────────────────────────┐");
    println!("│ Query the latest available version from repositories         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let test_packages = ["pacman", "glibc", "bash", "nonexistent-package"];

    for pkg_name in &test_packages {
        match get_available_version(pkg_name) {
            Some(version) => {
                println!("  ✓ {}: available version {}", pkg_name, version);
            }
            None => {
                println!("  ✗ {}: not found in repositories", pkg_name);
            }
        }
    }

    println!("\nNote: Returns None if package is not found or pacman is unavailable");
    println!("      Version strings are normalized (revision suffixes removed)");

    // ========================================================================
    // Example 5: Compare Installed vs Available Versions
    // ========================================================================
    println!("\n┌─ Example 5: Compare Installed vs Available Versions ───────────┐");
    println!("│ Compare installed and available versions to check for updates │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let packages_to_check = ["pacman", "glibc", "bash"];

    for pkg_name in &packages_to_check {
        let installed_ver = get_installed_version(pkg_name).ok();
        let available_ver = get_available_version(pkg_name);

        match (installed_ver, available_ver) {
            (Some(installed), Some(available)) => {
                if installed == available {
                    println!(
                        "  ✓ {}: up to date ({} == {})",
                        pkg_name, installed, available
                    );
                } else {
                    println!(
                        "  → {}: update available ({} -> {})",
                        pkg_name, installed, available
                    );
                }
            }
            (Some(installed), None) => {
                println!(
                    "  ? {}: installed ({}) but not in repos",
                    pkg_name, installed
                );
                println!("     (may be AUR or local package)");
            }
            (None, Some(available)) => {
                println!(
                    "  → {}: not installed, available version {}",
                    pkg_name, available
                );
            }
            (None, None) => {
                println!("  ✗ {}: not installed and not in repos", pkg_name);
            }
        }
    }

    // ========================================================================
    // Example 6: Get Provided Packages (Lazy Checking)
    // ========================================================================
    println!("\n┌─ Example 6: Get Provided Packages (Lazy Checking) ────────────┐");
    println!("│ Check provided packages using lazy on-demand checking         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let provided = get_provided_packages(&installed);
            println!("Provided packages set size: {}", provided.len());
            println!("\nNote: get_provided_packages() returns an empty set");
            println!(
                "      Provides are checked lazily on-demand using is_package_installed_or_provided()"
            );
            println!(
                "      This avoids querying all installed packages upfront, which is very slow"
            );
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    // ========================================================================
    // Example 7: Check if Package is Installed or Provided
    // ========================================================================
    println!("\n┌─ Example 7: Check if Package is Installed or Provided ────────┐");
    println!("│ Check if a package is directly installed or provided          │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let provided = get_provided_packages(&installed);

            let test_packages = [
                "pacman",      // Usually directly installed
                "glibc",       // Usually directly installed
                "rust",        // May be provided by rustup
                "python",      // May be provided by python3
                "nonexistent", // Not installed or provided
            ];

            println!("Checking packages (installed or provided):");
            for pkg_name in &test_packages {
                let is_available =
                    is_package_installed_or_provided(pkg_name, &installed, &provided);
                if is_available {
                    println!("  ✓ {}: installed or provided", pkg_name);
                } else {
                    println!("  ✗ {}: not installed or provided", pkg_name);
                }
            }

            println!("\nNote: This function uses lazy checking with 'pacman -Qqo'");
            println!("      It first checks if directly installed, then checks if provided");
            println!("      This is much faster than querying all packages upfront");
        }
        Err(e) => {
            println!("Error getting installed packages: {}", e);
        }
    }

    // ========================================================================
    // Example 8: Real-World Use Case - Check System Status
    // ========================================================================
    println!("\n┌─ Example 8: Real-World Use Case - Check System Status ────────┐");
    println!("│ Check system status by querying key packages                  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let critical_packages = ["pacman", "glibc", "bash", "systemd"];

    println!("Checking critical system packages:");
    match get_installed_packages() {
        Ok(installed) => {
            let provided = get_provided_packages(&installed);
            let upgradable = get_upgradable_packages().unwrap_or_default();

            for pkg_name in &critical_packages {
                let is_installed =
                    is_package_installed_or_provided(pkg_name, &installed, &provided);
                let installed_ver = get_installed_version(pkg_name).ok();
                let available_ver = get_available_version(pkg_name);
                let needs_update = upgradable.contains(*pkg_name);

                println!("\n  Package: {}", pkg_name);
                println!(
                    "    Installed: {}",
                    if is_installed { "✓ Yes" } else { "✗ No" }
                );
                if let Some(ver) = installed_ver {
                    println!("    Installed Version: {}", ver);
                }
                if let Some(ver) = available_ver {
                    println!("    Available Version: {}", ver);
                }
                if needs_update {
                    println!("    Status: → Update available");
                } else if is_installed {
                    println!("    Status: ✓ Up to date");
                }
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    // ========================================================================
    // Example 9: Graceful Degradation
    // ========================================================================
    println!("\n┌─ Example 9: Graceful Degradation ──────────────────────────────┐");
    println!("│ All functions gracefully degrade when pacman is unavailable  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("All query functions are designed to gracefully degrade:");
    println!("  - get_installed_packages(): returns empty set on error");
    println!("  - get_upgradable_packages(): returns empty set on error");
    println!("  - get_installed_version(): returns PackageNotFound error");
    println!("  - get_available_version(): returns None on error");
    println!("  - is_package_installed_or_provided(): returns false on error");
    println!("\nThis allows dependency resolution to continue even if pacman");
    println!("is temporarily unavailable or the system is in an unusual state.");

    // ========================================================================
    // Example 10: Performance Considerations
    // ========================================================================
    println!("\n┌─ Example 10: Performance Considerations ─────────────────────┐");
    println!("│ Understanding the performance characteristics of each function│");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("Performance characteristics:");
    println!("\n  Fast operations (single pacman call):");
    println!("    - get_installed_packages(): O(n) where n = installed packages");
    println!("    - get_upgradable_packages(): O(n) where n = upgradable packages");
    println!("    - get_installed_version(pkg): O(1) single package query)");
    println!("    - get_available_version(pkg): O(1) single package query)");
    println!("\n  Lazy operations (on-demand checking):");
    println!("    - get_provided_packages(): O(1) returns empty set immediately");
    println!("    - is_package_installed_or_provided(): O(1) per check");
    println!("      Uses 'pacman -Qqo' which is efficient");
    println!("\n  Best practices:");
    println!("    - Cache get_installed_packages() result if checking multiple packages");
    println!("    - Use is_package_installed_or_provided() instead of building full provided set");
    println!("    - Batch version queries when possible");

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    Ok(())
}
