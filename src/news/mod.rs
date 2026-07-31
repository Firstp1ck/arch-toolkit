//! News module for Arch Linux news and security advisories.
//!
//! This module provides:
//!
//! - **Arch news** — fetch and parse the official news RSS feed
//!   (`https://archlinux.org/feeds/news/`)
//! - **Security advisories** — fetch and parse the advisory Atom feed
//!   (`https://security.archlinux.org/advisory/feed.atom`)
//! - **Date normalization** — feed dates (RFC 2822 / RFC 3339 / ISO 8601)
//!   normalized to `YYYY-MM-DD` for lexicographic sorting
//!
//! Parse functions are pure (no network), so applications can test against
//! recorded feeds. Fetch functions take a caller-provided `reqwest::Client`,
//! keeping timeouts, user agent, and fetch cadence under caller control.
//!
//! # Features
//!
//! This module requires the `news` feature flag:
//!
//! ```toml
//! [dependencies]
//! arch-toolkit = { version = "0.2", features = ["news"] }
//! ```
//!
//! # Examples
//!
//! ## Fetch Recent News
//!
//! ```no_run
//! use arch_toolkit::news::fetch_arch_news;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = reqwest::Client::new();
//! let items = fetch_arch_news(&client, 10, None).await?;
//! for item in items {
//!     println!("{} {}", item.date, item.title);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Fetch Security Advisories Since a Date
//!
//! ```no_run
//! use arch_toolkit::news::fetch_security_advisories;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = reqwest::Client::new();
//! let advisories = fetch_security_advisories(&client, 50, Some("2026-01-01")).await?;
//! for advisory in advisories {
//!     println!("{} [{}] {}", advisory.date, advisory.severity, advisory.title);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Parse a Recorded Feed (no network)
//!
//! ```
//! use arch_toolkit::news::parse_arch_news_rss;
//!
//! let rss = "<item><title>Update</title><link>https://archlinux.org/news/u/</link>\
//!            <pubDate>Thu, 21 Aug 2025 12:00:00 +0000</pubDate></item>";
//! let items = parse_arch_news_rss(rss, 10, None);
//! assert_eq!(items[0].date, "2025-08-21");
//! ```

mod advisories;
mod arch;
mod article;
mod cache;
mod date;

// Re-export types from types module
pub use crate::types::news::{AdvisorySeverity, ArchNewsItem, SecurityAdvisory};

// Re-export generic cache boundary
pub use cache::{FeedCache, InMemoryFeedCache};

// Re-export news functions
pub use arch::{
    ARCH_NEWS_FEED_URL, MAX_FEED_RESPONSE_BYTES, fetch_arch_news, fetch_arch_news_cached,
    fetch_arch_news_cached_from, fetch_arch_news_from, parse_arch_news_rss,
};

// Re-export article extraction functions
pub use article::{
    MAX_ARTICLE_HTML_BYTES, MAX_ARTICLE_TEXT_BYTES, extract_article_text, fetch_article_text,
};

// Re-export advisory functions
pub use advisories::{
    ADVISORY_FEED_URL, fetch_security_advisories, fetch_security_advisories_cached,
    fetch_security_advisories_cached_from, fetch_security_advisories_from, parse_advisories_atom,
};

// Re-export date utilities
pub use date::normalize_feed_date;
