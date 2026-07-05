//! Feed date normalization utilities.

/// What: Normalize a feed date string to `YYYY-MM-DD`.
///
/// Inputs:
/// - `raw`: Date string in RFC 2822 (RSS), RFC 3339/ISO 8601 (Atom), or
///   already-normalized `YYYY-MM-DD` form.
///
/// Output:
/// - `YYYY-MM-DD` string on successful parse; otherwise the input with time
///   and timezone components stripped best-effort.
///
/// Details:
/// - Ported from Pacsea's `strip_time_and_tz()`.
/// - Normalized dates sort lexicographically, which the cutoff-date filtering
///   in the parsers relies on.
///
/// # Example
///
/// ```
/// use arch_toolkit::news::normalize_feed_date;
///
/// assert_eq!(normalize_feed_date("Thu, 21 Aug 2025 12:34:56 +0000"), "2025-08-21");
/// assert_eq!(normalize_feed_date("2025-12-07T14:00:00Z"), "2025-12-07");
/// assert_eq!(normalize_feed_date("2025-12-07"), "2025-12-07");
/// ```
#[must_use]
pub fn normalize_feed_date(raw: &str) -> String {
    let trimmed = raw.trim();

    // RFC 2822 (RSS pubDate: "Thu, 21 Aug 2025 12:34:56 +0000")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(trimmed) {
        return dt.format("%Y-%m-%d").to_string();
    }

    // RFC 3339 (Atom updated/published: "2025-12-07T14:00:00Z")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return dt.format("%Y-%m-%d").to_string();
    }

    // ISO 8601 without timezone
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return dt.format("%Y-%m-%d").to_string();
    }

    // Already date-only
    if chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").is_ok() {
        return trimmed.to_string();
    }

    // Partial RFC 2822 without timezone ("Thu, 21 Aug 2025" / "... 12:34:56")
    if let Some(date) = parse_partial_rfc2822(trimmed) {
        return date;
    }

    // Last resort: strip trailing "+ZZZZ" timezone and "HH:MM:SS" time manually.
    let mut t = trimmed.to_string();
    if let Some(pos) = t.rfind(" +") {
        t.truncate(pos);
        t = t.trim_end().to_string();
    }
    if t.len() >= 9 {
        let n = t.len();
        let time_part = &t[n - 8..n];
        let looks_time = time_part.chars().enumerate().all(|(i, c)| match i {
            2 | 5 => c == ':',
            _ => c.is_ascii_digit(),
        });
        if looks_time && t.as_bytes()[n - 9] == b' ' {
            t.truncate(n - 9);
        }
    }
    t.trim_end().to_string()
}

/// What: Parse an RFC 2822-like date lacking a timezone into `YYYY-MM-DD`.
///
/// Inputs:
/// - `s`: String like "Thu, 21 Aug 2025" or "Thu, 21 Aug 2025 12:34:56".
///
/// Output:
/// - `Some("YYYY-MM-DD")` on success, `None` otherwise.
///
/// Details:
/// - Handles feeds that omit the timezone, which `parse_from_rfc2822` rejects.
fn parse_partial_rfc2822(s: &str) -> Option<String> {
    let without_weekday = s.split_once(',').map_or(s, |(_, rest)| rest).trim();
    for fmt in ["%d %b %Y %H:%M:%S", "%d %b %Y"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(without_weekday, fmt) {
            return Some(dt.format("%Y-%m-%d").to_string());
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(without_weekday, fmt) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify all supported date formats normalize to `YYYY-MM-DD`.
    ///
    /// Inputs:
    /// - RFC 2822, RFC 3339, ISO 8601, date-only, and partial formats.
    ///
    /// Output:
    /// - Consistent `YYYY-MM-DD` output for each.
    ///
    /// Details:
    /// - Mirrors Pacsea's `strip_time_and_tz()` behavior.
    fn all_formats() {
        assert_eq!(
            normalize_feed_date("Thu, 21 Aug 2025 12:34:56 +0000"),
            "2025-08-21"
        );
        assert_eq!(normalize_feed_date("2025-12-07T14:00:00Z"), "2025-12-07");
        assert_eq!(normalize_feed_date("2025-12-07T14:00:00"), "2025-12-07");
        assert_eq!(normalize_feed_date("2025-12-07"), "2025-12-07");
        assert_eq!(normalize_feed_date("Thu, 21 Aug 2025"), "2025-08-21");
        assert_eq!(
            normalize_feed_date("Thu, 21 Aug 2025 12:34:56"),
            "2025-08-21"
        );
    }

    #[test]
    /// What: Verify unparseable input degrades gracefully.
    ///
    /// Inputs:
    /// - Arbitrary text with trailing time/timezone patterns.
    ///
    /// Output:
    /// - Time and timezone stripped; text otherwise preserved.
    ///
    /// Details:
    /// - Matches Pacsea's manual-strip fallback for resilience.
    fn fallback_stripping() {
        assert_eq!(normalize_feed_date("someday 12:34:56 +0100"), "someday");
        assert_eq!(normalize_feed_date("not a date"), "not a date");
        assert_eq!(normalize_feed_date(""), "");
    }
}
