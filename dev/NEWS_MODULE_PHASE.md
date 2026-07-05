# News Module - Phase 5 Implementation Plan

This document provides a detailed structured plan for implementing the News Module (`feature = "news"`) in arch-toolkit. This is Phase 5 of the extraction plan from Pacsea.

---

## Executive Summary

| Aspect | Details |
|--------|---------|
| **Module** | `news` (feature = "news") |
| **Source** | `Pacsea/src/sources/news/` + `src/sources/advisories.rs` (~2,100 lines — ~500 framework-agnostic) |
| **Estimated Effort** | 8-12 hours |
| **Complexity** | Low-Medium (feed parsing, date normalization) |
| **Dependencies** | `reqwest`, `chrono` (via feature flag) |
| **Status** | ✅ Core Complete - Tasks 5.1 through 5.4 complete (2026-07-05) |

## Design Philosophy

Following the toolkit's established pattern:

1. **Parse functions are pure** — `parse_arch_news_rss()` and `parse_advisories_atom()`
   take a feed body string and return items, so they are unit-testable without network.
2. **Fetch functions take `&reqwest::Client`** — same pattern as `deps::fetch_srcinfo()`;
   no hidden global HTTP client, callers control timeouts/user-agent.
3. **No caching layer** — Pacsea's in-memory/disk article caches are app policy; callers
   cache if they want (the aur module's `CacheConfig` pattern can be extended later).
4. **Date normalization built in** — feed dates (RFC 2822 / RFC 3339 / ISO 8601) are
   normalized to `YYYY-MM-DD` so items sort lexicographically, ported from Pacsea's
   `strip_time_and_tz()` using chrono.

## Source Analysis

### What to Extract

| Pacsea Source | What to Extract | What to Drop |
|---------------|-----------------|--------------|
| `news/fetch.rs` (~470) | `fetch_arch_news()` RSS item parsing (title/link/pubDate, cutoff filtering) | curl usage, article-content fetching, caches, AUR comment integration |
| `advisories.rs` (~160) | `fetch_security_advisories()` Atom entry parsing (title/link/updated/summary) | `NewsFeedItem` coupling |
| `news/utils.rs` (~175) | `extract_between()`, `strip_time_and_tz()` date normalization | URL classification helpers (article-fetch concerns) |
| `state/types.rs` | `AdvisorySeverity` + `severity_rank()` | `NewsFeedSource`, `NewsFeedItem` (UI aggregation types) |

### Explicitly Out of Scope

- **Article HTML extraction** (`news/parse.rs`, scraper-based) — depends on Pacsea's
  package-JSON caches and AUR comment rendering; possible future enhancement
- **Feed aggregation/sorting UI types** (`NewsFeedItem`, `NewsFeedSource`) — Pacsea
  merges five source kinds for its TUI; the library returns typed items per source
- **In-memory/disk article caches** — caller policy
- **AUR package update feeds** (`news/aur.rs`, `feeds/`) — built on Pacsea's index/state

---

## Proposed API Design

### Module Structure

```
arch-toolkit/src/
├── news/                       # feature = "news"
│   ├── mod.rs                  # Public API re-exports + docs
│   ├── arch.rs                 # Arch news RSS fetch + parse
│   ├── advisories.rs           # Security advisories Atom fetch + parse
│   └── date.rs                 # Feed date normalization (chrono)
└── types/
    └── news.rs                 # ArchNewsItem, SecurityAdvisory, AdvisorySeverity
```

### Core Types

```rust
/// A news item from the Arch Linux news RSS feed.
pub struct ArchNewsItem {
    pub date: String,       // normalized YYYY-MM-DD
    pub title: String,
    pub url: String,
}

/// Severity level of a security advisory.
pub enum AdvisorySeverity { Unknown, Low, Medium, High, Critical }
impl AdvisorySeverity {
    pub fn rank(self) -> u8;                  // for sorting (Critical=5 … Unknown=1)
    pub fn parse(s: &str) -> Self;            // case-insensitive from feed strings
}

/// A security advisory from security.archlinux.org.
pub struct SecurityAdvisory {
    pub id: String,          // advisory URL or title fallback
    pub date: String,        // normalized YYYY-MM-DD
    pub title: String,
    pub summary: Option<String>,
    pub url: Option<String>,
    pub severity: AdvisorySeverity,
    pub packages: Vec<String>,
}
```

### Public Functions

```rust
// Pure parsers (unit-testable, no network)
pub fn parse_arch_news_rss(body: &str, limit: usize, cutoff_date: Option<&str>) -> Vec<ArchNewsItem>;
pub fn parse_advisories_atom(body: &str, limit: usize, cutoff_date: Option<&str>) -> Vec<SecurityAdvisory>;

// Fetchers (async, take caller-provided client)
pub async fn fetch_arch_news(client: &reqwest::Client, limit: usize, cutoff_date: Option<&str>) -> Result<Vec<ArchNewsItem>>;
pub async fn fetch_security_advisories(client: &reqwest::Client, limit: usize, cutoff_date: Option<&str>) -> Result<Vec<SecurityAdvisory>>;

// Date normalization (exposed; useful for callers merging feeds)
pub fn normalize_feed_date(raw: &str) -> String;   // → YYYY-MM-DD best-effort
```

Feed URLs: `https://archlinux.org/feeds/news/` (RSS), `https://security.archlinux.org/advisory/feed.atom` (Atom).

---

## Implementation Tasks

### Task 5.1: Define Standalone Types

**File**: `src/types/news.rs`

- [x] `ArchNewsItem`, `SecurityAdvisory`, `AdvisorySeverity` with Serde + Display
- [x] `AdvisorySeverity::rank()` (from Pacsea's `severity_rank`) and `parse()`
- [x] Unit tests, rustdoc (What/Inputs/Output/Details)

**Estimated Effort**: 1-2 hours — **Status**: ✅ Complete

### Task 5.2: Arch News RSS

**File**: `src/news/arch.rs`, `src/news/date.rs`

- [x] Port `extract_between()` and iterative `<item>` parsing from fetch.rs
- [x] Port `strip_time_and_tz()` → `normalize_feed_date()` (chrono-based)
- [x] XML entity unescaping for titles (improvement over Pacsea)
- [x] Cutoff-date early termination
- [x] `fetch_arch_news()` using caller-provided `reqwest::Client`
- [x] Unit tests with sample RSS content

**Estimated Effort**: 3-4 hours — **Status**: ✅ Complete

### Task 5.3: Security Advisories

**File**: `src/news/advisories.rs`

- [x] Port Atom `<entry>` parsing from advisories.rs (title/link href/updated/summary)
- [x] Severity parsing from advisory titles (e.g., "[ASA-...] package: title (critical)")
- [x] Package name extraction from advisory titles (improvement over Pacsea, which left it empty)
- [x] Cutoff-date early termination
- [x] `fetch_security_advisories()` using caller-provided client
- [x] Unit tests with sample Atom content

**Estimated Effort**: 2-3 hours — **Status**: ✅ Complete

### Task 5.4: Integration, Testing, Documentation

- [x] `src/news/mod.rs` with module docs and re-exports
- [x] `news = ["dep:reqwest", "dep:chrono"]` feature flag
- [x] lib.rs conditional module + re-exports, prelude exports
- [x] Integration tests `tests/news_integration.rs` (parsers; network tests `#[ignore]`)
- [x] Example program `examples/news_example.rs`
- [x] README section
- [x] Quality checks across feature combos (including standalone `--no-default-features --features news`)

**Estimated Effort**: 2-3 hours — **Status**: ✅ Complete

---

## Acceptance Criteria

- [x] Parsers are pure functions testable without network
- [x] Fetchers accept caller-provided `reqwest::Client` (no global client)
- [x] Dates normalized to `YYYY-MM-DD` across RFC 2822 / RFC 3339 / ISO 8601 inputs
- [x] `cargo fmt` / `cargo clippy` clean (all feature combos)
- [x] All tests pass with `cargo test -- --test-threads=1`
- [x] Works standalone: `--no-default-features --features news`

## Future Enhancements

- [ ] Article content extraction (HTML → text, from Pacsea `news/parse.rs`)
- [ ] Per-advisory detail fetching (severity/packages from advisory pages)
- [ ] Optional caching integration with the `aur` module's cache layer

---

## References

- [AUR_TOOLKIT_CRATE_PREPARATION.md](./AUR_TOOLKIT_CRATE_PREPARATION.md) - Overall extraction plan
- Pacsea source: `src/sources/news/`, `src/sources/advisories.rs`
- Feeds: <https://archlinux.org/feeds/news/>, <https://security.archlinux.org/advisory/feed.atom>
