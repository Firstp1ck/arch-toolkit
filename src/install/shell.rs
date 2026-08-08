//! Shell safety utilities: quoting and package name validation.

use crate::error::{ArchToolkitError, Result};

/// What: Safely single-quote an arbitrary string for POSIX shells.
///
/// Inputs:
/// - `s`: Text to quote.
///
/// Output:
/// - New string wrapped in single quotes, escaping embedded quotes via the
///   `'"'"'` sequence.
///
/// Details:
/// - Returns `''` for empty input so the shell treats it as an empty argument.
/// - Ported from Pacsea's `install/utils.rs`.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::shell_single_quote;
///
/// assert_eq!(shell_single_quote("plain"), "'plain'");
/// assert_eq!(shell_single_quote("it's"), r#"'it'"'"'s'"#);
/// assert_eq!(shell_single_quote(""), "''");
/// ```
#[must_use]
pub fn shell_single_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// What: Check whether a package name matches the strict allowlist used for install commands.
///
/// Inputs:
/// - `name`: Candidate package name to validate.
///
/// Output:
/// - `true` when `name` starts with a lowercase ASCII letter or digit and every
///   remaining byte is one of `a-z`, `0-9`, `@`, `.`, `_`, `+`, `-`.
///
/// Details:
/// - Defense-in-depth gate before command construction, matching Arch's
///   package naming rules (lowercase only).
/// - The first byte may not be `-` or `.`, so a name can never be parsed as an
///   option (`--help`, `-S`) or a hidden path, even before the `--` operand
///   terminator that all builders emit.
/// - Internal `@ . _ + -` remain valid, preserving `lib32-*`, split packages,
///   versioned names such as `python3.12`, and `+` names.
/// - Ported from Pacsea's `install/utils.rs`.
///
/// # Example
///
/// ```
/// use arch_toolkit::install::is_safe_package_name;
///
/// assert!(is_safe_package_name("ripgrep"));
/// assert!(is_safe_package_name("libc++"));
/// assert!(is_safe_package_name("lib32-glibc"));
/// assert!(!is_safe_package_name("bad;rm -rf"));
/// assert!(!is_safe_package_name("Upper"));
/// assert!(!is_safe_package_name("--help"));
/// assert!(!is_safe_package_name(".hidden"));
/// assert!(!is_safe_package_name(""));
/// ```
#[must_use]
pub fn is_safe_package_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    bytes.all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'@' | b'.' | b'_' | b'+' | b'-')
    })
}

/// What: Validate a list of package names against the strict install-command allowlist.
///
/// Inputs:
/// - `names`: Package names that will be placed into commands.
/// - `context`: Human-readable operation context for actionable error messages.
///
/// Output:
/// - `Ok(())` when all names are valid.
/// - `Err(ArchToolkitError::InvalidPackageName)` naming the first invalid package.
///
/// Details:
/// - Centralises validation so all install builders apply the same safety policy.
/// - The reported pattern states the leading-byte restriction that prevents
///   option confusion.
/// - Ported from Pacsea's `install/utils.rs`, adapted to `ArchToolkitError`.
///
/// # Errors
///
/// Returns `ArchToolkitError::InvalidPackageName` for the first name that fails
/// `is_safe_package_name()`.
pub fn validate_package_names<S: AsRef<str>>(names: &[S], context: &str) -> Result<()> {
    if let Some(invalid) = names
        .iter()
        .find(|name| !is_safe_package_name(name.as_ref()))
    {
        return Err(ArchToolkitError::InvalidPackageName {
            name: invalid.as_ref().to_string(),
            reason: format!(
                "invalid name for {context}; allowed pattern: ^[a-z0-9][a-z0-9@._+-]*$"
            ),
        });
    }
    Ok(())
}

/// What: Determine whether a command is available on the Unix `PATH`.
///
/// Inputs:
/// - `cmd`: Program basename or path containing a path separator.
///
/// Output:
/// - `true` when an executable file is found with the executable bit set.
///
/// Details:
/// - Honours Unix permission bits so a non-executable file on `PATH` is not
///   treated as a tool.
/// - Ported from Pacsea's `install/utils.rs`.
#[must_use]
pub fn command_on_path(cmd: &str) -> bool {
    resolve_command_on_path(cmd).is_some()
}

/// What: Resolve an executable on `PATH` or by explicit path.
///
/// Inputs:
/// - `cmd`: Program basename or path containing a path separator.
///
/// Output:
/// - `Some(path)` for the first executable match; otherwise `None`.
///
/// Details:
/// - When `cmd` contains a path separator, only that path is checked.
/// - Otherwise, each `PATH` directory is searched in order.
#[must_use]
pub fn resolve_command_on_path(cmd: &str) -> Option<std::path::PathBuf> {
    use std::path::Path;

    if cmd.contains(std::path::MAIN_SEPARATOR) {
        let p = Path::new(cmd);
        return path_is_executable(p).then(|| p.to_path_buf());
    }

    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(cmd);
        if path_is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// What: Check whether a path exists and is executable.
///
/// Inputs:
/// - `path`: Filesystem path to inspect.
///
/// Output:
/// - `true` when the path is a file with an executable permission bit (Unix).
///
/// Details:
/// - On non-Unix platforms, only file existence is checked.
fn path_is_executable(path: &std::path::Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify single-quoting handles plain, empty, and quote-embedded strings.
    ///
    /// Inputs:
    /// - Assorted strings including embedded single quotes.
    ///
    /// Output:
    /// - Correctly escaped shell-safe strings.
    ///
    /// Details:
    /// - The `'"'"'` sequence must appear for embedded quotes.
    fn quoting() {
        assert_eq!(shell_single_quote("abc"), "'abc'");
        assert_eq!(shell_single_quote(""), "''");
        assert_eq!(shell_single_quote("a'b"), r#"'a'"'"'b'"#);
    }

    #[test]
    /// What: Verify the package name allowlist accepts valid and rejects unsafe names.
    ///
    /// Inputs:
    /// - Valid Arch names and injection attempts.
    ///
    /// Output:
    /// - `true` only for names matching `^[a-z0-9@._+-]+$`.
    ///
    /// Details:
    /// - Uppercase, whitespace, shell metacharacters, and leading `-`/`.` must
    ///   be rejected; internal punctuation must stay valid.
    fn safe_names() {
        for good in [
            "ripgrep",
            "gcc12+libs",
            "lib32-glibc",
            "python3.12",
            "a@b_c",
            "0ad",
        ] {
            assert!(is_safe_package_name(good), "{good} should be valid");
        }
        for bad in [
            "",
            "Upper",
            "a b",
            "x;y",
            "$(rm)",
            "a`b`",
            "name'quote",
            "-S",
            "--help",
            "-",
            ".hidden",
            ".",
            "@scoped",
            "_leading",
            "+plus",
        ] {
            assert!(!is_safe_package_name(bad), "{bad} should be invalid");
        }
    }

    #[test]
    /// What: Verify batch validation reports the first invalid name with context.
    ///
    /// Inputs:
    /// - Name list containing one injection attempt.
    ///
    /// Output:
    /// - `InvalidPackageName` error naming the offending package and context.
    ///
    /// Details:
    /// - Valid lists must pass unchanged.
    fn validation() {
        assert!(validate_package_names(&["vim", "git"], "test").is_ok());
        let leading = validate_package_names(&["vim", "--help"], "test install");
        assert!(leading.is_err(), "leading option names must be rejected");
        let err = validate_package_names(&["vim", "bad;name"], "test install")
            .expect_err("should reject");
        match err {
            crate::error::ArchToolkitError::InvalidPackageName { name, reason } => {
                assert_eq!(name, "bad;name");
                assert!(reason.contains("test install"));
                assert!(reason.contains("^[a-z0-9][a-z0-9@._+-]*$"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    /// What: Verify PATH lookup finds a universally present binary and rejects nonsense.
    ///
    /// Inputs:
    /// - `sh` (present on all POSIX systems) and a random missing name.
    ///
    /// Output:
    /// - `true` for `sh`, `false` for the missing binary.
    ///
    /// Details:
    /// - Keeps the check portable across CI environments.
    fn path_lookup() {
        assert!(command_on_path("sh"));
        assert!(!command_on_path("definitely-not-a-real-binary-xyz"));
    }
}
