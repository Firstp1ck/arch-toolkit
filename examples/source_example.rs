//! Comprehensive source determination example for arch-toolkit.
//!
//! This example demonstrates:
//! - Determining package source (official repository, AUR, or local)
//! - Identifying critical system packages
//! - Handling installed vs uninstalled packages
//! - Graceful degradation when pacman is unavailable
//!
//! Run with:
//!   `cargo run --example source_example --features deps`

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example source_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // Example file - comprehensive demonstration
fn main() {
    use arch_toolkit::deps::get_installed_packages;
    use arch_toolkit::deps::{determine_dependency_source, is_system_package};

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Source Determination Example               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Determine Source for Installed Packages
    // ========================================================================
    println!("┌─ Example 1: Determine Source for Installed Packages ───────────┐");
    println!("│ Check which repository installed packages come from           │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let test_packages = ["pacman", "glibc", "bash", "firefox", "systemd"];

            println!("Checking source for installed packages:");
            for pkg_name in &test_packages {
                if installed.contains(*pkg_name) {
                    let (source, is_core) = determine_dependency_source(pkg_name, &installed);
                    println!("\n  Package: {pkg_name}");
                    println!("    Source: {source:?}");
                    println!("    Is Core Repository: {is_core}");
                    match source {
                        arch_toolkit::types::dependency::DependencySource::Official { repo } => {
                            println!("    Repository: {repo}");
                        }
                        arch_toolkit::types::dependency::DependencySource::Aur => {
                            println!("    Repository: AUR");
                        }
                        arch_toolkit::types::dependency::DependencySource::Local => {
                            println!("    Repository: Local (not in repos)");
                        }
                    }
                } else {
                    println!("\n  Package: {pkg_name} (not installed)");
                }
            }
        }
        Err(e) => {
            println!("Error getting installed packages: {e}");
        }
    }

    // ========================================================================
    // Example 2: Determine Source for Uninstalled Packages
    // ========================================================================
    println!("\n┌─ Example 2: Determine Source for Uninstalled Packages ───────┐");
    println!("│ Check which repository uninstalled packages would come from   │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let test_packages = [
                "vim",         // Usually in official repos
                "yay",         // AUR package
                "nonexistent", // Not in repos or AUR
            ];

            println!("Checking source for uninstalled packages:");
            for pkg_name in &test_packages {
                if installed.contains(*pkg_name) {
                    println!("\n  Package: {pkg_name} (already installed)");
                } else {
                    let (source, is_core) = determine_dependency_source(pkg_name, &installed);
                    println!("\n  Package: {pkg_name}");
                    println!("    Source: {source:?}");
                    println!("    Is Core Repository: {is_core}");
                    match source {
                        arch_toolkit::types::dependency::DependencySource::Official { repo } => {
                            println!("    Repository: {repo} (found in official repos)");
                        }
                        arch_toolkit::types::dependency::DependencySource::Aur => {
                            println!("    Repository: AUR (or not found in official repos)");
                            println!("    Note: May be AUR package, binary/script, or missing");
                        }
                        arch_toolkit::types::dependency::DependencySource::Local => {
                            println!("    Repository: Local (not in repos)");
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error getting installed packages: {e}");
        }
    }

    // ========================================================================
    // Example 3: Core Repository Detection
    // ========================================================================
    println!("\n┌─ Example 3: Core Repository Detection ────────────────────────┐");
    println!("│ Identify packages from the core repository                    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let core_packages = ["pacman", "glibc", "bash", "systemd", "linux"];

            println!("Checking if packages are from core repository:");
            for pkg_name in &core_packages {
                if installed.contains(*pkg_name) {
                    let (_source, is_core) = determine_dependency_source(pkg_name, &installed);
                    println!("\n  Package: {pkg_name}");
                    println!("    Is Core: {is_core}");
                    if is_core {
                        println!("    ✓ This is a core repository package");
                    } else {
                        println!("    → This is not a core repository package");
                    }
                }
            }
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    // ========================================================================
    // Example 4: System Package Detection
    // ========================================================================
    println!("\n┌─ Example 4: System Package Detection ────────────────────────┐");
    println!("│ Identify critical system packages                            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let test_packages = [
        "glibc",   // Critical system package
        "pacman",  // Package manager (critical)
        "systemd", // Init system (critical)
        "bash",    // Shell (critical)
        "firefox", // Application (not critical)
        "vim",     // Editor (not critical)
    ];

    println!("Checking if packages are critical system packages:");
    for pkg_name in &test_packages {
        let is_system = is_system_package(pkg_name);
        println!("\n  Package: {pkg_name}");
        if is_system {
            println!("    ✓ Critical system package");
            println!("    Warning: Removing this package may break the system!");
        } else {
            println!("    → Not a critical system package");
        }
    }

    println!("\nNote: System packages are identified by:");
    println!("  - Being in the 'base' or 'base-devel' groups");
    println!("  - Being in the 'core' repository");
    println!("  - Being essential system components");

    // ========================================================================
    // Example 5: Local Package Detection
    // ========================================================================
    println!("\n┌─ Example 5: Local Package Detection ──────────────────────────┐");
    println!("│ Identify locally installed packages (not from repos)         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            println!("Checking for local packages:");
            println!("(Packages installed manually, not from official repos or AUR)");

            let mut local_count = 0;
            let mut official_count = 0;
            let mut aur_count = 0;

            // Sample a subset of installed packages to check
            for pkg_name in installed.iter().take(20) {
                let (source, _) = determine_dependency_source(pkg_name, &installed);
                match source {
                    arch_toolkit::types::dependency::DependencySource::Local => {
                        local_count += 1;
                        if local_count <= 3 {
                            println!("  - {pkg_name}: Local package");
                        }
                    }
                    arch_toolkit::types::dependency::DependencySource::Official { .. } => {
                        official_count += 1;
                    }
                    arch_toolkit::types::dependency::DependencySource::Aur => {
                        aur_count += 1;
                    }
                }
            }

            println!("\nSample statistics (first 20 packages):");
            println!("  Official repository: {official_count} packages");
            println!("  AUR: {aur_count} packages");
            println!("  Local: {local_count} packages");
            println!("\nNote: This is a sample - check all packages for full statistics");
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    // ========================================================================
    // Example 6: Repository-Specific Detection
    // ========================================================================
    println!("\n┌─ Example 6: Repository-Specific Detection ──────────────────┐");
    println!("│ Identify which specific repository packages come from        │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let test_packages = ["pacman", "glibc", "firefox", "vim"];

            println!("Checking specific repositories:");
            for pkg_name in &test_packages {
                if installed.contains(*pkg_name) {
                    let (source, is_core) = determine_dependency_source(pkg_name, &installed);
                    println!("\n  Package: {pkg_name}");
                    match source {
                        arch_toolkit::types::dependency::DependencySource::Official { repo } => {
                            println!("    Repository: {repo}");
                            if is_core {
                                println!("    Type: Core repository (essential system packages)");
                            } else {
                                println!("    Type: Official repository (extra, community, etc.)");
                            }
                        }
                        arch_toolkit::types::dependency::DependencySource::Aur => {
                            println!("    Repository: AUR (Arch User Repository)");
                        }
                        arch_toolkit::types::dependency::DependencySource::Local => {
                            println!("    Repository: Local (manually installed)");
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    // ========================================================================
    // Example 7: Real-World Use Case - Dependency Source Analysis
    // ========================================================================
    println!("\n┌─ Example 7: Real-World Use Case - Dependency Source Analysis ┐");
    println!("│ Analyze sources for a set of packages                        │");
    println!("└──────────────────────────────────────────────────────────────┘");

    match get_installed_packages() {
        Ok(installed) => {
            let packages_to_analyze = ["pacman", "glibc", "bash", "firefox", "systemd"];

            println!("Analyzing package sources:");
            let mut core_count = 0;
            let mut official_count = 0;
            let mut aur_count = 0;
            let mut local_count = 0;
            let mut system_count = 0;

            for pkg_name in &packages_to_analyze {
                if installed.contains(*pkg_name) {
                    let (source, is_core) = determine_dependency_source(pkg_name, &installed);
                    let is_system = is_system_package(pkg_name);

                    if is_core {
                        core_count += 1;
                    }
                    match source {
                        arch_toolkit::types::dependency::DependencySource::Official { .. } => {
                            official_count += 1;
                        }
                        arch_toolkit::types::dependency::DependencySource::Aur => {
                            aur_count += 1;
                        }
                        arch_toolkit::types::dependency::DependencySource::Local => {
                            local_count += 1;
                        }
                    }
                    if is_system {
                        system_count += 1;
                    }
                }
            }

            println!("\nSummary:");
            println!("  Total packages analyzed: {}", packages_to_analyze.len());
            println!("  Core repository: {core_count}");
            println!("  Official repositories: {official_count}");
            println!("  AUR: {aur_count}");
            println!("  Local: {local_count}");
            println!("  Critical system packages: {system_count}");
        }
        Err(e) => {
            println!("Error: {e}");
        }
    }

    // ========================================================================
    // Example 8: Graceful Degradation
    // ========================================================================
    println!("\n┌─ Example 8: Graceful Degradation ────────────────────────────┐");
    println!("│ Functions gracefully degrade when pacman is unavailable      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("Both functions are designed to gracefully degrade:");
    println!("\n  determine_dependency_source():");
    println!("    - Returns reasonable defaults when pacman is unavailable");
    println!("    - For installed packages: uses 'pacman -Qi' to read repository");
    println!("    - For uninstalled packages: uses 'pacman -Si' to check repos");
    println!("    - Falls back to AUR if not found in official repos");
    println!("\n  is_system_package():");
    println!("    - Returns false when pacman is unavailable");
    println!("    - Uses 'pacman -Si' to check package groups and repository");
    println!("    - Checks for 'base' or 'base-devel' groups");
    println!("\nThis allows dependency resolution to continue even if pacman");
    println!("is temporarily unavailable or the system is in an unusual state.");

    // ========================================================================
    // Example 9: Understanding Source Types
    // ========================================================================
    println!("\n┌─ Example 9: Understanding Source Types ────────────────────────┐");
    println!("│ Explanation of different dependency source types            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("DependencySource enum variants:");
    println!("\n  1. Official {{ repo: String }}");
    println!("     - Package from official Arch Linux repositories");
    println!("     - Repositories: core, extra, community, multilib, etc.");
    println!("     - Examples: pacman (core), firefox (extra), vim (extra)");
    println!("\n  2. Aur");
    println!("     - Package from Arch User Repository (AUR)");
    println!("     - Community-maintained packages");
    println!("     - Examples: yay, paru, pacsea");
    println!("     - Note: Also used as fallback for packages not found in repos");
    println!("\n  3. Local");
    println!("     - Package installed manually (not from repos)");
    println!("     - Installed via 'pacman -U' or similar");
    println!("     - Examples: Custom-built packages, packages from other sources");
    println!("\nWhen determining source:");
    println!("  - Installed packages: Check 'pacman -Qi' repository field");
    println!("  - Uninstalled packages: Check 'pacman -Si' to see if in repos");
    println!("  - If not in repos: Default to AUR (may need further verification)");

    // ========================================================================
    // Example 10: Best Practices
    // ========================================================================
    println!("\n┌─ Example 10: Best Practices ──────────────────────────────────┐");
    println!("│ Best practices for using source determination functions      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("Best practices:");
    println!("\n  1. Cache installed packages list:");
    println!("     - Call get_installed_packages() once and reuse");
    println!("     - Pass the same HashSet to multiple determine_dependency_source() calls");
    println!("\n  2. Check system packages before removal:");
    println!("     - Always check is_system_package() before removing packages");
    println!("     - Warn users about removing critical system packages");
    println!("\n  3. Handle AUR fallback:");
    println!("     - AUR source may indicate package not found in repos");
    println!("     - Verify with AUR API if you need to distinguish");
    println!("\n  4. Performance considerations:");
    println!("     - determine_dependency_source() makes pacman calls");
    println!("     - Batch operations when possible");
    println!("     - Cache results if checking same packages multiple times");

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}
