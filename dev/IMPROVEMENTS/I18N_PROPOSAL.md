# Proposal: Add i18n Support to arch-toolkit with English Fallback

## Overview

This proposal outlines how to add internationalization (i18n) support to arch-toolkit's pacman output parsing functions while maintaining backward compatibility and English fallback.

## Problem Statement

Currently, arch-toolkit hardcodes English labels for parsing pacman output:
- `"Depends On"` for dependency fields
- `"Conflicts With"` for conflict fields  
- `"None"` for empty fields

Pacsea uses a full i18n system that loads localized labels from YAML files, allowing it to parse pacman output in various languages (German, French, Spanish, etc.). When migrating to arch-toolkit, we lose this capability.

## Solution: Optional ParseConfig with English Fallback

Add an optional `ParseConfig` struct that allows users to provide custom labels, while defaulting to English if not provided. This approach:

1. **Maintains backward compatibility** - Existing code continues to work without changes
2. **No breaking changes** - All functions keep their current signatures with optional config
3. **English fallback** - Always falls back to English if custom labels aren't provided
4. **Zero dependencies** - Doesn't add any i18n dependencies to arch-toolkit
5. **Flexible** - Users can provide labels from any source (i18n system, hardcoded, etc.)

## Proposed API

### New Type: `ParseConfig`

```rust
/// What: Configuration for parsing pacman output with custom labels.
///
/// Inputs:
/// - `depends_labels`: Optional set of labels for "Depends On" field
/// - `conflicts_labels`: Optional set of labels for "Conflicts With" field
/// - `none_labels`: Optional set of labels for "None" (empty) field
///
/// Output:
/// - `ParseConfig` struct for customizing parsing behavior
///
/// Details:
/// - All fields are optional - if not provided, English defaults are used
/// - Labels are matched case-insensitively
/// - English labels are always included as fallback
pub struct ParseConfig {
    /// Custom labels for "Depends On" field (e.g., ["Ist abhängig von", "Dépend de"])
    pub depends_labels: Option<Vec<String>>,
    
    /// Custom labels for "Conflicts With" field
    pub conflicts_labels: Option<Vec<String>>,
    
    /// Custom labels for "None" (empty) field (e.g., ["Keine", "Aucune"])
    pub none_labels: Option<Vec<String>>,
}

impl ParseConfig {
    /// Create a new ParseConfig with all fields set to None (uses English defaults)
    pub fn new() -> Self {
        Self {
            depends_labels: None,
            conflicts_labels: None,
            none_labels: None,
        }
    }
    
    /// Builder method to set depends labels
    pub fn with_depends_labels(mut self, labels: Vec<String>) -> Self {
        self.depends_labels = Some(labels);
        self
    }
    
    /// Builder method to set conflicts labels
    pub fn with_conflicts_labels(mut self, labels: Vec<String>) -> Self {
        self.conflicts_labels = Some(labels);
        self
    }
    
    /// Builder method to set none labels
    pub fn with_none_labels(mut self, labels: Vec<String>) -> Self {
        self.none_labels = Some(labels);
        self
    }
    
    /// Get all depends labels (custom + English fallback)
    fn get_depends_labels(&self) -> Vec<String> {
        let mut labels = vec!["Depends On".to_string()]; // English fallback
        if let Some(custom) = &self.depends_labels {
            labels.extend_from_slice(custom);
        }
        labels
    }
    
    /// Get all conflicts labels (custom + English fallback)
    fn get_conflicts_labels(&self) -> Vec<String> {
        let mut labels = vec!["Conflicts With".to_string()]; // English fallback
        if let Some(custom) = &self.conflicts_labels {
            labels.extend_from_slice(custom);
        }
        labels
    }
    
    /// Get all none labels (custom + English fallback)
    fn get_none_labels(&self) -> Vec<String> {
        let mut labels = vec!["None".to_string()]; // English fallback
        if let Some(custom) = &self.none_labels {
            labels.extend_from_slice(custom);
        }
        labels
    }
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self::new()
    }
}
```

### Modified Functions

Update parsing functions to accept optional `ParseConfig`:

```rust
/// What: Extract dependency specifications from pacman -Si "Depends On" field.
///
/// Inputs:
/// - `text`: Raw stdout from `pacman -Si` for a package.
/// - `config`: Optional parsing configuration with custom labels.
///
/// Output:
/// - Returns vector of dependency specification strings.
///
/// Details:
/// - Uses custom labels from config if provided, otherwise uses English defaults
/// - Always includes English labels as fallback
pub fn parse_pacman_si_deps(text: &str, config: Option<&ParseConfig>) -> Vec<String> {
    let config = config.unwrap_or(&ParseConfig::default());
    let depends_labels = config.get_depends_labels();
    let none_labels = config.get_none_labels();
    
    // ... rest of implementation uses depends_labels and none_labels
}

/// What: Extract conflict specifications from pacman -Si "Conflicts With" field.
///
/// Inputs:
/// - `text`: Raw stdout from `pacman -Si` for a package.
/// - `config`: Optional parsing configuration with custom labels.
///
/// Output:
/// - Returns vector of package names that conflict.
pub fn parse_pacman_si_conflicts(text: &str, config: Option<&ParseConfig>) -> Vec<String> {
    let config = config.unwrap_or(&ParseConfig::default());
    let conflicts_labels = config.get_conflicts_labels();
    let none_labels = config.get_none_labels();
    
    // ... rest of implementation uses conflicts_labels and none_labels
}
```

### Backward Compatibility

To maintain backward compatibility, add convenience functions without the config parameter:

```rust
/// Convenience function that uses default English labels
pub fn parse_pacman_si_deps(text: &str) -> Vec<String> {
    parse_pacman_si_deps_with_config(text, None)
}

/// Full function with optional config
pub fn parse_pacman_si_deps_with_config(text: &str, config: Option<&ParseConfig>) -> Vec<String> {
    // ... implementation
}
```

Or use default parameter pattern (Rust doesn't support this directly, so we'd use `Option`):

```rust
// Keep existing function signature, add config as optional parameter
pub fn parse_pacman_si_deps(text: &str) -> Vec<String> {
    parse_pacman_si_deps_with_config(text, None)
}

// New function with explicit config
pub fn parse_pacman_si_deps_with_config(text: &str, config: Option<&ParseConfig>) -> Vec<String> {
    // ... implementation
}
```

## Implementation Details

### Changes to `src/deps/parse.rs`

1. **Add `ParseConfig` struct** at the top of the file
2. **Replace hardcoded constants** with config-based lookups:
   - `DEPENDS_LABELS` → `config.get_depends_labels()`
   - `CONFLICTS_LABELS` → `config.get_conflicts_labels()`
   - `NONE_LABELS` → `config.get_none_labels()`
3. **Update parsing functions** to accept optional `ParseConfig`
4. **Keep English as fallback** - Always include English labels even when custom labels are provided

### Example Usage in Pacsea

```rust
use arch_toolkit::deps::{ParseConfig, parse_pacman_si_deps_with_config};

// Load i18n labels from Pacsea's i18n system
let depends_labels = get_depends_labels(); // From Pacsea's i18n
let none_labels = get_none_labels(); // From Pacsea's i18n

// Create config with i18n labels
let config = ParseConfig::new()
    .with_depends_labels(depends_labels)
    .with_none_labels(none_labels);

// Use config when parsing
let deps = parse_pacman_si_deps_with_config(&pacman_output, Some(&config));
```

### Example: Loading from Pacsea's i18n System

Pacsea's i18n system stores arrays as YAML strings in the `TranslationMap`. Here's how to extract labels and use them with arch-toolkit:

```rust
use arch_toolkit::deps::{ParseConfig, parse_pacman_si_deps_with_config};
use std::collections::HashSet;
use std::sync::OnceLock;

/// What: Get all possible localized labels for "Depends On" field from Pacsea's i18n system.
///
/// Output:
/// - `Vec<String>` of all possible labels across all locales
///
/// Details:
/// - Loads labels from all locale files in Pacsea's locales directory
/// - Extracts from `app.parsing.pacman_depends_labels` (YAML array stored as string)
/// - Falls back to English if loading fails
/// - Cached on first access for performance
fn get_depends_labels_from_pacsea_i18n() -> Vec<String> {
    static LABELS: OnceLock<Vec<String>> = OnceLock::new();
    LABELS.get_or_init(|| {
        let mut labels = HashSet::new();

        // Try to load from all locale files (existing Pacsea pattern)
        let locales_dir = crate::i18n::find_locales_dir().unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("config")
                .join("locales")
        });
        
        if let Ok(entries) = std::fs::read_dir(&locales_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str()
                    && file_name.to_lowercase().ends_with(".yml")
                {
                    let locale = file_name.strip_suffix(".yml").unwrap_or(file_name);
                    if let Ok(translations) = crate::i18n::load_locale_file(locale, &locales_dir) {
                        // Extract labels from app.parsing.pacman_depends_labels
                        // Note: TranslationMap stores arrays as YAML strings, so we need to parse them
                        if let Some(labels_str) = translations.get("app.parsing.pacman_depends_labels") {
                            // Parse YAML string back to Value
                            if let Ok(yaml_value) = serde_norway::from_str::<serde_norway::Value>(labels_str) {
                                if let Some(seq) = yaml_value.as_sequence() {
                                    // Extract each label from the array
                                    for item in seq {
                                        if let Some(label) = item.as_str() {
                                            labels.insert(label.to_string());
                                        }
                                    }
                                }
                            }
                        } else if let Some(label) = translations.get("app.parsing.pacman_depends_label") {
                            // Fallback to single label format
                            labels.insert(label.clone());
                        }
                    }
                }
            }
        }

        // Always include English fallback
        labels.insert("Depends On".to_string());
        
        labels.into_iter().collect()
    }).clone()
}

/// What: Get all possible localized "None" labels from Pacsea's i18n system.
fn get_none_labels_from_pacsea_i18n() -> Vec<String> {
    static LABELS: OnceLock<Vec<String>> = OnceLock::new();
    LABELS.get_or_init(|| {
        let mut labels = HashSet::new();

        let locales_dir = crate::i18n::find_locales_dir().unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("config")
                .join("locales")
        });
        
        if let Ok(entries) = std::fs::read_dir(&locales_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str()
                    && file_name.to_lowercase().ends_with(".yml")
                {
                    let locale = file_name.strip_suffix(".yml").unwrap_or(file_name);
                    if let Ok(translations) = crate::i18n::load_locale_file(locale, &locales_dir) {
                        if let Some(labels_str) = translations.get("app.parsing.pacman_none_labels") {
                            if let Ok(yaml_value) = serde_norway::from_str::<serde_norway::Value>(labels_str) {
                                if let Some(seq) = yaml_value.as_sequence() {
                                    for item in seq {
                                        if let Some(label) = item.as_str() {
                                            labels.insert(label.to_string());
                                        }
                                    }
                                }
                            }
                        } else if let Some(label) = translations.get("app.parsing.pacman_none_label") {
                            labels.insert(label.clone());
                        }
                    }
                }
            }
        }

        // Always include English fallback
        labels.insert("None".to_string());
        
        labels.into_iter().collect()
    }).clone()
}

/// What: Get all possible localized "Conflicts With" labels from Pacsea's i18n system.
fn get_conflicts_labels_from_pacsea_i18n() -> Vec<String> {
    // Similar implementation to get_depends_labels_from_pacsea_i18n()
    // Extract from app.parsing.pacman_conflicts_labels or app.parsing.pacman_conflicts_label
    // ...
}

// Use in dependency resolution
let config = ParseConfig::new()
    .with_depends_labels(get_depends_labels_from_pacsea_i18n())
    .with_none_labels(get_none_labels_from_pacsea_i18n())
    .with_conflicts_labels(get_conflicts_labels_from_pacsea_i18n());

let deps = parse_pacman_si_deps_with_config(&pacman_output, Some(&config));
```

**Note:** This matches Pacsea's existing pattern in `src/logic/deps/parse.rs`. The key points are:
1. Pacsea's `TranslationMap` stores arrays as YAML strings (see `loader.rs:144-149`)
2. We need to parse the YAML string back to extract the array items
3. The labels are collected from ALL locale files (not just the current locale)
4. English labels are always included as fallback

## Compatibility with Pacsea's i18n System

✅ **Yes, this proposal works with Pacsea's current i18n implementation!**

### How Pacsea's i18n Works

1. **Locale Files**: Pacsea stores translations in `config/locales/*.yml` files
2. **TranslationMap**: Loaded translations are stored as `HashMap<String, String>` with dot-notation keys
3. **Array Storage**: Arrays (like `pacman_depends_labels`) are stored as YAML strings in the TranslationMap
4. **Label Extraction**: Pacsea's `get_depends_labels()` function:
   - Iterates through ALL locale files
   - Extracts `app.parsing.pacman_depends_labels` (YAML array as string)
   - Parses the YAML string back to extract individual labels
   - Collects labels from all locales into a HashSet
   - Falls back to hardcoded English labels if loading fails

### Integration Points

The proposal integrates seamlessly because:

1. **Same Pattern**: The example code follows Pacsea's existing pattern in `src/logic/deps/parse.rs`
2. **Same Data Source**: Uses the same `TranslationMap` and locale loading functions
3. **Same Label Keys**: Uses the same translation keys (`app.parsing.pacman_depends_labels`, etc.)
4. **Same Fallback**: Includes English labels as fallback, just like Pacsea does
5. **No Changes Needed**: Pacsea's locale files don't need any changes - they already have the labels

### Example: Direct Integration

Pacsea can reuse its existing `get_depends_labels()` function (or create a wrapper):

```rust
// Option 1: Reuse existing function (if made public or moved to shared module)
let depends_labels: Vec<String> = crate::logic::deps::parse::get_depends_labels()
    .iter()
    .cloned()
    .collect();

// Option 2: Create wrapper that extracts from TranslationMap
let config = ParseConfig::new()
    .with_depends_labels(extract_labels_from_translations("app.parsing.pacman_depends_labels"))
    .with_none_labels(extract_labels_from_translations("app.parsing.pacman_none_labels"))
    .with_conflicts_labels(extract_labels_from_translations("app.parsing.pacman_conflicts_labels"));
```

## Benefits

1. **Backward Compatible**: Existing code using arch-toolkit continues to work
2. **No Breaking Changes**: All current function signatures remain valid
3. **English Fallback**: Always works even if custom labels fail to load
4. **Zero Dependencies**: Doesn't add i18n dependencies to arch-toolkit
5. **Flexible**: Users can provide labels from any source
6. **Simple API**: Optional config parameter, defaults to English
7. **✅ Works with Pacsea's i18n**: Compatible with existing locale files and TranslationMap structure

## Migration Path

1. **Phase 1**: Add `ParseConfig` struct and new functions to arch-toolkit
2. **Phase 2**: Update arch-toolkit's internal usage to use config (optional)
3. **Phase 3**: Pacsea can start using config with i18n labels
4. **Phase 4**: Eventually deprecate old functions (optional, not required)

## Testing

Add tests for:
1. Default behavior (English labels) - should match current behavior
2. Custom labels (German, French, etc.)
3. Mixed labels (custom + English fallback)
4. Empty config (should use English)
5. Invalid labels (should still work with English fallback)

## Alternative Approaches Considered

### Option 1: Environment Variable Configuration
- **Pros**: Simple, no code changes needed
- **Cons**: Less flexible, harder to use programmatically

### Option 2: Global Static Configuration
- **Pros**: Simple API
- **Cons**: Not thread-safe, harder to test, global state issues

### Option 3: Trait-Based Approach
- **Pros**: Very flexible
- **Cons**: More complex, overkill for this use case

### Option 4: Feature Flag for i18n
- **Pros**: Could add full i18n support
- **Cons**: Adds dependencies, more complex, not needed if we use optional config

## Recommendation

The **optional `ParseConfig` approach** is the best solution because:
- It's simple and straightforward
- Maintains backward compatibility
- Provides flexibility without complexity
- No additional dependencies
- English fallback ensures it always works

## Next Steps

1. Review and approve this proposal
2. Implement `ParseConfig` in arch-toolkit
3. Update parsing functions to use config
4. Add tests for i18n support
5. Update documentation
6. Release new version of arch-toolkit
7. Update Pacsea migration plan to use new API

