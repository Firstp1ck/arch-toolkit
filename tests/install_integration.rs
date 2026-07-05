//! Integration tests for the install module command builders.

#[cfg(feature = "install")]
mod tests {
    use std::collections::HashSet;

    use arch_toolkit::install::{
        build_batch_install, build_pacman_install, build_remove_command, build_update_command,
        with_privilege,
    };
    use arch_toolkit::types::install::{
        AurHelper, CascadeMode, CommandSpec, InstallOptions, PrivilegeTool,
    };
    use arch_toolkit::{PackageRef, PackageSource};

    #[test]
    /// What: Verify the full workflow: plan a mixed batch, wrap, and render.
    ///
    /// Inputs:
    /// - Mixed official/AUR targets with helper, privilege, and installed set.
    ///
    /// Output:
    /// - Two shell-renderable commands in pacman-then-helper order.
    ///
    /// Details:
    /// - Exercises batch planning together with rendering as a caller would.
    fn mixed_batch_workflow() {
        let targets = vec![
            PackageRef::official("ripgrep", "14.0.0", "extra", "x86_64"),
            PackageRef::official("fd", "9.0.0", "extra", "x86_64"),
            PackageRef::aur("paru-bin", "2.0.0"),
        ];
        let installed: HashSet<String> = HashSet::from(["fd".to_string()]);

        let plan = build_batch_install(
            &targets,
            Some(AurHelper::Paru),
            Some(PrivilegeTool::Sudo),
            &InstallOptions::default(),
            Some(&installed),
        )
        .expect("plan should build");

        assert_eq!(plan.official, ["ripgrep", "fd"]);
        assert_eq!(plan.aur, ["paru-bin"]);
        assert_eq!(plan.commands.len(), 2);
        // fd is installed → the official group drops --needed (reinstall path)
        assert_eq!(
            plan.commands[0].to_shell_string(),
            "sudo pacman -S --noconfirm ripgrep fd"
        );
        // AUR group has no reinstalls → keeps --needed, never sudo-wrapped
        assert_eq!(
            plan.commands[1].to_shell_string(),
            "paru -S --aur --needed --noconfirm paru-bin"
        );
    }

    #[test]
    /// What: Verify built commands convert to spawnable `std::process::Command`s.
    ///
    /// Inputs:
    /// - A remove command wrapped with doas.
    ///
    /// Output:
    /// - `Command` with `doas` program and pacman argv.
    ///
    /// Details:
    /// - Confirms the argv path needs no shell quoting at all.
    fn command_spec_spawnable() {
        let spec = with_privilege(
            PrivilegeTool::Doas,
            build_remove_command(&["old-pkg"], CascadeMode::Cascade, true).expect("build"),
        );
        let cmd = spec.to_command();
        assert_eq!(cmd.get_program(), "doas");
        let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy()).collect();
        assert_eq!(args, ["pacman", "-Rs", "--noconfirm", "old-pkg"]);
    }

    #[test]
    /// What: Verify shell injection attempts cannot reach a built command.
    ///
    /// Inputs:
    /// - Target names with metacharacters, quotes, and subshell syntax.
    ///
    /// Output:
    /// - Every attempt is rejected with an error; no command is produced.
    ///
    /// Details:
    /// - Defense-in-depth check across install, remove, and batch builders.
    fn injection_attempts_rejected() {
        for evil in ["a;b", "a && rm -rf /", "$(reboot)", "`ls`", "a'b", "a b"] {
            assert!(
                build_pacman_install(&[evil], &InstallOptions::default()).is_err(),
                "install should reject {evil}"
            );
            assert!(
                build_remove_command(&[evil], CascadeMode::Basic, true).is_err(),
                "remove should reject {evil}"
            );
            let target = PackageRef {
                name: evil.to_string(),
                version: "1".to_string(),
                source: PackageSource::Aur,
            };
            assert!(
                build_batch_install(
                    std::slice::from_ref(&target),
                    Some(AurHelper::Yay),
                    None,
                    &InstallOptions::default(),
                    None::<&HashSet<String>>,
                )
                .is_err(),
                "batch should reject {evil}"
            );
        }
    }

    #[test]
    /// What: Verify shell rendering round-trips through `bash -c` correctly.
    ///
    /// Inputs:
    /// - A `CommandSpec` for `echo` with an awkward argument.
    ///
    /// Output:
    /// - Running the rendered string through bash reproduces the argument.
    ///
    /// Details:
    /// - End-to-end proof that `to_shell_string()` quoting is bash-safe.
    fn shell_string_roundtrip_through_bash() {
        let spec = CommandSpec::new("echo", ["it's a 'test'"]);
        let output = std::process::Command::new("bash")
            .args(["-c", &spec.to_shell_string()])
            .output()
            .expect("bash should run");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "it's a 'test'"
        );
    }

    #[test]
    /// What: Verify update commands for system-wide and helper-driven updates.
    ///
    /// Inputs:
    /// - `None` and `Some(AurHelper::Paru)` helper choices.
    ///
    /// Output:
    /// - `pacman -Syu --noconfirm` and `paru -Syu --noconfirm` respectively.
    ///
    /// Details:
    /// - Helper variant covers AUR packages too; pacman variant needs privilege.
    fn update_variants() {
        assert_eq!(
            with_privilege(PrivilegeTool::Sudo, build_update_command(None, true)).to_shell_string(),
            "sudo pacman -Syu --noconfirm"
        );
        assert_eq!(
            build_update_command(Some(AurHelper::Paru), true).to_shell_string(),
            "paru -Syu --noconfirm"
        );
    }
}
