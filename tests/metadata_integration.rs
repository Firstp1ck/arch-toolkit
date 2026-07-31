//! Deterministic integration tests for optional metadata capabilities.

#[cfg(all(feature = "aur", feature = "index"))]
mod tests {
    use arch_toolkit::aur::{
        MetadataFetchLimits, check_mirror_health, fetch_official_package_detail_from,
    };
    use arch_toolkit::types::index::{MirrorInfo, OfficialPackage};
    use arch_toolkit::{MirrorHealthLimits, MirrorHealthStatus};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    /// What: Fetch one official package detail through caller-owned HTTP transport.
    ///
    /// Inputs:
    /// - A local wiremock Arch Packages-shaped response and an index package selector.
    ///
    /// Output:
    /// - An enriched `OfficialPackage` with deterministic version and metadata.
    ///
    /// Details:
    /// - The test proves no system package command or external endpoint is used.
    async fn official_detail_uses_existing_index_model_and_caller_client() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/packages/search/json/"))
            .and(query_param("name", "ripgrep"))
            .and(query_param("repo", "extra"))
            .and(query_param("arch", "x86_64"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"results":[{"pkgname":"ripgrep","repo":"extra","arch":"x86_64","pkgver":"14.1.0","pkgrel":"2","pkgdesc":"Fast grep"}]}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let selector = OfficialPackage {
            name: "ripgrep".to_string(),
            repo: "extra".to_string(),
            arch: "x86_64".to_string(),
            version: String::new(),
            description: String::new(),
        };
        let endpoint = format!("{}/packages/search/json/", server.uri());
        let detail = fetch_official_package_detail_from(
            &reqwest::Client::new(),
            &endpoint,
            &selector,
            MetadataFetchLimits {
                max_response_bytes: 4 * 1024,
                max_candidates: 4,
            },
        )
        .await
        .expect("fetch fixture")
        .expect("exact package detail");

        assert_eq!(detail.version, "14.1.0-2");
        assert_eq!(detail.description, "Fast grep");
    }

    #[tokio::test]
    /// What: Report mirror reachability through bounded, caller-selected probes.
    ///
    /// Inputs:
    /// - Two local mirror URLs with success and failure status responses.
    ///
    /// Output:
    /// - Ordered per-mirror health records with status evidence.
    ///
    /// Details:
    /// - Wiremock makes network behavior deterministic and proves checks do not
    ///   apply generated mirror configuration or execute a system command.
    async fn mirror_health_uses_existing_mirror_models_and_bounded_probes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/healthy/core/os/x86_64/core.db"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/unhealthy/core/os/x86_64/core.db"))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&server)
            .await;

        let mirrors = vec![
            MirrorInfo {
                url: format!("{}/healthy/", server.uri()),
                active: true,
                protocols: vec!["http".to_string()],
            },
            MirrorInfo {
                url: format!("{}/unhealthy/", server.uri()),
                active: true,
                protocols: vec!["http".to_string()],
            },
        ];
        let health = check_mirror_health(
            &reqwest::Client::new(),
            &mirrors,
            "/core/os/x86_64/core.db",
            MirrorHealthLimits { max_mirrors: 2 },
        )
        .await
        .expect("probe fixture");

        assert_eq!(health.len(), 2);
        assert_eq!(health[0].status, MirrorHealthStatus::Reachable);
        assert_eq!(health[0].status_code, Some(200));
        assert_eq!(health[1].status, MirrorHealthStatus::Unreachable);
        assert_eq!(health[1].status_code, Some(503));
    }
}
