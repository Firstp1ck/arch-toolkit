//! Arch Linux news RSS feed fetching and parsing.

use crate::error::Result;
use crate::types::news::ArchNewsItem;

use super::article::fetch_bounded_text;
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

/// Maximum response size accepted for a raw news or advisory feed payload.
pub const MAX_FEED_RESPONSE_BYTES: usize = 512 * 1024;

/// What: Fetch recent Arch Linux news from the official feed URL.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client controlling transport policy.
/// - `limit`: Maximum number of items to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date for early filtering.
///
/// Output:
/// - `Ok(Vec<ArchNewsItem>)` with normalized dates, newest first.
///
/// Details:
/// - Delegates to [`fetch_arch_news_from`] with [`ARCH_NEWS_FEED_URL`].
/// - No cache is used unless callers opt into [`fetch_arch_news_cached`].
///
/// # Errors
///
/// Returns an error for transport, response-status, response-bound, or UTF-8
/// failures.
pub async fn fetch_arch_news(
    client: &reqwest::Client,
    limit: usize,
    cutoff_date: Option<&str>,
) -> Result<Vec<ArchNewsItem>> {
    fetch_arch_news_from(client, ARCH_NEWS_FEED_URL, limit, cutoff_date).await
}

/// What: Fetch and parse Arch news from a caller-specified feed URL.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client controlling transport policy.
/// - `feed_url`: Absolute HTTP(S) RSS URL, useful for proxies and fixtures.
/// - `limit`: Maximum number of items to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date for early filtering.
///
/// Output:
/// - Parsed news items from the successful bounded feed response.
///
/// Details:
/// - Reads at most [`MAX_FEED_RESPONSE_BYTES`] and keeps caller-client
///   configuration rather than constructing a hidden HTTP client.
///
/// # Errors
///
/// Returns an error for invalid URLs, failed requests, non-success statuses,
/// oversized bodies, or invalid UTF-8.
pub async fn fetch_arch_news_from(
    client: &reqwest::Client,
    feed_url: &str,
    limit: usize,
    cutoff_date: Option<&str>,
) -> Result<Vec<ArchNewsItem>> {
    let body = fetch_bounded_text(client, feed_url, MAX_FEED_RESPONSE_BYTES, "news feed").await?;
    tracing::debug!(bytes = body.len(), "fetched arch news feed");
    Ok(parse_arch_news_rss(&body, limit, cutoff_date))
}

/// What: Fetch official Arch news with an optional caller-owned feed cache.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client controlling transport policy.
/// - `limit`: Maximum number of items to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date for early filtering.
/// - `cache`: Optional generic feed cache; `None` always fetches fresh content.
///
/// Output:
/// - Parsed news items from a cache hit or successful bounded HTTP response.
///
/// Details:
/// - Delegates to [`fetch_arch_news_cached_from`] using the official feed URL.
/// - The generic cache is independent from AUR cache types and freshness policy.
///
/// # Errors
///
/// Returns requested cache, transport, status, bound, or UTF-8 errors.
pub async fn fetch_arch_news_cached(
    client: &reqwest::Client,
    limit: usize,
    cutoff_date: Option<&str>,
    cache: Option<&dyn super::FeedCache>,
) -> Result<Vec<ArchNewsItem>> {
    fetch_arch_news_cached_from(client, ARCH_NEWS_FEED_URL, limit, cutoff_date, cache).await
}

/// What: Fetch caller-specified Arch news with an optional generic feed cache.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client controlling transport policy.
/// - `feed_url`: Absolute HTTP(S) RSS URL, useful for proxies and fixtures.
/// - `limit`: Maximum number of items to return (best-effort).
/// - `cutoff_date`: Optional `YYYY-MM-DD` date for early filtering.
/// - `cache`: Optional generic feed cache; `None` always fetches fresh content.
///
/// Output:
/// - Parsed news items from a cache hit or a newly stored successful response.
///
/// Details:
/// - Raw payloads are cached by feed kind and URL before caller-specific limit
///   and cutoff parsing, so one bounded cached feed supports multiple queries.
///
/// # Errors
///
/// Returns requested cache, transport, status, bound, or UTF-8 errors.
pub async fn fetch_arch_news_cached_from(
    client: &reqwest::Client,
    feed_url: &str,
    limit: usize,
    cutoff_date: Option<&str>,
    cache: Option<&dyn super::FeedCache>,
) -> Result<Vec<ArchNewsItem>> {
    let body = fetch_cached_feed_text(client, feed_url, "arch-news", "news feed", cache).await?;
    Ok(parse_arch_news_rss(&body, limit, cutoff_date))
}

/// What: Fetch a raw feed response, consulting an optional generic cache first.
///
/// Inputs:
/// - `client`: Caller-provided HTTP client controlling transport policy.
/// - `feed_url`: Absolute HTTP(S) feed URL.
/// - `feed_kind`: Namespace preventing RSS/Atom cache-key collisions.
/// - `resource_name`: Human-readable feed name used in actionable errors.
/// - `cache`: Optional generic caller-owned feed cache.
///
/// Output:
/// - Bounded UTF-8 feed text from the cache or a successful HTTP response.
///
/// Details:
/// - Only successful HTTP responses are cached; cache errors are explicit when
///   callers request caching rather than silently degrading durable semantics.
///
/// # Errors
///
/// Returns cache, transport, status, bound, or UTF-8 errors.
pub(super) async fn fetch_cached_feed_text(
    client: &reqwest::Client,
    feed_url: &str,
    feed_kind: &str,
    resource_name: &str,
    cache: Option<&dyn super::FeedCache>,
) -> Result<String> {
    let cache_key = feed_cache_key(feed_kind, feed_url);
    if let Some(cache) = cache
        && let Some(body) = cache.get(&cache_key)?
    {
        return Ok(body);
    }

    let body = fetch_bounded_text(client, feed_url, MAX_FEED_RESPONSE_BYTES, resource_name).await?;
    if let Some(cache) = cache {
        cache.put(&cache_key, &body)?;
    }
    Ok(body)
}

/// What: Build a namespaced stable cache key for one feed URL.
///
/// Inputs:
/// - `feed_kind`: Short feed-type namespace.
/// - `feed_url`: Caller-selected absolute feed URL.
///
/// Output:
/// - A cache key that distinguishes RSS news from Atom advisories.
///
/// Details:
/// - URL-specific keys permit fixture/proxy URLs without coupling news to AUR
///   cache internals or their key formats.
pub(super) fn feed_cache_key(feed_kind: &str, feed_url: &str) -> String {
    format!("{feed_kind}:{feed_url}")
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
