//! Dependency parsing and resolution utilities.
//!
//! This module provides functions for parsing dependency specifications,
//! pacman output, .SRCINFO files, and resolving package dependencies.

mod parse;
mod pkgbuild;
mod srcinfo;

pub use parse::{parse_dep_spec, parse_pacman_si_conflicts, parse_pacman_si_deps};
pub use pkgbuild::{parse_pkgbuild_conflicts, parse_pkgbuild_deps};
pub use srcinfo::{parse_srcinfo, parse_srcinfo_conflicts, parse_srcinfo_deps};

#[cfg(feature = "aur")]
pub use srcinfo::fetch_srcinfo;
