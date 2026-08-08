//! Integration tests for the install module command builders.

#[cfg(feature = "install")]
mod tests {
    use std::collections::HashSet;

    use arch_toolkit::install::{
        NO_AUR_HELPER_MESSAGE, aur_install_shell_fallback, aur_update_shell_fallback,
        build_aur_install, build_aur_update_command, build_batch_install,
        build_force_sync_update_command, build_pacman_install, build_remove_command,
        build_update_command, is_safe_package_name, with_privilege,
    };
    use arch_toolkit::types::install::{
        AurHelper, CascadeMode, CommandSpec, InstallOptions, PrivilegeTool,
    };
    use arch_toolkit::{PackageRef, PackageSource};

    /// What: Collect the argv of a `CommandSpec` as owned strings.
    ///
    /// Inputs:
    /// - `spec`: The command specification to inspect.
    ///
    /// Output:
    /// - Vector containing the program followed by every argument.
    ///
    /// Details:
    /// - Used to assert exact argv including the `--` operand terminator,
    ///   independently of shell rendering.
    fn argv(spec: &CommandSpec) -> Vec<String> {
        let mut out = vec![spec.program.clone()];
        out.extend(spec.args.iter().cloned());
        out
    }

    /// What: Build a `PackageRef` for an AUR target with a fixed version.
    ///
    /// Inputs:
    /// - `name`: Package name to route to the AUR helper.
    ///
    /// Output:
    /// - `PackageRef` whose source is `PackageSource::Aur`.
    ///
    /// Details:
    /// - Keeps batch-planning regression tests short and deterministic.
    fn aur_target(name: &str) -> PackageRef {
        PackageRef {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            source: PackageSource::Aur,
        }
    }

    /// What: Build a `PackageRef` for an official target with fixed metadata.
    ///
    /// Inputs:
    /// - `name`: Package name to route to pacman.
    ///
    /// Output:
    /// - `PackageRef` whose source is `PackageSource::Official`.
    ///
    /// Details:
    /// - Keeps batch-planning regression tests short and deterministic.
    fn official_target(name: &str) -> PackageRef {
        PackageRef::official(name, "1.0.0", "extra", "x86_64")
    }

    /// Names that must never reach a built command because their first byte can
    /// be parsed as an option or a hidden-path prefix.
    const OPTION_LIKE_NAMES: [&str; 7] = ["--help", "-S", "-Rns", "--", "-", ".hidden", "."];

    /// Valid Arch names that keep internal `@ . _ + -` punctuation and must stay accepted.
    const VALID_NAMES: [&str; 6] = [
        "ripgrep",
        "lib32-glibc",
        "python3.12",
        "gcc12+libs",
        "a@b_c",
        "0ad",
    ];

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
            "sudo pacman -S --noconfirm -- ripgrep fd"
        );
        // AUR group has no reinstalls → keeps --needed, never sudo-wrapped
        assert_eq!(
            plan.commands[1].to_shell_string(),
            "paru -S --aur --needed --noconfirm -- paru-bin"
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
        assert_eq!(args, ["pacman", "-Rs", "--noconfirm", "--", "old-pkg"]);
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
    /// What: Verify option-like package names are rejected by every install path.
    ///
    /// Inputs:
    /// - Names starting with `-` or `.` such as `--help`, `-S`, and `.hidden`.
    ///
    /// Output:
    /// - `is_safe_package_name` is `false` and every builder returns an error.
    ///
    /// Details:
    /// - Covers direct pacman/AUR install, remove, batch planning (official and
    ///   AUR routes), and the runtime shell fallback (U5).
    fn option_like_names_rejected_across_all_builders() {
        for evil in OPTION_LIKE_NAMES {
            assert!(
                !is_safe_package_name(evil),
                "{evil} must not be a safe package name"
            );
            assert!(
                build_pacman_install(&[evil], &InstallOptions::default()).is_err(),
                "pacman install should reject {evil}"
            );
            assert!(
                build_aur_install(AurHelper::Paru, &[evil], &InstallOptions::default()).is_err(),
                "AUR install should reject {evil}"
            );
            assert!(
                build_remove_command(&[evil], CascadeMode::CascadeWithConfigs, true).is_err(),
                "remove should reject {evil}"
            );
            assert!(
                aur_install_shell_fallback(&[evil], &InstallOptions::default()).is_err(),
                "shell fallback should reject {evil}"
            );
            for target in [official_target(evil), aur_target(evil)] {
                assert!(
                    build_batch_install(
                        std::slice::from_ref(&target),
                        Some(AurHelper::Paru),
                        Some(PrivilegeTool::Sudo),
                        &InstallOptions::default(),
                        None::<&HashSet<String>>,
                    )
                    .is_err(),
                    "batch should reject {evil}"
                );
            }
        }
    }

    #[test]
    /// What: Verify legitimate Arch names with internal punctuation stay accepted.
    ///
    /// Inputs:
    /// - `lib32-glibc`, `python3.12`, `gcc12+libs`, `a@b_c`, `0ad`, `ripgrep`.
    ///
    /// Output:
    /// - Every direct, remove, batch, and fallback builder succeeds.
    ///
    /// Details:
    /// - Guards the tightened first-byte rule against false rejections (U5).
    fn valid_names_with_internal_punctuation_still_accepted() {
        for good in VALID_NAMES {
            assert!(is_safe_package_name(good), "{good} must remain valid");
            assert!(
                build_pacman_install(&[good], &InstallOptions::default()).is_ok(),
                "pacman install should accept {good}"
            );
            assert!(
                build_aur_install(AurHelper::Yay, &[good], &InstallOptions::default()).is_ok(),
                "AUR install should accept {good}"
            );
            assert!(
                build_remove_command(&[good], CascadeMode::Basic, false).is_ok(),
                "remove should accept {good}"
            );
            assert!(
                aur_install_shell_fallback(&[good], &InstallOptions::default()).is_ok(),
                "shell fallback should accept {good}"
            );
            let target = official_target(good);
            assert!(
                build_batch_install(
                    std::slice::from_ref(&target),
                    None,
                    None,
                    &InstallOptions::default(),
                    None::<&HashSet<String>>,
                )
                .is_ok(),
                "batch should accept {good}"
            );
        }
    }

    #[test]
    /// What: Verify `--` separates flags from operands in direct install/remove argv.
    ///
    /// Inputs:
    /// - pacman install, AUR helper install, and remove specifications.
    ///
    /// Output:
    /// - Exact argv with `--` after the last flag and before the first name.
    ///
    /// Details:
    /// - Also asserts privilege wrapping keeps the terminator in place and does
    ///   not reinterpret it (U6).
    fn direct_commands_place_operand_terminator_before_names() {
        let install =
            build_pacman_install(&["ripgrep", "fd"], &InstallOptions::default()).expect("build");
        assert_eq!(
            argv(&install),
            [
                "pacman",
                "-S",
                "--needed",
                "--noconfirm",
                "--",
                "ripgrep",
                "fd"
            ]
        );

        let aur = build_aur_install(AurHelper::Paru, &["yay-bin"], &InstallOptions::default())
            .expect("build");
        assert_eq!(
            argv(&aur),
            [
                "paru",
                "-S",
                "--aur",
                "--needed",
                "--noconfirm",
                "--",
                "yay-bin"
            ]
        );

        let remove = build_remove_command(&["old-pkg"], CascadeMode::CascadeWithConfigs, true)
            .expect("build");
        assert_eq!(
            argv(&remove),
            ["pacman", "-Rns", "--noconfirm", "--", "old-pkg"]
        );

        let privileged = with_privilege(PrivilegeTool::Sudo, remove);
        assert_eq!(
            argv(&privileged),
            ["sudo", "pacman", "-Rns", "--noconfirm", "--", "old-pkg"]
        );
        assert_eq!(
            privileged.to_shell_string(),
            "sudo pacman -Rns --noconfirm -- old-pkg"
        );
    }

    #[test]
    /// What: Verify batch plans terminate operands and preserve ordering and `&&`.
    ///
    /// Inputs:
    /// - Mixed official/AUR targets with sudo and paru.
    ///
    /// Output:
    /// - Privileged pacman command first, unprivileged helper second, both with
    ///   `--`, joined by `&&`.
    ///
    /// Details:
    /// - Guards U6 for the batch path and the short-circuit rendering contract.
    fn batch_plan_terminates_operands_and_preserves_order() {
        let targets = vec![aur_target("paru-bin"), official_target("ripgrep")];
        let plan = build_batch_install(
            &targets,
            Some(AurHelper::Paru),
            Some(PrivilegeTool::Sudo),
            &InstallOptions::default(),
            None::<&HashSet<String>>,
        )
        .expect("plan should build");

        assert_eq!(
            argv(&plan.commands[0]),
            [
                "sudo",
                "pacman",
                "-S",
                "--needed",
                "--noconfirm",
                "--",
                "ripgrep"
            ]
        );
        assert_eq!(
            argv(&plan.commands[1]),
            [
                "paru",
                "-S",
                "--aur",
                "--needed",
                "--noconfirm",
                "--",
                "paru-bin"
            ]
        );
        assert_eq!(
            plan.to_shell_string(),
            "sudo pacman -S --needed --noconfirm -- ripgrep && \
             paru -S --aur --needed --noconfirm -- paru-bin"
        );
    }

    #[test]
    /// What: Verify update commands gain no artificial operand terminator.
    ///
    /// Inputs:
    /// - `-Syu`, `-Syyu`, and `-Sua` builders for pacman and helpers.
    ///
    /// Output:
    /// - Argv without `--` because these commands take no package operands.
    ///
    /// Details:
    /// - Prevents the U6 change from altering operand-free update commands.
    fn update_commands_have_no_operand_terminator() {
        for spec in [
            build_update_command(None, true),
            build_update_command(Some(AurHelper::Paru), false),
            build_force_sync_update_command(None, true),
            build_aur_update_command(AurHelper::Yay, true),
        ] {
            assert!(
                !spec.args.iter().any(|arg| arg == "--"),
                "update command must not contain an operand terminator: {}",
                spec.to_shell_string()
            );
        }
        assert_eq!(
            aur_update_shell_fallback(true),
            "(if command -v paru >/dev/null 2>&1; then paru -Sua --noconfirm; \
             elif command -v yay >/dev/null 2>&1; then yay -Sua --noconfirm; \
             else echo 'No AUR helper (paru/yay) found.' >&2; exit 127; fi)"
        );
    }

    #[test]
    /// What: Verify the install fallback body golden with terminator and failure branch.
    ///
    /// Inputs:
    /// - Default install options for a single AUR package.
    ///
    /// Output:
    /// - Exact shell body with `--` before the operand and a stderr/exit-127
    ///   no-helper branch.
    ///
    /// Details:
    /// - Message text must stay byte-identical for migrating callers (U6, U7).
    fn install_fallback_body_golden() {
        let body = aur_install_shell_fallback(&["yay-bin"], &InstallOptions::default())
            .expect("build body");
        assert_eq!(
            body,
            "(if command -v paru >/dev/null 2>&1; then paru -S --aur --needed --noconfirm -- yay-bin; \
             elif command -v yay >/dev/null 2>&1; then yay -S --aur --needed --noconfirm -- yay-bin; \
             else echo 'No AUR helper (paru/yay) found.' >&2; exit 127; fi)"
        );
        assert!(body.contains(NO_AUR_HELPER_MESSAGE));
    }

    #[cfg(unix)]
    #[test]
    /// What: Verify fallback execution prefers paru, falls back to yay, and fails loudly.
    ///
    /// Inputs:
    /// - Isolated `PATH` fixtures containing paru only, yay only, or no helper.
    ///
    /// Output:
    /// - Recorded helper argv including `--`, and status 127 with the message on
    ///   stderr when no helper exists.
    ///
    /// Details:
    /// - Executes only fake helper scripts inside a temporary directory; no real
    ///   package operation runs (U7).
    fn shell_fallback_helper_preference_and_missing_helper_status() {
        let body = aur_install_shell_fallback(&["ripgrep"], &InstallOptions::default())
            .expect("build body");
        let expected_args = ["-S", "--aur", "--needed", "--noconfirm", "--", "ripgrep"];

        let both = tempfile::tempdir().expect("temporary fixture directory");
        write_fake_helper(both.path(), "paru");
        write_fake_helper(both.path(), "yay");
        let paru_run = run_shell_body(&body, both.path());
        assert!(paru_run.status.success(), "paru branch should succeed");
        assert_eq!(stdout_lines(&paru_run), {
            let mut expected = vec!["paru".to_string()];
            expected.extend(expected_args.iter().map(ToString::to_string));
            expected
        });

        let yay_only = tempfile::tempdir().expect("temporary fixture directory");
        write_fake_helper(yay_only.path(), "yay");
        let yay_run = run_shell_body(&body, yay_only.path());
        assert!(yay_run.status.success(), "yay branch should succeed");
        assert_eq!(stdout_lines(&yay_run), {
            let mut expected = vec!["yay".to_string()];
            expected.extend(expected_args.iter().map(ToString::to_string));
            expected
        });

        let empty = tempfile::tempdir().expect("temporary fixture directory");
        let missing = run_shell_body(&body, empty.path());
        assert_eq!(
            missing.status.code(),
            Some(127),
            "missing helper must exit non-zero with 127"
        );
        assert!(
            String::from_utf8_lossy(&missing.stdout).trim().is_empty(),
            "missing-helper message must not go to stdout"
        );
        assert_eq!(
            String::from_utf8_lossy(&missing.stderr).trim(),
            NO_AUR_HELPER_MESSAGE
        );
    }

    /// What: Write an executable fake AUR helper that records its argv.
    ///
    /// Inputs:
    /// - `dir`: Directory placed on the isolated `PATH`.
    /// - `name`: Helper basename (`paru` or `yay`).
    ///
    /// Output:
    /// - Side effect: an executable script printing its own name and arguments.
    ///
    /// Details:
    /// - Keeps helper-preference assertions deterministic without touching the
    ///   host system.
    #[cfg(unix)]
    fn write_fake_helper(dir: &std::path::Path, name: &str) {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join(name);
        std::fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' \"{name}\" \"$@\"\n"),
        )
        .expect("write fake helper");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("mark fake helper executable");
    }

    /// What: Run a shell fallback body with an isolated `PATH`.
    ///
    /// Inputs:
    /// - `body`: Shell snippet produced by a fallback builder.
    /// - `path_dir`: Only directory exposed through `PATH`.
    ///
    /// Output:
    /// - Captured `std::process::Output` of `/bin/sh -c <body>`.
    ///
    /// Details:
    /// - `/bin/sh` is invoked by absolute path so the isolated `PATH` cannot
    ///   affect interpreter lookup.
    #[cfg(unix)]
    fn run_shell_body(body: &str, path_dir: &std::path::Path) -> std::process::Output {
        std::process::Command::new("/bin/sh")
            .args(["-c", body])
            .env("PATH", path_dir)
            .output()
            .expect("shell should run")
    }

    /// What: Split captured stdout into trimmed lines.
    ///
    /// Inputs:
    /// - `output`: Process output from a fallback execution.
    ///
    /// Output:
    /// - Owned lines with trailing newline removed.
    ///
    /// Details:
    /// - Used to compare recorded helper argv exactly.
    #[cfg(unix)]
    fn stdout_lines(output: &std::process::Output) -> Vec<String> {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(ToString::to_string)
            .collect()
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
