//! Comprehensive dependency parsing example for arch-toolkit.
//!
//! This example demonstrates:
//! - Parsing dependency specification strings with version constraints
//! - Extracting dependencies from pacman -Si output
//! - Extracting conflicts from pacman -Si output
//! - Handling multi-line dependencies and continuation lines
//! - Filtering of .so files and invalid tokens
//! - Edge cases and special formats
//!
//! Run with:
//!   `cargo run --example parse_example --features deps`

#[cfg(not(feature = "deps"))]
fn main() {
    eprintln!("This example requires the 'deps' feature to be enabled.");
    eprintln!("Run with: cargo run --example parse_example --features deps");
}

#[cfg(feature = "deps")]
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args
)] // Example file - comprehensive demonstration
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::deps::{parse_dep_spec, parse_pacman_si_conflicts, parse_pacman_si_deps};

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Dependency Parsing Example                  ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic Dependency Spec Parsing
    // ========================================================================
    println!("┌─ Example 1: Basic Dependency Spec Parsing ────────────────────┐");
    println!("│ Parse dependency specification strings with version ops     │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("\n1.1: Simple package names (no version constraint):");
    let simple_deps = ["glibc", "bash", "python", "firefox"];
    for dep_str in &simple_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name,
            if spec.version_req.is_empty() {
                "(empty)"
            } else {
                &spec.version_req
            }
        );
        println!(
            "  Has version req: {}",
            if spec.has_version_req() { "yes" } else { "no" }
        );
        println!();
    }

    println!("1.2: Version constraints with >= operator:");
    let ge_deps = ["python>=3.12", "rust>=1.70", "glibc>=2.35"];
    for dep_str in &ge_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    println!("1.3: Version constraints with <= operator:");
    let le_deps = ["firefox<=121.0", "openssl<=1.1.1", "qt5-base<=5.15.10"];
    for dep_str in &le_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    println!("1.4: Version constraints with = operator:");
    let eq_deps = ["vim=9.0", "firefox=121.0", "pacman=6.1.0"];
    for dep_str in &eq_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    println!("1.5: Version constraints with > operator:");
    let greater_deps = ["rust>1.70", "cmake>3.20", "gcc>12.0"];
    for dep_str in &greater_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    println!("1.6: Version constraints with < operator:");
    let less_deps = ["cmake<4.0", "python<4.0", "gcc<15.0"];
    for dep_str in &less_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    println!("1.7: Complex version strings:");
    let complex_deps = ["qt5-base>=5.15.10-1", "python>=3.12.0", "firefox=121.0-1"];
    for dep_str in &complex_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    println!("1.8: Whitespace handling:");
    let whitespace_deps = ["  python >= 3.12  ", "  glibc  ", "  firefox <= 121.0  "];
    for dep_str in &whitespace_deps {
        let spec = parse_dep_spec(dep_str);
        println!("  Input:  \"{dep_str}\"");
        println!(
            "  Output: name=\"{}\", version_req=\"{}\"",
            spec.name, spec.version_req
        );
        println!();
    }

    // ========================================================================
    // Example 2: Parsing Pacman -Si Dependencies
    // ========================================================================
    println!("┌─ Example 2: Parsing Pacman -Si Dependencies ──────────────────┐");
    println!("│ Extract dependencies from pacman -Si output                    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("\n2.1: Basic single-line dependencies:");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_basic = r#"Repository      : extra
Name            : firefox
Version         : 121.0-1
Depends On      : glibc gtk3 libpulse nss libxt libxss
Optional Deps   : None"#;

    println!("Sample pacman -Si output:\n");
    println!("{}", pacman_output_basic);
    println!();

    let deps_basic = parse_pacman_si_deps(pacman_output_basic);
    println!("Extracted dependencies ({}):", deps_basic.len());
    for (i, dep) in deps_basic.iter().enumerate() {
        println!("  {}. {dep}", i + 1);
    }
    println!();

    println!("2.2: Multi-line dependencies (continuation lines):");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_multiline = r#"Repository      : extra
Name            : firefox
Version         : 121.0-1
Description     : Standalone web browser from mozilla.org
Depends On      : glibc gtk3 libpulse nss libxt libxss libxcomposite libxdamage
                  libxfixes libxrandr libxrender libx11 libxcb libxkbcommon
                  libxkbcommon-x11 libdrm libxshmfence libgl libxext libxfixes
                  libxrender libxtst libxrandr libxss libxcomposite libxdamage
Optional Deps   : None
Required By     : None
Conflicts With  : None"#;

    println!("Sample pacman -Si output with continuation lines:\n");
    println!(
        "{}",
        pacman_output_multiline
            .lines()
            .take(8)
            .collect::<Vec<_>>()
            .join("\n")
    );
    println!("...\n");

    let deps_multiline = parse_pacman_si_deps(pacman_output_multiline);
    println!("Extracted dependencies ({}):", deps_multiline.len());
    println!("  (First 10 dependencies)");
    for (i, dep) in deps_multiline.iter().take(10).enumerate() {
        println!("  {}. {dep}", i + 1);
    }
    if deps_multiline.len() > 10 {
        println!("  ... and {} more", deps_multiline.len() - 10);
    }
    println!();

    println!("2.3: Dependencies with version constraints:");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_versions = r#"Name            : python-pip
Version         : 24.0-1
Depends On      : python>=3.10 python-setuptools>=65.0.0"#;

    println!("Sample pacman -Si output with version constraints:\n");
    println!("{pacman_output_versions}\n");

    let deps_versions = parse_pacman_si_deps(pacman_output_versions);
    println!("Extracted dependencies ({}):", deps_versions.len());
    for (i, dep) in deps_versions.iter().enumerate() {
        println!("  {}. {dep}", i + 1);
    }
    println!();

    println!("2.4: Package with no dependencies (\"None\"):");
    let pacman_output_none = "Name            : base\nDepends On      : None\n";
    println!("Input:\n{pacman_output_none}");

    let deps_none = parse_pacman_si_deps(pacman_output_none);
    println!("Extracted dependencies: {}", deps_none.len());
    if deps_none.is_empty() {
        println!("  ✓ Correctly handled \"None\" case");
    }
    println!();

    println!("2.5: Filtering of .so files (virtual packages):");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_so = r#"Name            : test-package
Depends On      : glibc libedit.so libgit2.so.1 nss libfoo.so=0-64"#;

    println!("Sample pacman -Si output with .so files:\n");
    println!("{pacman_output_so}\n");

    let deps_so = parse_pacman_si_deps(pacman_output_so);
    println!("Extracted dependencies ({}):", deps_so.len());
    println!("  Note: .so files are filtered out (virtual packages)");
    for (i, dep) in deps_so.iter().enumerate() {
        println!("  {}. {dep}", i + 1);
    }
    println!();

    println!("2.6: Deduplication of dependencies:");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_duplicates = r#"Name            : test-package
Depends On      : glibc gtk3 glibc nss gtk3 libx11"#;

    println!("Sample pacman -Si output with duplicates:\n");
    println!("{pacman_output_duplicates}\n");

    let deps_duplicates = parse_pacman_si_deps(pacman_output_duplicates);
    println!("Extracted dependencies ({}):", deps_duplicates.len());
    println!("  Note: Duplicates are automatically removed");
    for (i, dep) in deps_duplicates.iter().enumerate() {
        println!("  {}. {dep}", i + 1);
    }
    println!();

    // ========================================================================
    // Example 3: Parsing Pacman -Si Conflicts
    // ========================================================================
    println!("┌─ Example 3: Parsing Pacman -Si Conflicts ────────────────────┐");
    println!("│ Extract conflicts from pacman -Si output                      │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("\n3.1: Basic conflicts:");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_conflicts_basic = r#"Name            : vim
Version         : 9.1.0000-1
Conflicts With : gvim vi
Replaces       : vi"#;

    println!("Sample pacman -Si output with conflicts:\n");
    println!("{pacman_output_conflicts_basic}\n");

    let conflicts_basic = parse_pacman_si_conflicts(pacman_output_conflicts_basic);
    println!("Extracted conflicts ({}):", conflicts_basic.len());
    for (i, conflict) in conflicts_basic.iter().enumerate() {
        println!("  {}. {conflict}", i + 1);
    }
    println!();

    println!("3.2: Conflicts with version constraints:");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_conflicts_versions = r#"Name            : test-package
Version         : 1.0-1
Conflicts With  : old-pkg<2.0 new-pkg>=3.0 another-pkg=1.5.0"#;

    println!("Sample pacman -Si output with version constraints:\n");
    println!("{pacman_output_conflicts_versions}\n");

    let conflicts_versions = parse_pacman_si_conflicts(pacman_output_conflicts_versions);
    println!("Extracted conflicts ({}):", conflicts_versions.len());
    println!("  Note: Version constraints are removed, only package names are kept");
    for (i, conflict) in conflicts_versions.iter().enumerate() {
        println!("  {}. {conflict}", i + 1);
    }
    println!();

    println!("3.3: Multi-line conflicts (continuation lines):");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_conflicts_multiline = r#"Name            : test-package
Version         : 1.0-1
Conflicts With  : pkg1 pkg2 pkg3
                  pkg4 pkg5 pkg6
                  pkg7"#;

    println!("Sample pacman -Si output with continuation lines:\n");
    println!("{pacman_output_conflicts_multiline}\n");

    let conflicts_multiline = parse_pacman_si_conflicts(pacman_output_conflicts_multiline);
    println!("Extracted conflicts ({}):", conflicts_multiline.len());
    for (i, conflict) in conflicts_multiline.iter().enumerate() {
        println!("  {}. {conflict}", i + 1);
    }
    println!();

    println!("3.4: Package with no conflicts (\"None\"):");
    let pacman_output_conflicts_none = "Name            : base\nConflicts With : None\n";
    println!("Input:\n{pacman_output_conflicts_none}");

    let conflicts_none = parse_pacman_si_conflicts(pacman_output_conflicts_none);
    println!("Extracted conflicts: {}", conflicts_none.len());
    if conflicts_none.is_empty() {
        println!("  ✓ Correctly handled \"None\" case");
    }
    println!();

    println!("3.5: Deduplication of conflicts:");
    #[allow(clippy::needless_raw_string_hashes)]
    let pacman_output_conflicts_duplicates = r#"Name            : test-package
Conflicts With  : pkg1 pkg2 pkg1 pkg3 pkg2"#;

    println!("Sample pacman -Si output with duplicate conflicts:\n");
    println!("{pacman_output_conflicts_duplicates}\n");

    let conflicts_duplicates = parse_pacman_si_conflicts(pacman_output_conflicts_duplicates);
    println!("Extracted conflicts ({}):", conflicts_duplicates.len());
    println!("  Note: Duplicates are automatically removed");
    for (i, conflict) in conflicts_duplicates.iter().enumerate() {
        println!("  {}. {conflict}", i + 1);
    }
    println!();

    // ========================================================================
    // Example 4: Edge Cases
    // ========================================================================
    println!("┌─ Example 4: Edge Cases ───────────────────────────────────────┐");
    println!("│ Handling edge cases and special formats                       │");
    println!("└──────────────────────────────────────────────────────────────┘");

    println!("\n4.1: Empty strings:");
    let empty_spec = parse_dep_spec("");
    println!(
        "  parse_dep_spec(\"\"): name=\"{}\", version_req=\"{}\"",
        empty_spec.name, empty_spec.version_req
    );
    println!();

    println!("4.2: Whitespace-only strings:");
    let whitespace_spec = parse_dep_spec("   ");
    println!(
        "  parse_dep_spec(\"   \"): name=\"{}\", version_req=\"{}\"",
        whitespace_spec.name, whitespace_spec.version_req
    );
    println!();

    println!("4.3: Missing \"Depends On\" field:");
    let pacman_no_depends = "Name            : test-package\nVersion         : 1.0-1\n";
    let deps_no_field = parse_pacman_si_deps(pacman_no_depends);
    println!("  Input: pacman -Si output without \"Depends On\" field");
    println!("  Output: {} dependencies", deps_no_field.len());
    println!();

    println!("4.4: Missing \"Conflicts With\" field:");
    let pacman_no_conflicts = "Name            : test-package\nVersion         : 1.0-1\n";
    let conflicts_no_field = parse_pacman_si_conflicts(pacman_no_conflicts);
    println!("  Input: pacman -Si output without \"Conflicts With\" field");
    println!("  Output: {} conflicts", conflicts_no_field.len());
    println!();

    println!("4.5: Empty \"Depends On\" field:");
    let pacman_empty_depends = "Name            : test-package\nDepends On      :\n";
    let deps_empty = parse_pacman_si_deps(pacman_empty_depends);
    println!("  Input: pacman -Si output with empty \"Depends On\" field");
    println!("  Output: {} dependencies", deps_empty.len());
    println!();

    println!("4.6: Empty \"Conflicts With\" field:");
    let pacman_empty_conflicts = "Name            : test-package\nConflicts With  :\n";
    let conflicts_empty = parse_pacman_si_conflicts(pacman_empty_conflicts);
    println!("  Input: pacman -Si output with empty \"Conflicts With\" field");
    println!("  Output: {} conflicts", conflicts_empty.len());
    println!();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Example Complete!                          ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    println!("This example demonstrated:");
    println!("  • parse_dep_spec() - Parse dependency specification strings");
    println!("    - Simple package names (no version constraint)");
    println!("    - Version constraints with all operators (>=, <=, =, >, <)");
    println!("    - Complex version strings");
    println!("    - Whitespace handling");
    println!("  • parse_pacman_si_deps() - Extract dependencies from pacman -Si output");
    println!("    - Single-line dependencies");
    println!("    - Multi-line dependencies (continuation lines)");
    println!("    - Dependencies with version constraints");
    println!("    - \"None\" case handling");
    println!("    - Filtering of .so files (virtual packages)");
    println!("    - Deduplication of dependencies");
    println!("  • parse_pacman_si_conflicts() - Extract conflicts from pacman -Si output");
    println!("    - Basic conflicts");
    println!("    - Conflicts with version constraints");
    println!("    - Multi-line conflicts (continuation lines)");
    println!("    - \"None\" case handling");
    println!("    - Deduplication of conflicts");
    println!("  • Edge cases and special formats");
    println!("    - Empty strings");
    println!("    - Missing fields");
    println!("    - Empty fields");

    Ok(())
}
