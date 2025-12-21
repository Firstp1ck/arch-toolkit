//! Utility functions for AUR operations.

use serde_json::Value;
use std::fmt::Write;

/// What: Percent-encode a string for use in URLs according to RFC 3986.
///
/// Inputs:
/// - `input`: String to encode.
///
/// Output:
/// - Returns a percent-encoded string where reserved characters are escaped.
///
/// Details:
/// - Unreserved characters as per RFC 3986 (`A-Z`, `a-z`, `0-9`, `-`, `.`, `_`, `~`) are left as-is.
/// - Space is encoded as `%20` (not `+`).
/// - All other bytes are encoded as two uppercase hexadecimal digits prefixed by `%`.
/// - Operates on raw bytes from the input string; any non-ASCII bytes are hex-escaped.
#[must_use]
pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push_str("%20"),
            _ => {
                out.push('%');
                let _ = write!(out, "{b:02X}");
            }
        }
    }
    out
}

/// What: Extract a string value from a JSON object by key, defaulting to empty string.
///
/// Inputs:
/// - `v`: JSON value to extract from.
/// - `key`: Key to look up in the JSON object.
///
/// Output:
/// - Returns the string value if found, or an empty string if the key is missing or not a string.
///
/// Details:
/// - Returns `""` if the key is missing or the value is not a string type.
#[must_use]
pub fn s(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// What: Extract a string value from a JSON object by key with fallback keys.
///
/// Inputs:
/// - `v`: JSON value to extract from.
/// - `keys`: Array of keys to try in order.
///
/// Output:
/// - Returns the first string value found, or an empty string if none found.
///
/// Details:
/// - Tries each key in order until one returns a string value.
#[must_use]
#[allow(dead_code)] // Utility function that may be used in future modules
pub fn ss(v: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = v.get(key).and_then(Value::as_str) {
            return s.to_string();
        }
    }
    String::new()
}

/// What: Extract an array of strings from a JSON object by trying keys in order.
///
/// Inputs:
/// - `v`: JSON value to extract from.
/// - `keys`: Array of candidate keys to try in order.
///
/// Output:
/// - Returns the first found array as `Vec<String>`, filtering out non-string elements.
/// - Returns an empty vector if no array of strings is found.
///
/// Details:
/// - Tries keys in the order provided and returns the first array found.
/// - Filters out non-string elements from the array.
#[must_use]
pub fn arrs(v: &Value, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(arr) = v.get(key).and_then(Value::as_array) {
            return arr
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect();
        }
    }
    Vec::new()
}

/// What: Extract an unsigned 64-bit integer by trying multiple keys and representations.
///
/// Inputs:
/// - `v`: JSON value to extract from.
/// - `keys`: Array of candidate keys to try in order.
///
/// Output:
/// - Returns `Some(u64)` if a valid value is found, or `None` if no usable value is found.
///
/// Details:
/// - Accepts any of the following representations for the first matching key:
///   - JSON `u64`
///   - JSON `i64` convertible to `u64` (must be positive)
///   - String that parses as `u64`
/// - Tries keys in the order provided and returns the first match.
#[must_use]
pub fn u64_of(v: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(n) = v.get(key) {
            if let Some(u) = n.as_u64() {
                return Some(u);
            }
            if let Some(i) = n.as_i64()
                && let Ok(u) = u64::try_from(i)
                && u > 0
            {
                return Some(u);
            }
            if let Some(s) = n.as_str()
                && let Ok(p) = s.parse::<u64>()
            {
                return Some(p);
            }
        }
    }
    None
}
