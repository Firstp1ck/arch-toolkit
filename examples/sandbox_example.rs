//! Comprehensive example demonstrating the sandbox module.
//!
//! This example shows how to use:
//! - PKGBUILD build-preflight analysis against the real host
//! - .SRCINFO analysis
//! - Dependency name extraction
//! - Readiness and missing-package reporting
//!
//! Run with:
//!   `cargo run --example sandbox_example --features sandbox`

#[cfg(not(feature = "sandbox"))]
fn main() {
    eprintln!("This example requires the 'sandbox' feature to be enabled.");
    eprintln!("Run with: cargo run --example sandbox_example --features sandbox");
    std::process::exit(1);
}

#[cfg(feature = "sandbox")]
fn main() {
    use arch_toolkit::deps::{get_installed_packages, get_provided_packages};
    use arch_toolkit::sandbox::{analyze_pkgbuild, analyze_srcinfo, extract_package_name};

    println!("=== Arch Toolkit Sandbox Module Examples ===\n");

    // Query the real host once; both analyses reuse the sets.
    let installed = get_installed_packages().unwrap_or_default();
    let provided = get_provided_packages(&installed);
    println!("Host has {} installed packages\n", installed.len());

    // Example 1: Analyze a sample PKGBUILD against this host
    println!("1. PKGBUILD Preflight Analysis");
    println!("-------------------------------");
    let pkgbuild = r"
pkgname=demo-tool
depends=('glibc' 'gcc-libs' 'imaginary-runtime-lib')
makedepends=('rust>=1.70' 'git')
optdepends=('cups: printing support')
";
    let info = analyze_pkgbuild("demo-tool", pkgbuild, &installed, &provided);
    print_report(&info);
    println!();

    // Example 2: Analyze .SRCINFO content
    println!("2. .SRCINFO Preflight Analysis");
    println!("-------------------------------");
    let srcinfo = "pkgbase = demo-tool\n\tdepends = glibc\n\tmakedepends = cmake\n\tmakedepends = imaginary-build-tool";
    let info = analyze_srcinfo("demo-tool", srcinfo, &installed, &provided);
    print_report(&info);
    println!();

    // Example 3: Dependency name extraction
    println!("3. Dependency Name Extraction");
    println!("------------------------------");
    for spec in ["python>=3.12", "qt6-base<7", "cups: printing support"] {
        println!("{spec:28} -> {}", extract_package_name(spec));
    }
    println!();

    println!("=== All examples completed ===");
}

/// Print a compact preflight report for one analysis result.
#[cfg(feature = "sandbox")]
fn print_report(info: &arch_toolkit::sandbox::SandboxInfo) {
    for (label, deltas) in [
        ("depends", &info.depends),
        ("makedepends", &info.makedepends),
        ("checkdepends", &info.checkdepends),
        ("optdepends", &info.optdepends),
    ] {
        for delta in deltas {
            let status = if delta.is_installed {
                "ok     "
            } else {
                "MISSING"
            };
            let version = delta.installed_version.as_deref().unwrap_or("-");
            println!(
                "  [{status}] {label:12} {:32} installed: {version}",
                delta.name
            );
        }
    }
    if info.is_ready_to_build() {
        println!("  => ready to build");
    } else {
        println!("  => missing: {}", info.missing_packages().join(", "));
    }
}
