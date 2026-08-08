//! Install module for building package installation, removal, and update commands.
//!
//! This module builds commands — it **never executes them**. Every builder
//! returns a [`CommandSpec`] (program + argument vector) that callers can:
//!
//! - spawn directly via [`CommandSpec::to_command`] (no shell, no quoting bugs),
//! - render for display or terminals via [`CommandSpec::to_shell_string`],
//! - or simply print (a "dry run" is displaying the command instead of running it).
//!
//! Functionality:
//!
//! - **Command building** — pacman install/remove/update, AUR helper install
//! - **Batch planning** — split mixed target lists between pacman and an AUR helper
//! - **Detection** — find the preferred AUR helper (`paru` → `yay`) and privilege
//!   tool (`doas` → `sudo`) on `PATH`
//! - **Shell safety** — strict package-name validation and POSIX quoting helpers
//!
//! # Features
//!
//! This module requires the `install` feature flag (which enables `deps` for
//! the shared `PackageRef`/`PackageSource` types):
//!
//! ```toml
//! [dependencies]
//! arch-toolkit = { version = "0.3", features = ["install"] }
//! ```
//!
//! # Examples
//!
//! ## Build and Display an Install Command
//!
//! ```
//! use arch_toolkit::install::build_pacman_install;
//! use arch_toolkit::types::install::InstallOptions;
//!
//! let spec = build_pacman_install(&["ripgrep", "fd"], &InstallOptions::default())?;
//! // Dry run: display instead of executing
//! println!("Would run: {}", spec.to_shell_string());
//! # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
//! ```
//!
//! ## Detect Tools and Plan a Mixed Batch
//!
//! ```no_run
//! use arch_toolkit::install::{build_batch_install, detect_aur_helper, detect_privilege_tool};
//! use arch_toolkit::types::install::InstallOptions;
//! use arch_toolkit::{PackageRef, PackageSource};
//!
//! let targets = vec![
//!     PackageRef::official("ripgrep", "14.0.0", "extra", "x86_64"),
//!     PackageRef::aur("yay-bin", "12.0.0"),
//! ];
//! let plan = build_batch_install(
//!     &targets,
//!     detect_aur_helper(),
//!     detect_privilege_tool(),
//!     &InstallOptions::default(),
//!     None::<&std::collections::HashSet<String>>,
//! )?;
//! for command in &plan.commands {
//!     println!("{command}");
//! }
//! # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
//! ```
//!
//! ## Remove Packages with Cascade Control
//!
//! ```
//! use arch_toolkit::install::{build_remove_command, with_privilege};
//! use arch_toolkit::types::install::{CascadeMode, PrivilegeTool};
//!
//! let spec = with_privilege(
//!     PrivilegeTool::Sudo,
//!     build_remove_command(&["old-package"], CascadeMode::CascadeWithConfigs, true)?,
//! );
//! assert_eq!(spec.to_shell_string(), "sudo pacman -Rns --noconfirm -- old-package");
//! # Ok::<(), arch_toolkit::error::ArchToolkitError>(())
//! ```
//!
//! ## Execute a Built Command (caller's decision)
//!
//! ```no_run
//! use arch_toolkit::install::build_update_command;
//!
//! let spec = build_update_command(None, true);
//! let status = spec.to_command().status()?;
//! println!("Update exited with: {status}");
//! # Ok::<(), std::io::Error>(())
//! ```

mod batch;
mod command;
mod detect;
mod shell;

// Re-export types from types module
pub use crate::types::install::{
    AurHelper, CascadeMode, CommandSpec, InstallOptions, PrivilegeTool,
};

// Re-export command builders
pub use command::{
    NO_AUR_HELPER_MESSAGE, aur_install_shell_fallback, aur_update_shell_fallback,
    build_aur_install, build_aur_update_command, build_force_sync_update_command,
    build_pacman_install, build_remove_command, build_update_command, with_privilege,
};

// Re-export batch planning
pub use batch::{InstallPlan, build_batch_install};

// Re-export detection
pub use detect::{
    detect_aur_helper, detect_privilege_tool, is_aur_helper_available, is_privilege_tool_available,
};

// Re-export shell utilities
pub use shell::{
    command_on_path, is_safe_package_name, resolve_command_on_path, shell_single_quote,
    validate_package_names,
};
