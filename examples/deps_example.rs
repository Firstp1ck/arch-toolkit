//! Comprehensive example demonstrating all deps module features.
//!
//! This example shows how to use:
//! - Dependency parsing (specs, SRCINFO, PKGBUILD)
//! - Version comparison
//! - Package querying
//! - Dependency resolution
//! - Reverse dependency analysis
//! - Source determination
//! - AUR integration (if enabled)
//!
//! Run with:
//!   `cargo run --example deps_example --features deps`
//!   `cargo run --example deps_example --features deps,aur`  # For AUR examples

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example deps_example --features deps");
    std::process::exit(1);
}

#[cfg(feature = "deps")]
#[allow(
    clippy::too_many_lines,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args,
    clippy::cognitive_complexity,
    clippy::single_match
)]
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::deps::{
        DependencyResolver, ReverseDependencyAnalyzer, batch_fetch_official_deps, compare_versions,
        determine_dependency_source, determine_status, extract_major_component,
        fetch_package_conflicts, get_available_version, get_installed_packages,
        get_installed_required_by, get_installed_version, get_provided_packages,
        get_upgradable_packages, has_installed_required_by, is_major_version_bump,
        is_package_installed_or_provided, is_system_package, parse_dep_spec,
        parse_pacman_si_conflicts, parse_pacman_si_deps, parse_pkgbuild_conflicts,
        parse_pkgbuild_deps, parse_srcinfo, parse_srcinfo_conflicts, parse_srcinfo_deps,
        version_satisfies,
    };
    use arch_toolkit::types::dependency::{DependencySpec, ResolverConfig};
    use arch_toolkit::{PackageRef, PackageSource};
    println!("=== Arch Toolkit Deps Module Examples ===\n");

    // Example 1: Parse dependency specifications
    println!("1. Parsing Dependency Specifications");
    println!("--------------------------------------");
    let spec = parse_dep_spec("python>=3.12");
    println!(
        "  Parsed: name={}, version_req={}",
        spec.name, spec.version_req
    );

    let spec2 = parse_dep_spec("glibc");
    println!(
        "  Parsed: name={}, version_req={}",
        spec2.name, spec2.version_req
    );
    println!();

    // Example 1b: DependencySpec constructors
    println!("1b. DependencySpec Constructors");
    println!("--------------------------------");
    let spec_no_version = DependencySpec::new("glibc");
    println!(
        "  DependencySpec::new(\"glibc\"): name={}, version_req={}, has_version_req={}",
        spec_no_version.name,
        if spec_no_version.version_req.is_empty() {
            "empty"
        } else {
            &spec_no_version.version_req
        },
        spec_no_version.has_version_req()
    );

    let spec_with_version = DependencySpec::with_version("python", ">=3.12");
    println!(
        "  DependencySpec::with_version(\"python\", \">=3.12\"): name={}, version_req={}, has_version_req={}",
        spec_with_version.name,
        spec_with_version.version_req,
        spec_with_version.has_version_req()
    );
    println!("  Display format: {}", spec_with_version);
    println!();

    // Example 2: Parse PKGBUILD dependencies
    println!("2. Parsing PKGBUILD Dependencies");
    println!("----------------------------------");
    let pkgbuild = r"
pkgname=test-package
pkgver=1.0.0
depends=('glibc' 'python>=3.10' 'rust>=1.70')
makedepends=('make' 'gcc')
checkdepends=('check')
optdepends=('optional: optional-package')
";
    let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);
    println!("  Depends: {:?}", depends);
    println!("  Makedepends: {:?}", makedepends);
    println!("  Checkdepends: {:?}", checkdepends);
    println!("  Optdepends: {:?}", optdepends);
    println!();

    // Example 3: Parse .SRCINFO
    println!("3. Parsing .SRCINFO");
    println!("-------------------");
    let srcinfo = r"
pkgbase = test-package
pkgname = test-package
pkgver = 1.0.0
pkgrel = 1
depends = glibc
depends = python>=3.12
makedepends = make
conflicts = conflicting-pkg
";
    let (depends, makedepends, _checkdepends, _optdepends) = parse_srcinfo_deps(srcinfo);
    println!("  Depends: {:?}", depends);
    println!("  Makedepends: {:?}", makedepends);

    let data = parse_srcinfo(srcinfo);
    println!(
        "  Full .SRCINFO: pkgbase={}, pkgname={}, pkgver={}",
        data.pkgbase, data.pkgname, data.pkgver
    );
    println!();

    // Example 4: Version comparison
    println!("4. Version Comparison");
    println!("----------------------");
    println!(
        "  compare_versions(\"1.2.3\", \"1.2.4\"): {:?}",
        compare_versions("1.2.3", "1.2.4")
    );
    println!(
        "  compare_versions(\"2.0.0\", \"1.9.9\"): {:?}",
        compare_versions("2.0.0", "1.9.9")
    );
    println!(
        "  compare_versions(\"1.2.3\", \"1.2.3alpha\"): {:?}",
        compare_versions("1.2.3", "1.2.3alpha")
    );
    println!(
        "  compare_versions(\"1.2.3-1\", \"1.2.3-2\"): {:?} (pkgrel normalized)",
        compare_versions("1.2.3-1", "1.2.3-2")
    );
    println!(
        "  version_satisfies(\"2.0\", \">=1.5\"): {}",
        version_satisfies("2.0", ">=1.5")
    );
    println!(
        "  version_satisfies(\"1.0\", \">=1.5\"): {}",
        version_satisfies("1.0", ">=1.5")
    );
    println!(
        "  version_satisfies(\"1.5\", \"=1.5\"): {}",
        version_satisfies("1.5", "=1.5")
    );
    println!(
        "  version_satisfies(\"1.4\", \"<1.5\"): {}",
        version_satisfies("1.4", "<1.5")
    );
    println!(
        "  version_satisfies(\"1.6\", \">1.5\"): {}",
        version_satisfies("1.6", ">1.5")
    );
    println!(
        "  extract_major_component(\"2.0.0\"): {:?}",
        extract_major_component("2.0.0")
    );
    println!(
        "  extract_major_component(\"10.5.2\"): {:?}",
        extract_major_component("10.5.2")
    );
    println!(
        "  extract_major_component(\"1.2.3-alpha\"): {:?}",
        extract_major_component("1.2.3-alpha")
    );
    println!(
        "  is_major_version_bump(\"1.2.3\", \"2.0.0\"): {}",
        is_major_version_bump("1.2.3", "2.0.0")
    );
    println!(
        "  is_major_version_bump(\"1.2.3\", \"1.3.0\"): {}",
        is_major_version_bump("1.2.3", "1.3.0")
    );
    println!(
        "  is_major_version_bump(\"1.2.3-1\", \"2.0.0-1\"): {}",
        is_major_version_bump("1.2.3-1", "2.0.0-1")
    );
    println!();

    // Example 5: Package querying
    println!("5. Package Querying");
    println!("--------------------");
    match get_installed_packages() {
        Ok(installed) => {
            println!("  Found {} installed packages", installed.len());
            if installed.contains("pacman") {
                println!("  pacman is installed");
            }
        }
        Err(e) => println!("  Error getting installed packages: {}", e),
    }

    match get_upgradable_packages() {
        Ok(upgradable) => {
            println!("  Found {} upgradable packages", upgradable.len());
        }
        Err(e) => println!("  Error getting upgradable packages: {}", e),
    }

    match get_installed_version("pacman") {
        Ok(version) => println!("  Installed pacman version: {}", version),
        Err(e) => println!("  Error getting installed version: {}", e),
    }

    if let Some(version) = get_available_version("pacman") {
        println!("  Available pacman version: {}", version);
    }

    if let Ok(installed) = get_installed_packages() {
        let provided = get_provided_packages(&installed);
        println!(
            "  Provided packages set size: {} (lazy checking enabled)",
            provided.len()
        );
        println!(
            "  is_package_installed_or_provided(\"pacman\", ...): {}",
            is_package_installed_or_provided("pacman", &installed, &provided)
        );
        if installed.contains("glibc") {
            println!(
                "  is_package_installed_or_provided(\"glibc\", ...): {}",
                is_package_installed_or_provided("glibc", &installed, &provided)
            );
        }
    }
    println!();

    // Example 6: Source determination
    println!("6. Source Determination");
    println!("------------------------");
    match get_installed_packages() {
        Ok(installed) => {
            // Using bash instead of glibc for faster execution (fewer dependents)
            let (source, is_core) = determine_dependency_source("bash", &installed);
            println!("  bash source: {:?}, is_core: {}", source, is_core);

            if is_system_package("bash") {
                println!("  bash is a critical system package");
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Example 7: Dependency resolution
    println!("7. Dependency Resolution");
    println!("-------------------------");
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
            println!("  Found {} dependencies", result.dependencies.len());
            println!("  Found {} conflicts", result.conflicts.len());
            println!("  Found {} missing packages", result.missing.len());
            println!("\n  Sample dependencies with full details:");
            for dep in result.dependencies.iter().take(3) {
                println!("    Package: {}", dep.name);
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
                println!("      Is core: {}", dep.is_core);
                println!("      Is system: {}", dep.is_system);
                println!();
            }
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }

    // Example 7b: Dependency resolver with custom configuration
    println!("\n7b. Dependency Resolver with Custom Configuration");
    println!("---------------------------------------------------");

    // Config 1: Include optional dependencies
    let config1 = ResolverConfig {
        include_optdepends: true,
        include_makedepends: false,
        include_checkdepends: false,
        max_depth: 0,
        pkgbuild_cache: None,
        check_aur: false,
    };
    let resolver1 = DependencyResolver::with_config(config1);
    let packages = vec![PackageRef {
        name: "firefox".to_string(),
        version: "121.0".to_string(),
        source: PackageSource::Official {
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
        },
    }];

    match resolver1.resolve(&packages) {
        Ok(result) => {
            println!("  Config 1: optdepends included, makedepends excluded");
            println!("  Found {} dependencies", result.dependencies.len());
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }

    // Config 2: Include make dependencies
    let config2 = ResolverConfig {
        include_optdepends: false,
        include_makedepends: true,
        include_checkdepends: false,
        max_depth: 0,
        pkgbuild_cache: None,
        check_aur: false,
    };
    let resolver2 = DependencyResolver::with_config(config2);
    match resolver2.resolve(&packages) {
        Ok(result) => {
            println!("  Config 2: makedepends included");
            println!("  Found {} dependencies", result.dependencies.len());
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }

    // Config 3: Include check dependencies
    let config3 = ResolverConfig {
        include_optdepends: false,
        include_makedepends: false,
        include_checkdepends: true,
        max_depth: 0,
        pkgbuild_cache: None,
        check_aur: false,
    };
    let resolver3 = DependencyResolver::with_config(config3);
    match resolver3.resolve(&packages) {
        Ok(result) => {
            println!("  Config 3: checkdepends included");
            println!("  Found {} dependencies", result.dependencies.len());
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }

    // Config 4: All dependency types included
    let config4 = ResolverConfig {
        include_optdepends: true,
        include_makedepends: true,
        include_checkdepends: true,
        max_depth: 0,
        pkgbuild_cache: None,
        check_aur: false,
    };
    let resolver4 = DependencyResolver::with_config(config4);
    match resolver4.resolve(&packages) {
        Ok(result) => {
            println!("  Config 4: all dependency types included (opt, make, check)");
            println!("  Found {} dependencies", result.dependencies.len());
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }

    // Config 5: With max_depth (note: currently only direct deps are resolved)
    let config5 = ResolverConfig {
        include_optdepends: false,
        include_makedepends: false,
        include_checkdepends: false,
        max_depth: 1, // Would be used for transitive deps if implemented
        pkgbuild_cache: None,
        check_aur: false,
    };
    let resolver5 = DependencyResolver::with_config(config5);
    match resolver5.resolve(&packages) {
        Ok(result) => {
            println!("  Config 5: max_depth=1 (currently resolves direct deps only)");
            println!("  Found {} dependencies", result.dependencies.len());
        }
        Err(e) => println!("  Error resolving dependencies: {}", e),
    }

    // Example 7c: Batch fetching dependencies
    println!("\n7c. Batch Fetching Dependencies");
    println!("-------------------------------");
    // Using packages with fewer dependents for faster execution
    let package_names = ["pacman", "bash", "vim"];
    let batch_deps = batch_fetch_official_deps(&package_names);
    println!(
        "  Batch fetched dependencies for {} packages:",
        package_names.len()
    );
    for (pkg_name, deps) in &batch_deps {
        println!("    {}: {} dependencies", pkg_name, deps.len());
        for dep in deps.iter().take(3) {
            println!("      - {}", dep);
        }
        if deps.len() > 3 {
            println!("      ... and {} more", deps.len() - 3);
        }
    }
    println!();

    // Example 7d: Determine dependency status
    println!("7d. Determine Dependency Status");
    println!("--------------------------------");
    match get_installed_packages() {
        Ok(installed) => {
            let provided = get_provided_packages(&installed);
            let upgradable = get_upgradable_packages().unwrap_or_default();

            // Using packages with fewer dependents for faster execution
            let test_cases = vec![
                ("bash", ""),
                ("nonexistent-package", ""),
                ("python", ">=3.10"),
            ];

            for (name, version_req) in test_cases {
                let status =
                    determine_status(name, version_req, &installed, &provided, &upgradable);
                println!(
                    "  {} (req: {}): {:?}",
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

    // Example 8: Reverse dependency analysis
    println!("8. Reverse Dependency Analysis");
    println!("-------------------------------");
    let analyzer = ReverseDependencyAnalyzer::new();
    // Using wget instead of glibc/bash for faster execution (fewer dependents, typically < 10)
    match get_installed_packages() {
        Ok(installed) if installed.contains("wget") => {
            let packages = vec![PackageRef {
                name: "wget".to_string(),
                version: "1.21.4".to_string(),
                source: PackageSource::Official {
                    repo: "core".to_string(),
                    arch: "x86_64".to_string(),
                },
            }];

            match analyzer.analyze(&packages) {
                Ok(report) => {
                    println!("  Found {} dependents", report.dependents.len());
                    println!("  Found {} summaries", report.summaries.len());
                    println!("\n  Summary statistics:");
                    for summary in report.summaries.iter().take(3) {
                        println!(
                            "    {}: {} direct, {} transitive, {} total",
                            summary.package,
                            summary.direct_dependents,
                            summary.transitive_dependents,
                            summary.total_dependents
                        );
                    }
                    println!("\n  Sample dependents with full details:");
                    for dep in report.dependents.iter().take(3) {
                        println!("    Package: {}", dep.name);
                        println!("      Status: {:?}", dep.status);
                        println!("      Source: {:?}", dep.source);
                        println!("      Required by: {} package(s)", dep.required_by.len());
                        if !dep.required_by.is_empty() {
                            println!("        {}", dep.required_by.join(", "));
                        }
                        println!("      Depends on: {} package(s)", dep.depends_on.len());
                        println!("      Is core: {}", dep.is_core);
                        println!("      Is system: {}", dep.is_system);
                        println!();
                    }
                }
                Err(e) => println!("  Error analyzing reverse dependencies: {}", e),
            }
        }
        _ => println!("  Skipping (wget not installed or error getting installed packages)"),
    }
    println!();

    // Example 9: Helper functions
    println!("9. Helper Functions");
    println!("-------------------");
    // Using wget instead of glibc/bash for faster execution (fewer dependents, typically < 10)
    match get_installed_packages() {
        Ok(installed) if installed.contains("wget") => {
            if has_installed_required_by("wget") {
                println!("  wget has installed dependents");
                let dependents = get_installed_required_by("wget");
                println!("  Found {} installed dependents", dependents.len());
                for dep in dependents.iter().take(5) {
                    println!("    {}", dep);
                }
            } else {
                println!("  wget has no installed dependents");
            }
        }
        _ => println!("  Skipping (wget not installed)"),
    }
    println!();

    // Example 10: Parse pacman output
    println!("10. Parsing Pacman Output");
    println!("--------------------------");
    let pacman_output = r"
Name            : firefox
Version         : 121.0-1
Depends On      : glibc gtk3 nss
Conflicts With  : firefox-esr
";
    let deps = parse_pacman_si_deps(pacman_output);
    let conflicts = parse_pacman_si_conflicts(pacman_output);
    println!("  Dependencies: {:?}", deps);
    println!("  Conflicts: {:?}", conflicts);
    println!();

    // Example 11: Parse PKGBUILD conflicts
    println!("11. Parsing PKGBUILD Conflicts");
    println!("-------------------------------");
    let pkgbuild_with_conflicts = r"
pkgname=test-package
conflicts=('old-package' 'conflicting-package')
";
    let conflicts = parse_pkgbuild_conflicts(pkgbuild_with_conflicts);
    println!("  Conflicts: {:?}", conflicts);
    println!();

    // Example 12: Parse .SRCINFO conflicts
    println!("12. Parsing .SRCINFO Conflicts");
    println!("-------------------------------");
    let srcinfo_with_conflicts = r"
pkgname = test-package
conflicts = old-package
conflicts = conflicting-package
";
    let conflicts = parse_srcinfo_conflicts(srcinfo_with_conflicts);
    println!("  Conflicts: {:?}", conflicts);
    println!();

    // Example 13: Fetch package conflicts
    println!("13. Fetching Package Conflicts");
    println!("-------------------------------");
    let conflicts = fetch_package_conflicts(
        "firefox",
        &PackageSource::Official {
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
        },
    );
    println!("  Found {} conflicts for firefox", conflicts.len());
    println!();

    #[cfg(feature = "aur")]
    {
        // Example 14: AUR integration (if enabled)
        println!("14. AUR Integration");
        println!("--------------------");
        println!("  AUR feature is enabled!");
        println!("  Use fetch_srcinfo() to fetch .SRCINFO from AUR");
        println!("  Example: use reqwest::Client;");
        println!("           let client = Client::new();");
        println!("           let srcinfo = fetch_srcinfo(&client, \"yay\").await?;");
        println!();
    }

    #[cfg(not(feature = "aur"))]
    {
        println!("14. AUR Integration");
        println!("--------------------");
        println!("  AUR feature is not enabled");
        println!("  Enable with: cargo run --example deps_example --features \"deps,aur\"");
        println!();
    }

    println!("=== Examples Complete ===");
    Ok(())
}
