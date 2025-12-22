//! Shared data types for arch-toolkit.

pub mod package;

#[cfg(feature = "aur")]
pub mod health;

pub use package::{AurComment, AurPackage, AurPackageDetails};

#[cfg(feature = "aur")]
pub use health::{HealthStatus, ServiceStatus};
