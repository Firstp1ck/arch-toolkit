# Install Module - Phase 4 Implementation Plan

This document provides a detailed structured plan for implementing the Install Module (`feature = "install"`) in arch-toolkit. This is Phase 4 of the extraction plan from Pacsea.

---

## Executive Summary

| Aspect | Details |
|--------|---------|
| **Module** | `install` (feature = "install", requires `deps`) |
| **Source** | `Pacsea/src/install/` (11 files, ~4,800 lines — only ~800 lines are framework-agnostic) |
| **Estimated Effort** | 12-16 hours |
| **Complexity** | Medium (shell quoting/safety, command composition) |
| **Dependencies** | `types` module, `deps` module (for `PackageRef`/`PackageSource`) |
| **Status** | ✅ Core Complete - Tasks 4.1 through 4.5 complete (2026-07-05) |

## Design Philosophy

Pacsea's install module is heavily TUI-coupled: it spawns terminal emulators, appends
"Press any key to close" hold tails, pipes passwords over stdin, and reads global state
(privilege tool config, package index). **None of that belongs in a library.**

The arch-toolkit install module follows a **"build, don't execute"** design:

1. **Commands are data** — every builder returns a `CommandSpec { program, args }`,
   not a spawned process. Callers decide how to run it (std::process, terminal
   emulator, display-only).
2. **Dry-run is trivially the caller's choice** — since nothing executes, "dry run"
   means displaying `spec.to_shell_string()` instead of running it. This resolves
   Pacsea's dry-run global-state coupling by design (no `dry_run` parameter needed).
3. **No global detection inside builders** — AUR helper and privilege tool are
   explicit parameters; `detect_aur_helper()` / `detect_privilege_tool()` are
   separate opt-in functions.
4. **Argv-style, not shell strings** — `CommandSpec` holds a program + argument
   vector (no quoting bugs possible when spawned directly). A `to_shell_string()`
   method with proper single-quote escaping exists for display and terminal use.
5. **No passwords** — password piping and credential warm-up stay in Pacsea; they
   are UI/session concerns.

## Source Analysis

### Framework-Agnostic Parts to Extract

| Pacsea Source | What to Extract | What to Drop |
|---------------|-----------------|--------------|
| `install/command.rs` (~250) | Install command construction, `--needed`/reinstall flag logic, AUR helper flags | Hold tails, password piping, dry-run echo wrapping |
| `install/executor.rs` (~990) | Helper preference logic (paru → yay) | Terminal spawning, executor plumbing |
| `install/batch.rs` (~510) | Official/AUR target splitting, single-invocation grouping | Terminal spawning, logging, global index queries |
| `install/remove.rs` (~410) | `CascadeMode` flags (`-R`/`-Rs`/`-Rns`), remove command construction | Terminal spawning, config-dir scanning |
| `install/utils.rs` (~570) | `shell_single_quote`, `is_safe_package_name`, `validate_package_names`, `command_on_path` | Terminal choosers, PowerShell/Windows helpers, editor commands |
| `logic/privilege.rs` (~840) | `PrivilegeTool` enum (sudo/doas), availability detection, command prefixing | Password validation, PAM fingerprint, credential cache, config keys |
| `state/modal.rs` | `CascadeMode` enum | Everything else |

### Key Dependencies to Remove

1. **`crate::state::{PackageItem, Source}`** → reuse arch-toolkit's `PackageRef`/`PackageSource` (deps feature)
2. **`crate::index::is_installed` global lookup** → accept installed set as parameter (reinstall detection)
3. **`crate::logic::privilege::active_tool()` global config** → explicit `PrivilegeTool` parameter + `detect_privilege_tool()`
4. **Terminal/hold-tail/password coupling** → dropped (caller concern)
5. **Dry-run global** → dropped by design (commands are data)

---

## Proposed API Design

### Module Structure

```
arch-toolkit/src/
├── install/                       # feature = "install" (requires deps)
│   ├── mod.rs                     # Public API re-exports + docs
│   ├── shell.rs                   # shell_single_quote, package name validation
│   ├── detect.rs                  # AUR helper + privilege tool detection
│   ├── command.rs                 # Single-purpose command builders
│   └── batch.rs                   # Batch install planning (official + AUR split)
└── types/
    └── install.rs                 # AurHelper, PrivilegeTool, CascadeMode, CommandSpec, InstallOptions
```

### Core Types

```rust
/// A ready-to-run command: program + argv (no shell interpretation needed).
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}
impl CommandSpec {
    pub fn to_shell_string(&self) -> String;      // properly quoted, for display/terminals
    pub fn to_command(&self) -> std::process::Command;
}

/// AUR helper preference order: Paru, then Yay.
pub enum AurHelper { Paru, Yay }

/// Privilege escalation tool: Sudo, then Doas.
pub enum PrivilegeTool { Sudo, Doas }

/// Removal cascade level: Basic (-R), Cascade (-Rs), CascadeWithConfigs (-Rns).
pub enum CascadeMode { Basic, Cascade, CascadeWithConfigs }

/// Options for install command building.
pub struct InstallOptions {
    pub needed: bool,       // pass --needed (skip up-to-date); disable for reinstalls
    pub noconfirm: bool,    // pass --noconfirm
    pub aur_only: bool,     // pass --aur to helpers (avoid Chaotic-AUR shadowing)
}
```

### Public Functions

```rust
// Detection (explicit, never called implicitly by builders)
pub fn detect_aur_helper() -> Option<AurHelper>;          // paru preferred, then yay
pub fn detect_privilege_tool() -> Option<PrivilegeTool>;  // sudo preferred, then doas

// Single-purpose builders (validated names, argv-style output)
pub fn build_pacman_install(names, &InstallOptions) -> Result<CommandSpec>;
pub fn build_aur_install(AurHelper, names, &InstallOptions) -> Result<CommandSpec>;
pub fn build_remove_command(names, CascadeMode, noconfirm) -> Result<CommandSpec>;
pub fn build_update_command(Option<AurHelper>, noconfirm) -> CommandSpec;

// Privilege wrapping (pure transformation)
pub fn with_privilege(&PrivilegeTool, CommandSpec) -> CommandSpec;

// Batch planning
pub struct InstallPlan {
    pub commands: Vec<CommandSpec>,   // 0-2 commands: pacman batch, helper batch
    pub official: Vec<String>,        // names routed to pacman
    pub aur: Vec<String>,             // names routed to the AUR helper
}
pub fn build_batch_install(&[PackageRef], &BatchOptions) -> Result<InstallPlan>;

pub struct BatchOptions {
    pub helper: Option<AurHelper>,           // None = build AUR command only if helper detected? No: error if AUR targets present without helper
    pub privilege: Option<PrivilegeTool>,    // wrap pacman command when Some
    pub install: InstallOptions,
    pub installed: Option<&HashSet<String>>, // for reinstall (--needed) detection
}
```

### Shell Safety

- `is_safe_package_name()` / `validate_package_names()` ported from Pacsea:
  reject names with shell metacharacters, whitespace, or leading dashes
  (defense-in-depth; arch package names are `[a-z0-9@._+-]`).
- `shell_single_quote()` ported for `to_shell_string()` rendering.
- Builders return `ArchToolkitError::InvalidPackageName` on validation failure.

---

## Implementation Tasks

### Task 4.1: Define Standalone Types

**File**: `src/types/install.rs`

- [x] `AurHelper` with `binary_name()`, availability check, Display, Serde
- [x] `PrivilegeTool` with `binary_name()`, availability check, Display, Serde
- [x] `CascadeMode` with `flag()`, `description()`, Display, Serde
- [x] `CommandSpec` with `to_shell_string()`, `to_command()`
- [x] `InstallOptions` with sensible `Default` (needed=true, noconfirm=true, aur_only=true)
- [x] Unit tests, rustdoc (What/Inputs/Output/Details)

**Estimated Effort**: 2-3 hours — **Status**: ✅ Complete

### Task 4.2: Shell Utilities

**File**: `src/install/shell.rs`

- [x] Port `shell_single_quote()` from Pacsea utils.rs
- [x] Port `is_safe_package_name()` and `validate_package_names()`
- [x] `command_on_path()` helper for detection
- [x] Unit tests (quoting edge cases, injection attempts)

**Estimated Effort**: 2 hours — **Status**: ✅ Complete

### Task 4.3: Detection and Command Builders

**Files**: `src/install/detect.rs`, `src/install/command.rs`

- [x] `detect_aur_helper()` (paru → yay preference from executor.rs)
- [x] `detect_privilege_tool()` (sudo → doas)
- [x] `build_pacman_install()` with `--needed`/`--noconfirm` flag logic from command.rs
- [x] `build_aur_install()` with `--aur` flag handling
- [x] `build_remove_command()` with `CascadeMode` flags from remove.rs
- [x] `build_update_command()` (pacman -Syu / helper -Syu)
- [x] `with_privilege()` command wrapping
- [x] Unit tests for all builders

**Estimated Effort**: 4-5 hours — **Status**: ✅ Complete

### Task 4.4: Batch Operations

**File**: `src/install/batch.rs`

- [x] `build_batch_install()` splitting official/AUR targets from batch.rs
- [x] Reinstall detection via caller-provided installed set (no global index)
- [x] Error when AUR targets present but no helper provided
- [x] `InstallPlan` result type with routed names
- [x] Unit tests (official-only, AUR-only, mixed, empty, no-helper error)

**Estimated Effort**: 3-4 hours — **Status**: ✅ Complete

### Task 4.5: Integration, Testing, Documentation

- [x] `src/install/mod.rs` with module docs and re-exports
- [x] `install = ["deps"]` feature flag in Cargo.toml
- [x] lib.rs conditional module + re-exports, prelude exports
- [x] Integration tests `tests/install_integration.rs`
- [x] Example program `examples/install_example.rs`
- [x] README section
- [x] Quality checks across feature combos

**Estimated Effort**: 3-4 hours — **Status**: ✅ Complete

---

## Explicitly Out of Scope

- **Command execution / terminal spawning** — caller concern (Pacsea keeps its own)
- **Password handling** (stdin piping, credential warm-up, validation) — UI/session concern
- **Downgrade/scan commands** (`build_downgrade_command_for_executor`, VirusTotal scan) — Pacsea-specific workflows
- **Config directory scanning on removal** — filesystem policy, not command building
- **Windows PowerShell paths** — Linux-only module (like index fetch)

## Acceptance Criteria

- [x] All builders return `CommandSpec`, never execute anything
- [x] Package names validated against shell injection
- [x] No global state; helper/privilege/installed-set are parameters
- [x] `cargo fmt` / `cargo clippy` clean (all feature combos)
- [x] All tests pass with `cargo test -- --test-threads=1`
- [x] Works standalone: `--no-default-features --features install`

---

## References

- [AUR_TOOLKIT_CRATE_PREPARATION.md](./AUR_TOOLKIT_CRATE_PREPARATION.md) - Overall extraction plan
- [INDEX_MODULE_PHASE.md](./INDEX_MODULE_PHASE.md) - Phase 3 plan (pattern reference)
- Pacsea source: `src/install/`, `src/logic/privilege.rs`
