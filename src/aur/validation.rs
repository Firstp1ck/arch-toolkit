//! Input validation for AUR operations.

use crate::error::{ArchToolkitError, Result};
use std::sync::LazyLock;

/// Default validation configuration (lazy static).
static DEFAULT_VALIDATION_CONFIG: LazyLock<ValidationConfig> =
    LazyLock::new(ValidationConfig::default);

/// What: Configuration for input validation behavior.
///
/// Inputs: None (created via `ValidationConfig::default()` or builder methods)
///
/// Output: `ValidationConfig` instance with validation settings
///
/// Details:
/// - Controls validation strictness for empty inputs
/// - Configures maximum length limits for inputs
/// - Can be customized via `ArchClientBuilder`
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Whether to return errors for empty inputs (strict) or empty results (lenient).
    pub strict_empty: bool,
    /// Maximum search query length in characters (default: 256).
    pub max_query_length: usize,
    /// Maximum package name length in characters (default: 127).
    pub max_package_name_length: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_empty: true,
            max_query_length: 256,
            max_package_name_length: 127,
        }
    }
}

/// What: Validate a package name according to Arch Linux packaging standards.
///
/// Inputs:
/// - `name`: Package name to validate
/// - `config`: Optional validation configuration (uses defaults if None)
///
/// Output:
/// - `Result<&str>` containing the validated name, or an error
///
/// Details:
/// - Validates against PKGBUILD naming rules:
///   - Allowed characters: lowercase letters (a-z), digits (0-9), `@`, `.`, `_`, `+`, `-`
///   - Cannot start with hyphen (`-`) or period (`.`)
///   - Must be non-empty
///   - Maximum length: 127 characters (default, configurable)
/// - Returns the input string on success for method chaining
///
/// # Errors
/// - Returns `Err(ArchToolkitError::EmptyInput)` if name is empty and strict mode is enabled
/// - Returns `Err(ArchToolkitError::InvalidPackageName)` if name contains invalid characters
/// - Returns `Err(ArchToolkitError::InputTooLong)` if name exceeds maximum length
pub fn validate_package_name<'a>(
    name: &'a str,
    config: Option<&ValidationConfig>,
) -> Result<&'a str> {
    let config = config.unwrap_or(&DEFAULT_VALIDATION_CONFIG);

    // Check for empty input
    if name.is_empty() {
        if config.strict_empty {
            return Err(ArchToolkitError::EmptyInput {
                field: "package name".to_string(),
                message: "package name cannot be empty".to_string(),
            });
        }
        return Ok(name); // Lenient mode: allow empty
    }

    // Check length
    if name.len() > config.max_package_name_length {
        return Err(ArchToolkitError::InputTooLong {
            field: "package name".to_string(),
            max_length: config.max_package_name_length,
            actual_length: name.len(),
        });
    }

    // Check for invalid starting characters
    if name.starts_with('-') {
        return Err(ArchToolkitError::InvalidPackageName {
            name: name.to_string(),
            reason: "package name cannot start with a hyphen (-)".to_string(),
        });
    }

    if name.starts_with('.') {
        return Err(ArchToolkitError::InvalidPackageName {
            name: name.to_string(),
            reason: "package name cannot start with a period (.)".to_string(),
        });
    }

    // Check for invalid characters
    // Allowed: lowercase letters (a-z), digits (0-9), @, ., _, +, -
    for (idx, ch) in name.char_indices() {
        let is_valid = matches!(ch,
            'a'..='z' | '0'..='9' | '@' | '.' | '_' | '+' | '-'
        );

        if !is_valid {
            return Err(ArchToolkitError::InvalidPackageName {
                name: name.to_string(),
                reason: format!(
                    "package name contains invalid character '{ch}' at position {idx} (allowed: lowercase letters, digits, @, ., _, +, -)"
                ),
            });
        }
    }

    Ok(name)
}

/// What: Validate multiple package names.
///
/// Inputs:
/// - `names`: Slice of package names to validate
/// - `config`: Optional validation configuration (uses defaults if None)
///
/// Output:
/// - `Result<()>` indicating success, or the first validation error encountered
///
/// Details:
/// - Validates each package name in the slice
/// - Returns the first error encountered, or `Ok(())` if all are valid
/// - In strict mode, empty slice returns an error; in lenient mode, it's allowed
///
/// # Errors
/// - Returns `Err(ArchToolkitError::EmptyInput)` if names slice is empty and strict mode is enabled
/// - Returns `Err(ArchToolkitError::InvalidPackageName)` for the first invalid package name
/// - Returns `Err(ArchToolkitError::InputTooLong)` for the first package name exceeding maximum length
pub fn validate_package_names(names: &[&str], config: Option<&ValidationConfig>) -> Result<()> {
    let config = config.unwrap_or(&DEFAULT_VALIDATION_CONFIG);

    // Check for empty slice
    if names.is_empty() {
        if config.strict_empty {
            return Err(ArchToolkitError::EmptyInput {
                field: "package names".to_string(),
                message: "at least one package name is required".to_string(),
            });
        }
        return Ok(()); // Lenient mode: allow empty
    }

    // Validate each package name
    for name in names {
        validate_package_name(name, Some(config))?;
    }

    Ok(())
}

/// What: Validate a search query string.
///
/// Inputs:
/// - `query`: Search query to validate
/// - `config`: Optional validation configuration (uses defaults if None)
///
/// Output:
/// - `Result<&str>` containing the trimmed query, or an error
///
/// Details:
/// - Trims whitespace from the query
/// - In strict mode, empty queries after trimming return an error
/// - In lenient mode, empty queries are allowed
/// - Checks maximum length (default: 256 characters)
/// - Any characters are allowed (will be percent-encoded)
///
/// # Errors
/// - Returns `Err(ArchToolkitError::EmptyInput)` if query is empty after trimming and strict mode is enabled
/// - Returns `Err(ArchToolkitError::InputTooLong)` if query exceeds maximum length
pub fn validate_search_query<'a>(
    query: &'a str,
    config: Option<&ValidationConfig>,
) -> Result<&'a str> {
    let config = config.unwrap_or(&DEFAULT_VALIDATION_CONFIG);
    let trimmed = query.trim();

    // Check for empty input
    if trimmed.is_empty() {
        if config.strict_empty {
            return Err(ArchToolkitError::EmptyInput {
                field: "search query".to_string(),
                message: "search query cannot be empty".to_string(),
            });
        }
        return Ok(trimmed); // Lenient mode: allow empty
    }

    // Check length
    if trimmed.len() > config.max_query_length {
        return Err(ArchToolkitError::InputTooLong {
            field: "search query".to_string(),
            max_length: config.max_query_length,
            actual_length: trimmed.len(),
        });
    }

    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_package_name_valid() {
        let valid_names = [
            "yay",
            "paru",
            "linux-zen",
            "lib32-mesa",
            "python-numpy",
            "gcc@12",
            "package_name",
            "pkg+plus",
            "123package",
        ];

        for name in &valid_names {
            assert!(
                validate_package_name(name, None).is_ok(),
                "Package name '{name}' should be valid"
            );
        }
    }

    #[test]
    fn test_validate_package_name_empty() {
        // Strict mode (default)
        let result = validate_package_name("", None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::EmptyInput { field, .. } => {
                assert_eq!(field, "package name");
            }
            _ => panic!("Expected EmptyInput error"),
        }

        // Lenient mode
        let config = ValidationConfig {
            strict_empty: false,
            ..Default::default()
        };
        assert!(validate_package_name("", Some(&config)).is_ok());
    }

    #[test]
    fn test_validate_package_name_starts_with_hyphen() {
        let result = validate_package_name("-invalid", None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InvalidPackageName { name, reason } => {
                assert_eq!(name, "-invalid");
                assert!(reason.contains("hyphen"));
            }
            _ => panic!("Expected InvalidPackageName error"),
        }
    }

    #[test]
    fn test_validate_package_name_starts_with_period() {
        let result = validate_package_name(".invalid", None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InvalidPackageName { name, reason } => {
                assert_eq!(name, ".invalid");
                assert!(reason.contains("period"));
            }
            _ => panic!("Expected InvalidPackageName error"),
        }
    }

    #[test]
    fn test_validate_package_name_uppercase() {
        let result = validate_package_name("Invalid", None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InvalidPackageName { name, reason } => {
                assert_eq!(name, "Invalid");
                assert!(reason.contains("invalid character"));
            }
            _ => panic!("Expected InvalidPackageName error"),
        }
    }

    #[test]
    fn test_validate_package_name_special_chars() {
        let invalid = ["package#name", "package name", "package!"];

        for name in &invalid {
            let result = validate_package_name(name, None);
            assert!(result.is_err(), "Package name '{name}' should be invalid");
            match result.expect_err("Expected validation error") {
                ArchToolkitError::InvalidPackageName { .. } => {}
                _ => panic!("Expected InvalidPackageName error for '{name}'"),
            }
        }
    }

    #[test]
    fn test_validate_package_name_too_long() {
        let long_name = "a".repeat(128); // Exceeds default max of 127
        let result = validate_package_name(&long_name, None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InputTooLong {
                field,
                max_length,
                actual_length,
            } => {
                assert_eq!(field, "package name");
                assert_eq!(max_length, 127);
                assert_eq!(actual_length, 128);
            }
            _ => panic!("Expected InputTooLong error"),
        }
    }

    #[test]
    fn test_validate_package_name_custom_max_length() {
        let config = ValidationConfig {
            max_package_name_length: 10,
            ..Default::default()
        };
        let name = "a".repeat(11);
        let result = validate_package_name(&name, Some(&config));
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InputTooLong { max_length, .. } => {
                assert_eq!(max_length, 10);
            }
            _ => panic!("Expected InputTooLong error"),
        }
    }

    #[test]
    fn test_validate_package_names_valid() {
        let names = &["yay", "paru", "linux-zen"];
        assert!(validate_package_names(names, None).is_ok());
    }

    #[test]
    fn test_validate_package_names_empty() {
        // Strict mode (default)
        let result = validate_package_names(&[], None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::EmptyInput { field, .. } => {
                assert_eq!(field, "package names");
            }
            _ => panic!("Expected EmptyInput error"),
        }

        // Lenient mode
        let config = ValidationConfig {
            strict_empty: false,
            ..Default::default()
        };
        assert!(validate_package_names(&[], Some(&config)).is_ok());
    }

    #[test]
    fn test_validate_package_names_invalid() {
        let names = &["yay", "-invalid", "paru"];
        let result = validate_package_names(names, None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InvalidPackageName { name, .. } => {
                assert_eq!(name, "-invalid");
            }
            _ => panic!("Expected InvalidPackageName error"),
        }
    }

    #[test]
    fn test_validate_search_query_valid() {
        let queries = ["yay", "paru helper", "linux", "  trimmed  "];

        for query in &queries {
            let result = validate_search_query(query, None);
            assert!(result.is_ok(), "Query '{query}' should be valid");
            // Should return trimmed version
            if let Ok(trimmed) = result {
                assert_eq!(trimmed, query.trim());
            }
        }
    }

    #[test]
    fn test_validate_search_query_empty() {
        // Strict mode (default)
        let result = validate_search_query("", None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::EmptyInput { field, .. } => {
                assert_eq!(field, "search query");
            }
            _ => panic!("Expected EmptyInput error"),
        }

        // Whitespace-only
        let result = validate_search_query("   ", None);
        assert!(result.is_err());

        // Lenient mode
        let config = ValidationConfig {
            strict_empty: false,
            ..Default::default()
        };
        assert!(validate_search_query("", Some(&config)).is_ok());
        assert!(validate_search_query("   ", Some(&config)).is_ok());
    }

    #[test]
    fn test_validate_search_query_too_long() {
        let long_query = "a".repeat(257); // Exceeds default max of 256
        let result = validate_search_query(&long_query, None);
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InputTooLong {
                field,
                max_length,
                actual_length,
            } => {
                assert_eq!(field, "search query");
                assert_eq!(max_length, 256);
                assert_eq!(actual_length, 257);
            }
            _ => panic!("Expected InputTooLong error"),
        }
    }

    #[test]
    fn test_validate_search_query_custom_max_length() {
        let config = ValidationConfig {
            max_query_length: 10,
            ..Default::default()
        };
        let query = "a".repeat(11);
        let result = validate_search_query(&query, Some(&config));
        assert!(result.is_err());
        match result.expect_err("Expected validation error") {
            ArchToolkitError::InputTooLong { max_length, .. } => {
                assert_eq!(max_length, 10);
            }
            _ => panic!("Expected InputTooLong error"),
        }
    }

    #[test]
    fn test_validation_config_default() {
        let config = ValidationConfig::default();
        assert!(config.strict_empty);
        assert_eq!(config.max_query_length, 256);
        assert_eq!(config.max_package_name_length, 127);
    }
}
