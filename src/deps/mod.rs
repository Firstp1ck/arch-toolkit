//! Dependency parsing and resolution utilities.
//!
//! This module provides functions for parsing dependency specifications,
//! pacman output, and resolving package dependencies.

mod parse;

pub use parse::{parse_dep_spec, parse_pacman_si_conflicts, parse_pacman_si_deps};
