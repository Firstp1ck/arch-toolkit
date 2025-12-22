//! Comprehensive dependency types usage example for arch-toolkit.
//!
//! This example demonstrates all dependency-related types and operations:
//! - Creating and working with dependency status types
//! - Dependency source determination
//! - Package references for dependency resolution
//! - Parsing dependency specifications
//! - Working with .SRCINFO data structures
//! - Reverse dependency summaries
//! - Display formatting and serialization
//!
//! Note: This example focuses on the types themselves. For actual dependency
//! resolution functionality, see the deps module documentation once implemented.

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example deps_types_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(clippy::too_many_lines)] // Example file - comprehensive demonstration
fn main() {
    use arch_toolkit::{
        Dependency, DependencySource, DependencySpec, DependencyStatus, PackageRef, PackageSource,
        ReverseDependencySummary, SrcinfoData,
    };

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Dependency Types Example                   ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: DependencyStatus Enum
    // ========================================================================
    println!("┌─ Example 1: DependencyStatus Enum ────────────────────────────┐");
    println!("│ Working with dependency status variants                        │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let statuses = vec![
        DependencyStatus::Installed {
            version: "2.35-1".to_string(),
        },
        DependencyStatus::ToInstall,
        DependencyStatus::ToUpgrade {
            current: "1.0.0".to_string(),
            required: "2.0.0".to_string(),
        },
        DependencyStatus::Conflict {
            reason: "conflicts with installed package 'old-lib'".to_string(),
        },
        DependencyStatus::Missing,
    ];

    println!("All dependency status variants:\n");
    for (i, status) in statuses.iter().enumerate() {
        println!("  {}. {}", i + 1, status);
        println!("     Priority: {}", status.priority());
        println!("     Is installed: {}", status.is_installed());
        println!("     Needs action: {}", status.needs_action());
        println!("     Is conflict: {}", status.is_conflict());
        println!();
    }

    // Demonstrate priority ordering
    println!("Priority ordering (lower = more urgent):");
    let mut sorted_statuses = statuses;
    sorted_statuses.sort_by_key(DependencyStatus::priority);
    for status in sorted_statuses {
        println!("  [{}] {}", status.priority(), status);
    }
    println!();

    // ========================================================================
    // Example 2: DependencySource Enum
    // ========================================================================
    println!("┌─ Example 2: DependencySource Enum ───────────────────────────┐");
    println!("│ Determining where dependencies come from                       │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let sources = vec![
        DependencySource::Official {
            repo: "core".to_string(),
        },
        DependencySource::Official {
            repo: "extra".to_string(),
        },
        DependencySource::Aur,
        DependencySource::Local,
    ];

    println!("Dependency sources:\n");
    for source in &sources {
        println!("  • {source}");
    }
    println!();

    // ========================================================================
    // Example 3: PackageSource Enum
    // ========================================================================
    println!("┌─ Example 3: PackageSource Enum ───────────────────────────────┐");
    println!("│ Specifying package sources for resolution input                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let package_sources = vec![
        PackageSource::Official {
            repo: "core".to_string(),
            arch: "x86_64".to_string(),
        },
        PackageSource::Official {
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
        },
        PackageSource::Aur,
    ];

    println!("Package sources for resolution:\n");
    for source in &package_sources {
        println!("  • {source}");
    }
    println!();

    // ========================================================================
    // Example 4: DependencySpec - Parsing Dependency Strings
    // ========================================================================
    println!("┌─ Example 4: DependencySpec - Parsing ──────────────────────────┐");
    println!("│ Creating dependency specifications from strings                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let dep_strings = vec![
        "glibc",
        "python>=3.12",
        "firefox=121.0",
        "gcc<13",
        "rust>=1.70.0",
    ];

    println!("Parsing dependency strings:\n");
    for dep_str in &dep_strings {
        // In real usage, this would use parse_dep_spec() from the deps module
        // For now, demonstrate manual creation
        let spec = dep_str
            .find(['>', '<', '='])
            .map_or_else(
                || DependencySpec::new(dep_str.to_string()),
                |pos| {
                    let (name, version) = dep_str.split_at(pos);
                    DependencySpec::with_version(name.trim(), version.trim())
                },
            );

        println!("  Input:  \"{dep_str}\"");
        println!("  Output: {spec}");
        println!("  Name:   {}", spec.name);
        println!(
            "  Version req: {}",
            if spec.has_version_req() {
                &spec.version_req
            } else {
                "(none)"
            }
        );
        println!();
    }

    // ========================================================================
    // Example 5: Creating Dependency Instances
    // ========================================================================
    println!("┌─ Example 5: Creating Dependency Instances ────────────────────┐");
    println!("│ Building complete dependency information                       │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let dependencies = vec![
        Dependency {
            name: "glibc".to_string(),
            version_req: ">=2.35".to_string(),
            status: DependencyStatus::Installed {
                version: "2.35-1".to_string(),
            },
            source: DependencySource::Official {
                repo: "core".to_string(),
            },
            required_by: vec!["firefox".to_string(), "chromium".to_string()],
            depends_on: vec!["linux-api-headers".to_string()],
            is_core: true,
            is_system: true,
        },
        Dependency {
            name: "python".to_string(),
            version_req: ">=3.12".to_string(),
            status: DependencyStatus::ToInstall,
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["my-python-app".to_string()],
            depends_on: vec!["gcc".to_string(), "make".to_string()],
            is_core: false,
            is_system: false,
        },
        Dependency {
            name: "old-lib".to_string(),
            version_req: String::new(),
            status: DependencyStatus::Conflict {
                reason: "conflicts with new-lib in install list".to_string(),
            },
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["legacy-app".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
    ];

    println!("Example dependencies:\n");
    for dep in &dependencies {
        println!("  📦 {}", dep.name);
        println!("     Status:      {}", dep.status);
        println!("     Source:      {}", dep.source);
        println!(
            "     Version req: {}",
            if dep.version_req.is_empty() {
                "(none)"
            } else {
                &dep.version_req
            }
        );
        println!("     Required by: {}", dep.required_by.join(", "));
        if !dep.depends_on.is_empty() {
            println!("     Depends on:  {}", dep.depends_on.join(", "));
        }
        println!("     Core:        {}", dep.is_core);
        println!("     System:      {}", dep.is_system);
        println!();
    }

    // ========================================================================
    // Example 6: PackageRef - Input for Resolution
    // ========================================================================
    println!("┌─ Example 6: PackageRef - Resolution Input ────────────────────┐");
    println!("│ Creating package references for dependency resolution          │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let package_refs = vec![
        PackageRef {
            name: "firefox".to_string(),
            version: "121.0-1".to_string(),
            source: PackageSource::Official {
                repo: "extra".to_string(),
                arch: "x86_64".to_string(),
            },
        },
        PackageRef {
            name: "yay".to_string(),
            version: "12.3.5-1".to_string(),
            source: PackageSource::Aur,
        },
    ];

    println!("Package references for resolution:\n");
    for pkg_ref in &package_refs {
        println!("  • {} ({})", pkg_ref.name, pkg_ref.version);
        println!("    Source: {}", pkg_ref.source);
    }
    println!();

    // ========================================================================
    // Example 7: SrcinfoData - Parsed .SRCINFO
    // ========================================================================
    println!("┌─ Example 7: SrcinfoData - Parsed .SRCINFO ────────────────────┐");
    println!("│ Working with parsed .SRCINFO file data                        │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let srcinfo = SrcinfoData {
        pkgbase: "my-awesome-package".to_string(),
        pkgname: "my-awesome-package".to_string(),
        pkgver: "1.2.3".to_string(),
        pkgrel: "1".to_string(),
        depends: vec![
            "glibc".to_string(),
            "python>=3.12".to_string(),
            "gcc".to_string(),
        ],
        makedepends: vec!["make".to_string(), "cmake".to_string()],
        checkdepends: vec!["check".to_string()],
        optdepends: vec![
            "optional: optional-feature".to_string(),
            "optional: another-optional".to_string(),
        ],
        conflicts: vec!["old-package".to_string()],
        provides: vec!["my-package".to_string()],
        replaces: vec!["legacy-package".to_string()],
    };

    println!("Parsed .SRCINFO data:\n");
    println!("  Package: {} ({})", srcinfo.pkgname, srcinfo.pkgver);
    println!("  Base:    {}", srcinfo.pkgbase);
    println!("  Release: {}", srcinfo.pkgrel);
    println!();
    println!("  Dependencies ({}):", srcinfo.depends.len());
    for dep in &srcinfo.depends {
        println!("    • {dep}");
    }
    println!();
    println!("  Make Dependencies ({}):", srcinfo.makedepends.len());
    for dep in &srcinfo.makedepends {
        println!("    • {dep}");
    }
    println!();
    println!("  Check Dependencies ({}):", srcinfo.checkdepends.len());
    for dep in &srcinfo.checkdepends {
        println!("    • {dep}");
    }
    println!();
    println!("  Optional Dependencies ({}):", srcinfo.optdepends.len());
    for dep in &srcinfo.optdepends {
        println!("    • {dep}");
    }
    println!();
    if !srcinfo.conflicts.is_empty() {
        println!("  Conflicts ({}):", srcinfo.conflicts.len());
        for conflict in &srcinfo.conflicts {
            println!("    • {conflict}");
        }
        println!();
    }
    if !srcinfo.provides.is_empty() {
        println!("  Provides ({}):", srcinfo.provides.len());
        for provide in &srcinfo.provides {
            println!("    • {provide}");
        }
        println!();
    }
    if !srcinfo.replaces.is_empty() {
        println!("  Replaces ({}):", srcinfo.replaces.len());
        for replace in &srcinfo.replaces {
            println!("    • {replace}");
        }
        println!();
    }

    // ========================================================================
    // Example 8: ReverseDependencySummary
    // ========================================================================
    println!("┌─ Example 8: ReverseDependencySummary ─────────────────────────┐");
    println!("│ Analyzing reverse dependency impact                            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let summaries = vec![
        ReverseDependencySummary {
            package: "qt5-base".to_string(),
            direct_dependents: 15,
            transitive_dependents: 42,
            total_dependents: 57,
        },
        ReverseDependencySummary {
            package: "python".to_string(),
            direct_dependents: 8,
            transitive_dependents: 23,
            total_dependents: 31,
        },
        ReverseDependencySummary {
            package: "glibc".to_string(),
            direct_dependents: 3,
            transitive_dependents: 156,
            total_dependents: 159,
        },
    ];

    println!("Reverse dependency summaries:\n");
    for summary in &summaries {
        println!("  📦 {}", summary.package);
        println!("     Direct dependents:     {}", summary.direct_dependents);
        println!(
            "     Transitive dependents: {}",
            summary.transitive_dependents
        );
        println!("     Total dependents:       {}", summary.total_dependents);
        println!();
    }

    // ========================================================================
    // Example 9: Serialization (JSON)
    // ========================================================================
    println!("┌─ Example 9: Serialization (JSON) ────────────────────────────┐");
    println!("│ Serializing dependency types to JSON                           │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let dep = Dependency {
        name: "example-package".to_string(),
        version_req: ">=1.0.0".to_string(),
        status: DependencyStatus::ToInstall,
        source: DependencySource::Official {
            repo: "extra".to_string(),
        },
        required_by: vec!["parent-package".to_string()],
        depends_on: Vec::new(),
        is_core: false,
        is_system: false,
    };

    match serde_json::to_string_pretty(&dep) {
        Ok(json) => {
            println!("Dependency as JSON:\n");
            println!("{json}");
            println!();
        }
        Err(e) => {
            println!("Serialization error: {e}\n");
        }
    }

    // ========================================================================
    // Example 10: Status Filtering and Analysis
    // ========================================================================
    println!("┌─ Example 10: Status Filtering and Analysis ───────────────────┐");
    println!("│ Filtering and analyzing dependencies by status                 │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let all_deps = [
        Dependency {
            name: "installed-pkg".to_string(),
            version_req: String::new(),
            status: DependencyStatus::Installed {
                version: "1.0.0".to_string(),
            },
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["app1".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
        Dependency {
            name: "to-install-pkg".to_string(),
            version_req: String::new(),
            status: DependencyStatus::ToInstall,
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["app2".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
        Dependency {
            name: "conflict-pkg".to_string(),
            version_req: String::new(),
            status: DependencyStatus::Conflict {
                reason: "test conflict".to_string(),
            },
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["app3".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
    ];

    println!("Dependency analysis:\n");
    println!("  Total dependencies: {}", all_deps.len());
    println!("  Already installed:  {}", all_deps.iter().filter(|d| d.status.is_installed()).count());
    println!("  Need action:        {}", all_deps.iter().filter(|d| d.status.needs_action()).count());
    println!("  Conflicts:          {}", all_deps.iter().filter(|d| d.status.is_conflict()).count());
    println!();

    // ========================================================================
    // Example 11: Priority-Based Sorting
    // ========================================================================
    println!("┌─ Example 11: Priority-Based Sorting ─────────────────────────┐");
    println!("│ Sorting dependencies by urgency (priority)                     │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let mut deps_to_sort = vec![
        Dependency {
            name: "installed".to_string(),
            version_req: String::new(),
            status: DependencyStatus::Installed {
                version: "1.0".to_string(),
            },
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["app".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
        Dependency {
            name: "conflict".to_string(),
            version_req: String::new(),
            status: DependencyStatus::Conflict {
                reason: "test".to_string(),
            },
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["app".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
        Dependency {
            name: "to-install".to_string(),
            version_req: String::new(),
            status: DependencyStatus::ToInstall,
            source: DependencySource::Official {
                repo: "extra".to_string(),
            },
            required_by: vec!["app".to_string()],
            depends_on: Vec::new(),
            is_core: false,
            is_system: false,
        },
    ];

    deps_to_sort.sort_by_key(|d| d.status.priority());

    println!("Dependencies sorted by priority (most urgent first):\n");
    for dep in &deps_to_sort {
        println!(
            "  [{}] {} - {}",
            dep.status.priority(),
            dep.name,
            dep.status
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
    println!("  • DependencyStatus enum with all variants and helper methods");
    println!("  • DependencySource and PackageSource enums");
    println!("  • DependencySpec for parsing dependency strings");
    println!("  • Creating complete Dependency instances");
    println!("  • PackageRef for resolution input");
    println!("  • SrcinfoData for parsed .SRCINFO files");
    println!("  • ReverseDependencySummary for impact analysis");
    println!("  • JSON serialization");
    println!("  • Status filtering and analysis");
    println!("  • Priority-based sorting");
}
