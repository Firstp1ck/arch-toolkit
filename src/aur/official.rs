//! Bounded caller-client official package metadata and mirror-health requests.
//!
//! The official package detail helper reuses the established Arch Packages JSON
//! search endpoint and the public index package selector model. Mirror health
//! checks accept existing `MirrorInfo` rows plus a caller-selected relative probe
//! path. Neither helper creates a reqwest client, changes configuration, invokes
//! a shell command, ranks mirrors, or applies a mirrorlist.

use crate::error::{ArchToolkitError, Result};
use crate::types::index::{MirrorInfo, OfficialPackage};
use crate::types::package::{
    MetadataFetchLimits, MirrorHealth, MirrorHealthLimits, MirrorHealthStatus,
};

/// Established Arch Packages JSON search endpoint used by the opt-in convenience helper.
pub const ARCH_PACKAGE_SEARCH_URL: &str = "https://archlinux.org/packages/search/json/";
/// Maximum source URL bytes retained in one mirror-health result.
const MAX_MIRROR_URL_BYTES: usize = 4 * 1024;
/// Maximum bytes retained in one mirror probe failure detail.
const MAX_PROBE_DETAIL_BYTES: usize = 240;

/// What: Fetch one official Arch package detail with caller-owned transport policy.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client controlling timeout, proxy, TLS,
///   redirect, and user-agent behavior.
/// - `package`: Existing public index model selecting a package name and,
///   optionally, repository and architecture.
/// - `limits`: Explicit response and candidate bounds for this request.
///
/// Output:
/// - An enriched `OfficialPackage` when the endpoint has an exact matching row,
///   or `None` when no exact package row is present.
///
/// Details:
/// - Uses [`ARCH_PACKAGE_SEARCH_URL`], which is already the repository's
///   official-index fallback endpoint. For deterministic tests, derivative
///   distributions, or another trusted endpoint, use
///   [`fetch_official_package_detail_from`].
/// - It performs one bounded HTTP request and never invokes `pacman` or a shell.
///
/// # Errors
///
/// Returns an error for invalid selector/limits, transport/status/read failures,
/// oversized responses, or malformed JSON response roots.
pub async fn fetch_arch_package_detail(
    client: &reqwest::Client,
    package: &OfficialPackage,
    limits: MetadataFetchLimits,
) -> Result<Option<OfficialPackage>> {
    fetch_official_package_detail_from(client, ARCH_PACKAGE_SEARCH_URL, package, limits).await
}

/// What: Fetch one official package detail from a caller-selected JSON endpoint.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client controlling all transport policy.
/// - `endpoint`: Absolute HTTP(S) Arch Packages-compatible search endpoint.
/// - `package`: Existing public index model selecting package name/repo/arch.
/// - `limits`: Explicit response and candidate bounds for this request.
///
/// Output:
/// - An exact `OfficialPackage` match enriched from one bounded response, or
///   `None` when no matching row exists.
///
/// Details:
/// - Adds `name`, and non-empty `repo` / `arch`, query parameters to the
///   caller-selected endpoint.
/// - Response parsing is restricted to the existing `OfficialPackage` fields;
///   it does not introduce a parallel official-package model or pagination API.
/// - The supplied client makes this fixture-friendly and keeps retry/timeouts
///   under caller ownership.
///
/// # Errors
///
/// Returns an error for invalid input, endpoint, response, or JSON root.
pub async fn fetch_official_package_detail_from(
    client: &reqwest::Client,
    endpoint: &str,
    package: &OfficialPackage,
    limits: MetadataFetchLimits,
) -> Result<Option<OfficialPackage>> {
    validate_metadata_request(package, limits)?;
    let request_url = detail_request_url(endpoint, package)?;
    let body = fetch_bounded_json(
        client,
        request_url,
        limits.max_response_bytes,
        "package detail",
    )
    .await?;
    parse_official_package_detail(&body, package, limits.max_candidates)
}

/// What: Probe existing public mirror rows with a caller-selected relative path.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client controlling all transport policy.
/// - `mirrors`: Existing public index mirror metadata in caller-selected order.
/// - `probe_path`: Absolute-path component appended to each mirror base URL.
/// - `limits`: Explicit maximum number of sequential probes.
///
/// Output:
/// - One ordered `MirrorHealth` record per selected input row, with response
///   status evidence or a bounded validation/transport detail.
///
/// Details:
/// - Only the first `limits.max_mirrors` rows are checked, sequentially.
/// - A 2xx final HTTP status is `Reachable`; all other statuses and transport
///   errors are `Unreachable`; malformed source URLs are `Invalid`.
/// - No fixed health endpoint is assumed: callers supply a safe relative probe
///   path suitable for their repositories and mirrors.
///
/// # Errors
///
/// Returns an error only for an invalid global probe bound or probe path. Per
/// mirror failures are returned as structured health evidence.
pub async fn check_mirror_health(
    client: &reqwest::Client,
    mirrors: &[MirrorInfo],
    probe_path: &str,
    limits: MirrorHealthLimits,
) -> Result<Vec<MirrorHealth>> {
    validate_probe_request(probe_path, limits)?;
    let mut health = Vec::with_capacity(mirrors.len().min(limits.max_mirrors));
    for mirror in mirrors.iter().take(limits.max_mirrors) {
        health.push(probe_one_mirror(client, mirror, probe_path).await);
    }
    Ok(health)
}

/// What: Validate a bounded official package-detail request before I/O.
///
/// Inputs:
/// - `package`: Existing index package selector.
/// - `limits`: Candidate response and parse bounds.
///
/// Output:
/// - `Ok(())` when package name and all bounds are usable.
///
/// Details:
/// - Rejecting zero bounds avoids silently converting a request into a success
///   with no parsed data.
fn validate_metadata_request(package: &OfficialPackage, limits: MetadataFetchLimits) -> Result<()> {
    if package.name.trim().is_empty() {
        return Err(ArchToolkitError::EmptyInput {
            field: "official package name".to_string(),
            message: "an official package detail request needs a package name".to_string(),
        });
    }
    if limits.max_response_bytes == 0 || limits.max_candidates == 0 {
        return Err(ArchToolkitError::InvalidInput(
            "official package response and candidate limits must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// What: Build a validated official package-detail request URL.
///
/// Inputs:
/// - `endpoint`: Caller-selected absolute HTTP(S) search endpoint.
/// - `package`: Existing public selector model.
///
/// Output:
/// - URL with encoded name/repository/architecture query parameters.
///
/// Details:
/// - Query construction uses `reqwest::Url` rather than string concatenation so
///   package selector values cannot change the endpoint path or authority.
fn detail_request_url(endpoint: &str, package: &OfficialPackage) -> Result<reqwest::Url> {
    let mut url = parse_http_url(endpoint, "official package endpoint")?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("name", &package.name);
        if !package.repo.is_empty() {
            query.append_pair("repo", &package.repo);
        }
        if !package.arch.is_empty() {
            query.append_pair("arch", &package.arch);
        }
    }
    Ok(url)
}

/// What: Fetch a successful JSON body without exceeding an explicit byte bound.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client.
/// - `url`: Prevalidated HTTP(S) endpoint.
/// - `maximum_bytes`: Inclusive response-size bound.
/// - `resource_name`: Error-context label for the requested resource.
///
/// Output:
/// - UTF-8 response text no larger than `maximum_bytes`.
///
/// Details:
/// - Checks both `Content-Length` and streamed chunks, so absent or misleading
///   response headers cannot bypass the resource bound.
async fn fetch_bounded_json(
    client: &reqwest::Client,
    url: reqwest::Url,
    maximum_bytes: usize,
    resource_name: &str,
) -> Result<String> {
    let mut response = client.get(url).send().await.map_err(|error| {
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
        return Err(response_too_large(resource_name, maximum_bytes));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        ArchToolkitError::Parse(format!("{resource_name} response read failed: {error}"))
    })? {
        if chunk.len() > maximum_bytes.saturating_sub(bytes.len()) {
            return Err(response_too_large(resource_name, maximum_bytes));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|error| {
        ArchToolkitError::Parse(format!(
            "{resource_name} response was not valid UTF-8: {error}"
        ))
    })
}

/// What: Parse a bounded official package search response into an exact selector match.
///
/// Inputs:
/// - `body`: Bounded UTF-8 JSON response text.
/// - `selector`: Existing index model that identifies the desired package.
/// - `maximum_candidates`: Maximum rows considered for an exact match.
///
/// Output:
/// - Enriched matching `OfficialPackage`, or `None` when no exact row appears.
///
/// Details:
/// - A malformed non-matching row is skipped. Parsing does not create a new
///   official data model or infer metadata from a partial name match.
fn parse_official_package_detail(
    body: &str,
    selector: &OfficialPackage,
    maximum_candidates: usize,
) -> Result<Option<OfficialPackage>> {
    let document: serde_json::Value = serde_json::from_str(body)?;
    let results = document
        .get("results")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ArchToolkitError::Parse(
                "official package detail response is missing a 'results' array".to_string(),
            )
        })?;

    Ok(results
        .iter()
        .take(maximum_candidates)
        .filter_map(parse_official_package)
        .find(|candidate| exact_official_match(candidate, selector)))
}

/// What: Convert one API JSON row into the existing official package model.
///
/// Inputs:
/// - `row`: Candidate JSON object from a bounded response.
///
/// Output:
/// - Populated `OfficialPackage`, or `None` if its package name is unavailable.
///
/// Details:
/// - Missing non-name fields become empty strings, matching existing index
///   model conventions and allowing callers to distinguish absent enrichment.
fn parse_official_package(row: &serde_json::Value) -> Option<OfficialPackage> {
    let name = row.get("pkgname")?.as_str()?.to_string();
    let version = row
        .get("pkgver")
        .and_then(serde_json::Value::as_str)
        .map_or_else(String::new, |pkgver| {
            row.get("pkgrel")
                .and_then(serde_json::Value::as_str)
                .filter(|pkgrel| !pkgrel.is_empty())
                .map_or_else(|| pkgver.to_string(), |pkgrel| format!("{pkgver}-{pkgrel}"))
        });
    Some(OfficialPackage {
        name,
        repo: string_field(row, "repo"),
        arch: string_field(row, "arch"),
        version,
        description: string_field(row, "pkgdesc"),
    })
}

/// What: Read an optional JSON string field as an owned string.
///
/// Inputs:
/// - `row`: Candidate JSON object.
/// - `field`: JSON key to retrieve.
///
/// Output:
/// - Field value or an empty string when absent/non-string.
///
/// Details:
/// - Matches the existing `OfficialPackage` convention for optional metadata.
fn string_field(row: &serde_json::Value, field: &str) -> String {
    row.get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// What: Verify an API package row exactly matches the caller's index selector.
///
/// Inputs:
/// - `candidate`: Parsed API row.
/// - `selector`: Caller-selected name and optional repo/architecture filters.
///
/// Output:
/// - `true` only for an exact name and every non-empty selector field.
///
/// Details:
/// - Empty repo or architecture selector fields intentionally mean "do not
///   constrain this field", which supports enrichment of pacman `-Sl` rows.
fn exact_official_match(candidate: &OfficialPackage, selector: &OfficialPackage) -> bool {
    candidate.name == selector.name
        && (selector.repo.is_empty() || candidate.repo == selector.repo)
        && (selector.arch.is_empty() || candidate.arch == selector.arch)
}

/// What: Validate global mirror probe bounds and relative path semantics.
///
/// Inputs:
/// - `probe_path`: Relative-to-mirror absolute path component.
/// - `limits`: Maximum selected mirrors.
///
/// Output:
/// - `Ok(())` when the request cannot change mirror authorities or traverse up.
///
/// Details:
/// - Requiring a leading slash and rejecting `..`, query, and fragment syntax
///   keeps caller-selected probes scoped beneath each source mirror base URL.
fn validate_probe_request(probe_path: &str, limits: MirrorHealthLimits) -> Result<()> {
    if limits.max_mirrors == 0 {
        return Err(ArchToolkitError::InvalidInput(
            "mirror health maximum probes must be greater than zero".to_string(),
        ));
    }
    let has_parent = probe_path.split('/').any(|segment| segment == "..");
    if !probe_path.starts_with('/')
        || probe_path.contains('?')
        || probe_path.contains('#')
        || has_parent
    {
        return Err(ArchToolkitError::InvalidInput(
            "mirror health probe path must be an absolute path without query, fragment, or '..'"
                .to_string(),
        ));
    }
    Ok(())
}

/// What: Probe one source mirror and return structured per-mirror evidence.
///
/// Inputs:
/// - `client`: Caller-provided reqwest client.
/// - `mirror`: Existing public mirror model to probe.
/// - `probe_path`: Already-validated relative path to append.
///
/// Output:
/// - A reachable, unreachable, or invalid `MirrorHealth` record.
///
/// Details:
/// - The response body is not read, so the check is bounded by one request and
///   the caller's client timeout; no configuration is written or command run.
async fn probe_one_mirror(
    client: &reqwest::Client,
    mirror: &MirrorInfo,
    probe_path: &str,
) -> MirrorHealth {
    let mirror_url = bounded_detail(&mirror.url, MAX_MIRROR_URL_BYTES);
    let probe_url = match mirror_probe_url(&mirror.url, probe_path) {
        Ok(url) => url,
        Err(error) => return invalid_mirror_health(mirror_url, &error),
    };

    match client.get(probe_url).send().await {
        Ok(response) if response.status().is_success() => MirrorHealth {
            mirror_url,
            status: MirrorHealthStatus::Reachable,
            status_code: Some(response.status().as_u16()),
            detail: None,
        },
        Ok(response) => MirrorHealth {
            mirror_url,
            status: MirrorHealthStatus::Unreachable,
            status_code: Some(response.status().as_u16()),
            detail: Some(format!("probe returned HTTP {}", response.status())),
        },
        Err(error) => MirrorHealth {
            mirror_url,
            status: MirrorHealthStatus::Unreachable,
            status_code: None,
            detail: Some(bounded_detail(&error.to_string(), MAX_PROBE_DETAIL_BYTES)),
        },
    }
}

/// What: Build a safe probe URL under one existing mirror base URL.
///
/// Inputs:
/// - `mirror_url`: Source mirror base URL.
/// - `probe_path`: Validated absolute path component to append.
///
/// Output:
/// - HTTP(S) probe URL scoped under the mirror base path.
///
/// Details:
/// - Clears source query/fragment metadata and ensures the base is directory
///   shaped before URL joining, so caller probe paths cannot replace authority.
fn mirror_probe_url(mirror_url: &str, probe_path: &str) -> Result<reqwest::Url> {
    if mirror_url.len() > MAX_MIRROR_URL_BYTES {
        return Err(ArchToolkitError::InputTooLong {
            field: "mirror URL".to_string(),
            max_length: MAX_MIRROR_URL_BYTES,
            actual_length: mirror_url.len(),
        });
    }
    let mut base = parse_http_url(mirror_url, "mirror URL")?;
    base.set_query(None);
    base.set_fragment(None);
    if !base.path().ends_with('/') {
        let directory_path = format!("{}/", base.path());
        base.set_path(&directory_path);
    }
    base.join(probe_path.trim_start_matches('/'))
        .map_err(|error| {
            ArchToolkitError::InvalidInput(format!("invalid mirror health probe URL: {error}"))
        })
}

/// What: Validate an absolute HTTP(S) URL used by a caller-client helper.
///
/// Inputs:
/// - `input`: Candidate URL string.
/// - `field`: Human-readable input label for error context.
///
/// Output:
/// - Parsed HTTP(S) URL ready for a reqwest request.
///
/// Details:
/// - Other schemes are rejected before any request is made.
fn parse_http_url(input: &str, field: &str) -> Result<reqwest::Url> {
    let parsed = reqwest::Url::parse(input)
        .map_err(|error| ArchToolkitError::InvalidInput(format!("invalid {field}: {error}")))?;
    if matches!(parsed.scheme(), "http" | "https") {
        return Ok(parsed);
    }
    Err(ArchToolkitError::InvalidInput(format!(
        "{field} must use http or https"
    )))
}

/// What: Build a consistent oversized-response error.
///
/// Inputs:
/// - `resource_name`: Error-context label.
/// - `maximum_bytes`: Configured response size bound.
///
/// Output:
/// - `InputTooLong` with a saturating actual-size sentinel.
///
/// Details:
/// - Streamed responses can be rejected before their total size is known.
fn response_too_large(resource_name: &str, maximum_bytes: usize) -> ArchToolkitError {
    ArchToolkitError::InputTooLong {
        field: format!("{resource_name} response"),
        max_length: maximum_bytes,
        actual_length: maximum_bytes.saturating_add(1),
    }
}

/// What: Produce a structured invalid-mirror record from a validation error.
///
/// Inputs:
/// - `mirror_url`: Bounded source URL retained as evidence.
/// - `error`: Validation failure from URL construction.
///
/// Output:
/// - `MirrorHealthStatus::Invalid` with no HTTP status code.
///
/// Details:
/// - Per-mirror invalid data does not abort checks for the remaining bounded
///   input rows.
fn invalid_mirror_health(mirror_url: String, error: &ArchToolkitError) -> MirrorHealth {
    MirrorHealth {
        mirror_url,
        status: MirrorHealthStatus::Invalid,
        status_code: None,
        detail: Some(bounded_detail(&error.to_string(), MAX_PROBE_DETAIL_BYTES)),
    }
}

/// What: Limit a detail string to a fixed byte-compatible character count.
///
/// Inputs:
/// - `value`: Arbitrary source URL or error detail.
/// - `maximum_chars`: Maximum Unicode scalar values retained.
///
/// Output:
/// - Original value when within the bound, otherwise a truncated value with an ellipsis.
///
/// Details:
/// - This avoids preserving an unbounded remote error or source string in a
///   structured health record.
fn bounded_detail(value: &str, maximum_chars: usize) -> String {
    let mut characters = value.chars();
    let detail: String = characters.by_ref().take(maximum_chars).collect();
    if characters.next().is_some() {
        return format!("{detail}…");
    }
    detail
}

#[cfg(test)]
mod tests {
    use super::{
        MetadataFetchLimits, MirrorHealthLimits, detail_request_url, validate_probe_request,
    };
    use crate::types::index::OfficialPackage;

    #[test]
    /// What: Encode official selector values in the caller endpoint query.
    ///
    /// Inputs:
    /// - A selector with spaces and query-significant characters.
    ///
    /// Output:
    /// - A URL retaining the endpoint authority and encoded query values.
    ///
    /// Details:
    /// - Guards against string-concatenation endpoint injection.
    fn detail_url_encodes_selector_values() {
        let package = OfficialPackage {
            name: "pkg+name".to_string(),
            repo: "extra&bad".to_string(),
            arch: "x86_64".to_string(),
            version: String::new(),
            description: String::new(),
        };
        let url =
            detail_request_url("https://example.invalid/search", &package).expect("valid test URL");
        assert_eq!(url.host_str(), Some("example.invalid"));
        assert!(
            url.query()
                .is_some_and(|query| query.contains("pkg%2Bname"))
        );
        assert_eq!(MetadataFetchLimits::default().max_candidates, 16);
    }

    #[test]
    /// What: Reject an authority-changing or unbounded mirror probe request.
    ///
    /// Inputs:
    /// - A parent traversal path and zero mirror bound.
    ///
    /// Output:
    /// - Both requests return validation errors before I/O.
    ///
    /// Details:
    /// - Protects the bounded caller-selected relative probe contract.
    fn rejects_invalid_probe_requests() {
        assert!(
            validate_probe_request("/../etc/passwd", MirrorHealthLimits { max_mirrors: 1 })
                .is_err()
        );
        assert!(validate_probe_request("/core.db", MirrorHealthLimits { max_mirrors: 0 }).is_err());
    }
}
