//! Parser for PKGBUILD files.
//!
//! This module provides functions for parsing PKGBUILD files, which are
//! bash scripts that define how Arch Linux packages are built.
//!
//! The parser extracts dependency arrays (depends, makedepends, checkdepends, optdepends)
//! and conflicts from PKGBUILD content, handling both single-line and multi-line
//! bash array syntax.

use std::collections::HashSet;

use crate::deps::parse::parse_dep_spec;

/// What: Parse dependencies from PKGBUILD content.
///
/// Inputs:
/// - `pkgbuild`: Raw PKGBUILD file content.
///
/// Output:
/// - Returns a tuple of (depends, makedepends, checkdepends, optdepends) vectors.
///
/// Details:
/// - Parses bash array syntax: `depends=('foo' 'bar>=1.2')` (single-line)
/// - Also handles `depends+=` patterns used in functions like `package()`
/// - Handles both quoted and unquoted dependencies
/// - Also handles multi-line arrays:
///   ```text
///   depends=(
///       'foo'
///       'bar>=1.2'
///   )
///   ```
/// - Filters out .so files (virtual packages) and invalid package names
/// - Only parses specific dependency fields (depends, makedepends, checkdepends, optdepends)
/// - Deduplicates dependencies (returns unique list)
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[must_use]
pub fn parse_pkgbuild_deps(pkgbuild: &str) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut depends = Vec::new();
    let mut makedepends = Vec::new();
    let mut checkdepends = Vec::new();
    let mut optdepends = Vec::new();

    // Use HashSet for deduplication
    let mut seen_depends = HashSet::new();
    let mut seen_makedepends = HashSet::new();
    let mut seen_checkdepends = HashSet::new();
    let mut seen_optdepends = HashSet::new();

    let lines: Vec<&str> = pkgbuild.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        i += 1;

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse array declarations: depends=('foo' 'bar') or depends=( or depends+=('foo' 'bar')
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Handle both depends= and depends+= patterns
            let base_key = key.strip_suffix('+').map_or(key, |stripped| stripped);

            // Only parse specific dependency fields, ignore other PKGBUILD fields
            if !matches!(
                base_key,
                "depends" | "makedepends" | "checkdepends" | "optdepends"
            ) {
                continue;
            }

            // Check if this is an array declaration
            if value.starts_with('(') {
                let deps = find_matching_closing_paren(value).map_or_else(
                    || {
                        // Multi-line array: depends=(
                        //     'foo'
                        //     'bar'
                        // )
                        let mut array_lines = Vec::new();
                        // Collect lines until we find the closing parenthesis
                        while i < lines.len() {
                            let next_line = lines[i].trim();
                            i += 1;

                            // Skip empty lines and comments
                            if next_line.is_empty() || next_line.starts_with('#') {
                                continue;
                            }

                            // Check if this line closes the array
                            if next_line == ")" {
                                break;
                            }

                            // Check if this line contains a closing parenthesis (may be on same line as content)
                            if let Some(paren_pos) = next_line.find(')') {
                                // Extract content before the closing paren
                                let content_before_paren = &next_line[..paren_pos].trim();
                                if !content_before_paren.is_empty() {
                                    array_lines.push((*content_before_paren).to_string());
                                }
                                break;
                            }

                            // Add this line to the array content
                            array_lines.push(next_line.to_string());
                        }

                        // Parse all collected lines as array content
                        // Ensure proper spacing between items (each line should be a separate item)
                        let array_content = array_lines
                            .iter()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ");
                        parse_array_content(&array_content)
                    },
                    |closing_paren_pos| {
                        // Single-line array (may have content after closing paren): depends=('foo' 'bar') or depends+=('foo' 'bar') other_code
                        let array_content = &value[1..closing_paren_pos];
                        parse_array_content(array_content)
                    },
                );

                // Filter out invalid dependencies (.so files, invalid names, etc.)
                let filtered_deps: Vec<String> = deps
                    .into_iter()
                    .filter_map(|dep| {
                        let dep_trimmed = dep.trim();
                        if dep_trimmed.is_empty() {
                            return None;
                        }

                        if is_valid_dependency(dep_trimmed) {
                            Some(dep_trimmed.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Add dependencies to the appropriate vector (using base_key to handle both = and +=)
                // Deduplicate using HashSet
                match base_key {
                    "depends" => {
                        for dep in filtered_deps {
                            if seen_depends.insert(dep.clone()) {
                                depends.push(dep);
                            }
                        }
                    }
                    "makedepends" => {
                        for dep in filtered_deps {
                            if seen_makedepends.insert(dep.clone()) {
                                makedepends.push(dep);
                            }
                        }
                    }
                    "checkdepends" => {
                        for dep in filtered_deps {
                            if seen_checkdepends.insert(dep.clone()) {
                                checkdepends.push(dep);
                            }
                        }
                    }
                    "optdepends" => {
                        for dep in filtered_deps {
                            if seen_optdepends.insert(dep.clone()) {
                                optdepends.push(dep);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (depends, makedepends, checkdepends, optdepends)
}

/// What: Parse conflicts from PKGBUILD content.
///
/// Inputs:
/// - `pkgbuild`: Raw PKGBUILD file content.
///
/// Output:
/// - Returns a vector of conflicting package names (without version constraints).
///
/// Details:
/// - Parses bash array syntax: `conflicts=('foo' 'bar')` (single-line)
/// - Also handles `conflicts+=` patterns used in functions like `package()`
/// - Handles both quoted and unquoted conflicts
/// - Also handles multi-line arrays:
///   ```text
///   conflicts=(
///       'foo'
///       'bar'
///   )
///   ```
/// - Filters out .so files (virtual packages) and invalid package names
/// - Extracts package names from version constraints (e.g., "jujutsu-git>=1.0" -> "jujutsu-git")
/// - Deduplicates conflicts (returns unique list)
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[must_use]
pub fn parse_pkgbuild_conflicts(pkgbuild: &str) -> Vec<String> {
    let mut conflicts = Vec::new();
    let mut seen = HashSet::new();

    let lines: Vec<&str> = pkgbuild.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        i += 1;

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse array declarations: conflicts=('foo' 'bar') or conflicts=( or conflicts+=('foo' 'bar')
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Handle both conflicts= and conflicts+= patterns
            let base_key = key.strip_suffix('+').map_or(key, |stripped| stripped);

            // Only parse conflicts field
            if base_key != "conflicts" {
                continue;
            }

            // Check if this is an array declaration
            if value.starts_with('(') {
                let conflict_deps = find_matching_closing_paren(value).map_or_else(
                    || {
                        // Multi-line array: conflicts=(
                        //     'foo'
                        //     'bar'
                        // )
                        let mut array_lines = Vec::new();
                        // Collect lines until we find the closing parenthesis
                        while i < lines.len() {
                            let next_line = lines[i].trim();
                            i += 1;

                            // Skip empty lines and comments
                            if next_line.is_empty() || next_line.starts_with('#') {
                                continue;
                            }

                            // Check if this line closes the array
                            if next_line == ")" {
                                break;
                            }

                            // Check if this line contains a closing parenthesis (may be on same line as content)
                            if let Some(paren_pos) = next_line.find(')') {
                                // Extract content before the closing paren
                                let content_before_paren = &next_line[..paren_pos].trim();
                                if !content_before_paren.is_empty() {
                                    array_lines.push((*content_before_paren).to_string());
                                }
                                break;
                            }

                            // Add this line to the array content
                            array_lines.push(next_line.to_string());
                        }

                        // Parse all collected lines as array content
                        let array_content = array_lines
                            .iter()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ");
                        parse_array_content(&array_content)
                    },
                    |closing_paren_pos| {
                        // Single-line array (may have content after closing paren): conflicts=('foo' 'bar') or conflicts+=('foo' 'bar') other_code
                        let array_content = &value[1..closing_paren_pos];
                        parse_array_content(array_content)
                    },
                );

                // Filter out invalid conflicts (.so files, invalid names, etc.)
                let filtered_conflicts: Vec<String> = conflict_deps
                    .into_iter()
                    .filter_map(|conflict| {
                        let conflict_trimmed = conflict.trim();
                        if conflict_trimmed.is_empty() {
                            return None;
                        }

                        if is_valid_dependency(conflict_trimmed) {
                            // Extract package name (remove version constraints if present)
                            // Use a simple approach: split on version operators
                            let spec = parse_dep_spec(conflict_trimmed);
                            if !spec.name.is_empty() && seen.insert(spec.name.clone()) {
                                Some(spec.name)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Add conflicts to the vector (using base_key to handle both = and +=)
                conflicts.extend(filtered_conflicts);
            }
        }
    }

    conflicts
}

/// What: Find the position of the matching closing parenthesis in a string.
///
/// Inputs:
/// - `s`: String starting with an opening parenthesis.
///
/// Output:
/// - `Some(position)` if a matching closing parenthesis is found, `None` otherwise.
///
/// Details:
/// - Handles nested parentheses and quoted strings.
fn find_matching_closing_paren(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_quotes = false;
    let mut quote_char = '\0';

    for (pos, ch) in s.char_indices() {
        match ch {
            '\'' | '"' => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quotes = false;
                    quote_char = '\0';
                }
            }
            '(' if !in_quotes => {
                depth += 1;
            }
            ')' if !in_quotes => {
                depth -= 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
            _ => {}
        }
    }
    None
}

/// What: Parse quoted and unquoted strings from bash array content.
///
/// Inputs:
/// - `content`: Array content string (e.g., "'foo' 'bar>=1.2'" or "libcairo.so libdbus-1.so").
///
/// Output:
/// - Vector of dependency strings.
///
/// Details:
/// - Handles both quoted ('foo') and unquoted (foo) dependencies.
/// - Splits on whitespace for unquoted values.
fn parse_array_content(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut current = String::new();

    for ch in content.chars() {
        match ch {
            '\'' | '"' => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    if !current.is_empty() {
                        deps.push(current.clone());
                        current.clear();
                    }
                    in_quotes = false;
                    quote_char = '\0';
                } else {
                    current.push(ch);
                }
            }
            _ if in_quotes => {
                current.push(ch);
            }
            ch if ch.is_whitespace() => {
                // Whitespace outside quotes - end current unquoted value
                if !current.is_empty() {
                    deps.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                // Non-whitespace character outside quotes - add to current value
                current.push(ch);
            }
        }
    }

    // Handle unclosed quote or trailing unquoted value
    if !current.is_empty() {
        deps.push(current);
    }

    deps
}

/// What: Check if a dependency string is valid (not a .so file, has valid format).
///
/// Inputs:
/// - `dep`: Dependency string to validate.
///
/// Output:
/// - Returns `true` if the dependency appears to be valid, `false` otherwise.
///
/// Details:
/// - Filters out .so files (virtual packages)
/// - Filters out names ending with ) (parsing errors)
/// - Filters out names that don't start with alphanumeric or underscore
/// - Filters out names that are too short (< 2 characters)
/// - Requires at least one alphanumeric character
fn is_valid_dependency(dep: &str) -> bool {
    // Filter out .so files (virtual packages)
    let dep_lower = dep.to_lowercase();
    if std::path::Path::new(&dep_lower)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("so"))
        || dep_lower.contains(".so.")
        || dep_lower.contains(".so=")
    {
        return false;
    }

    // Filter out names ending with ) - this is a parsing error
    // But first check if it's actually a valid name with version constraint ending in )
    // like "package>=1.0)" which would be a parsing error
    if dep.ends_with(')') {
        // Check if it might be a valid version constraint that accidentally ends with )
        // If it contains version operators before the ), it's likely a parsing error
        if dep.contains(">=") || dep.contains("<=") || dep.contains("==") {
            // This looks like "package>=1.0)" which is invalid
            return false;
        }
        // Otherwise, it might be "package)" which is also invalid
        return false;
    }

    // Filter out names that don't look like package names
    // Package names should start with alphanumeric or underscore
    let Some(first_char) = dep.chars().next() else {
        return false;
    };
    if !first_char.is_alphanumeric() && first_char != '_' {
        return false;
    }

    // Filter out names that are too short
    if dep.len() < 2 {
        return false;
    }

    // Filter out names containing invalid characters (but allow version operators)
    // Allow: alphanumeric, dash, underscore, and version operators (>=, <=, ==, >, <)
    let has_valid_chars = dep
        .chars()
        .any(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if !has_valid_chars {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // === parse_pkgbuild_deps tests ===

    #[test]
    fn test_parse_pkgbuild_deps_basic() {
        let pkgbuild = r"
pkgname=test-package
pkgver=1.0.0
depends=('foo' 'bar>=1.2')
makedepends=('make' 'gcc')
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar>=1.2".to_string()));

        assert_eq!(makedepends.len(), 2);
        assert!(makedepends.contains(&"make".to_string()));
        assert!(makedepends.contains(&"gcc".to_string()));

        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_append() {
        let pkgbuild = r#"
pkgname=test-package
pkgver=1.0.0
package() {
    depends+=(foo bar)
    cd $_pkgname
    make DESTDIR="$pkgdir" PREFIX=/usr install
}
"#;

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar".to_string()));

        assert_eq!(makedepends.len(), 0);
        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_unquoted() {
        let pkgbuild = r"
pkgname=test-package
depends=(foo bar libcairo.so libdbus-1.so)
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        // .so files should be filtered out
        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar".to_string()));

        assert_eq!(makedepends.len(), 0);
        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_multiline() {
        let pkgbuild = r"
pkgname=test-package
depends=(
    'foo'
    'bar>=1.2'
    'baz'
)
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        assert_eq!(depends.len(), 3);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar>=1.2".to_string()));
        assert!(depends.contains(&"baz".to_string()));

        assert_eq!(makedepends.len(), 0);
        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_makedepends_append() {
        let pkgbuild = r"
pkgname=test-package
build() {
    makedepends+=(cmake ninja)
    cmake -B build
}
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        assert_eq!(makedepends.len(), 2);
        assert!(makedepends.contains(&"cmake".to_string()));
        assert!(makedepends.contains(&"ninja".to_string()));

        assert_eq!(depends.len(), 0);
        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_jujutsu_git_scenario() {
        let pkgbuild = r"
pkgname=jujutsu-git
pkgver=0.1.0
pkgdesc=Git-compatible VCS that is both simple and powerful
url=https://github.com/martinvonz/jj
license=(Apache-2.0)
arch=(i686 x86_64 armv6h armv7h)
depends=(
    glibc
    libc.so
    libm.so
)
makedepends=(
    libgit2
    libgit2.so
    libssh2
    libssh2.so)
    openssh
    git)
cargo
checkdepends=()
optdepends=()
source=($pkgname::git+$url)
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        // depends should only contain glibc, .so files filtered out
        assert_eq!(depends.len(), 1);
        assert!(depends.contains(&"glibc".to_string()));

        // makedepends should contain libgit2, libssh2
        // .so files are filtered out
        // Note: openssh, git), and cargo are after the array closes, so they're not part of makedepends
        assert_eq!(makedepends.len(), 2);
        assert!(makedepends.contains(&"libgit2".to_string()));
        assert!(makedepends.contains(&"libssh2".to_string()));

        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_ignore_other_fields() {
        let pkgbuild = r"
pkgname=test-package
pkgver=1.0.0
pkgdesc=Test package description
url=https://example.com
license=(MIT)
arch=(x86_64)
source=($pkgname-$pkgver.tar.gz)
depends=(foo bar)
makedepends=(make)
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        // Only depends and makedepends should be parsed
        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar".to_string()));

        assert_eq!(makedepends.len(), 1);
        assert!(makedepends.contains(&"make".to_string()));

        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_filter_invalid_names() {
        // Test filtering of invalid names (using single-line format for reliability)
        let pkgbuild = r"
depends=('valid-package' 'invalid)' '=invalid' 'a' 'valid>=1.0')
";

        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps(pkgbuild);

        // Only valid package names should remain
        // Note: 'invalid)' should be filtered out (ends with ))
        // Note: '=invalid' should be filtered out (starts with =)
        // Note: 'a' should be filtered out (too short)
        // So we should have: valid-package and valid>=1.0
        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"valid-package".to_string()));
        assert!(depends.contains(&"valid>=1.0".to_string()));

        assert_eq!(makedepends.len(), 0);
        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_deduplicates() {
        let pkgbuild = r"
depends=('foo' 'bar' 'foo' 'baz' 'bar')
";

        let (depends, _, _, _) = parse_pkgbuild_deps(pkgbuild);
        assert_eq!(depends.len(), 3, "Should deduplicate dependencies");
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar".to_string()));
        assert!(depends.contains(&"baz".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_deps_empty() {
        let (depends, makedepends, checkdepends, optdepends) = parse_pkgbuild_deps("");
        assert_eq!(depends.len(), 0);
        assert_eq!(makedepends.len(), 0);
        assert_eq!(checkdepends.len(), 0);
        assert_eq!(optdepends.len(), 0);
    }

    #[test]
    fn test_parse_pkgbuild_deps_comments_and_blank_lines() {
        let pkgbuild = r"
# This is a comment
pkgname=test-package

depends=(foo bar)
# Another comment
makedepends=(make)
";

        let (depends, makedepends, _, _) = parse_pkgbuild_deps(pkgbuild);
        assert_eq!(depends.len(), 2);
        assert!(depends.contains(&"foo".to_string()));
        assert!(depends.contains(&"bar".to_string()));
        assert_eq!(makedepends.len(), 1);
        assert!(makedepends.contains(&"make".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_deps_mixed_quoted_unquoted() {
        let pkgbuild = r"
depends=('quoted' unquoted 'another-quoted' unquoted2)
";

        let (depends, _, _, _) = parse_pkgbuild_deps(pkgbuild);
        assert_eq!(depends.len(), 4);
        assert!(depends.contains(&"quoted".to_string()));
        assert!(depends.contains(&"unquoted".to_string()));
        assert!(depends.contains(&"another-quoted".to_string()));
        assert!(depends.contains(&"unquoted2".to_string()));
    }

    // === parse_pkgbuild_conflicts tests ===

    #[test]
    fn test_parse_pkgbuild_conflicts_basic() {
        let pkgbuild = r"
pkgname=jujutsu-git
pkgver=0.1.0
conflicts=('jujutsu')
";

        let conflicts = parse_pkgbuild_conflicts(pkgbuild);

        assert_eq!(conflicts.len(), 1);
        assert!(conflicts.contains(&"jujutsu".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_conflicts_multiline() {
        let pkgbuild = r"
pkgname=pacsea-git
pkgver=0.1.0
conflicts=(
    'pacsea'
    'pacsea-bin'
)
";

        let conflicts = parse_pkgbuild_conflicts(pkgbuild);

        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.contains(&"pacsea".to_string()));
        assert!(conflicts.contains(&"pacsea-bin".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_conflicts_with_versions() {
        let pkgbuild = r"
pkgname=test-package
conflicts=('old-pkg<2.0' 'new-pkg>=3.0')
";

        let conflicts = parse_pkgbuild_conflicts(pkgbuild);

        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.contains(&"old-pkg".to_string()));
        assert!(conflicts.contains(&"new-pkg".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_conflicts_filter_so() {
        let pkgbuild = r"
pkgname=test-package
conflicts=('foo' 'libcairo.so' 'bar' 'libdbus-1.so=1-64')
";

        let conflicts = parse_pkgbuild_conflicts(pkgbuild);

        // .so files should be filtered out
        assert_eq!(conflicts.len(), 2);
        assert!(conflicts.contains(&"foo".to_string()));
        assert!(conflicts.contains(&"bar".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_conflicts_deduplicates() {
        let pkgbuild = r"
conflicts=('pkg1' 'pkg2' 'pkg1' 'pkg3')
";

        let conflicts = parse_pkgbuild_conflicts(pkgbuild);
        assert_eq!(conflicts.len(), 3, "Should deduplicate conflicts");
        assert!(conflicts.contains(&"pkg1".to_string()));
        assert!(conflicts.contains(&"pkg2".to_string()));
        assert!(conflicts.contains(&"pkg3".to_string()));
    }

    #[test]
    fn test_parse_pkgbuild_conflicts_empty() {
        let conflicts = parse_pkgbuild_conflicts("");
        assert!(conflicts.is_empty());
    }
}
