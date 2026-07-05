//! Integration tests for the news module.
//!
//! Parser tests run offline against recorded feed snippets. Network tests are
//! `#[ignore]`d by default; run them with `cargo test -- --ignored`.

#[cfg(feature = "news")]
mod tests {
    use arch_toolkit::news::{normalize_feed_date, parse_advisories_atom, parse_arch_news_rss};
    use arch_toolkit::types::news::AdvisorySeverity;

    #[test]
    /// What: Verify a realistic RSS feed parses end to end with sorting-safe dates.
    ///
    /// Inputs:
    /// - Multi-item RSS snippet in the archlinux.org feed shape.
    ///
    /// Output:
    /// - Items in feed order with normalized, lexicographically sortable dates.
    ///
    /// Details:
    /// - Confirms normalized dates compare correctly across month boundaries.
    fn news_feed_workflow() {
        let rss = r"<rss><channel>
        <item><title>news one</title><link>https://archlinux.org/news/one/</link>
        <pubDate>Tue, 01 Jul 2025 10:00:00 +0000</pubDate></item>
        <item><title>news two</title><link>https://archlinux.org/news/two/</link>
        <pubDate>Mon, 30 Jun 2025 10:00:00 +0000</pubDate></item>
        </channel></rss>";
        let items = parse_arch_news_rss(rss, 10, None);
        assert_eq!(items.len(), 2);
        assert!(items[0].date > items[1].date);
        assert_eq!(items[0].date, "2025-07-01");
        assert_eq!(items[1].date, "2025-06-30");
    }

    #[test]
    /// What: Verify advisories sort by severity rank as Pacsea does.
    ///
    /// Inputs:
    /// - Atom snippet with critical-marked and unmarked advisories.
    ///
    /// Output:
    /// - Sorting by `severity.rank()` puts the critical advisory first.
    ///
    /// Details:
    /// - Exercises the severity extraction and rank ordering together.
    fn advisory_severity_sorting() {
        let atom = r#"<feed>
        <entry><title>[ASA-1] pkga: minor issue</title>
        <link href="https://security.archlinux.org/ASA-1"/>
        <updated>2026-07-01T00:00:00Z</updated></entry>
        <entry><title>[ASA-2] pkgb: bad issue</title>
        <link href="https://security.archlinux.org/ASA-2"/>
        <updated>2026-07-02T00:00:00Z</updated>
        <summary>Remote code execution (critical)</summary></entry>
        </feed>"#;
        let mut advisories = parse_advisories_atom(atom, 10, None);
        advisories.sort_by_key(|a| std::cmp::Reverse(a.severity.rank()));
        assert_eq!(advisories[0].severity, AdvisorySeverity::Critical);
        assert_eq!(advisories[0].packages, ["pkgb"]);
        assert_eq!(advisories[1].severity, AdvisorySeverity::Unknown);
    }

    #[test]
    /// What: Verify date normalization supports merging items from both feeds.
    ///
    /// Inputs:
    /// - RSS-style RFC 2822 and Atom-style RFC 3339 dates for the same day.
    ///
    /// Output:
    /// - Identical normalized values.
    ///
    /// Details:
    /// - Callers merging news + advisories rely on this for stable sorting.
    fn cross_feed_date_normalization() {
        assert_eq!(
            normalize_feed_date("Tue, 01 Jul 2025 10:00:00 +0000"),
            normalize_feed_date("2025-07-01T10:00:00Z"),
        );
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    /// What: Verify the live Arch news feed fetches and parses.
    ///
    /// Inputs:
    /// - Live request to archlinux.org (ignored by default).
    ///
    /// Output:
    /// - At least one item with a normalized date.
    ///
    /// Details:
    /// - Run explicitly with `cargo test --features news -- --ignored`.
    async fn live_arch_news() {
        let client = reqwest::Client::new();
        let items = arch_toolkit::news::fetch_arch_news(&client, 5, None)
            .await
            .expect("fetch should succeed");
        assert!(!items.is_empty());
        assert!(items[0].date.len() == 10 && items[0].date.contains('-'));
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    /// What: Verify the live advisory feed fetches and parses.
    ///
    /// Inputs:
    /// - Live request to security.archlinux.org (ignored by default).
    ///
    /// Output:
    /// - At least one advisory with an id.
    ///
    /// Details:
    /// - Run explicitly with `cargo test --features news -- --ignored`.
    async fn live_advisories() {
        let client = reqwest::Client::new();
        let advisories = arch_toolkit::news::fetch_security_advisories(&client, 5, None)
            .await
            .expect("fetch should succeed");
        assert!(!advisories.is_empty());
        assert!(!advisories[0].id.is_empty());
    }
}
