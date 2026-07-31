//! Sandbox module for build-preflight dependency and static PKGBUILD analysis.
//!
//! Given a package's PKGBUILD or .SRCINFO, this module compares its declared
//! dependencies against the host's installed packages and reports, per
//! category (`depends`, `makedepends`, `checkdepends`, `optdepends`):
//!
//! - whether each dependency is installed (or provided) on the host,
//! - the installed version, and
//! - whether the declared version constraint is satisfied.
//!
//! It answers "what would I need to install to build this AUR package?"
//! before any build starts — ported from Pacsea's sandbox preflight. The
//! optional security analysis reads PKGBUILD text only: it never executes,
//! sources, expands, or builds that content, and it reports stable rule IDs and
//! explicit limitations instead of an aggregate risk score.
//!
//! # Features
//!
//! This module requires the `sandbox` feature flag (which enables `deps` for
//! parsing and querying):
//!
//! ```toml
//! [dependencies]
//! arch-toolkit = { version = "0.2", features = ["sandbox"] }
//! ```
//!
//! # Examples
//!
//! ## Analyze a PKGBUILD Against the Host
//!
//! ```no_run
//! use arch_toolkit::deps::{get_installed_packages, get_provided_packages};
//! use arch_toolkit::sandbox::analyze_pkgbuild;
//!
//! let pkgbuild = std::fs::read_to_string("PKGBUILD")?;
//! let installed = get_installed_packages().unwrap_or_default();
//! let provided = get_provided_packages(&installed);
//!
//! let info = analyze_pkgbuild("my-package", &pkgbuild, &installed, &provided);
//! if info.is_ready_to_build() {
//!     println!("All build dependencies present");
//! } else {
//!     println!("Missing: {:?}", info.missing_packages());
//! }
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ## Analyze an AUR Package via .SRCINFO (with the `aur` feature)
//!
//! ```ignore
//! use arch_toolkit::deps::{fetch_srcinfo, get_installed_packages, get_provided_packages};
//! use arch_toolkit::sandbox::analyze_srcinfo;
//!
//! let client = reqwest::Client::new();
//! let srcinfo = fetch_srcinfo(&client, "yay").await?;
//! let installed = get_installed_packages().unwrap_or_default();
//! let provided = get_provided_packages(&installed);
//! let info = analyze_srcinfo("yay", &srcinfo, &installed, &provided);
//! println!("{} missing dependencies", info.missing_packages().len());
//! ```

mod analyze;
mod security;

// Re-export types from types module
pub use crate::types::sandbox::{
    DependencyDelta, SandboxAnalysisLimitation, SandboxFinding, SandboxInfo, SandboxRuleId,
    SandboxStaticAnalysis,
};

// Re-export analysis functions
pub use analyze::{analyze_dependencies, analyze_pkgbuild, analyze_srcinfo, extract_package_name};
pub use security::analyze_pkgbuild_security;
