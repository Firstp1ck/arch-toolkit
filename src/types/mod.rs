//! Shared data types for arch-toolkit.

pub mod package;

#[cfg(feature = "aur")]
pub mod health;

#[cfg(feature = "deps")]
pub mod dependency;

#[cfg(feature = "index")]
pub mod index;

pub use package::{AurComment, AurPackage, AurPackageDetails};

#[cfg(feature = "aur")]
pub use health::{HealthStatus, ServiceStatus};

#[cfg(feature = "deps")]
pub use dependency::{
    Dependency, DependencySource, DependencySpec, DependencyStatus, PackageRef, PackageSource,
    ReverseDependencySummary, SrcinfoData,
};

#[cfg(feature = "index")]
pub use index::{IndexQueryResult, OfficialIndex, OfficialPackage};
