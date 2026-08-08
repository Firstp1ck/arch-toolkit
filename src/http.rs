//! Shared private HTTP response-body safeguards.

use crate::error::{ArchToolkitError, Result};

/// What: Read one HTTP response body with a strict streamed byte ceiling and UTF-8 validation.
///
/// Inputs:
/// - `response`: Successful response whose body remains unread.
/// - `maximum_bytes`: Maximum accepted body length in bytes.
/// - `resource_label`: Operation and resource context used in errors.
/// - `map_read_error`: Maps streamed transport errors to the caller's existing error variant.
///
/// Output:
/// - The complete UTF-8 body when it does not exceed `maximum_bytes`.
///
/// Details:
/// - Rejects an oversized declared length before allocating or polling the body.
/// - Enforces the same ceiling on every streamed chunk when the length is absent or inaccurate.
/// - Stops as soon as the next chunk would cross the ceiling and never logs body content.
pub async fn read_bounded_response_text<F>(
    mut response: reqwest::Response,
    maximum_bytes: usize,
    resource_label: &str,
    map_read_error: F,
) -> Result<String>
where
    F: FnOnce(reqwest::Error) -> ArchToolkitError,
{
    if let Some(length) = response.content_length()
        && length > u64::try_from(maximum_bytes).unwrap_or(u64::MAX)
    {
        return Err(response_too_large(
            resource_label,
            maximum_bytes,
            usize::try_from(length).unwrap_or(usize::MAX),
        ));
    }

    let initial_capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0);
    let mut body = Vec::with_capacity(initial_capacity);
    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => return Err(map_read_error(error)),
        };
        let observed_length = body.len().saturating_add(chunk.len());
        if observed_length > maximum_bytes {
            return Err(response_too_large(
                resource_label,
                maximum_bytes,
                observed_length,
            ));
        }
        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body).map_err(|error| {
        ArchToolkitError::Parse(format!(
            "{resource_label} response body was not valid UTF-8: {error}"
        ))
    })
}

/// What: Build a contextual response-size error.
///
/// Inputs:
/// - `resource_label`: Operation and package context.
/// - `maximum_bytes`: Configured byte ceiling.
/// - `actual_bytes`: Declared or observed body length.
///
/// Output:
/// - `InputTooLong` using the existing public error surface.
///
/// Details:
/// - The field identifies the response body rather than exposing an untrusted URL or content.
fn response_too_large(
    resource_label: &str,
    maximum_bytes: usize,
    actual_bytes: usize,
) -> ArchToolkitError {
    ArchToolkitError::InputTooLong {
        field: format!("{resource_label} response body"),
        max_length: maximum_bytes,
        actual_length: actual_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// What: Map a fixture transport error to a deterministic contextual crate error.
    ///
    /// Inputs:
    /// - `error`: Reqwest body-stream error from a local fixture.
    ///
    /// Output:
    /// - `Parse` retaining the fixture operation label.
    ///
    /// Details:
    /// - Production callers instead retain their existing operation-specific network variants.
    fn fixture_read_error(error: reqwest::Error) -> ArchToolkitError {
        ArchToolkitError::Parse(format!("fixture body read failed: {}", error.without_url()))
    }

    /// What: Start a one-response HTTP/1.1 fixture server.
    ///
    /// Inputs:
    /// - `response`: Complete response bytes to write after one request.
    /// - `linger`: Time to keep the connection open after flushing.
    ///
    /// Output:
    /// - Local HTTP URL accepted by reqwest.
    ///
    /// Details:
    /// - Supports connection-close and intentionally incomplete chunked fixtures unavailable in wiremock.
    fn spawn_raw_response(response: Vec<u8>, linger: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind raw HTTP fixture");
        let address = listener.local_addr().expect("raw HTTP fixture address");
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept fixture request");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set fixture read timeout");
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request);
            stream.write_all(&response).expect("write fixture response");
            stream.flush().expect("flush fixture response");
            thread::sleep(linger);
        });
        format!("http://{address}/fixture")
    }

    /// What: Build a raw connection-close response without `Content-Length`.
    ///
    /// Inputs:
    /// - `body`: Response bytes.
    ///
    /// Output:
    /// - Complete HTTP response bytes.
    ///
    /// Details:
    /// - Closing the connection delimits the body and exercises the missing-header path.
    fn response_without_length(body: &[u8]) -> Vec<u8> {
        let mut response = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        response.extend_from_slice(body);
        response
    }

    #[tokio::test]
    /// What: Reject a declared response length above the configured ceiling.
    ///
    /// Inputs:
    /// - A wiremock body of nine bytes and an eight-byte ceiling.
    ///
    /// Output:
    /// - `InputTooLong` reporting the declared length.
    ///
    /// Details:
    /// - Hyper supplies an accurate `Content-Length` for the full response body.
    async fn declared_oversize_is_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/body"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"123456789"))
            .mount(&server)
            .await;
        let response = reqwest::get(format!("{}/body", server.uri()))
            .await
            .expect("declared-length fixture response");

        let error = read_bounded_response_text(response, 8, "declared fixture", fixture_read_error)
            .await
            .expect_err("declared oversize must fail");

        assert_eq!(
            error.to_string(),
            "declared fixture response body exceeds maximum length of 8 bytes (got 9)"
        );
        assert!(matches!(
            error,
            ArchToolkitError::InputTooLong {
                max_length: 8,
                actual_length: 9,
                ..
            }
        ));
    }

    #[tokio::test]
    /// What: Reject a dishonest overreported length before waiting for its short body.
    ///
    /// Inputs:
    /// - A response declaring nine bytes, sending one byte, and lingering with an eight-byte ceiling.
    ///
    /// Output:
    /// - Immediate `InputTooLong` based on the declared length.
    ///
    /// Details:
    /// - Proves an inaccurate header cannot force allocation or a body wait above the bound.
    async fn dishonest_declared_oversize_is_rejected_early() {
        let response =
            b"HTTP/1.1 200 OK\r\nContent-Length: 9\r\nConnection: close\r\n\r\nx".to_vec();
        let url = spawn_raw_response(response, Duration::from_secs(1));
        let response = reqwest::get(url).await.expect("dishonest-length response");

        let result = tokio::time::timeout(
            Duration::from_millis(250),
            read_bounded_response_text(response, 8, "dishonest fixture", fixture_read_error),
        )
        .await
        .expect("declared oversize should reject before body completion");

        assert!(matches!(
            result,
            Err(ArchToolkitError::InputTooLong {
                max_length: 8,
                actual_length: 9,
                ..
            })
        ));
    }

    #[tokio::test]
    /// What: Enforce the streamed ceiling when `Content-Length` is absent.
    ///
    /// Inputs:
    /// - A connection-close response of nine bytes and an eight-byte ceiling.
    ///
    /// Output:
    /// - `InputTooLong` after observing the oversized bytes.
    ///
    /// Details:
    /// - The body is delimited only by EOF, so the header cannot provide safety.
    async fn missing_length_oversize_is_rejected() {
        let url = spawn_raw_response(
            response_without_length(b"123456789"),
            Duration::from_millis(0),
        );
        let response = reqwest::get(url).await.expect("missing-length response");

        let error = read_bounded_response_text(response, 8, "missing fixture", fixture_read_error)
            .await
            .expect_err("missing-length oversize must fail");

        assert!(matches!(
            error,
            ArchToolkitError::InputTooLong {
                max_length: 8,
                actual_length: 9,
                ..
            }
        ));
    }

    #[tokio::test]
    /// What: Enforce streamed bytes when a short declared length conflicts with chunked framing.
    ///
    /// Inputs:
    /// - A response declaring one byte while chunked framing delivers nine bytes.
    ///
    /// Output:
    /// - `InputTooLong` based on the streamed body rather than the dishonest short header.
    ///
    /// Details:
    /// - Transfer framing controls the delivered chunks; the byte ceiling remains authoritative.
    async fn dishonest_short_length_cannot_bypass_stream_limit() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n9\r\n123456789\r\n0\r\n\r\n".to_vec();
        let url = spawn_raw_response(response, Duration::from_millis(0));
        let response = reqwest::get(url)
            .await
            .expect("dishonest short-length response");

        let error =
            read_bounded_response_text(response, 8, "dishonest short fixture", fixture_read_error)
                .await
                .expect_err("streamed overflow must override a dishonest short length");

        assert!(matches!(
            error,
            ArchToolkitError::InputTooLong {
                max_length: 8,
                actual_length: 9,
                ..
            }
        ));
    }

    #[tokio::test]
    /// What: Stop immediately when an incomplete chunked body crosses the limit.
    ///
    /// Inputs:
    /// - A nine-byte first chunk, no terminating chunk, and an eight-byte ceiling.
    ///
    /// Output:
    /// - `InputTooLong` before the server closes the incomplete response.
    ///
    /// Details:
    /// - A timeout shorter than the fixture linger proves the reader does not poll another chunk.
    async fn chunked_overflow_stops_immediately() {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n9\r\n123456789\r\n".to_vec();
        let url = spawn_raw_response(response, Duration::from_secs(1));
        let response = reqwest::get(url).await.expect("chunked fixture response");

        let result = tokio::time::timeout(
            Duration::from_millis(250),
            read_bounded_response_text(response, 8, "chunked fixture", fixture_read_error),
        )
        .await
        .expect("overflow should stop before the next chunk");

        assert!(matches!(
            result,
            Err(ArchToolkitError::InputTooLong {
                max_length: 8,
                actual_length: 9,
                ..
            })
        ));
    }

    #[tokio::test]
    /// What: Accept a UTF-8 body exactly at the configured byte ceiling.
    ///
    /// Inputs:
    /// - An eight-byte response and an eight-byte ceiling.
    ///
    /// Output:
    /// - The unchanged response string.
    ///
    /// Details:
    /// - Establishes inclusive boundary behavior.
    async fn exact_limit_is_accepted() {
        let url = spawn_raw_response(
            response_without_length(b"12345678"),
            Duration::from_millis(0),
        );
        let response = reqwest::get(url).await.expect("exact-limit response");

        let body = read_bounded_response_text(response, 8, "exact fixture", fixture_read_error)
            .await
            .expect("exact limit should succeed");

        assert_eq!(body, "12345678");
    }

    #[tokio::test]
    /// What: Reject invalid UTF-8 only after enforcing the response ceiling.
    ///
    /// Inputs:
    /// - A bounded three-byte sequence that is not valid UTF-8.
    ///
    /// Output:
    /// - A contextual `Parse` error.
    ///
    /// Details:
    /// - The error exposes no response-body content.
    async fn invalid_utf8_is_rejected() {
        let url = spawn_raw_response(
            response_without_length(&[0xf0, 0x28, 0x8c]),
            Duration::from_millis(0),
        );
        let response = reqwest::get(url).await.expect("invalid UTF-8 response");

        let error = read_bounded_response_text(response, 8, "UTF-8 fixture", fixture_read_error)
            .await
            .expect_err("invalid UTF-8 must fail");
        let message = error.to_string();

        assert!(matches!(error, ArchToolkitError::Parse(_)));
        assert!(message.contains("UTF-8 fixture"));
        assert!(message.contains("not valid UTF-8"));
        assert!(!message.contains('�'));
    }
}
