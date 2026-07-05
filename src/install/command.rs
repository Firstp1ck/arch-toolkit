//! Command builders for package installation, removal, and updates.
//!
//! All builders return [`CommandSpec`] values — arch-toolkit never executes
//! commands. Dry runs are the caller's choice: display `spec.to_shell_string()`
//! instead of spawning.

use crate::error::Result;
use crate::types::install::{AurHelper, CascadeMode, CommandSpec, InstallOptions, PrivilegeTool};

use super::shell::validate_package_names;

/// Message printed by shell-fallback bodies when no AUR helper is installed.
///
/// Matches Pacsea's terminal-install error text so migrating callers keep
/// byte-identical output.
pub const NO_AUR_HELPER_MESSAGE: &str = "No AUR helper (paru/yay) found.";

/// What: Build a pacman install command for official repository packages.
///
/// Inputs:
/// - `names`: Package names to install (validated against the safe-name allowlist).
/// - `options`: Flag options (`needed`, `noconfirm`; `aur_only` is ignored).
///
/// Output:
/// - `Ok(CommandSpec)` like `pacman -S --needed --noconfirm <names...>`.
///
/// Details:
/// - Does NOT prefix a privilege tool; use [`with_privilege`] for that.
/// - Omit `--needed` (set `options.needed = false`) for explicit reinstalls,
///   mirroring Pacsea's reinstall path.
///
/// # Errors
///
/// Returns `ArchToolkitError::InvalidPackageName` when a name fails validation,
/// or `ArchToolkitError::EmptyInput` when `names` is empty.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_pacman_install;
/// use arch_toolkit::types::install::InstallOptions;
///
/// let spec = build_pacman_install(&["ripgrep", "fd"], &InstallOptions::default())?;
/// assert_eq!(spec.to_shell_string(), "pacman -S --needed --noconfirm ripgrep fd");
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn build_pacman_install<S: AsRef<str>>(
    names: &[S],
    options: &InstallOptions,
) -> Result<CommandSpec> {
    validate_non_empty(names, "pacman install")?;
    validate_package_names(names, "pacman install")?;
    let mut args = vec!["-S".to_string()];
    if options.needed {
        args.push("--needed".to_string());
    }
    if options.noconfirm {
        args.push("--noconfirm".to_string());
    }
    args.extend(names.iter().map(|n| n.as_ref().to_string()));
    Ok(CommandSpec {
        program: "pacman".to_string(),
        args,
    })
}

/// What: Build an AUR helper install command for AUR packages.
///
/// Inputs:
/// - `helper`: The AUR helper to use (from caller config or [`super::detect_aur_helper`]).
/// - `names`: AUR package names to install (validated).
/// - `options`: Flag options (`needed`, `noconfirm`, `aur_only`).
///
/// Output:
/// - `Ok(CommandSpec)` like `paru -S --aur --needed --noconfirm <names...>`.
///
/// Details:
/// - `--aur` (when `options.aur_only`) ensures helpers do not prefer a sync
///   database (e.g., Chaotic-AUR) when the same name exists on the AUR —
///   matching Pacsea's `aur_install_helper_flags`.
/// - AUR helpers must NOT run under sudo; they invoke sudo themselves for the
///   pacman step. Do not wrap the result in [`with_privilege`].
///
/// # Errors
///
/// Returns `ArchToolkitError::InvalidPackageName` when a name fails validation,
/// or `ArchToolkitError::EmptyInput` when `names` is empty.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_aur_install;
/// use arch_toolkit::types::install::{AurHelper, InstallOptions};
///
/// let spec = build_aur_install(AurHelper::Paru, &["yay-bin"], &InstallOptions::default())?;
/// assert_eq!(spec.to_shell_string(), "paru -S --aur --needed --noconfirm yay-bin");
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn build_aur_install<S: AsRef<str>>(
    helper: AurHelper,
    names: &[S],
    options: &InstallOptions,
) -> Result<CommandSpec> {
    validate_non_empty(names, "AUR install")?;
    validate_package_names(names, "AUR install")?;
    let mut args = vec!["-S".to_string()];
    if options.aur_only {
        args.push("--aur".to_string());
    }
    if options.needed {
        args.push("--needed".to_string());
    }
    if options.noconfirm {
        args.push("--noconfirm".to_string());
    }
    args.extend(names.iter().map(|n| n.as_ref().to_string()));
    Ok(CommandSpec {
        program: helper.binary_name().to_string(),
        args,
    })
}

/// What: Build a pacman remove command with the requested cascade level.
///
/// Inputs:
/// - `names`: Package names to remove (validated).
/// - `cascade`: Cascade level (`-R`, `-Rs`, or `-Rns`).
/// - `noconfirm`: Pass `--noconfirm` for non-interactive removal.
///
/// Output:
/// - `Ok(CommandSpec)` like `pacman -Rns --noconfirm <names...>`.
///
/// Details:
/// - Does NOT prefix a privilege tool; use [`with_privilege`] for that.
/// - Cascade semantics ported from Pacsea's `CascadeMode`.
///
/// # Errors
///
/// Returns `ArchToolkitError::InvalidPackageName` when a name fails validation,
/// or `ArchToolkitError::EmptyInput` when `names` is empty.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_remove_command;
/// use arch_toolkit::types::install::CascadeMode;
///
/// let spec = build_remove_command(&["ripgrep"], CascadeMode::CascadeWithConfigs, true)?;
/// assert_eq!(spec.to_shell_string(), "pacman -Rns --noconfirm ripgrep");
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn build_remove_command<S: AsRef<str>>(
    names: &[S],
    cascade: CascadeMode,
    noconfirm: bool,
) -> Result<CommandSpec> {
    validate_non_empty(names, "remove")?;
    validate_package_names(names, "remove")?;
    let mut args = vec![cascade.flag().to_string()];
    if noconfirm {
        args.push("--noconfirm".to_string());
    }
    args.extend(names.iter().map(|n| n.as_ref().to_string()));
    Ok(CommandSpec {
        program: "pacman".to_string(),
        args,
    })
}

/// What: Build a full-system update command.
///
/// Inputs:
/// - `helper`: When `Some`, use the AUR helper (`paru -Syu`) which updates both
///   official and AUR packages. When `None`, use plain `pacman -Syu`.
/// - `noconfirm`: Pass `--noconfirm` for non-interactive updates.
///
/// Output:
/// - `CommandSpec` like `paru -Syu --noconfirm` or `pacman -Syu --noconfirm`.
///
/// Details:
/// - The pacman variant requires privilege wrapping ([`with_privilege`]);
///   helper variants must not be wrapped (helpers call sudo themselves).
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_update_command;
/// use arch_toolkit::types::install::AurHelper;
///
/// let pacman = build_update_command(None, true);
/// assert_eq!(pacman.to_shell_string(), "pacman -Syu --noconfirm");
///
/// let helper = build_update_command(Some(AurHelper::Yay), false);
/// assert_eq!(helper.to_shell_string(), "yay -Syu");
/// ```
#[must_use]
pub fn build_update_command(helper: Option<AurHelper>, noconfirm: bool) -> CommandSpec {
    let program = helper.map_or("pacman", AurHelper::binary_name).to_string();
    let mut args = vec!["-Syu".to_string()];
    if noconfirm {
        args.push("--noconfirm".to_string());
    }
    CommandSpec { program, args }
}

/// What: Build a full-system update command that force-refreshes sync databases.
///
/// Inputs:
/// - `helper`: When `Some`, use the AUR helper; when `None`, plain pacman.
/// - `noconfirm`: Pass `--noconfirm` for non-interactive updates.
///
/// Output:
/// - `CommandSpec` like `pacman -Syyu --noconfirm`.
///
/// Details:
/// - `-Syyu` re-downloads all sync databases even when they appear up to date;
///   use after mirror changes (mirrors Pacsea's force-sync update option).
/// - Same privilege rules as [`build_update_command`].
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_force_sync_update_command;
///
/// let spec = build_force_sync_update_command(None, true);
/// assert_eq!(spec.to_shell_string(), "pacman -Syyu --noconfirm");
/// ```
#[must_use]
pub fn build_force_sync_update_command(helper: Option<AurHelper>, noconfirm: bool) -> CommandSpec {
    let program = helper.map_or("pacman", AurHelper::binary_name).to_string();
    let mut args = vec!["-Syyu".to_string()];
    if noconfirm {
        args.push("--noconfirm".to_string());
    }
    CommandSpec { program, args }
}

/// What: Build an AUR-only update command (`-Sua`).
///
/// Inputs:
/// - `helper`: The AUR helper to run the update with.
/// - `noconfirm`: Pass `--noconfirm` for non-interactive updates.
///
/// Output:
/// - `CommandSpec` like `paru -Sua --noconfirm`.
///
/// Details:
/// - Updates AUR packages only, leaving official packages to a separate
///   `pacman -Syu` step — mirroring Pacsea's split system-update flow where
///   the AUR step runs conditionally after the pacman step succeeds.
/// - Must NOT be wrapped in [`with_privilege`]; helpers escalate internally.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_aur_update_command;
/// use arch_toolkit::types::install::AurHelper;
///
/// let spec = build_aur_update_command(AurHelper::Paru, true);
/// assert_eq!(spec.to_shell_string(), "paru -Sua --noconfirm");
/// ```
#[must_use]
pub fn build_aur_update_command(helper: AurHelper, noconfirm: bool) -> CommandSpec {
    let mut args = vec!["-Sua".to_string()];
    if noconfirm {
        args.push("--noconfirm".to_string());
    }
    CommandSpec {
        program: helper.binary_name().to_string(),
        args,
    }
}

/// What: Build a shell body that installs AUR packages with runtime helper fallback.
///
/// Inputs:
/// - `names`: AUR package names to install (validated).
/// - `options`: Flag options (`needed`, `noconfirm`, `aur_only`).
///
/// Output:
/// - `Ok(String)` with a POSIX shell snippet that picks `paru`, then `yay`,
///   at execution time, or prints [`NO_AUR_HELPER_MESSAGE`].
///
/// Details:
/// - Unlike [`build_aur_install`], helper selection happens inside the spawned
///   shell (the terminal's `PATH`), not in the calling process — matching
///   Pacsea's `aur_install_body`. Prefer this when the command runs in an
///   external terminal whose environment may differ from the caller's.
/// - Names pass the same strict validation as all builders, so interpolating
///   them into the shell string is safe without quoting.
///
/// # Errors
///
/// Returns `ArchToolkitError::InvalidPackageName` when a name fails validation,
/// or `ArchToolkitError::EmptyInput` when `names` is empty.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::aur_install_shell_fallback;
/// use arch_toolkit::types::install::InstallOptions;
///
/// let body = aur_install_shell_fallback(&["yay-bin"], &InstallOptions::default())?;
/// assert!(body.contains("if command -v paru >/dev/null 2>&1; then paru"));
/// assert!(body.contains("elif command -v yay >/dev/null 2>&1; then yay"));
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn aur_install_shell_fallback<S: AsRef<str>>(
    names: &[S],
    options: &InstallOptions,
) -> Result<String> {
    validate_non_empty(names, "AUR install")?;
    validate_package_names(names, "AUR install")?;
    let mut flags = String::from("-S");
    if options.aur_only {
        flags.push_str(" --aur");
    }
    if options.needed {
        flags.push_str(" --needed");
    }
    if options.noconfirm {
        flags.push_str(" --noconfirm");
    }
    let joined = names
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(" ");
    Ok(helper_fallback_body(&format!("{flags} {joined}")))
}

/// What: Build a shell body that updates AUR packages with runtime helper fallback.
///
/// Inputs:
/// - `noconfirm`: Pass `--noconfirm` for non-interactive updates.
///
/// Output:
/// - A POSIX shell snippet running `paru -Sua` / `yay -Sua`, or printing
///   [`NO_AUR_HELPER_MESSAGE`] when neither helper exists.
///
/// Details:
/// - Shell-time counterpart of [`build_aur_update_command`], for callers that
///   spawn the update in an external terminal (Pacsea's system-update flow).
///
/// # Example
///
/// ```
/// use arch_toolkit::install::aur_update_shell_fallback;
///
/// let body = aur_update_shell_fallback(true);
/// assert!(body.contains("paru -Sua --noconfirm"));
/// ```
#[must_use]
pub fn aur_update_shell_fallback(noconfirm: bool) -> String {
    let flags = if noconfirm {
        "-Sua --noconfirm"
    } else {
        "-Sua"
    };
    helper_fallback_body(flags)
}

/// What: Wrap helper arguments in the paru → yay runtime-fallback shell body.
///
/// Inputs:
/// - `tail`: Flags and package names appended to the chosen helper.
///
/// Output:
/// - Parenthesized `if/elif/else` snippet matching Pacsea's `aur_install_body`.
fn helper_fallback_body(tail: &str) -> String {
    format!(
        "(if command -v paru >/dev/null 2>&1; then paru {tail}; \
         elif command -v yay >/dev/null 2>&1; then yay {tail}; \
         else echo '{NO_AUR_HELPER_MESSAGE}'; fi)"
    )
}

/// What: Wrap a command with a privilege escalation tool prefix.
///
/// Inputs:
/// - `tool`: The privilege tool (`sudo` or `doas`).
/// - `spec`: The command to wrap.
///
/// Output:
/// - New `CommandSpec` like `sudo pacman -S ...`.
///
/// Details:
/// - Pure transformation: the original program becomes the first argument.
/// - Do NOT wrap AUR helper commands; helpers escalate internally.
/// - Password handling (stdin piping, askpass) is intentionally out of scope.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::{build_update_command, with_privilege};
/// use arch_toolkit::types::install::PrivilegeTool;
///
/// let spec = with_privilege(PrivilegeTool::Sudo, build_update_command(None, true));
/// assert_eq!(spec.to_shell_string(), "sudo pacman -Syu --noconfirm");
/// ```
#[must_use]
pub fn with_privilege(tool: PrivilegeTool, spec: CommandSpec) -> CommandSpec {
    let mut args = Vec::with_capacity(spec.args.len() + 1);
    args.push(spec.program);
    args.extend(spec.args);
    CommandSpec {
        program: tool.binary_name().to_string(),
        args,
    }
}

/// What: Reject empty name lists with a descriptive error.
///
/// Inputs:
/// - `names`: The name list to check.
/// - `context`: Operation context for the error message.
///
/// Output:
/// - `Ok(())` when non-empty, `Err(ArchToolkitError::EmptyInput)` otherwise.
///
/// Details:
/// - Prevents building commands like `pacman -S --noconfirm` with no targets.
fn validate_non_empty<S: AsRef<str>>(names: &[S], context: &str) -> Result<()> {
    if names.is_empty() {
        return Err(crate::error::ArchToolkitError::EmptyInput {
            field: "names".to_string(),
            message: format!("at least one package name is required for {context}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ArchToolkitError;

    #[test]
    /// What: Verify pacman install flag combinations (fresh install vs reinstall).
    ///
    /// Inputs:
    /// - Default options and reinstall options (needed=false).
    ///
    /// Output:
    /// - `--needed` present only for fresh installs.
    ///
    /// Details:
    /// - Mirrors Pacsea's `-S --needed --noconfirm` vs `-S --noconfirm` split.
    fn pacman_install_flags() {
        let fresh =
            build_pacman_install(&["ripgrep"], &InstallOptions::default()).expect("build fresh");
        assert_eq!(
            fresh.to_shell_string(),
            "pacman -S --needed --noconfirm ripgrep"
        );

        let reinstall_opts = InstallOptions {
            needed: false,
            ..Default::default()
        };
        let reinstall =
            build_pacman_install(&["ripgrep"], &reinstall_opts).expect("build reinstall");
        assert_eq!(reinstall.to_shell_string(), "pacman -S --noconfirm ripgrep");

        let interactive = InstallOptions {
            noconfirm: false,
            ..Default::default()
        };
        let spec = build_pacman_install(&["a", "b"], &interactive).expect("build interactive");
        assert_eq!(spec.to_shell_string(), "pacman -S --needed a b");
    }

    #[test]
    /// What: Verify AUR install commands include `--aur` and helper preference.
    ///
    /// Inputs:
    /// - Paru and yay helpers with default and reinstall options.
    ///
    /// Output:
    /// - Flag sets matching Pacsea's `aur_install_helper_flags`.
    ///
    /// Details:
    /// - Reinstall path omits `--needed`; `aur_only=false` omits `--aur`.
    fn aur_install_flags() {
        let spec = build_aur_install(AurHelper::Paru, &["yay-bin"], &InstallOptions::default())
            .expect("build");
        assert_eq!(
            spec.to_shell_string(),
            "paru -S --aur --needed --noconfirm yay-bin"
        );

        let reinstall = InstallOptions {
            needed: false,
            ..Default::default()
        };
        let spec2 = build_aur_install(AurHelper::Yay, &["yay-bin"], &reinstall).expect("build");
        assert_eq!(spec2.to_shell_string(), "yay -S --aur --noconfirm yay-bin");

        let no_aur_flag = InstallOptions {
            aur_only: false,
            ..Default::default()
        };
        let spec3 = build_aur_install(AurHelper::Paru, &["x"], &no_aur_flag).expect("build");
        assert_eq!(spec3.to_shell_string(), "paru -S --needed --noconfirm x");
    }

    #[test]
    /// What: Verify remove commands map cascade modes to pacman flags.
    ///
    /// Inputs:
    /// - All three cascade modes.
    ///
    /// Output:
    /// - `-R`, `-Rs`, `-Rns` respectively, with optional `--noconfirm`.
    ///
    /// Details:
    /// - Matches Pacsea's `CascadeMode::flag()` semantics.
    fn remove_cascade_modes() {
        let basic = build_remove_command(&["pkg"], CascadeMode::Basic, false).expect("build basic");
        assert_eq!(basic.to_shell_string(), "pacman -R pkg");

        let cascade =
            build_remove_command(&["pkg"], CascadeMode::Cascade, true).expect("build cascade");
        assert_eq!(cascade.to_shell_string(), "pacman -Rs --noconfirm pkg");

        let full = build_remove_command(&["a", "b"], CascadeMode::CascadeWithConfigs, true)
            .expect("build full");
        assert_eq!(full.to_shell_string(), "pacman -Rns --noconfirm a b");
    }

    #[test]
    /// What: Verify update command builder for pacman and helper variants.
    ///
    /// Inputs:
    /// - `None` (pacman) and `Some(helper)` variants.
    ///
    /// Output:
    /// - `pacman -Syu` / `<helper> -Syu` with optional `--noconfirm`.
    ///
    /// Details:
    /// - Helper variant updates both official and AUR packages.
    fn update_commands() {
        assert_eq!(
            build_update_command(None, true).to_shell_string(),
            "pacman -Syu --noconfirm"
        );
        assert_eq!(
            build_update_command(Some(AurHelper::Paru), false).to_shell_string(),
            "paru -Syu"
        );
    }

    #[test]
    /// What: Verify force-sync and AUR-only update builders.
    ///
    /// Inputs:
    /// - Pacman and helper variants with and without `--noconfirm`.
    ///
    /// Output:
    /// - `-Syyu` and `-Sua` flag sets matching Pacsea's system-update flow.
    ///
    /// Details:
    /// - `-Sua` must target the helper only; `-Syyu` force-refreshes databases.
    fn force_sync_and_aur_only_updates() {
        assert_eq!(
            build_force_sync_update_command(None, true).to_shell_string(),
            "pacman -Syyu --noconfirm"
        );
        assert_eq!(
            build_force_sync_update_command(Some(AurHelper::Yay), false).to_shell_string(),
            "yay -Syyu"
        );
        assert_eq!(
            build_aur_update_command(AurHelper::Paru, true).to_shell_string(),
            "paru -Sua --noconfirm"
        );
        assert_eq!(
            build_aur_update_command(AurHelper::Yay, false).to_shell_string(),
            "yay -Sua"
        );
    }

    #[test]
    /// What: Verify runtime-fallback shell bodies match Pacsea's format.
    ///
    /// Inputs:
    /// - Install and update fallback bodies with default options.
    ///
    /// Output:
    /// - Parenthesized paru → yay `if/elif/else` with the exact error message.
    ///
    /// Details:
    /// - Helper selection happens at shell execution time, not plan time.
    fn shell_fallback_bodies() {
        let body = aur_install_shell_fallback(&["yay-bin"], &InstallOptions::default())
            .expect("build body");
        assert_eq!(
            body,
            "(if command -v paru >/dev/null 2>&1; then paru -S --aur --needed --noconfirm yay-bin; \
             elif command -v yay >/dev/null 2>&1; then yay -S --aur --needed --noconfirm yay-bin; \
             else echo 'No AUR helper (paru/yay) found.'; fi)"
        );

        let update = aur_update_shell_fallback(false);
        assert!(update.contains("paru -Sua;"));
        assert!(update.contains(NO_AUR_HELPER_MESSAGE));

        let inj = aur_install_shell_fallback(&["bad;rm -rf /"], &InstallOptions::default());
        assert!(matches!(
            inj,
            Err(ArchToolkitError::InvalidPackageName { .. })
        ));

        let empty: [&str; 0] = [];
        assert!(matches!(
            aur_install_shell_fallback(&empty, &InstallOptions::default()),
            Err(ArchToolkitError::EmptyInput { .. })
        ));
    }

    #[test]
    /// What: Verify privilege wrapping prepends the tool and shifts the program.
    ///
    /// Inputs:
    /// - A pacman spec wrapped with sudo and doas.
    ///
    /// Output:
    /// - `sudo pacman ...` / `doas pacman ...`.
    ///
    /// Details:
    /// - The wrapped spec must preserve all original arguments in order.
    fn privilege_wrapping() {
        let spec = build_pacman_install(&["vim"], &InstallOptions::default()).expect("build");
        let sudo = with_privilege(PrivilegeTool::Sudo, spec.clone());
        assert_eq!(
            sudo.to_shell_string(),
            "sudo pacman -S --needed --noconfirm vim"
        );
        let doas = with_privilege(PrivilegeTool::Doas, spec);
        assert_eq!(
            doas.to_shell_string(),
            "doas pacman -S --needed --noconfirm vim"
        );
    }

    #[test]
    /// What: Verify builders reject invalid names and empty lists.
    ///
    /// Inputs:
    /// - Injection attempt and empty slice.
    ///
    /// Output:
    /// - `InvalidPackageName` and `EmptyInput` errors respectively.
    ///
    /// Details:
    /// - Defense-in-depth: names are validated before any command is produced.
    fn validation_errors() {
        let inj = build_pacman_install(&["good", "bad;rm -rf /"], &InstallOptions::default());
        assert!(matches!(
            inj,
            Err(ArchToolkitError::InvalidPackageName { .. })
        ));

        let empty: [&str; 0] = [];
        let none = build_pacman_install(&empty, &InstallOptions::default());
        assert!(matches!(none, Err(ArchToolkitError::EmptyInput { .. })));

        let aur_inj = build_aur_install(AurHelper::Paru, &["$(evil)"], &InstallOptions::default());
        assert!(matches!(
            aur_inj,
            Err(ArchToolkitError::InvalidPackageName { .. })
        ));

        let rm_inj = build_remove_command(&["a b"], CascadeMode::Basic, true);
        assert!(matches!(
            rm_inj,
            Err(ArchToolkitError::InvalidPackageName { .. })
        ));
    }
}
