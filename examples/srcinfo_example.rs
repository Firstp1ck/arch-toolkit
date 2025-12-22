//! Comprehensive .SRCINFO parsing example for arch-toolkit.
//!
//! This example demonstrates:
//! - Parsing .SRCINFO content into structured data
//! - Extracting dependencies, conflicts, and other fields
//! - Handling architecture-specific dependencies
//! - Fetching .SRCINFO from AUR (requires `aur` feature)
//! - Working with split packages
//!
//! Run with:
//!   `cargo run --example srcinfo_example --features deps`
//!   `cargo run --example srcinfo_example --features deps,aur`  # For fetch example

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example srcinfo_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)] // Example file - comprehensive demonstration
fn main() {
    use arch_toolkit::deps::{parse_srcinfo, parse_srcinfo_conflicts, parse_srcinfo_deps};

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: .SRCINFO Parsing Example                   ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic .SRCINFO Parsing
    // ========================================================================
    println!("┌─ Example 1: Basic .SRCINFO Parsing ───────────────────────────┐");
    println!("│ Parse a simple .SRCINFO file into structured data            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let srcinfo_content = r"
pkgbase = example-package
pkgname = example-package
pkgver = 1.2.3
pkgrel = 1
depends = glibc
depends = python>=3.10
makedepends = make
makedepends = gcc
checkdepends = check
optdepends = optional: optional-feature
conflicts = old-package
provides = example
replaces = legacy-package
";

    let data = parse_srcinfo(srcinfo_content);

    println!("Package Base: {}", data.pkgbase);
    println!("Package Name: {}", data.pkgname);
    println!("Version: {}-{}", data.pkgver, data.pkgrel);
    println!("\nDependencies:");
    for dep in &data.depends {
        println!("  - {dep}");
    }
    println!("\nMake Dependencies:");
    for dep in &data.makedepends {
        println!("  - {dep}");
    }
    println!("\nCheck Dependencies:");
    for dep in &data.checkdepends {
        println!("  - {dep}");
    }
    println!("\nOptional Dependencies:");
    for dep in &data.optdepends {
        println!("  - {dep}");
    }
    println!("\nConflicts:");
    for conflict in &data.conflicts {
        println!("  - {conflict}");
    }
    println!("\nProvides:");
    for provide in &data.provides {
        println!("  - {provide}");
    }
    println!("\nReplaces:");
    for replace in &data.replaces {
        println!("  - {replace}");
    }

    // ========================================================================
    // Example 2: Parsing Dependencies Only
    // ========================================================================
    println!("\n┌─ Example 2: Parsing Dependencies Only ────────────────────────┐");
    println!("│ Extract just dependency arrays without full parsing          │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let (depends, makedepends, checkdepends, optdepends) = parse_srcinfo_deps(srcinfo_content);

    println!("Runtime Dependencies: {} items", depends.len());
    println!("  {depends:?}");
    println!("\nMake Dependencies: {} items", makedepends.len());
    println!("  {makedepends:?}");
    println!("\nCheck Dependencies: {} items", checkdepends.len());
    println!("  {checkdepends:?}");
    println!("\nOptional Dependencies: {} items", optdepends.len());
    println!("  {optdepends:?}");

    // ========================================================================
    // Example 3: Parsing Conflicts Only
    // ========================================================================
    println!("\n┌─ Example 3: Parsing Conflicts Only ─────────────────────────┐");
    println!("│ Extract conflict specifications with version extraction      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let conflicts_content = r"
conflicts = old-package
conflicts = conflicting-pkg>=2.0
conflicts = another-conflict<1.5
conflicts = libfoo.so=1-64
";

    let conflicts = parse_srcinfo_conflicts(conflicts_content);
    println!("Conflicts (with .so filtering and version extraction):");
    for conflict in &conflicts {
        println!("  - {conflict}");
    }
    println!("\nNote: .so virtual packages are filtered out");
    println!("Note: Version constraints are removed from conflict names");

    // ========================================================================
    // Example 4: Architecture-Specific Dependencies
    // ========================================================================
    println!("\n┌─ Example 4: Architecture-Specific Dependencies ──────────────┐");
    println!("│ Handle architecture-specific dependency fields                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let arch_specific_content = r"
pkgbase = multiarch-package
pkgname = multiarch-package
pkgver = 1.0.0
pkgrel = 1
depends = common-dependency
depends_x86_64 = x86-specific-lib
depends_aarch64 = arm-specific-lib
makedepends = build-tool
makedepends_x86_64 = x86-build-tool
";

    let arch_data = parse_srcinfo(arch_specific_content);
    println!("All Dependencies (merged from all architectures):");
    for dep in &arch_data.depends {
        println!("  - {dep}");
    }
    println!("\nAll Make Dependencies (merged from all architectures):");
    for dep in &arch_data.makedepends {
        println!("  - {dep}");
    }
    println!("\nNote: Architecture-specific dependencies are merged into main arrays");

    // ========================================================================
    // Example 5: Split Packages
    // ========================================================================
    println!("\n┌─ Example 5: Split Packages ────────────────────────────────────┐");
    println!("│ Handle packages with multiple pkgname entries                 │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let split_package_content = r"
pkgbase = split-package
pkgname = split-package-base
pkgname = split-package-gui
pkgname = split-package-cli
pkgver = 2.0.0
pkgrel = 1
depends = glibc
";

    let split_data = parse_srcinfo(split_package_content);
    println!("Package Base: {}", split_data.pkgbase);
    println!("Package Name (first found): {}", split_data.pkgname);
    println!("\nNote: For split packages, the first pkgname is used");
    println!("Note: Full split package handling would require parsing per-package sections");

    // ========================================================================
    // Example 6: Filtering Virtual Packages (.so files)
    // ========================================================================
    println!("\n┌─ Example 6: Filtering Virtual Packages ──────────────────────┐");
    println!("│ Automatic filtering of .so virtual packages                   │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let virtual_pkg_content = r"
depends = glibc
depends = libedit.so
depends = libgit2.so.1
depends = libfoo.so=1-64
depends = python
";

    let (filtered_deps, _, _, _) = parse_srcinfo_deps(virtual_pkg_content);
    println!("Dependencies after .so filtering:");
    for dep in &filtered_deps {
        println!("  - {dep}");
    }
    println!("\nNote: All .so virtual packages are automatically filtered out");

    // ========================================================================
    // Example 7: Deduplication
    // ========================================================================
    println!("\n┌─ Example 7: Automatic Deduplication ──────────────────────────┐");
    println!("│ Duplicate dependencies are automatically removed              │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let duplicate_content = r"
depends = glibc
depends = gtk3
depends = glibc
depends = nss
depends = gtk3
";

    let (deduped_deps, _, _, _) = parse_srcinfo_deps(duplicate_content);
    println!("Dependencies after deduplication:");
    for dep in &deduped_deps {
        println!("  - {dep}");
    }
    println!("\nTotal unique dependencies: {}", deduped_deps.len());
    println!("Note: Duplicates are automatically removed");

    // ========================================================================
    // Example 8: Comments and Blank Lines
    // ========================================================================
    println!("\n┌─ Example 8: Comments and Blank Lines ─────────────────────────┐");
    println!("│ Comments and blank lines are automatically ignored            │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let commented_content = r"
# This is a comment
pkgbase = commented-package
# Another comment

pkgname = commented-package
pkgver = 1.0.0
# Version comment
pkgrel = 1
depends = glibc
";

    let commented_data = parse_srcinfo(commented_content);
    println!("Parsed data from .SRCINFO with comments:");
    println!("  Package Base: {}", commented_data.pkgbase);
    println!("  Package Name: {}", commented_data.pkgname);
    println!(
        "  Version: {}-{}",
        commented_data.pkgver, commented_data.pkgrel
    );
    println!("  Dependencies: {:?}", commented_data.depends);
    println!("\nNote: All comments (lines starting with #) are ignored");
    println!("Note: Blank lines are also ignored");

    // ========================================================================
    // Example 9: Fetching from AUR (requires `aur` feature)
    // ========================================================================
    #[cfg(feature = "aur")]
    {
        use arch_toolkit::deps::fetch_srcinfo;

        println!("\n┌─ Example 9: Fetching .SRCINFO from AUR ──────────────────────┐");
        println!("│ Fetch and parse .SRCINFO directly from AUR                  │");
        println!("└──────────────────────────────────────────────────────────────┘");

        println!("Fetching .SRCINFO for 'yay' package from AUR...");
        // Create a reqwest client for the example
        match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("arch-toolkit-example")
            .build()
        {
            Ok(client) => {
                // Use tokio runtime for async execution
                match tokio::runtime::Runtime::new() {
                    Ok(rt) => match rt.block_on(fetch_srcinfo(&client, "yay")) {
                        Ok(srcinfo) => {
                            println!("✓ Successfully fetched .SRCINFO");
                            let fetched_data = parse_srcinfo(&srcinfo);
                            println!("\nFetched Package Info:");
                            println!("  Package Base: {}", fetched_data.pkgbase);
                            println!("  Package Name: {}", fetched_data.pkgname);
                            println!("  Version: {}-{}", fetched_data.pkgver, fetched_data.pkgrel);
                            println!("  Dependencies: {} items", fetched_data.depends.len());
                            println!(
                                "  Make Dependencies: {} items",
                                fetched_data.makedepends.len()
                            );
                            println!(
                                "  Optional Dependencies: {} items",
                                fetched_data.optdepends.len()
                            );
                            if !fetched_data.depends.is_empty() {
                                println!("\n  First few dependencies:");
                                for dep in fetched_data.depends.iter().take(5) {
                                    println!("    - {dep}");
                                }
                                if fetched_data.depends.len() > 5 {
                                    println!("    ... and {} more", fetched_data.depends.len() - 5);
                                }
                            }
                        }
                        Err(e) => {
                            println!("✗ Failed to fetch .SRCINFO: {e}");
                            println!(
                                "\nNote: This might be due to network issues or the package not existing."
                            );
                            println!("Example usage code:");
                            println!("  use arch_toolkit::deps::fetch_srcinfo;");
                            println!("  use reqwest::Client;");
                            println!("  ");
                            println!("  let client = Client::new();");
                            println!("  let srcinfo = fetch_srcinfo(&client, \"yay\").await?;");
                            println!("  let data = parse_srcinfo(&srcinfo);");
                        }
                    },
                    Err(e) => {
                        println!("✗ Failed to create async runtime: {e}");
                        println!("\nNote: fetch_srcinfo() requires async runtime");
                        println!("Example usage:");
                        println!("  use arch_toolkit::deps::fetch_srcinfo;");
                        println!("  use reqwest::Client;");
                        println!("  ");
                        println!("  let client = Client::new();");
                        println!("  let srcinfo = fetch_srcinfo(&client, \"yay\").await?;");
                        println!("  let data = parse_srcinfo(&srcinfo);");
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to create HTTP client: {e}");
                println!("\nNote: fetch_srcinfo() requires a reqwest::Client");
                println!("Example usage:");
                println!("  use arch_toolkit::deps::fetch_srcinfo;");
                println!("  use reqwest::Client;");
                println!("  ");
                println!("  let client = Client::new();");
                println!("  let srcinfo = fetch_srcinfo(&client, \"yay\").await?;");
                println!("  let data = parse_srcinfo(&srcinfo);");
            }
        }
    }

    #[cfg(not(feature = "aur"))]
    {
        println!("\n┌─ Example 9: Fetching .SRCINFO from AUR ──────────────────────┐");
        println!("│ (Requires 'aur' feature - not enabled)                      │");
        println!("└──────────────────────────────────────────────────────────────┘");
        println!("To enable AUR fetching, run with:");
        println!("  cargo run --example srcinfo_example --features deps,aur");
    }

    // ========================================================================
    // Example 10: Real-World Package Example
    // ========================================================================
    println!("\n┌─ Example 10: Real-World Package Example ──────────────────────┐");
    println!("│ Parse a realistic .SRCINFO file                                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let real_world_content = r"
pkgbase = yay
pkgname = yay
pkgver = 12.3.4
pkgrel = 1
url = https://github.com/Jguer/yay
arch = x86_64
license = GPL
makedepends = git
makedepends = go
checkdepends = git
depends = pacman
depends = git
depends = sudo
optdepends = pacman-contrib: rankmirrors support
optdepends = yajl: faster JSON parsing
provides = yay
";

    let real_data = parse_srcinfo(real_world_content);
    println!(
        "Package: {} ({}-{})",
        real_data.pkgname, real_data.pkgver, real_data.pkgrel
    );
    println!("\nRuntime Dependencies:");
    for dep in &real_data.depends {
        println!("  - {dep}");
    }
    println!("\nBuild Dependencies:");
    for dep in &real_data.makedepends {
        println!("  - {dep}");
    }
    println!("\nOptional Dependencies:");
    for dep in &real_data.optdepends {
        println!("  - {dep}");
    }
    println!("\nProvides:");
    for provide in &real_data.provides {
        println!("  - {provide}");
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}
