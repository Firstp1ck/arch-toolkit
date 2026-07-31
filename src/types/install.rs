//! Install-related data types for package installation command building.

use serde::{Deserialize, Serialize};

/// What: AUR helper used to install AUR packages.
///
/// Inputs:
/// - Selected explicitly by callers or via `install::detect_aur_helper()`.
///
/// Output:
/// - Determines which helper binary appears in built AUR install commands.
///
/// Details:
/// - Preference order follows Pacsea: `paru` first, then `yay`.
/// - Both helpers accept the same `-S --aur --needed --noconfirm` flag set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AurHelper {
    /// The `paru` AUR helper (preferred).
    Paru,
    /// The `yay` AUR helper (fallback).
    Yay,
}

impl AurHelper {
    /// What: Return the shell binary name for this helper.
    ///
    /// Inputs: None.
    ///
    /// Output: `"paru"` or `"yay"`.
    ///
    /// Details: Used in command construction and `PATH` lookups.
    #[must_use]
    pub const fn binary_name(self) -> &'static str {
        match self {
            Self::Paru => "paru",
            Self::Yay => "yay",
        }
    }
}

impl std::fmt::Display for AurHelper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.binary_name())
    }
}

/// What: Privilege escalation tool used to run pacman as root.
///
/// Inputs:
/// - Selected explicitly by callers or via `install::detect_privilege_tool()`.
///
/// Output:
/// - Determines the prefix binary in privileged commands (e.g., `sudo pacman ...`).
///
/// Details:
/// - Automatic detection prefers `doas`, then falls back to `sudo`, matching Pacsea.
/// - Password handling (stdin piping, credential caching) is intentionally NOT
///   part of arch-toolkit; it is a UI/session concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegeTool {
    /// The `sudo` privilege tool (automatic fallback).
    Sudo,
    /// The `doas` privilege tool (preferred for automatic detection).
    Doas,
}

impl PrivilegeTool {
    /// What: Return the shell binary name for this tool.
    ///
    /// Inputs: None.
    ///
    /// Output: `"sudo"` or `"doas"`.
    ///
    /// Details: Used in command construction and `PATH` lookups.
    #[must_use]
    pub const fn binary_name(self) -> &'static str {
        match self {
            Self::Sudo => "sudo",
            Self::Doas => "doas",
        }
    }
}

impl std::fmt::Display for PrivilegeTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.binary_name())
    }
}

/// What: Removal cascade level controlling which pacman remove flags are used.
///
/// Inputs:
/// - Passed to `install::build_remove_command()`.
///
/// Output:
/// - Maps to the pacman flag sequence via `flag()`.
///
/// Details:
/// - `Basic`: `pacman -R` — remove targets only.
/// - `Cascade`: `pacman -Rs` — remove targets and now-orphaned dependencies.
/// - `CascadeWithConfigs`: `pacman -Rns` — cascade removal and prune configuration files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CascadeMode {
    /// `pacman -R` – remove targets only.
    Basic,
    /// `pacman -Rs` – remove targets and orphaned dependencies.
    Cascade,
    /// `pacman -Rns` – cascade removal and prune configuration files.
    CascadeWithConfigs,
}

impl CascadeMode {
    /// What: Return the pacman flag sequence corresponding to this cascade mode.
    ///
    /// Inputs: None.
    ///
    /// Output: `"-R"`, `"-Rs"`, or `"-Rns"`.
    ///
    /// Details: Used directly as the first pacman argument in remove commands.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Basic => "-R",
            Self::Cascade => "-Rs",
            Self::CascadeWithConfigs => "-Rns",
        }
    }

    /// What: Short text describing the effect of this cascade mode.
    ///
    /// Inputs: None.
    ///
    /// Output: Human-readable description string.
    ///
    /// Details: Suitable for confirmation prompts in calling applications.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Basic => "targets only",
            Self::Cascade => "remove dependents",
            Self::CascadeWithConfigs => "dependents + configs",
        }
    }
}

impl std::fmt::Display for CascadeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.description())
    }
}

/// What: A ready-to-run command as program plus argument vector.
///
/// Inputs:
/// - Produced by the `install` module command builders.
///
/// Output:
/// - Can be spawned directly (`to_command()`) or rendered for display (`to_shell_string()`).
///
/// Details:
/// - Argv-style representation avoids shell interpretation entirely when spawned
///   directly, eliminating quoting bugs.
/// - arch-toolkit never executes commands itself; callers decide whether to run,
///   display (dry run), or embed the command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    /// The program to execute (e.g., `pacman`, `paru`, `sudo`).
    pub program: String,
    /// Arguments passed to the program, unquoted (argv semantics).
    pub args: Vec<String>,
}

impl CommandSpec {
    /// What: Create a new command spec from a program and arguments.
    ///
    /// Inputs:
    /// - `program`: Program name or path.
    /// - `args`: Argument list (unquoted).
    ///
    /// Output: `CommandSpec` instance.
    ///
    /// Details: Convenience constructor accepting anything convertible to `String`.
    #[must_use]
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// What: Render this command as a properly quoted POSIX shell string.
    ///
    /// Inputs: None.
    ///
    /// Output: Shell-safe string like `sudo pacman -S --needed --noconfirm 'ripgrep'`.
    ///
    /// Details:
    /// - Arguments containing anything outside `[A-Za-z0-9@%^_+=:,./-]` are
    ///   single-quoted with embedded quotes escaped.
    /// - Intended for display (dry runs) and for passing to `bash -c` in
    ///   terminal-spawning callers.
    #[must_use]
    pub fn to_shell_string(&self) -> String {
        let mut out = shell_quote_word(&self.program);
        for arg in &self.args {
            out.push(' ');
            out.push_str(&shell_quote_word(arg));
        }
        out
    }

    /// What: Convert this spec into a `std::process::Command` ready to spawn.
    ///
    /// Inputs: None.
    ///
    /// Output: Configured `std::process::Command` (not yet spawned).
    ///
    /// Details:
    /// - No shell is involved; arguments are passed verbatim to the OS.
    /// - Callers remain responsible for spawning and privilege context.
    #[must_use]
    pub fn to_command(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(&self.program);
        cmd.args(&self.args);
        cmd
    }
}

impl std::fmt::Display for CommandSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_shell_string())
    }
}

/// What: Quote a single word for POSIX shells when necessary.
///
/// Inputs:
/// - `word`: The word to quote.
///
/// Output:
/// - The word unchanged if it contains only safe characters, otherwise single-quoted.
///
/// Details:
/// - Empty strings render as `''`.
/// - Embedded single quotes are escaped via the `'"'"'` sequence.
fn shell_quote_word(word: &str) -> String {
    let safe = !word.is_empty()
        && word.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'@' | b'%' | b'^' | b'_' | b'+' | b'=' | b':' | b',' | b'.' | b'/' | b'-'
                )
        });
    if safe {
        return word.to_string();
    }
    crate::install::shell_single_quote(word)
}

/// What: Options controlling install command flag construction.
///
/// Inputs:
/// - Passed to `build_pacman_install()`, `build_aur_install()`, and batch builders.
///
/// Output:
/// - Determines which flags (`--needed`, `--noconfirm`, `--aur`) appear in commands.
///
/// Details:
/// - `needed`: Pass `--needed` so up-to-date packages are skipped. Disable for
///   explicit reinstalls (mirrors Pacsea's reinstall path).
/// - `noconfirm`: Pass `--noconfirm` for non-interactive operation.
/// - `aur_only`: Pass `--aur` to AUR helpers so they do not prefer a sync
///   database (e.g., Chaotic-AUR) when the same name exists on the AUR.
///   Ignored by pacman builders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallOptions {
    /// Pass `--needed` (skip reinstalling up-to-date packages).
    pub needed: bool,
    /// Pass `--noconfirm` (non-interactive).
    pub noconfirm: bool,
    /// Pass `--aur` to AUR helpers (restrict resolution to the AUR).
    pub aur_only: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            needed: true,
            noconfirm: true,
            aur_only: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify binary names and flag mappings for the install enums.
    ///
    /// Inputs:
    /// - All enum variants.
    ///
    /// Output:
    /// - Expected binary names and pacman flags.
    ///
    /// Details:
    /// - Guards against accidental flag drift from Pacsea semantics.
    fn enum_mappings() {
        assert_eq!(AurHelper::Paru.binary_name(), "paru");
        assert_eq!(AurHelper::Yay.binary_name(), "yay");
        assert_eq!(PrivilegeTool::Sudo.binary_name(), "sudo");
        assert_eq!(PrivilegeTool::Doas.binary_name(), "doas");
        assert_eq!(CascadeMode::Basic.flag(), "-R");
        assert_eq!(CascadeMode::Cascade.flag(), "-Rs");
        assert_eq!(CascadeMode::CascadeWithConfigs.flag(), "-Rns");
        assert_eq!(CascadeMode::Cascade.description(), "remove dependents");
    }

    #[test]
    /// What: Verify `CommandSpec::to_shell_string` quotes only when necessary.
    ///
    /// Inputs:
    /// - Command with plain, empty, and quote-containing arguments.
    ///
    /// Output:
    /// - Safe words stay bare; unsafe words are single-quoted with escapes.
    ///
    /// Details:
    /// - Covers the `'"'"'` escape sequence for embedded single quotes.
    fn command_spec_shell_string() {
        let spec = CommandSpec::new("pacman", ["-S", "--needed", "ripgrep"]);
        assert_eq!(spec.to_shell_string(), "pacman -S --needed ripgrep");

        let tricky = CommandSpec::new("echo", ["it's", "", "a b"]);
        assert_eq!(tricky.to_shell_string(), r#"echo 'it'"'"'s' '' 'a b'"#);
    }

    #[test]
    /// What: Verify `CommandSpec::to_command` preserves program and args.
    ///
    /// Inputs:
    /// - Simple pacman spec.
    ///
    /// Output:
    /// - `std::process::Command` with matching program and argument list.
    ///
    /// Details:
    /// - Confirms argv passthrough without shell interpretation.
    fn command_spec_to_command() {
        let spec = CommandSpec::new("pacman", ["-Qq"]);
        let cmd = spec.to_command();
        assert_eq!(cmd.get_program(), "pacman");
        let args: Vec<_> = cmd.get_args().collect();
        assert_eq!(args, ["-Qq"]);
    }

    #[test]
    /// What: Verify `InstallOptions::default` matches Pacsea's non-interactive install path.
    ///
    /// Inputs:
    /// - `InstallOptions::default()`.
    ///
    /// Output:
    /// - needed=true, noconfirm=true, `aur_only=true`.
    ///
    /// Details:
    /// - The default corresponds to a fresh (non-reinstall) install.
    fn install_options_default() {
        let opts = InstallOptions::default();
        assert!(opts.needed);
        assert!(opts.noconfirm);
        assert!(opts.aur_only);
    }

    #[test]
    /// What: Verify serde roundtrips for install types.
    ///
    /// Inputs:
    /// - One value of each type serialized to JSON and back.
    ///
    /// Output:
    /// - Deserialized values equal the originals.
    ///
    /// Details:
    /// - Ensures the types can be persisted in caller configuration.
    fn serde_roundtrips() {
        let helper: AurHelper = serde_json::from_str(
            &serde_json::to_string(&AurHelper::Paru).expect("serialize helper"),
        )
        .expect("deserialize helper");
        assert_eq!(helper, AurHelper::Paru);
        let tool: PrivilegeTool = serde_json::from_str(
            &serde_json::to_string(&PrivilegeTool::Doas).expect("serialize tool"),
        )
        .expect("deserialize tool");
        assert_eq!(tool, PrivilegeTool::Doas);
        let mode: CascadeMode = serde_json::from_str(
            &serde_json::to_string(&CascadeMode::Cascade).expect("serialize mode"),
        )
        .expect("deserialize mode");
        assert_eq!(mode, CascadeMode::Cascade);
        let spec: CommandSpec = serde_json::from_str(
            &serde_json::to_string(&CommandSpec::new("x", ["y"])).expect("serialize spec"),
        )
        .expect("deserialize spec");
        assert_eq!(spec, CommandSpec::new("x", ["y"]));
    }
}
