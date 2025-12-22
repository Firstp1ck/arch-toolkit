//! Example demonstrating health check functionality.

use arch_toolkit::{ArchClient, ServiceStatus};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with custom health check timeout
    let client = ArchClient::builder()
        .health_check_timeout(Duration::from_secs(10))
        .build()?;

    println!("Checking archlinux.org connectivity...\n");

    // Simple health check
    match client.health_check().await {
        Ok(true) => println!("✓ Services are healthy"),
        Ok(false) => println!("✗ Services are not fully operational"),
        Err(e) => println!("✗ Health check failed: {e}"),
    }

    println!();

    // Detailed health status
    match client.health_status().await {
        Ok(status) => {
            println!("Detailed Health Status:");
            println!("  AUR API: {:?}", status.aur_api);
            if let Some(latency) = status.latency {
                println!("  Latency: {latency:?}");
            }
            println!("  Is Healthy: {}", status.is_healthy());
            println!("  Checked At: {:?}", status.checked_at);

            // Pattern matching on service status
            match status.aur_api {
                ServiceStatus::Healthy => {
                    println!("\n✓ AUR API is responding normally");
                }
                ServiceStatus::Degraded => {
                    println!("\n⚠ AUR API is slow but functional");
                }
                ServiceStatus::Unreachable => {
                    println!("\n✗ AUR API is unreachable");
                }
                ServiceStatus::Timeout => {
                    println!("\n✗ Health check timed out");
                }
            }
        }
        Err(e) => {
            println!("Failed to get health status: {e}");
        }
    }

    // Example: Using health check in a conditional
    println!("\n--- Conditional Example ---");
    if matches!(client.health_check().await, Ok(true)) {
        println!("Proceeding with AUR operations...");
        // In a real application, you would perform AUR operations here
    } else {
        println!("Skipping AUR operations due to connectivity issues");
    }

    Ok(())
}
