//! Comprehensive example demonstrating all news module features.
//!
//! This example shows how to use:
//! - Arch news RSS fetching
//! - Security advisory fetching with cutoff dates
//! - Offline parsing of recorded feeds
//! - Date normalization for merging feeds
//!
//! Run with:
//!   `cargo run --example news_example --features news`

#[cfg(not(feature = "news"))]
fn main() {
    eprintln!("This example requires the 'news' feature to be enabled.");
    eprintln!("Run with: cargo run --example news_example --features news");
    std::process::exit(1);
}

#[cfg(feature = "news")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use arch_toolkit::news::{
        fetch_arch_news, fetch_security_advisories, normalize_feed_date, parse_arch_news_rss,
    };

    println!("=== Arch Toolkit News Module Examples ===\n");

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "arch-toolkit-example/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    // Example 1: Fetch recent Arch news
    println!("1. Recent Arch Linux News");
    println!("--------------------------");
    match fetch_arch_news(&client, 5, None).await {
        Ok(items) => {
            for item in items {
                println!("{}  {}", item.date, item.title);
                println!("            {}", item.url);
            }
        }
        Err(e) => println!("Fetch failed (offline?): {e}"),
    }
    println!();

    // Example 2: Fetch recent security advisories
    println!("2. Recent Security Advisories");
    println!("------------------------------");
    match fetch_security_advisories(&client, 5, None).await {
        Ok(mut advisories) => {
            // Sort most severe first, then newest
            advisories.sort_by(|a, b| {
                b.severity
                    .rank()
                    .cmp(&a.severity.rank())
                    .then(b.date.cmp(&a.date))
            });
            for advisory in advisories {
                println!(
                    "{}  [{}] {}",
                    advisory.date, advisory.severity, advisory.title
                );
                if !advisory.packages.is_empty() {
                    println!("            packages: {}", advisory.packages.join(", "));
                }
            }
        }
        Err(e) => println!("Fetch failed (offline?): {e}"),
    }
    println!();

    // Example 3: Offline parsing of a recorded feed
    println!("3. Offline Parsing (recorded feed)");
    println!("-----------------------------------");
    let recorded = r"<item><title>Recorded item</title>
        <link>https://archlinux.org/news/recorded/</link>
        <pubDate>Thu, 21 Aug 2025 12:00:00 +0000</pubDate></item>";
    let items = parse_arch_news_rss(recorded, 10, None);
    println!("Parsed {} item(s) without network", items.len());
    println!("  {} {}", items[0].date, items[0].title);
    println!();

    // Example 4: Date normalization across feed formats
    println!("4. Date Normalization");
    println!("----------------------");
    for raw in [
        "Thu, 21 Aug 2025 12:34:56 +0000",
        "2025-12-07T14:00:00Z",
        "2025-12-07",
    ] {
        println!("{raw:38} -> {}", normalize_feed_date(raw));
    }
    println!();

    println!("=== All examples completed ===");
    Ok(())
}
