//! Batch install planning: split targets between pacman and an AUR helper.

use std::collections::HashSet;
use std::hash::BuildHasher;

use crate::error::{ArchToolkitError, Result};
use crate::types::dependency::{PackageRef, PackageSource};
use crate::types::install::{AurHelper, CommandSpec, InstallOptions, PrivilegeTool};

use super::command::{build_aur_install, build_pacman_install, with_privilege};

/// What: The result of planning a batch installation.
///
/// Inputs:
/// - Produced by [`build_batch_install`].
///
/// Output:
/// - Between zero and two commands (one pacman batch, one AUR helper batch)
///   plus the package names routed to each.
///
/// Details:
/// - Commands should be executed in order: official packages first, then AUR
///   packages (AUR builds may depend on official packages installed earlier) —
///   matching Pacsea's mixed-install chaining.
#[derive(Clone, Debug, Default)]
pub struct InstallPlan {
    /// Commands to execute in order (pacman batch first, then AUR helper batch).
    pub commands: Vec<CommandSpec>,
    /// Names routed to the pacman command.
    pub official: Vec<String>,
    /// Names routed to the AUR helper command.
    pub aur: Vec<String>,
}

impl InstallPlan {
    /// What: Render the plan as a single `&&`-chained shell command line.
    ///
    /// Inputs:
    /// - `&self`: The planned commands, in execution order.
    ///
    /// Output:
    /// - Shell string like `sudo pacman -S ... && paru -S --aur ...`;
    ///   empty string for an empty plan.
    ///
    /// Details:
    /// - `&&` chaining stops the AUR step when the pacman step fails —
    ///   matching Pacsea's mixed-install semantics. Callers executing the
    ///   `commands` vector directly must replicate this by checking each
    ///   command's exit status before running the next.
    ///
    /// # Example
    ///
    /// ```
    /// use arch_toolkit::install::build_batch_install;
    /// use arch_toolkit::types::install::{AurHelper, InstallOptions};
    /// use arch_toolkit::PackageRef;
    ///
    /// let targets = vec![
    ///     PackageRef::official("ripgrep", "14.0.0", "extra", "x86_64"),
    ///     PackageRef::aur("yay-bin", "12.0.0"),
    /// ];
    /// let plan = build_batch_install(
    ///     &targets,
    ///     Some(AurHelper::Paru),
    ///     None,
    ///     &InstallOptions::default(),
    ///     None::<&std::collections::HashSet<String>>,
    /// )?;
    /// assert_eq!(
    ///     plan.to_shell_string(),
    ///     "pacman -S --needed --noconfirm ripgrep && paru -S --aur --needed --noconfirm yay-bin"
    /// );
    /// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
    /// ```
    #[must_use]
    pub fn to_shell_string(&self) -> String {
        self.commands
            .iter()
            .map(CommandSpec::to_shell_string)
            .collect::<Vec<_>>()
            .join(" && ")
    }
}

/// What: Plan a batch installation by splitting targets between pacman and an AUR helper.
///
/// Inputs:
/// - `targets`: Packages to install; the `source` field routes each to pacman or the helper.
/// - `helper`: AUR helper to use for AUR targets. Required when any target is from the AUR.
/// - `privilege`: When `Some`, the pacman command is wrapped (e.g., `sudo pacman ...`).
///   AUR helper commands are never wrapped (helpers escalate internally).
/// - `options`: Flag options; `options.needed` is refined per group when `installed` is given.
/// - `installed`: Optional set of installed package names. When any name in a group is
///   already installed, `--needed` is dropped for that group so reinstalls proceed —
///   mirroring Pacsea's batch reinstall detection. Pass `None` to use `options.needed` as-is.
///
/// Output:
/// - `Ok(InstallPlan)` with grouped commands and routed names.
///
/// Details:
/// - Official packages are grouped into a single `pacman` invocation, AUR packages
///   into a single helper invocation (from Pacsea's `build_batch_install_command`).
/// - Empty `targets` produces an empty plan (no error), so callers can pass
///   through selection results unchecked.
/// - arch-toolkit never executes the plan; run the commands with
///   `spec.to_command()` or display them with `spec.to_shell_string()` (dry run).
///
/// # Errors
///
/// - `ArchToolkitError::InvalidInput` when AUR targets are present but `helper` is `None`.
/// - `ArchToolkitError::InvalidPackageName` when a target name fails validation.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::build_batch_install;
/// use arch_toolkit::types::install::{AurHelper, InstallOptions, PrivilegeTool};
/// use arch_toolkit::{PackageRef, PackageSource};
///
/// let targets = vec![
///     PackageRef::official("ripgrep", "14.0.0", "extra", "x86_64"),
///     PackageRef::aur("yay-bin", "12.0.0"),
/// ];
/// let plan = build_batch_install(
///     &targets,
///     Some(AurHelper::Paru),
///     Some(PrivilegeTool::Sudo),
///     &InstallOptions::default(),
///     None::<&std::collections::HashSet<String>>,
/// )?;
/// assert_eq!(plan.commands.len(), 2);
/// assert_eq!(plan.commands[0].to_shell_string(), "sudo pacman -S --needed --noconfirm ripgrep");
/// assert_eq!(plan.commands[1].to_shell_string(), "paru -S --aur --needed --noconfirm yay-bin");
/// # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
/// ```
pub fn build_batch_install<S: BuildHasher>(
    targets: &[PackageRef],
    helper: Option<AurHelper>,
    privilege: Option<PrivilegeTool>,
    options: &InstallOptions,
    installed: Option<&HashSet<String, S>>,
) -> Result<InstallPlan> {
    let mut official: Vec<String> = Vec::new();
    let mut aur: Vec<String> = Vec::new();
    for target in targets {
        match &target.source {
            PackageSource::Official { .. } => official.push(target.name.clone()),
            PackageSource::Aur => aur.push(target.name.clone()),
        }
    }

    let mut commands = Vec::new();

    if !official.is_empty() {
        let opts = group_options(*options, &official, installed);
        let mut spec = build_pacman_install(&official, &opts)?;
        if let Some(tool) = privilege {
            spec = with_privilege(tool, spec);
        }
        commands.push(spec);
    }

    if !aur.is_empty() {
        let Some(helper) = helper else {
            return Err(ArchToolkitError::InvalidInput(format!(
                "{} AUR target(s) present but no AUR helper provided; \
                 pass one explicitly or use detect_aur_helper()",
                aur.len()
            )));
        };
        let opts = group_options(*options, &aur, installed);
        commands.push(build_aur_install(helper, &aur, &opts)?);
    }

    Ok(InstallPlan {
        commands,
        official,
        aur,
    })
}

/// What: Refine install options for a target group based on installed state.
///
/// Inputs:
/// - `options`: Base options from the caller.
/// - `names`: The group's package names.
/// - `installed`: Optional installed-package set.
///
/// Output:
/// - Options with `needed` cleared when any group member is already installed.
///
/// Details:
/// - Mirrors Pacsea's batch reinstall detection: a group containing an
///   already-installed package drops `--needed` so pacman/helper reinstalls it.
fn group_options<S: BuildHasher>(
    options: InstallOptions,
    names: &[String],
    installed: Option<&HashSet<String, S>>,
) -> InstallOptions {
    let has_reinstall = installed.is_some_and(|set| names.iter().any(|name| set.contains(name)));
    InstallOptions {
        needed: options.needed && !has_reinstall,
        ..options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn official(name: &str) -> PackageRef {
        PackageRef::official(name, "1.0", "extra", "x86_64")
    }

    fn aur_pkg(name: &str) -> PackageRef {
        PackageRef::aur(name, "1.0")
    }

    const NO_INSTALLED: Option<&HashSet<String>> = None;

    #[test]
    /// What: Verify official-only batches produce a single privileged pacman command.
    ///
    /// Inputs:
    /// - Two official targets with sudo privilege.
    ///
    /// Output:
    /// - One command: `sudo pacman -S --needed --noconfirm a b`.
    ///
    /// Details:
    /// - Grouping mirrors Pacsea's single-invocation batching.
    fn official_only_batch() {
        let targets = vec![official("a"), official("b")];
        let plan = build_batch_install(
            &targets,
            None,
            Some(PrivilegeTool::Sudo),
            &InstallOptions::default(),
            NO_INSTALLED,
        )
        .expect("plan");
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(
            plan.commands[0].to_shell_string(),
            "sudo pacman -S --needed --noconfirm a b"
        );
        assert_eq!(plan.official, ["a", "b"]);
        assert!(plan.aur.is_empty());
    }

    #[test]
    /// What: Verify AUR-only batches produce a single unprivileged helper command.
    ///
    /// Inputs:
    /// - Two AUR targets with paru and sudo configured.
    ///
    /// Output:
    /// - One command without sudo: `paru -S --aur --needed --noconfirm x y`.
    ///
    /// Details:
    /// - Helpers must never be wrapped in sudo; they escalate internally.
    fn aur_only_batch_never_privileged() {
        let targets = vec![aur_pkg("x"), aur_pkg("y")];
        let plan = build_batch_install(
            &targets,
            Some(AurHelper::Paru),
            Some(PrivilegeTool::Sudo),
            &InstallOptions::default(),
            NO_INSTALLED,
        )
        .expect("plan");
        assert_eq!(plan.commands.len(), 1);
        assert_eq!(
            plan.commands[0].to_shell_string(),
            "paru -S --aur --needed --noconfirm x y"
        );
    }

    #[test]
    /// What: Verify mixed batches order pacman before the AUR helper.
    ///
    /// Inputs:
    /// - One official and one AUR target.
    ///
    /// Output:
    /// - Two commands: privileged pacman first, then the helper.
    ///
    /// Details:
    /// - AUR builds may depend on official packages installed in the first step.
    fn mixed_batch_ordering() {
        let targets = vec![aur_pkg("helper-pkg"), official("base-pkg")];
        let plan = build_batch_install(
            &targets,
            Some(AurHelper::Yay),
            Some(PrivilegeTool::Doas),
            &InstallOptions::default(),
            NO_INSTALLED,
        )
        .expect("plan");
        assert_eq!(plan.commands.len(), 2);
        assert!(
            plan.commands[0]
                .to_shell_string()
                .starts_with("doas pacman")
        );
        assert!(
            plan.commands[1]
                .to_shell_string()
                .starts_with("yay -S --aur")
        );
    }

    #[test]
    /// What: Verify reinstall detection drops `--needed` per group.
    ///
    /// Inputs:
    /// - Installed set containing one official target; AUR target not installed.
    ///
    /// Output:
    /// - Pacman command without `--needed`; helper command keeps `--needed`.
    ///
    /// Details:
    /// - Mirrors Pacsea's per-group `has_reinstall` logic.
    fn reinstall_detection_per_group() {
        let targets = vec![official("vim"), aur_pkg("fresh-pkg")];
        let installed: HashSet<String> = HashSet::from(["vim".to_string()]);
        let plan = build_batch_install(
            &targets,
            Some(AurHelper::Paru),
            None,
            &InstallOptions::default(),
            Some(&installed),
        )
        .expect("plan");
        assert_eq!(
            plan.commands[0].to_shell_string(),
            "pacman -S --noconfirm vim"
        );
        assert_eq!(
            plan.commands[1].to_shell_string(),
            "paru -S --aur --needed --noconfirm fresh-pkg"
        );
    }

    #[test]
    /// What: Verify AUR targets without a helper produce a clear error.
    ///
    /// Inputs:
    /// - One AUR target, `helper = None`.
    ///
    /// Output:
    /// - `InvalidInput` error mentioning the missing helper.
    ///
    /// Details:
    /// - Callers should detect or configure a helper before planning.
    fn aur_without_helper_errors() {
        let targets = vec![aur_pkg("x")];
        let err = build_batch_install(
            &targets,
            None,
            None,
            &InstallOptions::default(),
            NO_INSTALLED,
        )
        .expect_err("should fail");
        assert!(matches!(err, ArchToolkitError::InvalidInput(_)));
        assert!(err.to_string().contains("AUR helper"));
    }

    #[test]
    /// What: Verify an empty target list yields an empty plan without error.
    ///
    /// Inputs:
    /// - Empty target slice.
    ///
    /// Output:
    /// - Plan with no commands and no routed names.
    ///
    /// Details:
    /// - Allows callers to pass selection results through unchecked.
    fn empty_targets_empty_plan() {
        let plan = build_batch_install(&[], None, None, &InstallOptions::default(), NO_INSTALLED)
            .expect("plan");
        assert!(plan.commands.is_empty());
        assert!(plan.official.is_empty());
        assert!(plan.aur.is_empty());
    }
}
