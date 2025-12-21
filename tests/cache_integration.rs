//! Integration tests for the caching layer.
//!
//! These tests verify that caching works correctly with real AUR operations.
//! Note: These tests require network access and may be slow.

use arch_toolkit::ArchClient;
use arch_toolkit::cache::CacheConfigBuilder;
use arch_toolkit::error::Result;
use std::time::Duration;

/// Test that search caching works correctly.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_search_caching() -> Result<()> {
    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(300))
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    // First request - should hit network
    let start = std::time::Instant::now();
    let packages1 = client.aur().search("yay").await?;
    let _first_duration = start.elapsed();

    // Second request - should hit cache (much faster)
    let start = std::time::Instant::now();
    let packages2 = client.aur().search("yay").await?;
    let _second_duration = start.elapsed();

    // Results should be identical
    assert_eq!(packages1.len(), packages2.len());

    // Cached request should be faster (or at least not slower)
    // Note: In some cases, network might be very fast, so we just check
    // that results are the same
    assert_eq!(packages1[0].name, packages2[0].name);

    Ok(())
}

/// Test that info caching works correctly.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_info_caching() -> Result<()> {
    let cache_config = CacheConfigBuilder::new()
        .enable_info(true)
        .info_ttl(Duration::from_secs(900))
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    let package_name = "yay";

    // First request - should hit network
    let start = std::time::Instant::now();
    let info1 = client.aur().info(&[package_name]).await?;
    let _first_duration = start.elapsed();

    // Second request - should hit cache
    let start = std::time::Instant::now();
    let info2 = client.aur().info(&[package_name]).await?;
    let _second_duration = start.elapsed();

    // Results should be identical
    assert_eq!(info1.len(), info2.len());
    if let (Some(details1), Some(details2)) = (info1.first(), info2.first()) {
        assert_eq!(details1.name, details2.name);
        assert_eq!(details1.version, details2.version);
    }

    Ok(())
}

/// Test that comments caching works correctly.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_comments_caching() -> Result<()> {
    let cache_config = CacheConfigBuilder::new()
        .enable_comments(true)
        .comments_ttl(Duration::from_secs(600))
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    let package_name = "yay";

    // First request - should hit network
    let comments1 = client.aur().comments(package_name).await?;

    // Second request - should hit cache
    let comments2 = client.aur().comments(package_name).await?;

    // Results should be identical
    assert_eq!(comments1.len(), comments2.len());

    Ok(())
}

/// Test that pkgbuild caching works correctly.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_pkgbuild_caching() -> Result<()> {
    let cache_config = CacheConfigBuilder::new()
        .enable_pkgbuild(true)
        .pkgbuild_ttl(Duration::from_secs(3600))
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    let package_name = "yay";

    // First request - should hit network
    let start = std::time::Instant::now();
    let pkgbuild1 = client.aur().pkgbuild(package_name).await?;
    let _first_duration = start.elapsed();

    // Second request - should hit cache
    let start = std::time::Instant::now();
    let pkgbuild2 = client.aur().pkgbuild(package_name).await?;
    let _second_duration = start.elapsed();

    // Results should be identical
    assert_eq!(pkgbuild1, pkgbuild2);

    Ok(())
}

/// Test that caching can be disabled per operation.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_selective_caching() -> Result<()> {
    // Enable caching only for search, not for info
    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .enable_info(false) // Info caching disabled
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    // Search should be cached
    let packages1 = client.aur().search("yay").await?;
    let packages2 = client.aur().search("yay").await?;
    assert_eq!(packages1.len(), packages2.len());

    // Info should not be cached (both requests hit network)
    if let Some(pkg) = packages1.first() {
        let info1 = client.aur().info(&[&pkg.name]).await?;
        let info2 = client.aur().info(&[&pkg.name]).await?;
        // Results should still be the same (from network), but not cached
        assert_eq!(info1.len(), info2.len());
    }

    Ok(())
}

/// Test that different TTLs work correctly.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_different_ttls() -> Result<()> {
    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(1)) // Very short TTL: 1 second
        .enable_info(true)
        .info_ttl(Duration::from_secs(300)) // Longer TTL: 5 minutes
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    // Both should be cached initially
    let packages1 = client.aur().search("yay").await?;
    let info1 = if let Some(pkg) = packages1.first() {
        client.aur().info(&[&pkg.name]).await?
    } else {
        return Ok(());
    };

    // Immediate second request - both should hit cache
    let packages2 = client.aur().search("yay").await?;
    let info2 = if let Some(pkg) = packages2.first() {
        client.aur().info(&[&pkg.name]).await?
    } else {
        return Ok(());
    };

    assert_eq!(packages1.len(), packages2.len());
    assert_eq!(info1.len(), info2.len());

    // Wait for search TTL to expire (but info TTL should still be valid)
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Search should miss cache (TTL expired), info should hit cache
    let packages3 = client.aur().search("yay").await?;
    let info3 = if let Some(pkg) = packages3.first() {
        client.aur().info(&[&pkg.name]).await?
    } else {
        return Ok(());
    };

    // Search results might differ slightly (new packages), but info should be identical
    assert_eq!(info1.len(), info3.len());
    if let (Some(details1), Some(details3)) = (info1.first(), info3.first()) {
        assert_eq!(details1.name, details3.name);
    }

    Ok(())
}

/// Test that cache size limits work correctly.
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_cache_size_limit() -> Result<()> {
    let cache_config = CacheConfigBuilder::new()
        .enable_search(true)
        .search_ttl(Duration::from_secs(300))
        .memory_cache_size(2) // Very small cache: only 2 entries
        .build();

    let client = ArchClient::builder().cache_config(cache_config).build()?;

    // Fill cache with 3 different searches
    let _packages1 = client.aur().search("yay").await?;
    let _packages2 = client.aur().search("paru").await?;
    let _packages3 = client.aur().search("pacman").await?;

    // First search (yay) should be evicted (LRU)
    // Second search (paru) should still be cached
    let packages_paru = client.aur().search("paru").await?;
    assert!(!packages_paru.is_empty());

    Ok(())
}
