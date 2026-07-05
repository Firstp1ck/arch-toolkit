//! Comprehensive example demonstrating all index module features.
//!
//! This example shows how to use:
//! - Installed package queries (pacman -Qq)
//! - Explicit package tracking (pacman -Qeq / -Qetq)
//! - Official repository queries (search, lookup)
//! - Index fetching (pacman -Sl or Arch Packages API)
//! - Index persistence (save/load JSON)
//!
//! Run with:
//!   `cargo run --example index_example --features index`
//!   `cargo run --example index_example --features index,fuzzy-search`  # For fuzzy search

#[cfg(not(feature = "index"))]
fn main() {
    eprintln!("This example requires the 'index' feature to be enabled.");
    eprintln!("Run with: cargo run --example index_example --features index");
    std::process::exit(1);
}

#[cfg(feature = "index")]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::index::{
        InstalledPackagesMode, all_official, fetch_official_index, get_installed_packages,
        is_explicit, is_installed, load_from_disk, refresh_explicit_cache, refresh_installed_cache,
        save_to_disk, search_official,
    };
    use std::collections::HashSet;

    println!("=== Arch Toolkit Index Module Examples ===\n");

    // Example 1: Query installed packages
    println!("1. Installed Package Queries");
    println!("-----------------------------");
    let installed = get_installed_packages()?;
    println!("Found {} installed packages", installed.len());
    for name in ["vim", "git", "pacman"] {
        if is_installed(name, Some(&installed)) {
            println!("  {name} is installed");
        } else {
            println!("  {name} is not installed");
        }
    }
    println!();

    // Example 2: Use a cache for repeated lookups
    println!("2. Cached Installed Lookups");
    println!("----------------------------");
    let mut cache = HashSet::new();
    refresh_installed_cache(Some(&mut cache))?;
    println!("Cache holds {} packages", cache.len());
    println!();

    // Example 3: Explicit package tracking
    println!("3. Explicit Package Tracking");
    println!("-----------------------------");
    let explicit = refresh_explicit_cache(InstalledPackagesMode::AllExplicit, None)?;
    println!("{} explicitly installed packages", explicit.len());
    let leaf = refresh_explicit_cache(InstalledPackagesMode::LeafOnly, None)?;
    println!(
        "{} leaf packages (explicit, not required by others)",
        leaf.len()
    );
    if is_explicit("vim", InstalledPackagesMode::AllExplicit, Some(&explicit)) {
        println!("  vim was explicitly installed");
    }
    println!();

    // Example 4: Fetch the official index
    println!("4. Official Index Fetching");
    println!("---------------------------");
    let index = match fetch_official_index() {
        Ok(index) => {
            println!("Fetched {} official packages", index.pkgs.len());
            index
        }
        Err(e) => {
            println!("Fetch failed ({e}); using a small demo index instead");
            demo_index()
        }
    };
    println!();

    // Example 5: Search the official index
    println!("5. Official Index Search");
    println!("-------------------------");
    let results = search_official(&index, "grep", false);
    println!(
        "Substring search for 'grep' found {} packages:",
        results.len()
    );
    for result in results.iter().take(5) {
        println!(
            "  {}/{} {} - {}",
            result.package.repo,
            result.package.name,
            result.package.version,
            result.package.description
        );
    }
    #[cfg(feature = "fuzzy-search")]
    {
        let fuzzy = search_official(&index, "rgep", true);
        println!("Fuzzy search for 'rgep' found {} packages", fuzzy.len());
    }
    println!();

    // Example 6: O(1) name lookup
    println!("6. Package Lookup by Name");
    println!("--------------------------");
    if let Some(pkg) = index.find_package_by_name("ripgrep") {
        println!("Found: {}/{} {}", pkg.repo, pkg.name, pkg.version);
    } else {
        println!("ripgrep not found in index");
    }
    let total = all_official(&index).len();
    println!("Index contains {total} packages in total");
    println!();

    // Example 7: Persist and reload the index
    println!("7. Index Persistence");
    println!("---------------------");
    let mut path = std::env::temp_dir();
    path.push("arch_toolkit_index_example.json");
    save_to_disk(&index, &path)?;
    println!("Saved index to {}", path.display());

    // The load-or-fetch pattern: prefer the cached file, fall back to fetching
    let reloaded = load_from_disk(&path).or_else(|_| fetch_official_index())?;
    println!("Reloaded {} packages from disk", reloaded.pkgs.len());
    let _ = std::fs::remove_file(&path);
    println!();

    println!("=== All examples completed ===");
    Ok(())
}

/// Build a small in-memory index so the example works without pacman or network.
#[cfg(feature = "index")]
fn demo_index() -> arch_toolkit::index::OfficialIndex {
    use arch_toolkit::index::{OfficialIndex, OfficialPackage};

    let mut index = OfficialIndex {
        pkgs: vec![
            OfficialPackage {
                name: "ripgrep".to_string(),
                repo: "extra".to_string(),
                arch: "x86_64".to_string(),
                version: "14.0.0".to_string(),
                description: "A search tool that combines grep with ripgrep".to_string(),
            },
            OfficialPackage {
                name: "grep".to_string(),
                repo: "core".to_string(),
                arch: "x86_64".to_string(),
                version: "3.11".to_string(),
                description: "A string search utility".to_string(),
            },
            OfficialPackage {
                name: "vim".to_string(),
                repo: "extra".to_string(),
                arch: "x86_64".to_string(),
                version: "9.0".to_string(),
                description: "Vi Improved, a highly configurable text editor".to_string(),
            },
        ],
        name_to_idx: std::collections::HashMap::new(),
    };
    index.rebuild_name_index();
    index
}
