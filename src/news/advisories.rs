//! Security advisory Atom feed fetching and parsing (security.archlinux.org).

use crate::error::{ArchToolkitError, Result};
use crate::types::news::{AdvisorySeverity, SecurityAdvisory};

use super::arch::{extract_between, unescape_xml};
use super::date::normalize_feed_date;

/// URL of the official Arch Linux security advisory Atom feed.
pub const ADVISORY_FEED_URL: &str = "https://security.archlinux.org/advisory/feed.atom";

/// What: Parse security advisory Atom content into advisories.
///
/// Inputs:
/// - `body`: Raw Atom feed XML.
/// - `limit`: Maximum number of advisories to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date; parsing stops at the first
///   entry older than this (feeds are newest-first).
///
/// Output:
/// - Parsed advisories with normalized dates, newest first.
///
/// Details:
/// - Iteratively scans `<entry>` blocks extracting `<title>`, link `href`,
///   `<updated>`/`<published>`, and `<summary>`, ported from Pacsea's
///   `fetch_security_advisories()`.
/// - Package names and severity are extracted from the advisory title
///   best-effort (Pacsea left both empty/Unknown); titles look like
///   `[ASA-202607-1] openssl: multiple issues` or `ASA-202607-1: openssl: ...`.
/// - Pure function: unit-testable without network access.
///
/// # Example
///
/// ```
/// use arch_toolkit::news::parse_advisories_atom;
///
/// let atom = r#"<entry><title>[ASA-202607-1] openssl: multiple issues</title>
/// <link href="https://security.archlinux.org/ASA-202607-1"/>
/// <updated>2026-07-01T12:00:00Z</updated></entry>"#;
/// let advisories = parse_advisories_atom(atom, 10, None);
/// assert_eq!(advisories.len(), 1);
/// assert_eq!(advisories[0].packages, vec!["openssl".to_string()]);
/// ```
#[must_use]
pub fn parse_advisories_atom(
    body: &str,
    limit: usize,
    cutoff_date: Option<&str>,
) -> Vec<SecurityAdvisory> {
    let mut items: Vec<SecurityAdvisory> = Vec::new();
    let mut pos = 0;
    while items.len() < limit {
        let Some(start) = body[pos..].find("<entry>") else {
            break;
        };
        let s = pos + start;
        let end = body[s..].find("</entry>").map_or(body.len(), |e| s + e + 8);
        let chunk = &body[s..end];

        let title = extract_between(chunk, "<title>", "</title>")
            .map(|t| unescape_xml(&t))
            .unwrap_or_default();
        let link = extract_link_href(chunk).unwrap_or_default();
        let raw_date = extract_between(chunk, "<updated>", "</updated>")
            .or_else(|| extract_between(chunk, "<published>", "</published>"))
            .unwrap_or_default();
        let date = normalize_feed_date(&raw_date);
        // Early date filtering: stop when entries become older than the cutoff
        if let Some(cutoff) = cutoff_date
            && date.as_str() < cutoff
        {
            break;
        }
        let summary = extract_between(chunk, "<summary>", "</summary>")
            .map(|t| unescape_xml(&t))
            .filter(|t| !t.is_empty());
        let entry_id = extract_between(chunk, "<id>", "</id>")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let id = entry_id.clone().unwrap_or_else(|| {
            if link.is_empty() {
                if title.is_empty() {
                    raw_date.clone()
                } else {
                    title.clone()
                }
            } else {
                link.clone()
            }
        });
        let url = if link.is_empty() {
            entry_id
        } else {
            Some(link)
        };

        // The feed's <content> block carries structured "Severity:" and
        // "Package :" fields; fall back to title heuristics when absent.
        let (content_severity, content_packages) = extract_content(chunk).map_or_else(
            || (AdvisorySeverity::Unknown, Vec::new()),
            |c| parse_content_fields(&c),
        );
        let severity = if content_severity == AdvisorySeverity::Unknown {
            extract_severity(&title, summary.as_deref())
        } else {
            content_severity
        };
        let packages = if content_packages.is_empty() {
            extract_packages(&title)
        } else {
            content_packages
        };

        items.push(SecurityAdvisory {
            id,
            date,
            severity,
            packages,
            title: if title.is_empty() {
                "Advisory".to_string()
            } else {
                title
            },
            summary,
            url,
        });
        pos = end;
    }
    items
}

/// What: Fetch recent security advisories from security.archlinux.org.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client (controls timeouts, user agent).
/// - `limit`: Maximum number of advisories to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date for early filtering.
///
/// Output:
/// - `Ok(Vec<SecurityAdvisory>)` with normalized dates, newest first.
///
/// Details:
/// - Fetches the official advisory Atom feed and delegates to
///   [`parse_advisories_atom`].
/// - No caching or rate limiting; callers decide their fetch cadence.
///
/// # Errors
///
/// Returns `ArchToolkitError::Parse` when the request fails or the server
/// returns a non-success status.
///
/// # Example
///
/// ```no_run
/// use arch_toolkit::news::fetch_security_advisories;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = reqwest::Client::new();
/// let advisories = fetch_security_advisories(&client, 10, None).await?;
/// for advisory in advisories {
///     println!("{} [{}] {}", advisory.date, advisory.severity, advisory.title);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn fetch_security_advisories(
    client: &reqwest::Client,
    limit: usize,
    cutoff_date: Option<&str>,
) -> Result<Vec<SecurityAdvisory>> {
    let response = client
        .get(ADVISORY_FEED_URL)
        .send()
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("advisory feed request failed: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("advisory feed body read failed: {e}")))?;
    tracing::debug!(
        status = status.as_u16(),
        bytes = body.len(),
        "fetched advisories feed"
    );
    if !status.is_success() {
        return Err(ArchToolkitError::Parse(format!(
            "advisory feed returned status {status}"
        )));
    }
    Ok(parse_advisories_atom(&body, limit, cutoff_date))
}

/// What: Extract the href attribute of the first `<link>` tag in an entry.
///
/// Inputs:
/// - `s`: Atom entry XML chunk.
///
/// Output:
/// - `Some(href)` when found, `None` otherwise.
///
/// Details:
/// - Ported from Pacsea's `extract_link_href()`.
fn extract_link_href(s: &str) -> Option<String> {
    let link_pos = s.find("<link")?;
    let rest = &s[link_pos..];
    let href_pos = rest.find("href=\"")?;
    let after = &rest[href_pos + 6..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

/// What: Extract and unescape the `<content>` block from an Atom entry.
///
/// Inputs:
/// - `chunk`: Atom entry XML chunk.
///
/// Output:
/// - `Some(text)` with entities decoded when a content block exists.
///
/// Details:
/// - The tag carries attributes (`<content type="html">`), so the opening
///   tag is scanned to its closing `>` before extracting the body.
fn extract_content(chunk: &str) -> Option<String> {
    let start_tag = chunk.find("<content")?;
    let rest = &chunk[start_tag..];
    let open_end = rest.find('>')? + 1;
    let end = rest.find("</content>")?;
    if open_end >= end {
        return None;
    }
    Some(unescape_xml(&rest[open_end..end]))
}

/// What: Parse severity and package fields from advisory content text.
///
/// Inputs:
/// - `content`: Unescaped advisory content with `<br/>`-separated lines
///   (format: `Severity: High`, `Package : nodejs-lts-jod`).
///
/// Output:
/// - Parsed severity (Unknown when absent) and package names.
///
/// Details:
/// - The security.archlinux.org feed embeds the full advisory text in
///   `<content>`; its key-value header is the authoritative severity source.
fn parse_content_fields(content: &str) -> (AdvisorySeverity, Vec<String>) {
    let mut severity = AdvisorySeverity::Unknown;
    let mut packages: Vec<String> = Vec::new();
    for line in content
        .split("<br/>")
        .flat_map(|part| part.split("<br>"))
        .flat_map(str::lines)
    {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "severity" => severity = AdvisorySeverity::parse(value),
            "package" | "packages" => {
                packages = value
                    .split_whitespace()
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
            _ => {}
        }
        if severity != AdvisorySeverity::Unknown && !packages.is_empty() {
            break;
        }
    }
    (severity, packages)
}

/// What: Extract affected package names from an advisory title.
///
/// Inputs:
/// - `title`: Advisory title like `[ASA-202607-1] openssl: multiple issues`
///   or `ASA-202607-1: chromium: arbitrary code execution`.
///
/// Output:
/// - Package names (comma-separated lists are split), empty when the title
///   does not match the expected shape.
///
/// Details:
/// - Best-effort improvement over Pacsea, which always returned an empty list.
fn extract_packages(title: &str) -> Vec<String> {
    // Strip a leading "[ASA-...]" or "ASA-...:" identifier
    let rest = title.strip_prefix('[').map_or_else(
        || {
            if title.starts_with("ASA-") || title.starts_with("AVG-") {
                title.split_once(':').map_or("", |(_, rest)| rest)
            } else {
                title
            }
        },
        |after| after.split_once(']').map_or("", |(_, rest)| rest),
    );
    // The package segment is everything before the next ':'
    let Some((pkg_part, _issue)) = rest.split_once(':') else {
        return Vec::new();
    };
    pkg_part
        .split(',')
        .map(str::trim)
        .filter(|p| {
            !p.is_empty()
                && p.bytes().all(|b| {
                    b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(b, b'@' | b'.' | b'_' | b'+' | b'-')
                })
        })
        .map(ToString::to_string)
        .collect()
}

/// What: Extract a severity classification from advisory title or summary.
///
/// Inputs:
/// - `title`: Advisory title.
/// - `summary`: Optional advisory summary.
///
/// Output:
/// - Parsed severity; `Unknown` when neither text states one.
///
/// Details:
/// - Looks for "(critical)"-style markers and "severity: high"-style phrases.
/// - The Atom feed usually omits severity, so `Unknown` is the common case
///   (matching Pacsea's behavior).
fn extract_severity(title: &str, summary: Option<&str>) -> AdvisorySeverity {
    for text in [Some(title), summary].into_iter().flatten() {
        let lower = text.to_ascii_lowercase();
        for candidate in ["critical", "high", "medium", "low"] {
            if lower.contains(&format!("({candidate})"))
                || lower.contains(&format!("severity: {candidate}"))
            {
                return AdvisorySeverity::parse(candidate);
            }
        }
    }
    AdvisorySeverity::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ATOM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
<entry>
  <title>[ASA-202607-1] openssl: multiple issues</title>
  <link href="https://security.archlinux.org/ASA-202607-1"/>
  <updated>2026-07-01T12:00:00Z</updated>
  <summary>Multiple issues have been found (critical)</summary>
</entry>
<entry>
  <title>ASA-202606-9: chromium,electron32: arbitrary code execution</title>
  <link href="https://security.archlinux.org/ASA-202606-9"/>
  <published>2026-06-20T08:30:00Z</published>
</entry>
<entry>
  <title>Old advisory</title>
  <link href="https://security.archlinux.org/ASA-202501-1"/>
  <updated>2026-01-01T00:00:00Z</updated>
</entry>
</feed>"#;

    #[test]
    /// What: Verify Atom entries parse with dates, links, and summaries.
    ///
    /// Inputs:
    /// - Sample feed with `updated` and `published` date variants.
    ///
    /// Output:
    /// - Three advisories with normalized dates and correct URLs.
    ///
    /// Details:
    /// - `published` must be used when `updated` is absent.
    fn parses_entries() {
        let advisories = parse_advisories_atom(SAMPLE_ATOM, 10, None);
        assert_eq!(advisories.len(), 3);
        assert_eq!(advisories[0].date, "2026-07-01");
        assert_eq!(
            advisories[0].url.as_deref(),
            Some("https://security.archlinux.org/ASA-202607-1")
        );
        assert_eq!(
            advisories[0].summary.as_deref(),
            Some("Multiple issues have been found (critical)")
        );
        assert_eq!(advisories[1].date, "2026-06-20");
        assert!(advisories[2].summary.is_none());
    }

    #[test]
    /// What: Verify package extraction from bracketed and colon title forms.
    ///
    /// Inputs:
    /// - Sample feed titles in `[ASA-...] pkg:` and `ASA-...: pkg,pkg2:` forms.
    ///
    /// Output:
    /// - Single and comma-separated package lists extracted.
    ///
    /// Details:
    /// - Improvement over Pacsea (which returned empty lists).
    fn extracts_packages() {
        let advisories = parse_advisories_atom(SAMPLE_ATOM, 10, None);
        assert_eq!(advisories[0].packages, ["openssl"]);
        assert_eq!(advisories[1].packages, ["chromium", "electron32"]);
        assert!(advisories[2].packages.is_empty());
    }

    #[test]
    /// What: Verify severity extraction from summary markers.
    ///
    /// Inputs:
    /// - Entry with "(critical)" in the summary; entries without markers.
    ///
    /// Output:
    /// - Critical for the first, Unknown for the rest.
    ///
    /// Details:
    /// - The feed usually omits severity, so Unknown is the default.
    fn extracts_severity() {
        let advisories = parse_advisories_atom(SAMPLE_ATOM, 10, None);
        assert_eq!(advisories[0].severity, AdvisorySeverity::Critical);
        assert_eq!(advisories[1].severity, AdvisorySeverity::Unknown);
    }

    #[test]
    /// What: Verify limit and cutoff-date filtering.
    ///
    /// Inputs:
    /// - Sample feed with limit 1 and a cutoff between entries.
    ///
    /// Output:
    /// - Truncated result sets respecting both bounds.
    ///
    /// Details:
    /// - Cutoff stops at the first entry older than the given date.
    fn respects_limit_and_cutoff() {
        assert_eq!(parse_advisories_atom(SAMPLE_ATOM, 1, None).len(), 1);
        let filtered = parse_advisories_atom(SAMPLE_ATOM, 10, Some("2026-06-01"));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    /// What: Verify identifier fallback (URL → title → raw date).
    ///
    /// Inputs:
    /// - Entries with and without links.
    ///
    /// Output:
    /// - URL used as id when present; title used otherwise.
    ///
    /// Details:
    /// - Matches Pacsea's id fallback chain.
    fn id_fallback() {
        let advisories = parse_advisories_atom(SAMPLE_ATOM, 10, None);
        assert_eq!(
            advisories[0].id,
            "https://security.archlinux.org/ASA-202607-1"
        );

        let no_link =
            "<entry><title>Some advisory</title><updated>2026-07-01T00:00:00Z</updated></entry>";
        let items = parse_advisories_atom(no_link, 10, None);
        assert_eq!(items[0].id, "Some advisory");
    }

    #[test]
    /// What: Verify severity and packages parse from the live feed's content block.
    ///
    /// Inputs:
    /// - Entry shaped like the real security.archlinux.org feed: entity-encoded
    ///   HTML content with `Severity:` and `Package :` fields, plus an `<id>`.
    ///
    /// Output:
    /// - Severity High, package from the content header, id from `<id>`.
    ///
    /// Details:
    /// - The content header is authoritative and overrides title heuristics.
    fn parses_live_feed_content_block() {
        let atom = r#"<entry>
    <id>https://security.archlinux.org/ASA-202505-7</id>
    <title>[ASA-202505-7] nodejs-lts-jod: denial of service</title>
    <updated>2025-05-18T23:32:35.759771+00:00</updated>
    <content type="html">&lt;pre&gt;Arch Linux Security Advisory ASA-202505-7&lt;br/&gt;Severity: High&lt;br/&gt;Date    : 2025-05-18&lt;br/&gt;Package : nodejs-lts-jod&lt;br/&gt;Type    : denial of service&lt;/pre&gt;</content>
  </entry>"#;
        let advisories = parse_advisories_atom(atom, 10, None);
        assert_eq!(advisories.len(), 1);
        let advisory = &advisories[0];
        assert_eq!(advisory.severity, AdvisorySeverity::High);
        assert_eq!(advisory.packages, ["nodejs-lts-jod"]);
        assert_eq!(advisory.id, "https://security.archlinux.org/ASA-202505-7");
        assert_eq!(
            advisory.url.as_deref(),
            Some("https://security.archlinux.org/ASA-202505-7")
        );
        assert_eq!(advisory.date, "2025-05-18");
    }

    #[test]
    /// What: Verify empty and malformed feeds yield no advisories.
    ///
    /// Inputs:
    /// - Empty string and non-Atom text.
    ///
    /// Output:
    /// - Empty vectors, no panic.
    ///
    /// Details:
    /// - Parser must degrade gracefully on unexpected content.
    fn handles_garbage() {
        assert!(parse_advisories_atom("", 10, None).is_empty());
        assert!(parse_advisories_atom("no entries here", 10, None).is_empty());
    }
}
