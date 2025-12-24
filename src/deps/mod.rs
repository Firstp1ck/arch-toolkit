//! Dependency parsing and resolution utilities for Arch Linux packages.
//!
//! This module provides a comprehensive set of functions and types for working with
//! Arch Linux package dependencies, including parsing, version comparison, package
//! querying, dependency resolution, and reverse dependency analysis.
//!
//! # Features
//!
//! This module is enabled with the `deps` feature flag:
//!
//! ```toml
//! [dependencies]
//! arch-toolkit = { version = "0.1.2", features = ["deps"] }
//! ```
//!
//! Some functions require the `aur` feature for AUR integration:
//!
//! ```toml
//! [dependencies]
//! arch-toolkit = { version = "0.1.2", features = ["deps", "aur"] }
//! ```
//!
//! # Overview
//!
//! The deps module provides:
//!
//! - **Parsing**: Parse dependency specifications, pacman output, .SRCINFO files, and PKGBUILD files
//! - **Version Comparison**: Compare package versions using pacman-compatible algorithms
//! - **Package Querying**: Query installed packages, upgradable packages, and package versions
//! - **Dependency Resolution**: Resolve dependencies for packages from official repos, AUR, or local packages
//! - **Reverse Dependency Analysis**: Find all packages that depend on a given package
//!
//! All functions gracefully degrade when pacman is unavailable, returning empty sets or `None`
//! as appropriate rather than failing.
//!
//! # Examples
//!
//! ## Parsing Dependency Specifications
//!
//! ```no_run
//! use arch_toolkit::deps::parse_dep_spec;
//!
//! let spec = parse_dep_spec("python>=3.12");
//! assert_eq!(spec.name, "python");
//! assert_eq!(spec.version_req, ">=3.12");
//! ```
//!
//! ## Parsing .SRCINFO Files
//!
//! ```no_run
//! use arch_toolkit::deps::parse_srcinfo;
//!
//! let srcinfo_content = r#"
//! pkgbase = my-package
//! pkgname = my-package
//! pkgver = 1.0.0
//! pkgrel = 1
//! depends = glibc
//! depends = python>=3.10
//! "#;
//!
//! let data = parse_srcinfo(srcinfo_content);
//! assert_eq!(data.pkgname, "my-package");
//! assert!(data.depends.contains(&"glibc".to_string()));
//! ```
//!
//! ## Parsing PKGBUILD Files
//!
//! ```no_run
//! use arch_toolkit::deps::parse_pkgbuild_deps;
//!
//! let pkgbuild = r#"
//! depends=('glibc' 'python>=3.10')
//! makedepends=('rust' 'cargo')
//! "#;
//!
//! let (deps, makedeps, checkdeps, optdeps) = parse_pkgbuild_deps(pkgbuild);
//! assert!(deps.contains(&"glibc".to_string()));
//! ```
//!
//! ## Version Comparison
//!
//! ```no_run
//! use arch_toolkit::deps::version_satisfies;
//!
//! assert!(version_satisfies("2.0", ">=1.5"));
//! assert!(!version_satisfies("1.0", ">=1.5"));
//! ```
//!
//! ## Querying Installed Packages
//!
//! ```no_run
//! use arch_toolkit::deps::get_installed_packages;
//!
//! let installed = get_installed_packages().unwrap();
//! println!("Found {} installed packages", installed.len());
//! ```
//!
//! ## Dependency Resolution
//!
//! ```no_run
//! use arch_toolkit::deps::DependencyResolver;
//! use arch_toolkit::{PackageRef, PackageSource};
//!
//! let resolver = DependencyResolver::new();
//! let packages = vec![
//!     PackageRef {
//!         name: "firefox".into(),
//!         version: "121.0".into(),
//!         source: PackageSource::Official {
//!             repo: "extra".into(),
//!             arch: "x86_64".into(),
//!         },
//!     },
//! ];
//!
//! let result = resolver.resolve(&packages).unwrap();
//! println!("Found {} dependencies", result.dependencies.len());
//! ```
//!
//! ## Reverse Dependency Analysis
//!
//! ```no_run
//! use arch_toolkit::deps::ReverseDependencyAnalyzer;
//! use arch_toolkit::{PackageRef, PackageSource};
//!
//! let analyzer = ReverseDependencyAnalyzer::new();
//! let packages = vec![
//!     PackageRef {
//!         name: "qt5-base".into(),
//!         version: "5.15.10".into(),
//!         source: PackageSource::Official {
//!             repo: "extra".into(),
//!             arch: "x86_64".into(),
//!         },
//!     },
//! ];
//!
//! let report = analyzer.analyze(&packages).unwrap();
//! println!("{} packages would be affected", report.dependents.len());
//! ```
//!
//! ## Fetching .SRCINFO from AUR (requires `aur` feature)
//!
//! ```no_run
//! # #[cfg(feature = "aur")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use arch_toolkit::deps::fetch_srcinfo;
//! use reqwest::Client;
//!
//! let client = Client::new();
//! let srcinfo = fetch_srcinfo(&client, "yay").await?;
//! let data = arch_toolkit::deps::parse_srcinfo(&srcinfo);
//! println!("Package: {}", data.pkgname);
//! # Ok(())
//! # }
//! ```
//!
//! # Example Programs
//!
//! See the following example programs for more comprehensive usage:
//!
//! - [`examples/deps_types_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/deps_types_example.rs) - Comprehensive type usage examples
//! - [`examples/parse_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/parse_example.rs) - Dependency parsing examples
//! - [`examples/srcinfo_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/srcinfo_example.rs) - .SRCINFO parsing examples
//! - [`examples/pkgbuild_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/pkgbuild_example.rs) - PKGBUILD parsing examples
//! - [`examples/query_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/query_example.rs) - Package querying examples
//! - [`examples/source_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/source_example.rs) - Source determination examples
//! - [`examples/version_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/version_example.rs) - Version comparison examples
//! - [`examples/resolve_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/resolve_example.rs) - Dependency resolution examples
//! - [`examples/reverse_example.rs`](https://github.com/Firstp1ck/arch-toolkit/blob/main/examples/reverse_example.rs) - Reverse dependency analysis examples

mod parse;
mod pkgbuild;
mod query;
mod resolve;
mod reverse;
mod source;
mod srcinfo;
mod version;

// Re-export parsing functions
pub use parse::{parse_dep_spec, parse_pacman_si_conflicts, parse_pacman_si_deps};
pub use pkgbuild::{parse_pkgbuild_conflicts, parse_pkgbuild_deps};
pub use query::{
    get_available_version, get_installed_packages, get_installed_version, get_provided_packages,
    get_upgradable_packages, is_package_installed_or_provided,
};
pub use resolve::{
    DependencyResolver, batch_fetch_official_deps, determine_status, fetch_package_conflicts,
};
pub use reverse::{
    ReverseDependencyAnalyzer, get_installed_required_by, has_installed_required_by,
};
pub use source::{determine_dependency_source, is_system_package};
pub use srcinfo::{parse_srcinfo, parse_srcinfo_conflicts, parse_srcinfo_deps};
pub use version::{
    compare_versions, extract_major_component, is_major_version_bump, version_satisfies,
};

// AUR integration (requires aur feature)
#[cfg(feature = "aur")]
pub use srcinfo::fetch_srcinfo;

// Re-export types from types module
pub use crate::types::dependency::{
    DependencyResolution, ResolverConfig, ReverseDependencyReport, ReverseDependencySummary,
};
