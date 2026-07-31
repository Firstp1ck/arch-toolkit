//! Bounded caller-client mirror discovery and deterministic mirrorlist generation.

use std::fmt::Write;

use crate::error::{ArchToolkitError, Result};
use crate::types::index::{MirrorDiscoveryLimits, MirrorInfo};

/// Official Arch mirror-status endpoint used only by the opt-in convenience API.
pub const ARCH_MIRROR_STATUS_URL: &str = "https://archlinux.org/mirrors/status/json/";
/// Maximum bytes emitted by one generated mirrorlist.
pub const MAX_MIRRORLIST_BYTES: usize = 512 * 1024;
/// Maximum accepted mirror base URL length before storing or generating it.
const MAX_MIRROR_URL_BYTES: usize = 4 * 1024;

/// What: Fetch Arch's standard mirror-status endpoint with caller-owned transport policy.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client controlling timeout, proxy, TLS,
///   redirect, and user-agent policy.
/// - `limits`: Explicit response and result bounds for this request.
///
/// Output:
/// - Valid mirror rows accepted from the standard Arch mirror-status schema.
///
/// Details:
/// - Delegates to [`fetch_mirrors_from`] and does not create its own client or
///   execute any system command.
/// - Callers needing another source can supply it directly with
///   [`fetch_mirrors_from`].
///
/// # Errors
///
/// Returns an error for invalid limits, failed requests, non-success statuses,
/// oversized or invalid JSON bodies, and malformed root schema.
pub async fn fetch_arch_mirrors(
    client: &reqwest::Client,
    limits: MirrorDiscoveryLimits,
) -> Result<Vec<MirrorInfo>> {
    fetch_mirrors_from(client, ARCH_MIRROR_STATUS_URL, limits).await
}

/// What: Discover portable mirror metadata from a caller-selected JSON endpoint.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client controlling transport policy.
/// - `status_url`: Absolute HTTP(S) endpoint returning an Arch-compatible
///   `{ "urls": [...] }` mirror-status document.
/// - `limits`: Explicit response and accepted-row bounds for this request.
///
/// Output:
/// - Deterministically URL-sorted valid [`MirrorInfo`] rows, capped at
///   `limits.max_mirrors`.
///
/// Details:
/// - The parser accepts only active/inactive boolean metadata, a base HTTP(S)
///   URL, and a bounded string protocol list. It does not rank mirrors or make
///   any claim about current latency/health.
/// - The endpoint is caller-selected, enabling derivative distributions, test
///   servers, and trusted application mirrors without shelling out to `curl`.
///
/// # Errors
///
/// Returns an error for invalid URLs/limits, request or response failures,
/// oversized bodies, invalid UTF-8/JSON, or a missing `urls` array.
pub async fn fetch_mirrors_from(
    client: &reqwest::Client,
    status_url: &str,
    limits: MirrorDiscoveryLimits,
) -> Result<Vec<MirrorInfo>> {
    validate_discovery_limits(limits)?;
    let body = fetch_bounded_mirror_status(client, status_url, limits.max_response_bytes).await?;
    parse_mirrors_from_json(&body, limits.max_mirrors)
}

/// What: Generate deterministic pacman mirrorlist text from discovered metadata.
///
/// Inputs:
/// - `mirrors`: Mirror rows from discovery or trusted caller-owned metadata.
/// - `maximum_mirrors`: Explicit cap on generated server lines.
///
/// Output:
/// - Pacman-compatible `Server = .../$repo/os/$arch` lines for active HTTPS
///   mirrors, sorted and deduplicated.
///
/// Details:
/// - Invalid, inactive, non-HTTPS, oversized, and duplicate base URLs are
///   excluded. No file is written and no command is executed.
/// - The generated output is independently bounded by
///   [`MAX_MIRRORLIST_BYTES`].
///
/// # Errors
///
/// Returns `InvalidInput` for a zero line limit or `InputTooLong` when valid
/// input would exceed the generated-output bound.
pub fn generate_mirrorlist(mirrors: &[MirrorInfo], maximum_mirrors: usize) -> Result<String> {
    if maximum_mirrors == 0 {
        return Err(ArchToolkitError::InvalidInput(
            "maximum mirrorlist entries must be greater than zero".to_string(),
        ));
    }
    let mut urls = collect_active_https_urls(mirrors);
    urls.truncate(maximum_mirrors);

    let mut output = String::from(
        "# Generated from caller-selected mirror status data.\n# Only active HTTPS mirrors are listed.\n",
    );
    for url in urls {
        append_mirror_server_line(&mut output, &url)?;
    }
    Ok(output)
}

/// What: Validate explicit mirror discovery bounds before making a request.
///
/// Inputs:
/// - `limits`: Candidate response and result limits.
///
/// Output:
/// - `Ok(())` when both limits are non-zero.
///
/// Details:
/// - Rejecting zero avoids silently treating a request as a successful empty
///   response or generating an accidental unbounded default.
fn validate_discovery_limits(limits: MirrorDiscoveryLimits) -> Result<()> {
    if limits.max_response_bytes == 0 || limits.max_mirrors == 0 {
        return Err(ArchToolkitError::InvalidInput(
            "mirror discovery response and row limits must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// What: Fetch a successful mirror-status body without exceeding its byte bound.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client.
/// - `status_url`: Absolute HTTP(S) mirror-status endpoint.
/// - `maximum_bytes`: Inclusive response-size bound.
///
/// Output:
/// - UTF-8 JSON text no larger than `maximum_bytes`.
///
/// Details:
/// - Checks Content-Length and streamed chunks so absent or misleading headers
///   cannot bypass the resource bound.
///
/// # Errors
///
/// Returns errors for invalid URLs, request/status/read failures, oversized
/// bodies, or invalid UTF-8.
async fn fetch_bounded_mirror_status(
    client: &reqwest::Client,
    status_url: &str,
    maximum_bytes: usize,
) -> Result<String> {
    let parsed_url = parse_http_url(status_url)?;
    let mut response = client.get(parsed_url).send().await.map_err(|error| {
        ArchToolkitError::Parse(format!("mirror status request failed: {error}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ArchToolkitError::Parse(format!(
            "mirror status returned status {status}"
        )));
    }

    let maximum_length = u64::try_from(maximum_bytes).map_err(|_| {
        ArchToolkitError::InvalidInput("mirror response bound is too large".to_string())
    })?;
    if response
        .content_length()
        .is_some_and(|length| length > maximum_length)
    {
        return Err(mirror_response_too_large(maximum_bytes));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        ArchToolkitError::Parse(format!("mirror status response read failed: {error}"))
    })? {
        if chunk.len() > maximum_bytes.saturating_sub(bytes.len()) {
            return Err(mirror_response_too_large(maximum_bytes));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|error| {
        ArchToolkitError::Parse(format!(
            "mirror status response was not valid UTF-8: {error}"
        ))
    })
}

/// What: Validate a caller-selected mirror status URL.
///
/// Inputs:
/// - `url`: Candidate mirror-status endpoint.
///
/// Output:
/// - Parsed HTTP(S) URL ready for a caller-client request.
///
/// Details:
/// - Rejects other schemes without making a network request.
fn parse_http_url(url: &str) -> Result<reqwest::Url> {
    let parsed = reqwest::Url::parse(url).map_err(|error| {
        ArchToolkitError::InvalidInput(format!("invalid mirror status URL: {error}"))
    })?;
    if matches!(parsed.scheme(), "http" | "https") {
        return Ok(parsed);
    }
    Err(ArchToolkitError::InvalidInput(
        "mirror status URL must use http or https".to_string(),
    ))
}

/// What: Build a consistent response-bound error for mirror discovery.
///
/// Inputs:
/// - `maximum_bytes`: Configured maximum body size.
///
/// Output:
/// - `InputTooLong` with the known mirror response bound.
///
/// Details:
/// - The exact received size can be incomplete when a streamed body is rejected.
fn mirror_response_too_large(maximum_bytes: usize) -> ArchToolkitError {
    ArchToolkitError::InputTooLong {
        field: "mirror status response".to_string(),
        max_length: maximum_bytes,
        actual_length: maximum_bytes.saturating_add(1),
    }
}

/// What: Parse an Arch-compatible mirror-status JSON document into bounded rows.
///
/// Inputs:
/// - `body`: Successful bounded UTF-8 JSON response body.
/// - `maximum_mirrors`: Maximum valid rows to return.
///
/// Output:
/// - URL-sorted valid mirror rows capped at `maximum_mirrors`.
///
/// Details:
/// - Rows lacking a valid base HTTP(S) URL are skipped rather than causing one
///   remote record to make all discovery unusable.
fn parse_mirrors_from_json(body: &str, maximum_mirrors: usize) -> Result<Vec<MirrorInfo>> {
    let document: serde_json::Value = serde_json::from_str(body)?;
    let rows = document
        .get("urls")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ArchToolkitError::Parse("mirror status response is missing a 'urls' array".to_string())
        })?;

    let mut mirrors = rows
        .iter()
        .filter_map(parse_mirror_row)
        .collect::<Vec<MirrorInfo>>();
    mirrors.sort_by(|left, right| left.url.cmp(&right.url));
    mirrors.dedup_by(|left, right| left.url == right.url);
    mirrors.truncate(maximum_mirrors);
    Ok(mirrors)
}

/// What: Parse one candidate JSON row into safe bounded mirror metadata.
///
/// Inputs:
/// - `row`: One value from the mirror-status `urls` array.
///
/// Output:
/// - Valid normalized mirror metadata, or `None` for an unusable row.
///
/// Details:
/// - The mirror URL must be absolute HTTP(S), base-URL length is bounded, and
///   protocol names are bounded before copying into public output.
fn parse_mirror_row(row: &serde_json::Value) -> Option<MirrorInfo> {
    let raw_url = row.get("url")?.as_str()?.trim();
    if raw_url.is_empty() || raw_url.len() > MAX_MIRROR_URL_BYTES {
        return None;
    }
    let parsed_url = reqwest::Url::parse(raw_url).ok()?;
    if !matches!(parsed_url.scheme(), "http" | "https") {
        return None;
    }
    let protocols = row
        .get("protocols")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|protocol| !protocol.is_empty() && protocol.len() <= 32)
        .take(16)
        .map(ToString::to_string)
        .collect();
    Some(MirrorInfo {
        url: raw_url.trim_end_matches('/').to_string(),
        active: row
            .get("active")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        protocols,
    })
}

/// What: Collect active HTTPS mirror base URLs in deterministic lexical order.
///
/// Inputs:
/// - `mirrors`: Candidate mirror metadata.
///
/// Output:
/// - Sorted, deduplicated, validated base URLs suitable for server-line output.
///
/// Details:
/// - The protocol list must include HTTPS even when the URL itself is HTTPS, so
///   callers retain the mirror-status source's advertised transport contract.
fn collect_active_https_urls(mirrors: &[MirrorInfo]) -> Vec<String> {
    let mut urls = mirrors
        .iter()
        .filter(|mirror| mirror.active && supports_https(mirror))
        .filter_map(valid_mirror_base_url)
        .collect::<Vec<String>>();
    urls.sort();
    urls.dedup();
    urls
}

/// What: Confirm a mirror-status row advertises HTTPS support.
///
/// Inputs:
/// - `mirror`: Candidate mirror metadata.
///
/// Output:
/// - `true` when one advertised protocol equals `https` case-insensitively.
///
/// Details:
/// - Avoids emitting HTTP-only mirrors even if a malformed row contains an
///   HTTPS-looking URL string.
fn supports_https(mirror: &MirrorInfo) -> bool {
    mirror
        .protocols
        .iter()
        .any(|protocol| protocol.eq_ignore_ascii_case("https"))
}

/// What: Validate a mirror base URL before generating a pacman server line.
///
/// Inputs:
/// - `mirror`: Candidate active HTTPS mirror metadata.
///
/// Output:
/// - Normalized base URL, or `None` when invalid/oversized/non-HTTPS.
///
/// Details:
/// - Revalidating caller-built `MirrorInfo` makes generation safe independently
///   of whether values came from this module's discovery parser.
fn valid_mirror_base_url(mirror: &MirrorInfo) -> Option<String> {
    let url = mirror.url.trim_end_matches('/');
    if url.is_empty() || url.len() > MAX_MIRROR_URL_BYTES {
        return None;
    }
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    Some(url.to_string())
}

/// What: Append one bounded pacman `Server` line to a generated mirrorlist.
///
/// Inputs:
/// - `output`: Mirrorlist buffer to extend.
/// - `base_url`: Prevalidated HTTPS mirror base URL.
///
/// Output:
/// - `Ok(())` after a server line is appended, or a size error.
///
/// Details:
/// - Uses pacman's standard variable placeholders without expanding them or
///   writing to any system mirrorlist file.
fn append_mirror_server_line(output: &mut String, base_url: &str) -> Result<()> {
    let line = format!("Server = {base_url}/$repo/os/$arch\n");
    if line.len() > MAX_MIRRORLIST_BYTES.saturating_sub(output.len()) {
        return Err(ArchToolkitError::InputTooLong {
            field: "generated mirrorlist".to_string(),
            max_length: MAX_MIRRORLIST_BYTES,
            actual_length: MAX_MIRRORLIST_BYTES.saturating_add(1),
        });
    }
    output
        .write_str(&line)
        .map_err(|_| ArchToolkitError::Parse("failed to build mirrorlist text".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{MirrorDiscoveryLimits, MirrorInfo, generate_mirrorlist, parse_mirrors_from_json};

    #[test]
    /// What: Verify mirror JSON parsing filters invalid rows and sorts valid URLs.
    ///
    /// Inputs:
    /// - A fixture-shaped response with valid, invalid, and duplicate rows.
    ///
    /// Output:
    /// - Bounded unique valid mirror metadata in lexical URL order.
    ///
    /// Details:
    /// - Proves parser behavior without a live Arch endpoint.
    fn parses_bounded_mirror_fixture() {
        let body = r#"{"urls":[
            {"url":"https://z.example/","active":true,"protocols":["https"]},
            {"url":"javascript:bad","active":true,"protocols":["https"]},
            {"url":"https://a.example/","active":false,"protocols":["https","rsync"]},
            {"url":"https://z.example/","active":true,"protocols":["https"]}
        ]}"#;
        let mirrors = parse_mirrors_from_json(body, 10).expect("parse fixture");

        assert_eq!(mirrors.len(), 2);
        assert_eq!(mirrors[0].url, "https://a.example");
        assert_eq!(mirrors[1].url, "https://z.example");
    }

    #[test]
    /// What: Verify mirrorlist generation emits only bounded active HTTPS rows.
    ///
    /// Inputs:
    /// - Active/inactive HTTP/HTTPS fixture metadata with a duplicate URL.
    ///
    /// Output:
    /// - One deterministic pacman server line for the active HTTPS mirror.
    ///
    /// Details:
    /// - No file write occurs; callers own applying the generated text.
    fn generates_deterministic_https_mirrorlist() {
        let mirrors = vec![
            MirrorInfo {
                url: "https://fast.example/".to_string(),
                active: true,
                protocols: vec!["https".to_string()],
            },
            MirrorInfo {
                url: "http://insecure.example/".to_string(),
                active: true,
                protocols: vec!["http".to_string()],
            },
            MirrorInfo {
                url: "https://inactive.example/".to_string(),
                active: false,
                protocols: vec!["https".to_string()],
            },
        ];
        let mirrorlist = generate_mirrorlist(&mirrors, 4).expect("generate mirrorlist");

        assert!(mirrorlist.contains("Server = https://fast.example/$repo/os/$arch"));
        assert!(!mirrorlist.contains("insecure.example"));
        assert!(!mirrorlist.contains("inactive.example"));
    }

    #[test]
    /// What: Verify default discovery limits remain explicitly non-zero.
    ///
    /// Inputs:
    /// - Default [`MirrorDiscoveryLimits`].
    ///
    /// Output:
    /// - Positive response and row limits.
    ///
    /// Details:
    /// - Guards the resource-bound invariant for convenience callers.
    fn default_limits_are_bounded() {
        let limits = MirrorDiscoveryLimits::default();
        assert!(limits.max_response_bytes > 0);
        assert!(limits.max_mirrors > 0);
    }
}
