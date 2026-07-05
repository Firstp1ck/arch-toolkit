//! Detection of AUR helpers and privilege escalation tools on the system.

use crate::types::install::{AurHelper, PrivilegeTool};

use super::shell::command_on_path;

/// What: Detect the preferred AUR helper available on `PATH`.
///
/// Inputs: None.
///
/// Output:
/// - `Some(AurHelper::Paru)` when `paru` is available.
/// - `Some(AurHelper::Yay)` when only `yay` is available.
/// - `None` when neither helper is installed.
///
/// Details:
/// - Preference order (paru first, yay fallback) matches Pacsea's install flow.
/// - Detection is explicit: command builders never call this implicitly, so
///   callers can override the choice (e.g., from user configuration).
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::install::detect_aur_helper;
///
/// match detect_aur_helper() {
///     Some(helper) => println!("Using AUR helper: {helper}"),
///     None => eprintln!("No AUR helper (paru/yay) found."),
/// }
/// ```
#[must_use]
pub fn detect_aur_helper() -> Option<AurHelper> {
    if command_on_path(AurHelper::Paru.binary_name()) {
        Some(AurHelper::Paru)
    } else if command_on_path(AurHelper::Yay.binary_name()) {
        Some(AurHelper::Yay)
    } else {
        None
    }
}

/// What: Detect the preferred privilege escalation tool available on `PATH`.
///
/// Inputs: None.
///
/// Output:
/// - `Some(PrivilegeTool::Doas)` when `doas` is available.
/// - `Some(PrivilegeTool::Sudo)` when only `sudo` is available.
/// - `None` when neither tool is installed.
///
/// Details:
/// - Preference order (doas first, sudo fallback) matches Pacsea's `Auto`
///   privilege mode: sudo is present on most systems by default, so an
///   installed doas signals a deliberate user choice.
/// - Callers with an explicit user configuration should honor it via
///   [`is_privilege_tool_available`] instead of calling this.
/// - Password handling is intentionally out of scope for arch-toolkit.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::install::detect_privilege_tool;
///
/// match detect_privilege_tool() {
///     Some(tool) => println!("Privilege tool: {tool}"),
///     None => eprintln!("No privilege tool (sudo/doas) found."),
/// }
/// ```
#[must_use]
pub fn detect_privilege_tool() -> Option<PrivilegeTool> {
    if command_on_path(PrivilegeTool::Doas.binary_name()) {
        Some(PrivilegeTool::Doas)
    } else if command_on_path(PrivilegeTool::Sudo.binary_name()) {
        Some(PrivilegeTool::Sudo)
    } else {
        None
    }
}

/// What: Check whether a specific AUR helper is available on `PATH`.
///
/// Inputs:
/// - `helper`: The helper to check.
///
/// Output:
/// - `true` when the helper binary is executable on `PATH`.
///
/// Details:
/// - Useful when the caller has a configured preference and wants to verify it.
#[must_use]
pub fn is_aur_helper_available(helper: AurHelper) -> bool {
    command_on_path(helper.binary_name())
}

/// What: Check whether a specific privilege tool is available on `PATH`.
///
/// Inputs:
/// - `tool`: The tool to check.
///
/// Output:
/// - `true` when the tool binary is executable on `PATH`.
///
/// Details:
/// - Useful when the caller has a configured preference and wants to verify it.
#[must_use]
pub fn is_privilege_tool_available(tool: PrivilegeTool) -> bool {
    command_on_path(tool.binary_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify detection functions return consistent results with availability checks.
    ///
    /// Inputs:
    /// - Current system `PATH` (environment-dependent).
    ///
    /// Output:
    /// - When detection returns a helper/tool, its availability check agrees.
    ///
    /// Details:
    /// - Cannot assert specific tools exist (CI environments differ), so this
    ///   validates internal consistency and preference ordering instead.
    fn detection_consistency() {
        if let Some(helper) = detect_aur_helper() {
            assert!(is_aur_helper_available(helper));
            // Preference: if paru is available, it must be chosen over yay.
            if is_aur_helper_available(AurHelper::Paru) {
                assert_eq!(helper, AurHelper::Paru);
            }
        }
        if let Some(tool) = detect_privilege_tool() {
            assert!(is_privilege_tool_available(tool));
            // Preference: if doas is available, it must be chosen over sudo.
            if is_privilege_tool_available(PrivilegeTool::Doas) {
                assert_eq!(tool, PrivilegeTool::Doas);
            }
        }
    }
}
