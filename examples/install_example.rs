//! Comprehensive example demonstrating all install module features.
//!
//! This example shows how to use:
//! - Pacman install command building
//! - AUR helper install command building
//! - Remove commands with cascade modes
//! - Update commands
//! - AUR helper and privilege tool detection
//! - Batch install planning (mixed official + AUR targets)
//!
//! The install module builds commands but never executes them — this example
//! only prints what would run (an inherent "dry run").
//!
//! Run with:
//!   `cargo run --example install_example --features install`

#[cfg(not(feature = "install"))]
fn main() {
    eprintln!("This example requires the 'install' feature to be enabled.");
    eprintln!("Run with: cargo run --example install_example --features install");
    std::process::exit(1);
}

#[cfg(feature = "install")]
fn main() -> arch_toolkit::error::Result<()> {
    use arch_toolkit::PackageRef;
    use arch_toolkit::install::{
        build_aur_install, build_batch_install, build_pacman_install, build_remove_command,
        build_update_command, detect_aur_helper, detect_privilege_tool, with_privilege,
    };
    use arch_toolkit::types::install::{AurHelper, CascadeMode, InstallOptions, PrivilegeTool};
    use std::collections::HashSet;

    println!("=== Arch Toolkit Install Module Examples ===\n");

    // Example 1: Detect available tools
    println!("1. Tool Detection");
    println!("------------------");
    let helper = detect_aur_helper();
    let privilege = detect_privilege_tool();
    match helper {
        Some(h) => println!("AUR helper: {h}"),
        None => println!("No AUR helper (paru/yay) found"),
    }
    match privilege {
        Some(t) => println!("Privilege tool: {t}"),
        None => println!("No privilege tool (sudo/doas) found"),
    }
    println!();

    // Example 2: Build a pacman install command
    println!("2. Pacman Install Command");
    println!("--------------------------");
    let spec = build_pacman_install(&["ripgrep", "fd"], &InstallOptions::default())?;
    println!("Unprivileged: {spec}");
    let wrapped = with_privilege(privilege.unwrap_or(PrivilegeTool::Sudo), spec);
    println!("Privileged:   {wrapped}");
    println!();

    // Example 3: Build an AUR helper install command
    println!("3. AUR Install Command");
    println!("-----------------------");
    let aur_spec = build_aur_install(
        helper.unwrap_or(AurHelper::Paru),
        &["yay-bin"],
        &InstallOptions::default(),
    )?;
    println!("{aur_spec}");
    println!("(AUR helpers are never wrapped in sudo — they escalate internally)");
    println!();

    // Example 4: Reinstall (drop --needed)
    println!("4. Reinstall Options");
    println!("---------------------");
    let reinstall = InstallOptions {
        needed: false,
        ..Default::default()
    };
    let spec = build_pacman_install(&["ripgrep"], &reinstall)?;
    println!("Reinstall: {spec}");
    println!();

    // Example 5: Remove commands with cascade modes
    println!("5. Remove Commands");
    println!("-------------------");
    for mode in [
        CascadeMode::Basic,
        CascadeMode::Cascade,
        CascadeMode::CascadeWithConfigs,
    ] {
        let spec = build_remove_command(&["old-package"], mode, true)?;
        println!("{} ({})", spec, mode.description());
    }
    println!();

    // Example 6: Update commands
    println!("6. Update Commands");
    println!("-------------------");
    println!("Official only: {}", build_update_command(None, true));
    println!(
        "With AUR:      {}",
        build_update_command(Some(AurHelper::Paru), true)
    );
    println!();

    // Example 7: Batch planning with mixed targets
    println!("7. Batch Install Planning");
    println!("--------------------------");
    let targets = vec![
        PackageRef::official("ripgrep", "14.0.0", "extra", "x86_64"),
        PackageRef::official("fd", "9.0.0", "extra", "x86_64"),
        PackageRef::aur("paru-bin", "2.0.0"),
    ];
    // Simulate: fd is already installed, so the official group reinstalls
    let installed: HashSet<String> = HashSet::from(["fd".to_string()]);
    let plan = build_batch_install(
        &targets,
        Some(helper.unwrap_or(AurHelper::Paru)),
        privilege,
        &InstallOptions::default(),
        Some(&installed),
    )?;
    println!(
        "{} official, {} AUR target(s):",
        plan.official.len(),
        plan.aur.len()
    );
    for (i, command) in plan.commands.iter().enumerate() {
        println!("  step {}: {}", i + 1, command);
    }
    println!();

    // Example 8: Executing is the caller's decision
    println!("8. Execution Is Yours");
    println!("----------------------");
    println!("Nothing above was executed. To run a built command:");
    println!("  spec.to_command().status()   // argv, no shell");
    println!("  bash -c \"$(spec.to_shell_string())\"  // via shell/terminal");
    println!();

    println!("=== All examples completed ===");
    Ok(())
}
