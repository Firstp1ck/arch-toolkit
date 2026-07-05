# Sandbox Module - Phase 6 Implementation Plan

This document provides a detailed structured plan for implementing the Sandbox Module (`feature = "sandbox"`) in arch-toolkit. This is Phase 6 — the final planned module — of the extraction plan from Pacsea.

---

## Executive Summary

| Aspect | Details |
|--------|---------|
| **Module** | `sandbox` (feature = "sandbox", requires `deps`) |
| **Source** | `Pacsea/src/logic/sandbox/` (~1,480 lines — most parsing already ported in Phase 2) |
| **Estimated Effort** | 4-6 hours (thin layer over the deps module) |
| **Complexity** | Low |
| **Dependencies** | `deps` module (parsing, querying, version comparison) |
| **Status** | ✅ Core Complete - Tasks 6.1 through 6.3 complete (2026-07-05) |

## Scope Correction

Earlier planning documents described this phase as "PKGBUILD security analysis" with
"risk categorization". **Pacsea's actual sandbox module is build-preflight dependency
delta analysis**: given a package's PKGBUILD or .SRCINFO, compare its dependency
arrays against the host's installed packages and report what is missing, what is
installed, and whether installed versions satisfy the declared constraints. There is
no risk-scoring code in Pacsea to port (its security angle is the external aur-sleuth /
VirusTotal integration, which is app-level tooling, not library material).

This phase ports what exists. A future security-lint enhancement can build on it.

## Source Analysis

| Pacsea Source | What to Extract | Notes |
|---------------|-----------------|-------|
| `sandbox/types.rs` (~30) | `DependencyDelta`, `SandboxInfo` | Ported with added helper methods |
| `sandbox/analyze.rs` (~210) | `analyze_dependencies()`, PKGBUILD/SRCINFO analysis entry points, `extract_package_name()`, `is_local_package()` | Reuses deps-module functions instead of Pacsea's `logic::deps` |
| `sandbox/parse.rs` (~565) | Nothing — already ported | Phase 2 ported this as `deps::parse_pkgbuild_deps` etc. |
| `sandbox/mod.rs` (~275) | Nothing — async multi-package orchestration | Uses Pacsea's `PackageItem` + `FuturesUnordered` fetch pipeline; callers can compose `deps::fetch_srcinfo` + `analyze_srcinfo` themselves |
| `sandbox/fetch.rs` (~4) | Nothing | Re-export shim |

### Improvements over Pacsea

1. **Correct version checking** — Pacsea passed the full dep spec (`foo>=1.2`) as the
   requirement to `version_satisfies`, which treats it as "no operator → satisfied".
   arch-toolkit parses the spec with `deps::parse_dep_spec()` and checks the actual
   constraint (`>=1.2`) against the installed version.
2. **Generic over `BuildHasher`** — matches the deps module's flexibility.
3. **Explicit inputs** — installed/provided sets are parameters (obtainable via
   `deps::get_installed_packages()` / `deps::get_provided_packages()`); no hidden
   global queries besides the documented per-dependency version/local lookups.

---

## Proposed API Design

### Module Structure

```
arch-toolkit/src/
├── sandbox/                    # feature = "sandbox" (requires deps)
│   ├── mod.rs                  # Public API re-exports + docs
│   └── analyze.rs              # Dependency delta analysis
└── types/
    └── sandbox.rs              # DependencyDelta, SandboxInfo
```

### Core Types

```rust
/// Status of one declared dependency relative to the host.
pub struct DependencyDelta {
    pub name: String,                       // full spec as declared (e.g. "foo>=1.2")
    pub is_installed: bool,                 // installed or provided on the host
    pub installed_version: Option<String>,  // from pacman -Q when installed
    pub version_satisfied: bool,            // constraint check against installed version
}

/// Build-preflight analysis result for a package.
pub struct SandboxInfo {
    pub package_name: String,
    pub depends: Vec<DependencyDelta>,
    pub makedepends: Vec<DependencyDelta>,
    pub checkdepends: Vec<DependencyDelta>,
    pub optdepends: Vec<DependencyDelta>,
}
impl SandboxInfo {
    pub fn missing_packages(&self) -> Vec<&str>;   // not installed (excl. optdepends)
    pub fn is_ready_to_build(&self) -> bool;       // depends+makedepends+checkdepends all installed
}
```

### Public Functions

```rust
pub fn analyze_pkgbuild<S: BuildHasher>(name, pkgbuild_text, installed, provided) -> SandboxInfo;
pub fn analyze_srcinfo<S: BuildHasher>(name, srcinfo_text, installed, provided) -> SandboxInfo;
pub fn analyze_dependencies<S: BuildHasher>(deps, installed, provided) -> Vec<DependencyDelta>;
pub fn extract_package_name(dep_spec: &str) -> String;  // strips version ops + optdepends descriptions
```

---

## Implementation Tasks

### Task 6.1: Define Standalone Types

**File**: `src/types/sandbox.rs`

- [x] `DependencyDelta`, `SandboxInfo` with Serde
- [x] `missing_packages()` and `is_ready_to_build()` helpers
- [x] Unit tests, rustdoc (What/Inputs/Output/Details)

**Estimated Effort**: 1 hour — **Status**: ✅ Complete

### Task 6.2: Port Dependency Delta Analysis

**File**: `src/sandbox/analyze.rs`

- [x] `analyze_dependencies()` with installed/provided membership via `deps::is_package_installed_or_provided`
- [x] Proper constraint checking via `deps::parse_dep_spec` + `deps::version_satisfies` (fixes Pacsea quirk)
- [x] `extract_package_name()` handling optdepends `pkg: description` form
- [x] Local-package filtering (`pacman -Qi` Repository field) with graceful degradation
- [x] `analyze_pkgbuild()` / `analyze_srcinfo()` entry points reusing Phase 2 parsers
- [x] Unit tests (membership, version constraints, optdepends, empty input)

**Estimated Effort**: 2-3 hours — **Status**: ✅ Complete

### Task 6.3: Integration, Testing, Documentation

- [x] `src/sandbox/mod.rs` with module docs and re-exports
- [x] `sandbox = ["deps"]` feature flag; lib.rs/types/prelude wiring
- [x] Integration tests `tests/sandbox_integration.rs`
- [x] Example program `examples/sandbox_example.rs`
- [x] README section
- [x] Quality checks across feature combos (including standalone `--no-default-features --features sandbox`)

**Estimated Effort**: 1-2 hours — **Status**: ✅ Complete

---

## Explicitly Out of Scope

- **Security risk scoring / PKGBUILD linting** — does not exist in Pacsea; future enhancement
- **Async multi-package orchestration** — compose `deps::fetch_srcinfo` (aur feature) with `analyze_srcinfo` instead
- **aur-sleuth / VirusTotal integration** — external app-level tooling

## Acceptance Criteria

- [x] Analysis works from both PKGBUILD and .SRCINFO text
- [x] Version constraints actually checked (improvement over Pacsea)
- [x] Installed/provided sets are explicit parameters
- [x] `cargo fmt` / `cargo clippy` clean (all feature combos)
- [x] All tests pass with `cargo test -- --test-threads=1`
- [x] Works standalone: `--no-default-features --features sandbox`

---

## References

- [AUR_TOOLKIT_CRATE_PREPARATION.md](./AUR_TOOLKIT_CRATE_PREPARATION.md) - Overall extraction plan
- [DEPENDENCIES_MODULE_PHASE.md](./DEPENDENCIES_MODULE_PHASE.md) - Phase 2 (parsers reused here)
- Pacsea source: `src/logic/sandbox/`
