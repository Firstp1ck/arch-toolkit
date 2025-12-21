//! Example demonstrating the optional caching layer for arch-toolkit.
//!
//! This example shows how to:
//! - Configure caching for AUR operations
//! - Enable caching for specific operations (search, info, comments, pkgbuild)
//! - Configure TTL (time-to-live) for cache entries
//! - Enable disk cache for persistence across restarts
//! - Use the cache to reduce network requests
//!
//! Note: Caching is disabled by default. You must explicitly enable it
//! via `CacheConfig` when creating the `ArchClient`.

use arch_toolkit::ArchClient;
use arch_toolkit::cache::CacheConfigBuilder;
use arch_toolkit::error::Result;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║       arch-toolkit: Caching Layer Example                     ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Basic Caching Configuration
    // ========================================================================
    println!("┌─ Example 1: Basic Caching Configuration ────────────────────┐");
    println!("│ Enable caching for search operations with default TTL         │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(300)) // 5 minutes
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    println!("✓ Client created with search caching enabled (5 min TTL)\n");

    // First search - will hit the network
    println!("Performing first search (network request)...");
    let start = std::time::Instant::now();
    let packages1 = client.aur().search("yay").await?;
    let first_duration = start.elapsed();
    println!(
        "  Found {} packages in {:?}\n",
        packages1.len(),
        first_duration
    );

    // Second search - should hit the cache
    println!("Performing second search (should use cache)...");
    let start = std::time::Instant::now();
    let packages2 = client.aur().search("yay").await?;
    let second_duration = start.elapsed();
    println!(
        "  Found {} packages in {:?}",
        packages2.len(),
        second_duration
    );
    println!(
        "  Cache speedup: {:.1}x faster\n",
        first_duration.as_secs_f64() / second_duration.as_secs_f64().max(0.001)
    );

    // ========================================================================
    // Example 2: Multiple Operation Caching
    // ========================================================================
    println!("┌─ Example 2: Multiple Operation Caching ───────────────────┐");
    println!("│ Enable caching for search, info, and comments                │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(300)) // 5 minutes
        .enable_info(true)
        .info_ttl(Duration::from_secs(900)) // 15 minutes
        .enable_comments(true)
        .comments_ttl(Duration::from_secs(600)) // 10 minutes
        .memory_cache_size(200) // Increase cache size
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    println!("✓ Client created with caching for search, info, and comments\n");

    // Search with caching
    println!("Searching for 'paru'...");
    let packages = client.aur().search("paru").await?;
    println!("  Found {} packages\n", packages.len());

    if let Some(pkg) = packages.first() {
        // Info with caching
        println!("Fetching info for '{}'...", pkg.name);
        let info = client.aur().info(&[&pkg.name]).await?;
        if let Some(details) = info.first() {
            println!("  Package: {}", details.name);
            println!("  Version: {}", details.version);
            println!("  Description: {}", details.description);
        }

        // Comments with caching
        println!("\nFetching comments for '{}'...", pkg.name);
        let comments = client.aur().comments(&pkg.name).await?;
        println!("  Found {} comments\n", comments.len());
    }

    // ========================================================================
    // Example 3: Disk Cache (Optional Feature)
    // ========================================================================
    #[cfg(feature = "cache-disk")]
    {
        println!("┌─ Example 3: Disk Cache (Optional Feature) ─────────────────┐");
        println!("│ Enable disk cache for persistence across restarts          │");
        println!("└──────────────────────────────────────────────────────────────┘");

        let cache_config = CacheConfigBuilder::new()
            .enable_search(true)
            .search_ttl(Duration::from_secs(300))
            .enable_pkgbuild(true)
            .pkgbuild_ttl(Duration::from_secs(3600)) // 1 hour
            .enable_disk_cache(true) // Enable disk cache
            .memory_cache_size(100)
            .build();

        let client = ArchClient::builder().cache_config(cache_config).build()?;

        println!("✓ Client created with disk cache enabled\n");
        println!("  Disk cache persists data across application restarts");
        println!("  PKGBUILD cache entries will be stored on disk\n");

        // PKGBUILD with disk caching
        if let Some(pkg) = packages.first() {
            println!(
                "Fetching PKGBUILD for '{}' (will be cached to disk)...",
                pkg.name
            );
            let pkgbuild = client.aur().pkgbuild(&pkg.name).await?;
            println!("  PKGBUILD length: {} bytes\n", pkgbuild.len());
        }
    }

    #[cfg(not(feature = "cache-disk"))]
    {
        println!("┌─ Example 3: Disk Cache (Optional Feature) ─────────────────┐");
        println!("│ Disk cache feature not enabled in this build                │");
        println!("└──────────────────────────────────────────────────────────────┘");
        println!("  To enable disk cache, compile with: --features cache-disk\n");
    }

    // ========================================================================
    // Example 4: Custom TTL Configuration
    // ========================================================================
    println!("┌─ Example 4: Custom TTL Configuration ───────────────────────┐");
    println!("│ Configure different TTLs for different operations             │");
    println!("└──────────────────────────────────────────────────────────────┘");

    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(60)) // Short TTL: 1 minute
        .enable_info(true)
        .info_ttl(Duration::from_secs(1800)) // Longer TTL: 30 minutes
        .enable_pkgbuild(true)
        .pkgbuild_ttl(Duration::from_secs(7200)) // Very long TTL: 2 hours
        .memory_cache_size(50) // Smaller cache
        .build();

    let _client = ArchClient::builder().cache_config(cache_config).build()?;

    println!("✓ Client created with custom TTLs:");
    println!("  Search: 1 minute");
    println!("  Info: 30 minutes");
    println!("  PKGBUILD: 2 hours\n");

    // ========================================================================
    // Example 5: Cache Performance Comparison
    // ========================================================================
    println!("┌─ Example 5: Cache Performance Comparison ───────────────────┐");
    println!("│ Compare performance with and without caching                  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // Without cache
    let no_cache_client = ArchClient::new()?;
    println!("Testing without cache...");
    let start = std::time::Instant::now();
    let _packages1 = no_cache_client.aur().search("yay").await?;
    let no_cache_time = start.elapsed();
    println!("  Time without cache: {no_cache_time:?}\n");

    // With cache (first request)
    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(300))
        .build();
    let cache_client = ArchClient::builder().cache_config(cache_config).build()?;

    println!("Testing with cache (first request - network)...");
    let start = std::time::Instant::now();
    let _packages2 = cache_client.aur().search("yay").await?;
    let cache_first_time = start.elapsed();
    println!("  Time with cache (first): {cache_first_time:?}");

    // With cache (second request - should hit cache)
    println!("Testing with cache (second request - cached)...");
    let start = std::time::Instant::now();
    let _packages3 = cache_client.aur().search("yay").await?;
    let cache_second_time = start.elapsed();
    println!("  Time with cache (second): {cache_second_time:?}");

    if cache_second_time < cache_first_time {
        let speedup = cache_first_time.as_secs_f64() / cache_second_time.as_secs_f64().max(0.001);
        println!("  Cache speedup: {speedup:.1}x faster\n");
    }

    println!("✅ Caching example completed successfully!");
    Ok(())
}
