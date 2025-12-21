//! AUR package comments fetching via web scraping.

use crate::cache::cache_key_comments;
use crate::client::{
    ArchClient, extract_retry_after, is_archlinux_url, rate_limit_archlinux,
    reset_archlinux_backoff, retry_with_policy,
};
use crate::error::{ArchToolkitError, Result};
use crate::types::AurComment;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use reqwest::Client as ReqwestClient;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

/// Context for extracting comment data from HTML elements.
struct CommentExtractionContext<'a> {
    /// Parsed HTML document
    document: &'a Html,
    /// Selector for date elements
    date_selector: &'a Selector,
    /// Package name for URL construction
    pkgname: &'a str,
    /// Full HTML text for pinned detection
    html_text: &'a str,
    /// Whether pinned section exists
    has_pinned_section: bool,
    /// Position of "Latest Comments" heading
    latest_comments_pos: Option<usize>,
}

/// What: Fetch AUR package comments by scraping the AUR package page.
///
/// Inputs:
/// - `client`: `ArchClient` to use for requests.
/// - `pkgname`: Package name to fetch comments for.
///
/// Output:
/// - `Result<Vec<AurComment>>` with parsed comments sorted by date (latest first); `Err` on failure.
///
/// Details:
/// - Fetches HTML from `https://aur.archlinux.org/packages/<pkgname>`
/// - Uses `scraper` to parse HTML and extract comment elements
/// - Parses dates to Unix timestamps for sorting
/// - Sorts comments by date descending (latest first)
/// - Handles pinned comments (appear before "Latest Comments" heading)
/// - Uses retry policy if enabled for comments operations.
/// - Checks cache before making network request if caching is enabled.
///
/// # Errors
/// - Returns `Err(ArchToolkitError::Network)` if the HTTP request fails
/// - Returns `Err(ArchToolkitError::InvalidInput)` if the URL is not from archlinux.org
/// - Returns `Err(ArchToolkitError::Parse)` if HTML parsing fails
pub async fn comments(client: &ArchClient, pkgname: &str) -> Result<Vec<AurComment>> {
    // Check cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_comments
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_comments(pkgname);
        if let Some(cached) = cache.get::<Vec<AurComment>>(&cache_key) {
            debug!(pkgname = %pkgname, "cache hit for comments");
            return Ok(cached);
        }
    }

    let url = format!("https://aur.archlinux.org/packages/{pkgname}");

    debug!(pkgname = %pkgname, url = %url, "fetching AUR comments");

    // Apply rate limiting for archlinux.org
    let _permit = if is_archlinux_url(&url) {
        rate_limit_archlinux().await
    } else {
        return Err(ArchToolkitError::InvalidInput(format!(
            "Unexpected URL domain: {url}"
        )));
    };

    let retry_policy = client.retry_policy();
    let http_client = client.http_client();

    // Wrap the request in retry logic if enabled
    let html_text = if retry_policy.enabled && retry_policy.retry_comments {
        retry_with_policy(retry_policy, "comments", || async {
            perform_comments_request(http_client, &url).await
        })
        .await?
    } else {
        perform_comments_request(http_client, &url).await?
    };

    // Parse HTML
    let result = parse_comments_html(&html_text, pkgname)?;

    // Store in cache if enabled
    if let Some(cache_config) = client.cache_config()
        && cache_config.enable_comments
        && let Some(cache) = client.cache()
    {
        let cache_key = cache_key_comments(pkgname);
        let _ = cache.set(&cache_key, &result, cache_config.comments_ttl);
    }

    Ok(result)
}

/// What: Perform the actual comments request without retry logic.
///
/// Inputs:
/// - `client`: HTTP client to use for requests.
/// - `url`: URL to request.
///
/// Output:
/// - `Result<String>` containing HTML text, or an error.
///
/// Details:
/// - Internal helper function that performs the HTTP request
/// - Used by both retry and non-retry code paths
async fn perform_comments_request(client: &ReqwestClient, url: &str) -> Result<String> {
    // Create request with browser-like headers
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.5"));

    let response = match client.get(url).headers(headers).send().await {
        Ok(resp) => {
            reset_archlinux_backoff();
            resp
        }
        Err(e) => {
            debug!(error = %e, "AUR comments request failed");
            return Err(ArchToolkitError::Network(e));
        }
    };

    // Check for Retry-After header before consuming response
    let _retry_after = extract_retry_after(&response);

    let response = match response.error_for_status() {
        Ok(resp) => resp,
        Err(e) => {
            debug!(error = %e, "AUR comments returned non-success status");
            return Err(ArchToolkitError::Network(e));
        }
    };

    let html_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            debug!(error = %e, "failed to read AUR comments response");
            return Err(ArchToolkitError::Network(e));
        }
    };

    Ok(html_text)
}

/// What: Parse HTML and extract comments.
///
/// Inputs:
/// - `html_text`: HTML text to parse.
/// - `pkgname`: Package name for context.
///
/// Output:
/// - `Result<Vec<AurComment>>` with parsed comments.
///
/// Details:
/// - Internal helper function that parses HTML and extracts comments
/// - Separated from request logic for reuse
fn parse_comments_html(html_text: &str, pkgname: &str) -> Result<Vec<AurComment>> {
    // Parse HTML
    let document = Html::parse_document(html_text);

    // AUR comments structure:
    // - Each comment has an <h4 class="comment-header"> with author and date
    // - The content is in a following <div class="article-content"> with id "comment-{id}-content"
    // - Pinned comments appear before "Latest Comments" heading
    let comment_header_selector = Selector::parse("h4.comment-header").map_err(|e| {
        ArchToolkitError::Parse(format!("Failed to parse comment header selector: {e}"))
    })?;

    let date_selector = Selector::parse("a.date")
        .map_err(|e| ArchToolkitError::Parse(format!("Failed to parse date selector: {e}")))?;

    // Find the "Latest Comments" heading to separate pinned from regular comments
    let heading_selector = Selector::parse("h3, h2, h4")
        .map_err(|e| ArchToolkitError::Parse(format!("Failed to parse heading selector: {e}")))?;

    // Check if there's a "Pinned Comments" section
    let has_pinned_section = document.select(&heading_selector).any(|h| {
        let text: String = h.text().collect();
        text.contains("Pinned Comments")
    });

    // Find the "Latest Comments" heading position in the HTML text
    let html_text_lower = html_text.to_lowercase();
    let latest_comments_pos = html_text_lower.find("latest comments");

    // Collect all headers
    let all_headers: Vec<_> = document.select(&comment_header_selector).collect();

    // Use a HashSet to track seen comment IDs to avoid duplicates
    let mut seen_comment_ids = std::collections::HashSet::new();
    let mut comments = Vec::new();

    // Process each header and find its corresponding content by ID
    for (index, header) in all_headers.iter().enumerate() {
        // Extract comment ID from header
        let comment_id = header.value().attr("id");

        // Skip if we've already seen this comment ID (deduplication)
        if let Some(id) = comment_id
            && !seen_comment_ids.insert(id)
        {
            continue; // Skip duplicate
        }

        // Extract comment data from header
        let context = CommentExtractionContext {
            document: &document,
            date_selector: &date_selector,
            pkgname,
            html_text,
            has_pinned_section,
            latest_comments_pos,
        };
        if let Some(comment) = extract_comment_from_header(header, comment_id, index, &context) {
            comments.push(comment);
        }
    }

    // Separate, sort, and combine comments
    Ok(separate_and_sort_comments(comments))
}

/// What: Extract comment data from a header element.
///
/// Inputs:
/// - `header`: Header element containing comment metadata
/// - `comment_id`: Optional comment ID from header attribute
/// - `index`: Index of header in collection
/// - `context`: Extraction context with document, selectors, and metadata
///
/// Output:
/// - `Some(AurComment)` if comment is valid; `None` if empty/invalid
///
/// Details:
/// - Extracts author, date, URL, content, and pinned status
/// - Skips empty comments with unknown authors
fn extract_comment_from_header(
    header: &ElementRef,
    comment_id: Option<&str>,
    index: usize,
    context: &CommentExtractionContext,
) -> Option<AurComment> {
    // Extract the full header text to parse author
    let header_text = header.text().collect::<String>();

    // Extract author: text before " commented on"
    let author = header_text.find(" commented on ").map_or_else(
        || {
            // Fallback: try to find author in links or text nodes
            header_text
                .split_whitespace()
                .next()
                .unwrap_or("Unknown")
                .to_string()
        },
        |pos| header_text[..pos].trim().to_string(),
    );

    // Extract date and URL from <a class="date"> inside the header
    let base_url = format!("https://aur.archlinux.org/packages/{}", context.pkgname);
    let (date_text, date_url) = header.select(context.date_selector).next().map_or_else(
        || (String::new(), None),
        |e| {
            let text = e.text().collect::<String>().trim().to_string();
            let url = e.value().attr("href").map(|href| {
                // Convert relative URLs to absolute
                if href.starts_with("http://") || href.starts_with("https://") {
                    href.to_string()
                } else if href.starts_with('#') {
                    // Fragment-only URL: combine with package page URL
                    format!("{base_url}{href}")
                } else {
                    // Relative path: prepend AUR domain
                    format!("https://aur.archlinux.org{href}")
                }
            });
            (text, url)
        },
    );

    // Get content by finding the corresponding content div by ID
    let comment_content = comment_id
        .and_then(|id| id.strip_prefix("comment-"))
        .and_then(|comment_id_str| {
            Selector::parse(&format!("div#comment-{comment_id_str}-content")).ok()
        })
        .and_then(|content_id_selector| context.document.select(&content_id_selector).next())
        .map_or_else(String::new, |div| {
            // Parse HTML and extract formatted text
            html_to_formatted_text(div)
        });

    // Skip empty comments
    if comment_content.is_empty() && author == "Unknown" {
        return None;
    }

    // Parse date to timestamp
    let date_timestamp = parse_date_to_timestamp(&date_text);
    if date_timestamp.is_none() && !date_text.is_empty() {
        debug!(
            pkgname = %context.pkgname,
            author = %author,
            date_text = %date_text,
            "Failed to parse comment date to timestamp"
        );
    }

    // Convert UTC date to local timezone for display
    let local_date = convert_utc_to_local_date(&date_text);

    // Determine if this comment is pinned
    let is_pinned = determine_pinned_status(comment_id, index, context);

    let stable_id = comment_id.map(str::to_string).or_else(|| date_url.clone());
    Some(AurComment {
        id: stable_id,
        author,
        date: local_date,
        date_timestamp,
        date_url,
        content: comment_content,
        pinned: is_pinned,
    })
}

/// What: Determine if a comment is pinned based on its position in the HTML.
///
/// Inputs:
/// - `comment_id`: Optional comment ID
/// - `index`: Index of comment in collection
/// - `context`: Extraction context with HTML text and pinned section info
///
/// Output:
/// - `true` if comment is pinned; `false` otherwise
///
/// Details:
/// - Pinned comments appear before the "Latest Comments" heading
fn determine_pinned_status(
    comment_id: Option<&str>,
    index: usize,
    context: &CommentExtractionContext,
) -> bool {
    if !context.has_pinned_section {
        return false;
    }

    let Some(latest_pos) = context.latest_comments_pos else {
        return false;
    };

    comment_id.map_or(index < 10, |id| {
        context
            .html_text
            .find(id)
            .map_or(index < 10, |comment_pos| comment_pos < latest_pos)
    })
}

/// What: Separate pinned and regular comments, sort them, and combine.
///
/// Inputs:
/// - `comments`: Vector of all comments
///
/// Output:
/// - Vector with pinned comments first, then regular, both sorted by date descending
///
/// Details:
/// - Separates comments into pinned and regular
/// - Sorts each group by date descending (latest first)
/// - Combines with pinned first
fn separate_and_sort_comments(comments: Vec<AurComment>) -> Vec<AurComment> {
    // Separate pinned and regular comments
    let mut pinned_comments: Vec<AurComment> =
        comments.iter().filter(|c| c.pinned).cloned().collect();
    let mut regular_comments: Vec<AurComment> =
        comments.into_iter().filter(|c| !c.pinned).collect();

    // Sort both groups by date descending
    sort_comments_by_date(&mut pinned_comments);
    sort_comments_by_date(&mut regular_comments);

    // Combine: pinned first, then regular
    pinned_comments.extend(regular_comments);
    pinned_comments
}

/// What: Sort comments by date descending (latest first).
///
/// Inputs:
/// - `comments`: Mutable reference to comments vector to sort
///
/// Output:
/// - Comments are sorted in-place by date descending
///
/// Details:
/// - Uses timestamp for sorting if available
/// - Falls back to string comparison if timestamp is missing
fn sort_comments_by_date(comments: &mut [AurComment]) {
    comments.sort_by(|a, b| {
        match (a.date_timestamp, b.date_timestamp) {
            (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a), // Descending order
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.date.cmp(&a.date), // Fallback to string comparison
        }
    });
}

/// What: Convert UTC date string from AUR to local timezone string.
///
/// Inputs:
/// - `utc_date_str`: UTC date string from AUR page (e.g., "2025-05-15 03:55 (UTC)").
///
/// Output:
/// - Local timezone date string formatted as "YYYY-MM-DD HH:MM (TZ)" where TZ is local timezone abbreviation.
/// - Returns original string if parsing fails.
///
/// Details:
/// - Parses UTC date from AUR format
/// - Converts to local timezone using system timezone
/// - Formats with local timezone abbreviation
fn convert_utc_to_local_date(utc_date_str: &str) -> String {
    let utc_date_str = utc_date_str.trim();

    // AUR format: "YYYY-MM-DD HH:MM (UTC)" or "YYYY-MM-DD HH:MM (CEST)" etc.
    // Try to parse the date/time part before the timezone
    if let Some(tz_start) = utc_date_str.rfind('(') {
        let date_time_part = utc_date_str[..tz_start].trim();

        // Try parsing "YYYY-MM-DD HH:MM" format as UTC
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(date_time_part, "%Y-%m-%d %H:%M") {
            // Treat as UTC and convert to local timezone
            let utc_dt = naive_dt.and_utc();
            let local_dt = utc_dt.with_timezone(&Local);

            // Format with local timezone
            let formatted = local_dt.format("%Y-%m-%d %H:%M");

            // Get timezone abbreviation
            let tz_abbr = get_timezone_abbreviation(&local_dt);

            return format!("{formatted} ({tz_abbr})");
        }
    }

    // If parsing fails, return original string
    utc_date_str.to_string()
}

/// What: Get timezone abbreviation (CEST, CET, PST, etc.) for a local datetime.
///
/// Inputs:
/// - `local_dt`: Local datetime to get timezone for.
///
/// Output:
/// - Timezone abbreviation string (e.g., "CEST", "CET", "UTC+2").
///
/// Details:
/// - First tries chrono's %Z format specifier
/// - Falls back to TZ environment variable parsing
/// - Finally falls back to UTC offset format
fn get_timezone_abbreviation(local_dt: &DateTime<Local>) -> String {
    // Try chrono's %Z format specifier first
    let tz_from_format = local_dt.format("%Z").to_string();

    // Check if %Z gave us a valid abbreviation (3-6 chars, alphabetic)
    if !tz_from_format.is_empty()
        && tz_from_format.len() >= 3
        && tz_from_format.len() <= 6
        && tz_from_format.chars().all(char::is_alphabetic)
        && !tz_from_format.starts_with("UTC")
    {
        return tz_from_format;
    }

    // Try to get timezone from TZ environment variable
    if let Ok(tz_env) = std::env::var("TZ") {
        // Extract timezone abbreviation from TZ variable
        if let Some(tz_name) = tz_env.rsplit('/').next() {
            // Check if it looks like a timezone abbreviation (3-6 uppercase letters)
            if tz_name.len() >= 3
                && tz_name.len() <= 6
                && tz_name.chars().all(|c| c.is_uppercase() || c == '-')
            {
                // Extract just the abbreviation part (before any offset)
                let abbr = tz_name.split('-').next().unwrap_or(tz_name);
                if abbr.len() >= 3 && abbr.chars().all(char::is_alphabetic) {
                    return abbr.to_string();
                }
            }
        }
    }

    // Fallback: Use UTC offset format
    let offset_secs = local_dt.offset().local_minus_utc();
    let hours = offset_secs / 3600;
    let minutes = (offset_secs.abs() % 3600) / 60;

    if offset_secs == 0 {
        "UTC".to_string()
    } else if minutes == 0 {
        format!("UTC{hours:+}")
    } else {
        format!("UTC{hours:+}:{minutes:02}")
    }
}

/// What: Parse a date string to Unix timestamp.
///
/// Inputs:
/// - `date_str`: Date string from AUR page (e.g., "2025-05-15 03:55 (UTC)").
///
/// Output:
/// - `Some(i64)` with Unix timestamp if parsing succeeds; `None` otherwise.
///
/// Details:
/// - Attempts to parse common AUR date formats
/// - AUR uses format: "YYYY-MM-DD HH:MM (TZ)" where TZ is timezone abbreviation
/// - Returns None if parsing fails (will use string comparison for sorting)
fn parse_date_to_timestamp(date_str: &str) -> Option<i64> {
    let date_str = date_str.trim();

    // Skip empty strings early
    if date_str.is_empty() {
        return None;
    }

    // AUR format: "YYYY-MM-DD HH:MM (UTC)" or "YYYY-MM-DD HH:MM (CEST)" etc.
    // Try to parse the date/time part before the timezone
    if let Some(tz_start) = date_str.rfind('(') {
        let date_time_part = date_str[..tz_start].trim();

        // Try parsing "YYYY-MM-DD HH:MM" format
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_time_part, "%Y-%m-%d %H:%M") {
            // AUR dates are in UTC, so we can treat them as UTC
            return Some(dt.and_utc().timestamp());
        }

        // Try with seconds: "YYYY-MM-DD HH:MM:SS"
        if let Ok(dt) = NaiveDateTime::parse_from_str(date_time_part, "%Y-%m-%d %H:%M:%S") {
            return Some(dt.and_utc().timestamp());
        }
    }

    // Try ISO 8601-like format: "YYYY-MM-DD HH:MM:SS"
    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc().timestamp());
    }

    // Try ISO 8601 format: "YYYY-MM-DDTHH:MM:SS" (with T separator)
    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().timestamp());
    }

    // Try ISO 8601 with timezone: "YYYY-MM-DDTHH:MM:SSZ" or "YYYY-MM-DDTHH:MM:SS+HH:MM"
    if let Ok(dt) = DateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%z") {
        return Some(dt.timestamp());
    }

    // Try date-only format: "YYYY-MM-DD"
    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        && let Some(dt) = d.and_hms_opt(0, 0, 0)
    {
        return Some(dt.and_utc().timestamp());
    }

    // Try Unix timestamp as string
    if let Ok(ts) = date_str.parse::<i64>() {
        // Validate it's a reasonable timestamp (between 2000 and 2100)
        if ts > 946_684_800 && ts < 4_102_444_800 {
            return Some(ts);
        }
    }

    None
}

/// What: Convert HTML content to formatted text preserving markdown-like structures.
///
/// Inputs:
/// - `element`: HTML element to parse
///
/// Output:
/// - Formatted text string with markdown-like syntax for bold, italic, code, etc.
///
/// Details:
/// - Converts HTML tags to markdown-like syntax:
///   - `<strong>`, `<b>` → `**text**`
///   - `<em>`, `<i>` → `*text*`
///   - `<code>` → `` `text` ``
///   - `<pre>` → preserves code blocks with triple backticks
///   - `<a>` → preserves links as `[text](url)`
///   - `<p>` → newlines between paragraphs
fn html_to_formatted_text(element: ElementRef) -> String {
    let mut result = String::new();

    // Process paragraphs to preserve structure
    let p_selector = Selector::parse("p").ok();
    if let Some(ref p_sel) = p_selector {
        let paragraphs: Vec<_> = element.select(p_sel).collect();
        if !paragraphs.is_empty() {
            for (i, p) in paragraphs.iter().enumerate() {
                if i > 0 {
                    result.push_str("\n\n");
                }
                result.push_str(&format_text_node(p));
            }
            return result;
        }
    }

    // If no paragraphs, process as a single text node
    format_text_node(&element)
}

/// Format a single HTML element to text with markdown-like syntax.
fn format_text_node(element: &ElementRef) -> String {
    // Simple approach: extract all text and format common HTML tags
    let mut result = element.html();

    // Process <pre> blocks (code blocks)
    let pre_selector = Selector::parse("pre").ok();
    if let Some(ref pre_sel) = pre_selector {
        for pre in element.select(pre_sel) {
            let text = pre.text().collect::<String>();
            let pre_html = pre.html();
            let replacement = format!("```\n{}\n```", text.trim());
            result = result.replace(&pre_html, &replacement);
        }
    }

    // Process <a> tags (links)
    let a_selector = Selector::parse("a").ok();
    if let Some(ref a_sel) = a_selector {
        for link in element.select(a_sel) {
            let text = link.text().collect::<String>().trim().to_string();
            if let Some(href) = link.value().attr("href") {
                let link_html = link.html();
                let replacement = format!("[{text}]({href})");
                result = result.replace(&link_html, &replacement);
            }
        }
    }

    // Process <strong> and <b> tags (bold)
    let strong_selector = Selector::parse("strong, b").ok();
    if let Some(ref strong_sel) = strong_selector {
        for bold in element.select(strong_sel) {
            let text = bold.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                let bold_html = bold.html();
                let replacement = format!("**{text}**");
                result = result.replace(&bold_html, &replacement);
            }
        }
    }

    // Process <em> and <i> tags (italic)
    let em_selector = Selector::parse("em, i").ok();
    if let Some(ref em_sel) = em_selector {
        for italic in element.select(em_sel) {
            let text = italic.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                let italic_html = italic.html();
                let replacement = format!("*{text}*");
                result = result.replace(&italic_html, &replacement);
            }
        }
    }

    // Process <code> tags
    let code_selector = Selector::parse("code").ok();
    if let Some(ref code_sel) = code_selector {
        for code in element.select(code_sel) {
            let text = code.text().collect::<String>().trim().to_string();
            if !text.is_empty() {
                let code_html = code.html();
                let replacement = format!("`{text}`");
                result = result.replace(&code_html, &replacement);
            }
        }
    }

    // Parse the modified HTML and extract text (this removes remaining HTML tags)
    let temp_doc = Html::parse_fragment(&result);
    let text_result = temp_doc.root_element().text().collect::<String>();

    // Replace <br> with newlines
    text_result
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
}
