//! Safe, bounded HTML-to-text extraction for news article pages.

use crate::error::{ArchToolkitError, Result};

/// Maximum HTML response size accepted for one article extraction.
pub const MAX_ARTICLE_HTML_BYTES: usize = 512 * 1024;
/// Maximum text size emitted from one article extraction.
pub const MAX_ARTICLE_TEXT_BYTES: usize = 256 * 1024;
/// Maximum raw anchor destination length accepted before URL resolution.
const MAX_LINK_BYTES: usize = 4 * 1024;

/// What: Extract readable text from a bounded HTML news article.
///
/// Inputs:
/// - `html`: Article HTML, limited to [`MAX_ARTICLE_HTML_BYTES`] bytes.
/// - `base_url`: Absolute HTTP(S) article URL used to resolve relative links.
///
/// Output:
/// - Plain text with paragraphs, list items, code blocks, and Markdown-safe
///   HTTP(S) links, or an explicit validation/size error.
///
/// Details:
/// - Script, style, template, and noscript content is discarded rather than
///   interpreted. HTML is never executed.
/// - Relative links are resolved against `base_url`; non-HTTP(S) links remain
///   readable text without a destination.
/// - This intentionally small extractor is not a general browser or HTML
///   sanitizer. It preserves the article structures needed by news callers
///   while enforcing input and output bounds.
///
/// # Errors
///
/// Returns `ArchToolkitError::InputTooLong` when the input or output exceeds
/// its bound, and `ArchToolkitError::InvalidInput` for an invalid base URL.
pub fn extract_article_text(html: &str, base_url: &str) -> Result<String> {
    ensure_article_input_bound(html)?;
    let base = parse_article_base_url(base_url)?;
    let mut extractor = ArticleTextExtractor::new();
    scan_article_html(html, &base, &mut extractor)?;
    extractor.finish()
}

/// What: Fetch one article through a caller-provided client and extract its text.
///
/// Inputs:
/// - `client`: Caller-configured reqwest client controlling transport policy.
/// - `article_url`: Absolute HTTP(S) article URL to fetch and use as link base.
///
/// Output:
/// - Extracted bounded article text or a transport, status, validation, or
///   extraction error.
///
/// Details:
/// - The caller owns timeouts, redirects, proxy policy, and fetch cadence.
/// - The response body is read incrementally and rejected above
///   [`MAX_ARTICLE_HTML_BYTES`].
///
/// # Errors
///
/// Returns an error for invalid URLs, failed requests, non-success responses,
/// oversized bodies, invalid UTF-8, or extraction failures.
pub async fn fetch_article_text(client: &reqwest::Client, article_url: &str) -> Result<String> {
    let html =
        fetch_bounded_text(client, article_url, MAX_ARTICLE_HTML_BYTES, "news article").await?;
    extract_article_text(&html, article_url)
}

/// What: Read a successful HTTP response into a string without exceeding a byte bound.
///
/// Inputs:
/// - `client`: Caller-configured reqwest client.
/// - `url`: HTTP(S) URL to request.
/// - `maximum_bytes`: Inclusive response-size bound.
/// - `resource_name`: Human-readable resource label for errors.
///
/// Output:
/// - UTF-8 response text no larger than `maximum_bytes`.
///
/// Details:
/// - Checks both Content-Length when provided and streamed chunks, so omitted
///   or misleading headers cannot bypass the bound.
/// - Shared by feed and article fetchers without depending on AUR internals.
///
/// # Errors
///
/// Returns an error for zero bounds, request failures, non-success statuses,
/// oversized responses, or invalid UTF-8.
pub(super) async fn fetch_bounded_text(
    client: &reqwest::Client,
    url: &str,
    maximum_bytes: usize,
    resource_name: &str,
) -> Result<String> {
    if maximum_bytes == 0 {
        return Err(ArchToolkitError::InvalidInput(format!(
            "{resource_name} response bound must be greater than zero"
        )));
    }
    let parsed_url = parse_http_url(url, resource_name)?;
    let mut response = client.get(parsed_url).send().await.map_err(|error| {
        ArchToolkitError::Parse(format!("{resource_name} request failed: {error}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ArchToolkitError::Parse(format!(
            "{resource_name} returned status {status}"
        )));
    }

    let maximum_length = u64::try_from(maximum_bytes).map_err(|_| {
        ArchToolkitError::InvalidInput(format!("{resource_name} response bound is too large"))
    })?;
    if response
        .content_length()
        .is_some_and(|length| length > maximum_length)
    {
        return Err(response_too_large_error(resource_name, maximum_bytes));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        ArchToolkitError::Parse(format!("{resource_name} response read failed: {error}"))
    })? {
        if chunk.len() > maximum_bytes.saturating_sub(bytes.len()) {
            return Err(response_too_large_error(resource_name, maximum_bytes));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|error| {
        ArchToolkitError::Parse(format!(
            "{resource_name} response was not valid UTF-8: {error}"
        ))
    })
}

/// What: Validate an absolute article base URL.
///
/// Inputs:
/// - `base_url`: Candidate absolute URL supplied by the caller.
///
/// Output:
/// - Parsed HTTP(S) URL suitable for relative-link resolution.
///
/// Details:
/// - Rejects non-HTTP(S) schemes before any extraction output is generated.
fn parse_article_base_url(base_url: &str) -> Result<reqwest::Url> {
    parse_http_url(base_url, "article base URL")
}

/// What: Validate an HTTP(S) URL for a bounded caller-owned request.
///
/// Inputs:
/// - `url`: Candidate URL to parse.
/// - `resource_name`: Resource label used in errors.
///
/// Output:
/// - Parsed URL restricted to `http` or `https`.
///
/// Details:
/// - Does not make a request; caller-client transport policy remains in the
///   supplied reqwest client.
fn parse_http_url(url: &str, resource_name: &str) -> Result<reqwest::Url> {
    let parsed = reqwest::Url::parse(url).map_err(|error| {
        ArchToolkitError::InvalidInput(format!("invalid {resource_name} URL: {error}"))
    })?;
    if matches!(parsed.scheme(), "http" | "https") {
        return Ok(parsed);
    }
    Err(ArchToolkitError::InvalidInput(format!(
        "{resource_name} URL must use http or https"
    )))
}

/// What: Build a consistent explicit error for response-bound violations.
///
/// Inputs:
/// - `resource_name`: Resource label shown to callers.
/// - `maximum_bytes`: Configured maximum body size.
///
/// Output:
/// - `ArchToolkitError::InputTooLong` with the known safety bound.
///
/// Details:
/// - The actual streamed size is intentionally not reported because it may be
///   incomplete when the body is rejected mid-stream.
fn response_too_large_error(resource_name: &str, maximum_bytes: usize) -> ArchToolkitError {
    ArchToolkitError::InputTooLong {
        field: format!("{resource_name} response"),
        max_length: maximum_bytes,
        actual_length: maximum_bytes.saturating_add(1),
    }
}

/// What: Reject article HTML larger than the extractor's fixed bound.
///
/// Inputs:
/// - `html`: Candidate article HTML.
///
/// Output:
/// - `Ok(())` within the bound, otherwise `InputTooLong`.
///
/// Details:
/// - This check occurs before parsing to bound scanner work and allocations.
fn ensure_article_input_bound(html: &str) -> Result<()> {
    if html.len() <= MAX_ARTICLE_HTML_BYTES {
        return Ok(());
    }
    Err(ArchToolkitError::InputTooLong {
        field: "article HTML".to_string(),
        max_length: MAX_ARTICLE_HTML_BYTES,
        actual_length: html.len(),
    })
}

/// What: Scan article HTML and dispatch text/tag tokens to an extractor.
///
/// Inputs:
/// - `html`: Previously bounded article markup.
/// - `base_url`: Valid HTTP(S) URL for relative links.
/// - `extractor`: Mutable extraction state.
///
/// Output:
/// - `Ok(())` after all complete tokens are processed.
///
/// Details:
/// - Unterminated tags are treated as literal text, which avoids silently
///   dropping visible content from malformed pages.
fn scan_article_html(
    html: &str,
    base_url: &reqwest::Url,
    extractor: &mut ArticleTextExtractor,
) -> Result<()> {
    let mut remaining = html;
    while let Some(start) = remaining.find('<') {
        extractor.append_text(&remaining[..start])?;
        remaining = &remaining[start..];
        if let Some(after_comment) = skip_html_comment(remaining) {
            remaining = after_comment;
            continue;
        }
        let Some(end) = find_tag_end(remaining) else {
            extractor.append_text(remaining)?;
            return Ok(());
        };
        extractor.handle_tag(&remaining[1..end], base_url)?;
        remaining = &remaining[end + 1..];
    }
    extractor.append_text(remaining)
}

/// What: Skip one complete HTML comment when the input begins with one.
///
/// Inputs:
/// - `input`: Remaining HTML beginning at a possible `<` token.
///
/// Output:
/// - Remaining input after a complete comment, or `None` for non-comments and
///   unterminated comments.
///
/// Details:
/// - Comment text is never emitted into article output.
fn skip_html_comment(input: &str) -> Option<&str> {
    let suffix = input.strip_prefix("<!--")?;
    let end = suffix.find("-->")?;
    Some(&suffix[end + 3..])
}

/// What: Locate the closing `>` of one HTML tag while respecting quotes.
///
/// Inputs:
/// - `input`: Remaining HTML that starts with `<`.
///
/// Output:
/// - Byte index of the closing `>`, or `None` for an unterminated tag.
///
/// Details:
/// - Quoted attribute values may contain `>` and must not terminate a tag.
fn find_tag_end(input: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in input.char_indices().skip(1) {
        match (quote, character) {
            (None, '\'' | '"') => quote = Some(character),
            (Some(active), current) if active == current => quote = None,
            (None, '>') => return Some(index),
            _ => {}
        }
    }
    None
}

/// What: Hold one open link's output position and resolved destination.
///
/// Inputs:
/// - Created when an opening anchor tag is encountered.
///
/// Output:
/// - Allows the closing tag to append a safe destination or discard an empty link.
///
/// Details:
/// - Invalid or non-HTTP(S) destinations use `None` and preserve label text.
struct OpenLink {
    /// Position of the opening `[` in the output buffer.
    output_start: usize,
    /// Resolved and Markdown-escaped HTTP(S) destination when valid.
    destination: Option<String>,
}

/// What: Maintain bounded structural state while scanning article HTML.
///
/// Inputs:
/// - Constructed internally by [`extract_article_text`].
///
/// Output:
/// - A plain-text buffer with preserved supported article structures.
///
/// Details:
/// - Suppressed-tag, preformatted, inline-code, and link stacks are independent
///   so malformed nested tags cannot execute or alter unrelated parser state.
struct ArticleTextExtractor {
    /// Accumulated extracted text.
    output: String,
    /// Nested tags whose text must be discarded.
    suppressed_tags: Vec<String>,
    /// Nesting depth for preformatted code blocks.
    pre_depth: usize,
    /// Nesting depth for inline code tags outside preformatted blocks.
    inline_code_depth: usize,
    /// Open anchors awaiting their closing tag.
    open_links: Vec<OpenLink>,
}

impl ArticleTextExtractor {
    /// What: Create empty extraction state.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - Fresh parser state with no visible text or open structures.
    ///
    /// Details:
    /// - All state remains local to one extraction call.
    const fn new() -> Self {
        Self {
            output: String::new(),
            suppressed_tags: Vec::new(),
            pre_depth: 0,
            inline_code_depth: 0,
            open_links: Vec::new(),
        }
    }

    /// What: Append one visible text token using context-appropriate whitespace.
    ///
    /// Inputs:
    /// - `text`: Raw HTML text token outside tag delimiters.
    ///
    /// Output:
    /// - Updates the output buffer or reports a text-size violation.
    ///
    /// Details:
    /// - Preformatted sections preserve whitespace; ordinary text collapses it.
    /// - Text inside suppressed elements is ignored.
    fn append_text(&mut self, text: &str) -> Result<()> {
        if !self.suppressed_tags.is_empty() || text.is_empty() {
            return Ok(());
        }
        let decoded = decode_html_entities(text);
        if self.pre_depth > 0 {
            self.push_visible(&decoded);
            return self.ensure_output_bound();
        }
        for word in decoded.split_whitespace() {
            if self.needs_word_separator(word) {
                self.push_visible(" ");
            }
            self.push_visible(word);
        }
        self.ensure_output_bound()
    }

    /// What: Handle one HTML tag and update supported structural state.
    ///
    /// Inputs:
    /// - `raw_tag`: Tag content without surrounding `<` and `>`.
    /// - `base_url`: Valid URL used to resolve anchor destinations.
    ///
    /// Output:
    /// - Updates extraction state or reports a text-size violation.
    ///
    /// Details:
    /// - Unknown tags are ignored while their text remains visible.
    /// - Script-like elements are suppressed before their content is observed.
    fn handle_tag(&mut self, raw_tag: &str, base_url: &reqwest::Url) -> Result<()> {
        let Some((tag_name, attributes, closing, self_closing)) = parse_tag(raw_tag) else {
            return Ok(());
        };
        if self.handle_suppressed_tag(&tag_name, closing, self_closing) {
            return Ok(());
        }
        if closing {
            self.close_tag(&tag_name)?;
        } else {
            self.open_tag(&tag_name, attributes, base_url)?;
            if self_closing {
                self.close_tag(&tag_name)?;
            }
        }
        self.ensure_output_bound()
    }

    /// What: Suppress script-like tag content and consume matching close tags.
    ///
    /// Inputs:
    /// - `tag_name`: Normalized HTML tag name.
    /// - `closing`: Whether this is a closing tag.
    /// - `self_closing`: Whether this is a self-closing tag.
    ///
    /// Output:
    /// - `true` when the caller should not process the tag further.
    ///
    /// Details:
    /// - Nested suppressed tags are tracked by name, avoiding accidental exit
    ///   when malformed markup contains unrelated closing tags.
    fn handle_suppressed_tag(&mut self, tag_name: &str, closing: bool, self_closing: bool) -> bool {
        if let Some(open_tag) = self.suppressed_tags.last() {
            if closing && open_tag == tag_name {
                let _ = self.suppressed_tags.pop();
            }
            return true;
        }
        if !closing && !self_closing && is_suppressed_tag(tag_name) {
            self.suppressed_tags.push(tag_name.to_string());
            return true;
        }
        false
    }

    /// What: Process an opening supported HTML tag.
    ///
    /// Inputs:
    /// - `tag_name`: Normalized HTML tag name.
    /// - `attributes`: Raw tag attributes.
    /// - `base_url`: URL used to resolve an optional anchor destination.
    ///
    /// Output:
    /// - Updates visible structural output and parser stacks.
    ///
    /// Details:
    /// - Paragraph-like tags delimit blocks, list items gain a `- ` marker,
    ///   and code tags produce Markdown-safe code delimiters.
    fn open_tag(
        &mut self,
        tag_name: &str,
        attributes: &str,
        base_url: &reqwest::Url,
    ) -> Result<()> {
        match tag_name {
            "br" => self.ensure_line_breaks(1),
            "li" => {
                self.ensure_line_breaks(1);
                self.push_visible("- ");
            }
            "pre" => {
                self.ensure_line_breaks(2);
                self.push_visible("```\n");
                self.pre_depth += 1;
            }
            "code" if self.pre_depth == 0 => {
                if self.needs_word_separator("code") {
                    self.push_visible(" ");
                }
                self.push_visible("`");
                self.inline_code_depth += 1;
            }
            "a" => self.open_link(attributes, base_url),
            _ if is_block_tag(tag_name) => self.ensure_line_breaks(2),
            _ => {}
        }
        self.ensure_output_bound()
    }

    /// What: Process a closing supported HTML tag.
    ///
    /// Inputs:
    /// - `tag_name`: Normalized HTML tag name.
    ///
    /// Output:
    /// - Updates visible structural output and parser stacks.
    ///
    /// Details:
    /// - Unmatched close tags are harmless, making extraction resilient to
    ///   partially malformed article markup.
    fn close_tag(&mut self, tag_name: &str) -> Result<()> {
        match tag_name {
            "li" => self.ensure_line_breaks(1),
            "pre" if self.pre_depth > 0 => {
                self.pre_depth -= 1;
                if self.pre_depth == 0 {
                    self.ensure_line_breaks(1);
                    self.push_visible("```\n\n");
                }
            }
            "code" if self.pre_depth == 0 && self.inline_code_depth > 0 => {
                self.inline_code_depth -= 1;
                self.push_visible("`");
            }
            "a" => self.close_link(),
            _ if is_block_tag(tag_name) => self.ensure_line_breaks(2),
            _ => {}
        }
        self.ensure_output_bound()
    }

    /// What: Open an anchor while preserving its visible label text.
    ///
    /// Inputs:
    /// - `attributes`: Raw anchor attributes.
    /// - `base_url`: Valid URL used to resolve a relative `href`.
    ///
    /// Output:
    /// - Pushes an open-link state for a later closing anchor tag.
    ///
    /// Details:
    /// - Only HTTP(S) destinations are retained, and invalid destinations do
    ///   not cause label text to be dropped.
    fn open_link(&mut self, attributes: &str, base_url: &reqwest::Url) {
        let destination =
            attribute_value(attributes, "href").and_then(|href| resolve_http_link(base_url, &href));
        if destination.is_some() && self.needs_word_separator("link") {
            self.push_visible(" ");
        }
        let output_start = self.output.len();
        if destination.is_some() {
            self.push_visible("[");
        }
        self.open_links.push(OpenLink {
            output_start,
            destination,
        });
    }

    /// What: Close the most recent anchor and append a safe destination.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - A Markdown-safe `](url)` suffix, or unchanged visible label text.
    ///
    /// Details:
    /// - Empty links remove their opening bracket rather than emitting `[]()`.
    fn close_link(&mut self) {
        let Some(open_link) = self.open_links.pop() else {
            return;
        };
        let Some(destination) = open_link.destination else {
            return;
        };
        if self.output.len() == open_link.output_start + 1 {
            self.output.truncate(open_link.output_start);
            return;
        }
        self.output.push_str("](");
        self.output.push_str(&destination);
        self.output.push(')');
    }

    /// What: Append one visible text fragment, escaping anchor labels when needed.
    ///
    /// Inputs:
    /// - `text`: Already normalized visible text fragment.
    ///
    /// Output:
    /// - Appends directly to the extractor output buffer.
    ///
    /// Details:
    /// - Escaping `[`/`]`/`\\` inside live anchor labels prevents HTML text from
    ///   breaking the generated Markdown link structure.
    fn push_visible(&mut self, text: &str) {
        if self
            .open_links
            .last()
            .is_some_and(|link| link.destination.is_some())
        {
            for character in text.chars() {
                if matches!(character, '[' | ']' | '\\') {
                    self.output.push('\\');
                }
                self.output.push(character);
            }
        } else {
            self.output.push_str(text);
        }
    }

    /// What: Decide whether a normal text word needs a preceding space.
    ///
    /// Inputs:
    /// - `word`: Collapsed non-whitespace text token to append.
    ///
    /// Output:
    /// - `true` when a single separator should be emitted first.
    ///
    /// Details:
    /// - Punctuation following a link or word does not gain a spurious space.
    fn needs_word_separator(&self, word: &str) -> bool {
        let Some(previous) = self.output.chars().last() else {
            return false;
        };
        !previous.is_whitespace()
            && !matches!(previous, '[' | '`')
            && !word_starts_with_punctuation(word)
    }

    /// What: Ensure a requested number of trailing line breaks.
    ///
    /// Inputs:
    /// - `count`: Number of line breaks that should end the output.
    ///
    /// Output:
    /// - Appends only the missing line breaks.
    ///
    /// Details:
    /// - Whitespace before a block boundary is removed to keep output stable.
    fn ensure_line_breaks(&mut self, count: usize) {
        while self.output.ends_with([' ', '\t']) {
            let _ = self.output.pop();
        }
        let existing = self.output.chars().rev().take_while(|c| *c == '\n').count();
        for _ in existing..count {
            self.output.push('\n');
        }
    }

    /// What: Enforce the article text output bound.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - `Ok(())` within the bound or `InputTooLong` when exceeded.
    ///
    /// Details:
    /// - Checked incrementally so malformed markup cannot force an unbounded
    ///   intermediate output allocation.
    fn ensure_output_bound(&self) -> Result<()> {
        if self.output.len() <= MAX_ARTICLE_TEXT_BYTES {
            return Ok(());
        }
        Err(ArchToolkitError::InputTooLong {
            field: "article text".to_string(),
            max_length: MAX_ARTICLE_TEXT_BYTES,
            actual_length: self.output.len(),
        })
    }

    /// What: Finalize extracted text after the scanner reaches end of input.
    ///
    /// Inputs: None.
    ///
    /// Output:
    /// - Trimmed human-readable article text.
    ///
    /// Details:
    /// - Unterminated structures retain their already-visible label or code
    ///   content but cannot create executable HTML.
    fn finish(self) -> Result<String> {
        self.ensure_output_bound()?;
        Ok(self.output.trim().to_string())
    }
}

/// What: Parse a tag into name, attributes, closing state, and self-closing state.
///
/// Inputs:
/// - `raw_tag`: Tag text excluding `<` and `>` delimiters.
///
/// Output:
/// - Normalized tag metadata, or `None` for declarations and malformed tags.
///
/// Details:
/// - Tag names are restricted to ASCII HTML-style identifier characters to
///   keep downstream matching simple and deterministic.
fn parse_tag(raw_tag: &str) -> Option<(String, &str, bool, bool)> {
    let trimmed = raw_tag.trim();
    if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('?') {
        return None;
    }
    let closing = trimmed.starts_with('/');
    let without_marker = trimmed.trim_start_matches('/').trim_start();
    let name_end = without_marker
        .find(|character: char| !is_tag_name_character(character))
        .unwrap_or(without_marker.len());
    if name_end == 0 {
        return None;
    }
    let tag_name = without_marker[..name_end].to_ascii_lowercase();
    let attributes = &without_marker[name_end..];
    let self_closing = !closing && attributes.trim_end().ends_with('/');
    Some((tag_name, attributes, closing, self_closing))
}

/// What: Decide whether a character is valid in an HTML-style tag or attribute name.
///
/// Inputs:
/// - `character`: Candidate identifier character.
///
/// Output:
/// - `true` for ASCII letters, digits, `-`, `_`, or `:`.
///
/// Details:
/// - Restricting names to these characters avoids treating punctuation in
///   malformed markup as a supported tag or attribute.
const fn is_tag_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':')
}

/// What: Extract one case-insensitive attribute value from a raw tag suffix.
///
/// Inputs:
/// - `attributes`: Text after an opening tag name.
/// - `wanted`: Attribute name to find.
///
/// Output:
/// - Decoded attribute value when present, otherwise `None`.
///
/// Details:
/// - Handles single-quoted, double-quoted, and unquoted values without using
///   a browser parser. Boolean and malformed attributes are ignored.
fn attribute_value(attributes: &str, wanted: &str) -> Option<String> {
    let mut remaining = attributes.trim();
    while !remaining.is_empty() {
        let name_end = remaining
            .find(|character: char| !is_tag_name_character(character))
            .unwrap_or(remaining.len());
        if name_end == 0 {
            remaining = &remaining[1..];
            continue;
        }
        let name = &remaining[..name_end];
        remaining = remaining[name_end..].trim_start();
        let Some(after_equals) = remaining.strip_prefix('=') else {
            continue;
        };
        remaining = after_equals.trim_start();
        let (value, after_value) = split_attribute_value(remaining);
        remaining = after_value.trim_start();
        if name.eq_ignore_ascii_case(wanted) {
            return Some(decode_html_entities(value));
        }
    }
    None
}

/// What: Split the next HTML attribute value from trailing attributes.
///
/// Inputs:
/// - `input`: Text beginning at a quoted or unquoted attribute value.
///
/// Output:
/// - Tuple of value slice and unconsumed suffix.
///
/// Details:
/// - An unterminated quote consumes the rest of the tag as its value, which is
///   safer than scanning past a malformed delimiter.
fn split_attribute_value(input: &str) -> (&str, &str) {
    let Some(first) = input.chars().next() else {
        return ("", "");
    };
    if matches!(first, '\'' | '"') {
        let quoted = &input[first.len_utf8()..];
        if let Some(end) = quoted.find(first) {
            return (&quoted[..end], &quoted[end + first.len_utf8()..]);
        }
        return (quoted, "");
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    (&input[..end], &input[end..])
}

/// What: Resolve an anchor destination against an article URL and limit schemes.
///
/// Inputs:
/// - `base_url`: Valid article HTTP(S) URL.
/// - `href`: Decoded raw anchor destination.
///
/// Output:
/// - Escaped absolute HTTP(S) URL, or `None` when invalid/unsupported.
///
/// Details:
/// - Bounds raw link length before URL parsing and rejects schemes such as
///   `javascript:`, `data:`, and `mailto:`.
fn resolve_http_link(base_url: &reqwest::Url, href: &str) -> Option<String> {
    if href.is_empty() || href.len() > MAX_LINK_BYTES {
        return None;
    }
    let resolved = base_url.join(href).ok()?;
    if !matches!(resolved.scheme(), "http" | "https") {
        return None;
    }
    Some(escape_markdown_destination(resolved.as_str()))
}

/// What: Escape a URL for use inside a Markdown link destination.
///
/// Inputs:
/// - `url`: Valid absolute HTTP(S) URL.
///
/// Output:
/// - URL with Markdown delimiter characters escaped.
///
/// Details:
/// - Prevents a URL containing parentheses or backslashes from injecting text
///   outside the generated link destination.
fn escape_markdown_destination(url: &str) -> String {
    url.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

/// What: Identify tags whose contents are not article text.
///
/// Inputs:
/// - `tag_name`: Normalized tag name.
///
/// Output:
/// - `true` for tags that should suppress all nested text.
///
/// Details:
/// - These tags commonly contain executable, styling, fallback, or templating
///   content rather than readable article prose.
fn is_suppressed_tag(tag_name: &str) -> bool {
    matches!(tag_name, "script" | "style" | "template" | "noscript")
}

/// What: Identify HTML tags that form visible text block boundaries.
///
/// Inputs:
/// - `tag_name`: Normalized tag name.
///
/// Output:
/// - `true` for supported paragraph-like block elements.
///
/// Details:
/// - Lists and preformatted blocks have dedicated formatting rules elsewhere.
fn is_block_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "p" | "article"
            | "section"
            | "div"
            | "header"
            | "footer"
            | "main"
            | "aside"
            | "blockquote"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
    )
}

/// What: Identify tokens that should attach to preceding visible text.
///
/// Inputs:
/// - `word`: Collapsed text token being appended.
///
/// Output:
/// - `true` when its first character is closing punctuation.
///
/// Details:
/// - Prevents `word .` and `](url) .` output from ordinary HTML text.
fn word_starts_with_punctuation(word: &str) -> bool {
    word.starts_with(['.', ',', ';', ':', '!', '?', ')', ']', '}'])
}

/// What: Decode common named and numeric HTML entities in text or attributes.
///
/// Inputs:
/// - `input`: Raw text containing possible `&name;` or `&#...;` entities.
///
/// Output:
/// - Decoded text while preserving unknown or malformed entities literally.
///
/// Details:
/// - Entity decoding happens before whitespace handling and link resolution so
///   visible text and relative query strings are represented correctly.
fn decode_html_entities(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some(start) = remaining.find('&') {
        output.push_str(&remaining[..start]);
        let after_ampersand = &remaining[start + 1..];
        let Some(end) = after_ampersand.find(';') else {
            output.push('&');
            output.push_str(after_ampersand);
            return output;
        };
        let entity = &after_ampersand[..end];
        if let Some(decoded) = decode_entity(entity) {
            output.push(decoded);
        } else {
            output.push('&');
            output.push_str(entity);
            output.push(';');
        }
        remaining = &after_ampersand[end + 1..];
    }
    output.push_str(remaining);
    output
}

/// What: Decode one named or numeric HTML entity.
///
/// Inputs:
/// - `entity`: Entity name without leading `&` or trailing `;`.
///
/// Output:
/// - Decoded character when recognized, otherwise `None`.
///
/// Details:
/// - Supports the standard entities used by Arch feeds and numeric Unicode
///   code points without adding an HTML parser dependency.
fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        _ => decode_numeric_entity(entity),
    }
}

/// What: Decode a numeric decimal or hexadecimal HTML entity.
///
/// Inputs:
/// - `entity`: Numeric entity body without delimiters.
///
/// Output:
/// - Unicode character when the code point is valid, otherwise `None`.
///
/// Details:
/// - Invalid numeric values remain literal in the caller's output.
fn decode_numeric_entity(entity: &str) -> Option<char> {
    let hexadecimal = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"));
    if let Some(value) = hexadecimal {
        return u32::from_str_radix(value, 16).ok().and_then(char::from_u32);
    }
    entity
        .strip_prefix('#')
        .and_then(|value| value.parse::<u32>().ok())
        .and_then(char::from_u32)
}

#[cfg(test)]
mod tests {
    use super::{MAX_ARTICLE_HTML_BYTES, extract_article_text};

    #[test]
    /// What: Verify supported article structures become safe readable text.
    ///
    /// Inputs:
    /// - HTML with paragraphs, lists, inline/preformatted code, relative links,
    ///   entities, and script content.
    ///
    /// Output:
    /// - Text preserves visible structures, resolves the relative link, and
    ///   discards executable script content.
    ///
    /// Details:
    /// - This is the core fixture proof that the extractor does not execute or
    ///   emit raw HTML while retaining the requested article structures.
    fn extracts_supported_article_content_safely() {
        let html = r#"<article><p>Read <a href="/guide?one=1&amp;two=2">the [guide]</a>.</p>
<ul><li>First item</li><li>Use <code>--needed</code></li></ul>
<pre><code>pacman -Syu
</code></pre><script>alert('ignored')</script></article>"#;
        let text = extract_article_text(html, "https://archlinux.org/news/update/")
            .expect("extract article text");

        assert!(
            text.contains("Read [the \\[guide\\]](https://archlinux.org/guide?one=1&two=2)."),
            "actual extracted text: {text:?}"
        );
        assert!(text.contains("- First item\n- Use `--needed`"));
        assert!(text.contains("```\npacman -Syu\n```"));
        assert!(!text.contains("alert"));
        assert!(!text.contains("<script"));
    }

    #[test]
    /// What: Verify unsafe destinations retain labels without generated links.
    ///
    /// Inputs:
    /// - Article HTML containing a JavaScript URL and a malformed relative URL.
    ///
    /// Output:
    /// - Visible labels remain while unsupported destinations are omitted.
    ///
    /// Details:
    /// - Prevents article HTML from turning non-web schemes into active output.
    fn rejects_unsafe_link_schemes() {
        let html =
            r#"<p><a href="javascript:alert(1)">unsafe</a> and <a href="mailto:x@y">mail</a></p>"#;
        let text = extract_article_text(html, "https://archlinux.org/news/update/")
            .expect("extract article text");

        assert_eq!(text, "unsafe and mail");
    }

    #[test]
    /// What: Verify the HTML input safety bound is enforced before parsing.
    ///
    /// Inputs:
    /// - One byte more than the documented article HTML limit.
    ///
    /// Output:
    /// - An explicit size error.
    ///
    /// Details:
    /// - Avoids unbounded work for hostile or unexpectedly large pages.
    fn rejects_oversized_article_html() {
        let html = "x".repeat(MAX_ARTICLE_HTML_BYTES + 1);
        assert!(extract_article_text(&html, "https://archlinux.org/news/update/").is_err());
    }
}
