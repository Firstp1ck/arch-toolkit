//! Environment variable configuration example for arch-toolkit.
//!
//! This example demonstrates how to configure `ArchClient` using environment variables,
//! which is particularly useful for:
//! - CI/CD pipelines where configuration is set via environment variables
//! - Docker containers with environment-based configuration
//! - Runtime configuration without code changes
//! - Testing with different configurations
//!
//! # Supported Environment Variables
//!
//! - `ARCH_TOOLKIT_TIMEOUT`: HTTP request timeout in seconds (default: 30)
//! - `ARCH_TOOLKIT_USER_AGENT`: Custom user agent string (default: "arch-toolkit/0.1.0")
//! - `ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT`: Health check timeout in seconds (default: 5)
//! - `ARCH_TOOLKIT_MAX_RETRIES`: Maximum retry attempts (default: 3)
//! - `ARCH_TOOLKIT_RETRY_ENABLED`: Enable/disable retries ("true"/"false", default: true)
//! - `ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS`: Initial retry delay in milliseconds (default: 1000)
//! - `ARCH_TOOLKIT_RETRY_MAX_DELAY_MS`: Maximum retry delay in milliseconds (default: 30000)
//! - `ARCH_TOOLKIT_VALIDATION_STRICT`: Strict validation mode ("true"/"false", default: true)
//! - `ARCH_TOOLKIT_CACHE_SIZE`: Memory cache size (default: 100)
//!
//! # Usage
//!
//! ```bash
//! # Set environment variables
//! export ARCH_TOOLKIT_TIMEOUT=60
//! export ARCH_TOOLKIT_USER_AGENT="my-app/1.0"
//! export ARCH_TOOLKIT_MAX_RETRIES=5
//!
//! # Run the example
//! cargo run --example env_config
//! ```

use arch_toolkit::error::Result;
use arch_toolkit::{ArchClient, ArchClientBuilder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║     arch-toolkit: Environment Variable Configuration         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // Example 1: Pure Environment Configuration
    // ========================================================================
    println!("┌─ Example 1: Pure Environment Configuration ──────────────────┐");
    println!("│ Creating client entirely from environment variables           │");
    println!("│ Use: ARCH_TOOLKIT_TIMEOUT=60 cargo run --example env_config  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // This will read all configuration from environment variables
    // If environment variables are not set, defaults are used
    let client = ArchClientBuilder::from_env().build()?;
    println!("✓ Client created from environment variables\n");

    // Demonstrate that the client works
    let _packages = client.aur().search("yay").await?;
    println!("✓ Successfully performed AUR search\n");

    // ========================================================================
    // Example 2: Code Defaults with Environment Overrides
    // ========================================================================
    println!("┌─ Example 2: Code Defaults with Environment Overrides ────────┐");
    println!("│ Set code defaults, but allow environment to override         │");
    println!("│ Use: ARCH_TOOLKIT_TIMEOUT=120 cargo run --example env_config  │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // Set a timeout in code
    let _client = ArchClient::builder()
        .timeout(Duration::from_secs(60))
        .with_env() // Environment variables override the 60s timeout if ARCH_TOOLKIT_TIMEOUT is set
        .build()?;
    println!("✓ Client created with code defaults + environment overrides\n");

    // ========================================================================
    // Example 3: Environment First, Then Code Overrides
    // ========================================================================
    println!("┌─ Example 3: Environment First, Then Code Overrides ─────────┐");
    println!("│ Load from environment, then override specific values          │");
    println!("│ Use: ARCH_TOOLKIT_USER_AGENT=\"env-agent/1.0\" cargo run...    │");
    println!("└──────────────────────────────────────────────────────────────┘");

    // Load from environment first
    let _client = ArchClientBuilder::from_env()
        .user_agent("my-app/1.0") // Always use this, ignore ARCH_TOOLKIT_USER_AGENT
        .build()?;
    println!("✓ Client created from environment, then overridden in code\n");

    // ========================================================================
    // Example 4: Demonstrating Specific Environment Variables
    // ========================================================================
    println!("┌─ Example 4: Environment Variable Examples ───────────────────┐");
    println!("│                                                               │");
    println!("│ Set these environment variables to configure the client:      │");
    println!("│                                                               │");
    println!("│   # Timeout configuration                                     │");
    println!("│   export ARCH_TOOLKIT_TIMEOUT=60                              │");
    println!("│   export ARCH_TOOLKIT_HEALTH_CHECK_TIMEOUT=10                 │");
    println!("│                                                               │");
    println!("│   # User agent                                               │");
    println!("│   export ARCH_TOOLKIT_USER_AGENT=\"my-app/1.0\"                │");
    println!("│                                                               │");
    println!("│   # Retry policy                                             │");
    println!("│   export ARCH_TOOLKIT_MAX_RETRIES=5                          │");
    println!("│   export ARCH_TOOLKIT_RETRY_ENABLED=true                      │");
    println!("│   export ARCH_TOOLKIT_RETRY_INITIAL_DELAY_MS=2000            │");
    println!("│   export ARCH_TOOLKIT_RETRY_MAX_DELAY_MS=60000               │");
    println!("│                                                               │");
    println!("│   # Validation                                               │");
    println!("│   export ARCH_TOOLKIT_VALIDATION_STRICT=false                │");
    println!("│                                                               │");
    println!("│   # Cache                                                    │");
    println!("│   export ARCH_TOOLKIT_CACHE_SIZE=200                         │");
    println!("│                                                               │");
    println!("│ Then run: cargo run --example env_config                       │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ========================================================================
    // Example 5: CI/CD Usage Pattern
    // ========================================================================
    println!("┌─ Example 5: CI/CD Usage Pattern ───────────────────────────────┐");
    println!("│ Typical CI/CD configuration:                                  │");
    println!("│                                                               │");
    println!("│   # In CI/CD pipeline (e.g., GitHub Actions, GitLab CI)      │");
    println!("│   env:                                                       │");
    println!("│     ARCH_TOOLKIT_TIMEOUT: 120                                │");
    println!("│     ARCH_TOOLKIT_MAX_RETRIES: 5                              │");
    println!("│     ARCH_TOOLKIT_USER_AGENT: \"ci-runner/1.0\"                 │");
    println!("│                                                               │");
    println!("│   # In code:                                                 │");
    println!("│   let client = ArchClient::builder()                          │");
    println!("│       .from_env()                                            │");
    println!("│       .build()?;                                             │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    // ========================================================================
    // Example 6: Docker Container Usage
    // ========================================================================
    println!("┌─ Example 6: Docker Container Usage ───────────────────────────┐");
    println!("│ Dockerfile or docker-compose.yml:                            │");
    println!("│                                                               │");
    println!("│   # Dockerfile                                               │");
    println!("│   ENV ARCH_TOOLKIT_TIMEOUT=60                                 │");
    println!("│   ENV ARCH_TOOLKIT_MAX_RETRIES=3                              │");
    println!("│                                                               │");
    println!("│   # docker-compose.yml                                       │");
    println!("│   environment:                                               │");
    println!("│     - ARCH_TOOLKIT_TIMEOUT=60                                 │");
    println!("│     - ARCH_TOOLKIT_MAX_RETRIES=3                              │");
    println!("│                                                               │");
    println!("│   # In code (same as CI/CD):                                 │");
    println!("│   let client = ArchClient::builder()                          │");
    println!("│       .from_env()                                            │");
    println!("│       .build()?;                                             │");
    println!("└──────────────────────────────────────────────────────────────┘\n");

    println!("✓ All examples completed successfully!");
    println!("\n💡 Tip: Try setting different environment variables and running");
    println!("   this example to see how configuration changes behavior.");

    Ok(())
}
