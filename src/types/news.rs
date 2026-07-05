//! News-related data types for Arch Linux news and security advisories.

use serde::{Deserialize, Serialize};

/// What: A news item from the Arch Linux news RSS feed.
///
/// Inputs:
/// - Produced by `news::parse_arch_news_rss()` / `news::fetch_arch_news()`.
///
/// Output:
/// - Date, title, and URL of a news posting on archlinux.org.
///
/// Details:
/// - `date` is normalized to `YYYY-MM-DD` so items sort lexicographically.
/// - Serializable via Serde for caller-side caching.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchNewsItem {
    /// Publication date, normalized to `YYYY-MM-DD`.
    pub date: String,
    /// News headline.
    pub title: String,
    /// Link to the full article on archlinux.org.
    pub url: String,
}

/// What: Severity level of a security advisory.
///
/// Inputs:
/// - Parsed from advisory feed/title strings via `AdvisorySeverity::parse()`.
///
/// Output:
/// - Ordered severity classification, sortable via `rank()`.
///
/// Details:
/// - Ported from Pacsea's `AdvisorySeverity` with the same rank ordering
///   (Critical=5 > High=4 > Medium=3 > Low=2 > Unknown=1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvisorySeverity {
    /// Unknown or not provided.
    #[default]
    Unknown,
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
    /// Critical severity.
    Critical,
}

impl AdvisorySeverity {
    /// What: Return a numeric rank for sorting advisories by severity.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - `5` for Critical down to `1` for Unknown.
    ///
    /// Details:
    /// - Matches Pacsea's `severity_rank()` so sorting behavior is identical.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Critical => 5,
            Self::High => 4,
            Self::Medium => 3,
            Self::Low => 2,
            Self::Unknown => 1,
        }
    }

    /// What: Parse a severity string from a feed into a variant.
    ///
    /// Inputs:
    /// - `s`: Severity text (e.g., "Critical", "high", "MEDIUM").
    ///
    /// Output:
    /// - Matching variant; `Unknown` for unrecognized input.
    ///
    /// Details:
    /// - Case-insensitive; tolerates surrounding whitespace.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for AdvisorySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Unknown => "unknown",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        };
        f.write_str(s)
    }
}

/// What: A security advisory from security.archlinux.org.
///
/// Inputs:
/// - Produced by `news::parse_advisories_atom()` / `news::fetch_security_advisories()`.
///
/// Output:
/// - Advisory metadata: id, date, title, optional summary/URL, severity, packages.
///
/// Details:
/// - `id` falls back from URL → title → raw date, matching Pacsea's behavior.
/// - `severity` and `packages` are best-effort extracted from the advisory title
///   (format: `ASA-YYYYMM-N: package: issue type`); severity defaults to Unknown.
/// - `date` is normalized to `YYYY-MM-DD`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAdvisory {
    /// Stable identifier (advisory URL, or title/date fallback).
    pub id: String,
    /// Publication or update date, normalized to `YYYY-MM-DD`.
    pub date: String,
    /// Advisory headline.
    pub title: String,
    /// Optional summary text from the feed.
    pub summary: Option<String>,
    /// Optional link to the advisory page.
    pub url: Option<String>,
    /// Parsed severity (Unknown when the feed does not state one).
    pub severity: AdvisorySeverity,
    /// Affected package names extracted from the advisory title (best-effort).
    pub packages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// What: Verify severity ranks preserve Pacsea's sorting order.
    ///
    /// Inputs:
    /// - All severity variants.
    ///
    /// Output:
    /// - Critical > High > Medium > Low > Unknown.
    ///
    /// Details:
    /// - Rank values must match Pacsea's `severity_rank()` exactly.
    fn severity_ranks() {
        assert_eq!(AdvisorySeverity::Critical.rank(), 5);
        assert_eq!(AdvisorySeverity::High.rank(), 4);
        assert_eq!(AdvisorySeverity::Medium.rank(), 3);
        assert_eq!(AdvisorySeverity::Low.rank(), 2);
        assert_eq!(AdvisorySeverity::Unknown.rank(), 1);
    }

    #[test]
    /// What: Verify severity parsing is case-insensitive with Unknown fallback.
    ///
    /// Inputs:
    /// - Mixed-case severity strings and garbage input.
    ///
    /// Output:
    /// - Correct variants; Unknown for unrecognized text.
    ///
    /// Details:
    /// - Feed severity capitalization varies, so parsing must normalize.
    fn severity_parsing() {
        assert_eq!(
            AdvisorySeverity::parse("Critical"),
            AdvisorySeverity::Critical
        );
        assert_eq!(AdvisorySeverity::parse("HIGH"), AdvisorySeverity::High);
        assert_eq!(
            AdvisorySeverity::parse(" medium "),
            AdvisorySeverity::Medium
        );
        assert_eq!(AdvisorySeverity::parse("low"), AdvisorySeverity::Low);
        assert_eq!(AdvisorySeverity::parse("weird"), AdvisorySeverity::Unknown);
        assert_eq!(AdvisorySeverity::parse(""), AdvisorySeverity::Unknown);
    }

    #[test]
    /// What: Verify serde roundtrips for news types.
    ///
    /// Inputs:
    /// - Sample `ArchNewsItem` and `SecurityAdvisory` values.
    ///
    /// Output:
    /// - Deserialized values equal the originals.
    ///
    /// Details:
    /// - Ensures caller-side JSON caching works.
    fn serde_roundtrips() {
        let item = ArchNewsItem {
            date: "2026-07-01".to_string(),
            title: "Grub update".to_string(),
            url: "https://archlinux.org/news/grub-update/".to_string(),
        };
        let back: ArchNewsItem =
            serde_json::from_str(&serde_json::to_string(&item).expect("serialize item"))
                .expect("deserialize item");
        assert_eq!(back, item);

        let advisory = SecurityAdvisory {
            id: "https://security.archlinux.org/ASA-202607-1".to_string(),
            date: "2026-07-01".to_string(),
            title: "ASA-202607-1: openssl: multiple issues".to_string(),
            summary: Some("Multiple issues".to_string()),
            url: Some("https://security.archlinux.org/ASA-202607-1".to_string()),
            severity: AdvisorySeverity::High,
            packages: vec!["openssl".to_string()],
        };
        let back: SecurityAdvisory =
            serde_json::from_str(&serde_json::to_string(&advisory).expect("serialize advisory"))
                .expect("deserialize advisory");
        assert_eq!(back, advisory);
    }
}
