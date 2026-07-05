//! Arch Linux news RSS feed fetching and parsing.

use crate::error::{ArchToolkitError, Result};
use crate::types::news::ArchNewsItem;

use super::date::normalize_feed_date;

/// URL of the official Arch Linux news RSS feed.
pub const ARCH_NEWS_FEED_URL: &str = "https://archlinux.org/feeds/news/";

/// What: Extract the substring between two markers.
///
/// Inputs:
/// - `s`: Haystack to search.
/// - `start`: Opening marker.
/// - `end`: Closing marker (searched after `start`).
///
/// Output:
/// - `Some(inner)` when both markers are found in order, `None` otherwise.
///
/// Details:
/// - Ported from Pacsea's `news/utils.rs`; used for lightweight feed parsing
///   without a full XML parser dependency.
pub fn extract_between(s: &str, start: &str, end: &str) -> Option<String> {
    let i = s.find(start)? + start.len();
    let j = s[i..].find(end)? + i;
    Some(s[i..j].to_string())
}

/// What: Decode the five predefined XML entities in feed text.
///
/// Inputs:
/// - `s`: Raw text extracted from a feed element.
///
/// Output:
/// - Text with `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` (and `&#39;`) decoded.
///
/// Details:
/// - Improvement over Pacsea, which returned entity-encoded titles verbatim.
/// - Also unwraps `<![CDATA[...]]>` sections.
pub fn unescape_xml(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix("<![CDATA[")
        .and_then(|rest| rest.strip_suffix("]]>"))
        .unwrap_or(s);
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// What: Parse Arch Linux news RSS content into news items.
///
/// Inputs:
/// - `body`: Raw RSS feed XML.
/// - `limit`: Maximum number of items to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date; parsing stops at the first
///   item older than this (feeds are newest-first).
///
/// Output:
/// - Parsed items with normalized dates, newest first.
///
/// Details:
/// - Iteratively scans `<item>` blocks extracting `<title>`, `<link>`, and
///   `<pubDate>`, ported from Pacsea's `fetch_arch_news()`.
/// - Pure function: unit-testable without network access.
///
/// # Example
///
/// ```
/// use arch_toolkit::news::parse_arch_news_rss;
///
/// let rss = r#"<item><title>Grub changes</title>
/// <link>https://archlinux.org/news/grub/</link>
/// <pubDate>Thu, 21 Aug 2025 12:34:56 +0000</pubDate></item>"#;
/// let items = parse_arch_news_rss(rss, 10, None);
/// assert_eq!(items.len(), 1);
/// assert_eq!(items[0].date, "2025-08-21");
/// assert_eq!(items[0].title, "Grub changes");
/// ```
#[must_use]
pub fn parse_arch_news_rss(
    body: &str,
    limit: usize,
    cutoff_date: Option<&str>,
) -> Vec<ArchNewsItem> {
    let mut items: Vec<ArchNewsItem> = Vec::new();
    let mut pos = 0;
    while items.len() < limit {
        let Some(start) = body[pos..].find("<item>") else {
            break;
        };
        let s = pos + start;
        let end = body[s..].find("</item>").map_or(body.len(), |e| s + e + 7);
        let chunk = &body[s..end];
        let title = extract_between(chunk, "<title>", "</title>")
            .map(|t| unescape_xml(&t))
            .unwrap_or_default();
        let link = extract_between(chunk, "<link>", "</link>")
            .map(|l| l.trim().to_string())
            .unwrap_or_default();
        let raw_date = extract_between(chunk, "<pubDate>", "</pubDate>")
            .map(|d| d.trim().to_string())
            .unwrap_or_default();
        let date = normalize_feed_date(&raw_date);
        // Early date filtering: stop when items become older than the cutoff
        if let Some(cutoff) = cutoff_date
            && date.as_str() < cutoff
        {
            break;
        }
        items.push(ArchNewsItem {
            date,
            title,
            url: link,
        });
        pos = end;
    }
    items
}

/// What: Fetch recent Arch Linux news items from the official RSS feed.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client (controls timeouts, user agent).
/// - `limit`: Maximum number of items to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date for early filtering.
///
/// Output:
/// - `Ok(Vec<ArchNewsItem>)` with normalized dates, newest first.
///
/// Details:
/// - Fetches `https://archlinux.org/feeds/news/` and delegates to
///   [`parse_arch_news_rss`].
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
/// use arch_toolkit::news::fetch_arch_news;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = reqwest::Client::new();
/// let items = fetch_arch_news(&client, 10, None).await?;
/// for item in items {
///     println!("{} {} ({})", item.date, item.title, item.url);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn fetch_arch_news(
    client: &reqwest::Client,
    limit: usize,
    cutoff_date: Option<&str>,
) -> Result<Vec<ArchNewsItem>> {
    let response = client
        .get(ARCH_NEWS_FEED_URL)
        .send()
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("news feed request failed: {e}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ArchToolkitError::Parse(format!("news feed body read failed: {e}")))?;
    tracing::debug!(
        status = status.as_u16(),
        bytes = body.len(),
        "fetched arch news feed"
    );
    if !status.is_success() {
        return Err(ArchToolkitError::Parse(format!(
            "news feed returned status {status}"
        )));
    }
    Ok(parse_arch_news_rss(&body, limit, cutoff_date))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"><channel>
<item>
  <title>Package &amp; repo changes</title>
  <link>https://archlinux.org/news/pkg-repo-changes/</link>
  <pubDate>Thu, 21 Aug 2025 12:34:56 +0000</pubDate>
</item>
<item>
  <title><![CDATA[Manual intervention required]]></title>
  <link>https://archlinux.org/news/manual-intervention/</link>
  <pubDate>Mon, 04 Aug 2025 09:00:00 +0000</pubDate>
</item>
<item>
  <title>Old news</title>
  <link>https://archlinux.org/news/old-news/</link>
  <pubDate>Wed, 01 Jan 2025 00:00:00 +0000</pubDate>
</item>
</channel></rss>"#;

    #[test]
    /// What: Verify RSS items parse with normalized dates and decoded titles.
    ///
    /// Inputs:
    /// - Sample feed with entity-encoded and CDATA titles.
    ///
    /// Output:
    /// - Three items with `YYYY-MM-DD` dates and clean titles.
    ///
    /// Details:
    /// - Covers the XML unescaping improvement over Pacsea.
    fn parses_items() {
        let items = parse_arch_news_rss(SAMPLE_RSS, 10, None);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].title, "Package & repo changes");
        assert_eq!(items[0].date, "2025-08-21");
        assert_eq!(items[0].url, "https://archlinux.org/news/pkg-repo-changes/");
        assert_eq!(items[1].title, "Manual intervention required");
        assert_eq!(items[1].date, "2025-08-04");
    }

    #[test]
    /// What: Verify the limit parameter truncates results.
    ///
    /// Inputs:
    /// - Sample feed with three items, limit of 1.
    ///
    /// Output:
    /// - Exactly one (newest) item.
    ///
    /// Details:
    /// - Parsing stops early instead of scanning the whole feed.
    fn respects_limit() {
        let items = parse_arch_news_rss(SAMPLE_RSS, 1, None);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].date, "2025-08-21");
    }

    #[test]
    /// What: Verify cutoff-date filtering stops at older items.
    ///
    /// Inputs:
    /// - Sample feed and a cutoff between the second and third item.
    ///
    /// Output:
    /// - Only the two items newer than the cutoff.
    ///
    /// Details:
    /// - Relies on lexicographic comparison of normalized dates.
    fn respects_cutoff() {
        let items = parse_arch_news_rss(SAMPLE_RSS, 10, Some("2025-06-01"));
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.date.as_str() >= "2025-06-01"));
    }

    #[test]
    /// What: Verify empty and malformed input yield no items.
    ///
    /// Inputs:
    /// - Empty string and non-RSS text.
    ///
    /// Output:
    /// - Empty vectors, no panic.
    ///
    /// Details:
    /// - Parser must degrade gracefully on unexpected content.
    fn handles_garbage() {
        assert!(parse_arch_news_rss("", 10, None).is_empty());
        assert!(parse_arch_news_rss("not xml at all", 10, None).is_empty());
        assert!(parse_arch_news_rss("<item>unclosed", 10, None).len() <= 1);
    }

    #[test]
    /// What: Verify XML entity and CDATA unescaping.
    ///
    /// Inputs:
    /// - Entity-encoded strings and CDATA wrappers.
    ///
    /// Output:
    /// - Decoded plain text.
    ///
    /// Details:
    /// - `&amp;` must be decoded last to avoid double-decoding.
    fn unescaping() {
        assert_eq!(unescape_xml("a &amp; b"), "a & b");
        assert_eq!(unescape_xml("&lt;tag&gt;"), "<tag>");
        assert_eq!(unescape_xml("&amp;lt;"), "&lt;");
        assert_eq!(unescape_xml("<![CDATA[raw & text]]>"), "raw & text");
        assert_eq!(unescape_xml("it&#39;s"), "it's");
    }
}
